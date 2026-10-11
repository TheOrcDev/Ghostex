use std::fs;

use rusqlite::params;

use super::*;
use crate::{
    paths::get_gxserver_paths,
    storage::{initialize_gxserver_storage, open_gxserver_database},
};

#[test]
fn route_sync_validation_rejects_pid_zero_invalid_hosts_and_invalid_ports() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    sync_portless_routes(&paths, &[route("valid.localhost", 3000, 41)]).expect("initial sync");
    let original = fs::read_to_string(paths.portless_state_dir.join(PORTLESS_ROUTES_FILE))
        .expect("read original routes");

    let invalid_routes = [
        route("pid-zero.localhost", 3000, 0),
        route("port-zero.localhost", 0, 42),
        route("", 3000, 42),
        route("https://raw-url.localhost", 3000, 42),
        route("raw-url.localhost/path", 3000, 42),
        route("localhost", 3000, 42),
        route("bad..label.localhost", 3000, 42),
        route("-bad.localhost", 3000, 42),
        route("bad-.localhost", 3000, 42),
        route("BadUpper.localhost", 3000, 42),
        route("wrong.test", 3000, 42),
    ];

    for invalid in invalid_routes {
        assert!(
            sync_portless_routes(&paths, &[invalid]).is_err(),
            "invalid route should be rejected"
        );
        assert_eq!(
            fs::read_to_string(paths.portless_state_dir.join(PORTLESS_ROUTES_FILE))
                .expect("read routes after rejected sync"),
            original
        );
    }
}

#[test]
fn desired_routes_single_project_server_uses_project_base_domain() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Psingle", "Single App");
    PortlessRepository::new(&db)
        .upsert_project_slug("Psingle", "ghostex")
        .expect("project slug");

    let routes = compute_desired_portless_routes(
        &db,
        &[owned_listener(
            "Psingle",
            "Gdev",
            "S90-Psingle-Gdev",
            None,
            8080,
            81,
        )],
    )
    .expect("desired routes");

    assert_eq!(routes, vec![route("ghostex.localhost", 8080, 81)]);
}

#[test]
fn desired_routes_choose_project_primary_by_port_preference_and_extra_domains() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Pmulti", "Multi App");
    PortlessRepository::new(&db)
        .upsert_project_slug("Pmulti", "ghostex")
        .expect("project slug");

    let routes = compute_desired_portless_routes(
        &db,
        &[
            owned_listener("Pmulti", "G8080", "S90-Pmulti-G8080", None, 8080, 80),
            owned_listener("Pmulti", "Glow", "S90-Pmulti-Glow", None, 4000, 40),
            owned_listener("Pmulti", "G5173", "S90-Pmulti-G5173", None, 5173, 51),
        ],
    )
    .expect("desired routes");

    assert_eq!(
        routes,
        vec![
            route("ghostex.localhost", 5173, 51),
            route("p4000.ghostex.localhost", 4000, 40),
            route("p8080.ghostex.localhost", 8080, 80),
        ]
    );
}

#[test]
fn desired_routes_primary_falls_back_to_lowest_port_without_preferred_ports() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Pfallback", "Fallback App");
    PortlessRepository::new(&db)
        .upsert_project_slug("Pfallback", "ghostex")
        .expect("project slug");

    let routes = compute_desired_portless_routes(
        &db,
        &[
            owned_listener("Pfallback", "G9000", "S90-Pfallback-G9000", None, 9000, 90),
            owned_listener("Pfallback", "G4242", "S90-Pfallback-G4242", None, 4242, 42),
        ],
    )
    .expect("desired routes");

    assert_eq!(
        routes,
        vec![
            route("ghostex.localhost", 4242, 42),
            route("p9000.ghostex.localhost", 9000, 90),
        ]
    );
}

