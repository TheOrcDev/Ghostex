//! Project collections: the coloured folders projects are grouped into, and the top-level row
//! sequence they produce.
//!
//! SEE-ALSO: packages/core-ui/project-collections.ts (deleted 2026-10-01) and
//! packages/core-ui/sidebar-app/project-collection-model.ts (deleted 2026-10-01).

use std::collections::BTreeMap;

use ghostex_gx_protocol::SidebarProjectCollectionsState as WireCollectionsState;
use serde_json::Value;

use super::text::{js_trim, utf16_prefix};

/// The palette a collection without a valid colour falls back to, by position.
const COLLECTION_COLORS: &[&str] = &[
    "#4f5663", "#808080", "#7c6df2", "#3aa675", "#d6873f", "#d75b72", "#3f8fc7", "#b36ad4",
    "#8c9b45", "#c95353", "#c4a23d", "#2f9b95", "#596fd1",
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Collection {
    pub collection_id: String,
    pub title: String,
    pub color: String,
    pub project_ids: Vec<String>,
}

/// One machine's collections after client normalization.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CollectionsState {
    pub collections: Vec<Collection>,
}

/// One collection as it arrives, before the client sanitizer.
struct RawCollection<'a> {
    collection_id: &'a str,
    title: &'a str,
    color: &'a str,
    project_ids: Vec<&'a str>,
}

impl CollectionsState {
    /// `parseSidebarProjectCollectionsFromGxserver`: the order array first, then anything the map
    /// holds beyond it.
    pub fn from_wire(state: &WireCollectionsState) -> Self {
        let mut ordered_ids: Vec<&String> = Vec::new();
        for entry in &state.order {
            if state.collections.contains_key(entry) && !ordered_ids.contains(&entry) {
                ordered_ids.push(entry);
            }
        }
        // A collection the `order` array does not name follows in id order here, where
        // `Object.keys` gives the TypeScript the document's own order, which also moves the
        // positional colour fallback. Accepted rather than fixed: the wire type is a `BTreeMap`,
        // so the document order is gone before this runs, and every document the daemon writes
        // has an `order` array naming every collection it stores.
        for collection_id in state.collections.keys() {
            if !ordered_ids.contains(&collection_id) {
                ordered_ids.push(collection_id);
            }
        }
        Self::sanitize(
            ordered_ids
                .into_iter()
                .filter_map(|collection_id| {
                    let raw = state.collections.get(collection_id)?;
                    Some(RawCollection {
                        // The map key wins over the row's own id, as the spread in the
                        // TypeScript does.
                        collection_id,
                        title: &raw.title,
                        color: &raw.color,
                        project_ids: raw.project_ids.iter().map(String::as_str).collect(),
                    })
                })
                .collect(),
        )
    }

    /// `readSidebarProjectCollections`: the client-storage copy, an ordered array rather than the
    /// daemon's map.
    ///
    /// CDXC:Projects 2026-09-20 WHY:
    /// The sidebar seeds its collections from this key and shows them until the daemon's first document arrives, pushing them up rather than dropping them (the deleted React sidebar's `sidebar-app.tsx`, first adoption). A list built from the daemon alone would show no collections, no colours and another top-level order for the whole window between a cold start and that first echo, so the same seed is read here. From the first document on, the daemon is authoritative, an empty one included: the sidebar writes every adopted document straight back to this key.
    pub fn from_local_json(value: &Value) -> Self {
        let collections = value
            .get("collections")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        Self::sanitize(
            collections
                .iter()
                .filter_map(|raw| {
                    let text = |key: &'static str| raw.get(key).and_then(Value::as_str);
                    Some(RawCollection {
                        collection_id: text("collectionId")?,
                        title: text("title").unwrap_or_default(),
                        color: text("color").unwrap_or_default(),
                        project_ids: raw
                            .get("projectIds")
                            .and_then(Value::as_array)
                            .map(|ids| ids.iter().filter_map(Value::as_str).collect())
                            .unwrap_or_default(),
                    })
                })
                .collect(),
        )
    }

    /// `sanitizeSidebarProjectCollections`: bounded id, title and project ids, a colour from the
    /// palette when the stored one is not a hex value, a project in one collection only, and no
    /// empty collection.
    fn sanitize(raw_collections: Vec<RawCollection<'_>>) -> Self {
        let mut sanitized = Self::default();
        let mut seen_projects: Vec<String> = Vec::new();
        for raw in raw_collections {
            let id = utf16_prefix(js_trim(raw.collection_id), 120).to_string();
            let title = utf16_prefix(js_trim(raw.title), 80).to_string();
            let color = sanitize_color(raw.color, sanitized.collections.len());
            if id.is_empty()
                || title.is_empty()
                || sanitized
                    .collections
                    .iter()
                    .any(|collection| collection.collection_id == id)
            {
                continue;
            }
            let mut project_ids: Vec<String> = Vec::new();
            for project_id in raw.project_ids {
                let project_id = utf16_prefix(js_trim(project_id), 300).to_string();
                if project_id.is_empty() || seen_projects.contains(&project_id) {
                    continue;
                }
                seen_projects.push(project_id.clone());
                project_ids.push(project_id);
            }
            if project_ids.is_empty() {
                continue;
            }
            sanitized.collections.push(Collection {
                collection_id: id,
                title,
                color,
                project_ids,
            });
        }
        sanitized
    }

