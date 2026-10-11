//! The native GPUI Automate view (the view panel's automations page) and its create/edit dialog.
//! SEE-ALSO: app/helpers/board_gxserver/automation.rs (the bridge logic it calls). It was ported
//! from the deleted React page (apps/desktop/views/project-board/automations.tsx,
//! automation-dialog.tsx and automations-drafts.ts, in git history).
mod actions;
mod detail;
mod dialog;
mod dialog_render;
mod drafts;
mod host;
mod lists;
mod model;
mod render;
mod requests;
mod style;
mod view;

pub(crate) use style::{AutomatePalette, ICON_ALERT, empty_state, icon, secondary_button};
pub(crate) use view::NativeAutomateView;
