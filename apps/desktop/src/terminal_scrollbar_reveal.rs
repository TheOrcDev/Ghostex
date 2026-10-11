//! Reveal state for the composited terminal's overlay scrollbar.
//!
//! CDXC:Terminal 2026-09-20 DECISION:
//! User: the terminal scrollbar must only appear while the viewport is actually being scrolled, the way it does on macOS, instead of staying up the whole time the pointer rests anywhere over the terminal.
//! Ghostty draws overlay scrollbars on every platform: macOS hands the surface to an `NSScrollView` with `.overlay` scrollers, and the Win32 apprt runs the four-state machine ported here (hidden -> fading in -> visible -> fading out). Scroll activity restarts a 1.5s countdown, hovering the track or dragging the thumb freezes it, and new activity during a fade-out resumes from the current alpha instead of snapping back to transparent.
//! SEE-ALSO: apps/desktop/src/terminal_element.rs owns the geometry and paints the knob with `alpha`.

use std::time::Duration;
use web_time::Instant;

/// Idle time after the last scroll activity before the bar starts to fade.
pub(crate) const AUTO_HIDE_DELAY: Duration = Duration::from_millis(1500);
/// Fade-in and fade-out duration.
pub(crate) const FADE_DURATION: Duration = Duration::from_millis(200);
/// Frame pacing while a fade is running.
const FADE_TICK: Duration = Duration::from_millis(16);
/// Re-check cadence while the pointer holds the bar open.
const HELD_TICK: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Visibility {
    Hidden,
    FadingIn,
    Visible,
    FadingOut,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScrollbarReveal {
    visibility: Visibility,
    /// Last scroll, hover or drag activity.
    last_activity: Instant,
    /// Start of the running fade, for `FadingIn` and `FadingOut`.
    fade_started: Instant,
    /// The pointer sits over the track; auto-hide is suspended.
    hovered: bool,
    /// The thumb is being dragged; auto-hide is suspended.
    dragging: bool,
}

impl Default for ScrollbarReveal {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            visibility: Visibility::Hidden,
            last_activity: now,
            fade_started: now,
            hovered: false,
            dragging: false,
        }
    }
}

impl ScrollbarReveal {
    /// The viewport moved under the user: restart the countdown, fade in from
    /// hidden, and resume a running fade-out from its current alpha.
    pub(crate) fn note_scroll_activity(&mut self, now: Instant) {
        self.last_activity = now;
        match self.visibility {
            Visibility::Hidden => {
                self.visibility = Visibility::FadingIn;
                self.fade_started = now;
            }
            Visibility::FadingOut => {
                let current = self.fade_out_alpha(now);
                self.visibility = Visibility::FadingIn;
                // Back-compute the start so alpha(now) keeps its current value
                // and the fade-in only covers the remaining range.
                let offset = FADE_DURATION.mul_f32(current);
                self.fade_started = now.checked_sub(offset).unwrap_or(now);
            }
            Visibility::FadingIn | Visibility::Visible => {}
        }
    }

    pub(crate) fn set_hovered(&mut self, hovered: bool, now: Instant) {
        if self.hovered == hovered {
            return;
        }
        self.hovered = hovered;
        self.note_hold_change(hovered, now);
    }

    pub(crate) fn set_dragging(&mut self, dragging: bool, now: Instant) {
        if self.dragging == dragging {
            return;
        }
        self.dragging = dragging;
        self.note_hold_change(dragging, now);
    }

    /// Taking hold of the bar reveals it; releasing it restarts the countdown
    /// from the moment the pointer left.
    fn note_hold_change(&mut self, held: bool, now: Instant) {
        self.last_activity = now;
        if !held {
            return;
        }
        match self.visibility {
            Visibility::Hidden => {
                self.visibility = Visibility::FadingIn;
                self.fade_started = now;
            }
            Visibility::FadingOut => self.note_scroll_activity(now),
            Visibility::FadingIn | Visibility::Visible => {}
        }
    }

    /// Advance the state machine. Returns true when the caller should redraw.
    pub(crate) fn tick(&mut self, now: Instant) -> bool {
        let previous = self.visibility;
        match self.visibility {
            Visibility::Hidden => {}
            Visibility::FadingIn => {
                if now.saturating_duration_since(self.fade_started) >= FADE_DURATION {
                    self.visibility = Visibility::Visible;
                    self.last_activity = now;
                }
            }
            Visibility::Visible => {
                if !self.hovered
                    && !self.dragging
                    && now.saturating_duration_since(self.last_activity) >= AUTO_HIDE_DELAY
                {
                    self.visibility = Visibility::FadingOut;
                    self.fade_started = now;
                }
            }
            Visibility::FadingOut => {
                if now.saturating_duration_since(self.fade_started) >= FADE_DURATION {
                    self.visibility = Visibility::Hidden;
                }
            }
        }
        // A running fade needs a redraw for every frame, not only for the
        // transitions between states.
        self.visibility != previous || self.fading()
    }

    /// How long until this state needs attention again, or `None` once the bar
    /// is hidden and nothing is holding it open.
    pub(crate) fn next_tick(&self, now: Instant) -> Option<Duration> {
        match self.visibility {
            Visibility::Hidden => None,
            Visibility::FadingIn | Visibility::FadingOut => Some(FADE_TICK),
            Visibility::Visible => Some(if self.hovered || self.dragging {
                HELD_TICK
            } else {
                AUTO_HIDE_DELAY.saturating_sub(now.saturating_duration_since(self.last_activity))
            }),
        }
    }

    pub(crate) fn alpha(&self, now: Instant) -> f32 {
        match self.visibility {
            Visibility::Hidden => 0.,
            Visibility::Visible => 1.,
            Visibility::FadingIn => (now
                .saturating_duration_since(self.fade_started)
                .as_secs_f32()
                / FADE_DURATION.as_secs_f32())
            .clamp(0., 1.),
            Visibility::FadingOut => self.fade_out_alpha(now),
        }
    }

    pub(crate) fn hidden(&self) -> bool {
        self.visibility == Visibility::Hidden
    }

    fn fading(&self) -> bool {
        matches!(
            self.visibility,
            Visibility::FadingIn | Visibility::FadingOut
        )
    }

    /// Hide immediately, without a fade: the terminal has nothing left to
    /// scroll, or the viewer is going away.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    fn fade_out_alpha(&self, now: Instant) -> f32 {
        (1. - now
            .saturating_duration_since(self.fade_started)
            .as_secs_f32()
            / FADE_DURATION.as_secs_f32())
        .clamp(0., 1.)
    }
}
