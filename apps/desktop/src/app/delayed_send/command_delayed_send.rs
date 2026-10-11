//! Command pane sidebar commands and the command delayed-send timers.

use std::time::Duration;
use std::time::SystemTime;

use crate::app::consts::*;
use crate::app::helpers::*;
use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn handle_gpui_schedule_delayed_send_command(
        &mut self,
        command: &serde_json::Map<String, serde_json::Value>,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:DelayedSend 2026-08-17:
        Remote sidebar rows carry their canonical machine/project/session id,
        but they do not belong to a local command tab or local Agents mapping.
        Hand that bounded command to the Rust store (gx_store/terminal_lifecycle/session_edits.rs)
        so it can submit the durable trigger to the gxserver that hosts the session.
        */
        if command
            .get("sessionId")
            .and_then(serde_json::Value::as_str)
            .and_then(gpui_remote_attach_session_reference_from_project_id)
            .is_some()
        {
            self.dispatch_gpui_sidebar_host_message(serde_json::Value::Object(command.clone()), cx);
            return;
        }
        /*
        CDXC:DelayedSend 2026-06-25-23:04:
        `scheduleDelayedSend` is a direct command-session sidebar command, so resolve its external `G{u64}` sessionId through the shared live command-tab bridge before reading delayMs. Malformed, legacy numeric, stale, missing, and orphan ids must no-op without falling back to the focused command group or surfacing duration validation for the wrong target.
        */
        let Some((_group_id, session_id)) =
            gpui_app_modal_sidebar_command_live_command_tab(&self.command_pane, command)
        else {
            self.handle_gpui_schedule_agents_delayed_send_command(command, cx);
            return;
        };
        let Some(delay_ms) = command.get("delayMs").and_then(serde_json::Value::as_u64) else {
            return;
        };
        let Some(duration) = gpui_command_delayed_send_duration_from_millis(delay_ms) else {
            self.dispatch_gpui_app_modal_toast(
                "warning",
                "Delayed Send unavailable",
                "Choose a future send time within 24 days.",
                cx,
            );
            return;
        };
        if self.schedule_gpui_command_delayed_send(session_id, duration, cx) {
            let description = format!(
                "Presses Enter in {}.",
                gpui_command_delayed_send_duration_label(duration)
            );
            self.dispatch_gpui_app_modal_toast("info", "Delayed Send scheduled", &description, cx);
        } else {
            self.dispatch_gpui_app_modal_toast(
                "warning",
                "Delayed Send unavailable",
                "Select a visible command terminal before scheduling Delayed Send.",
                cx,
            );
        }
    }

    pub(crate) fn handle_gpui_rename_command_session_command(
        &mut self,
        command: &serde_json::Map<String, serde_json::Value>,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:CommandPane 2026-06-25-16:33:
        Rename Session submissions from a GPUI command-pane modal are local command-tab title edits. Accept only the command session id and normalized title; generated-title requests require a gxserver-backed agent session and must not write long prompt text into a local command tab.
        */
        let Some(session_id) = command
            .get("sessionId")
            .and_then(gpui_command_session_id_from_modal_value)
        else {
            let session_id_is_sidebar_owned = command
                .get("sessionId")
                .and_then(serde_json::Value::as_str)
                .is_some_and(gpui_app_modal_sidebar_session_id_allowed);
            if session_id_is_sidebar_owned {
                self.dispatch_gpui_sidebar_host_message(
                    serde_json::Value::Object(command.clone()),
                    cx,
                );
            }
            return;
        };
        if command
            .get("shouldGenerateTitle")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            self.dispatch_gpui_app_modal_toast(
                "warning",
                "Rename unavailable",
                "Generate Name is not available for local GPUI command tabs yet.",
                cx,
            );
            return;
        }
        let Some(title) = command
            .get("title")
            .and_then(gpui_command_session_rename_title_from_modal_value)
        else {
            return;
        };
        let gxserver_key = self.command_gxserver_session_key_for_command_tab(session_id);
        if !self.command_pane.rename_session(session_id, title.clone()) {
            return;
        }
        if let Some(key) = gxserver_key {
            self.update_command_gxserver_session_title_in_background(key, title, cx);
        }
        self.persist_shell_layout_state();
        self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        cx.notify();
    }

    pub(crate) fn handle_gpui_cancel_delayed_send_command(
        &mut self,
        command: &serde_json::Map<String, serde_json::Value>,
        cx: &mut gpui::Context<Self>,
    ) {
        if command
            .get("sessionId")
            .and_then(serde_json::Value::as_str)
            .and_then(gpui_remote_attach_session_reference_from_project_id)
            .is_some()
        {
            self.dispatch_gpui_sidebar_host_message(serde_json::Value::Object(command.clone()), cx);
            return;
        }
        /*
        CDXC:DelayedSend 2026-06-25-23:04:
        Cancel submissions from the shared sidebar/app-modal bridge must target a live command tab, not a stale stored command-session row. Resolve the external `G{u64}` sessionId through the shared app-modal command bridge so malformed, legacy numeric, missing, orphan, or stale ids no-op before any runtime timer is cleared.
        */
        let Some((_group_id, session_id)) =
            gpui_app_modal_sidebar_command_live_command_tab(&self.command_pane, command)
        else {
            self.handle_gpui_cancel_agents_delayed_send_command(command, cx);
            return;
        };
        if self.clear_gpui_command_delayed_send_timer(session_id) {
            self.sync_gpui_keep_awake_automation_from_current_settings(cx);
            self.dispatch_gpui_app_modal_toast("info", "Delayed Send canceled", "", cx);
            self.persist_shell_layout_state();
            cx.notify();
        } else {
            self.dispatch_gpui_app_modal_toast("info", "No Delayed Send timer is active", "", cx);
        }
    }

    pub(crate) fn handle_gpui_toggle_close_after_done_command(
        &mut self,
        command: &serde_json::Map<String, serde_json::Value>,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(session_id) = self.gpui_agents_delayed_send_session_id_from_command(command) {
            let _ = self.toggle_gpui_close_after_done_for_agents_session(session_id, cx);
            return;
        }
        let Some((_group_id, session_id)) =
            gpui_app_modal_sidebar_command_live_command_tab(&self.command_pane, command)
        else {
            return;
        };
        self.toggle_gpui_command_close_after_done(session_id, cx);
    }

    pub(crate) fn restore_gpui_command_startup_activity_intents(
        &mut self,
        restore_intents: Vec<GpuiCommandStartupActivityRestoreIntent>,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:Workarea 2026-06-25-17:25:
        Startup activity restore mutates only the command-pane model and leaves persistence to the app startup pass after Delayed Send restore also runs. This keeps Working wake hints one-shot without rewriting a restored timer checkpoint before the runtime timer map is installed.
        */
        if !command_pane_apply_startup_activity_restore_intents(
            &mut self.command_pane,
            &restore_intents,
        ) {
            return false;
        }
        self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        cx.notify();
        true
    }

    pub(crate) fn restore_gpui_command_delayed_send_timers(
        &mut self,
        restore_timers: Vec<GpuiCommandDelayedSendRestoreTimer>,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:DelayedSend 2026-06-25-16:41:
        Startup re-arms restored command Delayed Send timers from the saved remaining-duration checkpoint as normal runtime timers. This keeps native parity without persisting command text, terminal content, paths, titles, Ghostty runtime ids, stdout/stderr, or old deadlines as authority after restart.

        CDXC:DelayedSend 2026-06-25-16:56:
        Restored Delayed Send timers are also startup wake reasons for command-pane tabs. Wake only after loading a safe persisted checkpoint so a restarted timer can reach a command terminal body, while manual in-process Sleep remains parked until the user wakes it.
        */
        let mut changed = false;
        for restore_timer in restore_timers {
            if command_pane_group_for_session(&self.command_pane, restore_timer.session_id)
                .is_none()
                || self
                    .command_pane
                    .session(restore_timer.session_id)
                    .is_none()
            {
                continue;
            }
            self.command_delayed_send_generation =
                self.command_delayed_send_generation.wrapping_add(1);
            let generation = self.command_delayed_send_generation;
            let duration = gpui_command_delayed_send_restore_duration(restore_timer.remaining_ms);
            let deadline_at = SystemTime::now()
                .checked_add(duration)
                .unwrap_or_else(SystemTime::now);
            self.command_delayed_send_timers.insert(
                restore_timer.session_id,
                GpuiCommandDelayedSendTimer {
                    deadline_at,
                    generation,
                },
            );
            command_pane_apply_delayed_send_restore_intent(
                &mut self.command_pane,
                restore_timer.session_id,
            );
            self.schedule_gpui_command_delayed_send_fire(
                restore_timer.session_id,
                generation,
                duration,
                cx,
            );
            changed = true;
        }
        if changed {
            self.ensure_gpui_command_delayed_send_countdown_ticker(cx);
            self.ensure_gpui_command_delayed_send_persistence_ticker(cx);
            self.refresh_sidebar_command_pane_sessions_if_changed(cx);
            cx.notify();
        }
        changed
    }

    pub(crate) fn restore_command_terminal_gxserver_sessions_from_shell_state(
        &mut self,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:Workarea 2026-07-04:
        Startup restore validates persisted command-surface daemon sessions by
        running the same wake/attach metadata flow as live command tabs. Awake
        restored tabs get one-shot attach payloads for their command mount
        slots; sleeping tabs keep only their gxserver key and defer attach
        until the user wakes them. Missing daemon sessions close through the
        existing attach-failure path instead of creating a replacement shell.
        */
        let restore_slots = self
            .command_pane
            .flat_tab_ids()
            .into_iter()
            .filter_map(|(group_id, session_id)| {
                let session = self.command_pane.session(session_id)?;
                if session.is_sleeping {
                    return None;
                }
                let key = self.command_gxserver_session_key_for_command_tab(session_id)?;
                Some((
                    CommandTerminalBodyMountSlotId {
                        group_id,
                        session_id,
                    },
                    key,
                ))
            })
            .collect::<Vec<_>>();
        let mut started = false;
        for (slot_id, key) in restore_slots {
            if self
                .command_gxserver_attach_pending
                .contains(&slot_id.session_id)
            {
                continue;
            }
            self.start_existing_command_terminal_gxserver_attach_for_slot(slot_id, key, None, cx);
            started = true;
        }
        if started {
            cx.notify();
        }
        started
    }

    pub(crate) fn schedule_gpui_command_delayed_send(
        &mut self,
        session_id: CommandSessionId,
        duration: Duration,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:DelayedSend 2026-06-25-15:11:
        A GPUI Delayed Send timer may be armed only for a currently mounted command terminal body. This preserves native's exact target-session behavior without falling back to shell focus, titles, command text, persisted state, or another visible terminal when the original command surface is unavailable.
        */
        if command_pane_mounted_slot_for_session(&self.command_pane, session_id).is_none() {
            return false;
        }
        let Some(session) = self.command_pane.session_mut(session_id) else {
            return false;
        };
        if session.is_sleeping {
            return false;
        }
        self.command_delayed_send_generation = self.command_delayed_send_generation.wrapping_add(1);
        let generation = self.command_delayed_send_generation;
        let deadline_at = SystemTime::now()
            .checked_add(duration)
            .unwrap_or_else(SystemTime::now);
        self.command_delayed_send_timers.insert(
            session_id,
            GpuiCommandDelayedSendTimer {
                deadline_at,
                generation,
            },
        );
        session.set_delayed_send_active(true, true);
        self.ensure_gpui_command_delayed_send_countdown_ticker(cx);
        self.ensure_gpui_command_delayed_send_persistence_ticker(cx);
        self.schedule_gpui_command_delayed_send_fire(session_id, generation, duration, cx);
        self.sync_gpui_keep_awake_automation_from_current_settings(cx);
        self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        self.persist_shell_layout_state();
        cx.notify();
        true
    }

    pub(crate) fn ensure_gpui_command_delayed_send_countdown_ticker(
        &mut self,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:DelayedSend 2026-06-25-15:42:
        The command-pane Delayed Send body badge is live countdown chrome. Run a process-local one-second ticker only while timers exist so the centered badge can update without persisting deadlines, logging command content, or creating a renderer-owned timer fallback.
        */
        if self.command_delayed_send_countdown_ticker_active
            || self.command_delayed_send_timers.is_empty()
        {
            return;
        }
        self.command_delayed_send_countdown_ticker_active = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let keep_running = this
                    .update(cx, |this, cx| {
                        if this.command_delayed_send_timers.is_empty() {
                            this.command_delayed_send_countdown_ticker_active = false;
                            cx.notify();
                            false
                        } else {
                            this.refresh_sidebar_command_pane_sessions_if_changed(cx);
                            cx.notify();
                            true
                        }
                    })
                    .unwrap_or(false);
                if !keep_running {
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn ensure_gpui_command_delayed_send_persistence_ticker(
        &mut self,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:DelayedSend 2026-06-25-16:41:
        Native refreshes Delayed Send remaining-duration checkpoints once per minute so restart resumes near the live countdown position. GPUI mirrors that with a low-frequency shell-state write while timers exist, still serializing only safe timer metadata through the central writer.
        */
        if self.command_delayed_send_persistence_ticker_active
            || self.command_delayed_send_timers.is_empty()
        {
            return;
        }
        self.command_delayed_send_persistence_ticker_active = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(COMMAND_PANE_DELAYED_SEND_PERSIST_INTERVAL)
                    .await;
                let keep_running = this
                    .update(cx, |this, _cx| {
                        if this.command_delayed_send_timers.is_empty() {
                            this.command_delayed_send_persistence_ticker_active = false;
                            false
                        } else {
                            this.persist_shell_layout_state();
                            true
                        }
                    })
                    .unwrap_or(false);
                if !keep_running {
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn schedule_gpui_command_delayed_send_fire(
        &mut self,
        session_id: CommandSessionId,
        generation: u64,
        duration: Duration,
        cx: &mut gpui::Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(duration).await;
            let _ = this.update(cx, |this, cx| {
                this.fire_gpui_command_delayed_send(session_id, generation, cx);
            });
        })
        .detach();
    }

    pub(crate) fn fire_gpui_command_delayed_send(
        &mut self,
        session_id: CommandSessionId,
        generation: u64,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let Some(timer) = self.command_delayed_send_timers.get(&session_id).copied() else {
            return false;
        };
        if timer.generation != generation {
            return false;
        }
        let target_was_sleeping = self
            .command_pane
            .session(session_id)
            .is_some_and(|session| session.is_sleeping);
        self.command_delayed_send_timers.remove(&session_id);
        if let Some(session) = self.command_pane.session_mut(session_id) {
            session.set_delayed_send_active(false, false);
        }
        let sent = command_pane_mounted_slot_for_session(&self.command_pane, session_id)
            .is_some_and(|slot_id| {
                self.send_return_key_to_mounted_command_terminal_surface(slot_id, cx)
            });
        self.sync_gpui_keep_awake_automation_from_current_settings(cx);
        self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        self.persist_shell_layout_state();
        cx.notify();
        if !sent && !target_was_sleeping {
            self.dispatch_gpui_app_modal_toast(
                "warning",
                "Delayed Send skipped",
                "The command terminal was no longer mounted.",
                cx,
            );
        }
        sent
    }

    pub(crate) fn clear_gpui_command_delayed_send_timer(
        &mut self,
        session_id: CommandSessionId,
    ) -> bool {
        let removed = self
            .command_delayed_send_timers
            .remove(&session_id)
            .is_some();
        if let Some(session) = self.command_pane.session_mut(session_id)
            && session.delayed_send_timer_owned
        {
            session.set_delayed_send_active(false, false);
        }
        removed
    }

    pub(crate) fn prune_gpui_command_delayed_send_timers_for_command_model(&mut self) -> bool {
        let stale_session_ids = command_delayed_send_stale_runtime_timer_session_ids(
            &self.command_pane,
            &self.command_delayed_send_timers,
        );
        let changed = !stale_session_ids.is_empty();
        for session_id in stale_session_ids {
            self.command_delayed_send_timers.remove(&session_id);
            if let Some(session) = self.command_pane.session_mut(session_id)
                && session.delayed_send_timer_owned
            {
                session.set_delayed_send_active(false, false);
            }
        }
        changed
    }
}
