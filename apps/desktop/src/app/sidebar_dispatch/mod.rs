//! Sidebar runtime settings, host-message dispatch, native pickers and OS integration, sidebar state refreshes, and the sidebar divider and collapse, one sibling module per concern (each with its own `impl GhostexGpuiApp` block).

// C1 wave-4 extraction: `impl GhostexGpuiApp` methods moved verbatim out of
// main.rs (pure move; the only edit is the `pub(crate) ` visibility prefix the
// cross-module split requires). See docs/2026-08-22/repo-restructure/SPLITS.md C1.
//
// Cluster: sidebar runtime settings, host-message dispatch, sidebar divider/collapse

mod app_modal_pickers;
mod folder_pickers_and_os_integration;
mod runtime_settings_and_host_messages;
mod sidebar_divider;
mod sidebar_state_refresh;
