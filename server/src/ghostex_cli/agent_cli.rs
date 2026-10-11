//! `ghostex agent-cli`: see, install, and update the agent CLIs on this computer.
//! The work happens in gxserver (`agent_cli::endpoint`), the same jobs Settings > Agents and onboarding run.

use crate::ghostex_cli::{
    args::parse_args,
    output::print_json,
    rpc::{call_gxserver_rpc, CliError, CliResult},
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const HELP: &str = "Install, update, and check the agent CLIs Ghostex runs.

USAGE
  ghostex agent-cli status [agent-id] [--json]
  ghostex agent-cli install <agent-id> [--method <id>] [--no-wait] [--json]
  ghostex agent-cli update <agent-id> [--no-wait] [--json]
  ghostex agent-cli add-to-path <agent-id> [--json]

  status lists every agent CLI Ghostex knows (installed version, latest version,
  path), or one agent in detail with its install methods. install uses the
  official installer by default (PowerShell on Windows); --method picks another
  listed method such as npm, brew, winget, or mise. update uses the method that
  installed the CLI. Both wait and print the installer's output unless --no-wait.
  add-to-path puts an installed CLI's folder on your PATH when its installer did
  not, so new terminals find it.

EXAMPLES
  ghostex agent-cli status
  ghostex agent-cli install claude
  ghostex agent-cli install codex --method npm
  ghostex agent-cli update grok
";

pub(crate) fn run(args: &[String]) -> CliResult<()> {
    let parsed = parse_args(args);
    let flags = &parsed.flags;
    if flags.contains("help") || flags.contains("h") || parsed.rest.is_empty() {
        print!("{HELP}");
        return Ok(());
    }
    let json_output = flags.contains("json");
    let command = parsed.rest[0].as_str();
    let agent_id = parsed.rest.get(1).map(String::as_str);
    let request = |params: Value| call_gxserver_rpc("/api/agentCliMaintenance", &params, flags);
    match (command, agent_id) {
        ("status", None) => {
            let result = request(json!({"action": "list"}))?;
            if json_output {
                print_json(&result);
                return Ok(());
            }
            println!("AGENT\tVERSION\tLATEST\tPATH");
            for state in result["agents"].as_array().into_iter().flatten() {
                let installed = state["executablePath"].as_str();
                let version = match installed {
                    Some(_) => text(state, "version", "installed"),
                    None => "not installed".to_string(),
                };
                let latest = match state["updateAvailable"].as_bool() {
                    Some(true) => format!("{} (update available)", text(state, "latestVersion", "")),
                    _ => text(state, "latestVersion", "-"),
                };
                println!(
                    "{}\t{version}\t{latest}\t{}",
                    text(state, "agentId", ""),
                    installed.unwrap_or("-")
                );
            }
        }
        ("status", Some(agent_id)) => {
            let state = request(json!({"action": "read", "agentId": agent_id}))?;
            if json_output {
                print_json(&state);
            } else {
                print_state(&state);
            }
        }
        ("install" | "update", Some(agent_id)) => {
            let state = request(json!({"action": "read", "agentId": agent_id}))?;
            let installed = state["executablePath"].is_string();
            if command == "install" && installed {
                return Err(CliError::Other(format!(
                    "{agent_id} is already installed at {}. Use `ghostex agent-cli update {agent_id}`.",
                    text(&state, "executablePath", "")
                )));
            }
            if command == "update" && !installed {
                return Err(CliError::Other(format!(
                    "{agent_id} is not installed. Use `ghostex agent-cli install {agent_id}`."
                )));
            }
            let method_id = match flags.text("method") {
                Some(method) => method,
                None if installed => state["detectedMethodId"]
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| {
                        CliError::Other(format!(
                            "Ghostex cannot tell how {agent_id} was installed. Pass --method (one of: {}).",
                            method_ids(&state)
                        ))
                    })?,
                None => state["methods"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|method| method["unavailableReason"].is_null())
                    .and_then(|method| method["id"].as_str())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        CliError::Other(format!(
                            "No install method for {agent_id} works on this computer yet ({}).",
                            unavailable_reasons(&state)
                        ))
                    })?,
            };
            let started = request(json!({
                "action": "start",
                "agentId": agent_id,
                "operation": command,
                "methodId": method_id,
            }))?;
            if flags.contains("no-wait") {
                if json_output {
                    print_json(&started);
                } else {
                    println!("Started: {}", text(&started["job"], "command", ""));
                }
                return Ok(());
            }
            let job_id = started["job"]["id"].as_str().unwrap_or_default().to_string();
            if !json_output {
                println!("> {}", text(&started["job"], "command", ""));
            }
            let mut printed = 0;
            let deadline = Instant::now() + Duration::from_secs(20 * 60);
            let finished = loop {
                std::thread::sleep(Duration::from_millis(1000));
                let state = request(json!({"action": "read", "agentId": agent_id}))?;
                let job = &state["job"];
                if job["id"].as_str() != Some(job_id.as_str()) {
                    return Err(CliError::Other("The install job disappeared; check `ghostex agent-cli status`.".into()));
                }
                let output = job["output"].as_str().unwrap_or_default();
                if !json_output && output.len() > printed {
                    let start = (printed..=output.len())
                        .find(|index| output.is_char_boundary(*index))
                        .unwrap_or(output.len());
                    print!("{}", &output[start..]);
                    printed = output.len();
                }
                if matches!(job["status"].as_str(), Some("succeeded" | "failed"))
                    || Instant::now() > deadline
                {
                    break state;
                }
            };
            if json_output {
                print_json(&finished);
            } else {
                println!();
                print_state(&finished);
            }
            if finished["job"]["status"] != "succeeded" {
                return Err(CliError::Other(text(
                    &finished["job"],
                    "error",
                    "The CLI operation did not finish.",
                )));
            }
        }
        ("add-to-path", Some(agent_id)) => {
            let state = request(json!({"action": "addToPath", "agentId": agent_id}))?;
            if json_output {
                print_json(&state);
            } else {
                print_state(&state);
            }
        }
        _ => {
            return Err(CliError::Other(format!("Unknown agent-cli command.\n\n{HELP}")));
        }
    }
    Ok(())
}

