//! The model selection outbox: what this session is heading for, and why a pick is a change.
//!
//! Port of `packages/shared/session-chat-controller/model-selection.ts`.
//!
//! CDXC:SessionChat 2026-09-05 DECISION:
//! User: Option+P and model selection always work, including while the agent is working; an
//! undeliverable selection waits for the next opportunity. The local outbox covers disconnects
//! until gxserver accepts the durable intent.
//! SEE-ALSO: server/src/session_chat_model_selection.rs owns delivery, coalescing and retries
//! after the client closes.

use ghostex_gx_protocol::Tri;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::event::StorageKey;
use crate::menus::picker::model_picker::{
    model_picker_supports_session_scope, ModelPickerProvider, ModelPickerSelection,
    ModelSelectionScope,
};

/// How long a failed delivery waits before it is tried again.
pub const MODEL_OUTBOX_RETRY_MS: f64 = 5000.0;

/// The store the outbox lives in; the host owns the
/// `ghostex.model-selection-outbox.` prefix and the per-session suffix.
pub const MODEL_OUTBOX_STORE: &str = "modelOutbox";

/// The outbox record for one SCOPED option key.
pub fn model_outbox_key(option_key: &str) -> StorageKey {
    StorageKey {
        store: MODEL_OUTBOX_STORE.to_string(),
        suffix: option_key.to_string(),
    }
}

/// The scoped option key this session's outbox lives under.
///
/// `sessionOptions.sessionKey`, which is the session key until a draft session latches its agent
/// and `<sessionKey>#<agentId>` afterwards. Family e1 owns the latch; this reads it.
pub fn scoped_outbox_key(state: &crate::state::ChatState) -> String {
    state
        .menus
        .options
        .storage_key
        .clone()
        .unwrap_or_else(|| state.identity.session_key.clone())
}

/// [`model_outbox_key`] of [`scoped_outbox_key`].
pub fn scoped_model_outbox_key(state: &crate::state::ChatState) -> StorageKey {
    model_outbox_key(&scoped_outbox_key(state))
}

/// One durable intent: the selection, the options that ride with it, and its scope.
///
/// **The stored record is user data.** `id`, `model` and `effort` are required for a record to
/// count as readable at all, which is the check `modelSelectionPersistence.read` makes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// The field ORDER is `persist`'s object literal, because the crate builds with `serde_json`'s
/// `preserve_order` and the record is written to the user's own outbox: a selection stored by one
/// brain and read by the other must be the same bytes.
pub struct ModelSelectionIntent {
    pub model: String,
    pub effort: String,
    /// `mode` and `fastMode`, the two options a selection can carry.
    ///
    /// Always on the wire, even empty: `persist` spreads `{...latest.options, ...options}`, which
    /// is an object on every intent it writes.
    #[serde(default)]
    pub options: Map<String, Value>,
    /// Omitted means `default`, which is what every pick did before the scope existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<ModelSelectionScope>,
    pub id: String,
}

impl ModelSelectionIntent {
    /// The selection this intent carries.
    pub fn selection(&self) -> ModelPickerSelection {
        ModelPickerSelection {
            model: self.model.clone(),
            effort: self.effort.clone(),
        }
    }
}

/// `modelSelectionPersistence.read`: a record without the three required strings is no record.
pub fn parse_model_selection_intent(raw: Option<&str>) -> Option<ModelSelectionIntent> {
    let value: Value = serde_json::from_str(raw?).ok()?;
    let object = value.as_object()?;
    if !["id", "model", "effort"]
        .into_iter()
        .all(|field| object.get(field).is_some_and(Value::is_string))
    {
        return None;
    }
    serde_json::from_value(value).ok()
}

/// `JSON.stringify(value)` for the record written back.
pub fn serialize_model_selection_intent(intent: &ModelSelectionIntent) -> String {
    serde_json::to_string(intent).unwrap_or_else(|_| "null".to_string())
}

