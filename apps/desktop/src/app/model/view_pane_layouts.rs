use crate::*;

/// CDXC:Workarea 2026-09-21 DECISION:
/// User: the bottom panel needs to be separate from the state of the side panel whether shown or hidden; its state (minimized, open, closed) is related to the project.
/// So the two layouts remember only the sessions sidebar, and only while the advanced `sidebarVisibilityMemory` setting is per-view: `Agents` is the window with no view open, `Wide` is the window with a view panel beside the sessions, so Browser, Code and Docs never reshuffle between each other. The Commands pane keeps the mode saved in its own per-project model, and no view or project switch writes it.
/// Sidebar visibility is still remembered for the whole app, never per project, so switching projects cannot toggle the sidebar on its own.
/// This supersedes the 2026-09-12 decision that the Commands pane always follows the layout of the view.
/// SEE-ALSO: packages/shared/ghostex-settings/types.ts (deleted 2026-10-01), apps/desktop/src/app/view_pane_state.rs, apps/desktop/native/macos/GpuiSidebarReveal.m.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GpuiViewPaneLayoutKind {
    /// No view is open; the Agents workspace has the whole workarea.
    Agents,
    /// A view panel is open beside the Agents workspace.
    Wide,
}

impl GpuiViewPaneLayoutKind {
    pub(crate) fn for_mode(mode: TitlebarMode) -> Self {
        if mode == TitlebarMode::Agents {
            Self::Agents
        } else {
            Self::Wide
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GpuiSidebarVisibilityMemory {
    Shared,
    PerView,
}

impl GpuiSidebarVisibilityMemory {
    pub(crate) fn from_shared_settings(
        settings: &shared_settings::SharedSidebarSettingsSnapshot,
    ) -> Self {
        match settings.sidebar_visibility_memory() {
            shared_settings::SharedSidebarVisibilityMemory::Shared => Self::Shared,
            shared_settings::SharedSidebarVisibilityMemory::PerView => Self::PerView,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct GpuiViewPaneState {
    pub(crate) sidebar_collapsed: bool,
}

impl GpuiViewPaneState {
    pub(crate) fn default_for_kind(kind: GpuiViewPaneLayoutKind) -> Self {
        Self {
            sidebar_collapsed: kind == GpuiViewPaneLayoutKind::Wide,
        }
    }

    fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "sidebarCollapsed": self.sidebar_collapsed,
        })
    }

    fn from_json(value: &serde_json::Value) -> Option<Self> {
        let object = value.as_object()?;
        Some(Self {
            sidebar_collapsed: json_bool_field(object, "sidebarCollapsed")?,
        })
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct GpuiViewPaneLayouts {
    pub(crate) agents: GpuiViewPaneState,
    pub(crate) wide: GpuiViewPaneState,
    /// The single sidebar state used while `sidebarVisibilityMemory` is "shared".
    pub(crate) shared_sidebar_collapsed: bool,
}

impl GpuiViewPaneLayouts {
    pub(crate) fn shell_default() -> Self {
        Self {
            agents: GpuiViewPaneState::default_for_kind(GpuiViewPaneLayoutKind::Agents),
            wide: GpuiViewPaneState::default_for_kind(GpuiViewPaneLayoutKind::Wide),
            shared_sidebar_collapsed: false,
        }
    }

    pub(crate) fn get(&self, kind: GpuiViewPaneLayoutKind) -> GpuiViewPaneState {
        match kind {
            GpuiViewPaneLayoutKind::Agents => self.agents,
            GpuiViewPaneLayoutKind::Wide => self.wide,
        }
    }

    pub(crate) fn get_mut(&mut self, kind: GpuiViewPaneLayoutKind) -> &mut GpuiViewPaneState {
        match kind {
            GpuiViewPaneLayoutKind::Agents => &mut self.agents,
            GpuiViewPaneLayoutKind::Wide => &mut self.wide,
        }
    }

    pub(crate) fn to_shell_state_json(&self) -> serde_json::Value {
        serde_json::json!({
            "agents": self.agents.to_json(),
            "wide": self.wide.to_json(),
            "sharedSidebarCollapsed": self.shared_sidebar_collapsed,
        })
    }

    pub(crate) fn from_shell_state(value: &serde_json::Value) -> Option<Self> {
        let object = value.as_object()?;
        Some(Self {
            agents: GpuiViewPaneState::from_json(object.get("agents")?)?,
            wide: GpuiViewPaneState::from_json(object.get("wide")?)?,
            shared_sidebar_collapsed: json_bool_field(object, "sharedSidebarCollapsed")?,
        })
    }
}
