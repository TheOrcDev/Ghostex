//! Keeps a resized `SurfaceView` from showing the previous size's buffer stretched.
//!
//! A `SurfaceView` whose size changed shows its old buffer scaled to the new frame until the app
//! queues a buffer of the new size: squashed text while the keyboard opens, stretched text while
//! it closes. `SurfaceHolder.Callback2.surfaceRedrawNeededAsync` lets the view system hold the
//! window's next frame until the surface has content of the new size; the Kotlin view hands that
//! wait to [`requested`], and the present observer ends it once GPUI has put a frame of exactly
//! that size on screen.

use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

/// The size a pending redraw waits for, `width << 32 | height`; 0 when nothing waits.
static WANTED: AtomicU64 = AtomicU64::new(0);

fn pack(width: i32, height: i32) -> u64 {
    ((width.max(0) as u64) << 32) | height.max(0) as u64
}

pub(super) fn install() {
    gpui_mobile::android::window::set_present_observer(|width, height| {
        let size = pack(width, height);
        if size != 0
            && WANTED
                .compare_exchange(size, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            super::events::redraw_done(width, height);
        }
    });
}

/// The view system waits for a frame of `width` x `height`. GPUI may already have drawn that size
/// before the request arrived, so ask it for one more frame; the observer answers when it lands.
pub(super) fn requested(width: i32, height: i32) {
    WANTED.store(pack(width, height), Ordering::Release);
    if let Err(error) = crate::runtime::post_value(json!({"type": "redraw"})) {
        log::warn!("redraw request dropped: {error:#}");
    }
}
