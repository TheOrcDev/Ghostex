use std::fs;
use std::time::Duration;

use super::*;
use crate::{
    paths::get_gxserver_paths,
    storage::{initialize_gxserver_storage, open_gxserver_database},
};

#[test]
fn route_sync_uses_routes_lock_and_blocks_when_lock_is_active() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    ensure_portless_state_dir(&paths).expect("ensure Portless state dir");
    fs::create_dir(paths.portless_state_dir.join(PORTLESS_ROUTES_LOCK)).expect("lock dir");

    let result = sync_portless_routes_with_options(
        &paths,
        &[route("blocked.localhost", 5173, 42)],
        PortlessRouteSyncOptions {
            lock_timeout: Duration::from_millis(25),
            lock_retry_delay: Duration::from_millis(5),
            stale_lock_age: Duration::from_secs(3600),
        },
    );

    assert!(result.is_err());
    assert!(paths.portless_state_dir.join(PORTLESS_ROUTES_LOCK).exists());
    assert!(!paths.portless_state_dir.join(PORTLESS_ROUTES_FILE).exists());
    fs::remove_dir_all(paths.portless_state_dir.join(PORTLESS_ROUTES_LOCK))
        .expect("remove test lock");
}

#[test]
fn route_sync_removes_deterministically_stale_routes_lock() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    ensure_portless_state_dir(&paths).expect("ensure Portless state dir");
    fs::create_dir(paths.portless_state_dir.join(PORTLESS_ROUTES_LOCK)).expect("lock dir");

    sync_portless_routes_with_options(
        &paths,
        &[route("fresh.localhost", 3000, 77)],
        PortlessRouteSyncOptions {
            lock_timeout: Duration::from_millis(100),
            lock_retry_delay: Duration::from_millis(1),
            stale_lock_age: Duration::ZERO,
        },
    )
    .expect("sync after stale lock removal");

    assert!(!paths.portless_state_dir.join(PORTLESS_ROUTES_LOCK).exists());
    assert_routes_file(&paths, &[route("fresh.localhost", 3000, 77)]);
}

#[test]
fn route_sync_replaces_stale_routes_with_exact_desired_set() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    ensure_portless_state_dir(&paths).expect("ensure Portless state dir");
    fs::write(
        paths.portless_state_dir.join(PORTLESS_ROUTES_FILE),
        serde_json::to_string_pretty(&vec![
            route("old.localhost", 3000, 11),
            route("keep.localhost", 5173, 12),
        ])
        .expect("serialize old routes"),
    )
    .expect("write old routes");

    sync_portless_routes(
        &paths,
        &[
            route("keep.localhost", 5173, 12),
            route("new.keep.localhost", 8080, 13),
        ],
    )
    .expect("sync routes");

    assert_routes_file(
        &paths,
        &[
            route("keep.localhost", 5173, 12),
            route("new.keep.localhost", 8080, 13),
        ],
    );
    assert_no_portless_temp_artifacts(&paths);
}

#[test]
fn route_sync_writes_valid_json_atomically_and_cleans_temp_artifacts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    ensure_portless_state_dir(&paths).expect("ensure Portless state dir");
    fs::write(
        paths.portless_state_dir.join(PORTLESS_ROUTES_FILE),
        r#"[{"hostname":"previous.localhost","port":3000,"pid":21}]"#,
    )
    .expect("write previous routes");

    sync_portless_routes(&paths, &[route("atomic.localhost", 5174, 22)]).expect("sync routes");

    let text = fs::read_to_string(paths.portless_state_dir.join(PORTLESS_ROUTES_FILE))
        .expect("read routes file");
    let parsed: Vec<PortlessRoute> = serde_json::from_str(&text).expect("valid routes json");
    assert_eq!(parsed, vec![route("atomic.localhost", 5174, 22)]);
    assert!(!text.contains("previous.localhost"));
    assert_no_portless_temp_artifacts(&paths);
}

#[test]
fn route_sync_empty_desired_routes_leaves_empty_valid_routes_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));

    sync_portless_routes(&paths, &[route("clear-me.localhost", 3000, 31)]).expect("initial sync");
    sync_portless_routes(&paths, &[]).expect("empty sync");

    let routes_path = paths.portless_state_dir.join(PORTLESS_ROUTES_FILE);
    assert!(routes_path.exists());
    assert_eq!(
        serde_json::from_str::<Vec<PortlessRoute>>(
            &fs::read_to_string(routes_path).expect("read empty routes")
        )
        .expect("parse empty routes"),
        Vec::<PortlessRoute>::new()
    );
    assert_no_portless_temp_artifacts(&paths);
}

#[test]
fn background_sync_policy_clears_routes_when_disabled_regardless_of_setup_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    sync_portless_routes(&paths, &[route("stale.localhost", 3000, 61)])
        .expect("initial stale route");
    let disabled_active_state = portless_state(
        false,
        PortlessSetupOwnership::Ghostex,
        PortlessSetupStatus::Active,
        PortlessRuntimeStatus::Active,
    );

    let outcome = apply_portless_background_sync_policy(
        &paths,
        Some(&disabled_active_state),
        &[route("desired.localhost", 5173, 62)],
        1,
    )
    .expect("apply disabled policy");

    assert_eq!(
        outcome.action,
        PortlessBackgroundRouteAction::ClearMirroredRoutes
    );
    assert_eq!(outcome.status, PortlessBackgroundStatus::Disabled);
    assert_eq!(outcome.desired_route_count, 1);
    assert_routes_file(&paths, &[]);
}

