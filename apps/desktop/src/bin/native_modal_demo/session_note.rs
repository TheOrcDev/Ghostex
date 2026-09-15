//! Session Note preview. States (`GHOSTEX_NATIVE_MODAL_DEMO_STATE`): default
//! (an existing note and a session title), `empty` (no note yet: placeholder
//! and the save hint), `long` (a note that scrolls), `notitle` (no session title).
use super::session_note_modal::*;
use gpui::{App, AppContext as _};
use std::rc::Rc;

const NOTE: &str =
    "Compare spacing, typography, action order, and surface colors across every modal.";

pub(super) fn open(demo: &super::DemoEnv, cx: &mut App) {
    let state = demo.state.as_str();
    let initial_note = match state {
        "empty" => String::new(),
        "long" => (1..=14)
            .map(|line| format!("{line}. Check the {line}th modal against its Storybook story in both appearances."))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => NOTE.to_string(),
    };
    let session_title = (state != "notitle").then(|| "Unify modal styling".to_string());
    let host: SessionNoteModalHost = Rc::new(|command, cx: &mut App| {
        match command {
            SessionNoteModalCommand::Save { note } => eprintln!("save note: {note:?}"),
            SessionNoteModalCommand::Cancel => eprintln!("cancel"),
        }
        cx.quit();
    });
    let config = SessionNoteModalConfig {
        initial_note,
        session_title,
        palette: demo.palette,
    };
    let _ = super::open_modal_window(
        SESSION_NOTE_MODAL_WIDTH,
        SESSION_NOTE_MODAL_INITIAL_HEIGHT,
        move |window, cx| cx.new(|cx| GpuiSessionNoteModalWindow::new(config, host, window, cx)),
        cx,
    );
}
