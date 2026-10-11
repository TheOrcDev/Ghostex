//! Ghostex patch: what `IosWindow::handle_touch` sends GPUI besides a position (GHOSTEX.md,
//! patch 18): each sample's time, UIKit's coalesced samples of a move, its predicted position,
//! and the switch that relays touches without `crate::fling_guard`.

use gpui::{Pixels, Point};
use objc2::msg_send;
use objc2::runtime::AnyObject;

use super::events::touch_location_in_view;

static FLING_GUARD_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

/// Ghostex patch: turn `crate::fling_guard` off (on by default, as upstream), the
/// iOS twin of `android::window::set_fling_guard_enabled`. GPUI's own recognizer
/// already treats a touch that catches a fling as a pan that can never be a tap;
/// the guard turned that touch into an ordinary tap, which opened the row under
/// the finger.
pub fn set_fling_guard_enabled(enabled: bool) {
    FLING_GUARD_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn fling_guard_enabled() -> bool {
    FLING_GUARD_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

#[link(name = "QuartzCore", kind = "framework")]
extern "C" {
    /// QuartzCore's clock: `mach_absolute_time` in seconds, the base of `UITouch.timestamp`.
    fn CACurrentMediaTime() -> f64;
}

/// Ghostex patch: the `Instant` a touch sample was taken, from `UITouch.timestamp`
/// (seconds since boot on `CACurrentMediaTime`'s clock). The sample's age is taken
/// off `Instant::now()`, so the two clocks' bases never have to agree. A sample from
/// the future or older than a few seconds gives `None`, and GPUI uses the time it
/// processes the touch instead.
pub(crate) fn touch_instant(touch: *mut AnyObject) -> Option<std::time::Instant> {
    let sampled: f64 = unsafe { msg_send![touch, timestamp] };
    let age = unsafe { CACurrentMediaTime() } - sampled;
    if !(0.0..=5.0).contains(&age) {
        return None;
    }
    std::time::Instant::now().checked_sub(std::time::Duration::from_secs_f64(age))
}

/// Ghostex patch: UIKit's coalesced samples of a moved touch, oldest first, the
/// last being `touch` itself. Empty when UIKit has none to offer.
pub(crate) fn coalesced_touches(
    touch: *mut AnyObject,
    event: *mut AnyObject,
) -> Vec<*mut AnyObject> {
    if event.is_null() {
        return Vec::new();
    }
    unsafe {
        let samples: *mut AnyObject = msg_send![event, coalescedTouchesForTouch: touch];
        if samples.is_null() {
            return Vec::new();
        }
        let count: usize = msg_send![samples, count];
        (0..count)
            .map(|index| msg_send![samples, objectAtIndex: index])
            .collect()
    }
}

/// Ghostex patch: where UIKit predicts the touch will be next (the furthest of its
/// predicted samples), for GPUI's latency compensation.
pub(crate) fn predicted_touch_location(
    touch: *mut AnyObject,
    event: *mut AnyObject,
    view: *mut AnyObject,
) -> Option<Point<Pixels>> {
    if event.is_null() {
        return None;
    }
    unsafe {
        let predicted: *mut AnyObject = msg_send![event, predictedTouchesForTouch: touch];
        if predicted.is_null() {
            return None;
        }
        let last: *mut AnyObject = msg_send![predicted, lastObject];
        (!last.is_null()).then(|| touch_location_in_view(last, view))
    }
}
