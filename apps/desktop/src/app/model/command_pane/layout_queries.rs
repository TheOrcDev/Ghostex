use super::*;
use crate::*;

impl CommandPaneModel {
    pub(crate) fn flat_tab_ids(&self) -> Vec<(CommandPaneGroupId, CommandSessionId)> {
        let mut tabs = Vec::new();
        collect_command_tabs(&self.root, &mut tabs);
        collect_command_tabs(&self.view_root, &mut tabs);
        tabs
    }

    pub(crate) fn pane_owner_session_ids(&self) -> HashSet<(CommandPaneGroupId, CommandSessionId)> {
        /*
        CDXC:SessionSleep 2026-06-27-06:53:
        Native Auto Sleep protects the selected owner of each visible command-panel split leaf, while HUD focus remains responder-exact. Derive this set from explicit pane-layout active tabs, not from focused_group fallback, so split siblings can stay protected without becoming `isActive`.
        */
        self.group_order()
            .into_iter()
            .filter_map(|group_id| {
                let leaf = self.find_leaf(group_id)?;
                self.visible_command_body_owner_for_leaf(leaf)
                    .map(|owner| (owner.group_id, owner.session_id))
            })
            .collect()
    }

    pub(crate) fn sidebar_command_session_sources(
        &self,
        command_pane_focused: bool,
        delayed_send_timers: &HashMap<CommandSessionId, GpuiCommandDelayedSendTimer>,
        close_after_done_timers: &HashMap<CommandSessionId, GpuiCommandCloseAfterDoneTimer>,
        now: SystemTime,
    ) -> serde_json::Value {
        /*
        CDXC:CommandPane 2026-06-25-10:50:
        GPUI Sidebar command-session indicators need the same live command-pane session matching as macOS. Export only sanitized command-pane summary fields: external `G{u64}` session ids, normalized title, lifecycle-style HUD status, and focused-tab boolean. Do not include command text, cwd, env, status-file paths, terminal output, shell-state JSON, or project paths.

        CDXC:SessionSleep 2026-06-25-14:27:
        Sleeping command tabs stay represented in the GPUI sidebar bridge with a boolean lifecycle marker while their command activity is idle. Keep the bridge sanitized to ids, normalized title, enum status, focus, action command id, and isSleeping only.

        CDXC:DelayedSend 2026-06-25-17:09:
        Native projects Delayed Send and Close After Done timer state into sidebar/titlebar terminal rows. GPUI command indicators should carry only the same safe timer fields: armed booleans, UTC deadlines, remaining labels, and remaining milliseconds. Keep command text, terminal output, paths, run ids, status files, titles beyond the visible sanitized label, and shell-state JSON out of this bridge.

        CDXC:FocusRouting 2026-06-26-04:15:
        Sidebar and app-modal commandSessionIndicator active state mirrors responder-exact command focus. Mark a command tab active only when shell focus is in the command pane and `focused_group` still resolves to a live command group; stale command focus and non-command focus must export every indicator with isActive=false instead of falling back to the first command group.

        CDXC:SessionStatus 2026-06-27-06:30:
        Native HUD status is terminal lifecycle-derived: running terminals are running, error lifecycle is error, and non-running lifecycle is idle. GPUI local command tabs currently expose only awake/sleeping lifecycle, so project awake tabs as running and sleeping tabs as idle; Action Attention remains separate button feedback and must not make HUD sessions error.

        CDXC:SessionSleep 2026-06-27-06:53:
        `isActive` is reserved for responder/focused-HUD state. Export `isPaneOwner` separately from the command layout owner set so native Auto Sleep can protect every active split owner without treating unfocused split siblings as HUD-active.
        */
        let active = if command_pane_focused {
            self.focused_group_active_session_id()
        } else {
            None
        };
        let pane_owner_session_ids = self.pane_owner_session_ids();
        serde_json::Value::Array(
            self.flat_tab_ids()
                .into_iter()
                .filter_map(|(group_id, session_id)| {
                    let session = self.session(session_id)?;
                    let title = gpui_command_pane_sidebar_indicator_text(&session.title)?;
                    let mut summary = serde_json::json!({
                        "isActive": active == Some((group_id, session_id)),
                        "isSleeping": session.is_sleeping,
                        "sessionId": gpui_command_session_external_id(session.id),
                        "status": session.sidebar_hud_indicator_status(),
                        "title": title,
                    });
                    if pane_owner_session_ids.contains(&(group_id, session_id)) {
                        summary["isPaneOwner"] = serde_json::json!(true);
                    }
                    if let Some(command_id) = session
                        .action_command_id
                        .as_deref()
                        .and_then(gpui_command_pane_sidebar_indicator_text)
                    {
                        summary["commandId"] = serde_json::json!(command_id);
                    }
                    if let Some(timer) = delayed_send_timers.get(&session_id).copied() {
                        let remaining_ms = timer.remaining_ms(now);
                        summary["delayedSendDeadlineAt"] =
                            serde_json::json!(gpui_iso8601_utc(timer.deadline_at));
                        summary["delayedSendRemainingLabel"] = serde_json::json!(
                            gpui_command_delayed_send_countdown_label(remaining_ms,)
                        );
                        summary["delayedSendRemainingMs"] = serde_json::json!(remaining_ms);
                    }
                    if session.close_after_done_armed {
                        summary["closeAfterDone"] = serde_json::json!(true);
                        if let Some(timer) = close_after_done_timers.get(&session_id).copied() {
                            let remaining_ms = timer.remaining_ms(now);
                            summary["closeAfterDoneDeadlineAt"] =
                                serde_json::json!(gpui_iso8601_utc(timer.deadline_at));
                            summary["closeAfterDoneRemainingLabel"] = serde_json::json!(
                                gpui_command_delayed_send_countdown_label(remaining_ms,)
                            );
                            summary["closeAfterDoneRemainingMs"] = serde_json::json!(remaining_ms);
                        }
                    }
                    Some(summary)
                })
                .collect(),
        )
    }

