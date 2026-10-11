use std::{collections::HashSet, fs, path::Path};

use super::*;

/*
CDXC:Docs 2026-08-09:
Docs folders is project-root-relative again, the meaning it had before a custom
root ever existed. Round 2 made it narrow the custom root instead; with additive
mounting that is no longer coherent, because the mounted Docs directory always
shows its whole tree.
*/
fn additional_docs_folder_relative_paths(value: &str, docs_is_implicit_root: bool) -> Vec<String> {
    let mut folders = Vec::new();
    let mut seen = HashSet::new();
    for raw_folder in value.split(',') {
        let normalized = raw_folder.trim().replace('\\', "/");
        let parts = normalized
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.is_empty()
            || normalized.contains('\0')
            || normalized.starts_with('~')
            || normalized.starts_with('/')
            || parts.iter().any(|part| *part == "." || *part == "..")
        {
            continue;
        }
        let folder = parts.join("/");
        let key = folder.to_lowercase();
        if docs_is_implicit_root && BUILT_IN_DOCS_RELATIVE_PATHS.contains(&key.as_str()) {
            continue;
        }
        if seen.insert(key) {
            folders.push(folder);
        }
    }
    folders
}

fn is_built_in_docs_folder_name(name: &str) -> bool {
    BUILT_IN_DOCS_RELATIVE_PATHS
        .iter()
        .any(|folder| folder.eq_ignore_ascii_case(name))
}

fn is_skipped_first_level_docs_parent(name: &str) -> bool {
    name.starts_with('.')
        || is_built_in_docs_folder_name(name)
        || IGNORED_DIRECTORY_NAMES.contains(&name)
}

fn relative_path_segments(relative_path: &str) -> Vec<&str> {
    relative_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect()
}

/// Built-in Docs folders at the project root (`docs`, `artifacts`, `ai`, `tmp`)
/// and the same names one folder down (`folder/docs`).
fn path_is_in_built_in_scan_root(relative_path: &str) -> bool {
    let parts = relative_path_segments(relative_path);
    match parts.as_slice() {
        [] => false,
        [first, ..] if is_built_in_docs_folder_name(first) => true,
        [parent, folder, ..]
            if !is_skipped_first_level_docs_parent(parent)
                && is_built_in_docs_folder_name(folder) =>
        {
            true
        }
        _ => false,
    }
}

fn path_is_built_in_scan_root(relative_path: &str) -> bool {
    let parts = relative_path_segments(relative_path);
    match parts.as_slice() {
        [name] if is_built_in_docs_folder_name(name) => true,
        [parent, folder]
            if !is_skipped_first_level_docs_parent(parent)
                && is_built_in_docs_folder_name(folder) =>
        {
            true
        }
        _ => false,
    }
}

/// First-level project children that themselves contain a built-in Docs folder.
fn nested_built_in_docs_relative_paths(root: &Path) -> Vec<String> {
    let mut folders = Vec::new();
    let mut seen = HashSet::new();
    let Ok(entries) = fs::read_dir(root) else {
        return folders;
    };
    let mut parents = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if is_skipped_first_level_docs_parent(&name) {
                return None;
            }
            entry.metadata().ok().filter(|metadata| metadata.is_dir())?;
            Some(name)
        })
        .collect::<Vec<_>>();
    parents.sort_unstable();
    for parent in parents {
        let parent_path = root.join(&parent);
        let Ok(children) = fs::read_dir(&parent_path) else {
            continue;
        };
        let mut nested = children
            .filter_map(Result::ok)
            .filter_map(|child| {
                let name = child.file_name().to_string_lossy().to_string();
                if !is_built_in_docs_folder_name(&name) {
                    return None;
                }
                child.metadata().ok().filter(|metadata| metadata.is_dir())?;
                Some(name)
            })
            .collect::<Vec<_>>();
        nested.sort_unstable();
        for name in nested {
            let relative = format!("{parent}/{name}");
            if project_directory(root, &relative).is_none() {
                continue;
            }
            if seen.insert(relative.to_lowercase()) {
                folders.push(relative);
            }
        }
    }
    folders
}

