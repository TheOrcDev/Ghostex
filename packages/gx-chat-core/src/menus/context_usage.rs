//! The two context-usage helpers the account panel needs.
//!
//! Port of the part of `packages/shared/session-chat-presentation/context-usage.ts` that
//! `native-accounts.ts` calls. The context meter itself belongs to family e2
//! (`menus/context/`); when that lands, this file folds into it and the account panel calls it
//! there instead.

use serde_json::Value;

/// `SessionChatContextMeterUsage`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContextMeterUsage {
    /// 0 to 100, or `None` while the agent has not reported context usage.
    pub used_percentage: Option<f64>,
    pub used_tokens: Option<f64>,
    pub window_size: Option<f64>,
}

/// `resolveSessionChatContextMeterUsage`.
///
/// Claude uses tokens over window size; Codex uses its baseline-adjusted reported percentage.
/// `None` means the agent has not reported enough data to draw usage.
pub fn context_meter_usage(
    usage: Option<&Value>,
    prefer_reported_percentage: bool,
) -> Option<ContextMeterUsage> {
    let usage = usage?.as_object()?;
    let finite_non_negative = |key: &str| {
        usage
            .get(key)
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value >= 0.0)
    };
    let used_tokens = finite_non_negative("usedTokens");
    let window_size = finite_non_negative("windowSize").filter(|size| *size > 0.0);
    let reported = finite_non_negative("usedPercentage");
    let used_percentage = if prefer_reported_percentage && reported.is_some() {
        reported.map(|value| value.min(100.0))
    } else if let (Some(tokens), Some(size)) = (used_tokens, window_size) {
        Some((tokens / size * 100.0).min(100.0))
    } else {
        reported.map(|value| value.min(100.0))
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
        return format!("{}", crate::menus::accounts_data::js_round(value) as i64);
    }
    if value < 10_000.0 {
        return format!("{}k", strip_trailing_zero(value / 1_000.0));
    }
    if value < 1_000_000.0 {
        return format!(
            "{}k",
            crate::menus::accounts_data::js_round(value / 1_000.0) as i64
        );
    }
    format!("{}m", strip_trailing_zero(value / 1_000_000.0))
}

/// `value.toFixed(1).replace(/\.0$/, '')`.
fn strip_trailing_zero(value: f64) -> String {
    let fixed = format!("{value:.1}");
    match fixed.strip_suffix(".0") {
        Some(head) => head.to_string(),
        None => fixed,
    }
}
