//! Open, close, and Portless action plumbing for the native Portless Setup prompt.
//! SEE-ALSO: apps/desktop/src/app/window/portless_setup_modal.rs (the window entity and its decision record), apps/desktop/src/app/native_app_modal_lifecycle.rs (the shared window path), `maybe_open_gpui_portless_setup_prompt` and the action handlers in apps/desktop/src/app/os_integration/notifications_and_portless.rs, and the `runPortlessSetupPromptAdminAction` / `setPortlessEnabled` / `postponePortlessSetupPrompt` arms in apps/desktop/src/app/delayed_send.rs (the React host's routes, mirrored here).
use crate::app::window::*;
use crate::*;

impl GhostexGpuiApp {
    /// Opens the native dialog for the `open` message of the `portlessSetup`
    /// modal kind. Refuses payloads the React host would have refused: `mode`
    /// must be `firstSetup` or `standaloneReconfigure` and `protocol` must be
    /// `https` or `http`.
    pub(crate) fn open_gpui_portless_setup_modal(
        &mut self,
        message: &serde_json::Value,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(mode) = message
            .get("mode")
            .and_then(serde_json::Value::as_str)
            .and_then(PortlessSetupModalMode::parse)
        else {
            return;
        };
        let Some(protocol) = message
            .get("protocol")
            .and_then(serde_json::Value::as_str)
            .and_then(PortlessSetupProtocol::parse)
        else {
            return;
        };
        let config = PortlessSetupModalConfig {
            mode,
            protocol,
            palette: self.gpui_native_modal_palette(),
        };
        let host = self.native_app_modal_host(cx, |app, command, cx| {
            app.handle_gpui_portless_setup_modal_command(command, cx);
        });
        self.open_native_app_modal(
            GpuiAppModalKind::PortlessSetup,
            PORTLESS_SETUP_MODAL_WIDTH,
            PORTLESS_SETUP_MODAL_INITIAL_HEIGHT,
            move |window, cx| {
                cx.new(|cx| GpuiPortlessSetupModalWindow::new(config, host, window, cx))
            },
            cx,
        );
    }

    /// The React host posted one bridge command and then closed; every arm
    /// here calls the function the delayed_send arm for that command calls.
    fn handle_gpui_portless_setup_modal_command(
        &mut self,
        command: PortlessSetupModalCommand,
        cx: &mut gpui::Context<Self>,
    ) {
        match command {
            PortlessSetupModalCommand::AdminAction {
                action,
                protocol,
                request_id,
            } => {
                let mut message = serde_json::Map::new();
                message.insert("action".to_string(), serde_json::json!(action.as_str()));
                message.insert("protocol".to_string(), serde_json::json!(protocol.as_str()));
                message.insert("requestId".to_string(), serde_json::json!(request_id));
                message.insert(
                    "type".to_string(),
                    serde_json::json!("runPortlessSetupPromptAdminAction"),
                );
                self.handle_gpui_portless_admin_action_message(&message, cx);
            }
            PortlessSetupModalCommand::Disable => {
                let mut message = serde_json::Map::new();
                message.insert("enabled".to_string(), serde_json::json!(false));
                message.insert("type".to_string(), serde_json::json!("setPortlessEnabled"));
                self.handle_gpui_set_portless_enabled_message(&message, cx);
            }
            PortlessSetupModalCommand::Postpone | PortlessSetupModalCommand::Cancel => {
                self.suppress_gpui_portless_setup_prompt_for_this_run();
                self.refresh_open_gpui_app_modal_sidebar_state_in_background(cx);
            }
        }
        self.release_native_app_modal_window(GpuiAppModalKind::PortlessSetup, cx);
    }
}
