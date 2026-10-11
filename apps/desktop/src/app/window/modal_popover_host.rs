//! The frosted window a native app modal's open dropdown draws in while the window is glass.
//!
//! CDXC:Theming 2026-09-27 DECISION:
//! User, of the Session Automations dialog whose Trigger list was a solid near-black box inside the frosted dialog: "yes pls make all those that could be glass glass". Under window glass an open modal dropdown draws in a frosted window of its own over the dialog, on the dialog's frosted surface, so it blurs the dialog behind it like the app's other frosted menus. The dialog keeps keyboard focus and its arrow keys, a click on a row answers in the dialog, and a press anywhere else in the dialog closes the list. Glass off, the list draws inside the dialog as before.
//!
//! CDXC:Theming 2026-09-27 WHY:
//! The searchable dropdowns (Create Worktree's branch search, the space editor's icon search) stay inside the dialog: their filter field needs keyboard focus, and a list in another window would take it from the dialog.

use std::cell::Cell;
use std::rc::Rc;

use gpui::AnyWindowHandle;

use super::frosted_host::{
    FrostedHostKind, frosted_hosting_active, hide_frosted_host, show_frosted_host,
};
use super::native_modal_kit::{ModalPopoverHost, install_modal_popover_host};

thread_local! {
    /// The dropdown the host shows: its dialog's window and its id.
    static SHOWING: Cell<Option<(AnyWindowHandle, &'static str)>> = const { Cell::new(None) };
}

/// Installs the host the native modal kit hands open dropdowns to.
pub(crate) fn install_frosted_modal_popover_host() {
    install_modal_popover_host(ModalPopoverHost {
        active: Rc::new(frosted_hosting_active),
        show: Rc::new(|request, cx| {
            SHOWING.with(|showing| showing.set(Some((request.owner, request.id))));
            show_frosted_host(
                FrostedHostKind::ModalPopover,
                request.owner,
                request.frame,
                None,
                request.content,
                None,
                cx,
            );
        }),
        hide: Rc::new(|owner, id, cx| {
            let shown = SHOWING.with(|showing| {
                let shown = showing.get() == Some((owner, id));
                if shown {
                    showing.set(None);
                }
                shown
            });
            if shown {
                hide_frosted_host(FrostedHostKind::ModalPopover, cx);
            }
        }),
    });
}

/// Forgets the dropdown of a dialog window that closed with it open, and hides it.
pub(crate) fn forget_modal_popover_over(owner: AnyWindowHandle, cx: &mut gpui::App) {
    let shown = SHOWING.with(|showing| {
        let shown = showing.get().is_some_and(|(window, _)| window == owner);
        if shown {
            showing.set(None);
        }
        shown
    });
    if shown {
        hide_frosted_host(FrostedHostKind::ModalPopover, cx);
    }
}
