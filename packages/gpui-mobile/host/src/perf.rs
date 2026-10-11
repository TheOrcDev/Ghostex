//! Frame timing, measured inside GPUI so scrolling can be judged by numbers, not by eye.
//!
//! A frame starts when the window's root view renders and its work ends when the root's last
//! child paints (layout, prepaint and paint of the whole tree; the GPU submit and present that
//! follow are not in "work" but do show up in the gap to the next frame). Frames are grouped into
//! busy spans: a gap longer than [`IDLE_GAP`] means nothing was animating, so it ends the span
//! instead of counting as a dropped frame. A span is reported every [`REPORT_EVERY`] while it
//! lasts and once more when it goes idle.

use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Longer than any frame we would call "slow" and shorter than a human pause between gestures.
pub const IDLE_GAP: Duration = Duration::from_millis(250);

pub const REPORT_EVERY: Duration = Duration::from_secs(1);

/// One 60 Hz frame; a gap above 1.5 of these missed at least one vsync.
const VSYNC_60: Duration = Duration::from_micros(16_667);

#[derive(Default)]
pub struct FrameStats {
    span_start: Option<Instant>,
    last_frame: Option<Instant>,
    frame_started: Option<Instant>,
    frames: u32,
    worst_gap: Duration,
    late_frames: u32,
    work_total: Duration,
    worst_work: Duration,
    total_frames: u64,
}

#[derive(Clone, Debug)]
pub struct FrameReport {
    pub frames: u32,
    pub span: Duration,
    pub worst_gap: Duration,
    pub late_frames: u32,
    pub avg_work: Duration,
    pub worst_work: Duration,
    pub total_frames: u64,
}

impl FrameReport {
    pub fn fps(&self) -> f64 {
        let seconds = self.span.as_secs_f64();
        if seconds <= 0.0 || self.frames < 2 {
            return 0.0;
        }
        (self.frames - 1) as f64 / seconds
    }

    pub fn to_event(&self) -> Value {
        json!({
            "type": "frameStats",
            "frames": self.frames,
            "spanMs": ms(self.span),
            "fps": (self.fps() * 10.0).round() / 10.0,
            "worstGapMs": ms(self.worst_gap),
            "lateFrames": self.late_frames,
            "avgWorkMs": ms(self.avg_work),
            "worstWorkMs": ms(self.worst_work),
            "totalFrames": self.total_frames,
        })
    }

    pub fn log(&self) {
        log::info!(
            "frames: {} in {:.0} ms ({:.1} fps), worst gap {:.1} ms, {} late, work avg {:.2} ms worst {:.2} ms",
            self.frames,
            ms(self.span),
            self.fps(),
            ms(self.worst_gap),
            self.late_frames,
            ms(self.avg_work),
            ms(self.worst_work),
        );
    }
}

fn ms(duration: Duration) -> f64 {
    (duration.as_secs_f64() * 1000.0 * 100.0).round() / 100.0
}

impl FrameStats {
    /// The root view is rendering: a new frame begins. Returns the report of a span that ended
    /// (idle gap) or filled [`REPORT_EVERY`].
    pub fn frame_started(&mut self, now: Instant) -> Option<FrameReport> {
        let mut report = None;
        if let Some(last) = self.last_frame {
            let gap = now.saturating_duration_since(last);
            if gap > IDLE_GAP {
                report = self.take_report();
            } else {
                self.worst_gap = self.worst_gap.max(gap);
                if gap.as_secs_f64() > VSYNC_60.as_secs_f64() * 1.5 {
                    self.late_frames += 1;
                }
            }
        }
        if report.is_none()
            && self
                .span_start
                .is_some_and(|start| now.saturating_duration_since(start) >= REPORT_EVERY)
        {
            report = self.take_report();
        }
        if self.span_start.is_none() {
            self.span_start = Some(now);
        }
        self.frames += 1;
        self.total_frames += 1;
        self.last_frame = Some(now);
        self.frame_started = Some(now);
        report
    }

    /// The root's last child painted: the frame's CPU work is done.
    pub fn frame_painted(&mut self, now: Instant) {
        if let Some(started) = self.frame_started.take() {
            let work = now.saturating_duration_since(started);
            self.work_total += work;
            self.worst_work = self.worst_work.max(work);
        }
    }

    /// Reports the current span if it has gone idle. Called from a timer after the last frame.
    pub fn flush_if_idle(&mut self, now: Instant) -> Option<FrameReport> {
        let last = self.last_frame?;
        if now.saturating_duration_since(last) > IDLE_GAP {
            self.take_report()
        } else {
            None
        }
    }

    fn take_report(&mut self) -> Option<FrameReport> {
        let start = self.span_start.take()?;
        let last = self.last_frame.unwrap_or(start);
        let frames = std::mem::take(&mut self.frames);
        let report = FrameReport {
            frames,
            span: last.saturating_duration_since(start),
            worst_gap: std::mem::take(&mut self.worst_gap),
            late_frames: std::mem::take(&mut self.late_frames),
            avg_work: if frames == 0 {
                Duration::ZERO
            } else {
                std::mem::take(&mut self.work_total) / frames
            },
            worst_work: std::mem::take(&mut self.worst_work),
            total_frames: self.total_frames,
        };
        self.work_total = Duration::ZERO;
        // A lone frame (a tap, one append) says nothing about smoothness.
        (frames > 1).then_some(report)
    }
}