    /// `createProjectCollectionIdByProjectId`: the collection each project resolves to, including
    /// what a worktree inherits from its parent project.
    pub(crate) fn collection_id_by_project(
        &self,
        group_ids: &[String],
        project_of_group: &dyn Fn(&str) -> Option<String>,
        parent_project_of_group: &dyn Fn(&str) -> Option<String>,
    ) -> BTreeMap<String, String> {
        let mut result: BTreeMap<String, String> = BTreeMap::new();
        for collection in &self.collections {
            for project_id in &collection.project_ids {
                result.insert(project_id.clone(), collection.collection_id.clone());
            }
        }
        for group_id in group_ids {
            let project_id = project_of_group(group_id);
            let inherited =
                parent_project_of_group(group_id).and_then(|parent| result.get(&parent).cloned());
            if let (Some(project_id), Some(inherited)) = (project_id, inherited) {
                result.insert(project_id, inherited);
            }
        }
        result
    }
}

/// The colour rule of `sanitizeSidebarProjectCollections`, which tests the RAW value: unlike the
/// Space sanitizer beside it, a padded `" #abcdef "` falls back to the palette here, and the
/// stored spelling is kept rather than lowercased.
fn sanitize_color(value: &str, position: usize) -> String {
    let is_hex = value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit);
    if is_hex {
        return value.to_string();
    }
    if value == "transparent" {
        return COLLECTION_COLORS[0].to_string();
    }
    COLLECTION_COLORS[position % COLLECTION_COLORS.len()].to_string()
}

/// One top-level row: a project group, or a collection with the groups it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CollectionItem {
    Project {
        group_id: String,
    },
    Collection {
        collection: Collection,
        group_ids: Vec<String>,
    },
}

/// `projectSidebarCollections`: the drawn rows in the section's own project order, with each
/// collection placed where its first project sits.
pub(crate) fn project_sidebar_collections(
    section_group_ids: &[String],
    state: &CollectionsState,
    project_of_group: &dyn Fn(&str) -> Option<String>,
    parent_project_of_group: &dyn Fn(&str) -> Option<String>,
) -> Vec<CollectionItem> {
    let mut group_id_by_project: BTreeMap<String, String> = BTreeMap::new();
    let mut project_id_by_group: BTreeMap<String, String> = BTreeMap::new();
    for group_id in section_group_ids {
        if let Some(project_id) = project_of_group(group_id) {
            group_id_by_project.insert(project_id.clone(), group_id.clone());
            project_id_by_group.insert(group_id.clone(), project_id);
        }
    }
    let collection_id_by_project = state.collection_id_by_project(
        section_group_ids,
        project_of_group,
        parent_project_of_group,
    );

    let mut items: Vec<CollectionItem> = section_group_ids
        .iter()
        .filter(|group_id| !project_id_by_group.contains_key(*group_id))
        .map(|group_id| CollectionItem::Project {
            group_id: group_id.clone(),
        })
        .collect();
    let mut emitted: Vec<String> = Vec::new();
    for collection in &state.collections {
        let mut visible_project_ids: Vec<String> = collection
            .project_ids
            .iter()
            .filter(|project_id| group_id_by_project.contains_key(*project_id))
            .cloned()
            .collect();
        for group_id in section_group_ids {
            let Some(project_id) = project_id_by_group.get(group_id) else {
                continue;
            };
            if !visible_project_ids.contains(project_id)
                && collection_id_by_project.get(project_id) == Some(&collection.collection_id)
            {
                visible_project_ids.push(project_id.clone());
            }
        }
        if visible_project_ids.is_empty() {
            continue;
        }
        emitted.push(collection.collection_id.clone());
        let group_ids = visible_project_ids
            .iter()
            .filter_map(|project_id| group_id_by_project.get(project_id).cloned())
            .collect();
        items.push(CollectionItem::Collection {
            collection: Collection {
                project_ids: visible_project_ids,
                ..collection.clone()
            },
            group_ids,
        });
    }
    for group_id in section_group_ids {
        let Some(project_id) = project_id_by_group.get(group_id) else {
            continue;
        };
        if collection_id_by_project
            .get(project_id)
            .is_some_and(|collection_id| emitted.contains(collection_id))
        {
            continue;
        }
        items.push(CollectionItem::Project {
            group_id: group_id.clone(),
        });
    }
    let position_of = |item: &CollectionItem| -> usize {
        let ids: Vec<&String> = match item {
            CollectionItem::Project { group_id } => vec![group_id],
            CollectionItem::Collection { group_ids, .. } => group_ids.iter().collect(),
        };
        ids.into_iter()
            .map(|group_id| {
                section_group_ids
                    .iter()
                    .position(|candidate| candidate == group_id)
                    .unwrap_or(usize::MAX)
            })
            .min()
            .unwrap_or(usize::MAX)
    };
    items.sort_by_key(position_of);
    items
}
