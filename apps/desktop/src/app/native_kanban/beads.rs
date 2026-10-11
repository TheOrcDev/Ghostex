//! Beads calls for the native Kanban. Each one goes through the same gxserver-backed bridge the
//! React board's `ghostexProjectBeads` messages reach (`project_beads_bridge_result_for_request`),
//! so bd command construction stays in gxserver. These are blocking and run on the background
//! executor; a multi-step flow is one background job.

use serde_json::{Value, json};

use super::model::{
    BeadsIssue, is_bootstrap_issue_prefix, normalize_issue_prefix, reconciled_workflow_statuses,
};
use super::text::beads_error_message;
use crate::app::helpers::{
    ProjectBoardBridgeRuntimeContext, project_beads_bridge_error_response,
    project_beads_bridge_result_for_request,
};

const REQUEST_ID: &str = "native-kanban";

/// One Beads action. `Ok` carries bd's parsed JSON stdout (`Null` when it printed nothing).
pub(crate) fn beads_call(
    context: &ProjectBoardBridgeRuntimeContext,
    mut request: Value,
) -> Result<Value, String> {
    request["requestId"] = json!(REQUEST_ID);
    if let Some(project_id) = context.project_id.as_ref() {
        request["projectId"] = json!(project_id);
    }
    let response = project_beads_bridge_result_for_request(&request, Some(context))
        .unwrap_or_else(|error| project_beads_bridge_error_response(REQUEST_ID, &error));
    let text = |key: &str| response[key].as_str().unwrap_or_default().to_string();
    if response["exitCode"].as_i64().unwrap_or(1) != 0 {
        let stderr = text("stderr");
        let output = if stderr.trim().is_empty() {
            text("stdout")
        } else {
            stderr
        };
        return Err(beads_error_message(&output));
    }
    let stdout = text("stdout");
    let stdout = stdout.trim();
    if stdout.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(stdout)
        .map_err(|_| "Beads returned output the board could not read.".into())
}

/// `normalizeBeadsPayload`: bd's `{ data }` envelope, unwrapped.
pub(crate) fn beads_payload(payload: Value) -> Value {
    match payload {
        Value::Object(mut object) if object.contains_key("data") => {
            object.remove("data").unwrap_or(Value::Null)
        }
        other => other,
    }
}

fn config_string(payload: Value) -> String {
    match beads_payload(payload) {
        Value::String(value) => value,
        Value::Object(object) => object
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

/// `ensureIssuePrefix`: replaces only a stale bootstrap prefix with the project's own.
pub(crate) fn ensure_issue_prefix(
    context: &ProjectBoardBridgeRuntimeContext,
    desired: &str,
) -> Result<(), String> {
    let desired = normalize_issue_prefix(desired);
    let current = normalize_issue_prefix(&config_string(beads_call(
        context,
        json!({ "action": "configGetIssuePrefix" }),
    )?));
    if current != desired && is_bootstrap_issue_prefix(&current) {
        beads_call(
            context,
            json!({ "action": "renamePrefix", "value": format!("{desired}-") }),
        )?;
    }
    Ok(())
}

/// `readWorkflowStatuses`.
pub(crate) fn read_workflow_statuses(
    context: &ProjectBoardBridgeRuntimeContext,
) -> Result<String, String> {
    Ok(config_string(beads_call(
        context,
        json!({ "action": "configGet" }),
    )?))
}

/// `ensureWorkflowStatuses`.
pub(crate) fn ensure_workflow_statuses(
    context: &ProjectBoardBridgeRuntimeContext,
) -> Result<String, String> {
    let current = read_workflow_statuses(context)?;
    match reconciled_workflow_statuses(&current) {
        Some(next) => {
            beads_call(context, json!({ "action": "configSet", "value": next }))?;
            Ok(next)
        }
        None => Ok(current),
    }
}

pub(crate) fn list_issues(
    context: &ProjectBoardBridgeRuntimeContext,
) -> Result<Vec<BeadsIssue>, String> {
    let payload = beads_payload(beads_call(context, json!({ "action": "listIssues" }))?);
    Ok(payload
        .as_array()
        .map(|issues| issues.iter().filter_map(BeadsIssue::from_value).collect())
        .unwrap_or_default())
}

pub(crate) struct KanbanLoadResult {
    pub(crate) column_config: String,
    pub(crate) issues: Vec<BeadsIssue>,
}

pub(crate) fn load_board(
    context: &ProjectBoardBridgeRuntimeContext,
    reconcile: bool,
    issue_prefix: &str,
) -> Result<KanbanLoadResult, String> {
    let column_config = if reconcile {
        ensure_issue_prefix(context, issue_prefix)?;
        ensure_workflow_statuses(context)?
    } else {
        read_workflow_statuses(context)?
    };
    Ok(KanbanLoadResult {
        column_config,
        issues: list_issues(context)?,
    })
}

pub(crate) fn show_issue(
    context: &ProjectBoardBridgeRuntimeContext,
    issue_id: &str,
) -> Result<Option<BeadsIssue>, String> {
    let payload = beads_payload(beads_call(
        context,
        json!({ "action": "show", "issueId": issue_id }),
    )?);
    let issue = match &payload {
        Value::Array(issues) => issues.first().and_then(BeadsIssue::from_value),
        other => BeadsIssue::from_value(other),
    };
    Ok(issue)
}

/// Everything Edit ticket's Save writes, in the React board's order.
pub(crate) struct KanbanSaveDraft {
    pub(crate) issue_id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) priority: String,
    pub(crate) estimate: Option<i64>,
    pub(crate) labels: Vec<String>,
    pub(crate) status: Option<String>,
    pub(crate) comment: String,
}

pub(crate) fn persist_ticket(
    context: &ProjectBoardBridgeRuntimeContext,
    draft: &KanbanSaveDraft,
) -> Result<(), String> {
    let id = &draft.issue_id;
    beads_call(
        context,
        json!({ "action": "updateTitle", "issueId": id, "title": draft.title }),
    )?;
    beads_call(
        context,
        json!({ "action": "updateDescription", "issueId": id, "description": draft.description }),
    )?;
    beads_call(
        context,
        json!({ "action": "updatePriority", "issueId": id, "priority": draft.priority }),
    )?;
    if let Some(estimate) = draft.estimate {
        beads_call(
            context,
            json!({ "action": "updateEstimate", "issueId": id, "estimate": estimate }),
        )?;
    }
    if !draft.labels.is_empty() {
        beads_call(
            context,
            json!({ "action": "setLabels", "issueId": id, "labels": draft.labels }),
        )?;
    }
    if let Some(status) = draft.status.as_ref() {
        beads_call(
            context,
            json!({ "action": "updateStatus", "issueId": id, "status": status }),
        )?;
    }
    if !draft.comment.trim().is_empty() {
        beads_call(
            context,
            json!({ "action": "addComment", "issueId": id, "comment": draft.comment.trim() }),
        )?;
    }
    Ok(())
}

pub(crate) struct KanbanCreateDraft {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) priority: String,
    pub(crate) estimate: Option<i64>,
    pub(crate) labels: Vec<String>,
}

