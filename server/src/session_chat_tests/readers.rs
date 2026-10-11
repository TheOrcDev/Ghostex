use super::*;

#[test]
fn stream_frames_are_published_in_seq_order() {
    use std::sync::Mutex;
    let stream = Arc::new(SessionChatStream::new());
    stream.begin_generation();
    let published: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    for _ in 0..8 {
        let stream = stream.clone();
        let published = published.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..50 {
                stream.emit_sequenced(
                    |seq| json!(seq),
                    |frame| {
                        published
                            .lock()
                            .expect("publish lock")
                            .push(frame.as_i64().expect("seq"));
                    },
                );
            }
        }));
    }
    for handle in handles {
        handle.join().expect("thread");
    }
    let published = published.lock().expect("publish lock").clone();
    assert_eq!(published.len(), 400);
    assert!(
        published.windows(2).all(|pair| pair[0] < pair[1]),
        "frames must reach the hub in seq order",
    );
}

pub(super) fn write_temp_transcript(lines: &[&str]) -> PathBuf {
    // Tests run in parallel: the name must be unique per CALL, not per
    // line count, or two tests share (and then delete) one file.
    static NEXT_TEMP_TRANSCRIPT: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "gxserver-session-chat-test-{}-{}.jsonl",
        std::process::id(),
        NEXT_TEMP_TRANSCRIPT.fetch_add(1, Ordering::SeqCst),
    ));
    let mut file = File::create(&path).expect("create temp transcript");
    for line in lines {
        writeln!(file, "{line}").expect("write line");
    }
    path
}

#[test]
fn tail_reader_limit_and_has_more_semantics() {
    let lines: Vec<String> = (0..10)
        .map(|index| {
            format!(
                r#"{{"type":"user","uuid":"u{index}","message":{{"role":"user","content":"prompt {index}"}}}}"#
            )
        })
        .collect();
    let line_refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let path = write_temp_transcript(&line_refs);

    let all = read_session_chat_transcript_tail_file(
        &path,
        300,
        decode_claude_transcript_line,
        true,
        None,
        Some(decode_claude_turn_lifecycle),
        Some(claude_transcript_lineage),
    )
    .expect("tail read");
    assert_eq!(all.messages.len(), 10);
    assert!(!all.has_more);
    assert_eq!(all.before_offset, 0);
    assert_eq!(
        all.lifecycle.as_ref().map(|lifecycle| lifecycle.state),
        Some(SessionChatTurnLifecycleState::Working)
    );

    let limited = read_session_chat_transcript_tail_file(
        &path,
        3,
        decode_claude_transcript_line,
        true,
        None,
        None,
        Some(claude_transcript_lineage),
    )
    .expect("limited read");
    assert_eq!(limited.messages.len(), 3);
    assert!(limited.has_more);
    assert_eq!(
        limited.messages.last().map(|message| message.id.as_str()),
        Some("u9")
    );

    // limit 0 ⇒ [] (never "everything").
    let zero = read_session_chat_transcript_tail_file(
        &path,
        0,
        decode_claude_transcript_line,
        true,
        None,
        None,
        Some(claude_transcript_lineage),
    )
    .expect("zero read");
    assert!(zero.messages.is_empty());
    assert!(!zero.has_more);

    // Paging by beforeOffset excludes the newer window.
    let older = read_session_chat_transcript_tail_file(
        &path,
        300,
        decode_claude_transcript_line,
        true,
        Some(limited.before_offset),
        None,
        Some(claude_transcript_lineage),
    )
    .expect("older page");
    assert_eq!(older.messages.len(), 7);
    assert_eq!(
        older.messages.last().map(|message| message.id.as_str()),
        Some("u6")
    );

    let _ = fs::remove_file(&path);
}

#[test]
fn incremental_reader_resumes_from_offset() {
    let path = write_temp_transcript(&[
        r#"{"type":"user","uuid":"u1","message":{"role":"user","content":"one"}}"#,
    ]);
    let mut state = SessionChatIncrementalState::new();
    let first = read_incremental_transcript_messages(
        &path,
        &mut state,
        decode_claude_transcript_line,
        None,
        None,
        None,
        Some(claude_transcript_lineage),
    )
    .expect("first pass");
    assert_eq!(first.len(), 1);
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("append");
    writeln!(
        file,
        r#"{{"type":"assistant","uuid":"a1","message":{{"role":"assistant","content":[{{"type":"text","text":"two"}}]}}}}"#
    )
    .expect("append line");
    drop(file);
    let second = read_incremental_transcript_messages(
        &path,
        &mut state,
        decode_claude_transcript_line,
        None,
        None,
        None,
        Some(claude_transcript_lineage),
    )
    .expect("second pass");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].id, "a1");
    let _ = fs::remove_file(&path);
}
