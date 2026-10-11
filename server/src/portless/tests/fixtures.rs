use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};

use super::*;

pub(super) fn insert_project(db: &Connection, project_id: &str, name: &str) {
    insert_project_with_path(
        db,
        project_id,
        name,
        Some(&format!("/tmp/{project_id}")),
        "2026-06-22T18:41:00.000Z",
    );
}

pub(super) fn insert_project_with_path(
    db: &Connection,
    project_id: &str,
    name: &str,
    path: Option<&str>,
    created_at: &str,
) {
    db.execute(
        r#"
            INSERT INTO projects (projectId, name, path, createdAt, updatedAt)
            VALUES (?1, ?2, ?3, ?4, ?4)
            "#,
        params![project_id, name, path, created_at],
    )
    .expect("insert project");
}

pub(super) fn insert_worktree_project(
    db: &Connection,
    project_id: &str,
    parent_project_id: &str,
    display_name: &str,
    worktree_name: &str,
    branch: &str,
    created_at: &str,
) {
    db.execute(
        r#"
            INSERT INTO projects (projectId, name, path, worktreeJson, createdAt, updatedAt)
            VALUES (?1, ?2, ?3, ?4, ?5, ?5)
            "#,
        params![
            project_id,
            display_name,
            format!("/tmp/{project_id}"),
            worktree_json(parent_project_id, worktree_name, branch),
            created_at,
        ],
    )
    .expect("insert worktree project");
}

pub(super) fn insert_session_row(
    db: &Connection,
    project_id: &str,
    session_id: &str,
    zmx_name: &str,
    lifecycle_state: &str,
    persistence_provider: &str,
) {
    db.execute(
        r#"
            INSERT INTO sessions (
              projectId,
              sessionId,
              kind,
              title,
              lifecycleState,
              providerStateJson,
              zmxName,
              launchSettingsJson,
              runtimeSettingsJson,
              completionRulesJson,
              attentionRulesJson,
              notificationRulesJson,
              worktreeJson,
              createdAt,
              updatedAt
            )
            VALUES (?1, ?2, 'terminal', ?3, ?4, ?5, ?6, '{}', ?7, '{}', '{}', '{}', '{}', ?8, ?8)
            "#,
        params![
            project_id,
            session_id,
            format!("Session {session_id}"),
            lifecycle_state,
            serde_json::json!({ "lifecycleState": "exists", "provider": "zmx" }).to_string(),
            zmx_name,
            serde_json::json!({ "sessionPersistenceProvider": persistence_provider }).to_string(),
            "2026-06-22T18:43:00.000Z",
        ],
    )
    .expect("insert session");
}

pub(super) fn update_worktree_metadata(
    db: &Connection,
    project_id: &str,
    parent_project_id: &str,
    worktree_name: &str,
    branch: &str,
) {
    db.execute(
        r#"
            UPDATE projects
            SET name = ?2,
                worktreeJson = ?3,
                updatedAt = ?4
            WHERE projectId = ?1
            "#,
        params![
            project_id,
            worktree_name,
            worktree_json(parent_project_id, worktree_name, branch),
            "2026-06-22T18:45:00.000Z",
        ],
    )
    .expect("update worktree metadata");
}

fn worktree_json(parent_project_id: &str, worktree_name: &str, branch: &str) -> String {
    serde_json::json!({
        "branch": branch,
        "createdAt": "2026-06-22T18:42:00.000Z",
        "name": worktree_name,
        "parentProjectId": parent_project_id,
        "parentProjectName": "Parent Project",
        "parentProjectPath": "/tmp/parent-project"
    })
    .to_string()
}

pub(super) fn route(hostname: &str, port: u16, pid: u32) -> PortlessRoute {
    PortlessRoute {
        hostname: hostname.to_string(),
        port,
        pid,
    }
}

pub(super) fn owned_listener(
    project_id: &str,
    session_id: &str,
    zmx_name: &str,
    worktree_parent_project_id: Option<&str>,
    port: u16,
    pid: u32,
) -> PortlessOwnedListener {
    PortlessOwnedListener {
        project_id: project_id.to_string(),
        session_id: session_id.to_string(),
        zmx_name: zmx_name.to_string(),
        worktree_parent_project_id: worktree_parent_project_id.map(str::to_string),
        port,
        pid,
    }
}

pub(super) fn enable_debugging_mode_for_test(paths: &crate::paths::GxserverPaths) {
    let settings_path = paths.app_config_dir.join("native-sidebar-settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings dir");
    fs::write(
            settings_path,
            r#"{"debuggingMode":true,"diagnosticLogging":{"scenarios":{"gxserver.requests":{"enabled":true}},"version":1}}"#,
        )
        .expect("write debugging setting");
}

pub(super) fn assert_portless_log_text_has_no_forbidden_raw_values(text: &str) {
    for forbidden in [
        "Private Project",
        "Feature Worktree",
        "/Users/person/dev/private-project",
        "https://ghostex.localhost/private?token=SECRET",
        "ghostex.localhost",
        "npm run dev",
        "PORTLESS_STATE_DIR=/Users/person/.ghostex/gxserver/portless",
        "SECRET",
        "stdout payload",
        "stderr payload",
    ] {
        assert!(
            !text.contains(forbidden),
            "Portless log leaked forbidden raw value {forbidden}: {text}"
        );
    }
}