/// `create`, then the created bead: from bd's answer, or found in a fresh list when bd's answer
/// carried no id (`resolveCreatedIssueFromRefresh`).
pub(crate) fn create_issue(
    context: &ProjectBoardBridgeRuntimeContext,
    issue_prefix: &str,
    draft: &KanbanCreateDraft,
    known_ids: &[String],
) -> Result<BeadsIssue, String> {
    ensure_issue_prefix(context, issue_prefix)?;
    let mut request = json!({
        "action": "create",
        "description": draft.description,
        "labels": draft.labels,
        "priority": draft.priority,
        "title": draft.title,
    });
    if let Some(estimate) = draft.estimate {
        request["estimate"] = json!(estimate);
    }
    let created = beads_payload(beads_call(context, request)?);
    let created = match &created {
        Value::Array(issues) => issues.first().and_then(BeadsIssue::from_value),
        other => BeadsIssue::from_value(other),
    };
    if let Some(issue) = created {
        return Ok(issue);
    }
    list_issues(context)?
        .into_iter()
        .filter(|issue| {
            !known_ids.contains(&issue.id)
                && issue.title == draft.title
                && issue.description == draft.description
        })
        .max_by_key(|issue| {
            super::model::parse_time_ms(issue.created_at.as_deref().or(issue.updated_at.as_deref()))
                .unwrap_or(0)
        })
        .ok_or_else(|| "Created ticket was not found after create.".to_string())
}

/// The prompt agent's title for an untitled ticket, written back to bd.
pub(crate) fn generate_and_store_title(
    context: &ProjectBoardBridgeRuntimeContext,
    issue_id: &str,
    prompt: &str,
    agent_id: &str,
    agent_command: Option<&str>,
) -> Result<String, String> {
    let mut request = json!({
        "action": "generateTitle",
        "agentId": agent_id,
        "issueId": issue_id,
        "prompt": prompt,
    });
    if let Some(command) = agent_command {
        request["agentCommand"] = json!(command);
    }
    let generated = beads_payload(beads_call(context, request)?);
    let title = generated["title"]
        .as_str()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| "Prompt-agent title generation returned an empty title.".to_string())?
        .to_string();
    beads_call(
        context,
        json!({ "action": "updateTitle", "issueId": issue_id, "title": title }),
    )?;
    Ok(title)
}
