use super::*;
use std::fs;

#[test]
fn title_signaled_process_identity_sync_only_targets_incomplete_live_zmx_identity() {
    assert!(terminal_title_indicates_agent_identity(
        "01a00854-13cb-7500-bde7-3d8d2b03abdd"
    ));
    assert!(terminal_title_indicates_agent_identity("Codex"));
    assert!(!terminal_title_indicates_agent_identity(
        "⠦ Fix GPUI Chat Mode Switching"
    ));

    let candidate = json!({
        "kind": "terminal",
        "lifecycleState": "running",
        "providerState": {
            "lifecycleState": "exists",
            "provider": "zmx",
        },
        "sessionId": "G9mmz",
        "surface": "terminal",
        "zmxName": "S9-P9-G9mmz",
    });
    assert!(should_probe_title_signaled_zmx_process_identity(&candidate));

    let mut promoted_agent = candidate.clone();
    promoted_agent["kind"] = json!("agent");
    promoted_agent["agentId"] = json!("codex");
    assert!(should_probe_title_signaled_zmx_process_identity(
        &promoted_agent
    ));

    promoted_agent["runtimeSettings"] = json!({
        "agentSessionId": "01a00854-13cb-7500-bde7-3d8d2b03abdd",
    });
    assert!(!should_probe_title_signaled_zmx_process_identity(
        &promoted_agent
    ));

    let mut stopped_terminal = candidate.clone();
    stopped_terminal["lifecycleState"] = json!("stopped");
    assert!(!should_probe_title_signaled_zmx_process_identity(
        &stopped_terminal
    ));

    let mut command_surface = candidate.clone();
    command_surface["surface"] = json!("commands");
    assert!(!should_probe_title_signaled_zmx_process_identity(
        &command_surface
    ));

    let mut non_zmx_terminal = candidate;
    non_zmx_terminal["providerState"]["provider"] = json!("none");
    assert!(!should_probe_title_signaled_zmx_process_identity(
        &non_zmx_terminal
    ));
}

#[test]
fn missing_zmx_agent_without_restore_plan_is_not_kept_running() {
    let project = json!({
        "path": "/tmp/ghostex-project",
        "projectId": "P9k9k",
    });
    let agent_settings = Map::new();
    let unrestorable_agent = json!({
        "agentId": "codex",
        "kind": "agent",
        "lifecycleState": "running",
        "projectId": "P9k9k",
        "providerState": {
            "lifecycleState": "missing",
            "provider": "zmx",
        },
        "runtimeSettings": {
            "sessionPersistenceProvider": "zmx",
            "titleSource": "terminal-auto",
        },
        "sessionId": "G9mmz",
        "title": "Codex Session",
    });
    assert!(should_sleep_unrestorable_missing_zmx_agent(
        &project,
        &unrestorable_agent,
        &agent_settings,
    ));

    let mut restorable_agent = unrestorable_agent.clone();
    restorable_agent["runtimeSettings"]["agentSessionId"] =
        json!("019fca32-5dad-73d3-a0eb-86b6a7486fdd");
    assert!(!should_sleep_unrestorable_missing_zmx_agent(
        &project,
        &restorable_agent,
        &agent_settings,
    ));

    let mut plain_terminal = unrestorable_agent;
    plain_terminal["kind"] = json!("terminal");
    assert!(!should_sleep_unrestorable_missing_zmx_agent(
        &project,
        &plain_terminal,
        &agent_settings,
    ));
}

