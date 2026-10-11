//! The Docs resource scope: which files a Docs document may load (images, stylesheets, scripts,
//! media) and reading them. Plain Rust shared by the native Files view, which reads images and
//! rendered blocks through it, and the CEF resource origin that serves the same files to the embed
//! page (`cef/shell/request_handling.rs`), so the Files view works without CEF installed.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub(crate) type ManageDocsRemoteResourceLoader = Arc<dyn Fn(&str) -> Option<Vec<u8>> + Send + Sync>;

/*
CDXC:Docs 2026-08-09:
The local Docs root is a configurable folder now, and resolving it reads the
project's Docs directory from the daemon. That resolution must not run on the
main thread while a CEF surface is being created, so the scope carries a
resolver that runs on the same blocking-capable worker sequence as the file
open, memoized so one document's images cost one lookup.

CDXC:Docs 2026-08-09: Docs serves TWO roots — the project's own and
the mounted Docs directory — so the resolver answers with every mount, each
carrying the path segment that addresses it and the relative roots a resource
may live under inside it. Which mounts exist and what they allow both come out
of the same daemon lookup, so neither can be answered before it.
*/
pub(crate) type ManageDocsLocalRootResolver =
    Arc<dyn Fn() -> Option<Vec<ManageDocsResourceRoot>> + Send + Sync>;
pub(crate) type ManageDocsDynamicRootResolver =
    Arc<dyn Fn(&str) -> Option<ManageDocsResourceRoot> + Send + Sync>;

/// One mounted Docs root as the resource scope sees it: the reserved first path
/// segment that addresses it (empty for the project root, which owns bare
/// paths), the root itself, and the relative roots inside it a resource may
/// live under. An empty relative root means the whole tree.
#[derive(Clone)]
pub struct ManageDocsResourceRoot {
    pub allowed_relative_roots: Vec<String>,
    pub mount_segment: String,
    pub path: PathBuf,
}

#[derive(Clone)]
pub(crate) enum ManageDocsResourceSource {
    Local {
        resolve_dynamic_root: ManageDocsDynamicRootResolver,
        resolve_root: ManageDocsLocalRootResolver,
        /*
        CDXC:Docs 2026-08-10:
        Memoize only a successful resolution. The lookup reads the project's
        Docs directory from the daemon, so it can answer `None` for a reason
        that passes — daemon not reachable yet, project row not loaded. Sealing
        that first answer would leave every image and stylesheet in the document
        broken until the surface is recreated, with nothing shown to say why.
        */
        resolved_root: Arc<Mutex<Option<Vec<ManageDocsResourceRoot>>>>,
    },
    Remote {
        loader: ManageDocsRemoteResourceLoader,
    },
}

#[derive(Clone)]
pub struct ManageDocsResourceScope {
    pub(crate) source: ManageDocsResourceSource,
}

impl ManageDocsResourceScope {
    pub fn new(
        resolve_root: ManageDocsLocalRootResolver,
        resolve_dynamic_root: ManageDocsDynamicRootResolver,
    ) -> Self {
        Self {
            source: ManageDocsResourceSource::Local {
                resolve_dynamic_root,
                resolve_root,
                resolved_root: Arc::new(Mutex::new(None)),
            },
        }
    }

    pub fn new_remote(loader: ManageDocsRemoteResourceLoader) -> Self {
        Self {
            source: ManageDocsResourceSource::Remote { loader },
        }
    }
}

/// The file a Docs resource path names, when it lies inside a mounted Docs root that allows it.
/// Runs off the main thread: resolving the roots reads the project's Docs directory from the daemon.
pub(crate) fn resolve_manage_docs_local_resource(
    resolve_dynamic_root: &ManageDocsDynamicRootResolver,
    resolve_root: &ManageDocsLocalRootResolver,
    resolved_root: &Arc<Mutex<Option<Vec<ManageDocsResourceRoot>>>>,
    relative_path: &str,
) -> Option<PathBuf> {
    /*
    CDXC:Docs 2026-08-09:
    The requested path names its own root through the reserved mount
    segment, exactly as the Docs bridge routes it, so an image beside a
    note in the mounted Docs directory resolves there and a path can
    never be resolved against the root it did not name.
    */
    let mut mounts = {
        let mut resolved = resolved_root.lock().ok()?;
        if resolved.is_none() {
            *resolved = resolve_root();
        }
        resolved.clone()?
    };
    if let Some(dynamic_root) = resolve_dynamic_root(relative_path) {
        mounts.retain(|mount| mount.mount_segment != dynamic_root.mount_segment);
        mounts.push(dynamic_root);
    }
    // A named mount claims its own segment first; the project root owns
    // every path no mount claimed.
    let (mount, relative_path) = mounts
        .iter()
        .filter(|mount| !mount.mount_segment.is_empty())
        .find_map(|mount| {
            relative_path
                .strip_prefix(&format!("{}/", mount.mount_segment))
                .map(|inner| (mount, inner))
        })
        .or_else(|| {
            mounts
                .iter()
                .find(|mount| mount.mount_segment.is_empty())
                .map(|mount| (mount, relative_path))
        })?;
    let root = std::fs::canonicalize(&mount.path).ok()?;
    let candidate = std::fs::canonicalize(
        relative_path
            .split('/')
            .fold(root.clone(), |path, component| path.join(component)),
    )
    .ok()?;
    if !candidate.is_file() || !candidate.starts_with(&root) {
        return None;
    }
    let allowed = mount.allowed_relative_roots.iter().any(|relative_root| {
        let allowed_root = root.join(relative_root);
        std::fs::canonicalize(allowed_root)
            .ok()
            .is_some_and(|allowed_root| {
                allowed_root.starts_with(&root) && candidate.starts_with(allowed_root)
            })
    });
    if !allowed {
        return None;
    }
    Some(candidate)
}

/// A Docs resource's bytes, for the native Docs view (images). Same roots and rules as the page's
/// resource origin; runs on a background thread.
pub(crate) fn read_manage_docs_resource(
    scope: &ManageDocsResourceScope,
    relative_path: &str,
) -> Option<Vec<u8>> {
    match &scope.source {
        ManageDocsResourceSource::Local {
            resolve_dynamic_root,
            resolve_root,
            resolved_root,
        } => std::fs::read(resolve_manage_docs_local_resource(
            resolve_dynamic_root,
            resolve_root,
            resolved_root,
            relative_path,
        )?)
        .ok(),
        ManageDocsResourceSource::Remote { loader } => loader(relative_path),
    }
}
