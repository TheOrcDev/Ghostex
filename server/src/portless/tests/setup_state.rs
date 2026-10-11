use std::fs;

use super::*;
use crate::{
    paths::get_gxserver_paths,
    storage::{initialize_gxserver_storage, open_gxserver_database},
};

#[test]
fn setup_runtime_state_round_trip() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    let repository = PortlessRepository::new(&db);

    assert_eq!(repository.read_state().expect("empty state"), None);

    let created = repository
        .upsert_state(PortlessState {
            enabled: true,
            protocol: PortlessProtocol::Https,
            setup_ownership: PortlessSetupOwnership::Missing,
            setup_status: PortlessSetupStatus::Needed,
            runtime_status: PortlessRuntimeStatus::Inactive,
        })
        .expect("create state");
    assert_eq!(
        created.state,
        PortlessState {
            enabled: true,
            protocol: PortlessProtocol::Https,
            setup_ownership: PortlessSetupOwnership::Missing,
            setup_status: PortlessSetupStatus::Needed,
            runtime_status: PortlessRuntimeStatus::Inactive,
        }
    );

    let updated = repository
        .upsert_state(PortlessState {
            enabled: false,
            protocol: PortlessProtocol::Http,
            setup_ownership: PortlessSetupOwnership::Ghostex,
            setup_status: PortlessSetupStatus::Disabled,
            runtime_status: PortlessRuntimeStatus::Active,
        })
        .expect("update state");
    assert_eq!(
        updated.state,
        PortlessState {
            enabled: false,
            protocol: PortlessProtocol::Http,
            setup_ownership: PortlessSetupOwnership::Ghostex,
            setup_status: PortlessSetupStatus::Disabled,
            runtime_status: PortlessRuntimeStatus::Active,
        }
    );
    assert_eq!(
        repository
            .read_state()
            .expect("read state")
            .map(|record| record.state),
        Some(updated.state)
    );
}

#[test]
fn state_update_protocol_change_marks_installed_service_for_reconfigure() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    let repository = PortlessRepository::new(&db);
    repository
        .upsert_state(portless_state(
            true,
            PortlessSetupOwnership::Ghostex,
            PortlessSetupStatus::Active,
            PortlessRuntimeStatus::Active,
        ))
        .expect("active state");

    let updated = apply_portless_state_update(
        &paths,
        &db,
        PortlessStateUpdate::SetProtocol {
            protocol: PortlessProtocol::Http,
        },
    )
    .expect("protocol update");

    assert_eq!(updated.state.protocol, PortlessProtocol::Http);
    assert_eq!(
        updated.state.setup_ownership,
        PortlessSetupOwnership::Ghostex
    );
    assert_eq!(updated.state.setup_status, PortlessSetupStatus::Needed);
    assert_eq!(
        updated.state.runtime_status,
        PortlessRuntimeStatus::Inactive
    );
    assert!(updated.state.enabled);
}

#[test]
fn state_update_admin_failure_keeps_portless_enabled_and_failed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");

    let updated = apply_portless_state_update(
        &paths,
        &db,
        PortlessStateUpdate::RecordAdminResult {
            action: PortlessAdminResultAction::Install,
            ok: false,
            protocol: Some(PortlessProtocol::Https),
        },
    )
    .expect("failed admin result");

    assert!(updated.state.enabled);
    assert_eq!(updated.state.protocol, PortlessProtocol::Https);
    assert_eq!(
        updated.state.setup_ownership,
        PortlessSetupOwnership::Ghostex
    );
    assert_eq!(updated.state.setup_status, PortlessSetupStatus::Failed);
    assert_eq!(updated.state.runtime_status, PortlessRuntimeStatus::Failed);
    assert_eq!(
        recommended_portless_admin_action(&updated.state),
        Some(PortlessAdminActionKind::Retry)
    );
}

#[test]
fn state_update_retry_success_recovers_failed_setup() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    let repository = PortlessRepository::new(&db);
    repository
        .upsert_state(portless_state(
            true,
            PortlessSetupOwnership::Ghostex,
            PortlessSetupStatus::Failed,
            PortlessRuntimeStatus::Failed,
        ))
        .expect("failed state");

    let updated = apply_portless_state_update(
        &paths,
        &db,
        PortlessStateUpdate::RecordAdminResult {
            action: PortlessAdminResultAction::Retry,
            ok: true,
            protocol: Some(PortlessProtocol::Http),
        },
    )
    .expect("retry admin result");

    assert!(updated.state.enabled);
    assert_eq!(updated.state.protocol, PortlessProtocol::Http);
    assert_eq!(
        updated.state.setup_ownership,
        PortlessSetupOwnership::Ghostex
    );
    assert_eq!(updated.state.setup_status, PortlessSetupStatus::Active);
    assert_eq!(updated.state.runtime_status, PortlessRuntimeStatus::Active);
}

