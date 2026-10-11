//! Preview fixtures for the Cloud Boxes Settings page: a scripted `/api/agentbox` with the data of
//! the Storybook `CloudBoxes` story (Docker ready, Hetzner logged in but not prepared, one
//! registered server, two boxes).
//!
//! States: `cloud-boxes` (the page), `cloud-boxes-add-server` (the Add Server form open),
//! `cloud-boxes-missing` (agentbox not installed), `cloud-boxes-error` (gxserver has no
//! `/api/agentbox`), `cloud-boxes-loading` (status never answers), `cloud-boxes-windows`
//! (`supported: false`), `cloud-boxes-offline` (no gxserver).
use serde_json::{Value, json};

pub(super) fn owns(state: &str) -> bool {
    state.starts_with("cloud-boxes")
}

pub(super) fn open_message(state: &str) -> Option<Value> {
    owns(state).then(|| json!({ "initialTab": "cloudBoxes" }))
}

pub(super) fn gxserver_rpc_available(state: &str) -> bool {
    owns(state) && state != "cloud-boxes-offline"
}

fn status(state: &str) -> Value {
    match state {
        "cloud-boxes-missing" => json!({ "supported": true, "installed": false, "providers": [] }),
        "cloud-boxes-windows" => json!({ "supported": false, "installed": false, "providers": [] }),
        _ => json!({
            "supported": true,
            "installed": true,
            "version": "0.33.0",
            "dockerReady": true,
            "portlessInstalled": false,
            "agentSignIns": { "claude": false, "codex": true },
            "providers": [
                { "id": "docker", "kind": "local", "ready": true, "configured": true, "prepared": true },
                { "id": "hetzner", "kind": "cloud", "ready": false, "configured": true, "prepared": false },
                { "id": "vercel", "kind": "cloud", "ready": false, "configured": false, "prepared": false },
                { "id": "daytona", "kind": "cloud", "ready": false, "configured": false, "prepared": false },
                { "id": "e2b", "kind": "cloud", "ready": false, "configured": false, "prepared": false },
                { "id": "digitalocean", "kind": "cloud", "ready": false, "configured": false, "prepared": false },
                { "id": "docker:selfhost", "kind": "remoteDocker", "label": "selfhost", "description": "Your server over SSH", "ready": true }
            ],
            "checkedAt": "2026-10-01T00:00:00Z",
            "error": null,
        }),
    }
}

fn boxes() -> Value {
    json!({
        "boxes": [
            {
                "name": "gx-ghostex-a1b2c3",
                "agent": "codex",
                "provider": "docker",
                "state": "running",
                "webUrl": "http://127.0.0.1:32772",
                "projectId": "project-ghostex",
                "sessionId": "session-1",
                "sessionTitle": "Fix the login page"
            },
            { "name": "gx-remote1", "agent": "claude", "provider": "remote-docker", "state": "paused" }
        ]
    })
}

/// The scripted `/api/agentbox`; `None` never answers (the loading state).
pub(super) fn rpc(
    state: &str,
    path: &str,
    params: &Value,
) -> Option<Option<Result<Value, String>>> {
    if path != "/api/agentbox" {
        return None;
    }
    if state == "cloud-boxes-error" {
        return Some(Some(Err(
            "gxserver does not know /api/agentbox yet. Update Ghostex.".to_string(),
        )));
    }
    Some(match params["action"].as_str() {
        Some("status") if state == "cloud-boxes-loading" => None,
        Some("status") => Some(Ok(status(state))),
        Some("list") => Some(Ok(boxes())),
        Some("openTarget") => Some(Ok(json!({ "url": "http://127.0.0.1:32772" }))),
        Some("stop" | "destroy") => Some(Ok(json!({ "ok": true, "output": "" }))),
        _ => Some(Err("Unknown agentbox action.".to_string())),
    })
}
