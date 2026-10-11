//! `HotkeyRecorderField` (packages/core-ui/hotkey-recorder-field.tsx (deleted 2026-10-01)): an outline button showing a
//! shortcut the way the app's menus label it, which records the next chord pressed while it is
//! armed, with Reset and Remove chips that show while the field is hovered. Also the hotkey text
//! helpers of packages/shared/ghostex-hotkeys.ts (deleted 2026-10-01) the recorder and the Hotkeys page need
//! (`normalizeHotkeyText`, `ghostexHotkeyTextFromKeyboardEvent`, `isReservedghostexHotkeyText`).
//!
//! A page embeds a [`HotkeyRecorder`] and implements [`HotkeyRecorderHost`]; while a field is
//! armed, keystrokes are taken before the window's bindings see them, as the React recorder took
//! them in the capture phase on `document`.
use super::super::super::native_modal_kit::*;
use super::super::palette::SettingsPalette;
use super::SettingsPage;
use super::row::{CONTROL_HEIGHT, settings_icon, tooltip_text};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClickEvent, Context, ElementId, InteractiveElement as _, IntoElement,
    Keystroke, ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _,
    Subscription, Window, div, px,
};
use gpui_component::h_flex;

const ICON_X: &str = "modals/settings/x.svg";

/// `tracking-widest`: 0.1em.
const CHORD_TRACKING_EM: f32 = 0.1;

/// The platform the stored `cmd` means (`detectghostexHotkeyPlatform`).
fn is_mac() -> bool {
    cfg!(target_os = "macos")
}

