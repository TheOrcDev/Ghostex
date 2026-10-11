//! The native Kanban's colours and type, taken from the native chat's appearance so the board and
//! the chat read as one app, with the chat's frosted treatment under window glass.

use gpui::{Hsla, Window, rgb};

use super::model::LaneTone;
use crate::app::native_chat::appearance::ChatAppearance;

#[derive(Clone)]
pub(crate) struct KanbanPalette {
    pub(crate) glass: bool,
    pub(crate) font: String,
    /// The page fill; transparent under glass so the frosted work area shows through.
    pub(crate) page: Hsla,
    pub(crate) lane: Hsla,
    pub(crate) lane_drop: Hsla,
    pub(crate) card: Hsla,
    pub(crate) card_hover: Hsla,
    pub(crate) control: Hsla,
    pub(crate) control_hover: Hsla,
    pub(crate) panel: Hsla,
    pub(crate) border: Hsla,
    pub(crate) border_strong: Hsla,
    pub(crate) foreground: Hsla,
    pub(crate) primary: Hsla,
    pub(crate) muted: Hsla,
    pub(crate) faint: Hsla,
    pub(crate) accent: Hsla,
    pub(crate) danger: Hsla,
    pub(crate) button: Hsla,
    pub(crate) button_text: Hsla,
}

fn ink(light: bool, alpha: f32) -> Hsla {
    Hsla::from(rgb(if light { 0x000000 } else { 0xffffff })).opacity(alpha)
}

fn step(color: Hsla, light: bool, amount: f32) -> Hsla {
    Hsla {
        l: if light {
            (color.l - amount).max(0.0)
        } else {
            (color.l + amount).min(1.0)
        },
        ..color
    }
}

impl KanbanPalette {
    pub(crate) fn current(window: &Window) -> Self {
        let glass = crate::app::helpers::window_glass_active_in(window);
        let chat = ChatAppearance::current(&serde_json::Value::Null);
        let light = chat.light;
        let (page, lane, card, control, panel, border) = if glass {
            (
                gpui::transparent_black(),
                ink(light, if light { 0.03 } else { 0.04 }),
                ink(light, if light { 0.04 } else { 0.06 }),
                ink(light, if light { 0.04 } else { 0.06 }),
                ink(light, if light { 0.03 } else { 0.05 }),
                ink(light, 0.08),
            )
        } else {
            (
                chat.background,
                chat.input,
                chat.card_panel,
                chat.input,
                chat.card_panel,
                chat.border,
            )
        };
        let card_hover = if glass {
            ink(light, if light { 0.07 } else { 0.09 })
        } else {
            step(card, light, 0.03)
        };
        let control_hover = if glass {
            ink(light, if light { 0.07 } else { 0.1 })
        } else {
            step(control, light, 0.04)
        };
        Self {
            glass,
            font: chat.font.clone(),
            page,
            lane,
            lane_drop: if glass {
                ink(light, if light { 0.06 } else { 0.08 })
            } else {
                step(lane, light, 0.03)
            },
            card,
            card_hover,
            control,
            control_hover,
            panel,
            border,
            border_strong: ink(light, if light { 0.16 } else { 0.18 }),
            foreground: chat.foreground,
            primary: chat.primary,
            muted: chat.muted,
            faint: chat.muted.opacity(0.7),
            accent: chat.accent,
            danger: chat.error(),
            button: chat.control_primary,
            button_text: rgb(if light { 0xffffff } else { 0x0d0d0d }).into(),
        }
    }

    /// The lane header dot (`LANE_TONE_COLORS`).
    pub(crate) fn tone(&self, tone: LaneTone) -> Hsla {
        match tone {
            LaneTone::Muted => rgb(0x8f9aa7).into(),
            LaneTone::Blue => rgb(0x5ea4ff).into(),
            LaneTone::Amber => rgb(0xe7b85b).into(),
            LaneTone::Violet => rgb(0xb18cff).into(),
            LaneTone::Green => rgb(0x95d7f6).into(),
            LaneTone::Neutral => self.foreground.opacity(0.42),
        }
    }

    /// `chipToneColor`: one stable colour per label or assignee.
    pub(crate) fn chip_tone(seed: &str) -> Hsla {
        const TONES: [u32; 7] = [
            0x5ea4ff, 0xe7b85b, 0xb18cff, 0x6fd19c, 0xf28b8b, 0x95d7f6, 0xe79ad0,
        ];
        let hash = seed.encode_utf16().fold(0_u32, |hash, unit| {
            hash.wrapping_mul(31).wrapping_add(u32::from(unit))
        });
        rgb(TONES[hash as usize % TONES.len()]).into()
    }

    /// The priority icon's colour (`TicketPriorityIcon`).
    pub(crate) fn priority_tone(&self, priority: Option<i64>) -> Hsla {
        match priority.unwrap_or(2) {
            value if value <= 0 => rgb(0xfb923c).opacity(0.9),
            1 => rgb(0xfbbf24).opacity(0.9),
            2 => rgb(0x38bdf8).opacity(0.8),
            _ => self.muted.opacity(0.7),
        }
    }
}

trait RgbOpacity {
    fn opacity(self, alpha: f32) -> Hsla;
}

impl RgbOpacity for gpui::Rgba {
    fn opacity(self, alpha: f32) -> Hsla {
        Hsla::from(self).opacity(alpha)
    }
}
