//! Agent activity status (working, attention, idle), split by concern. Every submodule is
//! glob re-exported here, so `crate::session_status::*` paths are unchanged.

use chrono::{SecondsFormat, TimeZone, Utc};
use serde_json::{Map, Value};

mod model;
#[cfg(test)]
mod tests;
mod title_classify;
mod title_transition;
mod transition;

pub use model::*;
use title_classify::*;
use title_transition::*;
pub use transition::*;
