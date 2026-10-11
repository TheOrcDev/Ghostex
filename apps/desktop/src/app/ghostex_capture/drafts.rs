//! Unsent prompts become Ghostex draft sessions.
//!
//! CDXC:GhostexCapture 2026-09-30 DECISION:
//! User: "every draft is a newly created session in ghostex so this is kept in ghostex", and "if i
//! just open the prompt screen and want to click on the prompt button there should be a button to
//! continue (icon button on it)". Closing the prompt box with unsent text makes (or updates) a draft
//! session in the target project: its agent starts in the background like a sidebar draft, and the
//! text with its screenshot links is that session's chat draft, so it shows in the sidebar and
//! survives restarts. "Write a prompt" starts a fresh prompt; the continue button on it reopens the
//! box on the draft. A prompt aimed at a running session stays in the box until a fresh prompt is
//! started, and is kept as a new draft session in that session's project then.

use std::path::PathBuf;

use ghostex_gx_core::{ProjectKey, SessionKey};
use gpui::Context;
use serde_json::{Value, json};

use super::send::{Plan, compose, upload_images};
use super::targets::Target;
use crate::GhostexGpuiApp;
use crate::app::gx_store::gx_rpc;
use crate::app::model::GpuiRemoteGxserverRequestTarget;

/// The `clientId` drafts saved by Ghostex Capture carry.
pub(super) const DRAFT_CLIENT_ID: &str = "ghostex-capture";

/// The draft session a prompt box's text is kept in.
#[derive(Clone)]
pub(crate) struct DraftSession {
    pub(crate) key: SessionKey,
    pub(crate) remote: Option<GpuiRemoteGxserverRequestTarget>,
}

/// Saves `content` as a session's chat draft (empty clears it).
pub(super) async fn set_chat_draft(
    remote: Option<GpuiRemoteGxserverRequestTarget>,
    key: &SessionKey,
    content: &str,
) -> Result<(), String> {
    gx_rpc(
        remote,
        "/api/setSessionChatDraft",
        json!({
            "clientId": DRAFT_CLIENT_ID,
            "content": content,
            "projectId": key.project_id,
            "sessionId": key.session_id,
        }),
    )
    .await
    .map(|_| ())
    .map_err(|error| error.message)
}

impl GhostexGpuiApp {
    /// Whether the box holds something a continue button can bring back.
    pub(crate) fn ghostex_capture_has_draft(&self) -> bool {
        let prompt = &self.ghostex_capture.prompt;
        prompt.draft_session.is_some()
            || !prompt.draft.trim().is_empty()
            || !prompt.attachments.is_empty()
    }

