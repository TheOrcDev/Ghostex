//! Command pane tab clicks, drags, drop feedback and drops (including workspace tabs dropped on the command pane).

use gpui::Window;

use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn set_command_drop_feedback(
        &mut self,
        feedback: Option<CommandPaneDropFeedback>,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.command_drop_feedback != feedback {
            self.command_drop_feedback = feedback;
            cx.notify();
        }
    }

    pub(crate) fn clear_command_drop_feedback(&mut self, cx: &mut gpui::Context<Self>) {
        self.set_command_drop_feedback(None, cx);
    }

    pub(crate) fn clear_command_tab_strip_feedback_for_group(
        &mut self,
        group_id: CommandPaneGroupId,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.command_drop_feedback.is_some_and(|feedback| {
            feedback.group_id == group_id
                && matches!(feedback.target, CommandPaneDropTarget::TabStrip(_))
        }) {
            self.clear_command_drop_feedback(cx);
        }
    }

    pub(crate) fn begin_pending_command_tab_click(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        expand_on_click: bool,
    ) {
        self.pending_command_tab_click = Some(CommandPanePendingTabClick {
            group_id,
            session_id,
            expand_on_click,
        });
    }

    pub(crate) fn cancel_pending_command_tab_click(&mut self) {
        self.pending_command_tab_click = None;
    }

    pub(crate) fn cancel_pending_command_tab_click_for_tab(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        expand_on_click: bool,
    ) {
        let target = CommandPanePendingTabClick {
            group_id,
            session_id,
            expand_on_click,
        };
        self.pending_command_tab_click = command_pane_tab_pending_click_after_mouse_up_out(
            self.pending_command_tab_click,
            target,
        );
    }

    pub(crate) fn handle_command_pane_tab_left_mouse_up(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        expand_on_click: bool,
        click_count: usize,
        window: &Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let target = CommandPanePendingTabClick {
            group_id,
            session_id,
            expand_on_click,
        };
        let pending_click = self.pending_command_tab_click.take();
        if command_pane_tab_left_mouse_up_focuses(
            click_count,
            pending_click,
            target,
            self.command_tab_drag_active,
            &self.command_pane,
        ) {
            self.toggle_command_pane_focus_mode_for_tab(group_id, session_id, cx);
            return;
        }
        if command_pane_tab_left_mouse_up_selects(
            pending_click,
            target,
            self.command_tab_drag_active,
        ) {
            self.select_command_pane_tab(group_id, session_id, expand_on_click, window, cx);
        }
    }

    pub(crate) fn begin_command_tab_drag(&mut self, cx: &mut gpui::Context<Self>) {
        self.pending_command_tab_click = None;
        if self.command_tab_drag_active {
            return;
        }

        self.command_tab_drag_active = true;
        self.update_active_mode_cef_child_visibility(cx);
        cx.notify();
    }

    pub(crate) fn finish_command_tab_drag_state(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        self.pending_command_tab_click = None;
        let drag_was_active = self.command_tab_drag_active;
        let changed = drag_was_active || self.command_drop_feedback.is_some();
        if !changed {
            return false;
        }

        self.command_tab_drag_active = false;
        self.command_drop_feedback = None;
        if drag_was_active {
            self.update_active_mode_cef_child_visibility(cx);
        }
        true
    }

    pub(crate) fn finish_command_tab_drag(&mut self, cx: &mut gpui::Context<Self>) {
        if self.finish_command_tab_drag_state(cx) {
            cx.notify();
        }
    }

    pub(crate) fn update_command_tab_drag_feedback(
        &mut self,
        event: &gpui::DragMoveEvent<DraggedCommandTab>,
        group_id: CommandPaneGroupId,
        tab_index: usize,
        cx: &mut gpui::Context<Self>,
    ) {
        self.begin_command_tab_drag(cx);

        let insertion_index =
            workspace_tab_insertion_index(event.bounds, event.event.position, tab_index);
        let feedback = CommandPaneDropFeedback {
            group_id,
            target: CommandPaneDropTarget::TabStrip(insertion_index),
        };
        let dragged = event.drag(cx);

        if dragged.source_group_id != group_id || !event.bounds.contains(&event.event.position) {
            if self.command_drop_feedback == Some(feedback) {
                self.clear_command_drop_feedback(cx);
            }
            return;
        }

        if !self.command_pane.tab_strip_reorder_changes_order(
            group_id,
            dragged.session_id,
            insertion_index,
        ) {
            self.clear_command_tab_strip_feedback_for_group(group_id, cx);
            return;
        }

        self.set_command_drop_feedback(Some(feedback), cx);
    }

    pub(crate) fn update_command_tab_end_drag_feedback(
        &mut self,
        event: &gpui::DragMoveEvent<DraggedCommandTab>,
        group_id: CommandPaneGroupId,
        insertion_index: usize,
        cx: &mut gpui::Context<Self>,
    ) {
        self.begin_command_tab_drag(cx);

        let feedback = CommandPaneDropFeedback {
            group_id,
            target: CommandPaneDropTarget::TabStrip(insertion_index),
        };
        let dragged = event.drag(cx);

        if dragged.source_group_id != group_id || !event.bounds.contains(&event.event.position) {
            if self.command_drop_feedback == Some(feedback) {
                self.clear_command_drop_feedback(cx);
            }
            return;
        }

        if !self.command_pane.tab_strip_reorder_changes_order(
            group_id,
            dragged.session_id,
            insertion_index,
        ) {
            self.clear_command_tab_strip_feedback_for_group(group_id, cx);
            return;
        }

        self.set_command_drop_feedback(Some(feedback), cx);
    }

    pub(crate) fn update_command_pane_drag_feedback(
        &mut self,
        event: &gpui::DragMoveEvent<DraggedCommandTab>,
        group_id: CommandPaneGroupId,
        cx: &mut gpui::Context<Self>,
    ) {
        self.begin_command_tab_drag(cx);

        if !event.bounds.contains(&event.event.position) {
            if self.command_drop_feedback.is_some_and(|feedback| {
                feedback.group_id == group_id
                    && matches!(feedback.target, CommandPaneDropTarget::PaneBody(_))
            }) {
                self.clear_command_drop_feedback(cx);
            }
            return;
        }

        let dragged = event.drag(cx);
        let dock = self
            .command_pane
            .dock_for_group(group_id)
            .unwrap_or(CommandPaneDock::Panel);
        let mut zone =
            command_pane_body_drop_zone_for_dock(dock, event.bounds, event.event.position);
        if dragged.source_group_id == group_id
            && !matches!(zone, WorkspaceDropZone::Center)
            && self
                .command_pane
                .pane_tab_count(group_id)
                .unwrap_or_default()
                <= 1
        {
            zone = WorkspaceDropZone::Center;
        }

        self.set_command_drop_feedback(
            Some(CommandPaneDropFeedback {
                group_id,
                target: CommandPaneDropTarget::PaneBody(zone),
            }),
            cx,
        );
    }

    pub(crate) fn update_workspace_tab_over_command_pane_drag_feedback(
        &mut self,
        event: &gpui::DragMoveEvent<DraggedWorkspaceTab>,
        group_id: CommandPaneGroupId,
        cx: &mut gpui::Context<Self>,
    ) {
        if !event.bounds.contains(&event.event.position) {
            if self.command_drop_feedback.is_some_and(|feedback| {
                feedback.group_id == group_id
                    && matches!(feedback.target, CommandPaneDropTarget::PaneBody(_))
            }) {
                self.clear_command_drop_feedback(cx);
            }
            return;
        }

        let dock = self
            .command_pane
            .dock_for_group(group_id)
            .unwrap_or(CommandPaneDock::Panel);
        let zone = command_pane_body_drop_zone_for_dock(dock, event.bounds, event.event.position);
        self.set_command_drop_feedback(
            Some(CommandPaneDropFeedback {
                group_id,
                target: CommandPaneDropTarget::PaneBody(zone),
            }),
            cx,
        );
    }

    pub(crate) fn update_workspace_tab_over_command_tab_drag_feedback(
        &mut self,
        event: &gpui::DragMoveEvent<DraggedWorkspaceTab>,
        group_id: CommandPaneGroupId,
        tab_index: usize,
        cx: &mut gpui::Context<Self>,
    ) {
        let insertion_index =
            workspace_tab_insertion_index(event.bounds, event.event.position, tab_index);
        self.update_workspace_tab_over_command_tab_strip_feedback(
            event,
            group_id,
            insertion_index,
            cx,
        );
    }

    pub(crate) fn update_workspace_tab_over_command_tab_end_drag_feedback(
        &mut self,
        event: &gpui::DragMoveEvent<DraggedWorkspaceTab>,
        group_id: CommandPaneGroupId,
        insertion_index: usize,
        cx: &mut gpui::Context<Self>,
    ) {
        self.update_workspace_tab_over_command_tab_strip_feedback(
            event,
            group_id,
            insertion_index,
            cx,
        );
    }

    pub(crate) fn update_workspace_tab_over_command_tab_strip_feedback(
        &mut self,
        event: &gpui::DragMoveEvent<DraggedWorkspaceTab>,
        group_id: CommandPaneGroupId,
        insertion_index: usize,
        cx: &mut gpui::Context<Self>,
    ) {
        let feedback = CommandPaneDropFeedback {
            group_id,
            target: CommandPaneDropTarget::TabStrip(insertion_index),
        };
        let dragged = event.drag(cx);
        let source_can_close = self
            .agents_workspace
            .can_transfer_tab_to_command_pane(dragged.source_pane_id, dragged.session_id);
        let target_exists = self.command_pane.find_leaf(group_id).is_some();

        if !source_can_close || !target_exists || !event.bounds.contains(&event.event.position) {
            if self.command_drop_feedback == Some(feedback) {
                self.clear_command_drop_feedback(cx);
            }
            return;
        }

        self.set_command_drop_feedback(Some(feedback), cx);
    }

    pub(crate) fn handle_command_tab_strip_drop(
        &mut self,
        group_id: CommandPaneGroupId,
        default_insertion_index: usize,
        dragged: &DraggedCommandTab,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        window.prevent_default();
        cx.stop_propagation();

        let insertion_index = match self.command_drop_feedback {
            Some(CommandPaneDropFeedback {
                group_id: feedback_group_id,
                target: CommandPaneDropTarget::TabStrip(insertion_index),
            }) if feedback_group_id == group_id => insertion_index,
            _ => default_insertion_index,
        };
        let mut changed = self.finish_command_tab_drag_state(cx);

        if dragged.source_group_id == group_id
            && self.command_pane.reorder_tab_within_group(
                group_id,
                dragged.session_id,
                insertion_index,
            )
        {
            self.scroll_command_group_active_tab(group_id);
            self.persist_shell_layout_state();
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    pub(crate) fn handle_workspace_tab_command_tab_strip_drop(
        &mut self,
        target_group_id: CommandPaneGroupId,
        default_insertion_index: usize,
        dragged: &DraggedWorkspaceTab,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:CommandPane 2026-06-22-16:18:
        Dropping an Agents tab on a command-pane tab strip uses the active command insertion marker as the exact grouping index, expands/focuses the command pane, and scrolls both the source Agents strip and inserted command strip. It is not a split/reorder path and transfers only the visible Agents title into a command placeholder.
        */
        window.prevent_default();
        cx.stop_propagation();

        let insertion_index = match self.command_drop_feedback {
            Some(CommandPaneDropFeedback {
                group_id,
                target: CommandPaneDropTarget::TabStrip(insertion_index),
            }) if group_id == target_group_id => insertion_index,
            _ => default_insertion_index,
        };
        self.command_drop_feedback = None;
        self.finish_workspace_tab_drag_state(cx);

        let source_pane_id = dragged.source_pane_id;
        let Some((inserted_group_id, _inserted_session_id)) =
            transfer_workspace_placeholder_to_command_tab_strip(
                &mut self.agents_workspace,
                &mut self.command_pane,
                dragged.source_pane_id,
                dragged.session_id,
                target_group_id,
                insertion_index,
            )
        else {
            cx.notify();
            return;
        };

        self.focus_command_pane(cx);
        self.scroll_workspace_pane_active_tab(source_pane_id);
        self.scroll_workspace_pane_active_tab(self.agents_workspace.focused_pane);
        self.scroll_command_group_active_tab(inserted_group_id);
        self.scroll_focused_command_active_tab();
        self.persist_shell_layout_state();
        self.sync_gpui_keep_awake_automation_from_current_settings(cx);
        cx.notify();
    }

    pub(crate) fn handle_command_pane_body_drop(
        &mut self,
        target_group_id: CommandPaneGroupId,
        dragged: &DraggedCommandTab,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        window.prevent_default();
        cx.stop_propagation();

        let zone = match self.command_drop_feedback {
            Some(CommandPaneDropFeedback {
                group_id,
                target: CommandPaneDropTarget::PaneBody(zone),
            }) if group_id == target_group_id => zone,
            _ => WorkspaceDropZone::Center,
        };
        self.finish_command_tab_drag_state(cx);

        let dock = self
            .command_pane
            .dock_for_group(target_group_id)
            .unwrap_or(CommandPaneDock::Panel);
        let changed = if command_pane_drop_zone_splits(dock, zone) {
            self.command_pane.split_tab_to_group(
                dragged.source_group_id,
                target_group_id,
                dragged.session_id,
                zone,
            )
        } else {
            self.command_pane.group_tab_into_group(
                dragged.source_group_id,
                target_group_id,
                dragged.session_id,
            )
        };

        if changed {
            self.focus_command_pane(cx);
            self.scroll_focused_command_active_tab();
            self.persist_shell_layout_state();
        }
        cx.notify();
    }

    pub(crate) fn handle_workspace_tab_command_pane_body_drop(
        &mut self,
        target_group_id: CommandPaneGroupId,
        dragged: &DraggedWorkspaceTab,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:CommandPane 2026-06-22-13:05:
        Dropping an Agents tab into the command pane must preserve the placeholder boundary: preflight the Agents final-root transfer guard, create only a command-pane placeholder with the visible title, then remove the Agents shell tab/session and persist layout state. Real process transfer, libghostty remounting, terminal content, command text, stdout/stderr, overlays, and hit-test routing remain deferred.

        CDXC:CommandPane 2026-06-25-19:45:
        Body drops share the rollback-capable Agents-to-command transfer helper with tab-strip drops so failed Agents source close restores prior command-pane mode, focus group, and active tabs after removing only the inserted command placeholder.
        */
        window.prevent_default();
        cx.stop_propagation();

        let zone = match self.command_drop_feedback {
            Some(CommandPaneDropFeedback {
                group_id,
                target: CommandPaneDropTarget::PaneBody(zone),
            }) if group_id == target_group_id => zone,
            _ => WorkspaceDropZone::Center,
        };
        self.command_drop_feedback = None;
        self.finish_workspace_tab_drag_state(cx);

        let Some((inserted_group_id, _inserted_session_id)) =
            transfer_workspace_placeholder_to_command_pane(
                &mut self.agents_workspace,
                &mut self.command_pane,
                dragged.source_pane_id,
                dragged.session_id,
                target_group_id,
                zone,
            )
        else {
            cx.notify();
            return;
        };

        self.focus_command_pane(cx);
        self.scroll_workspace_pane_active_tab(self.agents_workspace.focused_pane);
        self.scroll_command_group_active_tab(inserted_group_id);
        self.scroll_focused_command_active_tab();
        self.persist_shell_layout_state();
        self.sync_gpui_keep_awake_automation_from_current_settings(cx);
        cx.notify();
    }
}
