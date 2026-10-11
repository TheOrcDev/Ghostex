use super::{arguments, delivery, identity, lifecycle};
use crate::ghostex_cli::{output::print_json, rpc::CliResult, sessions};
use serde_json::json;

pub(crate) fn run(args: &[String]) -> CliResult<()> {
    let args = arguments::parse(args)?;
    if args.help {
        print!("{}", include_str!("help.txt"));
        return Ok(());
    }
    match args.command.as_str() {
        "whoami" => {
            let row = identity::summary(&identity::caller()?);
            if args.json {
                print_json(&row);
            } else {
                for (label, key) in [
                    ("Session", "title"),
                    ("Session ID", "sessionId"),
                    ("Agent", "agentName"),
                    ("Agent ID", "agentId"),
                    ("Agent Session ID", "agentSessionId"),
                    ("Project", "projectName"),
                    ("Project ID", "projectId"),
                    ("Project path", "projectPath"),
                    ("Reply to", "globalRef"),
                ] {
                    println!("{label}: {}", identity::text(&row, key));
                }
            }
        }
        "list" => {
            let caller = if args.all {
                None
            } else {
                Some(identity::caller()?)
            };
            let flags = if let Some(caller) = &caller {
                identity::inventory_flags(&args.flags, identity::text(caller, "globalRef"))?
            } else {
                args.flags.clone()
            };
            let mut inventory = sessions::fetch_session_list(&flags, false)?;
            identity::resolve_names(&mut inventory, &flags);
            let rows: Vec<_> = inventory
                .iter()
                .filter(|row| {
                    identity::is_agent(row)
                        && caller
                            .as_ref()
                            .is_none_or(|caller| row["projectId"] == caller["projectId"])
                })
                .map(identity::summary)
                .collect();
            if args.json {
                print_json(&json!({"ok": true, "sessions": rows}));
            } else {
                println!("REFERENCE\tAGENT\tACTIVITY\tLIFECYCLE\tPROJECT\tTITLE");
                for row in rows {
                    println!(
                        "{}",
                        [
                            "globalRef",
                            "agentName",
                            "activity",
                            "lifecycleState",
                            "projectName",
                            "title"
                        ]
                        .map(|key| identity::text(&row, key).replace(['\n', '\r', '\t'], " "))
                        .join("\t")
                    );
                }
            }
        }
        "send" => {
            print_json(&delivery::send(&args)?);
        }
        "create" => {
            print_json(&lifecycle::create(&args)?);
        }
        "close" => {
            print_json(&lifecycle::close(&args)?);
        }
        "types" => {
            let result = lifecycle::types(&args)?;
            if args.json {
                print_json(&result);
            } else {
                println!("AGENT ID\tNAME\tCOMMAND");
                if let Some(rows) = result["agents"].as_array() {
                    for row in rows {
                        println!("{}", ["agentId", "name", "command"].map(|key| identity::text(row, key).replace(['\n', '\r', '\t'], " ")).join("\t"));
                    }
                }
            }
        }
        _ => unreachable!("validated command"),
    }
    Ok(())
}
