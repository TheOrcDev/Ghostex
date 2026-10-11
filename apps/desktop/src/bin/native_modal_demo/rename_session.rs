//! Rename Session preview. States (`GHOSTEX_NATIVE_MODAL_DEMO_STATE`):
//! default (prefilled title, agents, history generation allowed), `empty`
//! (no title), `long` (text past the 70-character rule), `noagents` (no
//! Generate-with field), `nohistory` (the plain description and rules).
use super::rename_session_modal::*;
use gpui::{App, AppContext as _};
use std::rc::Rc;

const LONG_TITLE: &str = "Investigate why the sidebar loses the selected session after a remote machine reconnects and the presentation cache is rebuilt from the slim query";

pub(super) fn open(demo: &super::DemoEnv, cx: &mut App) {
    let state = demo.state.as_str();
    let agents = if state == "noagents" {
        Vec::new()
    } else {
        vec![
            RenameSessionAgent {
                agent_id: "codex".to_string(),
                name: "Codex".to_string(),
            },
            RenameSessionAgent {
                agent_id: "claude".to_string(),
                name: "Claude Code".to_string(),
            },
        ]
    };
    let initial_title = match state {
        "empty" => String::new(),
        "long" => LONG_TITLE.to_string(),
        _ => "Unify modal styling".to_string(),
    };
    let host: RenameSessionModalHost = Rc::new(|command, cx: &mut App| {
        match command {
            RenameSessionModalCommand::Rename { title } => eprintln!("rename: {title:?}"),
            RenameSessionModalCommand::GenerateName { title, agent_id } => {
                eprintln!("generate name from {title:?} with {agent_id:?}")
            }
            RenameSessionModalCommand::Cancel => eprintln!("cancel"),
        }
        cx.quit();
    });
    let config = RenameSessionModalConfig {
        agents,
        default_prompt_agent_id: Some("codex".to_string()),
        initial_title,
        can_generate_from_history: state != "nohistory",
        palette: demo.palette,
        prefs_path: None,
    };
    let _ = super::open_modal_window(
        RENAME_SESSION_MODAL_WIDTH,
        RENAME_SESSION_MODAL_INITIAL_HEIGHT,
        move |window, cx| cx.new(|cx| GpuiRenameSessionModalWindow::new(config, host, window, cx)),
        cx,
    );
}