#[test]
fn state_update_disable_clears_routes_without_removing_service() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    let repository = PortlessRepository::new(&db);
    repository
        .upsert_state(portless_state(
            true,
            PortlessSetupOwnership::Ghostex,
            PortlessSetupStatus::Active,
            PortlessRuntimeStatus::Active,
        ))
        .expect("active state");
    sync_portless_routes(&paths, &[route("clear-on-disable.localhost", 3000, 42)])
        .expect("seed route");

    let updated = apply_portless_state_update(
        &paths,
        &db,
        PortlessStateUpdate::SetEnabled { enabled: false },
    )
    .expect("disable update");

    assert!(!updated.state.enabled);
    assert_eq!(
        updated.state.setup_ownership,
        PortlessSetupOwnership::Ghostex
    );
    assert_eq!(updated.state.setup_status, PortlessSetupStatus::Disabled);
    assert_routes_file(&paths, &[]);
}

#[test]
fn state_update_explicit_remove_service_is_separate_from_disable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    let repository = PortlessRepository::new(&db);
    repository
        .upsert_state(portless_state(
            true,
            PortlessSetupOwnership::Ghostex,
            PortlessSetupStatus::Active,
            PortlessRuntimeStatus::Active,
        ))
        .expect("active state");
    sync_portless_routes(&paths, &[route("clear-on-remove.localhost", 5173, 77)])
        .expect("seed route");

    let updated = apply_portless_state_update(
        &paths,
        &db,
        PortlessStateUpdate::RecordAdminResult {
            action: PortlessAdminResultAction::Remove,
            ok: true,
            protocol: None,
        },
    )
    .expect("remove admin result");

    assert!(updated.state.enabled);
    assert_eq!(
        updated.state.setup_ownership,
        PortlessSetupOwnership::Missing
    );
    assert_eq!(updated.state.setup_status, PortlessSetupStatus::Needed);
    assert_eq!(
        updated.state.runtime_status,
        PortlessRuntimeStatus::Inactive
    );
    assert_routes_file(&paths, &[]);
}

#[test]
fn persistence_apis_do_not_create_or_require_portless_state_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let portless_state_dir = paths.portless_state_dir.clone();
    assert!(!portless_state_dir.exists());

    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "P3main", "Display Name");
    insert_worktree_project(
        &db,
        "P3wt",
        "P3main",
        "Worktree Project",
        "Feature A",
        "feature/a",
        "2026-06-22T18:42:00.000Z",
    );
    let repository = PortlessRepository::new(&db);
    repository
        .upsert_project_slug("P3main", "metadata-only")
        .expect("project slug");
    repository
        .upsert_worktree_slug("P3main", "P3wt", "feature-a")
        .expect("worktree slug");
    repository
        .upsert_state(PortlessState {
            enabled: true,
            protocol: PortlessProtocol::Https,
            setup_ownership: PortlessSetupOwnership::Unknown,
            setup_status: PortlessSetupStatus::Unknown,
            runtime_status: PortlessRuntimeStatus::Unknown,
        })
        .expect("state");
    repository
        .backfill_domain_identities()
        .expect("backfill identities");
    repository
        .ensure_project_slug("P3main")
        .expect("ensure project slug");
    repository
        .ensure_worktree_slug("P3wt")
        .expect("ensure worktree slug");

    assert!(!portless_state_dir.exists());
    assert!(repository
        .read_project_slug("P3main")
        .expect("read project slug")
        .is_some());
    assert!(repository
        .read_worktree_slug("P3main", "P3wt")
        .expect("read worktree slug")
        .is_some());
    assert!(repository.read_state().expect("read state").is_some());
    assert!(!portless_state_dir.exists());
}

#[test]
fn path_computation_includes_ghostex_managed_portless_state_dir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));

    assert_eq!(
        paths.portless_state_dir,
        temp.path()
            .join(".ghostex")
            .join("state")
            .join("gxserver")
            .join("portless")
    );
    assert!(paths.portless_state_dir.starts_with(&paths.root_dir));
}

#[test]
fn ensure_portless_state_dir_creates_writable_directory_under_gxserver_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));

    ensure_portless_state_dir(&paths).expect("ensure Portless state dir");

    assert!(paths.portless_state_dir.starts_with(&paths.root_dir));
    assert!(fs::metadata(&paths.portless_state_dir)
        .expect("Portless state dir metadata")
        .is_dir());
    let probe = paths.portless_state_dir.join("probe");
    fs::write(&probe, b"ok").expect("write probe");
    assert_eq!(fs::read(&probe).expect("read probe"), b"ok");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&paths.portless_state_dir)
                .expect("Portless state dir metadata")
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
}
