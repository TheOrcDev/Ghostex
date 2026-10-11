//! The Terminal view's content: the command pane's view tree drawn inside the view panel frame,
//! with the same leaves, tab bars and split handles as the Commands pane.

use gpui::AnyElement;
use gpui::InteractiveElement as _;
use gpui::IntoElement;
use gpui::ParentElement as _;
use gpui::Styled as _;
use gpui::div;

use crate::app::helpers::*;
use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn render_terminal_view_surface(&self, cx: &mut gpui::Context<Self>) -> AnyElement {
        let estimated_width = self
            .project_editor_surface_bounds_for_mode(TitlebarMode::Terminal)
            .map(|bounds| bounds.size.width.as_f32())
            .unwrap_or(WORKAREA_VIEW_PANEL_MIN_WIDTH);
        let body = match self
            .command_pane
            .focus_mode_leaf_for_dock(CommandPaneDock::View)
        {
            Some(focus_leaf) => {
                self.render_command_pane_leaf(focus_leaf, estimated_width, false, cx)
            }
            None => self.render_command_pane_node(
                &self.command_pane.view_root,
                estimated_width,
                false,
                cx,
            ),
        };
        div()
            .id("ghostex-gpui-terminal-view-surface")
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .bg(glass_clear(command_pane_chrome_color()))
            .child(body)
            .into_any_element()
    }
}
