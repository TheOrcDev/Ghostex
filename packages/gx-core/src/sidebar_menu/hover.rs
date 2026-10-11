//! The session card's hover button strip: which actions it offers and where the chevron splits it.
//!
//! SEE-ALSO: packages/shared/session-card-hover-actions.ts (deleted 2026-10-01), which carries the user decision that
//! defines this strip (whatever sits right of the chevron is always shown, whatever sits left of
//! it is hidden until the chevron is clicked, and an action enabled here is hidden from the row's
//! context menu).

use serde_json::Value;

/// The nine actions the strip can offer, in default order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverAction {
    Rename,
    Pin,
    Note,
    Snooze,
    CloseAfterDone,
    Tag,
    Park,
    Sleep,
    Close,
}

impl HoverAction {
    pub fn as_str(self) -> &'static str {
        match self {
            HoverAction::Rename => "rename",
            HoverAction::Pin => "pin",
            HoverAction::Note => "note",
            HoverAction::Snooze => "snooze",
            HoverAction::CloseAfterDone => "closeAfterDone",
            HoverAction::Tag => "tag",
            HoverAction::Park => "park",
            HoverAction::Sleep => "sleep",
            HoverAction::Close => "close",
        }
    }

    /// The settings id of an action, for a host routing a hover-button click back to its menu.
    pub fn from_id(value: &str) -> Option<Self> {
        Some(match value {
            "rename" => HoverAction::Rename,
            "pin" => HoverAction::Pin,
            "note" => HoverAction::Note,
            "snooze" => HoverAction::Snooze,
            "closeAfterDone" => HoverAction::CloseAfterDone,
            "tag" => HoverAction::Tag,
            "park" => HoverAction::Park,
            "sleep" => HoverAction::Sleep,
            "close" => HoverAction::Close,
            _ => return None,
        })
    }
}

const CHEVRON_ID: &str = "chevron";

/// One entry of the stored strip: an action or the chevron.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HoverButton {
    /// `None` is the chevron.
    action: Option<HoverAction>,
    enabled: bool,
}

impl HoverButton {
    fn id(&self) -> &'static str {
        match self.action {
            Some(action) => action.as_str(),
            None => CHEVRON_ID,
        }
    }
}

/// `DEFAULT_SESSION_CARD_HOVER_BUTTONS`: only Close shows at rest, and the chevron reveals Tag,
/// Park and Sleep.
fn defaults() -> Vec<HoverButton> {
    [
        (Some(HoverAction::Rename), false),
        (Some(HoverAction::Pin), false),
        (Some(HoverAction::Note), false),
        (Some(HoverAction::Snooze), false),
        (Some(HoverAction::CloseAfterDone), false),
        (Some(HoverAction::Tag), true),
        (Some(HoverAction::Park), true),
        (Some(HoverAction::Sleep), true),
        (None, true),
        (Some(HoverAction::Close), true),
    ]
    .into_iter()
    .map(|(action, enabled)| HoverButton { action, enabled })
    .collect()
}

/// The strip as the user ordered it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoverStrip {
    /// Hidden until the chevron is clicked.
    pub before: Vec<HoverAction>,
    /// Always shown on hover.
    pub after: Vec<HoverAction>,
    pub chevron: bool,
}

impl HoverStrip {
    /// Every enabled action, whichever side of the chevron it sits on. An action listed here is
    /// hidden from the row's context menu.
    pub(crate) fn enabled(&self) -> Vec<HoverAction> {
        self.before.iter().chain(&self.after).copied().collect()
    }

    pub(crate) fn includes(&self, action: HoverAction) -> bool {
        self.before.contains(&action) || self.after.contains(&action)
    }
}

/// `normalizeSessionCardHoverButtons` then `splitSessionCardHoverButtons`, over the raw settings
/// value.
pub fn hover_strip(setting: &Value) -> HoverStrip {
    split(&normalize(setting))
}

fn normalize(setting: &Value) -> Vec<HoverButton> {
    let Some(entries) = setting.as_array() else {
        return defaults();
    };
    // The earlier shape was a plain list of enabled action ids in no particular order; it keeps
    // the default order and leaves the chevron as the default has it.
    if entries.iter().all(Value::is_string) {
        let enabled: Vec<&str> = entries.iter().filter_map(Value::as_str).collect();
        return defaults()
            .into_iter()
            .map(|button| match button.action {
                None => button,
                Some(action) => HoverButton {
                    action: Some(action),
                    enabled: enabled.contains(&action.as_str()),
                },
            })
            .collect();
    }
    let mut items: Vec<HoverButton> = Vec::new();
    for entry in entries {
        let Some(object) = entry.as_object() else {
            continue;
        };
        let Some(id) = object.get("id").and_then(Value::as_str) else {
            continue;
        };
        let button = if id == CHEVRON_ID {
            HoverButton {
                action: None,
                enabled: false,
            }
        } else {
            match HoverAction::from_id(id) {
                Some(action) => HoverButton {
                    action: Some(action),
                    enabled: false,
                },
                None => continue,
            }
        };
        if items.iter().any(|seen| seen.id() == button.id()) {
            continue;
        }
        items.push(HoverButton {
            enabled: object.get("enabled").and_then(Value::as_bool) == Some(true),
            ..button
        });
    }
    // A button added to a newer build is appended disabled, so it never appears on a card until
    // the user turns it on.
    for button in defaults() {
        if !items.iter().any(|seen| seen.id() == button.id()) {
            items.push(HoverButton {
                enabled: false,
                ..button
            });
        }
    }
    items
}

fn split(items: &[HoverButton]) -> HoverStrip {
    let chevron_index = items.iter().position(|item| item.action.is_none());
    let chevron = chevron_index.is_some_and(|index| items[index].enabled);
    if !chevron {
        return HoverStrip {
            before: Vec::new(),
            after: enabled_actions(items),
            chevron: false,
        };
    }
    let index = chevron_index.expect("checked above");
    HoverStrip {
        before: enabled_actions(&items[..index]),
        after: enabled_actions(&items[index + 1..]),
        chevron: true,
    }
}

fn enabled_actions(items: &[HoverButton]) -> Vec<HoverAction> {
    items
        .iter()
        .filter(|item| item.enabled)
        .filter_map(|item| item.action)
        .collect()
}
