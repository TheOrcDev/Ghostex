//! Preview host for the Install remote gxserver prompt. State `longname`
//! uses a long machine name so the description wraps one line further.
use super::remote_gxserver_install_modal::*;
use gpui::{App, AppContext as _};
use std::rc::Rc;

pub(super) fn open(demo: &super::DemoEnv, cx: &mut App) {
    let machine_name = if demo.state == "longname" {
        "ubuntu-build-box-in-the-basement"
    } else {
        "build-box"
    };
    let host: RemoteGxserverInstallModalHost = Rc::new(move |command, cx: &mut App| {
        match command {
            RemoteGxserverInstallModalCommand::Approve => {
                eprintln!("reconnectRemoteMachine installApproved=true")
            }
            RemoteGxserverInstallModalCommand::Cancel => eprintln!("cancel"),
        }
        cx.quit();
    });
    let config = RemoteGxserverInstallModalConfig {
        machine_name: machine_name.to_string(),
        palette: demo.palette,
    };
    super::open_modal_window(
        REMOTE_GXSERVER_INSTALL_MODAL_WIDTH,
        REMOTE_GXSERVER_INSTALL_MODAL_INITIAL_HEIGHT,
        move |window, cx| {
            cx.new(|cx| GpuiRemoteGxserverInstallModalWindow::new(config, host, window, cx))
        },
        cx,
    );
}
