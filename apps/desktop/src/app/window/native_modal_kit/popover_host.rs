use super::*;
use gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, InteractiveElement as _, IntoElement,
    MouseDownEvent, Pixels, Styled as _, Window, div,
};
use std::rc::Rc;

/// What a modal hands a popover host: the list drawn in `owner`'s content coordinates at `frame`.
pub(crate) struct ModalPopoverRequest {
    pub(crate) owner: gpui::AnyWindowHandle,
    pub(crate) id: &'static str,
    pub(crate) frame: Bounds<Pixels>,
    pub(crate) content: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
}

/// Draws a frosted modal's open popover in a window of its own, so the list blurs the dialog
/// behind it the way the app's other frosted menus do. The app installs it
/// (`window/modal_popover_host.rs`); without it (the preview binaries) the popover draws inside
/// the modal's window as before.
#[derive(Clone)]
pub(crate) struct ModalPopoverHost {
    /// Whether popovers go to their own window right now.
    pub(crate) active: Rc<dyn Fn() -> bool>,
    pub(crate) show: Rc<dyn Fn(ModalPopoverRequest, &mut App)>,
    /// Hides the popover `id` of `owner`, if that is the one showing.
    pub(crate) hide: Rc<dyn Fn(gpui::AnyWindowHandle, &'static str, &mut App)>,
}

thread_local! {
    static MODAL_POPOVER_HOST: std::cell::RefCell<Option<ModalPopoverHost>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn install_modal_popover_host(host: ModalPopoverHost) {
    MODAL_POPOVER_HOST.with(|slot| *slot.borrow_mut() = Some(host));
}

/// The installed host, when it takes popovers now and this modal is frosted.
pub(crate) fn modal_popover_host(p: &ModalPalette) -> Option<ModalPopoverHost> {
    if !p.glass {
        return None;
    }
    MODAL_POPOVER_HOST.with(|slot| slot.borrow().clone().filter(|host| (host.active)()))
}

/// Hides popover `id` of this window if a host is showing it (every render of a closed select).
pub(crate) fn hide_hosted_modal_popover(id: &'static str, window: &Window, cx: &mut App) {
    let host = MODAL_POPOVER_HOST.with(|slot| slot.borrow().clone());
    if let Some(host) = host {
        (host.hide)(window.window_handle(), id, cx);
    }
}

/// Hands an open popover to the popover host and returns what the modal draws in its own window:
/// nothing but the outside-press listener that closes the popover (a press on the trigger is the
/// trigger's own toggle). `None` when no host takes it, so the caller draws the popover itself.
pub(crate) fn host_modal_popover<V: 'static>(
    p: &ModalPalette,
    id: &'static str,
    trigger: Bounds<Pixels>,
    frame: Bounds<Pixels>,
    content: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    on_dismiss: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    window: &Window,
    cx: &mut Context<V>,
) -> Option<AnyElement> {
    let host = modal_popover_host(p)?;
    (host.show)(
        ModalPopoverRequest {
            owner: window.window_handle(),
            id,
            frame,
            content,
        },
        cx,
    );
    Some(
        div()
            .absolute()
            .size_0()
            .on_mouse_down_out(
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    if trigger.contains(&event.position) {
                        return;
                    }
                    on_dismiss(this, window, cx);
                }),
            )
            .into_any_element(),
    )
}

/// A handler for a row drawn in a popover host's window: it runs `on_choose` on the modal entity
/// with the modal's own window, the way a click inside the modal would.
pub(crate) fn hosted_row_click<V: 'static>(
    entity: gpui::WeakEntity<V>,
    owner: gpui::AnyWindowHandle,
    index: usize,
    on_choose: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + Clone + 'static,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, _, cx| {
        let entity = entity.clone();
        let on_choose = on_choose.clone();
        let _ = owner.update(cx, move |_, window, cx| {
            let _ = entity.update(cx, |this, cx| on_choose(this, index, window, cx));
        });
    }
}
