use serde_json::{json, Value};

use super::*;
use crate::domain::DomainRepository;

#[test]
fn live_process_identity_promotes_running_zmx_terminal_to_codex() {
    let (_temp, db) = open_test_database();
    let repository = DomainRepository::new(&db, "test-server");
    let project = repository
        .create_project(
            json!({ "name": "Ghostex", "path": std::env::temp_dir() })
                .as_object()
                .expect("project params"),
        )
        .expect("create project");
    let project_id = project
        .get("projectId")
        .and_then(Value::as_str)
        .expect("project id")
        .to_string();
    let session = repository
        .create_session(
            json!({
                "kind": "terminal",
                "lifecycleState": "running",
                "projectId": project_id,
                "runtimeSettings": {
                    "sessionPersistenceProvider": "zmx",
                    "titleSource": "user"
                },
                "surface": "workspace",
                "title": "Sidebar scrolls after closing (set above)"
            })
            .as_object()
            .expect("session params"),
            false,
        )
        .expect("create session");
    let session_id = session
        .get("sessionId")
        .and_then(Value::as_str)
        .expect("session id")
        .to_string();

    let changed = apply_live_process_session_identity(
        &repository,
        &session,
        &project_id,
        &session_id,
        Some("codex".to_string()),
        Some("019EB8D0-D27B-7F30-B6D7-7A04AB8FAE78".to_string()),
        None,
    )
    .expect("apply live process identity");

    assert!(changed);
    let updated = repository
        .get_session(&project_id, &session_id)
        .expect("get updated session")
        .expect("updated session");
    assert_eq!(updated.get("kind"), Some(&json!("agent")));
    assert_eq!(updated.get("agentId"), Some(&json!("codex")));
    assert_eq!(
        updated.get("title"),
        Some(&json!("Sidebar scrolls after closing (set above)"))
    );
    let runtime_settings = updated
        .get("runtimeSettings")
        .and_then(Value::as_object)
        .expect("runtime settings");
    assert_eq!(runtime_settings.get("agentName"), Some(&json!("codex")));
    assert_eq!(runtime_settings.get("launchAgentId"), Some(&json!("codex")));
    assert_eq!(
        runtime_settings.get("agentSessionId"),
        Some(&json!("019EB8D0-D27B-7F30-B6D7-7A04AB8FAE78"))
    );
}

#[test]
fn live_process_identity_claims_codex_id_observed_before_process_promotion() {
    let (temp, db) = open_test_database();
    let repository = DomainRepository::new(&db, "test-server");
    let project = repository
        .create_project(
            json!({
                "name": "Ghostex",
                "path": temp.path().to_string_lossy()
            })
            .as_object()
            .expect("project params"),
        )
        .expect("create project");
    let project_id = project
        .get("projectId")
        .and_then(Value::as_str)
        .expect("project id")
        .to_string();
    let session = repository
        .create_session(
            json!({
                "kind": "terminal",
                "lifecycleState": "running",
                "projectId": project_id,
                "runtimeSettings": {
                    "agentActivity": {
                        "lastTitle": "019ff871-8b5c-7ce2-bcf7-5409263e2e0e"
                    },
                    "sessionPersistenceProvider": "zmx",
                    "titleSource": "placeholder"
                },
                "surface": "workspace",
                "title": "Terminal Session"
            })
            .as_object()
            .expect("session params"),
            false,
        )
        .expect("create session");
    let session_id = session
        .get("sessionId")
        .and_then(Value::as_str)
        .expect("session id")
        .to_string();

    let changed = apply_live_process_session_identity(
        &repository,
        &session,
        &project_id,
        &session_id,
        Some("codex".to_string()),
        None,
        None,
    )
    .expect("apply live process identity");

    assert!(changed);
    let updated = repository
        .get_session(&project_id, &session_id)
        .expect("get updated session")
        .expect("updated session");
    assert_eq!(updated.get("kind"), Some(&json!("agent")));
    assert_eq!(updated.get("agentId"), Some(&json!("codex")));
    assert_eq!(
        updated
            .get("runtimeSettings")
            .and_then(Value::as_object)
            .and_then(|settings| settings.get("agentSessionId")),
        Some(&json!("019ff871-8b5c-7ce2-bcf7-5409263e2e0e"))
    );
}

#[test]
fn live_process_identity_replaces_wsl_shell_title_without_claiming_codex_auto_title_job() {
    let (temp, db) = open_test_database();
    let repository = DomainRepository::new(&db, "test-server");
    let project = repository
        .create_project(
            json!({
                "name": "Ghostex",
                "path": temp.path().to_string_lossy()
            })
            .as_object()
            .expect("project params"),
        )
        .expect("create project");
    let project_id = project
        .get("projectId")
        .and_then(Value::as_str)
        .expect("project id")
        .to_string();
    let session = repository
        .create_session(
            json!({
                "kind": "terminal",
                "lifecycleState": "running",
                "projectId": project_id,
                "runtimeSettings": {
                    "sessionPersistenceProvider": "zmx",
                    "titleSource": "terminal-auto"
                },
                "surface": "workspace",
                "title": "madda@M7-Desktop: /mnt/c/dev/Ghostex"
            })
            .as_object()
            .expect("session params"),
            false,
        )
        .expect("create session");
    let session_id = session
        .get("sessionId")
        .and_then(Value::as_str)
        .expect("session id")
        .to_string();

    let changed = apply_live_process_session_identity(
        &repository,
        &session,
        &project_id,
        &session_id,
        Some("codex".to_string()),
        Some("019EB8D0-D27B-7F30-B6D7-7A04AB8FAE78".to_string()),
        None,
    )
    .expect("apply live process identity");

    assert!(changed);
    let updated = repository
        .get_session(&project_id, &session_id)
        .expect("get updated session")
        .expect("updated session");
    assert_eq!(updated.get("kind"), Some(&json!("agent")));
    assert_eq!(updated.get("agentId"), Some(&json!("codex")));
    assert_eq!(updated.get("title"), Some(&json!("Codex Session")));
    assert_eq!(
        updated
            .get("runtimeSettings")
            .and_then(Value::as_object)
            .and_then(|settings| settings.get("titleSource")),
        Some(&json!("placeholder"))
    );

    let lifecycle = LifecycleParams {
        project_id: project_id.clone(),
        session_id: session_id.clone(),
    };
    let result = ingest_agent_hook_event(
        &repository,
        &lifecycle,
        json!({
            "agentName": "codex",
            "agentSessionId": "019EB8D0-D27B-7F30-B6D7-7A04AB8FAE78",
            "eventName": "UserPromptSubmit",
            "firstUserMessage": "Please summarize this repository"
        })
        .as_object()
        .expect("hook params"),
        temp.path(),
    )
    .expect("hook result");

    assert_ne!(
        result.get("reason"),
        Some(&json!("first-prompt-auto-title-claimed"))
    );
    let hooked_session = result.get("session").expect("hooked session");
    assert_eq!(hooked_session.get("title"), Some(&json!("Codex Session")));
    assert_eq!(
        hooked_session
            .get("runtimeSettings")
            .and_then(Value::as_object)
            .and_then(|settings| settings.get("gxserverFirstPromptAutoTitleStatus")),
        None
    );
}
