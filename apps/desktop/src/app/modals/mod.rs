//! App modal windows, session modals, the shared-settings save fan-out, and modal messages and toasts, one sibling module per concern (each with its own `impl GhostexGpuiApp` block).

// C1 wave-4 extraction: `impl GhostexGpuiApp` methods moved verbatim out of
// main.rs (pure move; the only edit is the `pub(crate) ` visibility prefix the
// cross-module split requires). See docs/2026-08-22/repo-restructure/SPLITS.md C1.
//
// Cluster: app modal / titlebar panel windows and shared-settings save fan-out

mod modal_messages_and_toasts;
mod modal_window;
mod session_modals;
mod settings_save;
