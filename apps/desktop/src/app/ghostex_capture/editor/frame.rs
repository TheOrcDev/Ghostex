//! Where the editor window was left, and opening a picture from the prompt box in it again.

use gpui::{App, Context};

use super::super::persistence;
use super::super::placement;
use crate::GhostexGpuiApp;

impl GhostexGpuiApp {
    /// Saves the open editor's place and size, so the next one opens the same.
    pub(super) fn remember_ghostex_capture_editor_frame(&mut self, cx: &mut Context<Self>) {
        let Some((handle, native)) = self
            .ghostex_capture
            .editor
            .as_ref()
            .map(|editor| (editor.handle, editor.native))
        else {
            return;
        };
        let Some(frame) = placement::live_frame(handle, native, cx) else {
            return;
        };
        let Some(saved) = placement::save_frame(frame, true, cx) else {
            return;
        };
        if self.ghostex_capture.saved.editor_frame.as_ref() == Some(&saved) {
            return;
        }
        self.ghostex_capture.saved.editor_frame = Some(saved);
        persistence::save(&self.ghostex_capture.saved);
    }

    /// Opens a picture from the prompt box in the editor again, with its marks still movable.
    /// A picture already in the editor goes to the prompt first.
    ///
    /// CDXC:GhostexCapture 2026-09-30 DECISION:
    /// User: "please allow clicking on an image to show it in edit again". Enter puts the edited
    /// picture back in its place in the prompt, under the same number.
    pub(crate) fn reopen_ghostex_capture_attachment(
        &mut self,
        number: u32,
        cx: &mut Context<Self>,
    ) {
        if self.ghostex_capture.editor.is_some() {
            self.commit_open_ghostex_capture_editor(cx);
            let app = cx.weak_entity();
            // The commit is deferred too; this runs after it has closed the editor.
            App::defer(cx, move |cx| {
                let _ = app.update(cx, |app, cx| {
                    if app.ghostex_capture.editor.is_none() {
                        app.reopen_ghostex_capture_attachment(number, cx);
                    }
                });
            });
            return;
        }
        let Some((source, path)) = self
            .ghostex_capture
            .prompt
            .attachments
            .iter()
            .find(|attachment| attachment.number == number)
            .map(|attachment| (attachment.source.clone(), attachment.path.clone()))
        else {
            return;
        };
        self.open_ghostex_capture_editor_on(source, path, Some(number), None, cx);
    }
}
