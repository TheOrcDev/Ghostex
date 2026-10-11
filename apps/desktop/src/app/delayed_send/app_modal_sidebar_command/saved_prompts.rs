//! Saved Prompts commands: listing, saving, tagging, deleting and inserting saved prompts, and jumping to the session a prompt came from.

use std::time::Duration;

use gpui::ClipboardItem;

use crate::app::helpers::*;
use crate::*;

impl GhostexGpuiApp {
    pub(super) fn handle_gpui_app_modal_saved_prompts_command(
        &mut self,
        command_type: &str,
        command: &serde_json::Map<String, serde_json::Value>,
        cx: &mut gpui::Context<Self>,
    ) {
        match command_type {
            "requestStashedPrompts" => {
                /*
                CDXC:SavedPrompts 2026-07-29:
                The Prompts modal loads stashed prompt-editor saves on demand
                through the local gxserver daemon. The answer is a transient
                `stashedPromptsResult` sidebarState payload the modal host
                forwards as a window message; prompt bodies stay inside that
                round trip and are never logged or stored by Rust.
                */
                let request_id = command
                    .get("requestId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let project_id = command
                    .get("projectId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let include_recovery = command
                    .get("includeRecovery")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true);
                let include_delivered = command
                    .get("includeDelivered")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true);
                self.run_gpui_app_modal_sidebar_status_task(
                    move || {
                        gpui_stashed_prompts_result_message(
                            &request_id,
                            project_id.as_deref(),
                            include_recovery,
                            include_delivered,
                        )
                    },
                    cx,
                );
            }
            "saveStashedPrompt" => {
                let request_id = command
                    .get("requestId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let Some(content) = command
                    .get("content")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                else {
                    return;
                };
                let project_id = command
                    .get("projectId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let prompt_id = command
                    .get("promptId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let session_id = command
                    .get("sessionId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let tag_ids = command
                    .get("tagIds")
                    .and_then(serde_json::Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    });
                self.run_gpui_app_modal_sidebar_status_task(
                    move || {
                        gpui_save_stashed_prompt_result_message(
                            &request_id,
                            &content,
                            prompt_id.as_deref(),
                            project_id.as_deref(),
                            session_id.as_deref(),
                            tag_ids.as_deref(),
                        )
                    },
                    cx,
                );
            }
            "saveStashedPromptTag" => {
                /*
                CDXC:SavedPrompts 2026-08-23:
                Tag create/rename runs through the same local gxserver daemon as
                the prompts, and answers with the whole refreshed catalogue so
                the modal's rail cannot drift from what is stored.
                */
                let request_id = command
                    .get("requestId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let Some(name) = command
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                else {
                    return;
                };
                let color = command
                    .get("color")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let tag_id = command
                    .get("tagId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                self.run_gpui_app_modal_sidebar_status_task(
                    move || {
                        gpui_save_stashed_prompt_tag_result_message(
                            &request_id,
                            &name,
                            color.as_deref(),
                            tag_id.as_deref(),
                        )
                    },
                    cx,
                );
            }
            "deleteStashedPromptTag" => {
                let request_id = command
                    .get("requestId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let Some(tag_id) = command
                    .get("tagId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                else {
                    return;
                };
                self.run_gpui_app_modal_sidebar_status_task(
                    move || gpui_delete_stashed_prompt_tag_result_message(&request_id, &tag_id),
                    cx,
                );
            }
            "setStashedPromptTags" => {
                let request_id = command
                    .get("requestId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let Some(prompt_id) = command
                    .get("promptId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                else {
                    return;
                };
                let tag_ids = command
                    .get("tagIds")
                    .and_then(serde_json::Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                self.run_gpui_app_modal_sidebar_status_task(
                    move || {
                        gpui_set_stashed_prompt_tags_result_message(
                            &request_id,
                            &prompt_id,
                            &tag_ids,
                        )
                    },
                    cx,
                );
            }
            "deleteStashedPrompt" => {
                if let Some(prompt_id) = command
                    .get("promptId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                {
                    cx.background_executor()
                        .spawn(async move {
                            let _ = gpui_gxserver_rpc_result(
                                "/api/deleteStashedPrompt",
                                &serde_json::json!({ "promptId": prompt_id }),
                                Duration::from_secs(5),
                            );
                        })
                        .detach();
                }
            }
            "insertStashedPrompt" => {
                let Some(content) = command
                    .get("content")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                else {
                    return;
                };
                let session_id = command
                    .get("sessionId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let inserted = session_id.as_deref().is_some_and(|session_id| {
                    self.insert_stashed_prompt_into_agents_session(
                        session_id,
                        &content,
                        command
                            .get("promptId")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|id| id.starts_with("recovered:")),
                        cx,
                    )
                });
                if !inserted {
                    /*
                    CDXC:SavedPrompts 2026-07-29:
                    When the originating terminal is gone (closed tab, sleeping
                    session, all-projects row from another project), fall back
                    to the clipboard and say so instead of silently dropping
                    the selected prompt.
                    */
                    gpui_copy_to_clipboard(ClipboardItem::new_string(content), cx);
                    self.dispatch_gpui_app_modal_toast(
                        "info",
                        "Prompt copied to clipboard",
                        "The session's terminal is not available for direct insert.",
                        cx,
                    );
                }
            }
            /*
            CDXC:SavedPrompts 2026-08-24:
            Saved Prompts rows carry the raw gxserver ids of the session they
            were stashed from plus that session's provider conversation id. The
            modal closes itself (like the Quick Access rows above), so this arm
            only hands the bounded selector to the Rust store
            (stashed_prompt_jump.rs, `gx_store_open_conversation`), which
            owns the present → restore → resume routing.
            */
            "jumpToStashedPromptSession" => {
                let project_id = command
                    .get("projectId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let session_id = command
                    .get("sessionId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let agent_session_id = command
                    .get("agentSessionId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let _ = self.dispatch_gpui_stashed_prompt_session_jump(
                    project_id.as_deref(),
                    session_id.as_deref(),
                    agent_session_id.as_deref(),
                    cx,
                );
            }
            _ => {}
        }
    }
}
