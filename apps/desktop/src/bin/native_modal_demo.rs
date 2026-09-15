/*
CDXC:AppModal 2026-09-15 WHY:
Standalone preview of the native GPUI app modals so their look can be checked
against the React Storybook stories without launching Ghostex. Run with:
    cargo run --release --bin native-modal-demo
Environment:
    GHOSTEX_NATIVE_MODAL_DEMO=export-transcript|...           (which modal; default export-transcript)
    GHOSTEX_NATIVE_MODAL_DEMO_THEME=dark|light                (default dark)
    GHOSTEX_NATIVE_MODAL_DEMO_STATE=<per-modal state name>    (default: the opening state)
    GHOSTEX_EXPORT_MODAL_DEMO_MODE=handoff|export             (Handoff / Export only)
Hosts simulate the daemon: a primary action answers after one second with a
fake success, or with a failure message when the state is `failed`. Closing
actions quit the demo.
*/
#![allow(dead_code)]
#[path = "../assets.rs"]
mod assets;
#[path = "../app/window/export_transcript_modal.rs"]
mod export_transcript_modal;
#[path = "../app/window/native_modal_kit.rs"]
mod native_modal_kit;
// DEMO-MODULES: one `#[path]` include per converted modal, plus its demo module under native_modal_demo/.
#[path = "../app/window/update_available_modal.rs"]
mod update_available_modal;
#[path = "native_modal_demo/update_available.rs"]
mod update_available_demo;
#[path = "../app/window/remote_setup_modal.rs"]
mod remote_setup_modal;
#[path = "native_modal_demo/remote_setup.rs"]
mod remote_setup_demo;
#[path = "../app/window/delayed_send_modal.rs"]
mod delayed_send_modal;
#[path = "native_modal_demo/delayed_send.rs"]
mod delayed_send_demo;
#[path = "../app/window/space_editor_modal.rs"]
mod space_editor_modal;
#[path = "native_modal_demo/space_editor.rs"]
mod space_editor_demo;
#[path = "../app/window/create_worktree_modal.rs"]
mod create_worktree_modal;
#[path = "native_modal_demo/create_worktree.rs"]
mod create_worktree_demo;
#[path = "../app/window/remote_gxserver_install_modal.rs"]
mod remote_gxserver_install_modal;
#[path = "native_modal_demo/remote_gxserver_install.rs"]
mod remote_gxserver_install_demo;
#[path = "../app/window/portless_setup_modal.rs"]
mod portless_setup_modal;
#[path = "native_modal_demo/portless_setup.rs"]
mod portless_setup_demo;
#[path = "../app/window/delete_worktree_modal.rs"]
mod delete_worktree_modal;
#[path = "native_modal_demo/delete_worktree.rs"]
mod delete_worktree_demo;
#[path = "../app/window/rename_worktree_modal.rs"]
mod rename_worktree_modal;
#[path = "native_modal_demo/rename_worktree.rs"]
mod rename_worktree_demo;
#[path = "../app/window/missing_project_folder_modal.rs"]
mod missing_project_folder_modal;
#[path = "native_modal_demo/missing_project_folder.rs"]
mod missing_project_folder_demo;
#[path = "../app/window/rename_session_modal.rs"]
mod rename_session_modal;
#[path = "native_modal_demo/rename_session.rs"]
mod rename_session_demo;
#[path = "../app/window/session_note_modal.rs"]
mod session_note_modal;
#[path = "native_modal_demo/session_note.rs"]
mod session_note_demo;
#[path = "native_modal_demo/agent_hooks_required.rs"]
mod agent_hooks_required_demo;
#[path = "../app/window/agent_hooks_required_modal.rs"]
mod agent_hooks_required_modal;

use export_transcript_modal::*;
use gpui::{
    App, AppContext as _, Bounds, Entity, Render, Styled as _, WindowBounds, WindowHandle,
    WindowOptions, point, px, size,
};
use gpui_component::Root;
use native_modal_kit::ModalPalette;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const FAKE_PATH: &str = "/Users/you/Library/Application Support/ghostex/exports/fix-hookless-agent-modal-g04t1-20260915-081928.md";

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

struct DemoEnv {
    modal: String,
    state: String,
    palette: ModalPalette,
}

impl DemoEnv {
    fn read() -> Self {
        let modal = env("GHOSTEX_NATIVE_MODAL_DEMO");
        Self {
            modal: if modal.is_empty() {
                "export-transcript".to_string()
            } else {
                modal
            },
            state: env("GHOSTEX_NATIVE_MODAL_DEMO_STATE"),
            palette: ModalPalette::resolve(env("GHOSTEX_NATIVE_MODAL_DEMO_THEME") == "light", None),
        }
    }
}

/// Opens one modal window the way the app does: centered, borderless, at the
/// modal's first-frame size, with a gpui-component `Root` as the window root.
fn open_modal_window<V: Render>(
    width: f32,
    initial_height: f32,
    build: impl FnOnce(&mut gpui::Window, &mut App) -> Entity<V>,
    cx: &mut App,
) -> (WindowHandle<Root>, Entity<V>) {
    let window_size = size(px(width), px(initial_height));
    let bounds = cx
        .primary_display()
        .map(|display| Bounds::centered_at(display.bounds().center(), window_size))
        .unwrap_or_else(|| Bounds::new(point(px(240.0), px(160.0)), window_size));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        focus: true,
        show: true,
        is_resizable: false,
        is_minimizable: false,
        titlebar: None,
        ..Default::default()
    };
    let slot: Rc<RefCell<Option<Entity<V>>>> = Rc::new(RefCell::new(None));
    let out = slot.clone();
    let window = cx
        .open_window(options, move |window, cx| {
            window.set_window_title("");
            window.activate_window();
            let view = build(window, cx);
            *out.borrow_mut() = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx).bg(gpui::transparent_black()))
        })
        .expect("open the demo window");
    let view = slot.borrow_mut().take().expect("the demo view");
    (window, view)
}

