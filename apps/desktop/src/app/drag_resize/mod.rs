//! Tab and pane drag-and-drop feedback, command terminal attach, split handles and divider resize drags, one sibling module per concern (each with its own `impl GhostexGpuiApp` block).

// C1 wave-4 extraction: `impl GhostexGpuiApp` methods moved verbatim out of
// main.rs (pure move; the only edit is the `pub(crate) ` visibility prefix the
// cross-module split requires). See docs/2026-08-22/repo-restructure/SPLITS.md C1.
//
// Cluster: tab/pane drag-and-drop feedback, split handles, and divider resize drags

mod browser_and_workspace_tab_drag;
mod command_pane_controls;
mod command_tab_drag;
mod command_terminal_gxserver;
mod split_resize;
