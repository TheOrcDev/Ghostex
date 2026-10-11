//! What `start` and `start-server` need to know about a running gxserver: where its state and auth token live, its health, and how to stop its control plane.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::http::request_json;
use crate::util::{env_trimmed, home_dir, sleep_ms, xdg_state_root};

/// CDXC:ServerDaemon 2026-09-18 SEE-ALSO:
/// Mirrors the state_dir resolution in packages/paths (GhostexStoragePaths): GHOSTEX_HOME wins, native Windows keeps state under %LOCALAPPDATA%\Ghostex\State, and everything else follows XDG.
/// gxserver's own files sit under <state>/gxserver (server/src/paths.rs), so the token and runtime/server.json are both read from here.
pub fn ghostex_state_dir() -> PathBuf {
    if let Some(home) = explicit_ghostex_home() {
        return home.join("state");
    }
    if cfg!(windows) {
        return local_app_data().join("Ghostex").join("State");
    }
    xdg_state_root().join("ghostex")
}

pub fn explicit_ghostex_home() -> Option<PathBuf> {
    env_trimmed("GHOSTEX_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

pub fn local_app_data() -> PathBuf {
    env_trimmed("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join("AppData").join("Local"))
}

pub fn read_token() -> Option<String> {
    let mut token_paths = vec![ghostex_state_dir()
        .join("gxserver")
        .join("auth")
        .join("token")];
    if explicit_ghostex_home().is_none() {
        // Read-only upgrade compatibility before the app has had a chance to run the storage migration. Current XDG state always wins.
        let home = home_dir();
        token_paths
            .push(home.join("Library/Application Support/Ghostex/State/gxserver/auth/token"));
        token_paths.push(home.join(".ghostex/gxserver/auth/token"));
    }
    token_paths.iter().find_map(|path| {
        let token = fs::read_to_string(path).ok()?.trim().to_string();
        (!token.is_empty()).then_some(token)
    })
}

/// The health record when a gxserver answers at `base_url`.
pub fn health(base_url: &str, token: &str, timeout: Duration) -> Option<serde_json::Value> {
    let value = request_json(base_url, "GET", "/api/health/server", token, timeout)?;
    (value.get("product").and_then(|p| p.as_str()) == Some("gxserver")).then_some(value)
}

pub fn request_stop(base_url: &str, token: &str) {
    let _ = request_json(
        base_url,
        "POST",
        "/api/control/stop",
        token,
        Duration::from_secs(1),
    );
}

pub fn build_identity_of(health: &serde_json::Value) -> String {
    health
        .get("buildIdentity")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

/// The `buildIdentity` recorded in a gxserver package folder's build-identity.json.
pub fn read_build_identity(identity_file: &Path) -> Option<String> {
    let text = fs::read_to_string(identity_file).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&text).ok()?;
    let identity = parsed.get("buildIdentity")?.as_str()?.trim().to_string();
    (!identity.is_empty()).then_some(identity)
}

/// Waits until no gxserver answers at `base_url`.
pub fn wait_for_stop(base_url: &str, token: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if request_json(
            base_url,
            "GET",
            "/api/health/server",
            token,
            Duration::from_millis(500),
        )
        .is_none()
        {
            return true;
        }
        sleep_ms(100);
    }
    request_json(
        base_url,
        "GET",
        "/api/health/server",
        token,
        Duration::from_millis(500),
    )
    .is_none()
}
