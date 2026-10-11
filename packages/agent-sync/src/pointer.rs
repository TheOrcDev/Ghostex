//! Instruction entry files: the one-line pointer to `~/.agents/main.md` and how
//! an existing file is classified.
//!
//! CDXC:AgentSync 2026-09-16 WHY:
//! Entry files are written, never symlinked. Each agent has its own filename and some
//! tools refuse or de-duplicate a linked instruction file, and Cursor needs YAML frontmatter
//! around the sentence. The pointer keeps the real text in one place (`~/.agents/main.md`).

use crate::catalog::{InstructionKind, POINTER_LINE};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstructionState {
    /// No file.
    Missing,
    /// Exactly the pointer (plus the frontmatter wrapper where required).
    Pointer,
    /// Mentions `~/.agents/main.md` with older wording; rewritten with a backup.
    LegacyPointer,
    /// Real content of its own.
    OtherContent,
    /// A symlink; left alone and reported.
    Symlink,
    /// A folder or something else unexpected at the path.
    NotAFile,
}

/// The exact file content Agent Sync writes.
pub fn pointer_content(kind: InstructionKind) -> String {
    match kind {
        InstructionKind::Plain => format!("{POINTER_LINE}\n"),
        InstructionKind::Mdc => format!(
            "---\ndescription: Shared global agent instructions\nalwaysApply: true\n---\n\n{POINTER_LINE}\n"
        ),
    }
}

pub fn classify(path: &Path, kind: InstructionKind) -> InstructionState {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return InstructionState::Missing;
    };
    if metadata.file_type().is_symlink() {
        return InstructionState::Symlink;
    }
    if !metadata.is_file() {
        return InstructionState::NotAFile;
    }
    let Ok(content) = fs::read_to_string(path) else {
        return InstructionState::OtherContent;
    };
    classify_content(&content, kind)
}

pub fn classify_content(content: &str, kind: InstructionKind) -> InstructionState {
    let normalized = normalize_lines(content);
    if normalized == normalize_lines(&pointer_content(kind)) {
        return InstructionState::Pointer;
    }
    if kind == InstructionKind::Plain
        && normalized.lines().count() == 1
        && normalized.contains("~/.agents/main.md")
    {
        return InstructionState::LegacyPointer;
    }
    if content.contains("~/.agents/main.md") && normalized.lines().count() <= 6 {
        return InstructionState::LegacyPointer;
    }
    InstructionState::OtherContent
}

fn normalize_lines(content: &str) -> String {
    content
        .replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
