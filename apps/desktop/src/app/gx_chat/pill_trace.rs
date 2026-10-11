//! TEMPORARY: the model pill flicker hunt (2026-09-30). Remove once the cause is fixed.
//!
//! A new Claude chat on the desktop showed its model pill, then a bare "Model", then the model
//! again; the GPUI web build never reproduced it. After every event a chat handles, this computes
//! the pill's inputs with the core's own `compute_native_chat_options` and writes one
//! `gxChat.modelPillTrace` record when any of them moved, naming the event that moved them. Behind
//! the same two gates as `diagnostics.rs` (Show debug UI controls and the Chat area). It records
//! ids, model names and counters only, never message text.

use std::cell::RefCell;
use std::collections::HashMap;

use ghostex_gx_chat_core::{ChatContext, ChatState, Event};
use serde_json::{Value, json};

use crate::support_logs::{self, GpuiDiagnosticScenario, GpuiSupportLog};

thread_local! {
    /// The last facts written per chat, so only a change is logged.
    static LAST: RefCell<HashMap<String, Value>> = RefCell::new(HashMap::new());
}

/// Both gates, read once per drive.
pub(super) fn enabled() -> bool {
    crate::shared_settings::shared_sidebar_settings_snapshot().debugging_mode()
        && support_logs::scenario_enabled(GpuiDiagnosticScenario::SessionChat)
}

/// The event's kind, without any of its payload beyond stream positions and code names.
pub(super) fn event_kind(event: &Event) -> String {
    match event {
        Event::Start(_) => "start".to_string(),
        Event::Frame(frame) => {
            let position = frame.position();
            format!(
                "{}@{}:{}",
                frame.wire_type(),
                position.epoch,
                position.seq
            )
        }
        Event::Connection(update) => format!("connection.{update:?}"),
        Event::RpcSettled { request_id, .. } => format!("rpcSettled#{request_id}"),
        Event::Action(action) => format!("action.{}", action.kind.as_str()),
        Event::Tick => "tick".to_string(),
        Event::ComposerBootRead(_) => "composerBootRead".to_string(),
        Event::ModelCatalogChanged { .. } => "modelCatalogChanged".to_string(),
        Event::RetainedSnapshotLoaded { .. } => "retainedSnapshotLoaded".to_string(),
        Event::StorageLoaded { .. } => "storageLoaded".to_string(),
        Event::StorageBatchLoaded { .. } => "storageBatchLoaded".to_string(),
        _ => "other".to_string(),
    }
}

/// The pill's inputs after an event, logged when they moved.
pub(super) fn observe(key: &str, event: &str, state: &ChatState, context: &ChatContext) {
    let options = ghostex_gx_chat_core::menus::options::compute_native_chat_options(state, context);
    let model_id = options.catalog.as_ref().map(|catalog| catalog.model.id.clone());
    let store_model = model_id
        .as_deref()
        .and_then(|id| options.state.get(id))
        .map(|entry| json!({"value": entry.value, "label": entry.label, "source": format!("{:?}", entry.source)}));
    let selected_model = state
        .session
        .selected_options
        .as_ref()
        .map(|selected| {
            json!({
                "model": selected.get("model"),
                "detectedAt": selected.get("detectedAt"),
            })
        });
    let facts = json!({
        "pillModel": options.option_labels.model,
        "pillDisplay": options.option_labels.model_display,
        "showModel": options.option_labels.show_model,
        "screenProbed": state.session.screen_probed,
        "agent": state.session.agent,
        "agentSessionId": state.session.agent_session_id,
        "sessionAgentId": state.session.session_agent_id,
        "availableAgents": state.session.available_agents.is_some(),
        "storageKey": state.menus.options.storage_key,
        "optionsSeeded": state.menus.options_seeded,
        "storeGeneration": state.menus.options_store_generation,
        "catalogGeneration": state.menus.model_catalog_generation,
        "selectedGeneration": state.session.selected_options_generation,
        "hasCatalog": options.catalog.is_some(),
        "storeModel": store_model,
        "selected": selected_model,
        "epoch": state.messages.position.epoch,
        "seq": state.messages.position.seq,
    });
    let changed = LAST.with(|last| {
        let mut last = last.borrow_mut();
        if last.get(key) == Some(&facts) {
            return false;
        }
        last.insert(key.to_string(), facts.clone());
        true
    });
    if changed {
        support_logs::append(
            GpuiSupportLog::SessionChat,
            "gxChat.modelPillTrace",
            json!({"chat": key, "event": event, "facts": facts}),
        );
    }
}
