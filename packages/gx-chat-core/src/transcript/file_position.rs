//! The editor coordinates a path may carry: `:913`, `:913-940`, `:42:8`.
//!
//! Ported from `packages/shared/session-chat-presentation/file-position.ts`. The rule there is the
//! regular expression `/:(\d{1,7})(?:-(\d{1,7})|(?::(\d{1,7})))?$/`, matched by hand because this
//! crate has no regular-expression dependency. Two properties of that expression are load bearing
//! and are reproduced literally below: the engine takes the LEFTMOST colon whose remainder reaches
//! the end of the string (`a:1:2:3` splits at the second colon, not the third), and a suffix that
//! matches the shape but fails the validity test leaves the whole string as the path rather than
//! falling back to a shorter split.

use serde::{Deserialize, Serialize};

/// Where in a file a reference points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePosition {
    pub line: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u64>,
}

/// The leading run of ASCII digits.
fn digit_run(value: &str) -> &str {
    let end = value.bytes().take_while(u8::is_ascii_digit).count();
    &value[..end]
}

/// The three shapes the suffix can take, before the validity test.
struct Suffix {
    line: u64,
    end_line: Option<u64>,
    column: Option<u64>,
}

/// Matches `(\d{1,7})(?:-(\d{1,7})|(?::(\d{1,7})))?$` against the whole of `rest`.
///
/// A shorter first number can never work: the character after it would be a digit, and none of the
/// three continuations accepts one. So the run is taken whole and the quantifier's upper bound is a
/// plain length test.
fn match_suffix(rest: &str) -> Option<Suffix> {
    let first = digit_run(rest);
    if first.is_empty() || first.len() > 7 {
        return None;
    }
    let line = first.parse().ok()?;
    let tail = &rest[first.len()..];
    if tail.is_empty() {
        return Some(Suffix {
            line,
            end_line: None,
            column: None,
        });
    }
    let separator = tail.as_bytes()[0];
    if separator != b'-' && separator != b':' {
        return None;
    }
    let second = digit_run(&tail[1..]);
    if second.is_empty() || second.len() > 7 || 1 + second.len() != tail.len() {
        return None;
    }
    let number = second.parse().ok()?;
    Some(if separator == b'-' {
        Suffix {
            line,
            end_line: Some(number),
            column: None,
        }
    } else {
        Suffix {
            line,
            end_line: None,
            column: Some(number),
        }
    })
}

/// Splits a path from a valid line, line-range, or line-and-column suffix.
pub fn split_file_position(value: &str) -> (&str, Option<FilePosition>) {
    for (index, byte) in value.bytes().enumerate() {
        if byte != b':' {
            continue;
        }
        let Some(suffix) = match_suffix(&value[index + 1..]) else {
            continue;
        };
        let valid = suffix.line >= 1
            && suffix.end_line.is_none_or(|end| end >= suffix.line)
            && suffix.column.is_none_or(|column| column >= 1);
        if !valid {
            return (value, None);
        }
        return (
            &value[..index],
            Some(FilePosition {
                line: suffix.line,
                end_line: suffix.end_line,
                column: suffix.column,
            }),
        );
    }
    (value, None)
}

/// The exact coordinate suffix shown beside a file name and in its tooltip.
pub fn file_position_suffix(position: Option<&FilePosition>) -> String {
    let Some(position) = position else {
        return String::new();
    };
    let lines = match position.end_line {
        Some(end) => format!("{}-{end}", position.line),
        None => position.line.to_string(),
    };
    match position.column {
        Some(column) => format!(":{lines}:{column}"),
        None => format!(":{lines}"),
    }
}
