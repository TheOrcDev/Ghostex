//! A Project Group's own menu items: Rename, the colour swatches, and Ungroup.
//!
//! These are the three `collectionAction` arms that edit the collections DOCUMENT rather than the
//! sidebar's own state. The other four (`toggle`, `select`, `toggleProjects`, `hide`) move what is
//! collapsed, selected and hidden and are sidebar-UI intents, which is why they are not here.
//!
//! Neither edit re-sanitizes, because `updateSidebarProjectCollection` and
//! `removeSidebarProjectCollection` do not: a colour the swatch row sent is stored verbatim and the
//! sanitizer that runs on every READ is what decides whether it is drawable. Sanitizing on the way
//! out would write a document the daemon and the phone would then disagree with about one colour.
//!
//! SEE-ALSO: packages/core-ui/project-collections.ts (deleted 2026-10-01),
//! the deleted sidebar page's `collections.ts` (`runNativeCollectionAction`),
//! apps/desktop/src/app/gx_store/collection_menu.rs.

use serde_json::Value;

use crate::sidebar_view::text::{js_trim, utf16_prefix};
use crate::sidebar_view::CollectionsState;

use super::collections::CollectionsDocument;

/// The `type` of the payload every arm below arrives on.
pub const COLLECTION_MENU_COMMAND_TYPE: &str = "collectionAction";

/// The three actions that write the document.
pub const COLLECTION_MENU_DOCUMENT_ACTIONS: &[&str] = &["color", "rename", "ungroup"];

/// Whether this payload is one [`plan_collection_menu_edit`] answers, without building anything.
pub fn owns_collection_menu_command(command: &Value) -> bool {
    if command.get("type").and_then(Value::as_str) != Some(COLLECTION_MENU_COMMAND_TYPE) {
        return false;
    }
    command
        .get("action")
        .and_then(Value::as_str)
        .is_some_and(|action| COLLECTION_MENU_DOCUMENT_ACTIONS.contains(&action))
}

/// The document after the menu item ran, or `None` when nothing is written.
///
/// `None` is `runNativeCollectionAction`'s `if (!collection) return`: an id from a menu the list has
/// since rebuilt names no collection, and the TypeScript writes nothing at all rather than writing
/// the document back unchanged. It is a REFUSAL and not an empty edit, which is why it is `None`
/// here where an unchanged document elsewhere in this module is still a write.
pub fn plan_collection_menu_edit(
    document: &CollectionsDocument,
    command: &Value,
) -> Option<CollectionsDocument> {
    if !owns_collection_menu_command(command) {
        return None;
    }
    let collection_id = command.get("collectionId").and_then(Value::as_str)?;
    let action = command.get("action").and_then(Value::as_str)?;
    if !document
        .state
        .collections
        .iter()
        .any(|collection| collection.collection_id == collection_id)
    {
        return None;
    }
    // `command.value` is absent for Ungroup and for the reset swatch, and `?? item.color` keeps the
    // current colour for the second of those. An absent value and an empty string are different
    // things for Rename and the same thing for the colour, exactly as `??` and `||` differ.
    let value = command.get("value").and_then(Value::as_str);
    let collections = match action {
        "ungroup" => document
            .state
            .collections
            .iter()
            .filter(|collection| collection.collection_id != collection_id)
            .cloned()
            .collect(),
        _ => document
            .state
            .collections
            .iter()
            .map(|collection| {
                if collection.collection_id != collection_id {
                    return collection.clone();
                }
                let mut next = collection.clone();
                match action {
                    // `command.value?.trim().slice(0, 80) || item.title`: JavaScript's trim and a
                    // UTF-16 slice, and a name the user cleared keeps the old one rather than
                    // leaving a nameless folder the sanitizer would then drop on the next read.
                    "rename" => {
                        let typed = value.map(|value| utf16_prefix(js_trim(value), 80).to_string());
                        next.title = typed
                            .filter(|title| !title.is_empty())
                            .unwrap_or(next.title);
                    }
                    _ => next.color = value.map(str::to_string).unwrap_or(next.color),
                }
                next
            })
            .collect(),
    };
    Some(CollectionsDocument {
        state: CollectionsState { collections },
        // Untouched by both arms: `removeSidebarProjectCollection` and
        // `updateSidebarProjectCollection` spread the state, so ungrouping "Group 7" does not free
        // the number 7 and the next folder is still "Group 8".
        next_collection_number: document.next_collection_number,
    })
}
