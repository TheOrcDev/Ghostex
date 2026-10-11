//! Where Ghostex keeps the tools it installs itself, and the PATH entries that expose them.

use std::path::{Path, PathBuf};

/// `~/.local/share/ghostex/tools` (the platform data folder elsewhere).
pub(crate) fn tools_root() -> PathBuf {
    ghostex_paths::GhostexPaths::resolve()
        .data_dir
        .join("tools")
}

/// Node.js as unpacked from its official archive.
pub(crate) fn node_dir() -> PathBuf {
    tools_root().join("node")
}

/// Where `node`, `npm`, `npx` and every `npm install -g` command live.
pub(crate) fn node_bin_dir() -> PathBuf {
    if cfg!(windows) {
        node_dir()
    } else {
        node_dir().join("bin")
    }
}

/// uv and uvx.
pub(crate) fn uv_dir() -> PathBuf {
    tools_root().join("uv")
}

/// Single-binary tools: bd, gh, glab.
pub(crate) fn bin_dir() -> PathBuf {
    tools_root().join("bin")
}

/// Every folder Ghostex adds to PATH for its tools, in lookup order.
pub(crate) fn path_dirs() -> Vec<PathBuf> {
    vec![bin_dir(), node_bin_dir(), uv_dir()]
}

pub(crate) fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Whether `path` is one of Ghostex's own tool installs (as opposed to one the user made).
pub(crate) fn is_ghostex_owned(path: &Path) -> bool {
    let root = tools_root();
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical(path).starts_with(canonical(&root)) || path.starts_with(&root)
}

/// CDXC:ManagedTools 2026-09-29 WHY:
/// gxserver runs `gh`, `glab`, `bd`, `npm` and friends by bare name in many places (source control discovery, pull requests, the Project board, typed operations), all inheriting its own PATH. Appending Ghostex's fixed tool folders to that PATH once at startup lets every one of them find a tool Ghostex installed, even one installed after the server started, instead of teaching each caller where the tools live. They go last so the user's own copies win (user decision 1A).
pub(crate) fn extend_process_path() {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut entries: Vec<PathBuf> = std::env::split_paths(&current).collect();
    for dir in path_dirs() {
        if !entries.contains(&dir) {
            entries.push(dir);
        }
    }
    if let Ok(joined) = std::env::join_paths(entries) {
        std::env::set_var("PATH", joined);
    }
}

/// gxserver's PATH without the folders `extend_process_path` added: what a login shell probe
/// starts from, so "is this folder on the user's PATH?" answers for a new terminal rather than
/// for gxserver.
pub(crate) fn process_path_without_tools() -> std::ffi::OsString {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let tools = path_dirs();
    std::env::join_paths(std::env::split_paths(&current).filter(|dir| !tools.contains(dir)))
        .unwrap_or(current)
}
