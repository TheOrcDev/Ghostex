//! Docs (Manage) page helpers: the Files bridge, docs roots and paths, the file tree, file
//! operations and git baselines, plus the terminal host shims that shared the original file.

// C1 wave-1 extraction: stateless helper functions moved verbatim out of
// main.rs (pure move, no logic changes). See docs/2026-08-22/repo-restructure/SPLITS.md C1.

pub(crate) mod docs_paths;
pub(crate) mod file_operations;
pub(crate) mod files_bridge;
pub(crate) mod git_baseline;
pub(crate) mod session_context;
pub(crate) mod terminal_host_shims;
pub(crate) mod tree_entries;

pub(crate) use docs_paths::*;
pub(crate) use file_operations::*;
pub(crate) use files_bridge::*;
pub(crate) use git_baseline::*;
pub(crate) use session_context::*;
pub(crate) use terminal_host_shims::*;
pub(crate) use tree_entries::*;