#[test]
fn background_sync_policy_skips_setup_missing_without_writing_desired_routes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    let setup_missing_state = portless_state(
        true,
        PortlessSetupOwnership::Missing,
        PortlessSetupStatus::Needed,
        PortlessRuntimeStatus::Inactive,
    );

    let outcome = apply_portless_background_sync_policy(
        &paths,
        Some(&setup_missing_state),
        &[route("missing.localhost", 5173, 71)],
        1,
    )
    .expect("apply missing policy");

    assert_eq!(
        outcome.action,
        PortlessBackgroundRouteAction::SkipRouteFileWrite
    );
    assert_eq!(outcome.status, PortlessBackgroundStatus::SetupNeeded);
    assert_eq!(outcome.desired_route_count, 1);
    assert!(!paths.portless_state_dir.join(PORTLESS_ROUTES_FILE).exists());
}

#[test]
fn background_sync_policy_skips_failed_setup_without_writing_desired_routes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    let failed_state = portless_state(
        true,
        PortlessSetupOwnership::Ghostex,
        PortlessSetupStatus::Failed,
        PortlessRuntimeStatus::Failed,
    );

    let outcome = apply_portless_background_sync_policy(
        &paths,
        Some(&failed_state),
        &[route("failed.localhost", 8080, 81)],
        1,
    )
    .expect("apply failed policy");

    assert_eq!(
        outcome.action,
        PortlessBackgroundRouteAction::SkipRouteFileWrite
    );
    assert_eq!(outcome.status, PortlessBackgroundStatus::SetupFailed);
    assert_eq!(outcome.desired_route_count, 1);
    assert!(!paths.portless_state_dir.join(PORTLESS_ROUTES_FILE).exists());
}

#[test]
fn background_sync_policy_skips_non_ghostex_setup_without_writing_desired_routes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    let standalone_state = portless_state(
        true,
        PortlessSetupOwnership::Standalone,
        PortlessSetupStatus::Needed,
        PortlessRuntimeStatus::Inactive,
    );

    let outcome = apply_portless_background_sync_policy(
        &paths,
        Some(&standalone_state),
        &[route("standalone.localhost", 3000, 91)],
        1,
    )
    .expect("apply standalone policy");

    assert_eq!(
        outcome.action,
        PortlessBackgroundRouteAction::SkipRouteFileWrite
    );
    assert_eq!(outcome.status, PortlessBackgroundStatus::SetupNeeded);
    assert!(!paths.portless_state_dir.join(PORTLESS_ROUTES_FILE).exists());
}

#[test]
fn background_sync_policy_mirrors_active_ghostex_routes_and_removes_stale_empty_set() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    sync_portless_routes(&paths, &[route("old.localhost", 3000, 101)])
        .expect("initial stale route");
    let active_state = portless_state(
        true,
        PortlessSetupOwnership::Ghostex,
        PortlessSetupStatus::Active,
        PortlessRuntimeStatus::Active,
    );

    let mirrored = apply_portless_background_sync_policy(
        &paths,
        Some(&active_state),
        &[
            route("current.localhost", 5173, 102),
            route("p8080.current.localhost", 8080, 103),
        ],
        2,
    )
    .expect("mirror active routes");

    assert_eq!(
        mirrored.action,
        PortlessBackgroundRouteAction::MirrorDesiredRoutes
    );
    assert_eq!(mirrored.status, PortlessBackgroundStatus::SetupActive);
    assert_routes_file(
        &paths,
        &[
            route("current.localhost", 5173, 102),
            route("p8080.current.localhost", 8080, 103),
        ],
    );

    let emptied = apply_portless_background_sync_policy(&paths, Some(&active_state), &[], 0)
        .expect("mirror empty live route set");

    assert_eq!(
        emptied.action,
        PortlessBackgroundRouteAction::MirrorDesiredRoutes
    );
    assert_eq!(emptied.desired_route_count, 0);
    assert_routes_file(&paths, &[]);
}

#[test]
fn background_sync_once_clears_disabled_state_without_live_listener_snapshot() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    PortlessRepository::new(&db)
        .upsert_state(portless_state(
            false,
            PortlessSetupOwnership::Missing,
            PortlessSetupStatus::Needed,
            PortlessRuntimeStatus::Inactive,
        ))
        .expect("disabled state");
    sync_portless_routes(&paths, &[route("disabled-stale.localhost", 3000, 111)])
        .expect("initial stale route");

    let outcome = run_portless_background_sync_once(&paths).expect("one-shot sync");

    assert_eq!(
        outcome.action,
        PortlessBackgroundRouteAction::ClearMirroredRoutes
    );
    assert_eq!(outcome.status, PortlessBackgroundStatus::Disabled);
    assert_eq!(outcome.live_listener_count, 0);
    assert_eq!(outcome.desired_route_count, 0);
    assert_routes_file(&paths, &[]);
}
