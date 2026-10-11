//! The JSON a phone host sends for one turn's [`ChatContext`].
//!
//! `ChatContext`'s own serde form carries its two random ids as `u128`, which a JavaScript number
//! cannot hold, so the boundary takes them as UUID strings. When the host leaves the random draws
//! out, they are drawn here from the OS source, the same source the desktop host uses
//! (`context()` in `apps/desktop/src/app/gx_chat/worker.rs`); Hermes has no `crypto.randomUUID`.

use ghostex_gx_chat_core::{ChatContext, FormattedTime};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContextJson {
    now_ms: f64,
    #[serde(default)]
    utc_offset_minutes: i32,
    #[serde(default)]
    random_units: Option<[f64; 2]>,
    #[serde(default)]
    random_ids: Option<[String; 2]>,
    #[serde(default)]
    formatted_times: Vec<FormattedTime>,
    #[serde(default)]
    clock_reads: Vec<f64>,
}

pub(crate) fn parse(json: &str) -> Result<ChatContext, String> {
    let parsed: ContextJson = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let random_units = match parsed.random_units {
        Some(units) => units,
        None => {
            let bytes = entropy::<16>()?;
            [unit(&bytes[0..7]), unit(&bytes[8..15])]
        }
    };
    let random_ids = match parsed.random_ids {
        Some([first, second]) => [uuid_bits(&first)?, uuid_bits(&second)?],
        None => [
            u128::from_be_bytes(entropy::<16>()?),
            u128::from_be_bytes(entropy::<16>()?),
        ],
    };
    Ok(ChatContext::at(parsed.now_ms)
        .with_utc_offset_minutes(parsed.utc_offset_minutes)
        .with_random_units(random_units)
        .with_random_ids(random_ids)
        .with_formatted_times(parsed.formatted_times)
        .with_clock_reads(parsed.clock_reads))
}

fn entropy<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|error| format!("OS random source: {error}"))?;
    Ok(bytes)
}

/// 56 random bits as a uniform draw in `[0, 1)`, the desktop host's construction.
fn unit(bytes: &[u8]) -> f64 {
    let value = bytes
        .iter()
        .fold(0u64, |value, byte| (value << 8) | u64::from(*byte));
    value as f64 / (1u64 << 56) as f64
}

/// A UUID (hyphens optional) as the 128-bit number `ChatContext::random_id` prints back.
fn uuid_bits(text: &str) -> Result<u128, String> {
    let hex: String = text.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return Err(format!("randomIds entry is not a UUID: {text:?}"));
    }
    u128::from_str_radix(&hex, 16).map_err(|_| format!("randomIds entry is not a UUID: {text:?}"))
}
