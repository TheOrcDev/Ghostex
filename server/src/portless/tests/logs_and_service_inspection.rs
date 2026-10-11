use std::fs;
use std::path::Path;

use super::*;
use crate::paths::get_gxserver_paths;

#[test]
fn portless_routine_operational_logs_are_debug_gated_while_warnings_persist() {
    let disabled_temp = tempfile::tempdir().expect("disabled tempdir");
    let disabled_paths = get_gxserver_paths(Some(disabled_temp.path().to_path_buf()));
    let disabled_logger = crate::logging::GxserverLogger::new(disabled_paths.clone());
    let outcome = PortlessBackgroundSyncOutcome {
        action: PortlessBackgroundRouteAction::MirrorDesiredRoutes,
        desired_route_count: 2,
        live_listener_count: 1,
        status: PortlessBackgroundStatus::SetupActive,
    };

    log_portless_background_sync_outcome(&disabled_logger, &outcome, 11);
    assert!(!disabled_paths.log_file.exists());

    log_portless_background_sync_failure(
        &disabled_logger,
        PortlessLogErrorCode::BackgroundSyncFailed,
        12,
    );
    let warning_text = fs::read_to_string(&disabled_paths.log_file).expect("read warning log");
    assert!(warning_text.contains("portless.backgroundSyncFailed"));
    assert!(warning_text.contains("backgroundSyncFailed"));
    assert!(!warning_text.contains("portless.backgroundSync\""));

    let enabled_temp = tempfile::tempdir().expect("enabled tempdir");
    let enabled_paths = get_gxserver_paths(Some(enabled_temp.path().to_path_buf()));
    enable_debugging_mode_for_test(&enabled_paths);
    let enabled_logger = crate::logging::GxserverLogger::new(enabled_paths.clone());
    log_portless_background_sync_outcome(&enabled_logger, &outcome, 13);

    let debug_text = fs::read_to_string(&enabled_paths.log_file).expect("read debug log");
    assert!(debug_text.contains("portless.backgroundSync"));
    assert!(debug_text.contains("\"routeCount\":2"));
    assert!(debug_text.contains("\"liveListenerCount\":1"));
    assert_portless_log_text_has_no_forbidden_raw_values(&debug_text);
}

#[test]
fn portless_state_update_logs_do_not_persist_forbidden_raw_values() {
    /*
    CDXC:Portless 2026-06-23-04:45:
    Phase 17 tests must prove Portless persisted diagnostics do not carry raw project/worktree names, paths, full URLs, hostnames, command text, env values, tokens, secrets, stdout, or stderr. The log helper accepts only enum/count/boolean/protocol state, so the test scans both success and warning entries for those forbidden values and field names.
    */
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    enable_debugging_mode_for_test(&paths);
    let logger = crate::logging::GxserverLogger::new(paths.clone());
    let update = PortlessStateUpdate::RecordAdminResult {
        action: PortlessAdminResultAction::Reconfigure,
        ok: false,
        protocol: Some(PortlessProtocol::Http),
    };
    let record = PortlessStateRecord {
        state: portless_state(
            true,
            PortlessSetupOwnership::Ghostex,
            PortlessSetupStatus::Failed,
            PortlessRuntimeStatus::Failed,
        ),
        created_at: "2026-06-23T00:45:00.000Z".to_string(),
        updated_at: "2026-06-23T00:45:01.000Z".to_string(),
    };

    log_portless_state_update_success(&logger, &update, &record, 21);
    log_portless_state_update_failure(
        &logger,
        &update,
        PortlessLogErrorCode::StateUpdateFailed,
        22,
    );

    let text = fs::read_to_string(&paths.log_file).expect("read Portless log");
    assert!(text.contains("portless.stateUpdate"));
    assert!(text.contains("portless.stateUpdateFailed"));
    assert!(text.contains("\"protocol\":\"http\""));
    assert!(text.contains("\"setupStatus\":\"failed\""));
    assert!(text.contains("\"errorCode\":\"stateUpdateFailed\""));
    assert_portless_log_text_has_no_forbidden_raw_values(&text);
    for forbidden_field in [
        "hostname",
        "path",
        "url",
        "command",
        "env",
        "token",
        "secret",
        "stdout",
        "stderr",
        "projectName",
        "worktreeName",
    ] {
        assert!(
            !text.contains(forbidden_field),
            "Portless log included forbidden field {forbidden_field}: {text}"
        );
    }
}

