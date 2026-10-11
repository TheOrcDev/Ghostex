use serde_json::{json, Map, Value};

use super::*;
use crate::domain::DomainRepository;

#[test]
fn search_matches_case_insensitive_project_text_and_paginates() {
    let projects = vec![project("P100", "Search Project", false, false)];
    let sessions = vec![
        session("P100", "G100", "First", "running", 1000.0),
        session("P100", "G101", "Second", "running", 2000.0),
    ];
    let params = json!({
        "limit": 1,
        "query": "search project",
    });
    let result = search_sessions(
        projects,
        sessions,
        params.as_object().expect("params object"),
    )
    .expect("search sessions");
    let results = result
        .get("results")
        .and_then(Value::as_array)
        .expect("results");
    assert_eq!(results.len(), 1);
    assert_eq!(result.get("cursor").and_then(Value::as_str), Some("1"));
    assert_eq!(
        results[0]
            .get("match")
            .and_then(Value::as_object)
            .and_then(|matched| matched.get("field"))
            .and_then(Value::as_str),
        Some("project")
    );
}

#[test]
fn list_previous_sessions_reads_domain_rows_with_closed_at() {
    let (_temp, db) = open_test_database();
    let repository = DomainRepository::new(&db, "S7k");
    let project = repository
        .create_project(
            json!({
                "name": "History",
                "path": std::env::temp_dir(),
            })
            .as_object()
            .expect("project params"),
        )
        .expect("project created");
    let project_id = string_field(&project, "projectId").expect("project id");
    let session = repository
        .create_session(
            json!({
                "agentId": "codex",
                "kind": "agent",
                "lifecycleState": "stopped",
                "projectId": project_id,
                "providerState": {
                    "lifecycleState": "missing",
                    "probedAt": "2026-06-06T12:00:00.000Z",
                    "provider": "zmx",
                },
                "runtimeSettings": {
                    "titleSource": "terminal-auto",
                },
                "title": "Restorable session",
            })
            .as_object()
            .expect("session params"),
            false,
        )
        .expect("session created");

    let result = list_previous_sessions(&db, "S7k", &Map::new()).expect("previous sessions");
    let results = result
        .get("results")
        .and_then(Value::as_array)
        .expect("results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get("sessionId"), session.get("sessionId"));
    assert_eq!(
        results[0].get("closedAt").and_then(Value::as_str),
        Some("2026-06-06T12:00:00.000Z")
    );
}

#[test]
fn previous_sessions_filter_candidates_and_return_closed_at() {
    let projects = vec![project("P100", "History", false, false)];
    let trusted = previous_session(
        "G100",
        "Trusted title",
        "stopped",
        "workspace",
        Some("2026-06-06T12:00:00.000Z"),
        "2026-06-06T12:30:00.000Z",
        "2026-06-01T09:00:00.000Z",
    );
    let placeholder = previous_session(
        "G101",
        "Search by Text",
        "stopped",
        "workspace",
        Some("2026-06-07T12:00:00.000Z"),
        "2026-06-07T12:30:00.000Z",
        "2026-06-07T09:00:00.000Z",
    );
    let mut favorite_placeholder = previous_session(
        "G102",
        "Search by Text",
        "stopped",
        "workspace",
        None,
        "2026-06-05T12:30:00.000Z",
        "2026-06-05T09:00:00.000Z",
    );
    favorite_placeholder
        .as_object_mut()
        .expect("favorite object")
        .insert(
            "sessionTag".to_string(),
            Value::String("favorite".to_string()),
        );
    let mut command_pinned = previous_session(
        "G103",
        "Pinned command",
        "stopped",
        "commands",
        Some("2026-06-08T12:00:00.000Z"),
        "2026-06-08T12:30:00.000Z",
        "2026-06-08T09:00:00.000Z",
    );
    command_pinned
        .as_object_mut()
        .expect("command object")
        .insert("isPinned".to_string(), Value::Bool(true));
    let running = previous_session(
        "G104",
        "Running",
        "running",
        "workspace",
        Some("2026-06-09T12:00:00.000Z"),
        "2026-06-09T12:30:00.000Z",
        "2026-06-09T09:00:00.000Z",
    );

    let result = search_previous_sessions(
        projects,
        vec![
            trusted,
            placeholder,
            favorite_placeholder,
            command_pinned,
            running,
        ],
        &Map::new(),
    )
    .expect("previous sessions");
    let results = result
        .get("results")
        .and_then(Value::as_array)
        .expect("results");

    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.get("sessionId").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        vec!["G100", "G102"]
    );
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.get("closedAt").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        vec!["2026-06-06T12:00:00.000Z", "2026-06-05T12:30:00.000Z"]
    );

    let query_params = json!({ "query": "09:00:00.000Z" });
    let query_result = search_previous_sessions(
        vec![project("P100", "History", false, false)],
        vec![
            previous_session(
                "G100",
                "Trusted title",
                "stopped",
                "workspace",
                Some("2026-06-06T12:00:00.000Z"),
                "2026-06-06T12:30:00.000Z",
                "2026-06-01T09:00:00.000Z",
            ),
            previous_session(
                "G102",
                "Favorite title",
                "stopped",
                "workspace",
                None,
                "2026-06-05T12:30:00.000Z",
                "2026-06-05T10:00:00.000Z",
            ),
        ],
        query_params.as_object().expect("query params"),
    )
    .expect("previous session query");
    let query_results = query_result
        .get("results")
        .and_then(Value::as_array)
        .expect("query results");
    assert_eq!(query_results.len(), 1);
    assert_eq!(
        query_results[0].get("sessionId").and_then(Value::as_str),
        Some("G100")
    );
    assert_eq!(
        query_results[0]
            .get("match")
            .and_then(Value::as_object)
            .and_then(|matched| matched.get("field"))
            .and_then(Value::as_str),
        Some("timestamp")
    );
}

