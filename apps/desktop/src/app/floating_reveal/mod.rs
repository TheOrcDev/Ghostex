//! The sidebar and the sessions column, floating over the workarea while the pointer rests on the
//! window's left edge.
//!
//! The pieces are the same on every platform: a real 6px edge strip in the main window's body row
//! arms the reveal, one native child window carries the panels, and the pointer leaving that window
//! slides them away again. Only the host that owns the child window differs, because AppKit can
//! parent and clip a panel and the other two backends cannot.

pub(crate) mod edge_strip;
#[cfg(not(target_os = "macos"))]
mod host_desktop;
#[cfg(target_os = "macos")]
mod host_macos;
pub(crate) mod model;
pub(crate) mod window;