#[test]
fn service_inspection_classifies_missing_as_setup_needed_install_state() {
    let expectation =
        service_expectation(Path::new("/Users/ghostex-user"), PortlessProtocol::Https);

    let inspection = inspect_portless_service_from_plist_text(
        None,
        &expectation,
        PortlessServiceReachability::default(),
    )
    .expect("inspect missing service");
    let state = portless_state_for_service_inspection(None, expectation.protocol, &inspection);

    assert_eq!(
        inspection.classification,
        PortlessServiceClassification::Missing
    );
    assert!(state.enabled);
    assert_eq!(state.setup_ownership, PortlessSetupOwnership::Missing);
    assert_eq!(state.setup_status, PortlessSetupStatus::Needed);
    assert_eq!(state.runtime_status, PortlessRuntimeStatus::Inactive);
}

#[test]
fn service_inspection_accepts_escaped_ghostex_plist_as_active() {
    let home = Path::new("/Users/ghostex-user");
    let expectation = service_expectation(home, PortlessProtocol::Https);
    let plist = service_plist(
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/code-server/lib/node",
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/portless/dist/cli.js",
        "~/.ghostex/gxserver/portless",
        443,
        true,
        false,
        false,
        None,
        &["--foreground", "--port", "443", "--https", "--skip-trust"],
    );

    let inspection = inspect_portless_service_from_plist_text(
        Some(&plist),
        &expectation,
        PortlessServiceReachability {
            manager_running: Some(true),
            proxy_reachable: Some(true),
        },
    )
    .expect("inspect Ghostex service");
    let state = portless_state_for_service_inspection(None, expectation.protocol, &inspection);

    assert_eq!(
        inspection,
        PortlessServiceInspection {
            classification: PortlessServiceClassification::GhostexActive,
            mismatch_count: 0,
        }
    );
    assert_eq!(state.setup_ownership, PortlessSetupOwnership::Ghostex);
    assert_eq!(state.setup_status, PortlessSetupStatus::Active);
    assert_eq!(state.runtime_status, PortlessRuntimeStatus::Active);
}

#[test]
fn service_inspection_accepts_http_config_when_protocol_setting_is_http() {
    let home = Path::new("/Users/ghostex-user");
    let expectation = service_expectation(home, PortlessProtocol::Http);
    let plist = service_plist(
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/code-server/lib/node",
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/portless/dist/cli.js",
        "/Users/ghostex-user/.ghostex/gxserver/portless",
        80,
        false,
        false,
        false,
        None,
        &["--foreground", "--port", "80", "--no-tls", "--skip-trust"],
    );

    let inspection = inspect_portless_service_from_plist_text(
        Some(&plist),
        &expectation,
        PortlessServiceReachability {
            manager_running: Some(true),
            proxy_reachable: Some(true),
        },
    )
    .expect("inspect HTTP Ghostex service");
    let state = portless_state_for_service_inspection(None, expectation.protocol, &inspection);

    assert_eq!(
        inspection.classification,
        PortlessServiceClassification::GhostexActive
    );
    assert_eq!(inspection.mismatch_count, 0);
    assert_eq!(state.protocol, PortlessProtocol::Http);
    assert_eq!(state.setup_status, PortlessSetupStatus::Active);
    assert_eq!(state.runtime_status, PortlessRuntimeStatus::Active);
}

#[test]
fn service_inspection_classifies_standalone_service_separately_from_reconfigure() {
    let expectation =
        service_expectation(Path::new("/Users/ghostex-user"), PortlessProtocol::Https);
    let plist = service_plist(
        "/usr/local/bin/node",
        "/usr/local/lib/node_modules/portless/dist/cli.js",
        "/Users/ghostex-user/.portless",
        443,
        true,
        false,
        false,
        None,
        &["--foreground", "--port", "443", "--https", "--skip-trust"],
    );

    let inspection = inspect_portless_service_from_plist_text(
        Some(&plist),
        &expectation,
        PortlessServiceReachability {
            manager_running: Some(true),
            proxy_reachable: Some(true),
        },
    )
    .expect("inspect standalone service");
    let state = portless_state_for_service_inspection(None, expectation.protocol, &inspection);

    assert_eq!(
        inspection.classification,
        PortlessServiceClassification::Standalone
    );
    assert_eq!(state.setup_ownership, PortlessSetupOwnership::Standalone);
    assert_eq!(state.setup_status, PortlessSetupStatus::Needed);
    assert_eq!(state.runtime_status, PortlessRuntimeStatus::Inactive);
}