/// What the outbox is doing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelSelectionState {
    /// The durable intent waiting to reach gxserver, or `None`.
    pub outbox: Option<ModelSelectionIntent>,
    /// The selection gxserver already reports as pending, folded by family a.
    ///
    /// Three-state, because `desired` is `outbox ?? pending` and `JSON.stringify` leaves the key
    /// out only when both are `undefined`: absent, `null` and a value are three different
    /// documents. A `failed` one folds to `null` here, because a failed selection never applied
    /// and is not where the session is heading.
    pub pending: Tri<ModelSelectionIntent>,
    /// The reason the last selection was abandoned, for the surface that offered it.
    pub selection_error: Option<String>,
    /// When a failed delivery is retried.
    pub retry_at_ms: Option<f64>,
    /// A delivery is in flight; a second one never starts while one is running.
    pub delivering: bool,
    /// The intent id the option pills are currently showing as dispatched, and its receipt.
    ///
    /// `computeModelSelectionOutbox`'s second `useEffect`: while a selection is on its way, the
    /// pills read it rather than the agent's last confirmed values, and the receipt completes when
    /// the selection is acknowledged or replaced.
    pub dispatch_receipt: Option<(String, crate::menus::option_store::DispatchReceipt)>,
}

impl ModelSelectionState {
    /// `desired`: the outbox entry, else the pending selection.
    pub fn desired(&self) -> Option<&ModelSelectionIntent> {
        match (&self.outbox, &self.pending) {
            (Some(outbox), _) => Some(outbox),
            (None, Tri::Value(pending)) => Some(pending),
            _ => None,
        }
    }

    /// Whether `desired` is a key at all: `outbox ?? pending` is `undefined` only when the outbox
    /// is empty and no frame has carried a pending selection.
    pub fn desired_present(&self) -> bool {
        self.outbox.is_some() || !self.pending.is_absent()
    }

    /// Adopts what family a folded out of `pendingModelSelection`.
    ///
    /// A `failed` selection never applied, so it is not what this session is heading for; its
    /// message becomes [`ModelSelectionState::selection_error`] instead.
    pub fn adopt_pending(&mut self, pending: &Tri<Value>) {
        let value = match pending {
            Tri::Value(value) => Some(value),
            _ => None,
        };
        let state = value
            .and_then(|value| value.get("state"))
            .and_then(Value::as_str);
        if state == Some("failed") {
            self.pending = Tri::Null;
            self.selection_error = value
                .and_then(|value| value.get("errorMessage"))
                .and_then(Value::as_str)
                .map(str::to_string);
            return;
        }
        self.selection_error = None;
        self.pending = match pending {
            Tri::Absent => Tri::Absent,
            Tri::Null => Tri::Null,
            Tri::Value(value) => match serde_json::from_value(value.clone()) {
                Ok(intent) => Tri::Value(intent),
                Err(_) => Tri::Null,
            },
        };
    }

    /// `persist`: record a new intent. The id comes from the host, because the core has no random
    /// source.
    ///
    /// An options-only change carries no scope of its own, so it keeps the pending model choice's.
    pub fn persist(
        &mut self,
        selection: ModelPickerSelection,
        options: Option<&Map<String, Value>>,
        scope: Option<ModelSelectionScope>,
        id: String,
    ) -> ModelSelectionIntent {
        let inherited = scope.or_else(|| self.outbox.as_ref().and_then(|intent| intent.scope));
        let mut merged = self
            .outbox
            .as_ref()
            .map(|intent| intent.options.clone())
            .unwrap_or_default();
        if let Some(options) = options {
            for (key, value) in options {
                merged.insert(key.clone(), value.clone());
            }
        }
        let next = ModelSelectionIntent {
            id,
            model: selection.model,
            effort: selection.effort,
            options: merged,
            scope: inherited,
        };
        self.outbox = Some(next.clone());
        self.retry_at_ms = None;
        next
    }

    /// `selectOptions`: an options-only change keeps whatever model and effort the outbox holds.
    pub fn options_only_selection(&self) -> ModelPickerSelection {
        ModelPickerSelection {
            model: self
                .outbox
                .as_ref()
                .map(|intent| intent.model.clone())
                .unwrap_or_default(),
            effort: self
                .outbox
                .as_ref()
                .map(|intent| intent.effort.clone())
                .unwrap_or_default(),
        }
    }

