//! How full the context window is, and the three number formats the meter and the rows share.
//!
//! Port of `packages/shared/session-chat-presentation/context-usage.ts`.

use serde::{Deserialize, Serialize};

use crate::menus::picker::js::{is_finite, js_round, to_fixed_trimmed};

/// What Claude's statusline or Codex's transcript reported about the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsage {
    /// `context_window.used_percentage`, rounded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<f64>,
    /// `context_window.total_input_tokens`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_tokens: Option<f64>,
    /// `context_window.context_window_size`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_size: Option<f64>,
}

/// The meter's own view of that report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMeterUsage {
    /// 0 to 100, or `null` while the agent has not reported context usage.
    pub used_percentage: Option<f64>,
    pub used_tokens: Option<f64>,
    pub window_size: Option<f64>,
}

/// `isFiniteNonNegative`.
fn is_finite_non_negative(value: Option<f64>) -> bool {
    is_finite(value) && value.is_some_and(|value| value >= 0.0)
}

/// `resolveSessionChatContextMeterUsage`.
///
/// Claude uses tokens over window size; Codex uses its baseline-adjusted reported percentage.
/// `None` means the agent has not reported enough data to draw usage.
pub fn resolve_context_meter_usage(
    usage: Option<&ContextUsage>,
    prefer_reported_percentage: bool,
) -> Option<ContextMeterUsage> {
    let usage = usage?;
    let used_tokens = if is_finite_non_negative(usage.used_tokens) {
        usage.used_tokens
    } else {
        None
    };
    let window_size = if is_finite_non_negative(usage.window_size)
        && usage.window_size.is_some_and(|value| value > 0.0)
    {
        usage.window_size
    } else {
        None
    };
    let reported = is_finite_non_negative(usage.used_percentage);
    let used_percentage = if prefer_reported_percentage && reported {
        usage.used_percentage.map(|value| value.min(100.0))
    } else if let (Some(used), Some(window)) = (used_tokens, window_size) {
        Some((used / window * 100.0).min(100.0))
    } else if reported {
        usage.used_percentage.map(|value| value.min(100.0))
    } else {
        None
    };
    if used_percentage.is_none() && used_tokens.is_none() {
        return None;
    }
    Some(ContextMeterUsage {
        used_percentage,
        used_tokens,
        window_size,
    })
}

/// `formatSessionChatContextTokens`.
pub fn format_context_tokens(value: Option<f64>) -> String {
    let Some(value) = value.filter(|value| value.is_finite()) else {
        return "0".to_string();
    };
    if value < 1_000.0 {
        return format!("{}", js_round(value) as i64);
    }
    if value < 10_000.0 {
        return format!("{}k", to_fixed_trimmed(value / 1_000.0, 1));
    }
    if value < 1_000_000.0 {
        return format!("{}k", js_round(value / 1_000.0) as i64);
    }
    format!("{}m", to_fixed_trimmed(value / 1_000_000.0, 1))
}

/// `formatSessionChatContextPercentage`.
pub fn format_context_percentage(value: Option<f64>) -> Option<String> {
    let value = value.filter(|value| value.is_finite())?;
    if value < 10.0 {
        return Some(format!("{}%", to_fixed_trimmed(value, 1)));
    }
    Some(format!("{}%", js_round(value) as i64))
}

/// `formatSessionChatDuration`, family f's `session-chat-duration.ts`.
///
/// Family f owns that file. The context rows cannot draw without it, so this is the smallest copy
/// that keeps them honest; fold it into family f's port when that lands.
pub fn format_duration(ms: f64) -> String {
    let total_seconds = js_round(ms / 1000.0).max(0.0);
    let hours = (total_seconds / 3600.0).floor();
    let minutes = ((total_seconds % 3600.0) / 60.0).floor();
    let seconds = total_seconds % 60.0;
    if hours > 0.0 {
        return format!("{}h {:0>2}m", hours as i64, minutes as i64);
    }
    if minutes > 0.0 {
        return format!("{}m", minutes as i64);
    }
    format!("{}s", seconds as i64)
}

/// `formatResetCountdown`, `packages/shared/reset-countdown.ts` (deleted 2026-10-01).
///
/// CDXC:AgentProviders 2026-09-08 DECISION:
/// User: every reset stat shows days and hours at 24 hours or more (5d 12h), and only shows
/// minutes below 24 hours (19h 50m).
pub fn format_reset_countdown(ms: f64) -> String {
    let total_minutes = (ms / 60_000.0).floor().max(0.0);
    let hours = (total_minutes / 60.0).floor();
    if hours >= 24.0 {
        return format!(
            "{}d {}h",
            (hours / 24.0).floor() as i64,
            (hours % 24.0) as i64
        );
    }
    let minutes = (total_minutes % 60.0) as i64;
    if hours > 0.0 {
        format!("{}h {}m", hours as i64, minutes)
    } else {
        format!("{minutes}m")
    }
}

/// `maskAccountText`, `packages/shared/account-display.ts` (deleted 2026-10-01).
///
/// CDXC:AgentProviders 2026-09-10 DECISION:
/// Hide emails keeps the first and last characters before `@` and replaces every domain with the
/// same six `・` characters followed by `.•••`, including email-shaped account names.
pub fn mask_account_text(text: &str) -> String {
    // `/([^\s@]+)@[^\s@]+/gu`, written out because the crate carries no regex dependency.
    let characters: Vec<char> = text.chars().collect();
    let is_plain = |character: char| !character.is_whitespace() && character != '@';
    let mut out = String::new();
    let mut index = 0;
    while index < characters.len() {
        let start = index;
        while index < characters.len() && is_plain(characters[index]) {
            index += 1;
        }
        let address_end = index;
        if address_end > start && index < characters.len() && characters[index] == '@' {
            let mut domain = index + 1;
            while domain < characters.len() && is_plain(characters[domain]) {
                domain += 1;
            }
            if domain > index + 1 {
                let address = &characters[start..address_end];
                out.push(address[0]);
                out.push_str("•••");
                if address.len() > 1 {
                    out.push(address[address.len() - 1]);
                }
                out.push_str("@•••••.•••");
                index = domain;
                continue;
            }
        }
        if address_end > start {
            out.extend(&characters[start..address_end]);
            continue;
        }
        out.push(characters[index]);
        index += 1;
    }
    out
}