#[test]
fn desired_routes_worktree_listener_uses_project_and_worktree_base_domain() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Pparent", "Parent App");
    insert_worktree_project(
        &db,
        "Pwtfix",
        "Pparent",
        "Worktree App",
        "Fix UI",
        "feature/fix-ui",
        "2026-06-22T18:42:00.000Z",
    );
    let repository = PortlessRepository::new(&db);
    repository
        .upsert_project_slug("Pparent", "ghostex")
        .expect("project slug");
    repository
        .upsert_worktree_slug("Pparent", "Pwtfix", "fix-ui")
        .expect("worktree slug");

    let routes = compute_desired_portless_routes(
        &db,
        &[owned_listener(
            "Pwtfix",
            "Gdev",
            "S90-Pwtfix-Gdev",
            Some("Pparent"),
            8080,
            88,
        )],
    )
    .expect("desired routes");

    assert_eq!(routes, vec![route("ghostex.fix-ui.localhost", 8080, 88)]);
}

#[test]
fn desired_routes_worktree_extra_routes_use_port_prefixed_base_domain() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Pparent", "Parent App");
    insert_worktree_project(
        &db,
        "Pwtfix",
        "Pparent",
        "Worktree App",
        "Fix UI",
        "feature/fix-ui",
        "2026-06-22T18:42:00.000Z",
    );
    let repository = PortlessRepository::new(&db);
    repository
        .upsert_project_slug("Pparent", "ghostex")
        .expect("project slug");
    repository
        .upsert_worktree_slug("Pparent", "Pwtfix", "fix-ui")
        .expect("worktree slug");

    let routes = compute_desired_portless_routes(
        &db,
        &[
            owned_listener(
                "Pwtfix",
                "G8787",
                "S90-Pwtfix-G8787",
                Some("Pparent"),
                8787,
                87,
            ),
            owned_listener(
                "Pwtfix",
                "G5173",
                "S90-Pwtfix-G5173",
                Some("Pparent"),
                5173,
                51,
            ),
            owned_listener(
                "Pwtfix",
                "G3000",
                "S90-Pwtfix-G3000",
                Some("Pparent"),
                3000,
                30,
            ),
        ],
    )
    .expect("desired routes");

    assert_eq!(
        routes,
        vec![
            route("ghostex.fix-ui.localhost", 3000, 30),
            route("p5173.ghostex.fix-ui.localhost", 5173, 51),
            route("p8787.ghostex.fix-ui.localhost", 8787, 87),
        ]
    );
}

#[test]
fn desired_routes_are_temporary_and_tied_to_live_listener_input() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Ptemp", "Temporary App");
    PortlessRepository::new(&db)
        .upsert_project_slug("Ptemp", "ghostex")
        .expect("project slug");
    let primary_listener = owned_listener("Ptemp", "G3000", "S90-Ptemp-G3000", None, 3000, 30);
    let extra_listener = owned_listener("Ptemp", "G5173", "S90-Ptemp-G5173", None, 5173, 51);

    let with_extra =
        compute_desired_portless_routes(&db, &[primary_listener.clone(), extra_listener.clone()])
            .expect("desired routes with extra");
    let without_extra = compute_desired_portless_routes(&db, &[primary_listener])
        .expect("desired routes after removal");

    assert_eq!(
        with_extra,
        vec![
            route("ghostex.localhost", 3000, 30),
            route("p5173.ghostex.localhost", 5173, 51),
        ]
    );
    assert_eq!(without_extra, vec![route("ghostex.localhost", 3000, 30)]);
}

