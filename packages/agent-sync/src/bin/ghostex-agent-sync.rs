//! Developer entry point: print the scan report or the plan for the current
//! HOME as JSON. The user-facing verbs live in the `ghostex` CLI.

use ghostex_agent_sync::{build_plan, scan, PlanOptions, SyncScope};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let home = ghostex_agent_sync::scan::default_home().expect("HOME is not set");
    let verb = args.first().map(String::as_str).unwrap_or("status");
    let scope = SyncScope::parse(args.get(1).map(String::as_str));
    match verb {
        "status" => println!("{}", serde_json::to_string_pretty(&scan(&home)).unwrap()),
        "plan" => {
            let report = scan(&home);
            let options = PlanOptions {
                scope,
                ..PlanOptions::default()
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&build_plan(&home, &report, &options)).unwrap()
            );
        }
        other => {
            eprintln!("unknown verb {other}; use status or plan [agent-id]");
            std::process::exit(2);
        }
    }
}
