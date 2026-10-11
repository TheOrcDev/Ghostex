//! Local and remote workspace terminal lifecycle, attach, focus and rename plumbing, one sibling module per concern (each with its own `impl GhostexGpuiApp` block).

// C1 wave-4 extraction: `impl GhostexGpuiApp` methods moved verbatim out of
// main.rs (pure move; the only edit is the `pub(crate) ` visibility prefix the
// cross-module split requires). See docs/2026-08-22/repo-restructure/SPLITS.md C1.
//
// Cluster: local/remote workspace terminal lifecycle, attach, and rename plumbing

mod lifecycle;
mod open_local_terminal;
mod sidebar_deliveries;
mod surfaced_terminals;