#[test]
fn service_inspection_classifies_ghostex_config_mismatch_as_reconfigure_needed() {
    let home = Path::new("/Users/ghostex-user");
    let expectation = service_expectation(home, PortlessProtocol::Https);
    let plist = service_plist_with_lan_ip(
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/code-server/lib/node",
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/portless/dist/cli.js",
        "/Users/ghostex-user/.ghostex/gxserver/portless",
        80,
        false,
        true,
        true,
        Some("local"),
        Some("192.168.1.42"),
        &[
            "--foreground",
            "--port",
            "80",
            "--no-tls",
            "--lan",
            "--wildcard",
        ],
    );

    let inspection = inspect_portless_service_from_plist_text(
        Some(&plist),
        &expectation,
        PortlessServiceReachability {
            manager_running: Some(true),
            proxy_reachable: Some(true),
        },
    )
    .expect("inspect mismatch service");
    let state = portless_state_for_service_inspection(None, expectation.protocol, &inspection);

    assert_eq!(
        inspection.classification,
        PortlessServiceClassification::GhostexConfigMismatch
    );
    assert!(inspection.mismatch_count >= 6);
    assert_eq!(state.setup_ownership, PortlessSetupOwnership::Ghostex);
    assert_eq!(state.setup_status, PortlessSetupStatus::Needed);
    assert_eq!(state.runtime_status, PortlessRuntimeStatus::Inactive);
}

#[test]
fn service_inspection_requires_sync_hosts_disabled_for_ghostex_service() {
    let home = Path::new("/Users/ghostex-user");
    let expectation = service_expectation(home, PortlessProtocol::Https);
    let plist = service_plist(
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/code-server/lib/node",
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/portless/dist/cli.js",
        "/Users/ghostex-user/.ghostex/gxserver/portless",
        443,
        true,
        false,
        false,
        None,
        &["--foreground", "--port", "443", "--https", "--skip-trust"],
    )
    .replace(
        "    <key>PORTLESS_SYNC_HOSTS</key>\n    <string>0</string>\n",
        "",
    );

    let inspection = inspect_portless_service_from_plist_text(
        Some(&plist),
        &expectation,
        PortlessServiceReachability {
            manager_running: Some(true),
            proxy_reachable: Some(true),
        },
    )
    .expect("inspect sync-hosts mismatch service");
    let state = portless_state_for_service_inspection(None, expectation.protocol, &inspection);

    assert_eq!(
        inspection.classification,
        PortlessServiceClassification::GhostexConfigMismatch
    );
    assert_eq!(inspection.mismatch_count, 1);
    assert_eq!(state.setup_ownership, PortlessSetupOwnership::Ghostex);
    assert_eq!(state.setup_status, PortlessSetupStatus::Needed);
    assert_eq!(state.runtime_status, PortlessRuntimeStatus::Inactive);
}

