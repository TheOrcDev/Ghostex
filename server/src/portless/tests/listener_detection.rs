use super::*;
use crate::{
    paths::get_gxserver_paths,
    storage::{initialize_gxserver_storage, open_gxserver_database},
};

#[test]
fn owned_listener_detection_maps_manual_dev_server_to_running_session() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Pmanual", "Manual App");
    let zmx_name = "S90-Pmanual-Gmanual";
    insert_session_row(&db, "Pmanual", "Gmanual", zmx_name, "running", "zmx");

    let listeners = compute_portless_owned_listeners_from_snapshot(
        &db,
        "name=S90-Pmanual-Gmanual\tpid=100\tclients=1\tcreated=1\tstart_dir=/private",
        r#"
100 1 /bundle/zmx run S90-Pmanual-Gmanual
101 100 -zsh
220 101 npm run dev
221 220 node vite
"#
        .trim(),
        r#"
p221
cnode
n*:5173
"#
        .trim(),
    )
    .expect("owned listeners");

    assert_eq!(
        listeners,
        vec![owned_listener(
            "Pmanual", "Gmanual", zmx_name, None, 5173, 221
        )]
    );
}

#[test]
fn owned_listener_detection_ignores_external_project_looking_listener() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Pexternal", "External App");
    insert_session_row(
        &db,
        "Pexternal",
        "Gterminal",
        "S90-Pexternal-Gterminal",
        "running",
        "zmx",
    );

    let listeners = compute_portless_owned_listeners_from_snapshot(
        &db,
        "name=S90-Pexternal-Gterminal\tpid=100\tclients=1\tcreated=1\tstart_dir=/private",
        r#"
100 1 /bundle/zmx run S90-Pexternal-Gterminal
101 100 -zsh
700 1 /Applications/Visual Studio Code.app/Contents/MacOS/Electron /tmp/Pexternal
701 700 node /tmp/Pexternal/node_modules/vite/bin/vite.js
"#
        .trim(),
        r#"
p701
cPexternal-dev
n127.0.0.1:5173
"#
        .trim(),
    )
    .expect("owned listeners");

    assert_eq!(listeners, Vec::<PortlessOwnedListener>::new());
}

#[test]
fn owned_listener_detection_ignores_sleeping_stopped_and_missing_sessions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Pstale", "Stale Sessions");
    insert_session_row(
        &db,
        "Pstale",
        "Gsleep",
        "S90-Pstale-Gsleep",
        "sleeping",
        "zmx",
    );
    insert_session_row(&db, "Pstale", "Gstop", "S90-Pstale-Gstop", "stopped", "zmx");
    insert_session_row(&db, "Pstale", "Gmiss", "S90-Pstale-Gmiss", "missing", "zmx");

    let listeners = compute_portless_owned_listeners_from_snapshot(
        &db,
        r#"
name=S90-Pstale-Gsleep pid=200 clients=1
name=S90-Pstale-Gstop pid=300 clients=1
name=S90-Pstale-Gmiss pid=400 clients=1
"#
        .trim(),
        r#"
200 1 -zsh
201 200 node stale-sleep
300 1 -zsh
301 300 node stale-stop
400 1 -zsh
401 400 node stale-missing
"#
        .trim(),
        r#"
p201
n*:3000
p301
n*:5173
p401
n*:8080
"#
        .trim(),
    )
    .expect("owned listeners");

    assert_eq!(listeners, Vec::<PortlessOwnedListener>::new());
}

#[test]
fn owned_listener_detection_drops_exited_listener_absent_from_current_tree() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "Pexit", "Exit App");
    let zmx_name = "S90-Pexit-Gterm";
    insert_session_row(&db, "Pexit", "Gterm", zmx_name, "running", "zmx");
    let zmx_list = "name=S90-Pexit-Gterm pid=500 clients=1";
    let listener_output = r#"
p521
n*:3000
"#
    .trim();

    let first = compute_portless_owned_listeners_from_snapshot(
        &db,
        zmx_list,
        r#"
500 1 /bundle/zmx run S90-Pexit-Gterm
520 500 npm run dev
521 520 node vite
"#
        .trim(),
        listener_output,
    )
    .expect("first owned listeners");
    assert_eq!(
        first,
        vec![owned_listener("Pexit", "Gterm", zmx_name, None, 3000, 521)]
    );

    let current = compute_portless_owned_listeners_from_snapshot(
        &db,
        zmx_list,
        r#"
500 1 /bundle/zmx run S90-Pexit-Gterm
520 500 npm run dev
"#
        .trim(),
        listener_output,
    )
    .expect("current owned listeners");
    assert_eq!(current, Vec::<PortlessOwnedListener>::new());
}

#[test]
fn owned_listener_detection_preserves_worktree_parent_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project_with_path(
        &db,
        "Pparent",
        "Parent App",
        Some("/tmp/parent-app"),
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
    let zmx_name = "S90-Pwtfix-Gdev";
    insert_session_row(&db, "Pwtfix", "Gdev", zmx_name, "running", "zmx");

    let listeners = compute_portless_owned_listeners_from_snapshot(
        &db,
        "name=S90-Pwtfix-Gdev pid=600 clients=1",
        r#"
600 1 /bundle/zmx run S90-Pwtfix-Gdev
601 600 -zsh
602 601 npm run dev
"#
        .trim(),
        r#"
p602
n[::1]:8080
"#
        .trim(),
    )
    .expect("owned listeners");

    assert_eq!(
        listeners,
        vec![owned_listener(
            "Pwtfix",
            "Gdev",
            zmx_name,
            Some("Pparent"),
            8080,
            602
        )]
    );
}

#[test]
fn listener_parser_accepts_lsof_field_rows_and_rejects_invalid_pids_and_ports() {
    let rows = parse_portless_tcp_listener_rows(
        r#"
p0
n*:3000
p42
cnode
n*:0
n*:65536
n*:5173
n[::1]:8080 (LISTEN)
p999999999999
n*:9999
p43
nTCP 127.0.0.1:8000 (LISTEN)
LISTEN 0 511 127.0.0.1:5174 0.0.0.0:* users:(("node",pid=44,fd=23))
LISTEN 0 511 [::1]:9000 [::]:* users:(("vite",pid=45,fd=24))
LISTEN 0 511 *:0 *:* users:(("bad",pid=46,fd=24))
LISTEN 0 511 *:4242 *:* users:(("bad",pid=0,fd=24))
"#
        .trim(),
    );

    assert_eq!(
        rows,
        vec![
            PortlessTcpListenerRow {
                pid: 42,
                port: 5173
            },
            PortlessTcpListenerRow {
                pid: 42,
                port: 8080
            },
            PortlessTcpListenerRow {
                pid: 43,
                port: 8000
            },
            PortlessTcpListenerRow {
                pid: 44,
                port: 5174
            },
            PortlessTcpListenerRow {
                pid: 45,
                port: 9000
            },
        ]
    );
}
