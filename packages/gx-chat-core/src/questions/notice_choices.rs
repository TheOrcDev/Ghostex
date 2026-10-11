//! How a notice's picker rows read before the user expands the card.
//!
//! Port of `packages/shared/session-chat-presentation/notice-choices.ts`.

/// Options a collapsed picker shows before the user expands it.
pub const COLLAPSED_CHOICE_COUNT: u32 = 2;

/// The long form a rate-limit continuation row opens with, shortened in the collapsed layout.
const CONTINUE_PREFIX: &str = "Wait here, then continue automatically";
const CONTINUE_SHORT: &str = "Continue automatically";

/// The collapsed label for one picker row.
///
/// CDXC:SessionChat 2026-09-18 DECISION:
/// User: remove "(y)", "(default)" and "(n)" from the collapsed Yes/No buttons. Terminal key hints
/// like "(y)" are single characters, so any trailing one-character hint is dropped along with
/// "(recommended)" and "(default)", repeatedly so "No (default) (n)" reads "No".
pub fn collapsed_choice_label(label: &str) -> String {
    // CDXC:SessionChat 2026-09-07 WHY: Rate-limit continuation labels wrapped in the compact
    // two-button layout. Shorten their shared prefix while preserving the timing, whether an
    // explicit reset time or "shortly"; expanding still shows the terminal's full wording.
    let mut trimmed = shorten_continue_prefix(label.trim());
    loop {
        let next = strip_label_suffix(&trimmed);
        if next.is_empty() || next == trimmed {
            break;
        }
        trimmed = next.trim_end().to_string();
    }
    trimmed
}

/// `label.replace(/^Wait here, then continue automatically\b/i, 'Continue automatically')`.
fn shorten_continue_prefix(label: &str) -> String {
    if label.len() < CONTINUE_PREFIX.len() || !label.is_char_boundary(CONTINUE_PREFIX.len()) {
        return label.to_string();
    }
    let (head, rest) = label.split_at(CONTINUE_PREFIX.len());
    if !head.eq_ignore_ascii_case(CONTINUE_PREFIX) {
        return label.to_string();
    }
    // The `\b` after the prefix: the next character must not continue a word.
    if rest
        .chars()
        .next()
        .is_some_and(|character| character.is_alphanumeric() || character == '_')
    {
        return label.to_string();
    }
    format!("{CONTINUE_SHORT}{rest}")
}

/// One application of `/\s*\((?:recommended|default|[a-z0-9])\)$/i`.
///
/// Returns the input unchanged when nothing matched, which is the loop's stop condition.
fn strip_label_suffix(label: &str) -> String {
    let Some(body) = label.strip_suffix(')') else {
        return label.to_string();
    };
    let Some(open) = body.rfind('(') else {
        return label.to_string();
    };
    let inner = &body[open + 1..];
    let recognized =
        inner.eq_ignore_ascii_case("recommended") || inner.eq_ignore_ascii_case("default") || {
            let mut characters = inner.chars();
            match (characters.next(), characters.next()) {
                (Some(only), None) => only.is_ascii_alphanumeric(),
                _ => false,
            }
        };
    if !recognized {
        return label.to_string();
    }
    body[..open].trim_end().to_string()
}
