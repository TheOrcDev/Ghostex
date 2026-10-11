use std::{
    fs,
    path::{Path, PathBuf},
};

use super::*;
use crate::app::helpers::*;

pub(crate) fn manage_run_git(arguments: &[&str], cwd: &Path) -> Option<(i32, Vec<u8>)> {
    let output = std::process::Command::new("/usr/bin/git")
        .args(arguments)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    Some((output.status.code().unwrap_or(-1), output.stdout))
}

pub(crate) fn manage_git_trimmed_output(stdout: &[u8]) -> String {
    String::from_utf8_lossy(stdout).trim().to_string()
}

pub(crate) fn manage_unavailable_git_baseline(reason: &str) -> serde_json::Value {
    serde_json::json!({
        "available": false,
        "baseText": serde_json::Value::Null,
        "headOid": serde_json::Value::Null,
        "maxBytesExceeded": serde_json::Value::Null,
        "reason": reason,
        "tracked": false,
    })
}

pub(crate) fn manage_renderable_git_baseline(
    base_text: Option<String>,
    head_oid: Option<&str>,
    max_bytes_exceeded: Option<bool>,
    reason: Option<&str>,
    tracked: bool,
) -> serde_json::Value {
    serde_json::json!({
        "available": true,
        "baseText": base_text,
        "headOid": head_oid,
        "maxBytesExceeded": max_bytes_exceeded,
        "reason": reason,
        "tracked": tracked,
    })
}

pub(crate) fn manage_git_baseline_payload(
    root: &Path,
    file: &Path,
    relative_path: &str,
) -> serde_json::Value {
    /*
    macOS `manageGitBaselinePayload` parity for meo's CodeMirror Git gutter:
    resolve the repo from the file's parent, reject repos outside the active
    project root, cap baseline text at 1 MB, and return sanitized enum-like
    reasons instead of stderr or filesystem paths.
    */
    if relative_path.is_empty() {
        return manage_unavailable_git_baseline("not-file");
    }
    let Some(parent) = file.parent() else {
        return manage_unavailable_git_baseline("not-repo");
    };
    let Some((exit_code, stdout)) = manage_run_git(&["rev-parse", "--show-toplevel"], parent)
    else {
        return manage_unavailable_git_baseline("git-unavailable");
    };
    if exit_code != 0 {
        return manage_unavailable_git_baseline("not-repo");
    }
    let repo_root_path = manage_git_trimmed_output(&stdout);
    if repo_root_path.is_empty() {
        return manage_unavailable_git_baseline("not-repo");
    }
    let Ok(repo_root) = fs::canonicalize(PathBuf::from(&repo_root_path)) else {
        return manage_unavailable_git_baseline("not-repo");
    };
    if !path_is_inside_or_equal(&repo_root, root) {
        return manage_unavailable_git_baseline("not-repo");
    }
    let Ok(git_path) = file.strip_prefix(&repo_root) else {
        return manage_unavailable_git_baseline("not-repo");
    };
    let git_path = git_path.to_string_lossy().to_string();
    if git_path.is_empty() {
        return manage_unavailable_git_baseline("not-repo");
    }

    let Some((ignore_exit, _)) =
        manage_run_git(&["check-ignore", "-q", "--", &git_path], &repo_root)
    else {
        return manage_unavailable_git_baseline("git-unavailable");
    };
    match ignore_exit {
        0 => return manage_unavailable_git_baseline("ignored"),
        1 => {}
        _ => return manage_unavailable_git_baseline("error"),
    }

    let Some((tracked_exit, _)) = manage_run_git(
        &["ls-files", "--error-unmatch", "--", &git_path],
        &repo_root,
    ) else {
        return manage_unavailable_git_baseline("git-unavailable");
    };
    let tracked = tracked_exit == 0;

    let head_oid = manage_run_git(&["rev-parse", "--verify", "HEAD"], &repo_root)
        .filter(|(exit_code, _)| *exit_code == 0)
        .map(|(_, stdout)| manage_git_trimmed_output(&stdout))
        .filter(|head_oid| !head_oid.is_empty());

    if !tracked || head_oid.is_none() {
        return manage_renderable_git_baseline(None, head_oid.as_deref(), None, None, tracked);
    }
    let head_oid = head_oid.unwrap();
    if let Some(cached) = ghostex_docs::baseline::get(&repo_root, &git_path, &head_oid) {
        return cached;
    }
    let head_spec = format!("{head_oid}:{git_path}");

    let Some((size_exit, size_stdout)) =
        manage_run_git(&["cat-file", "-s", &head_spec], &repo_root)
    else {
        return manage_renderable_git_baseline(None, Some(&head_oid), None, Some("error"), tracked);
    };
    if size_exit != 0 {
        return manage_renderable_git_baseline(None, Some(&head_oid), None, Some("error"), tracked);
    }
    if manage_git_trimmed_output(&size_stdout)
        .parse::<u64>()
        .is_ok_and(|size| size > MANAGE_GIT_BASELINE_MAX_BYTES as u64)
    {
        return manage_renderable_git_baseline(
            None,
            Some(&head_oid),
            Some(true),
            Some("too-large"),
            tracked,
        );
    }
    let Some((baseline_exit, baseline_stdout)) =
        manage_run_git(&["cat-file", "-p", &head_spec], &repo_root)
    else {
        return manage_renderable_git_baseline(None, Some(&head_oid), None, Some("error"), tracked);
    };
    if baseline_exit != 0 {
        return manage_renderable_git_baseline(None, Some(&head_oid), None, Some("error"), tracked);
    }
    if baseline_stdout.len() > MANAGE_GIT_BASELINE_MAX_BYTES {
        return manage_renderable_git_baseline(
            None,
            Some(&head_oid),
            Some(true),
            Some("too-large"),
            tracked,
        );
    }
    if baseline_stdout.contains(&0) {
        return manage_renderable_git_baseline(
            None,
            Some(&head_oid),
            None,
            Some("binary"),
            tracked,
        );
    }
    let baseline = manage_renderable_git_baseline(
        Some(String::from_utf8_lossy(&baseline_stdout).to_string()),
        Some(&head_oid),
        None,
        None,
        tracked,
    );
    ghostex_docs::baseline::insert(&repo_root, &git_path, &head_oid, &baseline);
    baseline
}