/// `normalizeHotkeyText`: lower-case, glyphs and long names to `cmd`/`alt`/`ctrl`/`shift`, one
/// space between chords, and the legacy recorder spellings (`alt+ß`, shifted digits and braces)
/// back to their physical keys.
pub(crate) fn normalize_hotkey_text(value: &str) -> String {
    let mut text = value.trim().to_lowercase();
    for (from, to) in [
        ("⌘", "cmd"),
        ("command", "cmd"),
        ("⌥", "alt"),
        ("option", "alt"),
        ("⌃", "ctrl"),
        ("control", "ctrl"),
        ("⇧", "shift"),
    ] {
        text = text.replace(from, to);
    }
    let words: Vec<String> = text
        .split(|character: char| character.is_whitespace())
        .filter(|chord| !chord.is_empty())
        .map(|chord| {
            // `\bmod\b` -> `cmd`.
            chord
                .split('+')
                .map(|part| if part == "mod" { "cmd" } else { part })
                .collect::<Vec<_>>()
                .join("+")
        })
        .collect();
    if words.is_empty() {
        return String::new();
    }
    words
        .iter()
        .map(|chord| normalize_chord(chord))
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_chord(chord: &str) -> String {
    let mut parts: Vec<String> = chord
        .split('+')
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect();
    let Some(key) = parts.last().cloned() else {
        return chord.to_string();
    };
    let has = |parts: &[String], name: &str| parts.iter().any(|part| part == name);
    let last = parts.len() - 1;
    if has(&parts, "alt") && key == "ß" {
        parts[last] = "s".to_string();
    }
    if has(&parts, "shift") {
        let shifted = match key.as_str() {
            "!" => Some("1"),
            "@" => Some("2"),
            "#" => Some("3"),
            "$" => Some("4"),
            "%" => Some("5"),
            "^" => Some("6"),
            "&" => Some("7"),
            "*" => Some("8"),
            "(" => Some("9"),
            ")" => Some("0"),
            "{" => Some("["),
            "}" => Some("]"),
            _ => None,
        };
        if let Some(physical) = shifted {
            parts[last] = physical.to_string();
        }
    }
    parts.join("+")
}

/// `isReservedghostexHotkeyText`: Cmd+K belongs to the focused terminal on macOS
/// (CDXC:Hotkeys 2026-08-22 in packages/shared/ghostex-hotkeys.ts (deleted 2026-10-01)), judged by the opening chord.
pub(crate) fn is_reserved_hotkey(text: &str) -> bool {
    let normalized = normalize_hotkey_text(text);
    let opening = normalized.split(' ').next().unwrap_or_default();
    is_mac() && opening == "cmd+k"
}

/// `ghostexHotkeyTextFromKeyboardEvent` for a GPUI keystroke: the primary modifier as `cmd`
/// (Command on macOS, Ctrl elsewhere), macOS Control as `ctrl`, then `alt`, `shift` and the key's
/// layout-independent name. `None` for a lone modifier.
///
/// CDXC:Hotkeys 2026-09-10 WHY:
/// The key's identity is recorded rather than the character it typed (`key_char`): macOS reports Option+S as `ß` and an Arabic layout reports Cmd+V as `ر`, while GPUI dispatches the letter key in both cases.
pub(crate) fn hotkey_text_from_keystroke(keystroke: &Keystroke) -> Option<String> {
    let key = keystroke.key.to_lowercase();
    if key.is_empty()
        || matches!(
            key.as_str(),
            "shift" | "control" | "alt" | "platform" | "function" | "capslock"
        )
    {
        return None;
    }
    let modifiers = keystroke.modifiers;
    let mut parts: Vec<&str> = Vec::new();
    if if is_mac() {
        modifiers.platform
    } else {
        modifiers.control
    } {
        parts.push("cmd");
    }
    if is_mac() && modifiers.control {
        parts.push("ctrl");
    }
    if modifiers.alt {
        parts.push("alt");
    }
    if modifiers.shift {
        parts.push("shift");
    }
    parts.push(key.as_str());
    Some(normalize_hotkey_text(&parts.join("+")))
}

/// `formatSidebarHotkeyLabel`: each chord in the OS convention (`⌘⌥S` on macOS, `Ctrl+Alt+S`
/// elsewhere), chords separated by a space.
pub(crate) fn format_hotkey_label(text: &str) -> String {
    normalize_hotkey_text(text)
        .split(' ')
        .filter(|chord| !chord.is_empty())
        .map(crate::hotkey_label::terminal_overlay_hotkey_chord_label)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Text with `tracking-widest`: GPUI has no letter spacing, so each character is its own box
/// with a tenth of the font size after it.
///
/// CDXC:Hotkeys 2026-09-27 DECISION:
/// User: Settings hotkeys must look like the shortcuts in the app's menus, and an unassigned hotkey shows just `-`. Chords use the UI font with the menu shortcut's wide tracking, not the monospace font.
fn tracked_text(text: &str, size: f32) -> AnyElement {
    h_flex()
        .min_w_0()
        .overflow_hidden()
        .children(text.chars().map(|character| {
            div()
                .mr(px(size * CHORD_TRACKING_EM))
                .child(character.to_string())
        }))
        .into_any_element()
}

/// The armed recorder of a page.
#[derive(Default)]
pub(crate) struct HotkeyRecorder {
    /// The field listening for a chord.
    recording: Option<SharedString>,
    /// A reserved chord that was pressed while listening (`reservedHotkey`).
    reserved: Option<String>,
    intercept: Option<Subscription>,
}

impl HotkeyRecorder {
    pub(crate) fn recording(&self) -> Option<&SharedString> {
        self.recording.as_ref()
    }

    pub(crate) fn stop(&mut self) {
        self.recording = None;
        self.reserved = None;
        self.intercept = None;
    }
}

/// A page with hotkey recorders.
pub(crate) trait HotkeyRecorderHost: SettingsPage {
    fn hotkey_recorder(&mut self) -> &mut HotkeyRecorder;
    /// A field recorded (or cleared) its hotkey.
    fn hotkey_recorded(&mut self, id: &str, hotkey: String, cx: &mut Context<Self>);
}

/// Arms field `id`: every keystroke in this window goes to it until a chord is recorded, the
/// field is cleared with Backspace or Delete, or Escape stops it.
pub(crate) fn start_recording<V: HotkeyRecorderHost>(
    page: &mut V,
    id: SharedString,
    window: &mut Window,
    cx: &mut Context<V>,
) {
    let window_id = window.window_handle().window_id();
    let view = cx.weak_entity();
    let field = id.clone();
    let intercept = cx.intercept_keystrokes(move |event, window, cx: &mut App| {
        if window.window_handle().window_id() != window_id {
            return;
        }
        let keystroke = event.keystroke.clone();
        let field = field.clone();
        let handled = view
            .update(cx, |page, cx| {
                handle_recorded_key(page, &field, &keystroke, cx)
            })
            .unwrap_or(false);
        if handled {
            cx.stop_propagation();
        }
    });
    let recorder = page.hotkey_recorder();
    recorder.recording = Some(id);
    recorder.reserved = None;
    recorder.intercept = Some(intercept);
    cx.notify();
}

fn handle_recorded_key<V: HotkeyRecorderHost>(
    page: &mut V,
    field: &SharedString,
    keystroke: &Keystroke,
    cx: &mut Context<V>,
) -> bool {
    if page.hotkey_recorder().recording.as_ref() != Some(field) {
        return false;
    }
    let key = keystroke.key.to_lowercase();
    let modifiers = keystroke.modifiers;
    if key == "escape" {
        page.hotkey_recorder().stop();
        cx.notify();
        return true;
    }
    if (key == "backspace" || key == "delete")
        && !modifiers.alt
        && !modifiers.control
        && !modifiers.platform
        && !modifiers.shift
    {
        page.hotkey_recorder().stop();
        page.hotkey_recorded(field, String::new(), cx);
        cx.notify();
        return true;
    }
    let Some(recorded) = hotkey_text_from_keystroke(keystroke) else {
        // A lone modifier: keep listening, and keep it from the rest of the window.
        return true;
    };
    if is_reserved_hotkey(&recorded) {
        page.hotkey_recorder().reserved = Some(recorded);
        cx.notify();
        return true;
    }
    page.hotkey_recorder().stop();
    page.hotkey_recorded(field, recorded, cx);
    cx.notify();
    true
}

/// A 24px chip over the field's right end (Reset / Remove): `border-border bg-background/95`,
/// muted, `hover:bg-muted hover:text-foreground`.
fn chip(p: &SettingsPalette, id: ElementId) -> gpui::Stateful<gpui::Div> {
    let hover_bg = if p.light {
        gpui::rgb(0xf1f1f1)
    } else {
        gpui::rgb(0x262626)
    };
    let hover_text = p.foreground;
    div()
        .id(id)
        .flex_shrink_0()
        .h(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(hsla(p.hairline))
        .bg(hsla(css_fade(p.surface, 0.95)))
        .text_size(px(12.0))
        .line_height(px(16.0))
        .text_color(hsla(p.muted))
        .cursor_pointer()
        .hover(move |this| this.bg(hsla(hover_bg)).text_color(hsla(hover_text)))
}

/// The recorder. `hotkey` is the saved value, `original` the action's default, `invalid` marks a
/// chord another action also uses.
///
/// CDXC:Hotkeys 2026-05-11 WHY:
/// The Reset and Remove affordances are real buttons inside the field, revealed only while the field is hovered (or listening), so hotkey rows stay quiet until the user targets a specific binding.
#[allow(clippy::too_many_arguments)]
pub(crate) fn hotkey_recorder_field<V: HotkeyRecorderHost>(
    page: &mut V,
    p: &SettingsPalette,
    id: impl Into<SharedString>,
    hotkey: &str,
    original: &str,
    invalid: bool,
    cx: &mut Context<V>,
) -> AnyElement {
    let id: SharedString = id.into();
    let recorder = page.hotkey_recorder();
    let recording = recorder.recording.as_ref() == Some(&id);
    let reserved = recording.then(|| recorder.reserved.clone()).flatten();
    let normalized = normalize_hotkey_text(hotkey);
    let normalized_original = normalize_hotkey_text(original);
    let original_label = format_hotkey_label(&normalized_original);
    let original_description = if original_label.is_empty() {
        "Unassigned".to_string()
    } else {
        original_label.clone()
    };
    let modified = normalized != normalized_original;
    let label = if recording {
        match &reserved {
            Some(reserved) => format!("{} is reserved", format_hotkey_label(reserved)),
            None => "Press Shortcut".to_string(),
        }
    } else {
        format_hotkey_label(&normalized)
    };
    let shows_chord = !recording && !label.is_empty();
    let group: SharedString = format!("hotkey-recorder-{id}").into();
    let (hover_bg, border) = (
        if p.light {
            gpui::rgb(0xf1f1f1)
        } else {
            css_fade(p.hairline, 0.3)
        },
        if invalid {
            css_fade(p.destructive, if p.light { 1.0 } else { 0.5 })
        } else {
            p.hairline
        },
    );
    let ring = hsla(css_fade(p.destructive, if p.light { 0.2 } else { 0.4 }));
    let toggle_id = id.clone();
    let button = div()
        .id(ElementId::Name(format!("hotkey-{id}").into()))
        .w_full()
        .h(px(CONTROL_HEIGHT))
        .pl(px(12.0))
        .pr(px(36.0))
        .flex()
        .items_center()
        .overflow_hidden()
        .rounded(px(MODAL_RADIUS_CONTROL))
        .border_1()
        .border_color(hsla(border))
        .bg(hsla(if p.light {
            p.surface
        } else {
            gpui::rgba(0x00000000)
        }))
        .text_size(px(13.0))
        .line_height(px(20.0))
        .text_color(hsla(p.foreground))
        .cursor_pointer()
        .hover(move |this| this.bg(hsla(hover_bg)))
        .on_click(
            cx.listener(move |page: &mut V, _: &ClickEvent, window, cx| {
                let recording = page.hotkey_recorder().recording.as_ref() == Some(&toggle_id);
                if recording {
                    page.hotkey_recorder().stop();
                    cx.notify();
                } else {
                    start_recording(page, toggle_id.clone(), window, cx);
                }
            }),
        )
        .child(if shows_chord {
            tracked_text(&label, 13.0)
        } else {
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(if label.is_empty() {
                    "-".to_string()
                } else {
                    label
                })
                .into_any_element()
        });
    let mut chips: Vec<AnyElement> = Vec::new();
    if modified {
        let reset_id = id.clone();
        let original_for_reset = normalized_original.clone();
        let reset_label = if original_label.is_empty() {
            "-".to_string()
        } else {
            original_label.clone()
        };
        chips.push(
            chip(p, ElementId::Name(format!("hotkey-{id}-reset").into()))
                .px(px(8.0))
                .tooltip(tooltip_text(format!("Reset to {original_description}")))
                .on_click(
                    cx.listener(move |page: &mut V, _: &ClickEvent, _window, cx| {
                        cx.stop_propagation();
                        page.hotkey_recorder().stop();
                        page.hotkey_recorded(&reset_id, original_for_reset.clone(), cx);
                        cx.notify();
                    }),
                )
                .child(if normalized_original.is_empty() {
                    div().child(reset_label).into_any_element()
                } else {
                    tracked_text(&reset_label, 12.0)
                })
                .into_any_element(),
        );
    }
    if !normalized.is_empty() {
        let remove_id = id.clone();
        chips.push(
            chip(p, ElementId::Name(format!("hotkey-{id}-remove").into()))
                .w(px(24.0))
                .tooltip(tooltip_text("Remove hotkey"))
                .on_click(
                    cx.listener(move |page: &mut V, _: &ClickEvent, _window, cx| {
                        cx.stop_propagation();
                        page.hotkey_recorder().stop();
                        page.hotkey_recorded(&remove_id, String::new(), cx);
                        cx.notify();
                    }),
                )
                .child(settings_icon(ICON_X, 16.0, p.muted))
                .into_any_element(),
        );
    }
    div()
        .id(ElementId::Name(group.clone()))
        .group(group.clone())
        .relative()
        .w_full()
        .when(invalid, |this| {
            // `aria-invalid:ring-3`: a 3px ring outside the border (a GPUI shadow would also fill
            // under the transparent button).
            this.child(
                div()
                    .absolute()
                    .left(px(-3.0))
                    .top(px(-3.0))
                    .right(px(-3.0))
                    .bottom(px(-3.0))
                    .rounded(px(MODAL_RADIUS_CONTROL + 3.0))
                    .border_3()
                    .border_color(ring),
            )
        })
        .child(button)
        .when(!chips.is_empty(), |this| {
            this.child(
                h_flex()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right(px(6.0))
                    .items_center()
                    .gap(px(4.0))
                    .when(!recording, |this| {
                        this.opacity(0.0)
                            .group_hover(group, |this| this.opacity(1.0))
                    })
                    .children(chips),
            )
        })
        .into_any_element()
}
