//! Wheel normalization for the composited terminal, one branch per ghostty
//! apprt.
//!
//! CDXC:Terminal 2026-09-20 WHY:
//! Ghostty's core takes a scroll offset plus `precision`/`pixel_delta` flags and each apprt normalizes its platform's wheel input before that call: macOS doubles precise trackpad deltas and passes raw ticks otherwise, GTK multiplies precise deltas by 10, and the Win32 apprt turns the raw `WM_MOUSEWHEEL` delta into pixels through the system lines-per-notch setting, treating any sub-notch report as precision input. This module does the same for the gpui backends, so the element feeds ghostty's own accumulator instead of hand-converting wheel events to rows.
//! gpui's Windows backend has already multiplied the notch delta by the system setting when it hands over `ScrollDelta::Lines`, and it drops the "one screen at a time" and "no scrolling" values on the floor, so the Windows branch divides that factor back out and re-applies the rules from ghostty's win32 apprt.
//! SEE-ALSO: .dependencies/ghostty/src/Surface.zig (`scrollCallback`), apps/desktop/src/terminal_element.rs.

use gpui::ScrollDelta;

/// Neutral value of `mouse-scroll-multiplier`: the value at which a wheel
/// notch scrolls exactly what the platform asks for. This is ghostty's own
/// default of 3 rows per tick, and the Discrete scroll multiplier field is
/// that ghostty key (it is written straight into the ghostty config the
/// embedded macOS surfaces load), so the same number must mean the same
/// speed everywhere: 3 is platform speed, 6 is twice it.
const NEUTRAL_DISCRETE_MULTIPLIER: f32 = 3.0;

/// "Scroll one screen at a time" (`SPI_GETWHEELSCROLLLINES` sentinel).
#[cfg(target_os = "windows")]
const WHEEL_PAGESCROLL: u32 = u32::MAX;

/// A wheel event in ghostty's terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WheelScroll {
    /// Positive scrolls up. Pixels when `pixel_delta`, wheel ticks otherwise.
    pub(crate) offset: f32,
    /// High-precision device (trackpad, or a sub-notch high-resolution wheel).
    pub(crate) precision: bool,
    /// `offset` is already normalized to pixels.
    pub(crate) pixel_delta: bool,
}

/// Ghostty's `Surface.mouse.pending_scroll_y`: sub-cell remainders are held
/// back so a slow high-resolution device still scrolls smoothly instead of
/// dropping everything below one row.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WheelAccumulator {
    pending: f32,
}

impl WheelAccumulator {
    /// Ghostty's `accumulateScroll`: whole rows out, remainder kept.
    pub(crate) fn take_rows(&mut self, offset_pixels: f32, cell_size: f32) -> i32 {
        if cell_size <= 0. {
            return 0;
        }
        let total = self.pending + offset_pixels;
        if total.abs() < cell_size {
            self.pending = total;
            return 0;
        }
        let rows = (total / cell_size).trunc();
        self.pending = total - rows * cell_size;
        rows as i32
    }
}

/// Distance in rows for one wheel event, applying the configured
/// `mouse-scroll-multiplier` exactly where ghostty applies it.
pub(crate) fn wheel_rows(
    accumulator: &mut WheelAccumulator,
    scroll: WheelScroll,
    cell_height: f32,
    precision_multiplier: f32,
    discrete_multiplier: f32,
) -> i32 {
    let adjusted = if scroll.pixel_delta {
        scroll.offset
            * if scroll.precision {
                precision_multiplier
            } else {
                discrete_multiplier / NEUTRAL_DISCRETE_MULTIPLIER
            }
    } else if scroll.precision {
        scroll.offset * precision_multiplier
    } else {
        // Ticks: ghostty converts them to pixels itself. On macOS AppKit
        // ramps the magnitude with scroll speed and reports a very slow
        // click as 0.1, which ghostty rounds out to a whole tick.
        let ticks = if cfg!(target_os = "macos") {
            if scroll.offset > 0. {
                scroll.offset.max(1.)
            } else {
                scroll.offset.min(-1.)
            }
        } else {
            scroll.offset
        };
        ticks * cell_height * discrete_multiplier
    };
    accumulator.take_rows(adjusted, cell_height)
}

