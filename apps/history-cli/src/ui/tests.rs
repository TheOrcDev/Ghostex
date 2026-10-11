use super::*;
use crate::model::Agent;
use crate::model::TranscriptBlock;
use crate::model::TranscriptKind;
use ratatui::style::Modifier;
use ratatui::text::Line;

fn block(kind: TranscriptKind, text: &str) -> TranscriptBlock {
    TranscriptBlock::new(kind, text, None).expect("test transcript block")
}

fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn transcript_layout_indexes_user_blocks_and_uses_codex_prompt_prefix() {
    let layout = transcript_layout(
        &[
            block(TranscriptKind::User, "first prompt"),
            block(TranscriptKind::Agent, "first answer"),
            block(TranscriptKind::User, "second prompt"),
        ],
        80,
        None,
    );

    assert_eq!(layout.block_starts, vec![0, 4, 6]);
    assert_eq!(layout.user_blocks, vec![0, 2]);
    assert_eq!(line_text(&layout.lines[1]), "› first prompt");
    assert_eq!(line_text(&layout.lines[7]), "› second prompt");
}

#[test]
fn codex_message_jump_helpers_select_latest_then_pin_at_edges() {
    let user_blocks = [0, 2, 5];

    assert_eq!(previous_user_block(&user_blocks, None), Some(5));
    assert_eq!(previous_user_block(&user_blocks, Some(5)), Some(2));
    assert_eq!(previous_user_block(&user_blocks, Some(0)), Some(0));
    assert_eq!(next_user_block(&user_blocks, None), Some(5));
    assert_eq!(next_user_block(&user_blocks, Some(2)), Some(5));
    assert_eq!(next_user_block(&user_blocks, Some(5)), Some(5));
}

#[test]
fn selected_user_transcript_lines_use_codex_reversed_user_cell_style() {
    let lines = user_transcript_lines("selected prompt", 80, true);
    let style = user_message_style(true);

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].style, style);
    assert_eq!(line_text(&lines[1]), "› selected prompt");
    assert_eq!(lines[1].style, style);
    assert_eq!(lines[2].style, style);
    assert!(lines[1].style.add_modifier.contains(Modifier::REVERSED));
    assert!(lines[1].spans[0]
        .style
        .add_modifier
        .contains(Modifier::BOLD | Modifier::DIM));
}

#[test]
fn scroll_delta_matches_mouse_wheel_step_and_clamps() {
    assert_eq!(apply_scroll_delta(20, -3, 100), 17);
    assert_eq!(apply_scroll_delta(20, 3, 100), 23);
    assert_eq!(apply_scroll_delta(1, -3, 100), 0);
    assert_eq!(apply_scroll_delta(99, 3, 100), 100);
    assert_eq!(apply_scroll_delta(140, -3, 100), 100);
}

#[test]
fn resume_argv_matches_zehn_agent_commands() {
    assert_eq!(
        resume_argv(Agent::Codex, "codex-session", false),
        ["codex", "resume", "codex-session"]
    );
    assert_eq!(
        resume_argv(Agent::Codex, "codex-session", true),
        ["codex", "--yolo", "resume", "codex-session"]
    );
    assert_eq!(
        resume_argv(Agent::Claude, "claude-session", true),
        [
            "claude",
            "--dangerously-skip-permissions",
            "--resume",
            "claude-session"
        ]
    );
    assert_eq!(
        resume_argv(Agent::Pi, "pi-session", true),
        ["pi", "--session", "pi-session"]
    );
    assert_eq!(
        resume_argv(Agent::Cursor, "cursor-session", true),
        ["cursor-agent", "--yolo", "--resume", "cursor-session"]
    );
    assert_eq!(
        resume_argv(Agent::Grok, "grok-session", true),
        [
            "grok",
            "--permission-mode",
            "bypassPermissions",
            "--resume",
            "grok-session"
        ]
    );
}

#[test]
fn list_header_title_uses_codex_spaced_style() {
    assert_eq!(
        spaced_header_title("Agent history"),
        "A G E N T  H I S T O R Y"
    );
}