#[test]
fn previous_sessions_rank_by_close_time_then_session_id() {
    let projects = vec![project("P100", "History", false, false)];
    let closed_recent = previous_session(
        "G1close",
        "Closed recently",
        "stopped",
        "workspace",
        Some("2026-06-06T12:00:00.000Z"),
        "2026-06-06T12:00:00.000Z",
        "2026-06-01T09:00:00.000Z",
    );
    let active_before_close = previous_session(
        "G2active",
        "Active before close",
        "stopped",
        "workspace",
        Some("2026-06-05T12:00:00.000Z"),
        "2026-06-05T12:00:00.000Z",
        "2026-06-07T09:00:00.000Z",
    );
    let metadata_edited = previous_session(
        "G3meta",
        "Metadata edited after close",
        "stopped",
        "workspace",
        Some("2026-06-04T12:00:00.000Z"),
        "2026-06-08T12:00:00.000Z",
        "2026-06-04T09:00:00.000Z",
    );
    let same_time_later_id = previous_session(
        "G9same",
        "Same close later id",
        "stopped",
        "workspace",
        Some("2026-06-06T12:00:00.000Z"),
        "2026-06-06T12:00:00.000Z",
        "2026-06-06T09:00:00.000Z",
    );
    let same_time_earlier_id = previous_session(
        "G0same",
        "Same close earlier id",
        "stopped",
        "workspace",
        Some("2026-06-06T12:00:00.000Z"),
        "2026-06-06T12:00:00.000Z",
        "2026-06-06T09:00:00.000Z",
    );

    let result = search_previous_sessions(
        projects,
        vec![
            active_before_close,
            metadata_edited,
            same_time_later_id,
            closed_recent,
            same_time_earlier_id,
        ],
        &Map::new(),
    )
    .expect("previous sessions");
    let results = result
        .get("results")
        .and_then(Value::as_array)
        .expect("results");

    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.get("sessionId").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        vec!["G0same", "G1close", "G9same", "G2active", "G3meta"]
    );
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.get("closedAt").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        vec![
            "2026-06-06T12:00:00.000Z",
            "2026-06-06T12:00:00.000Z",
            "2026-06-06T12:00:00.000Z",
            "2026-06-05T12:00:00.000Z",
            "2026-06-04T12:00:00.000Z"
        ]
    );
}

#[test]
fn search_normalizes_unicode_query_project_id_and_bad_tags_like_typescript() {
    let projects = vec![
        project("P100", "Unicode", false, false),
        project("P200", "Other", false, false),
    ];
    let sessions = vec![
        session("P100", "G100", "\u{00dc}ber Build", "running", 1000.0),
        session("P200", "G200", "Other Build", "running", 2000.0),
    ];

    let params = json!({
        "projectId": "",
        "query": "\u{00fc}ber",
    });
    let result = search_sessions(
        projects.clone(),
        sessions.clone(),
        params.as_object().expect("params object"),
    )
    .expect("unicode search");
    let results = result
        .get("results")
        .and_then(Value::as_array)
        .expect("results");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].get("sessionId").and_then(Value::as_str),
        Some("G100")
    );

    let truthy_project_id = json!({ "projectId": true });
    let result = search_sessions(
        projects.clone(),
        sessions.clone(),
        truthy_project_id.as_object().expect("params object"),
    )
    .expect("truthy project id search");
    assert_eq!(
        result
            .get("results")
            .and_then(Value::as_array)
            .expect("results")
            .len(),
        0
    );

    let bad_tags = json!({ "sessionTags": "favorite" });
    let error = search_sessions(
        projects,
        sessions,
        bad_tags.as_object().expect("params object"),
    )
    .expect_err("bad sessionTags should match TypeScript internal error");
    assert_eq!(error.code, "internalError");
    assert_eq!(error.message, "values?.filter is not a function");
}

