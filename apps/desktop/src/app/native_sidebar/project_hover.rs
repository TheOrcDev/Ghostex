use std::{cell::RefCell, rc::Rc};

use gpui::{
    DispatchPhase, Entity, Hitbox, HitboxBehavior, IntoElement, MouseMoveEvent, Styled, Window,
    canvas,
};

use crate::GhostexGpuiApp;

/// CDXC:Projects 2026-09-19 WHY:
/// GPUI `on_hover` reports "not hovered" while a mouse button is held, so pressing the project header's last-used agent or Select Agent button and moving by a pixel cleared the hovered header mid-click; its buttons vanished and the release landed on the header, which toggled the project instead of launching.
/// Every rendered header (the list row and its sticky copy, which share a group id) gets a non-interactive hit-test probe, and the sidebar resolves the hovered header from the frame's hit test on every mouse move, like the session cards (session_hover.rs).
#[derive(Clone, Default)]
pub(crate) struct ProjectHeaderHoverProbes(Rc<RefCell<Vec<(String, Hitbox)>>>);

impl ProjectHeaderHoverProbes {
    /// A hitbox over the header it is a child of; it blocks nothing.
    pub(super) fn probe(&self, group_id: String) -> impl IntoElement {
        let probes = self.0.clone();
        canvas(
            move |bounds, window, _| {
                let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                probes.borrow_mut().push((group_id, hitbox));
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0()
    }

    pub(super) fn track(self, view: Entity<GhostexGpuiApp>, window: &mut Window) {
        window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble {
                return;
            }
            let hovered = if cx.has_active_drag() {
                None
            } else {
                self.0
                    .borrow()
                    .iter()
                    .find(|(_, hitbox)| hitbox.is_hovered(window))
                    .map(|(id, _)| id.clone())
            };
            view.update(cx, |app, cx| {
                if app.native_sidebar.hovered_group != hovered {
                    app.native_sidebar.hovered_group = hovered;
                    cx.notify();
                }
            });
        });
    }
}
