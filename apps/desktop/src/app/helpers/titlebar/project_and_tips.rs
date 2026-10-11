use std::collections::HashSet;

use anyhow::Result;

use crate::*;

pub(crate) fn gpui_titlebar_project_selection_from_settings(
    settings: &serde_json::Map<String, serde_json::Value>,
    settings_key: &str,
    project_id: &str,
) -> Option<String> {
    settings
        .get(settings_key)
        .and_then(serde_json::Value::as_object)
        .and_then(|selections| selections.get(project_id))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| {
            !value.is_empty() && value.chars().count() <= GPUI_PROJECT_CONTRACT_STRING_MAX_CHARS
        })
        .map(str::to_string)
}

pub(crate) fn gpui_persist_titlebar_project_selection(
    settings_key: &str,
    project_id: &str,
    value: &str,
) -> Result<(), shared_settings::SharedSidebarSettingsWriteError> {
    let project_id = project_id.trim();
    let value = value.trim();
    if project_id.is_empty()
        || value.is_empty()
        || project_id.chars().count() > GPUI_PROJECT_CONTRACT_STRING_MAX_CHARS
        || value.chars().count() > GPUI_PROJECT_CONTRACT_STRING_MAX_CHARS
        || !matches!(
            settings_key,
            GPUI_TITLEBAR_OPEN_TARGET_SELECTIONS_SETTINGS_KEY
                | GPUI_TITLEBAR_ACTION_SELECTIONS_SETTINGS_KEY
        )
    {
        return Ok(());
    }

    let mut settings = shared_settings::shared_sidebar_settings_snapshot()
        .object()
        .clone();
    let selections = settings
        .entry(settings_key.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !selections.is_object() {
        *selections = serde_json::Value::Object(serde_json::Map::new());
    }
    let selections = selections
        .as_object_mut()
        .expect("titlebar selection map must be an object");
    if !selections.contains_key(project_id)
        && selections.len() >= GPUI_TITLEBAR_SELECTION_PROJECT_LIMIT
        && let Some(oldest_key) = selections.keys().next().cloned()
    {
        selections.remove(&oldest_key);
    }
    selections.insert(
        project_id.to_string(),
        serde_json::Value::String(value.to_string()),
    );
    shared_settings::write_shared_sidebar_settings_object(settings).map(|_| ())
}

pub(crate) fn gpui_titlebar_tips_read_ids_from_settings() -> HashSet<String> {
    shared_settings::shared_sidebar_settings_snapshot()
        .object()
        .get(GPUI_TITLEBAR_TIPS_READ_IDS_SETTINGS_KEY)
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .filter(|id| TITLEBAR_TIP_IDS.contains(id))
        .map(str::to_string)
        .collect()
}

pub(crate) fn gpui_titlebar_tips_unread_count_from_settings() -> u64 {
    let read_ids = gpui_titlebar_tips_read_ids_from_settings();
    GPUI_NATIVE_TITLEBAR_TIPS
        .iter()
        .filter(|tip| !read_ids.contains(tip.id))
        .count() as u64
}

pub(crate) fn gpui_mark_titlebar_tip_read(tip_id: &str) {
    if !TITLEBAR_TIP_IDS.contains(&tip_id) {
        return;
    }
    let mut settings = shared_settings::shared_sidebar_settings_snapshot()
        .object()
        .clone();
    let mut read_ids = gpui_titlebar_tips_read_ids_from_settings();
    read_ids.insert(tip_id.to_string());
    let ordered = TITLEBAR_TIP_IDS
        .iter()
        .filter(|id| read_ids.contains(**id))
        .map(|id| serde_json::Value::String((*id).to_string()))
        .collect::<Vec<_>>();
    settings.insert(
        GPUI_TITLEBAR_TIPS_READ_IDS_SETTINGS_KEY.to_string(),
        serde_json::Value::Array(ordered),
    );
    let _ = shared_settings::write_shared_sidebar_settings_object(settings);
}

pub(crate) fn titlebar_project_label_from_latest_sidebar_snapshot(
    latest_snapshot: Option<&GpuiProjectSnapshot>,
) -> String {
    /*
    CDXC:Titlebar 2026-06-22-19:57:
    The titlebar label is runtime-only sidebar state: use the latest valid snapshot display name and show the static Ghostex label before any valid sidebar payload arrives. Do not read env vars, repo folders, .git metadata, workspace names, fixture names, paths, URLs, sidebar titles, persisted state, or logs to infer the label.
    */
    latest_snapshot
        .map(|snapshot| snapshot.display_name.clone())
        .unwrap_or_else(|| TITLEBAR_PROJECT_LABEL_FALLBACK.to_string())
}