/// Normalize one gpui wheel event the way the matching ghostty apprt does.
/// `viewport_height` carries the "one screen at a time" Windows setting.
pub(crate) fn normalize(
    delta: ScrollDelta,
    shift: bool,
    cell_height: f32,
    viewport_height: f32,
) -> Option<WheelScroll> {
    // Only the Windows branch consults the system wheel settings.
    #[cfg(not(target_os = "windows"))]
    let _ = (shift, cell_height, viewport_height);
    match delta {
        // Trackpads and other precise devices. Ghostty's AppKit surface
        // applies a 2x speed multiplier to precise deltas before the core
        // sees them (GTK uses 10x for the same reason); keep the platform's
        // own factor so an embedded ghostty surface and this element scroll
        // the same distance for the same gesture.
        ScrollDelta::Pixels(pixels) => {
            let offset = pixels.y.to_f64() as f32 * if cfg!(target_os = "macos") { 2. } else { 1. };
            (offset != 0.).then_some(WheelScroll {
                offset,
                precision: true,
                pixel_delta: true,
            })
        }
        #[cfg(not(target_os = "windows"))]
        ScrollDelta::Lines(lines) => (lines.y != 0.).then_some(WheelScroll {
            offset: lines.y,
            precision: false,
            pixel_delta: false,
        }),
        #[cfg(target_os = "windows")]
        ScrollDelta::Lines(lines) => {
            let (setting_lines, setting_chars) = windows::system_wheel_settings();
            // gpui routes a shift-wheel to the horizontal axis with the
            // characters-per-notch setting, and a real horizontal wheel the
            // same way. The terminal reads shift-wheel as a vertical scroll
            // that overrides mouse reporting, so take that axis back.
            let (raw, setting) = if lines.y != 0. || !shift {
                (lines.y, setting_lines)
            } else {
                (lines.x, setting_chars)
            };
            if raw == 0. || setting == 0 {
                return None;
            }
            let notches = raw / setting as f32;
            // Sub-notch reports come from high-resolution wheels and precision
            // touchpads, and scroll by the accumulated fraction of a row.
            let precision = notches.fract() != 0.;
            // Deliberate difference from ghostty's win32 apprt, which drops
            // the lines-per-notch setting for high-resolution reports: that
            // makes one detent of a high-resolution wheel travel a third of
            // what the same detent travels on a plain wheel. Windows means
            // the setting to apply to both, so only "one screen at a time"
            // is reserved for whole notches.
            let lines_per_notch = (setting != WHEEL_PAGESCROLL).then_some(setting as f32);
            let offset = match lines_per_notch {
                Some(lines) => notches * lines * cell_height,
                None if precision => notches * cell_height,
                None => notches * cell_height.max(viewport_height - cell_height),
            };
            Some(WheelScroll {
                offset,
                precision,
                pixel_delta: true,
            })
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SPI_GETWHEELSCROLLCHARS, SPI_GETWHEELSCROLLLINES, SYSTEM_PARAMETERS_INFO_ACTION,
        SystemParametersInfoW,
    };

    /// Windows defaults, used until the first successful read and whenever one
    /// fails.
    const DEFAULT_LINES: u32 = 3;
    const DEFAULT_CHARS: u32 = 3;
    /// The mouse control panel can change these at any time and the element
    /// has no message loop of its own, so re-read them on a short cadence
    /// instead of watching for `WM_SETTINGCHANGE`.
    const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

    thread_local! {
        static CACHE: Cell<Option<(u32, u32, Instant)>> = const { Cell::new(None) };
    }

    /// `(lines per notch, characters per notch)`.
    pub(super) fn system_wheel_settings() -> (u32, u32) {
        let now = Instant::now();
        if let Some((lines, chars, read_at)) = CACHE.get()
            && now.saturating_duration_since(read_at) < REFRESH_INTERVAL
        {
            return (lines, chars);
        }
        let lines = read_setting(SPI_GETWHEELSCROLLLINES, DEFAULT_LINES);
        let chars = read_setting(SPI_GETWHEELSCROLLCHARS, DEFAULT_CHARS);
        CACHE.set(Some((lines, chars, now)));
        (lines, chars)
    }

    fn read_setting(action: SYSTEM_PARAMETERS_INFO_ACTION, fallback: u32) -> u32 {
        let mut value: u32 = fallback;
        let ok = unsafe {
            SystemParametersInfoW(
                action,
                0,
                (&raw mut value).cast::<c_void>(),
                Default::default(),
            )
        };
        if ok == 0 { fallback } else { value }
    }
}
