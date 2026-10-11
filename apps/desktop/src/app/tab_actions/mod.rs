//! Agents and command tab creation, splitting, context menus, close and sleep actions, and keyboard and spatial focus moves, one sibling module per concern (each with its own `impl GhostexGpuiApp` block).

// C1 wave-4 extraction: `impl GhostexGpuiApp` methods moved verbatim out of
// main.rs (pure move; the only edit is the `pub(crate) ` visibility prefix the
// cross-module split requires). See docs/2026-08-22/repo-restructure/SPLITS.md C1.
//
// Cluster: agents/command tab creation, splitting, context menus, close/sleep actions

mod agents_tab_create_and_split;
mod command_pane_tabs;
mod focused_surface;
mod spatial_focus;
mod tab_context_menus;
