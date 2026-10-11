//! The desktop floats the sidebar in its own native window when it is collapsed. A browser page has one canvas, so nothing ever constructs this; the type exists because the shared menu code looks for it.
use gpui::{Context, IntoElement, Render, WeakEntity, Window, div};

use crate::GhostexGpuiApp;

pub(crate) struct FloatingRevealWindow {
    pub(crate) app: WeakEntity<GhostexGpuiApp>,
}

impl Render for FloatingRevealWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
