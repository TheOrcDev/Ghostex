//! Times the phone's boot path through the JSON boundary: create, start, boot read, subscribe,
//! one snapshot frame, then the first drain.
//!
//! Usage:
//! - `cargo run --release --example timing` uses a synthetic snapshot (`--messages N`, default 300).
//! - `cargo run --release --example timing -- --recording <file.jsonl>` feeds the largest snapshot
//!   frame a chat recording holds instead. Recordings are private: only sizes and timings are
//!   printed, never a value from the file.

use std::time::Instant;

use gx_chat_mobile::MobileChatCore;
use serde_json::{json, Value};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|at| args.get(at + 1))
            .cloned()
    };
    let (label, snapshot) = match flag("--recording") {
        Some(path) => ("recording".to_string(), largest_snapshot(&path)),
        None => {
            let count = flag("--messages")
                .and_then(|text| text.parse().ok())
                .unwrap_or(300);
            (
                format!("synthetic {count} messages"),
                synthetic_snapshot(count),
            )
        }
    };
    let snapshot_json = snapshot.to_string();
    println!(
        "snapshot: {label}, {} bytes of frame JSON",
        snapshot_json.len()
    );

    for run in 0..3 {
        let context =
            || json!({ "nowMs": 1_790_000_000_000.0_f64, "utcOffsetMinutes": 0 }).to_string();
        let started = Instant::now();
        let core = MobileChatCore::new();
        let created = started.elapsed();

        let effects = core.handle(
            json!({ "type": "start", "config": {
                "clientId": "timing-client", "projectId": "p", "sessionId": "s",
                "retainedKey": "[\"local\",\"p\",\"s\"]"
            }})
            .to_string(),
            context(),
        );
        let boot_id = serde_json::from_str::<Vec<Value>>(&effects)
            .unwrap_or_default()
            .iter()
            .find(|effect| effect["type"] == "readComposerBoot")
            .and_then(|effect| effect["requestId"].as_u64());
        core.handle(
            json!({ "type": "composerBootRead", "read": {
                "sessionKey": "p:s", "clientId": "timing-client", "summaryMode": false
            }})
            .to_string(),
            context(),
        );
        core.handle(
            json!({ "type": "connection", "update": "subscribed" }).to_string(),
            context(),
        );
        let booted = started.elapsed();

        let event = format!(r#"{{"type":"frame","frame":{snapshot_json}}}"#);
        let before_handle = Instant::now();
        let effects = core.handle(event, context());
        let handled = before_handle.elapsed();

        let before_frame = Instant::now();
        let frame = core.frame_at(-1, 1_790_000_000_000.0);
        let framed = before_frame.elapsed();

        let parsed: Value = serde_json::from_str(&frame).unwrap_or(Value::Null);
        println!(
            "run {run}: create {:?}, boot {:?} (boot request id {:?}), handle snapshot {:?} ({} effects bytes), frame {:?} ({} bytes, revision {}, {} items, status {})",
            created,
            booted,
            boot_id,
            handled,
            effects.len(),
            framed,
            frame.len(),
            parsed["revision"],
            parsed["itemsSplice"]["items"].as_array().map_or(0, Vec::len),
            parsed["snapshot"]["status"],
        );
    }
}

/// A plausible long chat: turns of a prompt, a markdown answer with a code block, and a tool call
/// with its result.
fn synthetic_snapshot(count: usize) -> Value {
    let mut messages = Vec::new();
    let base = 1_789_990_000_000_i64;
    for index in 0..count {
        let turn = index / 4;
        let stamp = base + index as i64 * 1_000;
        let message = match index % 4 {
            0 => json!({ "id": format!("u{index}"), "role": "user", "source": "transcript",
                "timestamp": stamp, "turnId": format!("t{turn}"), "byteOffset": index * 100,
                "blocks": [{ "type": "text", "text": format!("Please look at step {turn} of the build and fix the failing check.") }] }),
            1 => json!({ "id": format!("c{index}"), "role": "assistant", "source": "transcript",
                "timestamp": stamp, "turnId": format!("t{turn}"), "byteOffset": index * 100,
                "blocks": [{ "type": "tool-call", "name": "Bash", "input": { "command": format!("cargo test -p step{turn}"), "description": "Run the tests" } }] }),
            2 => json!({ "id": format!("r{index}"), "role": "tool", "source": "transcript",
                "timestamp": stamp, "turnId": format!("t{turn}"), "byteOffset": index * 100,
                "blocks": [{ "type": "tool-result", "output": "running 12 tests\ntest result: ok. 12 passed; 0 failed" }] }),
            _ => json!({ "id": format!("a{index}"), "role": "assistant", "source": "transcript",
                "timestamp": stamp, "turnId": format!("t{turn}"), "byteOffset": index * 100,
                "blocks": [{ "type": "text", "text": format!("## Step {turn}\n\nThe check failed because the **fixture** was stale. I updated `src/lib.rs`:\n\n```rust\nfn step() -> u32 {{ {turn} }}\n```\n\n- tests pass\n- lint is clean") }] }),
        };
        messages.push(message);
    }
    json!({
        "type": "sessionChatSnapshot", "projectId": "p", "sessionId": "s", "epoch": 1, "seq": 1,
        "protocolVersion": 1, "serverId": "timing", "messages": messages, "hasMore": false,
        "beforeOffset": 0, "status": "ready", "agent": "claude"
    })
}

/// The largest `sessionChatSnapshot` frame a recording carries, retargeted at this run's ids.
fn largest_snapshot(path: &str) -> Value {
    let text = std::fs::read_to_string(path).expect("recording is readable");
    let mut best: Option<(usize, Value)> = None;
    // Big snapshots cross the desktop broker as numbered chunks of one JSON string.
    let mut transfers: std::collections::BTreeMap<String, std::collections::BTreeMap<u64, String>> =
        Default::default();
    let mut frames = Vec::new();
    for line in text.lines() {
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match record["m"].as_str() {
            Some("event") => frames.push(record["a"][0].clone()),
            Some("brokerMessage") if record["a"][0]["kind"] == "chunk" => {
                let message = &record["a"][0];
                transfers
                    .entry(message["transferId"].to_string())
                    .or_default()
                    .insert(
                        message["index"].as_u64().unwrap_or_default(),
                        message["data"].as_str().unwrap_or_default().to_string(),
                    );
            }
            Some("brokerMessage") => frames.push(record["a"][0]["event"].clone()),
            _ => {}
        }
    }
    for parts in transfers.into_values() {
        let joined: String = parts.into_values().collect();
        if let Ok(message) = serde_json::from_str::<Value>(&joined) {
            frames.push(message["event"].clone());
        }
    }
    for frame in frames {
        if frame["type"] != "sessionChatSnapshot" {
            continue;
        }
        let size = frame.to_string().len();
        if best.as_ref().is_none_or(|(known, _)| size > *known) {
            best = Some((size, frame));
        }
    }
    let (_, mut frame) = best.expect("the recording holds a sessionChatSnapshot frame");
    frame["projectId"] = json!("p");
    frame["sessionId"] = json!("s");
    frame
}
