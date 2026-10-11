//! Agents and command terminal mount slot focus, zmx persistence refreshes and mouse forwarding.

use std::time::Duration;

use gpui::Modifiers;
use gpui::MouseButton;
use gpui::Pixels;
use gpui::Point;
use gpui::PressureStage;
use gpui::ScrollDelta;
use gpui::Window;

use crate::app::helpers::*;
use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn focus_agents_terminal_mount_slot(
        &mut self,
        slot_id: AgentsTerminalBodyMountSlotId,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:FocusRouting 2026-06-22-23:11:
        Running terminal body clicks are focus handoffs, not placeholder activation. Keep the existing body div as the click/drop owner, update the shell focus to the clicked Agents pane, and force one AppKit first-responder handoff for the mounted real surface without adding overlays, hit-test routing, synthetic input routing, process lifecycle, or persistence fields.
        */
        if !self
            .agents_workspace
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return;
        }

        let gpui_engine_view = self
            .agents_gpui_engine_terminals
            .get(&slot_id.session_id)
            .map(|record| record.view.clone());
        if let Some(view) = gpui_engine_view.as_ref() {
            // Reclaim AppKit and clear explicit CEF ownership before sidebar
            // selection notifications can run renderer work during this same
            // mouse event. The terminal FocusHandle remains the final GPUI
            // keyboard target throughout the model updates below.
            self.focus_gpui_engine_terminal_view(
                GpuiEngineTerminalEventTarget::Agents(slot_id.session_id),
                view,
                window,
                cx,
            );
        }

        let workspace_focus_changed = self.agents_workspace.focused_pane != slot_id.pane_id;
        self.agents_workspace.focus_pane(slot_id.pane_id);
        self.dispatch_gpui_workspace_session_attention_acknowledge(slot_id.session_id, cx);
        self.dispatch_gpui_workspace_active_session_selected(slot_id.pane_id, cx);
        let attention_acknowledged = self
            .agents_workspace
            .acknowledge_attention_for_session_activation(slot_id.session_id);
        let focus = ShellFocusTarget::AgentsPane(slot_id.pane_id);
        let shell_focus_changed = self.shell_focus != focus;
        self.focus_shell_target_now(focus, window, cx);
        if workspace_focus_changed || shell_focus_changed || attention_acknowledged {
            self.scroll_workspace_pane_active_tab(slot_id.pane_id);
            self.persist_shell_layout_state();
            cx.notify();
        }
    }

    /*
    CDXC:Zmx 2026-07-06:
    Clicking terminal content, including an already-focused pane, is an explicit
    opportunity to recover from a zmx daemon grid that another client changed.
    Mirrors macOS `refreshZmxPersistenceTerminalIfNeeded(mode: .ifStale)`: use
    zmx's conditional grid-size refresh for click-originated requests only, so a
    normal click inside an already-correct pane never repaints the terminal or
    scrolls it to the visible bottom.
    */
    pub(crate) fn refresh_zmx_persistence_agents_terminal_if_stale(
        &self,
        slot_id: AgentsTerminalBodyMountSlotId,
        cx: &gpui::Context<Self>,
    ) {
        if !self
            .agents_workspace
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return;
        }
        // A parked or just-surfacing engine grid is the zmx resting width,
        // not a displayed size (CDXC:Terminal 2026-09-03).
        if !self.agents_gpui_engine_terminal_zmx_grid_is_displayed(slot_id.session_id, cx) {
            return;
        }
        let session_name = self
            .agents_workspace
            .session(slot_id.session_id)
            .and_then(|session| session.zmx_session_name.clone());
        gpui_spawn_zmx_refresh_if_stale_process(
            session_name,
            self.agents_terminal_refresh_grid_size(slot_id, cx),
            "agentsTerminalContentMouseDown",
        );
    }

    pub(crate) fn refresh_zmx_persistence_command_terminal_if_stale(
        &self,
        slot_id: CommandTerminalBodyMountSlotId,
        cx: &gpui::Context<Self>,
    ) {
        if !self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return;
        }
        let session_name = self
            .command_pane
            .session(slot_id.session_id)
            .and_then(|session| session.zmx_session_name.clone());
        let grid_size = {
            #[cfg(target_os = "macos")]
            let native_size = self
                .command_terminal_ghostty_surfaces
                .get(&slot_id)
                .map(|surface| {
                    let size = surface.surface_size();
                    (size.rows, size.columns)
                });
            #[cfg(not(target_os = "macos"))]
            let native_size = None;
            native_size.or_else(|| {
                self.command_gpui_engine_terminals
                    .get(&slot_id.session_id)
                    .map(|record| {
                        let (columns, rows) = record.view.read(cx).model().size();
                        (rows, columns)
                    })
            })
        };
        gpui_spawn_zmx_refresh_if_stale_process(
            session_name,
            grid_size,
            "commandTerminalContentMouseDown",
        );
    }

    pub(crate) fn agents_terminal_refresh_grid_size(
        &self,
        slot_id: AgentsTerminalBodyMountSlotId,
        cx: &gpui::Context<Self>,
    ) -> Option<(u16, u16)> {
        #[cfg(target_os = "macos")]
        if let Some(surface) = self.agents_terminal_ghostty_surfaces.get(&slot_id) {
            let size = surface.surface_size();
            return Some((size.rows, size.columns));
        }
        self.agents_gpui_engine_terminals
            .get(&slot_id.session_id)
            .map(|record| {
                let (columns, rows) = record.view.read(cx).model().size();
                (rows, columns)
            })
    }

    /*
    CDXC:Zmx 2026-07-06:
    Mirrors macOS `scheduleZmxPersistenceTerminalRefreshAfterResize`: split,
    sidebar, companion-ratio, and window resizes re-arm a trailing-edge 0.8s
    debounce, and the settled pass refreshes every surfaced zmx pane. Unlike
    macOS the settled pass stays size-gated through `refresh-if-stale` instead
    of the unconditional repaint sequence, so a pane whose grid already matches
    never repaints or scrolls.
    */
    pub(crate) fn schedule_zmx_persistence_refresh_after_resize(
        &mut self,
        cx: &mut gpui::Context<Self>,
    ) {
        self.zmx_persistence_resize_refresh_generation = self
            .zmx_persistence_resize_refresh_generation
            .wrapping_add(1);
        let generation = self.zmx_persistence_resize_refresh_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(800))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.zmx_persistence_resize_refresh_generation != generation {
                    return;
                }
                this.refresh_zmx_persistence_surfaced_terminals_if_stale(cx);
            });
        })
        .detach();
    }

    /*
    CDXC:Zmx 2026-07-06:
    Mirrors macOS `zmxPersistenceTerminalSessionIdsForSurfacedPanes`: the visible Agents
    mount slots refresh, and rendered command-pane slots always join. Hidden panes must
    never refresh because `refresh-if-stale` conforms the daemon grid to the passed size,
    and a hidden pane's stale size would fight the surfaced owner.
    */
    pub(crate) fn refresh_zmx_persistence_surfaced_terminals_if_stale(
        &self,
        cx: &gpui::Context<Self>,
    ) {
        if self.agents_workspace_visible() {
            for slot_id in self.agents_workspace.rendered_terminal_body_mount_slots() {
                self.refresh_zmx_persistence_agents_terminal_if_stale(slot_id, cx);
            }
        }
        for slot_id in self.command_pane.rendered_terminal_body_mount_slots() {
            self.refresh_zmx_persistence_command_terminal_if_stale(slot_id, cx);
        }
    }

    pub(crate) fn current_zmx_persistence_focused_terminal_slot(
        &self,
    ) -> Option<ZmxPersistenceFocusedTerminalSlot> {
        match self.shell_focus {
            ShellFocusTarget::AgentsPane(pane_id) => {
                if !self.agents_workspace_visible() {
                    return None;
                }
                let session_id = self.agents_workspace.active_session_in_pane(pane_id)?;
                let slot_id = AgentsTerminalBodyMountSlotId {
                    pane_id,
                    session_id,
                };
                self.agents_workspace
                    .is_current_terminal_body_mount_slot(slot_id)
                    .then_some(ZmxPersistenceFocusedTerminalSlot::Agents(slot_id))
            }
            ShellFocusTarget::CommandPane => {
                let (group_id, session_id) = self.command_pane.focused_group_active_session_id()?;
                let slot_id = CommandTerminalBodyMountSlotId {
                    group_id,
                    session_id,
                };
                self.command_pane
                    .is_current_terminal_body_mount_slot(slot_id)
                    .then_some(ZmxPersistenceFocusedTerminalSlot::Command(slot_id))
            }
            _ => None,
        }
    }

    /*
    CDXC:Zmx 2026-07-06:
    Mirrors macOS `refreshZmxPersistenceTerminalIfFocusOrSurfaceChanged` for
    non-click focus movement (keyboard pane navigation, sidebar focus routing,
    programmatic focus). Render-start change detection covers every focus call
    site without threading the refresh through each one; the conditional
    refresh makes redundant firing after click focus a size-matched no-op.
    */
    pub(crate) fn refresh_zmx_persistence_focused_terminal_if_changed(
        &mut self,
        cx: &gpui::Context<Self>,
    ) {
        let focused = self.current_zmx_persistence_focused_terminal_slot();
        if self.zmx_persistence_last_focused_terminal_slot == focused {
            return;
        }
        self.zmx_persistence_last_focused_terminal_slot = focused;
        match focused {
            Some(ZmxPersistenceFocusedTerminalSlot::Agents(slot_id)) => {
                self.refresh_zmx_persistence_agents_terminal_if_stale(slot_id, cx);
            }
            Some(ZmxPersistenceFocusedTerminalSlot::Command(slot_id)) => {
                self.refresh_zmx_persistence_command_terminal_if_stale(slot_id, cx);
            }
            None => {}
        }
    }

    pub(crate) fn forward_agents_terminal_mount_slot_mouse_position(
        &mut self,
        slot_id: AgentsTerminalBodyMountSlotId,
        window_position: Point<Pixels>,
        modifiers: Modifiers,
    ) -> bool {
        /*
        CDXC:Terminal 2026-06-23-08:32:
        Mounted Running Agents bodies may forward only sanitized body-relative pointer position plus mapped keyboard modifier bits to the exact current Ghostty owner. Mouse movement uses the body event's GPUI modifiers and no capture, selection, drag, paste, keyboard, IME, logging, persistence, or coordinate routing outside the body event itself.
        */
        if !self
            .agents_workspace
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let Some(position) = agents_terminal_body_relative_mouse_position_for_slot(
            &self.agents_terminal_mount_slot_bounds,
            slot_id,
            window_position,
        ) else {
            return false;
        };
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.agents_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id {
                return false;
            }

            surface.mouse_pos(position.x, position.y, mouse_mods);
            true
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (position, mouse_mods);
            false
        }
    }

    pub(crate) fn forward_agents_terminal_mount_slot_mouse_button(
        &mut self,
        slot_id: AgentsTerminalBodyMountSlotId,
        window_position: Point<Pixels>,
        action: ghostty_kit::ffi::ghostty_input_mouse_state_e,
        button: MouseButton,
        modifiers: Modifiers,
    ) -> bool {
        /*
        CDXC:Terminal 2026-06-23-10:23:
        Mounted Running Agents button press/release forwarding uses the existing current-slot and body-boundary gates, verifies the mounted surface owner and runtime identity, then updates pointer position and sends the mapped left/right/middle Ghostty button value. GPUI navigation buttons no-op before position forwarding so parity does not create stored button state or fallback routing.
        */
        let Some(ghostty_button) = ghostty_mouse_button_from_gpui_button(button) else {
            return false;
        };
        if !self
            .agents_workspace
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let Some(position) = agents_terminal_body_relative_mouse_position_for_slot(
            &self.agents_terminal_mount_slot_bounds,
            slot_id,
            window_position,
        ) else {
            return false;
        };
        let Some(runtime_session_id) = self
            .agents_terminal_runtime_sessions
            .runtime_session_id_for_shell_session(slot_id.session_id)
        else {
            return false;
        };
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.agents_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id
                || surface.runtime_session_id() != runtime_session_id
            {
                return false;
            }

            surface.mouse_pos(position.x, position.y, mouse_mods);
            surface.mouse_button(action, ghostty_button, mouse_mods)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (
                position,
                runtime_session_id,
                action,
                ghostty_button,
                mouse_mods,
            );
            false
        }
    }

    pub(crate) fn forward_agents_terminal_mount_slot_mouse_release_outside(
        &mut self,
        slot_id: AgentsTerminalBodyMountSlotId,
        button: MouseButton,
        modifiers: Modifiers,
    ) -> bool {
        /*
        CDXC:Terminal 2026-06-23-10:23:
        Mounted Agents mouse-up-out remains capture recovery only. Require the current slot and exact Ghostty owner runtime identity to still match, require Ghostty mouse capture, then send the mapped left/right/middle release with mapped modifiers without updating mouse_pos, storing last positions, or synthesizing outside coordinates.
        */
        if !self
            .agents_workspace
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let Some(runtime_session_id) = self
            .agents_terminal_runtime_sessions
            .runtime_session_id_for_shell_session(slot_id.session_id)
        else {
            return false;
        };
        let Some(ghostty_button) = ghostty_mouse_button_from_gpui_button(button) else {
            return false;
        };
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.agents_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id
                || surface.runtime_session_id() != runtime_session_id
                || !surface.mouse_captured()
            {
                return false;
            }

            surface.mouse_button(
                ghostty_kit::ffi::GHOSTTY_MOUSE_RELEASE,
                ghostty_button,
                mouse_mods,
            )
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (runtime_session_id, ghostty_button, mouse_mods);
            false
        }
    }

    pub(crate) fn forward_agents_terminal_mount_slot_mouse_pressure(
        &mut self,
        slot_id: AgentsTerminalBodyMountSlotId,
        window_position: Point<Pixels>,
        stage: PressureStage,
        pressure: f32,
        modifiers: Modifiers,
    ) -> bool {
        /*
        CDXC:Terminal 2026-06-23-09:51:
        Mounted Running Agents body pressure is forwarded only from the exact current body mount slot. Require recorded body bounds and the matching Ghostty surface owner, update the body-relative pointer position with mapped modifiers first, then pass the mapped pressure stage and raw GPUI pressure value without capture, selection, paste, keyboard, IME, logging, persistence, overlays, or routing.
        */
        if !self
            .agents_workspace
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let Some(position) = agents_terminal_body_relative_mouse_position_for_slot(
            &self.agents_terminal_mount_slot_bounds,
            slot_id,
            window_position,
        ) else {
            return false;
        };
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);
        let pressure_stage = ghostty_mouse_pressure_stage_from_gpui_stage(stage);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.agents_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id {
                return false;
            }

            surface.mouse_pos(position.x, position.y, mouse_mods);
            surface.mouse_pressure(pressure_stage, f64::from(pressure));
            true
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (position, mouse_mods, pressure_stage, pressure);
            false
        }
    }

    pub(crate) fn forward_agents_terminal_mount_slot_mouse_scroll(
        &mut self,
        slot_id: AgentsTerminalBodyMountSlotId,
        window_position: Point<Pixels>,
        delta: ScrollDelta,
        modifiers: Modifiers,
    ) -> bool {
        if !self
            .agents_workspace
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let Some(position) = agents_terminal_body_relative_mouse_position_for_slot(
            &self.agents_terminal_mount_slot_bounds,
            slot_id,
            window_position,
        ) else {
            return false;
        };
        let (scroll_x, scroll_y, scroll_mods) = terminal_ghostty_scroll_delta(delta);
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.agents_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id {
                return false;
            }

            surface.mouse_pos(position.x, position.y, mouse_mods);
            surface.mouse_scroll(scroll_x, scroll_y, scroll_mods);
            true
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (position, scroll_x, scroll_y, scroll_mods, mouse_mods);
            false
        }
    }

    pub(crate) fn focus_command_terminal_mount_slot(
        &mut self,
        slot_id: CommandTerminalBodyMountSlotId,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:Terminal 2026-06-23-09:41:
        Mounted command terminal body clicks are command-pane focus handoffs before input forwarding. Accept only the current command body mount slot, focus that command group, keep shell focus on `CommandPane`, force the existing AppKit terminal handoff path, persist the same shell focus state as placeholder clicks, and avoid new persisted fields or hit-test routing.

        CDXC:FocusRouting 2026-06-26-00:00:
        Mounted command terminal body focus must reveal the active command tab in both the expanded group strip and collapsed strip, matching native `focusTerminal(...)->revealActivePaneTab` while leaving mouse forwarding and terminal handoff ownership on the body element.
        */
        if !self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return;
        }

        if !self.command_pane.focus_group(slot_id.group_id) {
            return;
        }
        let attention_acknowledged = self
            .command_pane
            .acknowledge_attention_for_session_activation(slot_id.session_id);
        let gpui_engine_view = self
            .command_gpui_engine_terminals
            .get(&slot_id.session_id)
            .map(|record| record.view.clone());
        if let Some(view) = gpui_engine_view.as_ref() {
            self.focus_gpui_engine_terminal_view(
                GpuiEngineTerminalEventTarget::Command(slot_id.session_id),
                view,
                window,
                cx,
            );
        }
        self.remember_current_non_command_focus();
        self.focus_shell_target_now(ShellFocusTarget::CommandPane, window, cx);
        self.scroll_command_group_active_tab(slot_id.group_id);
        self.persist_shell_layout_state();
        if attention_acknowledged {
            self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        }
        cx.notify();
    }

    pub(crate) fn forward_command_terminal_mount_slot_mouse_position(
        &mut self,
        slot_id: CommandTerminalBodyMountSlotId,
        window_position: Point<Pixels>,
        modifiers: Modifiers,
    ) -> bool {
        /*
        CDXC:Terminal 2026-06-23-09:41:
        Command terminal pointer movement forwards only body-relative coordinates plus mapped keyboard modifier bits from a current command body slot to the exact mounted Ghostty surface. Missing bounds, stale slots, mismatched owners, non-macOS builds, and absent surfaces no-op without logging, persistence, capture, overlays, hidden hit regions, or synthetic routing.
        */
        if !self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let Some(position) = command_terminal_body_relative_mouse_position_for_slot(
            &self.command_terminal_mount_slot_bounds,
            slot_id,
            window_position,
        ) else {
            return false;
        };
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.command_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id {
                return false;
            }

            surface.mouse_pos(position.x, position.y, mouse_mods);
            true
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (position, mouse_mods);
            false
        }
    }

    pub(crate) fn forward_command_terminal_mount_slot_mouse_button(
        &mut self,
        slot_id: CommandTerminalBodyMountSlotId,
        window_position: Point<Pixels>,
        action: ghostty_kit::ffi::ghostty_input_mouse_state_e,
        button: MouseButton,
        modifiers: Modifiers,
    ) -> bool {
        /*
        CDXC:Terminal 2026-06-23-10:23:
        Mounted command terminal button press/release forwarding keeps the current command body slot and body-boundary gates, verifies the mounted surface owner and runtime identity, then updates pointer position and sends the mapped left/right/middle Ghostty button value. GPUI navigation buttons no-op before position forwarding so command terminals do not store raw button state or add fallback routing.
        */
        let Some(ghostty_button) = ghostty_mouse_button_from_gpui_button(button) else {
            return false;
        };
        if !self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let Some(position) = command_terminal_body_relative_mouse_position_for_slot(
            &self.command_terminal_mount_slot_bounds,
            slot_id,
            window_position,
        ) else {
            return false;
        };
        let runtime_session_id = command_terminal_runtime_session_id(slot_id);
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.command_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id
                || surface.runtime_session_id() != runtime_session_id
            {
                return false;
            }

            surface.mouse_pos(position.x, position.y, mouse_mods);
            surface.mouse_button(action, ghostty_button, mouse_mods)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (
                position,
                runtime_session_id,
                action,
                ghostty_button,
                mouse_mods,
            );
            false
        }
    }

    pub(crate) fn forward_command_terminal_mount_slot_mouse_release_outside(
        &mut self,
        slot_id: CommandTerminalBodyMountSlotId,
        button: MouseButton,
        modifiers: Modifiers,
    ) -> bool {
        /*
        CDXC:Terminal 2026-06-23-10:23:
        Command terminal mouse-up-out mirrors Agents capture recovery on the mounted body element. It validates the current slot and exact Ghostty owner runtime identity, requires Ghostty mouse capture, and sends only the mapped left/right/middle release with mapped modifiers so outside coordinates never update or get synthesized.
        */
        if !self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let runtime_session_id = command_terminal_runtime_session_id(slot_id);
        let Some(ghostty_button) = ghostty_mouse_button_from_gpui_button(button) else {
            return false;
        };
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.command_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id
                || surface.runtime_session_id() != runtime_session_id
                || !surface.mouse_captured()
            {
                return false;
            }

            surface.mouse_button(
                ghostty_kit::ffi::GHOSTTY_MOUSE_RELEASE,
                ghostty_button,
                mouse_mods,
            )
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (runtime_session_id, ghostty_button, mouse_mods);
            false
        }
    }

    pub(crate) fn forward_command_terminal_mount_slot_mouse_pressure(
        &mut self,
        slot_id: CommandTerminalBodyMountSlotId,
        window_position: Point<Pixels>,
        stage: PressureStage,
        pressure: f32,
        modifiers: Modifiers,
    ) -> bool {
        /*
        CDXC:Terminal 2026-06-23-09:51:
        Mounted command body pressure mirrors Agents forwarding through the normal command body element only. Current slot, recorded body bounds, exact Ghostty surface identity, and macOS availability must all match before updating pointer position and sending raw GPUI pressure without fallback behavior, logging, persistence, overlays, hidden hit regions, or input routing.
        */
        if !self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let Some(position) = command_terminal_body_relative_mouse_position_for_slot(
            &self.command_terminal_mount_slot_bounds,
            slot_id,
            window_position,
        ) else {
            return false;
        };
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);
        let pressure_stage = ghostty_mouse_pressure_stage_from_gpui_stage(stage);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.command_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id {
                return false;
            }

            surface.mouse_pos(position.x, position.y, mouse_mods);
            surface.mouse_pressure(pressure_stage, f64::from(pressure));
            true
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (position, mouse_mods, pressure_stage, pressure);
            false
        }
    }

    pub(crate) fn forward_command_terminal_mount_slot_mouse_scroll(
        &mut self,
        slot_id: CommandTerminalBodyMountSlotId,
        window_position: Point<Pixels>,
        delta: ScrollDelta,
        modifiers: Modifiers,
    ) -> bool {
        if !self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        let Some(position) = command_terminal_body_relative_mouse_position_for_slot(
            &self.command_terminal_mount_slot_bounds,
            slot_id,
            window_position,
        ) else {
            return false;
        };
        let (scroll_x, scroll_y, scroll_mods) = terminal_ghostty_scroll_delta(delta);
        let mouse_mods = ghostty_mouse_mods_from_gpui_modifiers(modifiers);

        #[cfg(target_os = "macos")]
        {
            let Some(surface) = self.command_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id {
                return false;
            }

            surface.mouse_pos(position.x, position.y, mouse_mods);
            surface.mouse_scroll(scroll_x, scroll_y, scroll_mods);
            true
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (position, scroll_x, scroll_y, scroll_mods, mouse_mods);
            false
        }
    }
}