#[test]
fn protocol_status_payload_serializes_only_metadata_and_local_native_action_requirements() {
    /*
    CDXC:Portless 2026-06-23-00:25:
    Phase 12 status payloads must tell React which Portless setup action is relevant while keeping all privileged actions unavailable in gxserver metadata. The native sidebar may enable them only for local Mac admin bridge execution.
    */
    let payload = portless_status_payload_from_record(
        Some(PortlessStateRecord {
            state: PortlessState {
                enabled: true,
                protocol: PortlessProtocol::Https,
                setup_ownership: PortlessSetupOwnership::Missing,
                setup_status: PortlessSetupStatus::Needed,
                runtime_status: PortlessRuntimeStatus::Inactive,
            },
            created_at: "2026-06-23T00:25:00.000Z".to_string(),
            updated_at: "2026-06-23T00:25:00.000Z".to_string(),
        }),
        PortlessPayloadSourceStatus::Current,
    );

    let value = serde_json::to_value(&payload).expect("serialize status payload");
    assert_eq!(value["enabled"], true);
    assert_eq!(value["protocol"], "https");
    assert_eq!(value["setupOwnership"], "missing");
    assert_eq!(value["setupStatus"], "needed");
    assert_eq!(value["runtimeStatus"], "inactive");
    assert_eq!(value["sourceStatus"], "current");
    assert_eq!(value["actions"]["install"]["recommended"], true);
    assert_eq!(value["actions"]["install"]["available"], false);
    assert_eq!(value["actions"]["install"]["localMacOnly"], true);
    assert_eq!(
        value["actions"]["install"]["unavailableReason"],
        "nativeAdminBridgeRequired"
    );
    assert_eq!(value["actions"]["reconfigure"]["recommended"], false);
    assert_eq!(value["actions"]["reconfigure"]["available"], false);

    let text = value.to_string();
    for disallowed in [
        "stdout",
        "stderr",
        "token",
        "cookie",
        "env",
        "commandText",
        "filePath",
        "http://",
        "https://",
        "/tmp/",
    ] {
        assert!(
            !text.contains(disallowed),
            "Portless status payload exposed disallowed field/value {disallowed}"
        );
    }
}

#[test]
fn protocol_route_previews_join_desired_routes_to_stable_ids_without_pids_or_urls() {
    /*
    CDXC:Portless 2026-06-23-00:25:
    Route previews are presentation metadata, not Portless file contents. Carry protocol, hostname, port, stable project/session ids, and primary/additional kind so UI can render links later without full URLs, pids, raw paths, command text, or process output.
    */
    let listeners = vec![
        owned_listener(
            "Ppreview",
            "Gprimary",
            "S90-Ppreview-Gprimary",
            None,
            3000,
            30,
        ),
        owned_listener(
            "Ppreview",
            "Gadditional",
            "S90-Ppreview-Gadditional",
            None,
            5173,
            51,
        ),
    ];
    let previews = portless_route_previews_for_desired_routes(
        PortlessProtocol::Https,
        &listeners,
        &[
            route("ghostex.localhost", 3000, 30),
            route("p5173.ghostex.localhost", 5173, 51),
        ],
    );

    let value = serde_json::to_value(&previews).expect("serialize route previews");
    assert_eq!(value[0]["hostname"], "ghostex.localhost");
    assert_eq!(value[0]["kind"], "primary");
    assert_eq!(value[0]["port"], 3000);
    assert_eq!(value[0]["projectId"], "Ppreview");
    assert_eq!(value[0]["protocol"], "https");
    assert_eq!(value[0]["sessionId"], "Gprimary");
    assert_eq!(value[1]["hostname"], "p5173.ghostex.localhost");
    assert_eq!(value[1]["kind"], "additional");
    assert_eq!(value[1]["port"], 5173);
    assert_eq!(value[1]["sessionId"], "Gadditional");

    let text = value.to_string();
    for disallowed in [
        "pid",
        "stdout",
        "stderr",
        "token",
        "cookie",
        "env",
        "commandText",
        "http://",
        "https://",
        "/tmp/",
    ] {
        assert!(
            !text.contains(disallowed),
            "Portless route preview exposed disallowed field/value {disallowed}"
        );
    }
}