    pub(crate) fn group_order(&self) -> Vec<CommandPaneGroupId> {
        let mut group_ids = Vec::new();
        collect_command_leaf_ids(&self.root, &mut group_ids);
        collect_command_leaf_ids(&self.view_root, &mut group_ids);
        group_ids
    }

    pub(crate) fn visible_command_body_owner_for_leaf(
        &self,
        leaf: &CommandPaneLeaf,
    ) -> Option<CommandPaneVisibleBodyOwner> {
        /*
        CDXC:Terminal 2026-06-27-04:36:
        GPUI command-pane body ownership mirrors native `visibleCommandPaneOwnerSessionIds`: an expanded command group gives its visible body to the stored selected command tab only when that exact tab still has a stored session. Sleeping selected tabs own a placeholder body without a Ghostty mount slot, and stale active ids must not fall back to sibling tabs.
        */
        if !self.group_dock_visible(leaf.group_id) {
            return None;
        }

        let session_id = leaf.tab_group.active_session;
        if !leaf.tab_group.has_session(session_id) {
            return None;
        }

        let session = self.session(session_id)?;
        Some(CommandPaneVisibleBodyOwner {
            group_id: leaf.group_id,
            session_id,
            is_sleeping: session.is_sleeping,
        })
    }

    pub(crate) fn rendered_terminal_body_mount_slots(&self) -> Vec<CommandTerminalBodyMountSlotId> {
        /*
        CDXC:Terminal 2026-06-23-05:03:
        Real command-pane terminal bodies are limited to the expanded command pane and the active tab in each visible command group. Inactive command tabs, collapsed strip tabs, missing sessions, and command titles/status are intentionally excluded so command Ghostty surfaces stay body-bounds-driven and runtime-only.

        CDXC:SessionSleep 2026-06-25-14:27:
        Sleeping command tabs remain in the tab/group model but are not renderable body mount slots. Withhold their command terminal body until an explicit body activation wakes the session.

        CDXC:FocusMode 2026-06-25-21:40:
        Command Focus mode filters the mounted/rendered command body slots to the focused command group only after eligibility is computed from the full split tree. This preserves the reversible command split layout while preventing hidden command groups from retaining native terminal hosts or Ghostty focus.

        CDXC:Terminal 2026-06-27-04:36:
        Rendered command mount slots are the non-sleeping subset of explicit visible command body owners. Sleeping owners remain visible placeholders, while missing sessions, stale selected ids, inactive siblings, and collapsed panes produce no Ghostty host slot.
        */
        let slots = self.rendered_terminal_body_mount_slots_without_focus();
        match self.focus_mode_group {
            Some(focus_group_id)
                if self.focus_mode_eligible_group_count_for_group(focus_group_id) > 1
                    && slots
                        .iter()
                        .any(|slot_id| slot_id.group_id == focus_group_id) =>
            {
                // Focus mode zooms one group inside its own dock; the other dock's groups are
                // untouched.
                let focus_dock = self.dock_for_group(focus_group_id);
                slots
                    .into_iter()
                    .filter(|slot_id| {
                        slot_id.group_id == focus_group_id
                            || self.dock_for_group(slot_id.group_id) != focus_dock
                    })
                    .collect()
            }
            _ => slots,
        }
    }

    pub(crate) fn is_current_terminal_body_mount_slot(
        &self,
        slot_id: CommandTerminalBodyMountSlotId,
    ) -> bool {
        self.rendered_terminal_body_mount_slots()
            .into_iter()
            .any(|current_slot_id| current_slot_id == slot_id)
    }

