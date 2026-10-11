//! `JSON.stringify`, spelled out.
//!
//! Tool previews and the expanded tool body are built by serializing a tool's arguments, and that
//! text reaches the document. `serde_json::to_string` differs from `JSON.stringify` in one way that
//! matters here: a `f64` that happens to be whole prints as `1.0` where JavaScript prints `1`.
//! Everything else (the escape set, the two-space indent, the `null`/`true`/`false` spellings) is
//! already identical, and is reproduced here rather than reached for so the difference stays in one
//! readable place.

use serde_json::Value;

use crate::jsnum::js_number;

fn write_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control < ' ' => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// A number as `JSON.stringify` prints it, which is `Number.prototype.toString()`.
///
/// Every token `JSON.parse` produced is a double, so the whole rule is `crate::jsnum::js_number`:
/// no trailing `.0`, `1e+21` above the bound, `0` for `-0`. The old test asked serde whether the
/// token had been parsed as an integer, which answered differently for `3.0` and `3` and printed
/// ryu's `1e21` where JavaScript writes `1e+21`.
fn write_number(out: &mut String, value: &serde_json::Number) {
    match value.as_f64() {
        Some(float) => out.push_str(&js_number(float)),
        None => out.push_str(&value.to_string()),
    }
}

fn write_value(out: &mut String, value: &Value, indent: Option<usize>, depth: usize) {
    let (open_gap, close_gap) = match indent {
        Some(width) => (
            format!("\n{}", " ".repeat(width * (depth + 1))),
            format!("\n{}", " ".repeat(width * depth)),
        ),
        None => (String::new(), String::new()),
    };
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => write_number(out, number),
        Value::String(text) => write_string(out, text),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&open_gap);
                write_value(out, item, indent, depth + 1);
            }
            out.push_str(&close_gap);
            out.push(']');
        }
        Value::Object(entries) => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&open_gap);
                write_string(out, key);
                out.push(':');
                if indent.is_some() {
                    out.push(' ');
                }
                write_value(out, item, indent, depth + 1);
            }
            out.push_str(&close_gap);
            out.push('}');
        }
    }
}

/// `JSON.stringify(value)`.
pub fn stringify(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, None, 0);
    out
}

/// `JSON.stringify(value, null, spaces)`.
pub fn stringify_pretty(value: &Value, spaces: usize) -> String {
    let mut out = String::new();
    write_value(&mut out, value, Some(spaces), 0);
    out
}

/// `JSON.stringify` of an object literal, in the order the keys were written.
///
/// A `serde_json::Map` is a `BTreeMap` here, so serializing one sorts the keys. That is invisible
/// where the document is compared as JSON, but the two objects the native Markdown marks carry (the
/// code-block header and an inline image's source) are serialized INTO the Markdown string, where
/// the text itself is the contract. Those two are built through this.
pub fn stringify_pairs(entries: &[(&str, Value)]) -> String {
    let mut out = String::from("{");
    for (index, (key, value)) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        write_string(&mut out, key);
        out.push(':');
        write_value(&mut out, value, None, 0);
    }
    out.push('}');
    out
}
