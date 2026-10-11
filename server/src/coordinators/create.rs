//! Creating a coordinator: `/api/createAgentSession` with a `coordinator` object.

use rusqlite::Connection;
use serde_json::{Map, Value};

use super::records::{
    insert_coordinator, COORDINATOR_GOAL_MAX_CHARS, COORDINATOR_INSTRUCTIONS_MAX_CHARS,
};
use crate::domain::DomainStateError;

/// The `coordinator` object of a create request: `{ goal?, instructions?, roleFile }`. `roleFile`
/// is filled in by gxserver, never by a client.
pub struct CoordinatorCreateRequest {
    pub goal: String,
    pub instructions: String,
    pub role_file: Option<String>,
}

pub fn coordinator_create_request(
    params: &Map<String, Value>,
) -> Result<Option<CoordinatorCreateRequest>, DomainStateError> {
    let Some(object) = params.get("coordinator") else {
        return Ok(None);
    };
    if object.is_null() {
        return Ok(None);
    }
    let object = object.as_object().ok_or_else(|| {
        DomainStateError::bad_request("coordinator must be an object with an optional goal.")
    })?;
    let field = |key: &str| {
        object
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_string()
    };
    let goal = field("goal");
    let instructions = field("instructions");
    if goal.chars().count() > COORDINATOR_GOAL_MAX_CHARS {
        return Err(DomainStateError::bad_request(format!(
            "Keep the goal under {COORDINATOR_GOAL_MAX_CHARS} characters; put details in the instructions."
        )));
    }
    if instructions.chars().count() > COORDINATOR_INSTRUCTIONS_MAX_CHARS {
        return Err(DomainStateError::bad_request(format!(
            "Standing instructions are limited to {COORDINATOR_INSTRUCTIONS_MAX_CHARS} characters."
        )));
    }
    let role_file = Some(field("roleFile")).filter(|value| !value.is_empty());
    Ok(Some(CoordinatorCreateRequest {
        goal,
        instructions,
        role_file,
    }))
}

/// Stores the coordinator row for a session `/api/createAgentSession` just created.
pub fn register_created_coordinator(
    db: &Connection,
    params: &Map<String, Value>,
    session: &Value,
) -> Result<bool, DomainStateError> {
    let Some(request) = coordinator_create_request(params)? else {
        return Ok(false);
    };
    let text = |key: &str| {
        session
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    insert_coordinator(
        db,
        &text("projectId"),
        &text("sessionId"),
        &request.goal,
        &request.instructions,
    )?;
    Ok(true)
}
