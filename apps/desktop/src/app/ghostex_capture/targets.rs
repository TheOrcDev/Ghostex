//! Where the next prompt goes by default.

use crate::GhostexGpuiApp;
use crate::app::gx_store::CaptureTargetProject;

/// How long the project of the last prompt stays the default.
const RECENT_TARGET_MS: u64 = 8 * 60 * 60 * 1000;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Target {
    /// A new session in a project (`workspace_project_id`).
    NewSession { project: String, title: String },
    /// A running session (its sidebar session id).
    Session {
        session: String,
        title: String,
        project_title: String,
    },
}

impl Target {
    pub(crate) fn label(&self) -> String {
        match self {
            Target::NewSession { title, .. } => format!("+ New session · {title}"),
            Target::Session {
                title,
                project_title,
                ..
            } => format!("{title} · {project_title}"),
        }
    }
}

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

fn parse_ms(value: Option<&str>) -> i64 {
    value
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|time| time.timestamp_millis())
        .unwrap_or(0)
}

impl GhostexGpuiApp {
    pub(super) fn ghostex_capture_targets(&self) -> Vec<CaptureTargetProject> {
        self.gx_store_capture_targets(now_ms())
    }

    /// CDXC:GhostexCapture 2026-09-30 DECISION:
    /// User: a new session is the default target, in the project the last Ghostex Capture prompt
    /// went to, "but b if i didn't send prompts recently from capture": then the project of the
    /// most recently active session.
    pub(super) fn ghostex_capture_default_target(&self) -> Option<Target> {
        let projects = self.ghostex_capture_targets();
        let new_session = |project: &CaptureTargetProject| Target::NewSession {
            project: project.workspace_project_id.clone(),
            title: project.title.clone(),
        };
        if let Some(last) = &self.ghostex_capture.saved.last_target
            && now_ms().saturating_sub(last.at_ms) <= RECENT_TARGET_MS
            && let Some(project) = projects
                .iter()
                .find(|project| project.workspace_project_id == last.project_id)
        {
            return Some(new_session(project));
        }
        projects
            .iter()
            .filter_map(|project| {
                project
                    .sessions
                    .iter()
                    .map(|session| parse_ms(session.last_interaction_at.as_deref()))
                    .max()
                    .map(|latest| (latest, project))
            })
            .max_by_key(|(latest, _)| *latest)
            .map(|(_, project)| new_session(project))
            .or_else(|| projects.first().map(new_session))
    }
}
