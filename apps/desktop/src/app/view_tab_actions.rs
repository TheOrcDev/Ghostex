//! Popping a view out of the window. Reordering and pinning live in `view_strip_order.rs`.

use crate::app::helpers::*;
use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    /// CDXC:Workarea 2026-09-20 DECISION:
    /// User (screen 09): "Pop out (next to expand) moves the view to its own window, for a second
    /// monitor." Every view Ghostex can pop out is a page served over local HTTP (code-server for
    /// Code, gxserver for Kanban, Automate and Docs, the extension's own server, the tab's own
    /// address for Browser), so popping out hands that exact page to a window of its own instead of
    /// building a second CEF host inside the app. A view with no page yet cannot be popped out, and
    /// the control says so rather than opening an empty window.
    /// CDXC:Docs 2026-10-01 DECISION:
    /// User chose option 3A: Files is drawn natively and has no page of its own, so its Open Externally pops out the open HTML, Markdown or Excalidraw file through gxserver's file link (`native_docs/open_externally.rs`) instead of staying disabled; its embed page is never handed out, because it only works inside Ghostex.
    pub(crate) fn view_can_pop_out(&self, mode: TitlebarMode) -> bool {
        if mode == TitlebarMode::Manage {
            return self.native_docs_external_file().is_some();
        }
        self.view_pop_out_url(mode).is_some()
    }

    fn view_pop_out_url(&self, mode: TitlebarMode) -> Option<String> {
        if mode == TitlebarMode::Manage {
            return None;
        }
        if mode == TitlebarMode::Browser {
            return self
                .browser_tabs
                .active_tab()
                .map(|tab| tab.url.clone())
                .filter(|url| url.starts_with("http://") || url.starts_with("https://"));
        }
        let slot = ProjectWorkareaCefSurfaceSlotKey::for_titlebar_mode(mode)?;
        self.project_workarea_runtime_url_for_slot(slot)
            .map(|runtime_url| runtime_url.value.clone())
            .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
    }

    pub(crate) fn pop_out_view(&mut self, mode: TitlebarMode, cx: &mut gpui::Context<Self>) {
        if mode == TitlebarMode::Manage {
            if let Some(path) = self.native_docs_external_file() {
                self.native_docs_open_externally(&path, None, cx);
            }
            return;
        }
        let Some(url) = self.view_pop_out_url(mode) else {
            return;
        };
        if let Err(message) = gpui_open_external_http_url(&url) {
            self.upsert_gpui_app_toast(
                GpuiAppToast {
                    copy_text: None,
                    id: "gpui-view-pop-out-failed".to_string(),
                    level: GpuiAppToastLevel::from_raw(Some("warning")),
                    title: format!("Could not pop out {}", mode.tab_label()),
                    description: Some(message),
                    loading: false,
                    persistent: false,
                    duration_ms: GPUI_APP_TOAST_DEFAULT_DURATION_MS,
                    epoch: 0,
                },
                cx,
            );
        }
    }
}