pub(super) fn service_expectation(
    home_dir: &Path,
    protocol: PortlessProtocol,
) -> PortlessServiceExpectation {
    PortlessServiceExpectation {
        home_dir: home_dir.to_path_buf(),
        expected_node_paths: vec![normalize_path_for_comparison(Path::new(
            "/Applications/Ghostex & Dev.app/Contents/Resources/Web/code-server/lib/node",
        ))],
        expected_cli_paths: vec![normalize_path_for_comparison(Path::new(
            "/Applications/Ghostex & Dev.app/Contents/Resources/Web/portless/dist/cli.js",
        ))],
        expected_state_dir: normalize_path_for_comparison(
            &home_dir.join(".ghostex").join("gxserver").join("portless"),
        ),
        protocol,
        proxy_port: portless_service_port_for_protocol(protocol),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn service_plist(
    node: &str,
    cli: &str,
    state_dir: &str,
    port: u16,
    https: bool,
    lan: bool,
    wildcard: bool,
    tld: Option<&str>,
    proxy_args: &[&str],
) -> String {
    service_plist_with_lan_ip(
        node, cli, state_dir, port, https, lan, wildcard, tld, None, proxy_args,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn service_plist_with_lan_ip(
    node: &str,
    cli: &str,
    state_dir: &str,
    port: u16,
    https: bool,
    lan: bool,
    wildcard: bool,
    tld: Option<&str>,
    lan_ip: Option<&str>,
    proxy_args: &[&str],
) -> String {
    let mut args = vec![
        node.to_string(),
        cli.to_string(),
        "proxy".to_string(),
        "start".to_string(),
    ];
    args.extend(proxy_args.iter().map(|arg| (*arg).to_string()));
    let mut env = vec![
        ("PORTLESS_STATE_DIR", state_dir.to_string()),
        ("PORTLESS_PORT", port.to_string()),
        ("PORTLESS_HTTPS", if https { "1" } else { "0" }.to_string()),
        ("PORTLESS_LAN", if lan { "1" } else { "0" }.to_string()),
        (
            "PORTLESS_WILDCARD",
            if wildcard { "1" } else { "0" }.to_string(),
        ),
        ("PORTLESS_SYNC_HOSTS", "0".to_string()),
    ];
    if let Some(tld) = tld {
        env.push(("PORTLESS_TLD", tld.to_string()));
    }
    if let Some(lan_ip) = lan_ip {
        env.push(("PORTLESS_LAN_IP", lan_ip.to_string()));
    }
    let args_xml = args
        .iter()
        .map(|arg| format!("    <string>{}</string>", test_xml_escape(arg)))
        .collect::<Vec<_>>()
        .join("\n");
    let env_xml = env
        .iter()
        .map(|(key, value)| {
            format!(
                "    <key>{}</key>\n    <string>{}</string>",
                test_xml_escape(key),
                test_xml_escape(value)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{}</string>
  <key>ProgramArguments</key>
  <array>
{}
  </array>
  <key>EnvironmentVariables</key>
  <dict>
{}
  </dict>
  <key>StandardOutPath</key>
  <string>/dev/null</string>
  <key>StandardErrorPath</key>
  <string>/dev/null</string>
</dict>
</plist>
"#,
        PORTLESS_SERVICE_LABEL, args_xml, env_xml
    )
}

fn test_xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(super) fn portless_state(
    enabled: bool,
    setup_ownership: PortlessSetupOwnership,
    setup_status: PortlessSetupStatus,
    runtime_status: PortlessRuntimeStatus,
) -> PortlessState {
    PortlessState {
        enabled,
        protocol: PortlessProtocol::Https,
        setup_ownership,
        setup_status,
        runtime_status,
    }
}

pub(super) fn assert_routes_file(paths: &crate::paths::GxserverPaths, expected: &[PortlessRoute]) {
    let text = fs::read_to_string(paths.portless_state_dir.join(PORTLESS_ROUTES_FILE))
        .expect("read routes file");
    let routes: Vec<PortlessRoute> = serde_json::from_str(&text).expect("parse routes file");
    assert_eq!(routes, expected);
}

pub(super) fn assert_no_portless_temp_artifacts(paths: &crate::paths::GxserverPaths) {
    let names = fs::read_dir(&paths.portless_state_dir)
        .expect("read Portless state dir")
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    assert!(
        names
            .iter()
            .all(|name| !name.starts_with(".routes.json.tmp.")),
        "temporary Portless route files should be cleaned up: {names:?}"
    );
    assert!(
        !names.iter().any(|name| name == PORTLESS_ROUTES_LOCK),
        "Portless routes lock should be released"
    );
}

pub(super) fn project_slug(identities: &PortlessDomainIdentities, project_id: &str) -> String {
    identities
        .projects
        .iter()
        .find(|project| project.project_id == project_id)
        .map(|project| project.slug.clone())
        .expect("project slug")
}

pub(super) fn worktree_slug(
    identities: &PortlessDomainIdentities,
    worktree_project_id: &str,
) -> String {
    identities
        .worktrees
        .iter()
        .find(|worktree| worktree.worktree_project_id == worktree_project_id)
        .map(|worktree| worktree.worktree_slug.clone())
        .expect("worktree slug")
}

pub(super) fn sorted_project_slug_pairs(
    identities: &PortlessDomainIdentities,
) -> Vec<(String, String)> {
    let mut pairs = identities
        .projects
        .iter()
        .map(|project| (project.project_id.clone(), project.slug.clone()))
        .collect::<Vec<_>>();
    pairs.sort();
    pairs
}

pub(super) fn sorted_worktree_slug_pairs(
    identities: &PortlessDomainIdentities,
) -> Vec<(String, String)> {
    let mut pairs = identities
        .worktrees
        .iter()
        .map(|worktree| {
            (
                worktree.worktree_project_id.clone(),
                worktree.worktree_slug.clone(),
            )
        })
        .collect::<Vec<_>>();
    pairs.sort();
    pairs
}
