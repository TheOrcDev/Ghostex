//! Rename Worktree preview. States (`GHOSTEX_NATIVE_MODAL_DEMO_STATE`):
//! default (branch prefill, branch rename on, one warning), `blocking` (a
//! reason that keeps Rename disabled), `nobranch` (a detached worktree with
//! the folder suffix prefilled and branch rename off), `plain` (no warnings).
//! Type into the field to see the validation and collision refusals; the
//! registered path `<parent>-settings-redesign` collides.
use super::rename_worktree_modal::*;
use gpui::{App, AppContext as _};
use std::rc::Rc;

pub(super) fn open(demo: &super::DemoEnv, cx: &mut App) {
    let state = demo.state.as_str();
    let draft = RenameWorktreeDraft {
        blocking_reason: (state == "blocking").then(|| {
            "This worktree has populated submodules; rename it after deinitializing them."
                .to_string()
        }),
        branch: (state != "nobranch").then(|| "feat/modal-gallery".to_string()),
        current_name: "feat-modal-gallery".to_string(),
        current_path: "/Users/you/dev/Ghostex-feat-modal-gallery".to_string(),
        parent_folder_name: "Ghostex".to_string(),
        parent_project_path: "/Users/you/dev/Ghostex".to_string(),
        project_id: "demo-worktree-project".to_string(),
        registered_project_paths: vec!["/Users/you/dev/Ghostex-settings-redesign".to_string()],
        rename_branch_default: state != "nobranch",
        warnings: if matches!(state, "plain" | "nobranch") {
            Vec::new()
        } else {
            vec!["A remote branch already exists and will keep its current name.".to_string()]
        },
        worktree_name: "Ghostex-feat-modal-gallery".to_string(),
    };
    let host: RenameWorktreeModalHost = Rc::new(|command, cx: &mut App| {
        match command {
            RenameWorktreeModalCommand::Cancel => eprintln!("cancel"),
            RenameWorktreeModalCommand::Rename {
                project_id,
                name,
                rename_branch,
            } => eprintln!("rename: {project_id} name={name:?} branch={rename_branch}"),
        }
        cx.quit();
    });
    let config = RenameWorktreeModalConfig {
        draft,
        palette: demo.palette,
    };
    let _ = super::open_modal_window(
        RENAME_WORKTREE_MODAL_WIDTH,
        RENAME_WORKTREE_MODAL_INITIAL_HEIGHT,
        move |window, cx| cx.new(|cx| GpuiRenameWorktreeModalWindow::new(config, host, window, cx)),
        cx,
    );
}