#[test]
fn presentation_payload_includes_assigned_domains_without_live_listeners() {
    /*
    CDXC:Portless 2026-06-23-04:02:
    Assigned domains are persisted slug metadata, not a live listener view.
    The Settings UI needs these hostnames for stopped projects/worktrees
    while route previews remain limited to currently detected dev servers.
    */
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project_with_path(
        &db,
        "Passigned",
        "Assigned Project",
        Some("/tmp/assigned-project"),
        "2026-06-22T18:42:00.000Z",
    );
    insert_worktree_project(
        &db,
        "PassignedWt",
        "Passigned",
        "Assigned Worktree",
        "Feature UI",
        "feature/ui",
        "2026-06-22T18:43:00.000Z",
    );

    let payload = read_portless_presentation_payload(&db);

    assert_eq!(payload.assigned_domains.len(), 2);
    let project = payload
        .assigned_domains
        .iter()
        .find(|domain| domain.project_id == "Passigned")
        .expect("project assigned domain");
    assert_eq!(project.kind, PortlessAssignedDomainKind::Project);
    assert_eq!(project.parent_project_id, None);
    assert!(project.hostname.ends_with(".localhost"));

    let worktree = payload
        .assigned_domains
        .iter()
        .find(|domain| domain.project_id == "PassignedWt")
        .expect("worktree assigned domain");
    assert_eq!(worktree.kind, PortlessAssignedDomainKind::Worktree);
    assert_eq!(worktree.parent_project_id.as_deref(), Some("Passigned"));
    assert!(worktree.hostname.ends_with(".localhost"));
    assert!(payload.route_previews.is_empty());

    let text = serde_json::to_value(&payload.assigned_domains)
        .expect("serialize assigned domains")
        .to_string();
    for disallowed in [
        "stdout",
        "stderr",
        "token",
        "cookie",
        "env",
        "commandText",
        "filePath",
        "http://",
        "https://",
        "/tmp/",
    ] {
        assert!(
            !text.contains(disallowed),
            "Portless assigned domain exposed disallowed field/value {disallowed}"
        );
    }
}

#[test]
fn presentation_payload_marks_disabled_without_running_listener_detection() {
    /*
    CDXC:Portless 2026-06-23-00:25:
    Disabled Portless should render as explicit presentation metadata with no route previews and without probing live listeners, because disabled setup is a user-visible state rather than a listener-discovery failure.
    */
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    PortlessRepository::new(&db)
        .upsert_state(PortlessState {
            enabled: false,
            protocol: PortlessProtocol::Https,
            setup_ownership: PortlessSetupOwnership::Ghostex,
            setup_status: PortlessSetupStatus::Disabled,
            runtime_status: PortlessRuntimeStatus::Inactive,
        })
        .expect("disabled state");

    let payload = read_portless_presentation_payload(&db);

    assert_eq!(
        payload.route_preview_status,
        PortlessRoutePreviewStatus::Disabled
    );
    assert_eq!(payload.live_listener_count, 0);
    assert!(payload.route_previews.is_empty());
    assert_eq!(payload.status.setup_status, PortlessSetupStatus::Disabled);
}

#[test]
fn desired_routes_backfill_missing_slugs_with_stable_allocator() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project_with_path(
        &db,
        "Pparent",
        "Ghostex",
        Some("/tmp/ghostex"),
        "2026-06-22T18:41:00.000Z",
    );
    insert_worktree_project(
        &db,
        "Pwtfix",
        "Pparent",
        "Worktree App",
        "Fix UI",
        "feature/fix-ui",
        "2026-06-22T18:42:00.000Z",
    );
    let listener = owned_listener(
        "Pwtfix",
        "Gdev",
        "S90-Pwtfix-Gdev",
        Some("Pparent"),
        5173,
        73,
    );

    let first = compute_desired_portless_routes(&db, std::slice::from_ref(&listener))
        .expect("first desired routes");
    let repository = PortlessRepository::new(&db);
    assert_eq!(
        repository
            .read_project_slug("Pparent")
            .expect("read project slug")
            .map(|record| record.slug),
        Some("ghostex".to_string())
    );
    assert_eq!(
        repository
            .read_worktree_slug("Pparent", "Pwtfix")
            .expect("read worktree slug")
            .map(|record| record.slug),
        Some("fix-ui".to_string())
    );

    db.execute(
        "UPDATE projects SET name = ?2, path = ?3, updatedAt = ?4 WHERE projectId = ?1",
        params![
            "Pparent",
            "Renamed Parent",
            "/tmp/renamed-parent",
            "2026-06-22T18:45:00.000Z"
        ],
    )
    .expect("rename parent project");
    update_worktree_metadata(
        &db,
        "Pwtfix",
        "Pparent",
        "Renamed Worktree",
        "feature/renamed-worktree",
    );
    let second = compute_desired_portless_routes(&db, &[listener]).expect("second desired routes");

    assert_eq!(first, vec![route("ghostex.fix-ui.localhost", 5173, 73)]);
    assert_eq!(second, first);
    assert!(!paths.portless_state_dir.exists());
}