#[test]
fn session_state_sidecar_parser_matches_legacy_env_fields() {
    let raw = [
        "agent=codex",
        "agentSessionId=019eebdb-ba5a-7282-ac09-b926a9c09863",
        "agentSessionPath=/Users/example/.codex/sessions/2026/06/21/thread.jsonl",
        "firstUserMessageBase64=UGxlYXNlIGZpeCB0aGUgc2lkZWJhcg==",
        "lastActivityAt=2026-06-21T20:25:05.171Z",
        "status=working",
        "statusUpdatedAt=2026-06-21T20:25:06.000Z",
        "title=  GPUI Sidebar Resize Parity  ",
    ]
    .join("\n");
    let sidecar = parse_session_state_sidecar(&raw).expect("sidecar");

    assert_eq!(sidecar.agent_name.as_deref(), Some("codex"));
    assert_eq!(
        sidecar.agent_session_id.as_deref(),
        Some("019eebdb-ba5a-7282-ac09-b926a9c09863")
    );
    assert_eq!(
        sidecar.first_user_message.as_deref(),
        Some("Please fix the sidebar")
    );
    assert_eq!(sidecar.status.as_deref(), Some("working"));
    assert_eq!(
        sidecar.status_updated_at.as_deref(),
        Some("2026-06-21T20:25:06.000Z")
    );
    assert_eq!(sidecar.title.as_deref(), Some("GPUI Sidebar Resize Parity"));
    assert!(has_session_state_sidecar_payload(&sidecar));
    assert_eq!(
        sanitize_session_state_sidecar_path_part("P3lv0/../../G01q0"),
        "P3lv0-..-..-G01q0"
    );
}

#[test]
fn session_state_sidecar_reader_uses_typescript_one_mib_cap() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    let sidecar_path = build_session_state_sidecar_path(&paths, "P3lv0", "G01q0");
    fs::create_dir_all(sidecar_path.parent().expect("sidecar parent")).expect("sidecar dir");

    fs::write(
        &sidecar_path,
        format!("agent=codex\npadding={}", "x".repeat(70 * 1024)),
    )
    .expect("write sidecar under cap");
    let sidecar = read_session_state_sidecar(&paths, "P3lv0", "G01q0").expect("sidecar");
    assert_eq!(sidecar.agent_name.as_deref(), Some("codex"));

    fs::write(
        &sidecar_path,
        format!(
            "agent=codex\npadding={}",
            "x".repeat(GXSERVER_SESSION_STATE_SIDECAR_MAX_BYTES as usize + 1)
        ),
    )
    .expect("write sidecar over cap");
    assert!(read_session_state_sidecar(&paths, "P3lv0", "G01q0").is_none());
}

#[test]
fn zmx_title_observer_parses_lines_and_filters_observable_sessions() {
    assert_eq!(
        parse_zmx_title_line(r#"{"title":"  Codex Session  "}"#).as_deref(),
        Some("Codex Session")
    );
    assert!(parse_zmx_title_line(r#"{"title":"   "}"#).is_none());
    assert!(parse_zmx_title_line("not-json").is_none());

    let observable = json!({
        "kind": "terminal",
        "lifecycleState": "running",
        "providerState": { "lifecycleState": "exists", "provider": "zmx" },
        "projectId": "P1",
        "runtimeSettings": {},
        "sessionId": "G1",
        "zmxName": "S1-P1-G1"
    });
    assert!(is_zmx_title_observable_session(&observable));

    let missing_provider = json!({
        "kind": "terminal",
        "lifecycleState": "running",
        "providerState": { "lifecycleState": "missing", "provider": "zmx" },
        "projectId": "P1",
        "sessionId": "G1",
        "zmxName": "S1-P1-G1"
    });
    assert!(!is_zmx_title_observable_session(&missing_provider));
}

#[test]
fn live_zmx_process_identity_sync_accepts_provider_state_only_sessions() {
    let provider_state_only = json!({
        "kind": "terminal",
        "lifecycleState": "running",
        "providerState": { "lifecycleState": "exists", "provider": "zmx" },
        "runtimeSettings": {},
        "surface": "workspace",
        "zmxName": "S1-P1-G1"
    });
    assert!(should_sync_live_zmx_process_identity(&provider_state_only));

    let runtime_provider = json!({
        "kind": "terminal",
        "lifecycleState": "running",
        "providerState": { "lifecycleState": "exists" },
        "runtimeSettings": { "sessionPersistenceProvider": "zmx" },
        "surface": "workspace",
        "zmxName": "S1-P1-G1"
    });
    assert!(should_sync_live_zmx_process_identity(&runtime_provider));

    let command_surface = json!({
        "kind": "terminal",
        "lifecycleState": "running",
        "providerState": { "lifecycleState": "exists", "provider": "zmx" },
        "runtimeSettings": {},
        "surface": "commands",
        "zmxName": "S1-P1-G1"
    });
    assert!(!should_sync_live_zmx_process_identity(&command_surface));
}
