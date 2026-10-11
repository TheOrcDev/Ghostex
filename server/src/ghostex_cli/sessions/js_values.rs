use serde_json::{Map, Value};

use crate::ghostex_cli::args::{FlagValue, Flags};

// ---------------------------------------------------------------------------
// JS value-semantics helpers (String()/??/||/truthiness on serde_json::Value)
// ---------------------------------------------------------------------------

/// JS `String(value)` for primitives.
pub(crate) fn js_display(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// JS `String(value ?? "")`: missing/undefined and null both become "".
pub(crate) fn js_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(other) => js_display(other),
    }
}

/// JS template-literal coercion: `${undefined}` -> "undefined", `${null}` -> "null".
pub(crate) fn js_template(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(value) => js_display(value),
    }
}

/// JS truthiness where `None` models `undefined`.
pub(crate) fn js_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => number
            .as_f64()
            .map(|number| number != 0.0 && !number.is_nan())
            .unwrap_or(false),
        Some(Value::String(text)) => !text.is_empty(),
        Some(_) => true,
    }
}

/// JS `a ?? b ?? c`: first non-nullish operand; otherwise the last operand's
/// own value (Some(Null) if it was null, None if it was undefined).
pub(crate) fn js_coalesce<'a>(chain: &[Option<&'a Value>]) -> Option<&'a Value> {
    for item in chain {
        if let Some(value) = *item {
            if !value.is_null() {
                return Some(value);
            }
        }
    }
    match chain.last() {
        Some(Some(value)) => Some(*value),
        _ => None,
    }
}

/// `key: a ?? b ?? c` object-literal semantics under JSON.stringify: the key is
/// dropped when the chain resolves to undefined, kept as null when it resolves
/// to an explicit null.
pub(super) fn insert_js(map: &mut Map<String, Value>, key: &str, chain: &[Option<&Value>]) {
    if let Some(value) = js_coalesce(chain) {
        map.insert(key.to_string(), value.clone());
    }
}

/// `key: obj.field` semantics: keep the value (even null) when the key exists,
/// drop it when the key is missing (undefined).
pub(super) fn insert_present(map: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(value) = value {
        map.insert(key.to_string(), value.clone());
    }
}

/// `key: obj.field ?? undefined` semantics: drop both missing and null.
pub(super) fn insert_non_null(map: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(value) = value {
        if !value.is_null() {
            map.insert(key.to_string(), value.clone());
        }
    }
}

/// Spread-override semantics (`{ ...session, key: a ?? b }`): a resolved
/// undefined removes the key (JSON.stringify drops it), anything else sets it.
pub(super) fn set_js(map: &mut Map<String, Value>, key: &str, chain: &[Option<&Value>]) {
    match js_coalesce(chain) {
        Some(value) => {
            map.insert(key.to_string(), value.clone());
        }
        None => {
            map.remove(key);
        }
    }
}

/// Stable map key for JS `Map` keys built from raw JSON values; None models an
/// undefined key so missing ids stay distinct from null/"null".
pub(super) fn value_key(value: Option<&Value>) -> Option<String> {
    value.map(|value| value.to_string())
}

/// JS strict `flags.x === true` (only a bare boolean flag counts).
pub(super) fn strict_true(flags: &Flags, key: &str) -> bool {
    matches!(flags.0.get(key), Some(FlagValue::Bool(true)))
}
