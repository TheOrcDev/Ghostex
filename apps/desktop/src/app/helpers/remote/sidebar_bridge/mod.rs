//! Remote sidebar RPC bridge: the request allowlist and validators, the request param shapers, and
//! the response payload builders.

// C1 wave-1 deferred split: apps/desktop/src/app/helpers/remote.rs (~8.2k
// lines) further divided into responsibility-scoped submodules (pure move,
// no logic changes). This file holds the remote sidebar RPC request/response
// param and payload builders. See docs/2026-08-22/repo-restructure/SPLITS.md C1.

pub(crate) mod project_and_git_payloads;
pub(crate) mod request_allowlist;
pub(crate) mod request_params;
pub(crate) mod response_payloads;

pub(crate) use project_and_git_payloads::*;
pub(crate) use request_allowlist::*;
pub(crate) use request_params::*;
pub(crate) use response_payloads::*;
