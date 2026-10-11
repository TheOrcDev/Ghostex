use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::domain::DomainStateError;

use crate::agent_hooks::config::all_hook_events;
use crate::agent_hooks::plugin_sources::{toml_basic_quote, yaml_double_quote};
use crate::agent_hooks::probing::{io_error, path_string, read_file_text};

use super::*;

pub(super) fn install_marked_yaml_hooks(
    config_paths: &[PathBuf],
    agent_id: &str,
    command: &str,
) -> Result<Vec<String>, DomainStateError> {
    let mut installed_paths = Vec::new();
    for config_path in config_paths {
        install_marked_yaml_hook(config_path, agent_id, command)?;
        installed_paths.push(path_string(config_path));
    }
    Ok(installed_paths)
}

fn install_marked_yaml_hook(
    config_path: &Path,
    agent_id: &str,
    command: &str,
) -> Result<(), DomainStateError> {
    let begin_marker = format!("# ghostex hooks {agent_id} begin");
    let end_marker = format!("# ghostex hooks {agent_id} end");
    let current_text = read_file_text(config_path);
    let current_lines = current_text
        .replace("\r\n", "\n")
        .split('\n')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut lines = without_marked_block(&current_lines, &begin_marker, &end_marker);
    if lines.last().map(|line| !line.trim().is_empty()) == Some(true) {
        lines.push(String::new());
    }
    if agent_id == "kimi" {
        lines.push(begin_marker.clone());
        lines.extend(kimi_hook_block_body(command));
        lines.push(end_marker);
    } else if agent_id == "hermes-agent" {
        let shell_command = hermes_hook_shell_command(command);
        merge_hermes_hook_block(&mut lines, &begin_marker, &end_marker, &shell_command);
    } else {
        lines.extend([
            begin_marker.clone(),
            "eventHooks:".to_string(),
            "  events:".to_string(),
            "    - name: on_complete".to_string(),
            "      commands:".to_string(),
            format!("        - command: {}", yaml_double_quote(command)),
            "    - name: on_error".to_string(),
            "      commands:".to_string(),
            format!("        - command: {}", yaml_double_quote(command)),
            "    - name: on_tool_permission".to_string(),
            "      commands:".to_string(),
            format!("        - command: {}", yaml_double_quote(command)),
            end_marker,
        ]);
    }
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    fs::write(
        config_path,
        format!("{}\n", lines.join("\n").trim_end_matches('\n')),
    )
    .map_err(io_error)?;
    if agent_id == "hermes-agent" {
        update_hermes_shell_hook_allowlist(config_path, Some(&hermes_hook_shell_command(command)))?;
    }
    Ok(())
}

/// The TOML body Ghostex writes between Kimi Code's marked-block markers: one
/// `[[hooks]]` array-of-tables entry per registered event, separated by a blank
/// line. No `matcher` key is written — Kimi treats `matcher` as a regex, and an
/// absent one already matches every tool.
fn kimi_hook_block_body(command: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for event_name in all_hook_events("kimi") {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push("[[hooks]]".to_string());
        lines.push(format!("event = {}", toml_basic_quote(event_name)));
        lines.push(format!("command = {}", toml_basic_quote(command)));
        lines.push("timeout = 10".to_string());
    }
    lines
}
