//! The Terminal View hover preview's formatter, ported from
//! `packages/shared/session-chat-presentation/terminal-tail.ts`.
//!
//! CDXC:SessionChat 2026-09-05 DECISION:
//! User: the agent CLI's horizontal rules must be truncated in the Terminal View hover preview
//! instead of being copied verbatim. A zmx session is 200 columns wide, so a rule row carried
//! nothing but width: it pinned the tooltip to its max width and each rule wrapped onto a second
//! line, framing the real content in four ragged bars. Supersedes the earlier "keep full-width
//! rules verbatim" rule of this formatter; indentation, blank rows, and text content are still
//! verbatim.

/// The characters a rule is drawn from, matched in runs of eight or more.
const RULE_CHARACTERS: &[char] = &[
    '\u{2500}', // ─
    '\u{2501}', // ━
    '\u{2550}', // ═
    '\u{254c}', // ╌
    '\u{254d}', // ╍
    '\u{2504}', // ┄
    '\u{2505}', // ┅
    '\u{2508}', // ┈
    '\u{2509}', // ┉
    '\u{2594}', // ▔
    '\u{203e}', // ‾
    '\u{23af}', // ⎯
    '\u{2013}', // –
    '\u{2014}', // —
    '_', '-',
];

/// `{8,}`: the shortest run the rule pattern matches.
const RULE_RUN_MINIMUM: usize = 8;
/// Narrow enough to stay a frame, wide enough to still read as a rule.
const MIN_RULE_COLUMNS: usize = 24;
const MAX_RULE_COLUMNS: usize = 72;

/// `formatSessionTerminalTailPreview`: the captured rows with every long rule run cut down.
pub fn format_terminal_tail_preview(lines: &[String]) -> String {
    let columns = rule_columns_for(lines);
    lines
        .iter()
        .map(|line| truncate_rule_runs(line, columns))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `ruleColumnsFor`: rules are shortened to the widest row that is not itself a rule, so the box
/// still frames the content it was drawn around instead of dictating the tooltip's width.
fn rule_columns_for(lines: &[String]) -> usize {
    let mut widest = 0;
    for line in lines {
        if has_rule_run(line) {
            continue;
        }
        widest = widest.max(preview_columns(line));
    }
    MAX_RULE_COLUMNS.min(MIN_RULE_COLUMNS.max(widest))
}

/// `previewColumns`: `Array.from(line.trimEnd()).length`, which counts code points, not bytes.
fn preview_columns(line: &str) -> usize {
    line.trim_end_matches(|character: char| character.is_whitespace() || character == '\u{feff}')
        .chars()
        .count()
}

/// Whether the line carries at least one run the pattern matches.
fn has_rule_run(line: &str) -> bool {
    rule_runs(line).next().is_some()
}

/// `line.replace(RULE_RUNS, run => ...)`: every matching run cut to `columns` code points.
fn truncate_rule_runs(line: &str, columns: usize) -> String {
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0;
    for (start, end) in rule_runs(line) {
        out.push_str(&line[cursor..start]);
        let run = &line[start..end];
        let characters: Vec<char> = run.chars().collect();
        if characters.len() <= columns {
            out.push_str(run);
        } else {
            out.extend(characters.into_iter().take(columns));
        }
        cursor = end;
    }
    out.push_str(&line[cursor..]);
    out
}

/// The byte ranges of every maximal run of rule characters at least [`RULE_RUN_MINIMUM`] long,
/// which is what the greedy `{8,}` regex matches.
fn rule_runs(line: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    let mut indices = line.char_indices().peekable();
    std::iter::from_fn(move || {
        while let Some((at, character)) = indices.next() {
            if !RULE_CHARACTERS.contains(&character) {
                continue;
            }
            let mut end = at + character.len_utf8();
            let mut count = 1;
            while let Some((next_at, next)) = indices.peek() {
                if !RULE_CHARACTERS.contains(next) {
                    break;
                }
                end = next_at + next.len_utf8();
                count += 1;
                indices.next();
            }
            if count >= RULE_RUN_MINIMUM {
                return Some((at, end));
            }
        }
        None
    })
}
