//! Sidebar metadata commands, delayed sends, close-after-done timers and the app modal sidebar command switch, one sibling module per concern (each with its own `impl GhostexGpuiApp` block).

// C1 wave-4 extraction: `impl GhostexGpuiApp` methods moved verbatim out of
// main.rs (pure move; the only edit is the `pub(crate) ` visibility prefix the
// cross-module split requires). See docs/2026-08-22/repo-restructure/SPLITS.md C1.
//
// Cluster: sidebar metadata commands, delayed sends, close-after-done timers

mod agents_delayed_send;
mod app_modal_sidebar_command;
mod close_after_done;
mod command_delayed_send;
mod sidebar_metadata_commands;
