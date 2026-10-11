//! What the board draws from its tickets, computed once per change instead of once per frame.

/// Cached per `NativeKanbanState::revision`; see `NativeKanbanState::derived`.
pub(crate) struct KanbanDerived {
    pub(crate) revision: u64,
    /// Per column, in column order: indices into `tickets`, filtered and sorted.
    pub(crate) lanes: Vec<Vec<usize>>,
    pub(crate) filter_count: usize,
    pub(crate) nothing_matches: bool,
}