    /// `persistence.acknowledge`: the delivery landed, so the record goes.
    pub fn acknowledge(&mut self, id: &str) -> bool {
        if self.outbox.as_ref().is_some_and(|intent| intent.id == id) {
            self.outbox = None;
            self.retry_at_ms = None;
            self.delivering = false;
            return true;
        }
        false
    }

    /// A delivery failed; it is tried again in five seconds.
    pub fn defer(&mut self, now_ms: f64) {
        self.delivering = false;
        self.retry_at_ms = Some(now_ms + MODEL_OUTBOX_RETRY_MS);
    }

    /// The `modelSelection` document key.
    ///
    /// `desired` is left out when it is `undefined`, which is what `JSON.stringify` does to the
    /// `outbox ?? pending` of a session no frame has carried a pending selection for.
    pub fn to_json(&self) -> Value {
        let mut object = Map::new();
        if self.desired_present() {
            object.insert(
                "desired".into(),
                serde_json::to_value(self.desired()).unwrap_or(Value::Null),
            );
        }
        object.insert(
            "outbox".into(),
            serde_json::to_value(&self.outbox).unwrap_or(Value::Null),
        );
        object.insert(
            "selectionError".into(),
            serde_json::to_value(&self.selection_error).unwrap_or(Value::Null),
        );
        Value::Object(object)
    }
}

/// `queuedModelSelection`'s three refusals, checked against what `selectSessionChatModel`
/// answered.
///
/// Returns the accepted `pendingModelSelection`, or the message the delivery failed with.
pub fn accept_queued_model_selection(
    result: &Value,
    requested_options: &Map<String, Value>,
    requested_scope: Option<ModelSelectionScope>,
) -> Result<Value, String> {
    let queued = result.get("queued").and_then(Value::as_bool) == Some(true);
    let pending = result.get("pendingModelSelection");
    let pending = match (queued, pending) {
        (true, Some(pending)) if !pending.is_null() => pending,
        _ => return Err("The server has not accepted this selection into its queue.".to_string()),
    };
    let accepted_options = pending.get("options");
    for (key, value) in requested_options {
        let accepted = accepted_options.and_then(|options| options.get(key));
        if accepted != Some(value) {
            return Err("Waiting for the server to support queued mode changes.".to_string());
        }
    }
    // An older daemon drops the field and would apply a session-only pick as the saved default.
    if requested_scope == Some(ModelSelectionScope::Session)
        && pending.get("scope").and_then(Value::as_str) != Some("session")
    {
        return Err("Waiting for the server to support session-only model picks.".to_string());
    }
    Ok(pending.clone())
}

/// `modelSelectionUnchanged`: whether applying this selection would change nothing.
pub fn model_selection_unchanged(
    selection: &ModelPickerSelection,
    desired: Option<&ModelSelectionIntent>,
    current_model: Option<&str>,
    current_effort: Option<&str>,
    provider: Option<ModelPickerProvider>,
    effortless: bool,
    scope: Option<ModelSelectionScope>,
) -> bool {
    // Where the agent can tell the two scopes apart, the same model with the other scope is a
    // change: it promotes a session-only choice to the saved default, or spares the default from
    // a pending one.
    if let (Some(scope), Some(provider)) = (scope, provider) {
        if model_picker_supports_session_scope(provider) {
            match desired {
                Some(desired) => {
                    if desired.scope.unwrap_or_default() != scope {
                        return false;
                    }
                }
                None if scope == ModelSelectionScope::Default => {
                    // The session already runs this model; whether the saved default does is
                    // unknown here.
                    return false;
                }
                None => {}
            }
        }
    }
    if let Some(desired) = desired {
        return desired.model == selection.model && desired.effort == selection.effort;
    }
    let model_matches = Some(selection.model.as_str()) == current_model;
    let effort_matches = selection.effort.as_str() == current_effort.unwrap_or("") || effortless;
    model_matches && effort_matches
}
