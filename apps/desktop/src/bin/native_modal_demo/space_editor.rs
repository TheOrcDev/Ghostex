//! Demo host for the native New Space / Edit Space dialog.
//! States (`GHOSTEX_NATIVE_MODAL_DEMO_STATE`): `edit` (Edit Space with a name,
//! the Launch icon, the Blue swatch and the Delete action); anything else opens
//! the empty New Space dialog.
use super::space_editor_modal::*;
use gpui::{App, AppContext as _};
use std::rc::Rc;

pub(super) fn open(demo: &super::DemoEnv, cx: &mut App) {
    let edit = demo.state == "edit";
    let config = SpaceEditorModalConfig {
        mode: if edit {
            SpaceEditorMode::Edit
        } else {
            SpaceEditorMode::Create
        },
        initial_name: edit.then(|| "Client work".to_string()),
        initial_icon: edit.then(|| "rocket".to_string()),
        initial_color: edit.then(|| "#3f8fc7".to_string()),
        palette: demo.palette,
    };
    let host: SpaceEditorModalHost = Rc::new(|command, cx: &mut App| {
        match command {
            SpaceEditorModalCommand::Submit { name, icon, color } => {
                eprintln!("submit name={name:?} icon={icon} color={color}");
            }
            SpaceEditorModalCommand::Delete => eprintln!("delete"),
            SpaceEditorModalCommand::Cancel => eprintln!("cancel"),
        }
        cx.quit();
    });
    super::open_modal_window(
        SPACE_EDITOR_MODAL_WIDTH,
        SPACE_EDITOR_MODAL_INITIAL_HEIGHT,
        move |window, cx| cx.new(|cx| GpuiSpaceEditorModalWindow::new(config, host, window, cx)),
        cx,
    );
}