fn print_state(state: &Value) {
    println!("Agent: {}", text(state, "agentId", ""));
    match state["executablePath"].as_str() {
        Some(path) => {
            println!("Installed: {path}");
            println!("Version: {}", text(state, "version", "unknown"));
            if let Some(latest) = state["latestVersion"].as_str() {
                let note = if state["updateAvailable"] == true {
                    " (update available)"
                } else {
                    ""
                };
                println!("Latest: {latest}{note}");
            }
            if let Some(method) = state["detectedMethodId"].as_str() {
                println!("Installed with: {method}");
            }
            if let Some(directory) = state["pathDirectory"].as_str() {
                println!("Not on PATH: {directory} (run `ghostex agent-cli add-to-path {}`)", text(state, "agentId", ""));
            }
            if let Some(error) = state["versionError"].as_str() {
                println!("Version check failed: {error}");
            }
        }
        None => println!("Installed: no"),
    }
    for method in state["methods"].as_array().into_iter().flatten() {
        let reason = method["unavailableReason"]
            .as_str()
            .map(|reason| format!("  [{reason}]"))
            .unwrap_or_default();
        println!(
            "Method {}: {}{reason}",
            text(method, "id", ""),
            text(method, "command", "")
        );
    }
}

fn method_ids(state: &Value) -> String {
    state["methods"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|method| method["id"].as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn unavailable_reasons(state: &Value) -> String {
    state["methods"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|method| method["unavailableReason"].as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn text(value: &Value, key: &str, fallback: &str) -> String {
    value[key]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| fallback.to_string())
}
