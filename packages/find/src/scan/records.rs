use serde::{Deserialize, Serialize};

use crate::agent::Agent;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub output: u64,
    #[serde(default)]
    pub cache_read: u64,
    #[serde(default)]
    pub cache_write: u64,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub context_window: u64,
    #[serde(default)]
    pub rate_percent: f64,
    #[serde(default)]
    pub cost: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub thinking: String,
    #[serde(default)]
    pub plan: String,
    #[serde(default)]
    pub usage: Usage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub agent: Agent,
    #[serde(default)]
    pub title: String,
    pub text: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub session: String,
    /// Session last-active unix seconds, 0 if unknown.
    pub ts: i64,
    #[serde(default)]
    pub meta: Meta,
}

impl Record {
    /// Title shown under a result: explicit title, else session id, else a placeholder.
    pub fn display_title(&self) -> &str {
        if !self.title.is_empty() {
            return &self.title;
        }
        if !self.session.is_empty() {
            return &self.session;
        }
        "Untitled session"
    }

    /// Last path segment of the project directory, or "".
    pub fn project_display_name(&self) -> &str {
        project_display_name(&self.project)
    }
}

pub fn project_display_name(project: &str) -> &str {
    if project.is_empty() {
        return "";
    }
    let trimmed = project.trim_end_matches(['/', '\\']);
    let trimmed = if trimmed.is_empty() { project } else { trimmed };
    match trimmed.rsplit(['/', '\\']).next() {
        Some(base) if !base.is_empty() => base,
        _ => trimmed,
    }
}