    /// Keeps the box's unsent text in Ghostex: updates its draft session, or makes one when the
    /// prompt was aimed at a new session (or `force`, for a prompt aimed at a running session that
    /// is being replaced by a fresh one).
    pub(super) fn save_ghostex_capture_draft(&mut self, force: bool, cx: &mut Context<Self>) {
        let prompt = &self.ghostex_capture.prompt;
        let text = prompt.draft.trim().to_string();
        if text.is_empty() && prompt.attachments.is_empty() {
            return;
        }
        let images: Vec<(u32, PathBuf)> = prompt
            .attachments
            .iter()
            .map(|attachment| (attachment.number, attachment.path.clone()))
            .collect();
        if let Some(draft) = prompt.draft_session.clone() {
            cx.background_executor()
                .spawn(async move {
                    let uploaded = upload_images(
                        &draft.remote,
                        &draft.key.project_id,
                        &draft.key.session_id,
                        &images,
                    )
                    .await?;
                    set_chat_draft(draft.remote, &draft.key, &compose(&text, &uploaded)).await
                })
                .detach();
            return;
        }
        let project_target = match prompt.target.clone() {
            Some(Target::NewSession { project, title }) => Target::NewSession { project, title },
            Some(Target::Session {
                session,
                project_title,
                ..
            }) if force => {
                let Some(key) = SessionKey::parse_sidebar_session_id(&session) else {
                    return;
                };
                Target::NewSession {
                    project: ProjectKey {
                        machine: key.machine,
                        project_id: key.project_id,
                    }
                    .to_workspace_project_id(),
                    title: project_title,
                }
            }
            _ => return,
        };
        let Ok(Plan::NewSession {
            remote,
            key,
            params,
            title,
        }) = self.ghostex_capture_send_plan(&project_target, None)
        else {
            return;
        };
        let generation = self.ghostex_capture.prompt.generation;
        cx.spawn(async move |this, cx| {
            let created = async {
                let created = gx_rpc(
                    remote.clone(),
                    "/api/createAgentSession",
                    Value::Object(params),
                )
                .await
                .map_err(|error| error.message)?;
                let session_id = created
                    .pointer("/session/sessionId")
                    .and_then(Value::as_str)
                    .filter(|id| !id.trim().is_empty())
                    .ok_or_else(|| "Could not create the draft session.".to_string())?
                    .to_string();
                let project_id = created
                    .pointer("/session/projectId")
                    .and_then(Value::as_str)
                    .filter(|id| !id.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| key.project_id.clone());
                let session = SessionKey {
                    machine: key.machine.clone(),
                    project_id,
                    session_id,
                };
                // A sidebar draft runs its agent in the background so trust and login screens show
                // up before the first prompt; this one does the same.
                gx_rpc(
                    remote.clone(),
                    "/api/startSessionProvider",
                    json!({ "projectId": session.project_id, "sessionId": session.session_id }),
                )
                .await
                .map_err(|error| error.message)?;
                let uploaded =
                    upload_images(&remote, &session.project_id, &session.session_id, &images)
                        .await?;
                set_chat_draft(remote.clone(), &session, &compose(&text, &uploaded)).await?;
                Ok::<_, String>(session)
            }
            .await;
            let _ = this.update(cx, |app, cx| match created {
                Ok(session) => {
                    let prompt = &mut app.ghostex_capture.prompt;
                    // A fresh prompt started meanwhile: the draft stays in Ghostex on its own.
                    if prompt.generation != generation {
                        return;
                    }
                    prompt.target = Some(Target::Session {
                        session: session.to_sidebar_session_id(),
                        title: "Draft".into(),
                        project_title: title,
                    });
                    prompt.draft_session = Some(DraftSession {
                        key: session,
                        remote,
                    });
                    cx.notify();
                }
                Err(message) => {
                    app.dispatch_gpui_app_modal_toast(
                        "warning",
                        "Could not keep the draft",
                        &message,
                        cx,
                    );
                }
            });
        })
        .detach();
    }

    /// "Write a prompt": a fresh prompt. Whatever the box held is kept in Ghostex first.
    pub(super) fn start_fresh_ghostex_capture_prompt(&mut self, cx: &mut Context<Self>) {
        if self.ghostex_capture.prompt.window.is_some() {
            self.open_ghostex_capture_prompt(cx);
            return;
        }
        if self.ghostex_capture_has_draft() {
            self.save_ghostex_capture_draft(true, cx);
            let prompt = &mut self.ghostex_capture.prompt;
            prompt.generation += 1;
            prompt.draft.clear();
            prompt.attachments.clear();
            prompt.next_number = 0;
            prompt.target = None;
            prompt.draft_session = None;
        }
        self.open_ghostex_capture_prompt(cx);
    }

    /// After a send: the draft session the text was kept in has nothing left to keep.
    pub(super) fn clear_ghostex_capture_draft_session(&mut self, cx: &mut Context<Self>) {
        if let Some(draft) = self.ghostex_capture.prompt.draft_session.take() {
            cx.background_executor()
                .spawn(async move { set_chat_draft(draft.remote, &draft.key, "").await })
                .detach();
        }
    }
}
