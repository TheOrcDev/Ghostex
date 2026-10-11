//! Repository cloning (the Clone Repository flow): job types, the clone job runner,
//! preview, the git clone process, input parsing and destination checks, split by concern
//! from the former `repository_clone.rs`. Every submodule is glob re-exported here, so
//! callers keep using `repository_clone::…` unchanged. The module's unit tests stay
//! inline at the end of `destination.rs`.
mod clone_process;
mod destination;
mod input_parse;
mod jobs;
mod preview;
mod types;

use clone_process::*;
use destination::*;
pub(crate) use input_parse::*;
pub use jobs::*;
use preview::*;
pub use types::*;
