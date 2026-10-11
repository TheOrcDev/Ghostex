//! `POST /api/draftFeedback` and `POST /api/sendFeedback`.

use axum::http::StatusCode;
use serde_json::{json, Map, Value};

use crate::{
    domain::read_domain_rpc_params,
    protocol::{rpc_error, rpc_success},
    server::{routed_json, RoutedResponse},
};

use super::draft::{draft_title, FeedbackClient};
use super::relay::FeedbackSubmission;

/// `draftFeedback { app, message }` answers `{ title, body, footer }`: the issue the message
/// becomes, which the client shows for review and the user may edit, plus the line the relay adds
/// under it. `sendFeedback { app, title, body, images }` posts the reviewed issue and answers
/// `{ issueNumber, issueUrl }`.
pub(crate) async fn handle_feedback_http(
    endpoint_path: String,
    request_id: String,
    body: &Value,
) -> RoutedResponse {
    let params = match read_domain_rpc_params(body) {
        Ok(params) => params,
        Err(error) => {
            return feedback_error(
                endpoint_path,
                request_id,
                StatusCode::BAD_REQUEST,
                error.code,
                error.message,
            );
        }
    };
    let outcome = tokio::task::spawn_blocking({
        let endpoint_path = endpoint_path.clone();
        move || match endpoint_path.as_str() {
            "/api/draftFeedback" => draft_feedback(&params),
            _ => send_feedback(&params),
        }
    })
    .await;
    match outcome {
        Ok(Ok(result)) => routed_json(
            Some(endpoint_path),
            StatusCode::OK,
            rpc_success(request_id, result),
        ),
        Ok(Err((status, code, message))) => {
            feedback_error(endpoint_path, request_id, status, code, message)
        }
        Err(error) => feedback_error(
            endpoint_path,
            request_id,
            StatusCode::INTERNAL_SERVER_ERROR,
            "internalError",
            format!("Feedback request failed: {error}"),
        ),
    }
}

type FeedbackFailure = (StatusCode, &'static str, String);

fn client_for(params: &Map<String, Value>) -> Result<FeedbackClient, FeedbackFailure> {
    FeedbackClient::for_app(params.get("app").and_then(Value::as_str))
        .map_err(|message| (StatusCode::BAD_REQUEST, "badRequest", message))
}

fn draft_feedback(params: &Map<String, Value>) -> Result<Value, FeedbackFailure> {
    let client = client_for(params)?;
    let message = params
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if message.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "badRequest",
            "Write your feedback first.".to_string(),
        ));
    }
    Ok(json!({
        "title": draft_title(message),
        "body": message,
        "footer": client.footer(),
    }))
}

fn send_feedback(params: &Map<String, Value>) -> Result<Value, FeedbackFailure> {
    let client = client_for(params)?;
    let submission = FeedbackSubmission::from_params(params, client)
        .map_err(|message| (StatusCode::BAD_REQUEST, "badRequest", message))?;
    let (issue_number, issue_url) = submission
        .send()
        .map_err(|message| (StatusCode::BAD_GATEWAY, "feedbackRelay", message))?;
    Ok(json!({ "issueNumber": issue_number, "issueUrl": issue_url }))
}

fn feedback_error(
    endpoint_path: String,
    request_id: String,
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> RoutedResponse {
    routed_json(
        Some(endpoint_path),
        status,
        rpc_error(code, message.into(), Some(request_id)),
    )
}