/// The project root's scan roots: built-in Docs folders, each configured Docs
/// folder, and matching built-in folders one level down.
pub(super) fn scan_roots(root: &Path, additional_docs_folders: &str) -> Vec<String> {
    let mut roots = BUILT_IN_DOCS_RELATIVE_PATHS
        .iter()
        .map(|path| (*path).to_string())
        .collect::<Vec<_>>();
    roots.extend(additional_docs_folder_relative_paths(
        additional_docs_folders,
        true,
    ));
    let mut seen = roots
        .iter()
        .map(|path| path.to_lowercase())
        .collect::<HashSet<_>>();
    for nested in nested_built_in_docs_relative_paths(root) {
        let key = nested.to_lowercase();
        if roots.iter().any(|existing| {
            key == existing.to_lowercase()
                || key.starts_with(&format!("{}/", existing.to_lowercase()))
        }) {
            continue;
        }
        if seen.insert(key) {
            roots.push(nested);
        }
    }
    roots
}

fn path_is_in_scan_root(relative_path: &str, additional_docs_folders: &str) -> bool {
    if path_is_in_built_in_scan_root(relative_path) {
        return true;
    }
    additional_docs_folder_relative_paths(additional_docs_folders, true)
        .iter()
        .any(|root| relative_path == root || relative_path.starts_with(&format!("{root}/")))
}

pub(super) fn path_is_scan_root(relative_path: &str, additional_docs_folders: &str) -> bool {
    if path_is_built_in_scan_root(relative_path) {
        return true;
    }
    additional_docs_folder_relative_paths(additional_docs_folders, true)
        .iter()
        .any(|root| relative_path == root)
}

/// The nodes rename and move operations must preserve: the project root's scan
/// roots, and the mounted Docs directory itself.
pub(super) fn path_is_docs_root_node(path: &DocsPath<'_>, context: DocsContext<'_>) -> bool {
    if path.extra {
        return path.inner.is_empty();
    }
    path_is_scan_root(&path.inner, context.additional_docs_folders)
}

/// The extensions the Docs surface renders. One list for root artifacts and for
/// custom-root tree discovery, so the two can never drift apart.
pub(super) fn has_docs_artifact_extension(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, extension)| {
        ROOT_ARTIFACT_FILE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
    })
}

pub(super) fn is_root_artifact(relative_path: &str) -> bool {
    if relative_path.is_empty() || relative_path.contains('/') {
        return false;
    }
    has_docs_artifact_extension(relative_path)
}

/*
CDXC:Docs 2026-08-09:
The mounted Docs directory serves its whole tree, so a path that routed there
needs no further allowlist: it was already confined to that root by
canonicalization. Project-root paths keep exactly the allowlist they have
always had.
*/
pub(super) fn validate_accessible_path(
    path: &DocsPath<'_>,
    context: DocsContext<'_>,
) -> Result<(), String> {
    if path.extra
        || context.project_scope
        || path.inner == ANNOTATIONS_SIDECAR_RELATIVE_PATH
        || path_is_in_scan_root(&path.inner, context.additional_docs_folders)
        || is_root_artifact(&path.inner)
    {
        return Ok(());
    }
    Err("Files must be inside configured Docs folders or be root Markdown, HTML, or Excalidraw files.".to_string())
}

pub(super) fn validate_tree_path(
    path: &DocsPath<'_>,
    context: DocsContext<'_>,
) -> Result<(), String> {
    if path.extra
        || context.project_scope
        || path_is_in_scan_root(&path.inner, context.additional_docs_folders)
    {
        Ok(())
    } else {
        Err("Items must be inside configured Docs folders.".to_string())
    }
}

pub(super) fn validate_action_path(
    path: &DocsPath<'_>,
    context: DocsContext<'_>,
) -> Result<(), String> {
    if path.extra
        || context.project_scope
        || path_is_in_scan_root(&path.inner, context.additional_docs_folders)
        || is_root_artifact(&path.inner)
    {
        Ok(())
    } else {
        Err("Items must be inside configured Docs folders or be root Markdown, HTML, or Excalidraw files.".to_string())
    }
}

/// Two operations must never straddle the mount: a rename, duplicate, or move
/// that crosses roots is refused rather than silently rewriting one root's file
/// into the other.
pub(super) fn require_same_root(
    source: &DocsPath<'_>,
    destination: &DocsPath<'_>,
) -> Result<(), String> {
    if source.extra == destination.extra {
        return Ok(());
    }
    Err("Items cannot move between the project and the Docs directory.".to_string())
}
