use crate::app::helpers::*;
use crate::*;
use gpui_component::{WindowExt as _, notification::Notification};
use serde_json::json;

/// CDXC:Workarea 2026-09-22 WHY:
/// Keep the former extension's persisted tab identity while using the built-in project-view lifecycle. No installed manifest or extension process participates in this view.
pub(crate) fn storybook_view() -> GpuiCustomView {
    GpuiCustomView {
        id: ExtensionId::new("storybook").expect("built-in view ID"),
        title: "Storybook".into(),
        enabled: !shared_settings::shared_sidebar_settings_snapshot()
            .object()
            .get("storybookViewTabHidden")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        url: String::new(),
        definition: json!({
            "id": "storybook", "name": "Storybook", "availability": "matching",
            "source": {"kind": "report", "discovery": "storybook"}
        }),
    }
}

impl TitlebarMode {
    pub(crate) fn is_storybook(self) -> bool {
        matches!(self, Self::Extension(id) if id.as_str() == "storybook")
    }

    pub(crate) fn is_addon_view(self) -> bool {
        matches!(self, Self::Extension(_))
            && !self.is_storybook()
            && self.website_provider().is_none()
    }
}

impl GhostexGpuiApp {
    pub(crate) fn annotate_storybook(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let mode = TitlebarMode::Extension(storybook_view().id);
        if self.active_mode != mode || !self.titlebar_mode_available(mode) {
            return;
        }
        let script = browser_agentation_feedback_injection_script();
        let injected = ProjectWorkareaCefSurfaceSlotKey::for_titlebar_mode(mode)
            .and_then(|slot| self.project_workarea_runtime_cef_surfaces.get(&slot))
            .map(|owned| owned.surface.clone())
            .is_some_and(|surface| {
                surface.update(cx, |surface, _| {
                    surface.inject_feedback_tool_script(&script)
                })
            });
        if !injected {
            window.push_notification(
                Notification::warning("Wait for Storybook to finish loading before annotating."),
                cx,
            );
        }
        cx.notify();
    }
}
