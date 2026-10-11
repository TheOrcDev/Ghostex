use std::{
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

use gpui::{Bounds, Hsla, Pixels, Window, px, rgb};

use super::*;
use crate::*;

/// Paints the titlebar update-download ring: a dim full-circle track plus a
/// clockwise-from-noon fill arc for the normalized Sparkle progress ratio.
/// Unknown progress (`None`) paints the track only, matching the React ring's
/// empty-fill unknown-size state.
pub(crate) fn paint_titlebar_update_progress_ring(
    bounds: Bounds<Pixels>,
    progress: Option<f64>,
    window: &mut Window,
) {
    let center_x = bounds.left().as_f32() + bounds.size.width.as_f32() / 2.0;
    let center_y = bounds.top().as_f32() + bounds.size.height.as_f32() / 2.0;
    let radius = TITLEBAR_UPDATE_PROGRESS_RING_RADIUS;
    let radii = gpui::point(px(radius), px(radius));

    let mut track = gpui::PathBuilder::stroke(px(TITLEBAR_UPDATE_PROGRESS_RING_STROKE));
    track.move_to(gpui::point(px(center_x + radius), px(center_y)));
    track.arc_to(
        radii,
        px(0.0),
        false,
        true,
        gpui::point(px(center_x - radius), px(center_y)),
    );
    track.arc_to(
        radii,
        px(0.0),
        false,
        true,
        gpui::point(px(center_x + radius), px(center_y)),
    );
    if let Ok(path) = track.build() {
        window.paint_path(path, titlebar_update_progress_track_color());
    }

    let clamped = match progress {
        Some(progress) => progress.clamp(0.0, 1.0) as f32,
        None => {
            // Match the legacy CSS pending-fill animation: grow from 4% to
            // 72% over the first 55% of a 1.25s cycle, then contract again.
            let elapsed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f32();
            let phase = (elapsed % 1.25) / 1.25;
            let leg = if phase <= 0.55 {
                phase / 0.55
            } else {
                1.0 - ((phase - 0.55) / 0.45)
            };
            let eased = leg.clamp(0.0, 1.0);
            let eased = eased * eased * (3.0 - 2.0 * eased);
            0.04 + (0.72 - 0.04) * eased
        }
    };
    if clamped <= 0.0 {
        return;
    }
    // Cap just under a full turn so the arc endpoint never coincides with its
    // start point (a zero-length lyon arc would drop the full-progress ring).
    let sweep = clamped.min(0.999) * std::f32::consts::TAU;
    let end_angle = -std::f32::consts::FRAC_PI_2 + sweep;
    let mut fill = gpui::PathBuilder::stroke(px(TITLEBAR_UPDATE_PROGRESS_RING_STROKE));
    fill.move_to(gpui::point(px(center_x), px(center_y - radius)));
    fill.arc_to(
        radii,
        px(0.0),
        sweep > std::f32::consts::PI,
        true,
        gpui::point(
            px(center_x + radius * end_angle.cos()),
            px(center_y + radius * end_angle.sin()),
        ),
    );
    if let Ok(path) = fill.build() {
        window.paint_path(path, titlebar_active_text_color());
    }
}

pub(crate) fn paint_titlebar_git_busy_spinner(bounds: Bounds<Pixels>, window: &mut Window) {
    let center_x = bounds.left().as_f32() + bounds.size.width.as_f32() / 2.0;
    let center_y = bounds.top().as_f32() + bounds.size.height.as_f32() / 2.0;
    let radius = 5.5;
    let start_angle = -std::f32::consts::FRAC_PI_2;
    let sweep = std::f32::consts::PI * 1.45;
    let end_angle = start_angle + sweep;
    let radii = gpui::point(px(radius), px(radius));
    let mut path = gpui::PathBuilder::stroke(px(1.6));
    path.move_to(gpui::point(
        px(center_x + radius * start_angle.cos()),
        px(center_y + radius * start_angle.sin()),
    ));
    path.arc_to(
        radii,
        px(0.0),
        sweep > std::f32::consts::PI,
        true,
        gpui::point(
            px(center_x + radius * end_angle.cos()),
            px(center_y + radius * end_angle.sin()),
        ),
    );
    if let Ok(path) = path.build() {
        window.paint_path(path, titlebar_active_text_color());
    }
}

pub(crate) fn titlebar_update_progress_track_color() -> Hsla {
    rgb(0xffffff).opacity(0.24).into()
}

pub(crate) fn titlebar_update_available_color() -> Hsla {
    titlebar_active_text_color()
}

pub(crate) fn titlebar_update_downloading_color() -> Hsla {
    rgb(GPUI_TITLEBAR_FOREGROUND_RGB.load(Ordering::Relaxed) as u32)
        .opacity(0.92)
        .into()
}
