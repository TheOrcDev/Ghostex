use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use super::serve::FILE_LINK_ROUTE_PREFIX;

/// What a path segment must escape; everything else stays readable in the address bar.
const PATH_SEGMENT: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'\\')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

/// Folders a link may read, by token. Tokens live in memory only, so every link stops working
/// when gxserver restarts.
#[derive(Default)]
struct Registry {
    roots: HashMap<String, PathBuf>,
    tokens: HashMap<PathBuf, String>,
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

/// CDXC:Docs 2026-10-01 WHY:
/// The link's folder is the whole root the file was opened from (the project or the mounted Docs directory), not the file's own folder, so an HTML page's `../shared.css` and a Markdown file's `../images/x.png` still load. The unguessable token in the path is the only grant: the browser has no gxserver auth token, and one token per root keeps the table from growing with every click.
pub(crate) fn link_path(root: &Path, relative: &str) -> String {
    let mut registry = registry().lock().unwrap_or_else(|error| error.into_inner());
    let token = match registry.tokens.get(root) {
        Some(token) => token.clone(),
        None => {
            let token = uuid::Uuid::new_v4().simple().to_string();
            registry.roots.insert(token.clone(), root.to_path_buf());
            registry.tokens.insert(root.to_path_buf(), token.clone());
            token
        }
    };
    let encoded = relative
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| percent_encoding::utf8_percent_encode(segment, PATH_SEGMENT).to_string())
        .collect::<Vec<_>>()
        .join("/");
    format!("{FILE_LINK_ROUTE_PREFIX}{token}/{encoded}")
}

pub(super) fn root_for_token(token: &str) -> Option<PathBuf> {
    registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .roots
        .get(token)
        .cloned()
}
