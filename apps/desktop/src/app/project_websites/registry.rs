use crate::app::helpers::*;
use crate::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::OnceLock;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebsiteProvider {
    pub id: String,
    pub title: String,
    pub hidden_settings_key: String,
    pub hidden_by_default: bool,
    pub workspace_kind: String,
    pub placeholder: String,
    pub description: String,
}

pub(crate) fn website_providers() -> &'static [WebsiteProvider] {
    static PROVIDERS: OnceLock<Vec<WebsiteProvider>> = OnceLock::new();
    PROVIDERS.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../../packages/shared/project-website-providers.json"
        ))
        .expect("built-in website provider catalog")
    })
}

pub(crate) fn website_provider(id: ExtensionId) -> Option<&'static WebsiteProvider> {
    website_providers()
        .iter()
        .find(|provider| provider.id == id.as_str())
}

impl WebsiteProvider {
    pub(crate) fn automatic(&self) -> bool {
        self.workspace_kind == "repository"
    }

    pub(crate) fn workspace(&self, home: &str) -> Result<(String, String), String> {
        let home = home.trim();
        if home.len() > 8192 {
            return Err("This URL is too long.".into());
        }
        let url = gpui::http_client::Url::parse(home)
            .map_err(|_| "Paste a complete website URL, starting with https://.".to_string())?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("Use an HTTP or HTTPS website URL without sign-in details.".into());
        }
        if self.workspace_kind == "linear" {
            let workspace = url
                .path_segments()
                .and_then(|mut parts| parts.find(|part| !part.is_empty()))
                .unwrap_or_default();
            if url.host_str() != Some("linear.app") || workspace.is_empty() {
                return Err("Paste a Linear URL that includes your workspace.".into());
            }
            Ok((
                format!("{}/{workspace}", url.origin().ascii_serialization()),
                workspace.into(),
            ))
        } else {
            Ok((
                url.origin().ascii_serialization(),
                match url.port() {
                    Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
                    None => url.host_str().unwrap_or_default().into(),
                },
            ))
        }
    }
}

pub(crate) fn website_views() -> Vec<GpuiCustomView> {
    let settings = shared_settings::shared_sidebar_settings_snapshot();
    website_providers().iter().map(|provider| GpuiCustomView {
        id: ExtensionId::new(&provider.id).expect("built-in website ID"),
        title: provider.title.clone(),
        enabled: !settings.object().get(&provider.hidden_settings_key).and_then(Value::as_bool).unwrap_or(provider.hidden_by_default),
        url: String::new(),
        definition: json!({"id":provider.id, "name":provider.title, "availability":if provider.automatic() {"matching"} else {"all"}, "source":{"kind":"website"}}),
    }).collect()
}

impl TitlebarMode {
    pub(crate) fn website_provider(self) -> Option<&'static WebsiteProvider> {
        match self {
            Self::Extension(id) => website_provider(id),
            _ => None,
        }
    }
}

impl GhostexGpuiApp {
    pub(crate) fn website_parent_project_id(&self, project_id: &str) -> Option<String> {
        let parent = self
            .extension_projects
            .get(project_id)?
            .parent_project_id
            .as_ref()?;
        Some(
            gpui_remote_project_reference_from_project_id(project_id)
                .map(|remote| gpui_remote_scoped_project_id(&remote.remote_machine_id, parent))
                .unwrap_or_else(|| parent.clone()),
        )
    }

    /// CDXC:Workarea 2026-09-22 DECISION:
    /// User: each project and worktree can have a different website home, with worktrees following the parent until customized. Infer the workspace from the URL, preserve the entire home URL including filters, and resolve GitHub automatically from the repository.
    /// CDXC:Workarea 2026-09-23 WHY:
    /// Git origin is a live presentation fact, absent from the saved project records used by extension metadata. Use the active project's sanitized repository URL so GitHub appears as soon as its origin arrives, including remote projects and worktrees.
    pub(crate) fn website_home(
        &self,
        provider: &WebsiteProvider,
        project_id: &str,
    ) -> Option<String> {
        if provider.automatic() {
            let snapshot = self.latest_sidebar_project_snapshot.as_ref()?;
            if snapshot.active_project_id.as_ref()?.0 != project_id {
                return None;
            }
            let home = snapshot.browser_home_url.as_ref()?;
            let url = gpui::http_client::Url::parse(home).ok()?;
            return (url.host_str() == Some("github.com")).then(|| home.clone());
        }
        let settings = shared_settings::shared_sidebar_settings_snapshot();
        let homes = settings
            .object()
            .get("projectWebsiteViews")?
            .get(&provider.id)?
            .get("homes")?;
        let own = homes.get(project_id).and_then(Value::as_str);
        let inherited = self
            .website_parent_project_id(project_id)
            .and_then(|parent| homes.get(&parent).and_then(Value::as_str));
        own.or(inherited)
            .filter(|url| provider.workspace(url).is_ok())
            .map(str::to_string)
    }

    pub(crate) fn website_home_setup_visible(&self, mode: TitlebarMode) -> bool {
        let Some(provider) = mode
            .website_provider()
            .filter(|provider| !provider.automatic())
        else {
            return false;
        };
        let TitlebarMode::Extension(id) = mode else {
            return false;
        };
        self.website_home_editor_is_open(id)
            || self
                .active_project_id_for_view_scope()
                .is_some_and(|project| self.website_home(provider, &project).is_none())
    }

    pub(crate) fn website_home_editor_is_open(&self, id: ExtensionId) -> bool {
        self.project_views
            .website_editor
            .as_ref()
            .is_some_and(|editor| {
                editor.view_id == id
                    && self.active_project_id_for_view_scope().as_deref()
                        == Some(&editor.project_id)
            })
    }
}