#[test]
fn search_includes_untagged_sessions_when_untagged_filter_is_selected() {
    let projects = vec![project("P100", "Tags", false, false)];
    let mut tagged = session("P100", "G-tagged", "Tagged", "running", 1000.0);
    tagged
        .as_object_mut()
        .expect("session object")
        .insert("sessionTag".to_string(), json!("in-progress"));
    let untagged = session("P100", "G-untagged", "Untagged", "running", 2000.0);
    let mut favorite = session("P100", "G-favorite", "Favorite", "running", 3000.0);
    favorite
        .as_object_mut()
        .expect("session object")
        .insert("isFavorite".to_string(), json!(true));
    let sessions = vec![tagged, untagged, favorite];

    let result = search_sessions(
        projects.clone(),
        sessions.clone(),
        json!({ "sessionTags": ["untagged"] })
            .as_object()
            .expect("params object"),
    )
    .expect("untagged search");
    let results = result
        .get("results")
        .and_then(Value::as_array)
        .expect("results");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].get("sessionId").and_then(Value::as_str),
        Some("G-untagged")
    );

    let mixed_result = search_sessions(
        projects,
        sessions,
        json!({ "sessionTags": ["untagged", "in-progress"] })
            .as_object()
            .expect("params object"),
    )
    .expect("mixed tag search");
    let mixed_ids: Vec<&str> = mixed_result
        .get("results")
        .and_then(Value::as_array)
        .expect("results")
        .iter()
        .filter_map(|row| row.get("sessionId").and_then(Value::as_str))
        .collect();
    assert_eq!(mixed_ids.len(), 2);
    assert!(mixed_ids.contains(&"G-untagged"));
    assert!(mixed_ids.contains(&"G-tagged"));
    assert!(!mixed_ids.contains(&"G-favorite"));
}

#[test]
fn search_does_not_treat_provider_off_rows_as_active() {
    let projects = vec![project("P100", "Provider Off", false, false)];
    let mut provider_off = session("P100", "G100", "Provider off", "unknown", 1000.0);
    provider_off
        .as_object_mut()
        .expect("session object")
        .insert(
            "providerState".to_string(),
            json!({ "lifecycleState": "exists", "provider": "zmx" }),
        );
    provider_off
        .as_object_mut()
        .expect("session object")
        .insert(
            "runtimeSettings".to_string(),
            json!({ "sessionPersistenceProvider": "off" }),
        );
    let params = json!({
        "includeActive": true,
        "includePrevious": false,
    });

    let result = search_sessions(
        projects,
        vec![provider_off],
        params.as_object().expect("params object"),
    )
    .expect("provider off search");
    assert_eq!(
        result
            .get("results")
            .and_then(Value::as_array)
            .expect("results")
            .len(),
        0
    );
}

#[test]
fn previous_sessions_reject_non_restorable_title_noise() {
    let projects = vec![project("P100", "History", false, false)];
    let trusted = previous_session(
        "G100",
        "Trusted title",
        "stopped",
        "workspace",
        Some("2026-06-06T12:00:00.000Z"),
        "2026-06-06T12:30:00.000Z",
        "2026-06-01T09:00:00.000Z",
    );
    let path_title = previous_session(
        "G101",
        "/Users/madda/private",
        "stopped",
        "workspace",
        Some("2026-06-07T12:00:00.000Z"),
        "2026-06-07T12:30:00.000Z",
        "2026-06-07T09:00:00.000Z",
    );
    let command_title = previous_session(
        "G102",
        "codex resume 019e7f01-8243-7aa1-88db-dd84ebcf6aa4",
        "stopped",
        "workspace",
        Some("2026-06-08T12:00:00.000Z"),
        "2026-06-08T12:30:00.000Z",
        "2026-06-08T09:00:00.000Z",
    );
    let generic_title = previous_session(
        "G103",
        "Terminal Session",
        "stopped",
        "workspace",
        Some("2026-06-09T12:00:00.000Z"),
        "2026-06-09T12:30:00.000Z",
        "2026-06-09T09:00:00.000Z",
    );
    let gx_id_title = previous_session(
        "G104",
        "G1abc",
        "stopped",
        "workspace",
        Some("2026-06-10T12:00:00.000Z"),
        "2026-06-10T12:30:00.000Z",
        "2026-06-10T09:00:00.000Z",
    );

    let result = search_previous_sessions(
        projects,
        vec![
            trusted,
            path_title,
            command_title,
            generic_title,
            gx_id_title,
        ],
        &Map::new(),
    )
    .expect("previous sessions");
    let results = result
        .get("results")
        .and_then(Value::as_array)
        .expect("results");
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.get("sessionId").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        vec!["G100"]
    );
}
