//! The window's root view: the swappable content plus what every embedded window needs.

use std::{cell::RefCell, rc::Rc, time::Instant};

use gpui::{
    AnyView, Context, IntoElement, ParentElement, Pixels, Render, Size, Styled, Task, Window,
    canvas, div,
};
use serde_json::{Value, json};

use crate::{
    EventSink, RootContent,
    perf::{FrameStats, IDLE_GAP},
    root::CommandHandler,
};

pub(crate) struct HostView {
    content: AnyView,
    on_command: CommandHandler,
    events: EventSink,
    stats: Rc<RefCell<FrameStats>>,
    idle_flush: Option<Task<()>>,
    last_viewport: Option<Size<Pixels>>,
}

impl HostView {
    pub(crate) fn new(content: RootContent, events: EventSink) -> Self {
        Self {
            content: content.view,
            on_command: content.on_command,
            events,
            stats: Rc::default(),
            idle_flush: None,
            last_viewport: None,
        }
    }

    /// One command from the host, already parsed. Host-level commands are answered here; the rest
    /// go to the root content.
    pub(crate) fn command(&mut self, command: Value, window: &mut Window, cx: &mut Context<Self>) {
        let kind = command
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match kind {
            // Round trip probe: JS stamps `sentAt`, we echo it with our own receive time.
            "ping" => self.events.emit(json!({
                "type": "pong",
                "id": command.get("id").cloned().unwrap_or(Value::Null),
                "sentAt": command.get("sentAt").cloned().unwrap_or(Value::Null),
            })),
            // One more frame even when nothing changed (the platform waits for one at a new size).
            "redraw" => window.refresh(),
            _ => {
                if !(self.on_command)(&command, window, cx) {
                    log::warn!("unknown command: {command}");
                    self.events.emit(json!({
                        "type": "error",
                        "message": format!("unknown command type {kind:?}"),
                    }));
                }
            }
        }
    }

    fn frame_started(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        if let Some(report) = self.stats.borrow_mut().frame_started(now) {
            report.log();
            self.events.emit(report.to_event());
        }
        // Report the span once it goes idle; each frame pushes the deadline out again.
        let stats = self.stats.clone();
        let events = self.events;
        self.idle_flush = Some(cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(IDLE_GAP + std::time::Duration::from_millis(20))
                .await;
            if let Some(report) = stats.borrow_mut().flush_if_idle(Instant::now()) {
                report.log();
                events.emit(report.to_event());
            }
        }));

        let viewport = window.viewport_size();
        if self.last_viewport != Some(viewport) {
            self.last_viewport = Some(viewport);
            log::info!(
                "viewport {:.1}x{:.1} at scale {}",
                f32::from(viewport.width),
                f32::from(viewport.height),
                window.scale_factor()
            );
            self.events.emit(json!({
                "type": "viewport",
                "width": f32::from(viewport.width),
                "height": f32::from(viewport.height),
                "scale": window.scale_factor(),
            }));
        }
    }
}

impl Render for HostView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.frame_started(window, cx);
        let stats = self.stats.clone();
        div()
            .size_full()
            .relative()
            .child(self.content.clone())
            // Painted after the content: marks the end of the frame's layout and paint work.
            .child(
                canvas(
                    |_, _, _| {},
                    move |_, _, _, _| stats.borrow_mut().frame_painted(Instant::now()),
                )
                .absolute()
                .size_0(),
            )
    }
}
