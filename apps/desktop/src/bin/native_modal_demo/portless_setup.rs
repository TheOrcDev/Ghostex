//! Preview host for the Portless Setup prompt. States: `firstsetup`
//! (default, Postpone / Disable / Install) and `reconfigure`
//! (Cancel / Disable / Reconfigure).
use super::portless_setup_modal::*;
use gpui::{App, AppContext as _};
use std::rc::Rc;

pub(super) fn open(demo: &super::DemoEnv, cx: &mut App) {
    let mode = if demo.state == "reconfigure" {
        PortlessSetupModalMode::StandaloneReconfigure
    } else {
        PortlessSetupModalMode::FirstSetup
    };
    let host: PortlessSetupModalHost = Rc::new(move |command, cx: &mut App| {
        match command {
            PortlessSetupModalCommand::AdminAction {
                action,
                protocol,
                request_id,
            } => eprintln!(
                "runPortlessSetupPromptAdminAction action={} protocol={} requestId={request_id}",
                action.as_str(),
                protocol.as_str()
            ),
            PortlessSetupModalCommand::Disable => eprintln!("setPortlessEnabled enabled=false"),
            PortlessSetupModalCommand::Postpone => eprintln!("postponePortlessSetupPrompt"),
            PortlessSetupModalCommand::Cancel => eprintln!("cancelPortlessSetupPrompt"),
        }
        cx.quit();
    });
    let config = PortlessSetupModalConfig {
        mode,
        protocol: PortlessSetupProtocol::Https,
        palette: demo.palette,
    };
    super::open_modal_window(
        PORTLESS_SETUP_MODAL_WIDTH,
        PORTLESS_SETUP_MODAL_INITIAL_HEIGHT,
        move |window, cx| cx.new(|cx| GpuiPortlessSetupModalWindow::new(config, host, window, cx)),
        cx,
    );
}
