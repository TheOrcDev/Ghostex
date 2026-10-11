//! Split layout metrics, split handles, hover lines and resize drags for the workspace, command, browser and work area splits.

use gpui::Bounds;
use gpui::MouseDownEvent;
use gpui::MouseMoveEvent;
use gpui::MouseUpEvent;
use gpui::Pixels;
use gpui::Window;

use crate::app::consts::*;
use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn record_workspace_split_layout_metrics(
        &mut self,
        split_id: WorkspaceSplitId,
        axis: WorkspaceSplitAxis,
        child_bounds: &[Bounds<Pixels>],
    ) {
        let Some(content_span) = split_resize_content_span(child_bounds, axis) else {
            return;
        };

        self.workspace_split_layout_metrics
            .insert(split_id, SplitResizeMetrics { content_span });
    }

    pub(crate) fn record_command_split_layout_metrics(
        &mut self,
        split_id: CommandPaneSplitId,
        axis: WorkspaceSplitAxis,
        child_bounds: &[Bounds<Pixels>],
    ) {
        let Some(content_span) = split_resize_content_span(child_bounds, axis) else {
            return;
        };

        self.command_split_layout_metrics
            .insert(split_id, SplitResizeMetrics { content_span });
    }

    pub(crate) fn record_browser_split_layout_metrics(
        &mut self,
        split_id: BrowserSplitId,
        axis: WorkspaceSplitAxis,
        child_bounds: &[Bounds<Pixels>],
    ) {
        let Some(content_span) = split_resize_content_span(child_bounds, axis) else {
            return;
        };

        self.browser_split_layout_metrics
            .insert(split_id, SplitResizeMetrics { content_span });
    }

    pub(crate) fn record_workarea_split_layout_metrics(&mut self, child_bounds: &[Bounds<Pixels>]) {
        let Some(content_span) =
            split_resize_content_span(child_bounds, WorkspaceSplitAxis::Horizontal)
        else {
            return;
        };

        self.workarea_split_layout_metrics = Some(SplitResizeMetrics { content_span });
    }

    pub(crate) fn handle_workspace_split_handle_mouse_down(
        &mut self,
        split_id: WorkspaceSplitId,
        axis: WorkspaceSplitAxis,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:Workarea 2026-06-22-06:45:
        Agents workspace split handles are real two-pixel layout siblings painted in the neutral pane border colour. Dragging a horizontal handle updates the left/right split ratio and dragging a vertical handle updates the top/bottom ratio while persisting shell layout state.
        */
        window.prevent_default();
        cx.stop_propagation();

        self.workspace_split_drag = None;

        if event.click_count >= 2 {
            let reset_ratio = self
                .agents_workspace
                .equalize_split_panes(split_id, &self.workspace_split_layout_metrics);
            let cleared_hover = self.clear_workspace_split_hover_state();
            if reset_ratio {
                self.persist_shell_layout_state();
            }
            if reset_ratio || cleared_hover {
                cx.notify();
            }
            return;
        }

        let Some(start_ratio) = self.agents_workspace.split_ratio(split_id) else {
            return;
        };
        let Some(metrics) = self.workspace_split_layout_metrics.get(&split_id).copied() else {
            return;
        };
        let content_span = metrics.content_span.max(1.0);

        self.workspace_split_drag = Some(WorkspaceSplitResizeDragState {
            split_id,
            axis,
            start_position: split_resize_event_position(axis, event.position),
            start_ratio,
            content_span,
        });
        self.workspace_split_hover_epoch = self.workspace_split_hover_epoch.wrapping_add(1);
        self.workspace_split_hovering = Some(split_id);
        self.workspace_split_hover_visible = Some(split_id);
        cx.notify();
    }

    pub(crate) fn handle_workspace_split_resize_drag_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(drag) = self.workspace_split_drag else {
            return;
        };

        if !event.dragging() {
            self.finish_workspace_split_resize_drag(cx);
            return;
        }

        window.prevent_default();
        cx.stop_propagation();

        let raw_ratio = drag.start_ratio
            + (split_resize_event_position(drag.axis, event.position) - drag.start_position)
                / drag.content_span.max(1.0);
        let Some((min_ratio, max_ratio)) = self
            .agents_workspace
            .split_drag_ratio_bounds(drag.split_id, drag.content_span)
        else {
            return;
        };
        if self
            .agents_workspace
            .set_split_ratio(drag.split_id, raw_ratio.clamp(min_ratio, max_ratio))
        {
            cx.notify();
        }
    }

    pub(crate) fn handle_workspace_split_resize_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.workspace_split_drag.is_some() {
            window.prevent_default();
            cx.stop_propagation();
        }
        self.finish_workspace_split_resize_drag(cx);
    }

    pub(crate) fn finish_workspace_split_resize_drag(&mut self, cx: &mut gpui::Context<Self>) {
        if self.workspace_split_drag.take().is_some() {
            self.persist_shell_layout_state();
            cx.notify();
        }
    }

    pub(crate) fn workspace_split_hover_line_visible(&self, split_id: WorkspaceSplitId) -> bool {
        self.workspace_split_hover_visible == Some(split_id)
            || self
                .workspace_split_drag
                .is_some_and(|drag| drag.split_id == split_id)
    }

    pub(crate) fn set_workspace_split_hovering(
        &mut self,
        split_id: WorkspaceSplitId,
        hovered: bool,
        cx: &mut gpui::Context<Self>,
    ) {
        if hovered {
            if self.workspace_split_hovering == Some(split_id) {
                return;
            }

            self.workspace_split_hover_epoch = self.workspace_split_hover_epoch.wrapping_add(1);
            self.workspace_split_hovering = Some(split_id);
            self.workspace_split_hover_visible = None;
            let epoch = self.workspace_split_hover_epoch;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(SIDEBAR_DIVIDER_HOVER_DELAY)
                    .await;

                let _ = this.update(cx, |this, cx| {
                    if this.workspace_split_hover_epoch == epoch
                        && this.workspace_split_hovering == Some(split_id)
                    {
                        this.workspace_split_hover_visible = Some(split_id);
                        cx.notify();
                    }
                });
            })
            .detach();
            cx.notify();
            return;
        }

        if self.workspace_split_hovering == Some(split_id)
            || self.workspace_split_hover_visible == Some(split_id)
        {
            self.workspace_split_hover_epoch = self.workspace_split_hover_epoch.wrapping_add(1);
            self.workspace_split_hovering = None;
            self.workspace_split_hover_visible = None;
            cx.notify();
        }
    }

    pub(crate) fn clear_workspace_split_hover_state(&mut self) -> bool {
        if self.workspace_split_hovering.is_some() || self.workspace_split_hover_visible.is_some() {
            self.workspace_split_hover_epoch = self.workspace_split_hover_epoch.wrapping_add(1);
            self.workspace_split_hovering = None;
            self.workspace_split_hover_visible = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn handle_command_split_handle_mouse_down(
        &mut self,
        split_id: CommandPaneSplitId,
        axis: WorkspaceSplitAxis,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:CommandPane 2026-06-22-06:45:
        Command-pane split handles use the same real-layout resize contract as Agents splits. Dragging updates horizontal split ratios from x movement and vertical split ratios from y movement, double-click resets to 0.5, and finished mutations persist with the placeholder shell state without starting real command processes.

        CDXC:CommandPane 2026-06-27-03:38:
        Command split resize hover chrome is gesture-owned runtime state. Double-click reset must clear the split rail cursor/hover affordance and invalidate delayed hover timers even when the split ratio was already at the default, while layout persistence remains tied to an actual ratio reset.
        */
        window.prevent_default();
        cx.stop_propagation();

        self.command_split_drag = None;

        if event.click_count >= 2 {
            let reset_ratio = self.command_pane.reset_split_ratio(split_id);
            let cleared_hover = self.clear_command_resize_hover_state();
            if reset_ratio {
                self.persist_shell_layout_state();
            }
            if reset_ratio || cleared_hover {
                cx.notify();
            }
            return;
        }

        let Some(start_ratio) = self.command_pane.split_ratio(split_id) else {
            return;
        };
        let Some(metrics) = self.command_split_layout_metrics.get(&split_id).copied() else {
            return;
        };

        self.command_split_drag = Some(CommandPaneSplitResizeDragState {
            split_id,
            axis,
            start_position: split_resize_event_position(axis, event.position),
            start_ratio,
            content_span: metrics.content_span.max(1.0),
        });
        cx.notify();
    }

    pub(crate) fn handle_command_split_resize_drag_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(drag) = self.command_split_drag else {
            return;
        };

        if !event.dragging() {
            self.finish_command_split_resize_drag(cx);
            return;
        }

        window.prevent_default();
        cx.stop_propagation();

        let raw_ratio = drag.start_ratio
            + (split_resize_event_position(drag.axis, event.position) - drag.start_position)
                / drag.content_span.max(1.0);
        let Some((min_ratio, max_ratio)) = self
            .command_pane
            .split_drag_ratio_bounds(drag.split_id, drag.content_span)
        else {
            return;
        };
        if self
            .command_pane
            .set_split_ratio(drag.split_id, raw_ratio.clamp(min_ratio, max_ratio))
        {
            cx.notify();
        }
    }

    pub(crate) fn handle_command_split_resize_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.command_split_drag.is_some() {
            window.prevent_default();
            cx.stop_propagation();
        }
        self.finish_command_split_resize_drag(cx);
    }

    pub(crate) fn finish_command_split_resize_drag(&mut self, cx: &mut gpui::Context<Self>) {
        let consumed_drag = self.command_split_drag.take().is_some();
        let cleared_hover = self.clear_command_resize_hover_state();
        if consumed_drag {
            self.persist_shell_layout_state();
            cx.notify();
        } else if cleared_hover {
            cx.notify();
        }
    }

    pub(crate) fn handle_browser_split_handle_mouse_down(
        &mut self,
        split_id: BrowserSplitId,
        axis: WorkspaceSplitAxis,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:Browser 2026-06-22-09:05:
        Browser split handles use the same normal-layout resize contract as Agents workspace and command-pane splits. The visible two-pixel divider is the only grab target; dragging updates the targeted Browser split ratio from that branch's rendered first/handle/second child metrics, double-click resets to 0.5, and finished mutations persist through sanitized GPUI shell state while Browser leaf bodies keep existing tab-owned CEF surfaces keyed by BrowserTabId.
        */
        window.prevent_default();
        cx.stop_propagation();

        self.browser_split_drag = None;

        if event.click_count >= 2 {
            if self.browser_tabs.reset_split_ratio(split_id) {
                self.persist_shell_layout_state();
                cx.notify();
            }
            return;
        }

        let Some(start_ratio) = self.browser_tabs.split_ratio(split_id) else {
            return;
        };
        let Some(metrics) = self.browser_split_layout_metrics.get(&split_id).copied() else {
            return;
        };
        let content_span = metrics.content_span.max(1.0);

        self.browser_split_drag = Some(BrowserSplitResizeDragState {
            split_id,
            axis,
            start_position: split_resize_event_position(axis, event.position),
            start_ratio,
            content_span,
        });
        cx.notify();
    }

    pub(crate) fn handle_browser_split_resize_drag_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(drag) = self.browser_split_drag else {
            return;
        };

        if !event.dragging() {
            self.finish_browser_split_resize_drag(cx);
            return;
        }

        window.prevent_default();
        cx.stop_propagation();

        let raw_ratio = drag.start_ratio
            + (split_resize_event_position(drag.axis, event.position) - drag.start_position)
                / drag.content_span.max(1.0);
        let Some((min_ratio, max_ratio)) = self
            .browser_tabs
            .split_drag_ratio_bounds(drag.split_id, drag.content_span)
        else {
            return;
        };
        if self
            .browser_tabs
            .set_split_ratio(drag.split_id, raw_ratio.clamp(min_ratio, max_ratio))
        {
            cx.notify();
        }
    }

    pub(crate) fn handle_browser_split_resize_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.browser_split_drag.is_some() {
            window.prevent_default();
            cx.stop_propagation();
        }
        self.finish_browser_split_resize_drag(cx);
    }

    pub(crate) fn finish_browser_split_resize_drag(&mut self, cx: &mut gpui::Context<Self>) {
        if self.browser_split_drag.take().is_some() {
            self.persist_shell_layout_state();
            cx.notify();
        }
    }

    /// CDXC:Workarea 2026-09-20 WHY:
    /// One divider now separates the Agents column from the open view, replacing the project-editor
    /// companion's. The visible two-pixel rail is still the resize control: dragging moves the stored
    /// split ratio and a double click puts it back at the default share, both inside the clamps that
    /// keep each side above its minimum. This supersedes the 2026-06-22 companion-divider rule.
    pub(crate) fn handle_workarea_split_divider_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        window.prevent_default();
        cx.stop_propagation();

        self.workarea_split_drag = None;

        if !self.view_panel_open() {
            return;
        }

        self.set_workarea_split_divider_hovering(true, cx);

        if event.click_count >= 2 {
            let content_span = self
                .workarea_split_layout_metrics
                .map(|metrics| metrics.content_span);
            if self
                .project_editor_shell
                .reset_workarea_split_ratio(content_span)
            {
                self.persist_shell_layout_state();
                cx.notify();
            }
            return;
        }

        let Some(metrics) = self.workarea_split_layout_metrics else {
            return;
        };

        self.workarea_split_drag = Some(WorkareaSplitResizeDragState {
            start_x: event.position.x.as_f32(),
            start_ratio: self.project_editor_shell.workarea_split_ratio,
            content_span: metrics.content_span.max(1.0),
        });
        cx.notify();
    }

    pub(crate) fn handle_workarea_split_resize_drag_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(drag) = self.workarea_split_drag else {
            return;
        };

        if !event.dragging() {
            self.finish_workarea_split_resize_drag(cx);
            return;
        }

        window.prevent_default();
        cx.stop_propagation();

        let next_ratio =
            drag.start_ratio + (event.position.x.as_f32() - drag.start_x) / drag.content_span;
        if self
            .project_editor_shell
            .set_workarea_split_ratio(next_ratio, drag.content_span)
        {
            cx.notify();
        }
    }

    pub(crate) fn handle_workarea_split_resize_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.workarea_split_drag.is_some() {
            window.prevent_default();
            cx.stop_propagation();
        }
        self.finish_workarea_split_resize_drag(cx);
    }

    pub(crate) fn finish_workarea_split_resize_drag(&mut self, cx: &mut gpui::Context<Self>) {
        if self.workarea_split_drag.take().is_some() {
            self.clear_workarea_split_divider_hover_state();
            self.persist_shell_layout_state();
            cx.notify();
        }
    }

    pub(crate) fn set_workarea_split_divider_hovering(
        &mut self,
        hovered: bool,
        cx: &mut gpui::Context<Self>,
    ) {
        if hovered {
            if self.workarea_split_divider_hovering {
                return;
            }

            self.workarea_split_divider_hover_epoch =
                self.workarea_split_divider_hover_epoch.wrapping_add(1);
            self.workarea_split_divider_hovering = true;
            self.workarea_split_divider_hover_visible = false;
            let epoch = self.workarea_split_divider_hover_epoch;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(SIDEBAR_DIVIDER_HOVER_DELAY)
                    .await;

                let _ = this.update(cx, |this, cx| {
                    if this.workarea_split_divider_hover_epoch == epoch
                        && this.workarea_split_divider_hovering
                    {
                        this.workarea_split_divider_hover_visible = true;
                        cx.notify();
                    }
                });
            })
            .detach();
            cx.notify();
            return;
        }

        if self.workarea_split_divider_hovering || self.workarea_split_divider_hover_visible {
            self.clear_workarea_split_divider_hover_state();
            cx.notify();
        }
    }

    pub(crate) fn clear_workarea_split_divider_hover_state(&mut self) -> bool {
        if !self.workarea_split_divider_hovering && !self.workarea_split_divider_hover_visible {
            return false;
        }

        self.workarea_split_divider_hover_epoch =
            self.workarea_split_divider_hover_epoch.wrapping_add(1);
        self.workarea_split_divider_hovering = false;
        self.workarea_split_divider_hover_visible = false;
        true
    }
}