fn deliver_export_result(
    window: WindowHandle<Root>,
    view: Entity<GpuiExportTranscriptModalWindow>,
    ok: bool,
    cx: &mut App,
) {
    let _ = window.update(cx, |_root, window, cx| {
        view.update(cx, |modal, cx| {
            if ok {
                modal.receive_result(
                    true,
                    Some(FAKE_PATH.to_string()),
                    true,
                    Some("codex".to_string()),
                    None,
                    window,
                    cx,
                );
            } else {
                modal.receive_result(
                    false,
                    None,
                    false,
                    None,
                    Some("This session has no transcript yet. Send a prompt first.".to_string()),
                    window,
                    cx,
                );
            }
        });
    });
}

fn open_export_transcript(demo: &DemoEnv, cx: &mut App) {
    let initial_mode = match env("GHOSTEX_EXPORT_MODAL_DEMO_MODE").as_str() {
        "export" => Some(ExportTranscriptMode::Export),
        "handoff" => Some(ExportTranscriptMode::Handoff),
        _ => None,
    };
    let agents = if demo.state == "noagents" {
        Vec::new()
    } else {
        ["codex", "claude", "gemini"]
            .into_iter()
            .map(|id| ExportTranscriptAgent {
                agent_id: id.to_string(),
                name: {
                    let mut name = id.to_string();
                    name[..1].make_ascii_uppercase();
                    name
                },
            })
            .collect()
    };
    let slot: Rc<RefCell<Option<(WindowHandle<Root>, Entity<GpuiExportTranscriptModalWindow>)>>> =
        Rc::new(RefCell::new(None));
    let host_slot = slot.clone();
    let fail = demo.state == "failed";
    let host: ExportTranscriptModalHost = Rc::new(move |command, cx: &mut App| match command {
        ExportTranscriptModalCommand::RunExport(include) => {
            eprintln!(
                "run export: commands={} patches={} reasoning={}",
                include.commands, include.patches, include.reasoning
            );
            let slot = host_slot.clone();
            cx.spawn(async move |cx| {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let target = slot.borrow().clone();
                if let Some((window, view)) = target {
                    let _ = cx.update(|cx| deliver_export_result(window, view, !fail, cx));
                }
            })
            .detach();
        }
        ExportTranscriptModalCommand::StartConversation { agent_id } => {
            eprintln!("handoff to {agent_id}");
            cx.quit();
        }
        ExportTranscriptModalCommand::Cancel => {
            eprintln!("cancel");
            cx.quit();
        }
        ExportTranscriptModalCommand::Reveal => {
            eprintln!("reveal");
            cx.quit();
        }
    });
    let config = ExportTranscriptModalConfig {
        agents,
        default_agent_id: Some("codex".to_string()),
        palette: demo.palette,
        prefs_path: None,
        initial_mode,
    };
    let (window, view) = open_modal_window(
        EXPORT_TRANSCRIPT_MODAL_WIDTH,
        EXPORT_TRANSCRIPT_MODAL_INITIAL_HEIGHT,
        move |window, cx| {
            cx.new(|cx| GpuiExportTranscriptModalWindow::new(config, host, window, cx))
        },
        cx,
    );
    *slot.borrow_mut() = Some((window, view.clone()));
    if demo.state == "done" || demo.state == "failed" {
        let ok = demo.state == "done";
        cx.defer(move |cx| deliver_export_result(window, view, ok, cx));
    }
}

fn main() {
    let demo = DemoEnv::read();
    gpui_platform::application()
        .with_assets(assets::GhostexAssets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            match demo.modal.as_str() {
                "export-transcript" => open_export_transcript(&demo, cx),
                // DEMO-ARMS: one arm per converted modal.
                "update-available" => update_available_demo::open(&demo, cx),
                "remote-setup" => remote_setup_demo::open(&demo, cx),
                "delayed-send" => delayed_send_demo::open(&demo, cx),
                "space-editor" => space_editor_demo::open(&demo, cx),
                "create-worktree" => create_worktree_demo::open(&demo, cx),
                "remote-gxserver-install" => remote_gxserver_install_demo::open(&demo, cx),
                "portless-setup" => portless_setup_demo::open(&demo, cx),
                "delete-worktree" => delete_worktree_demo::open(&demo, cx),
                "rename-worktree" => rename_worktree_demo::open(&demo, cx),
                "missing-project-folder" => missing_project_folder_demo::open(&demo, cx),
                "rename-session" => rename_session_demo::open(&demo, cx),
                "session-note" => session_note_demo::open(&demo, cx),
                "install-hooks" => agent_hooks_required_demo::open(&demo, cx),
                other => {
                    eprintln!("unknown modal {other:?}");
                    cx.quit();
                    return;
                }
            }
            cx.activate(true);
        });
}
