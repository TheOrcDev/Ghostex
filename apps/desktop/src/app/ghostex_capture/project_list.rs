//! How the Running Agents panel and the prompt's Send to list show projects: every project
//! collapsed to its header, one opened at a time, and a filter that searches project and session
//! names.
//!
//! CDXC:GhostexCapture 2026-09-30 DECISION:
//! User: "the list of sessions needs to be collapsed by default same way that the sidebar is
//! collapsed in the real sidebar", and "add ability to filter by typing on the sessions list and
//! let me select the project then look at the sessions in that project i dont want to see all
//! projects and all sessions at once in the list".

/// Which project is open and what the filter says, for one open list.
#[derive(Default)]
pub(crate) struct ListState {
    pub(crate) open: Option<String>,
    pub(crate) filter: String,
}

/// What one project draws.
pub(crate) enum Shown {
    /// Filtered out.
    Hidden,
    /// Its header only.
    Collapsed,
    /// Its header and these sessions (indices into its session list).
    Open(Vec<usize>),
}

impl ListState {
    pub(crate) fn filtering(&self) -> bool {
        !self.filter.trim().is_empty()
    }

    /// With no filter, only the opened project shows its sessions. With one, a project whose name
    /// matches shows all of them, another shows the sessions whose names match, and the rest are
    /// hidden.
    pub(crate) fn shown(&self, project_id: &str, title: &str, sessions: &[&str]) -> Shown {
        let filter = self.filter.trim().to_lowercase();
        if filter.is_empty() {
            return if self.open.as_deref() == Some(project_id) {
                Shown::Open((0..sessions.len()).collect())
            } else {
                Shown::Collapsed
            };
        }
        if title.to_lowercase().contains(&filter) {
            return Shown::Open((0..sessions.len()).collect());
        }
        let matching: Vec<usize> = sessions
            .iter()
            .enumerate()
            .filter(|(_, session)| session.to_lowercase().contains(&filter))
            .map(|(index, _)| index)
            .collect();
        if matching.is_empty() {
            Shown::Hidden
        } else {
            Shown::Open(matching)
        }
    }

    /// A click on a project's header: opens it (closing the one that was open), or closes it when
    /// it was open. While filtering, it clears the filter and opens that project.
    pub(crate) fn toggle(&mut self, project_id: &str) -> bool {
        let cleared = self.filtering();
        if cleared {
            self.filter.clear();
            self.open = Some(project_id.to_string());
        } else if self.open.as_deref() == Some(project_id) {
            self.open = None;
        } else {
            self.open = Some(project_id.to_string());
        }
        cleared
    }
}
