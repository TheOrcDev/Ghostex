//! Calls to gxserver's `/api/managedTools` (server/src/managed_tools/): the tools Ghostex installs
//! for the user. Blocking; run them on the background executor.

use std::time::{Duration, Instant};

use crate::app::helpers::*;
use crate::app::model::GpuiRemoteGxserverRequestTarget;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Homebrew with Apple's Command Line Tools is the slowest install Ghostex runs.
const JOB_TIMEOUT: Duration = Duration::from_secs(70 * 60);
const POLL_INTERVAL: Duration = Duration::from_millis(1500);

/// One `/api/managedTools` request to the local gxserver, or to `remote` when the tool belongs to
/// a remote computer (the project's or the Add Project machine's).
pub(crate) fn gpui_managed_tools_request(
    remote: Option<&GpuiRemoteGxserverRequestTarget>,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    match remote {
        Some(target) => {
            gpui_remote_gxserver_rpc_result(target, "/api/managedTools", params, REQUEST_TIMEOUT)
        }
        None => gpui_gxserver_rpc_result("/api/managedTools", params, REQUEST_TIMEOUT),
    }
}

/// The tool's current state (`ManagedToolState` in packages/shared/managed-tools.ts (deleted 2026-10-01)).
pub(crate) fn gpui_managed_tool_read(
    remote: Option<&GpuiRemoteGxserverRequestTarget>,
    tool: &str,
) -> Result<serde_json::Value, String> {
    gpui_managed_tools_request(
        remote,
        &serde_json::json!({ "action": "read", "tool": tool }),
    )
}

fn job_active(state: &serde_json::Value) -> bool {
    matches!(state["job"]["status"].as_str(), Some("queued" | "running"))
}

/// The last non-empty line of a job's output, for a one-line progress label.
pub(crate) fn gpui_managed_tool_progress_line(state: &serde_json::Value) -> Option<String> {
    state["job"]["output"]
        .as_str()?
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

/// Starts `operation` on `tool` and waits for its job, calling `progress` with each state read
/// while it runs. `Ok` is the final state of a succeeded job; `Err` the reason it did not run or
/// failed.
pub(crate) fn gpui_managed_tool_run(
    remote: Option<&GpuiRemoteGxserverRequestTarget>,
    tool: &str,
    operation: &str,
    mut progress: impl FnMut(&serde_json::Value),
) -> Result<serde_json::Value, String> {
    let started = gpui_managed_tools_request(
        remote,
        &serde_json::json!({ "action": "start", "tool": tool, "operation": operation }),
    )?;
    let job_id = started["job"]["id"].as_str().map(str::to_string);
    progress(&started);
    let deadline = Instant::now() + JOB_TIMEOUT;
    loop {
        std::thread::sleep(POLL_INTERVAL);
        let state = gpui_managed_tool_read(remote, tool)?;
        // A different job id means ours finished and another started; ours is then the past one.
        let same_job = job_id.is_none() || state["job"]["id"].as_str() == job_id.as_deref();
        if same_job && job_active(&state) {
            progress(&state);
            if Instant::now() >= deadline {
                return Err(
                    "The install is still running; check Settings > Integrations > Tools.".into(),
                );
            }
            continue;
        }
        return match state["job"]["status"].as_str() {
            Some("succeeded") | None => Ok(state),
            _ => Err(state["job"]["error"]
                .as_str()
                .filter(|error| !error.trim().is_empty())
                .map(str::to_string)
                .or_else(|| gpui_managed_tool_progress_line(&state))
                .unwrap_or_else(|| "The install did not finish.".to_string())),
        };
    }
}
