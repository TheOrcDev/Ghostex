//! Absent versus `null` versus a value.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wire key whose absence and whose `null` mean different things.
///
/// A plain `Option<T>` collapses the two. Examples: `gitRemoteOriginUrl` (absent = not probed,
/// `null` = probed and no origin) and the chat `accountSwitch` key (absent = a daemon without
/// queue support, `null` = none).
///
/// Use it on a field with `#[serde(default, skip_serializing_if = "Tri::is_absent")]`: a missing
/// key takes the default (`Absent`), a present key goes through `Deserialize` (`Null` or `Value`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tri<T> {
    /// The key is not on the wire.
    #[default]
    Absent,
    /// The key is present with an explicit `null`.
    Null,
    /// The key is present with a value.
    Value(T),
}

impl<T> Tri<T> {
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// The value, when there is one. `Absent` and `Null` both give `None`; callers that need the
    /// difference match on the enum.
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Absent | Self::Null => None,
        }
    }

    pub fn into_value(self) -> Option<T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Absent | Self::Null => None,
        }
    }

    /// `None` when the key is absent, `Some(None)` for `null`, `Some(Some(value))` otherwise.
    pub fn into_nested_option(self) -> Option<Option<T>> {
        match self {
            Self::Absent => None,
            Self::Null => Some(None),
            Self::Value(value) => Some(Some(value)),
        }
    }
}

impl<T> From<Option<Option<T>>> for Tri<T> {
    fn from(value: Option<Option<T>>) -> Self {
        match value {
            None => Self::Absent,
            Some(None) => Self::Null,
            Some(Some(value)) => Self::Value(value),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Tri<T> {
    /// Only reached when the key is present, so `None` here is an explicit `null`.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

impl<T: Serialize> Serialize for Tri<T> {
    /// `Absent` serializes as `null` when the field is not skipped; pair the field with
    /// `skip_serializing_if = "Tri::is_absent"` to keep the key off the wire.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(value) => serializer.serialize_some(value),
            Self::Absent | Self::Null => serializer.serialize_none(),
        }
    }
}
