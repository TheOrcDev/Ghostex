//! The catalog's data values: `J`, a `const`-friendly ordered JSON tree for the irregular tables,
//! `Opt`/`NumOpt` for option lists, and `Json`, the owned form everything renders through.
//!
//! CDXC:Settings 2026-10-01 WHY:
//! The Help files are written byte for byte the way `JSON.stringify(value, null, 2)` wrote them, so object keys keep their source order and numbers print the JavaScript way (`120`, not `120.0`). `serde_json::Value` only keeps key order with the `preserve_order` feature, which would change map order across all of gxserver, so the catalog renders through its own ordered `Json` and converts to `Value` only for the desktop's lookups.

use serde_json::{Map, Number, Value};

/// An ordered JSON tree that can live in a `const`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(&'static str),
    Arr(&'static [J]),
    Obj(&'static [(&'static str, J)]),
}

impl J {
    pub fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(entries) => entries
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&'static str> {
        match self {
            J::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            J::Num(number) => Some(*number),
            _ => None,
        }
    }

    pub fn as_array(&self) -> &'static [J] {
        match self {
            J::Arr(items) => items,
            _ => &[],
        }
    }

    pub fn entries(&self) -> &'static [(&'static str, J)] {
        match self {
            J::Obj(entries) => entries,
            _ => &[],
        }
    }
}

/// One `{ label, value }` option with a string value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opt {
    pub label: &'static str,
    pub value: &'static str,
}

pub const fn opt(label: &'static str, value: &'static str) -> Opt {
    Opt { label, value }
}

/// One `{ label, value }` option with a numeric value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumOpt {
    pub label: &'static str,
    pub value: f64,
}

pub const fn num_opt(label: &'static str, value: f64) -> NumOpt {
    NumOpt { label, value }
}

/// An owned, ordered JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj<K: Into<String>>(entries: impl IntoIterator<Item = (K, Json)>) -> Json {
        Json::Obj(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    pub fn str(text: impl Into<String>) -> Json {
        Json::Str(text.into())
    }

    /// The `serde_json::Value` with the same content (key order follows the caller's `Map`).
    pub fn to_value(&self) -> Value {
        match self {
            Json::Null => Value::Null,
            Json::Bool(flag) => Value::Bool(*flag),
            Json::Num(number) => js_number_value(*number),
            Json::Str(text) => Value::String(text.clone()),
            Json::Arr(items) => Value::Array(items.iter().map(Json::to_value).collect()),
            Json::Obj(entries) => {
                let mut map = Map::new();
                for (key, value) in entries {
                    map.insert(key.clone(), value.to_value());
                }
                Value::Object(map)
            }
        }
    }

    /// `JSON.stringify(value, null, 2)`.
    pub fn to_pretty_string(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out
    }

    fn write_pretty(&self, out: &mut String, depth: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
            Json::Num(number) => out.push_str(&js_number_string(*number)),
            Json::Str(text) => write_js_string(out, text),
            Json::Arr(items) if items.is_empty() => out.push_str("[]"),
            Json::Obj(entries) if entries.is_empty() => out.push_str("{}"),
            Json::Arr(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    out.push_str(if index == 0 { "\n" } else { ",\n" });
                    indent(out, depth + 1);
                    item.write_pretty(out, depth + 1);
                }
                out.push('\n');
                indent(out, depth);
                out.push(']');
            }
            Json::Obj(entries) => {
                out.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    out.push_str(if index == 0 { "\n" } else { ",\n" });
                    indent(out, depth + 1);
                    write_js_string(out, key);
                    out.push_str(": ");
                    value.write_pretty(out, depth + 1);
                }
                out.push('\n');
                indent(out, depth);
                out.push('}');
            }
        }
    }

    /// `JSON.stringify(value)` (no whitespace), as the Help Markdown prints defaults.
    pub fn to_compact_string(&self) -> String {
        let mut out = String::new();
        self.write_compact(&mut out);
        out
    }

    fn write_compact(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
            Json::Num(number) => out.push_str(&js_number_string(*number)),
            Json::Str(text) => write_js_string(out, text),
            Json::Arr(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write_compact(out);
                }
                out.push(']');
            }
            Json::Obj(entries) => {
                out.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_js_string(out, key);
                    out.push(':');
                    value.write_compact(out);
                }
                out.push('}');
            }
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// `JSON.stringify` of a string: `"` and `\` escaped, control characters as short or `\u00XX` escapes, everything else raw.
fn write_js_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// `String(number)` for the finite numbers the catalog holds.
pub fn js_number_string(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{}", number as i64)
    } else {
        format!("{number}")
    }
}

/// A `serde_json` number that prints like JavaScript: integral values stay integers.
fn js_number_value(number: f64) -> Value {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        Value::Number(Number::from(number as i64))
    } else {
        Number::from_f64(number)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

/// Anything the catalog can render as JSON.
pub trait ToJson {
    fn to_json(&self) -> Json;
}

impl ToJson for J {
    fn to_json(&self) -> Json {
        match self {
            J::Null => Json::Null,
            J::Bool(flag) => Json::Bool(*flag),
            J::Num(number) => Json::Num(*number),
            J::Str(text) => Json::str(*text),
            J::Arr(items) => Json::Arr(items.iter().map(ToJson::to_json).collect()),
            J::Obj(entries) => {
                Json::obj(entries.iter().map(|(key, value)| (*key, value.to_json())))
            }
        }
    }
}

impl ToJson for &str {
    fn to_json(&self) -> Json {
        Json::str(*self)
    }
}

impl ToJson for String {
    fn to_json(&self) -> Json {
        Json::str(self.clone())
    }
}

impl ToJson for f64 {
    fn to_json(&self) -> Json {
        Json::Num(*self)
    }
}

impl ToJson for bool {
    fn to_json(&self) -> Json {
        Json::Bool(*self)
    }
}

impl ToJson for Opt {
    fn to_json(&self) -> Json {
        Json::obj([
            ("label", Json::str(self.label)),
            ("value", Json::str(self.value)),
        ])
    }
}

impl ToJson for NumOpt {
    fn to_json(&self) -> Json {
        Json::obj([
            ("label", Json::str(self.label)),
            ("value", Json::Num(self.value)),
        ])
    }
}

impl<T: ToJson> ToJson for [T] {
    fn to_json(&self) -> Json {
        Json::Arr(self.iter().map(ToJson::to_json).collect())
    }
}

impl<T: ToJson> ToJson for Vec<T> {
    fn to_json(&self) -> Json {
        self.as_slice().to_json()
    }
}

impl<T: ToJson + ?Sized> ToJson for &T {
    fn to_json(&self) -> Json {
        (**self).to_json()
    }
}

impl<V: ToJson> ToJson for [(&str, V)] {
    fn to_json(&self) -> Json {
        Json::obj(self.iter().map(|(key, value)| (*key, value.to_json())))
    }
}