#[test]
fn service_inspection_rejects_persistent_launchd_output_paths() {
    let home = Path::new("/Users/ghostex-user");
    let expectation = service_expectation(home, PortlessProtocol::Https);
    let plist = service_plist(
            "/Applications/Ghostex & Dev.app/Contents/Resources/Web/code-server/lib/node",
            "/Applications/Ghostex & Dev.app/Contents/Resources/Web/portless/dist/cli.js",
            "/Users/ghostex-user/.ghostex/gxserver/portless",
            443,
            true,
            false,
            false,
            None,
            &["--foreground", "--port", "443", "--https", "--skip-trust"],
        )
        .replace(
            "  <key>StandardOutPath</key>\n  <string>/dev/null</string>",
            "  <key>StandardOutPath</key>\n  <string>/Users/ghostex-user/.ghostex/gxserver/portless/service.log</string>",
        )
        .replace(
            "  <key>StandardErrorPath</key>\n  <string>/dev/null</string>",
            "  <key>StandardErrorPath</key>\n  <string>/Users/ghostex-user/.ghostex/gxserver/portless/service.log</string>",
        );

    let inspection = inspect_portless_service_from_plist_text(
        Some(&plist),
        &expectation,
        PortlessServiceReachability {
            manager_running: Some(true),
            proxy_reachable: Some(true),
        },
    )
    .expect("inspect persistent output mismatch service");
    let state = portless_state_for_service_inspection(None, expectation.protocol, &inspection);

    assert_eq!(
        inspection.classification,
        PortlessServiceClassification::GhostexConfigMismatch
    );
    assert_eq!(inspection.mismatch_count, 2);
    assert_eq!(state.setup_ownership, PortlessSetupOwnership::Ghostex);
    assert_eq!(state.setup_status, PortlessSetupStatus::Needed);
    assert_eq!(state.runtime_status, PortlessRuntimeStatus::Inactive);
}

#[test]
fn service_inspection_treats_moved_ghostex_binary_as_reconfigure_not_standalone() {
    let home = Path::new("/Users/ghostex-user");
    let expectation = service_expectation(home, PortlessProtocol::Https);
    let plist = service_plist(
        "/Applications/Old Ghostex.app/Contents/Resources/Web/code-server/lib/node",
        "/Applications/Old Ghostex.app/Contents/Resources/Web/portless/dist/cli.js",
        "/Users/ghostex-user/.ghostex/gxserver/portless",
        443,
        true,
        false,
        false,
        None,
        &["--foreground", "--port", "443", "--https", "--skip-trust"],
    );

    let inspection = inspect_portless_service_from_plist_text(
        Some(&plist),
        &expectation,
        PortlessServiceReachability {
            manager_running: Some(true),
            proxy_reachable: Some(true),
        },
    )
    .expect("inspect moved Ghostex service");

    assert_eq!(
        inspection.classification,
        PortlessServiceClassification::GhostexConfigMismatch
    );
    assert!(inspection.mismatch_count >= 2);
}

#[test]
fn service_inspection_classifies_unreachable_ghostex_service_as_failed_retry_state() {
    let home = Path::new("/Users/ghostex-user");
    let expectation = service_expectation(home, PortlessProtocol::Https);
    let plist = service_plist(
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/code-server/lib/node",
        "/Applications/Ghostex & Dev.app/Contents/Resources/Web/portless/dist/cli.js",
        "/Users/ghostex-user/.ghostex/gxserver/portless",
        443,
        true,
        false,
        false,
        None,
        &["--foreground", "--port", "443", "--https", "--skip-trust"],
    );

    let inspection = inspect_portless_service_from_plist_text(
        Some(&plist),
        &expectation,
        PortlessServiceReachability {
            manager_running: Some(true),
            proxy_reachable: Some(false),
        },
    )
    .expect("inspect failed service");
    let state = portless_state_for_service_inspection(None, expectation.protocol, &inspection);

    assert_eq!(
        inspection.classification,
        PortlessServiceClassification::GhostexFailed
    );
    assert_eq!(state.setup_ownership, PortlessSetupOwnership::Ghostex);
    assert_eq!(state.setup_status, PortlessSetupStatus::Failed);
    assert_eq!(state.runtime_status, PortlessRuntimeStatus::Failed);
}

#[test]
fn service_detection_preserves_existing_disabled_state_without_reenabling() {
    let expectation =
        service_expectation(Path::new("/Users/ghostex-user"), PortlessProtocol::Https);
    let existing = portless_state(
        false,
        PortlessSetupOwnership::Missing,
        PortlessSetupStatus::Disabled,
        PortlessRuntimeStatus::Inactive,
    );
    let inspection = PortlessServiceInspection {
        classification: PortlessServiceClassification::GhostexActive,
        mismatch_count: 0,
    };

    let state =
        portless_state_for_service_inspection(Some(&existing), expectation.protocol, &inspection);

    assert!(!state.enabled);
    assert_eq!(state.setup_ownership, PortlessSetupOwnership::Ghostex);
    assert_eq!(state.setup_status, PortlessSetupStatus::Disabled);
    assert_eq!(state.runtime_status, PortlessRuntimeStatus::Active);
}
