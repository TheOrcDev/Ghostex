//! Tolerant deserializers.
//!
//! gxserver builds its frames with `json!` and copies several row values onto the wire verbatim,
//! so a key that is normally a string, a bool, a list, or a count can arrive as `null`, as `1.0`,
//! or as `-1` from an old or damaged row. One such value must not fail a whole snapshot, because a
//! machine whose snapshot fails never loads. The rules:
//!
//! - A defaulted field reads `null` as its default ([`null_as_default`]).
//! - A count reads any JSON number ([`lenient_u64`], [`lenient_opt_u64`], [`lenient_opt_i64`]).
//! - A list of ids keeps its string elements ([`lenient_strings`]).
//! - A list of rows is read row by row; a row that still does not fit is skipped and counted, never
//!   silently ([`Rows`]).
//!
//! A value of the wrong kind (a string where a bool belongs) still fails its row.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use serde::de::{DeserializeOwned, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Number, Value};

/// Reads `null` as `T::default()`. Pair with `#[serde(default)]` so an absent key does the same.
pub fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

fn number_as_u64(number: &Number) -> Option<u64> {
    number.as_u64().or_else(|| {
        number
            .as_f64()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|value| value.floor() as u64)
    })
}

fn number_as_i64(number: &Number) -> Option<i64> {
    number.as_i64().or_else(|| {
        number
            .as_f64()
            .filter(|value| value.is_finite())
            .map(|value| value.trunc() as i64)
    })
}

/// A count: `3`, `3.0`, and `3.7` read as 3; `null` and negative numbers read as 0.
pub fn lenient_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    Ok(Option::<Number>::deserialize(deserializer)?
        .as_ref()
        .and_then(number_as_u64)
        .unwrap_or(0))
}

/// An optional count: like [`lenient_u64`], but `null` and negative numbers read as `None`.
pub fn lenient_opt_u64<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u64>, D::Error> {
    Ok(Option::<Number>::deserialize(deserializer)?
        .as_ref()
        .and_then(number_as_u64))
}

/// An optional signed number: floats are truncated, `null` reads as `None`.
pub fn lenient_opt_i64<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<i64>, D::Error> {
    Ok(Option::<Number>::deserialize(deserializer)?
        .as_ref()
        .and_then(number_as_i64))
}

/// A list of ids read element by element: elements that are not strings are left out instead of
/// failing the row that carries the list. `null` reads as an empty list.
///
/// Used for `sessionIds`, where dropping the whole group over one bad element would leave a
/// project that has sessions with an empty tab list. The store re-derives a group's list from its
/// session rows when a row is missing from it, so a left-out element is repaired, not lost.
pub fn lenient_strings<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    Ok(Option::<Vec<Value>>::deserialize(deserializer)?
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| match value {
            Value::String(id) => Some(id),
            _ => None,
        })
        .collect())
}

/// A row that did not fit its type and was left out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedRow {
    /// Position in the wire list.
    pub index: usize,
    /// `projectId` and `sessionId` of the row when it has them, for the host's log.
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub error: String,
}

/// A wire list read row by row. Rows that do not fit `T` are left out and listed in `skipped`, so
/// one damaged row cannot keep a machine from loading, and the host can still log that it
/// happened. Serializes as the plain list of rows; `null` reads as an empty list.
#[derive(Clone, Debug, PartialEq)]
pub struct Rows<T> {
    pub rows: Vec<T>,
    pub skipped: Vec<SkippedRow>,
}

impl<T> Default for Rows<T> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            skipped: Vec::new(),
        }
    }
}

impl<T> From<Vec<T>> for Rows<T> {
    fn from(rows: Vec<T>) -> Self {
        Self {
            rows,
            skipped: Vec::new(),
        }
    }
}

impl<T> Deref for Rows<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.rows
    }
}

impl<T: Serialize> Serialize for Rows<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.rows.serialize(serializer)
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Rows<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RowsVisitor<T>(PhantomData<T>);

        impl<'de, T: DeserializeOwned> Visitor<'de> for RowsVisitor<T> {
            type Value = Rows<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a list of rows")
            }

            fn visit_unit<E>(self) -> Result<Rows<T>, E> {
                Ok(Rows::default())
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Rows<T>, A::Error> {
                let mut rows = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(4096));
                let mut skipped = Vec::new();
                let mut index = 0;
                // Each row is buffered as a value first: a streaming parser cannot resume after a
                // row fails half way through.
                while let Some(value) = seq.next_element::<Value>()? {
                    let id = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
                    let (project_id, session_id) = (id("projectId"), id("sessionId"));
                    match T::deserialize(value) {
                        Ok(row) => rows.push(row),
                        Err(error) => skipped.push(SkippedRow {
                            index,
                            project_id,
                            session_id,
                            error: error.to_string(),
                        }),
                    }
                    index += 1;
                }
                Ok(Rows { rows, skipped })
            }
        }

        deserializer.deserialize_any(RowsVisitor(PhantomData))
    }
}
