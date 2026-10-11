//! Shortcut labels for the chords the catalog stores, in each platform's convention.
//!
//! CDXC:Hotkeys 2026-09-16 DECISION:
//! User: all app shortcut labels detect the OS, using compact macOS glyphs (⌥Enter) and Windows/Linux names (Alt+Enter), never combined Option/Alt labels.
//!
//! CDXC:Hotkeys 2026-09-27 DECISION:
//! User: named keys read like the menus, `Esc` for escape and a capitalized first letter for the rest (`Space`, `Backspace`), instead of the stored lowercase name.
//! SEE-ALSO: apps/desktop/src/hotkey_label.rs and packages/gx-core/src/quick_access/text.rs format the same labels for chords the user records.

use crate::Platform;

/// The label of a stored chord sequence (`cmd+shift+o`, `ctrl+k ctrl+s`): `⌘⇧O` on macOS, `Ctrl+Shift+O` elsewhere.
/// The catalog's chords are already in the stored lowercase `+` form, so no normalization is needed.
pub fn hotkey_label(hotkey: &str, platform: Platform) -> String {
    hotkey
        .split(' ')
        .map(|chord| chord_label(chord, platform))
        .collect::<Vec<_>>()
        .join(" ")
}

fn chord_label(chord: &str, platform: Platform) -> String {
    let parts: Vec<&str> = chord.split('+').collect();
    let has_primary = parts.contains(&"cmd");
    let separator = if platform == Platform::MacOs { "" } else { "+" };
    let mut labels: Vec<String> = Vec::with_capacity(parts.len());
    for part in parts {
        let label = part_label(part, platform, has_primary);
        if labels.last() != Some(&label) {
            labels.push(label);
        }
    }
    labels.join(separator)
}

fn part_label(part: &str, platform: Platform, has_primary: bool) -> String {
    if platform != Platform::MacOs {
        match part {
            "cmd" => return "Ctrl".to_string(),
            "ctrl" => return if has_primary { "Alt" } else { "Ctrl" }.to_string(),
            "alt" => return "Alt".to_string(),
            "shift" => return "Shift".to_string(),
            _ => {}
        }
    }
    match part {
        "cmd" => "⌘".to_string(),
        "ctrl" => "⌃".to_string(),
        "alt" => "⌥".to_string(),
        "shift" => "⇧".to_string(),
        "up" => "↑".to_string(),
        "right" => "→".to_string(),
        "down" => "↓".to_string(),
        "left" => "←".to_string(),
        "tab" => "Tab".to_string(),
        "enter" => "Enter".to_string(),
        "escape" => "Esc".to_string(),
        _ => {
            let is_function_key = part.len() >= 2
                && part.starts_with('f')
                && part[1..].bytes().all(|byte| byte.is_ascii_digit());
            if is_function_key || part.chars().count() == 1 {
                part.to_uppercase()
            } else {
                let mut characters = part.chars();
                characters
                    .next()
                    .map(|first| first.to_uppercase().chain(characters).collect())
                    .unwrap_or_default()
            }
        }
    }
}
