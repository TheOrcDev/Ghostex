//! IME preedit, sending text, Tab and Return keys and command action scripts to terminal surfaces.

use std::path::Path;
use std::sync::atomic::Ordering;

use gpui::Bounds;
use gpui::Pixels;
use gpui::Window;

use crate::app::consts::*;
use crate::app::helpers::*;
use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn set_preedit_on_focused_terminal_surface(&mut self, bytes: &[u8]) -> bool {
        let Some(target) = self.exact_focused_terminal_text_surface_target() else {
            return false;
        };
        self.set_preedit_on_terminal_text_target(target, bytes)
    }

    pub(crate) fn set_preedit_on_terminal_text_target(
        &mut self,
        target: FocusedTerminalTextMountTarget,
        bytes: &[u8],
    ) -> bool {
        #[cfg(target_os = "macos")]
        {
            match target {
                FocusedTerminalTextMountTarget::Agents(slot_id) => {
                    self.set_preedit_bytes_on_agents_terminal_surface(slot_id, bytes)
                }
                FocusedTerminalTextMountTarget::Command(slot_id) => {
                    self.set_preedit_bytes_on_command_terminal_surface(slot_id, bytes)
                }
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (target, bytes);
            false
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_preedit_bytes_on_agents_terminal_surface(
        &mut self,
        slot_id: AgentsTerminalBodyMountSlotId,
        bytes: &[u8],
    ) -> bool {
        let Some(runtime_session_id) = self
            .agents_terminal_runtime_sessions
            .runtime_session_id_for_shell_session(slot_id.session_id)
        else {
            return false;
        };
        let Some(surface) = self.agents_terminal_ghostty_surfaces.get_mut(&slot_id) else {
            return false;
        };
        if surface.mount_slot_id() != slot_id || surface.runtime_session_id() != runtime_session_id
        {
            return false;
        }

        surface.set_preedit_bytes(bytes);
        true
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_preedit_bytes_on_command_terminal_surface(
        &mut self,
        slot_id: CommandTerminalBodyMountSlotId,
        bytes: &[u8],
    ) -> bool {
        let runtime_session_id = command_terminal_runtime_session_id(slot_id);
        let Some(surface) = self.command_terminal_ghostty_surfaces.get_mut(&slot_id) else {
            return false;
        };
        if surface.mount_slot_id() != slot_id || surface.runtime_session_id() != runtime_session_id
        {
            return false;
        }

        surface.set_preedit_bytes(bytes);
        true
    }

    pub(crate) fn clear_focused_terminal_preedit(&mut self) {
        let _ = self.set_preedit_on_focused_terminal_surface(b"");
        self.terminal_text_marked_range = None;
    }

    pub(crate) fn bounds_for_focused_terminal_ime_point(
        &self,
        element_bounds: Bounds<Pixels>,
    ) -> Option<Bounds<Pixels>> {
        /*
        CDXC:Terminal 2026-06-23-10:45:
        IME candidate-window bounds may use only the current exact Ghostty surface `ime_point` plus the mounted terminal body bounds supplied by GPUI's paint-time input handler. If the focused surface is missing or stale, return None instead of inventing title/path/content/cursor fallbacks.
        */
        #[cfg(target_os = "macos")]
        {
            match self.exact_focused_terminal_text_surface_target()? {
                FocusedTerminalTextMountTarget::Agents(slot_id) => {
                    if !self.agents_terminal_ghostty_surface_matches(slot_id) {
                        return None;
                    }
                    let ime_point = self
                        .agents_terminal_ghostty_surfaces
                        .get(&slot_id)?
                        .ime_point();
                    terminal_ime_bounds_from_ghostty_point(element_bounds, ime_point)
                }
                FocusedTerminalTextMountTarget::Command(slot_id) => {
                    if !self.command_terminal_ghostty_surface_matches(slot_id) {
                        return None;
                    }
                    let ime_point = self
                        .command_terminal_ghostty_surfaces
                        .get(&slot_id)?
                        .ime_point();
                    terminal_ime_bounds_from_ghostty_point(element_bounds, ime_point)
                }
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = element_bounds;
            None
        }
    }

    pub(crate) fn send_text_to_focused_terminal_surface(
        &mut self,
        text: &str,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        if text.is_empty() {
            return false;
        }

        if let Some(view) = self.focused_gpui_engine_terminal_view() {
            view.update(cx, |view, cx| view.send_text_input(text, cx));
            return true;
        }

        #[cfg(target_os = "macos")]
        {
            match focused_terminal_text_target(self.active_mode, self.shell_focus) {
                Some(FocusedTerminalTextTarget::Command) => {
                    self.send_text_bytes_to_focused_command_terminal_surface(text.as_bytes())
                }
                Some(FocusedTerminalTextTarget::Agents) => {
                    self.send_text_bytes_to_focused_agents_terminal_surface(text.as_bytes())
                }
                None => false,
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = text;
            false
        }
    }

    pub(crate) fn send_tab_key_to_gpui_engine_terminal(
        &mut self,
        target: GpuiEngineTerminalEventTarget,
        action: ghostty_vt::VtKeyAction,
        shift: bool,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let Some(view) = self.gpui_engine_terminal_view_for_target(target) else {
            return false;
        };
        view.update(cx, |view, cx| view.send_tab_key_action(action, shift, cx))
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn dispatch_window_scoped_ghostex_hotkey(
        &mut self,
        action_id: &str,
        owner: GpuiKeyboardOwner,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:Hotkeys 2026-07-24:
        Native pre-dispatch captures the window and exact keyboard owner before
        queuing an app action. Never bounce the selector through a process-global
        callback or another window's current focus; the target app entity and
        Window supplied by update_in are the registration that accepted it.
        */
        support_logs::append(
            support_logs::GpuiSupportLog::TerminalFocus,
            "gpui.keyboardRouter.hotkeyDispatched",
            serde_json::json!({
                "actionId": action_id,
                "owner": format!("{owner:?}"),
            }),
        );
        self.handle_gpui_app_modal_sidebar_command(
            serde_json::json!({
                "message": {
                    "actionId": action_id,
                    "type": "runGhostexHotkeyAction",
                },
            }),
            window,
            cx,
        );
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn dispatch_window_scoped_application_keyboard_command(
        &mut self,
        command: GpuiApplicationKeyboardCommand,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        match command {
            GpuiApplicationKeyboardCommand::Hide => cx.hide(),
            GpuiApplicationKeyboardCommand::HideOthers => cx.hide_other_apps(),
            GpuiApplicationKeyboardCommand::MinimizeWindow => window.minimize_window(),
            GpuiApplicationKeyboardCommand::Quit => {
                GPUI_APP_QUIT_IN_PROGRESS.store(true, Ordering::Release);
                cx.quit();
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn send_text_bytes_to_focused_agents_terminal_surface(
        &mut self,
        bytes: &[u8],
    ) -> bool {
        let Some(slot_id) = focused_agents_terminal_surface_mount_slot(
            self.active_mode,
            self.shell_focus,
            &self.agents_workspace,
        ) else {
            return false;
        };
        self.send_text_bytes_to_mounted_agents_terminal_surface(slot_id, bytes)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn send_text_bytes_to_mounted_agents_terminal_surface(
        &mut self,
        slot_id: AgentsTerminalBodyMountSlotId,
        bytes: &[u8],
    ) -> bool {
        if bytes.is_empty()
            || !self
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
        let Some(surface) = self.agents_terminal_ghostty_surfaces.get_mut(&slot_id) else {
            return false;
        };
        if surface.mount_slot_id() != slot_id || surface.runtime_session_id() != runtime_session_id
        {
            return false;
        }

        surface.send_text_bytes(bytes);
        true
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn send_text_bytes_to_focused_command_terminal_surface(
        &mut self,
        bytes: &[u8],
    ) -> bool {
        let Some(slot_id) =
            focused_command_terminal_surface_mount_slot(self.shell_focus, &self.command_pane)
        else {
            return false;
        };
        let runtime_session_id = command_terminal_runtime_session_id(slot_id);
        let Some(surface) = self.command_terminal_ghostty_surfaces.get_mut(&slot_id) else {
            return false;
        };
        if surface.mount_slot_id() != slot_id || surface.runtime_session_id() != runtime_session_id
        {
            return false;
        }

        surface.send_text_bytes(bytes);
        true
    }

    pub(crate) fn send_return_key_to_mounted_command_terminal_surface(
        &mut self,
        slot_id: CommandTerminalBodyMountSlotId,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:DelayedSend 2026-06-25-15:11:
        Delayed Send must submit the staged prompt through Ghostty's key path, matching native `sendTerminalEnter`, rather than writing carriage-return text. Use the exact current mounted command slot and the macOS Return keycode/text tuple; if the surface is missing or stale, no other terminal receives the key.
        */
        if self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
            && let Some(record) = self.command_gpui_engine_terminals.get(&slot_id.session_id)
        {
            let view = record.view.clone();
            view.update(cx, |view, cx| view.send_return_key(cx));
            return true;
        }
        #[cfg(target_os = "macos")]
        {
            if !self
                .command_pane
                .is_current_terminal_body_mount_slot(slot_id)
            {
                return false;
            }
            let runtime_session_id = command_terminal_runtime_session_id(slot_id);
            let Some(surface) = self.command_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id
                || surface.runtime_session_id() != runtime_session_id
            {
                return false;
            }
            let Ok(return_text) = std::ffi::CString::new(COMMAND_PANE_DELAYED_SEND_RETURN_TEXT)
            else {
                return false;
            };
            surface.send_key(ghostty_kit::ffi::ghostty_input_key_s {
                action: COMMAND_PANE_GHOSTTY_KEY_ACTION_PRESS,
                mods: 0,
                consumed_mods: 0,
                keycode: COMMAND_PANE_DELAYED_SEND_RETURN_KEYCODE,
                text: return_text.as_ptr(),
                unshifted_codepoint: COMMAND_PANE_DELAYED_SEND_RETURN_UNSHIFTED_CODEPOINT,
                composing: false,
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = slot_id;
            false
        }
    }

    pub(crate) fn send_return_key_to_mounted_agents_terminal_surface(
        &mut self,
        slot_id: AgentsTerminalBodyMountSlotId,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:SessionTitles 2026-06-27-02:27:
        Mapped Agents rename submission must use Ghostty's key path for the exact mounted Agents slot, matching native Return delivery. If the slot is stale, hidden, sleeping, missing a runtime id, or owned by a different surface, no other terminal receives a newline or fallback key.
        */
        if self
            .agents_workspace
            .is_current_terminal_body_mount_slot(slot_id)
            && let Some(record) = self.agents_gpui_engine_terminals.get(&slot_id.session_id)
        {
            let view = record.view.clone();
            view.update(cx, |view, cx| view.send_return_key(cx));
            return true;
        }
        #[cfg(target_os = "macos")]
        {
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
            let Some(surface) = self.agents_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                return false;
            };
            if surface.mount_slot_id() != slot_id
                || surface.runtime_session_id() != runtime_session_id
            {
                return false;
            }
            let Ok(return_text) = std::ffi::CString::new(GPUI_TERMINAL_RETURN_TEXT) else {
                return false;
            };
            surface.send_key(ghostty_kit::ffi::ghostty_input_key_s {
                action: COMMAND_PANE_GHOSTTY_KEY_ACTION_PRESS,
                mods: 0,
                consumed_mods: 0,
                keycode: GPUI_TERMINAL_RETURN_KEYCODE,
                text: return_text.as_ptr(),
                unshifted_codepoint: GPUI_TERMINAL_RETURN_UNSHIFTED_CODEPOINT,
                composing: false,
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = slot_id;
            false
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn send_return_key_to_parked_agents_terminal_surface(
        &mut self,
        runtime_session_id: AgentsTerminalRuntimeSessionId,
    ) -> bool {
        let Some(owner) = self
            .agents_terminal_parked_runtime_owners
            .get_mut(&runtime_session_id)
        else {
            return false;
        };
        if !owner.matches_identity(
            runtime_session_id,
            owner.shell_session_id,
            owner.mount_slot_id,
        ) {
            return false;
        }
        let Ok(return_text) = std::ffi::CString::new(GPUI_TERMINAL_RETURN_TEXT) else {
            return false;
        };
        owner
            .surface_owner
            .send_key(ghostty_kit::ffi::ghostty_input_key_s {
                action: COMMAND_PANE_GHOSTTY_KEY_ACTION_PRESS,
                mods: 0,
                consumed_mods: 0,
                keycode: GPUI_TERMINAL_RETURN_KEYCODE,
                text: return_text.as_ptr(),
                unshifted_codepoint: GPUI_TERMINAL_RETURN_UNSHIFTED_CODEPOINT,
                composing: false,
            })
    }

    pub(crate) fn send_gpui_command_action_script_to_mounted_terminal(
        &mut self,
        slot_id: CommandTerminalBodyMountSlotId,
        execution_text: &str,
        status_file_path: &Path,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        if !self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        #[cfg(target_os = "windows")]
        {
            let input = if matches!(
                windows_terminal_backend::resolve_current(),
                Ok(windows_terminal_backend::ResolvedWindowsTerminalBackend::PowerShell)
            ) {
                execution_text.to_string()
            } else {
                gpui_command_action_mounted_terminal_script_text(execution_text, status_file_path)
            };
            let Some(record) = self.command_gpui_engine_terminals.get(&slot_id.session_id) else {
                return false;
            };
            let view = record.view.clone();
            view.update(cx, |view, cx| view.send_text_input(&input, cx));
            return self.send_return_key_to_mounted_command_terminal_surface(slot_id, cx);
        }
        #[cfg(not(target_os = "windows"))]
        let Some(source_command) = gpui_command_action_staged_mounted_script_source_command(
            execution_text,
            status_file_path,
        ) else {
            return false;
        };
        #[cfg(not(target_os = "windows"))]
        if let Some(record) = self.command_gpui_engine_terminals.get(&slot_id.session_id) {
            let view = record.view.clone();
            view.update(cx, |view, cx| view.send_text_input(&source_command, cx));
            return self.send_return_key_to_mounted_command_terminal_surface(slot_id, cx);
        }

        #[cfg(target_os = "macos")]
        {
            let runtime_session_id = command_terminal_runtime_session_id(slot_id);
            {
                let Some(surface) = self.command_terminal_ghostty_surfaces.get_mut(&slot_id) else {
                    return false;
                };
                if surface.mount_slot_id() != slot_id
                    || surface.runtime_session_id() != runtime_session_id
                {
                    return false;
                }
                surface.send_text_bytes(source_command.as_bytes());
            }
            /*
            CDXC:CommandPane 2026-06-27-07:54:
            Mounted reused default Actions mirror native `writeTerminalScript`: stage the private wrapper in a temp script for the exact current command surface, send only the short source command as terminal text, and submit it through the real Return key path so reruns execute immediately without relying on carriage-return text or a deferred launch payload.
            */
            self.send_return_key_to_mounted_command_terminal_surface(slot_id, cx)
        }

        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    pub(crate) fn gpui_command_action_mounted_reuse_surface_available(
        &self,
        slot_id: CommandTerminalBodyMountSlotId,
    ) -> bool {
        /*
        CDXC:CommandPane 2026-06-27-07:54:
        Default Action reuse may bypass launch payloads only when the selected reused tab already owns the exact current mounted command Ghostty surface. Missing, stale, sleeping, or unmounted reused tabs must use the exact-slot launch payload path instead of borrowing another terminal surface.
        */
        if !self
            .command_pane
            .is_current_terminal_body_mount_slot(slot_id)
        {
            return false;
        }
        if self
            .command_gpui_engine_terminals
            .contains_key(&slot_id.session_id)
        {
            return true;
        }
        #[cfg(target_os = "macos")]
        {
            let runtime_session_id = command_terminal_runtime_session_id(slot_id);
            self.command_terminal_ghostty_surfaces
                .get(&slot_id)
                .is_some_and(|surface| {
                    surface.mount_slot_id() == slot_id
                        && surface.runtime_session_id() == runtime_session_id
                })
        }

        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}