    pub(crate) fn pane_tab_count(&self, group_id: CommandPaneGroupId) -> Option<usize> {
        self.find_leaf(group_id)
            .map(|leaf| leaf.tab_group.tabs.len())
    }

    pub(crate) fn rendered_terminal_body_mount_slots_without_focus(
        &self,
    ) -> Vec<CommandTerminalBodyMountSlotId> {
        if !self.any_dock_visible() {
            return Vec::new();
        }

        self.group_order()
            .into_iter()
            .filter_map(|group_id| {
                let leaf = self.find_leaf(group_id)?;
                self.terminal_body_mount_slot_for_leaf(leaf)
            })
            .collect()
    }

    pub(crate) fn terminal_body_mount_slot_for_leaf(
        &self,
        leaf: &CommandPaneLeaf,
    ) -> Option<CommandTerminalBodyMountSlotId> {
        /*
        CDXC:Terminal 2026-06-27-04:36:
        Command-pane Ghostty slots are derived from the visible body-owner helper, not from tab-group fallback selection. A selected non-sleeping command tab may mount a blank pending terminal body; sleeping selected tabs, missing selected sessions, stale active ids, inactive siblings, and collapsed panes must not borrow a mount slot.
        */
        self.visible_command_body_owner_for_leaf(leaf)
            .and_then(CommandPaneVisibleBodyOwner::mount_slot_id)
    }

    /// How many groups in `group_id`'s dock have a rendered body: Focus mode is a per-dock zoom.
    pub(crate) fn focus_mode_eligible_group_count_for_group(
        &self,
        group_id: CommandPaneGroupId,
    ) -> usize {
        let dock = self.dock_for_group(group_id);
        self.rendered_terminal_body_mount_slots_without_focus()
            .into_iter()
            .filter(|slot_id| self.dock_for_group(slot_id.group_id) == dock)
            .count()
    }

    pub(crate) fn group_is_focus_mode_eligible_without_focus(
        &self,
        group_id: CommandPaneGroupId,
    ) -> bool {
        self.rendered_terminal_body_mount_slots_without_focus()
            .into_iter()
            .any(|slot_id| slot_id.group_id == group_id)
    }

    pub(crate) fn clear_focus_mode_if_invalid(&mut self) -> bool {
        let Some(focus_group_id) = self.focus_mode_group else {
            return false;
        };
        if self.focus_mode_eligible_group_count_for_group(focus_group_id) <= 1
            || !self.group_is_focus_mode_eligible_without_focus(focus_group_id)
        {
            self.focus_mode_group = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn tab_context_allows_focus_mode(
        &self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
    ) -> bool {
        /*
        CDXC:ContextMenus 2026-06-25-21:29:
        Native command-tab Focus is split-owner Focus mode, not tab selection or command-pane keyboard focus. GPUI allows the row only when the clicked tab belongs to a group with a rendered awake owner and the command pane has more than one rendered awake owner; one command group with multiple tabs does not qualify.
        */
        let Some(leaf) = self.find_leaf(group_id) else {
            return false;
        };
        if !leaf.tab_group.has_session(session_id) {
            return false;
        }

        self.focus_mode_eligible_group_count_for_group(group_id) > 1
            && self.group_is_focus_mode_eligible_without_focus(group_id)
    }

    pub(crate) fn tab_context_focus_row_index(
        &self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
    ) -> Option<usize> {
        self.tab_context_allows_focus_mode(group_id, session_id)
            .then(|| command_pane_tab_context_runtime_action_count(self, group_id, session_id))
    }

    pub(crate) fn tab_strip_reorder_indices(
        &self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        insertion_index: usize,
    ) -> Option<(usize, usize)> {
        /*
        CDXC:CommandPane 2026-06-25-19:57:
        Native same-group command tab-strip drops interpret the marker index before removing the dragged tab. Adjust forward moves by one after removal, and classify both same-index and adjacent same-slot markers as no-ops so persistence and reorder notifications only represent real user-visible order changes.
        */
        let leaf = self.find_leaf(group_id)?;
        let source_index = leaf
            .tab_group
            .tabs
            .iter()
            .position(|tab| tab.session_id == session_id)?;
        let bounded_insertion_index = insertion_index.min(leaf.tab_group.tabs.len());
        let final_index = if bounded_insertion_index > source_index {
            bounded_insertion_index - 1
        } else {
            bounded_insertion_index
        };
        Some((source_index, final_index))
    }

    pub(crate) fn tab_strip_reorder_changes_order(
        &self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        insertion_index: usize,
    ) -> bool {
        self.tab_strip_reorder_indices(group_id, session_id, insertion_index)
            .is_some_and(|(source_index, final_index)| final_index != source_index)
    }
}
