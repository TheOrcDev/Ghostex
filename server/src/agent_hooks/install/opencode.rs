use std::{fs, path::Path};

use serde_json::{json, Value};

use crate::domain::DomainStateError;

use crate::agent_hooks::config::{HookPaths, OPENCODE_PLUGIN_MARKER, OPENCODE_PLUGIN_SPEC};
use crate::agent_hooks::plugin_sources::build_opencode_plugin_source;
use crate::agent_hooks::probing::{io_error, path_string, push_unique_path, read_file_text};
use crate::agent_hooks::resolution::{
    is_opencode_session_plugin_registration, provider_hook_paths,
    text_contains_ghostex_owned_hook_command,
};

use super::*;

pub(super) fn uninstall_opencode_hook(hook_paths: &HookPaths) -> Result<Vec<String>, DomainStateError> {
    let paths = provider_hook_paths("opencode", hook_paths);
    let Some(plugin_path) = paths.first() else {
        return Ok(Vec::new());
    };
    let Some(config_path) = paths.get(1) else {
        return Ok(Vec::new());
    };
    let directory = config_path.parent().unwrap();
    let mut removed = super::opencode_v2::uninstall(directory)?;
    removed.extend(uninstall_opencode_hook_paths(&directory.join("plugins/ghostex-session.js"), &directory.join("opencode.json"))?);
    Ok(removed)
}

pub(crate) fn uninstall_opencode_hook_paths(
    plugin_path: &Path,
    config_path: &Path,
) -> Result<Vec<String>, DomainStateError> {
    let mut removed_paths = Vec::new();
    let config_text = read_file_text(config_path);
    if !config_text.trim().is_empty() {
        let mut data = read_json_object(&config_text);
        if let Some(object) = data.as_object_mut() {
            if let Some(plugins) = object.get_mut("plugin").and_then(Value::as_array_mut) {
                let next_plugins = plugins
                    .iter()
                    .filter(|plugin| !is_opencode_session_plugin_registration(plugin))
                    .cloned()
                    .collect::<Vec<_>>();
                if next_plugins.len() != plugins.len() {
                    *plugins = next_plugins;
                    write_json_file(config_path, &data)?;
                    push_unique_path(&mut removed_paths, path_string(config_path));
                }
            }
        }
    }
    let plugin_text = read_file_text(plugin_path);
    if (plugin_text.contains(OPENCODE_PLUGIN_MARKER)
        || text_contains_ghostex_owned_hook_command(&plugin_text))
        && remove_file_if_exists(plugin_path)?
    {
        push_unique_path(&mut removed_paths, path_string(plugin_path));
    }
    Ok(removed_paths)
}

pub(super) fn install_opencode_hook(hook_paths: &HookPaths) -> Result<Vec<String>, DomainStateError> {
    let paths = provider_hook_paths("opencode", hook_paths);
    let Some(plugin_path) = paths.first() else {
        return Ok(Vec::new());
    };
    let Some(config_path) = paths.get(1) else {
        return Ok(Vec::new());
    };
    if super::opencode_v2::installed(hook_paths) {
        return super::opencode_v2::install(hook_paths, config_path.parent().unwrap());
    }
    if let Some(parent) = plugin_path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    fs::write(
        plugin_path,
        build_opencode_plugin_source(&hook_paths.notify_hook_path),
    )
    .map_err(io_error)?;
    update_opencode_config_plugin_registration(config_path)?;
    Ok(vec![path_string(plugin_path)])
}

fn update_opencode_config_plugin_registration(config_path: &Path) -> Result<(), DomainStateError> {
    let mut data = read_json_object(&read_file_text(config_path));
    let object = ensure_json_object(&mut data);
    let plugins = object
        .get("plugin")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut next_plugins = plugins
        .into_iter()
        .filter(|plugin| !is_opencode_session_plugin_registration(plugin))
        .collect::<Vec<_>>();
    next_plugins.push(json!(OPENCODE_PLUGIN_SPEC));
    object.insert("plugin".to_string(), Value::Array(next_plugins));
    write_json_file(config_path, &data)
}
