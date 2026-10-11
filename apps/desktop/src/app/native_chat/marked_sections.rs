//! Marked sections in the transcript: the look (`marked_look.rs`) in this chat's tones, and the
//! summary chips' jump to their section.
//!
//! The core finds the sections and the counts (`packages/gx-chat-core/src/transcript/marked_sections.rs`)
//! and sends them as marks around the section's own lines; `rich_markdown.rs` splits on the marks.
use super::appearance::ChatAppearance;
use super::state::NativeChatView;
use gpui::{AnyElement, Bounds, IntoElement, Pixels, Styled as _, canvas, px};
use std::cell::RefCell;
use std::collections::HashMap;

pub(crate) use super::marked_look::{
    MarkedLook, MarkerStyle, SECTION_CLOSE, SECTION_OPEN, SUMMARY_OPEN, SectionMeta, SummaryChip,
    section, summary_strip,
};

/// The look of marked sections in this chat's tones.
pub(super) fn look(p: &ChatAppearance) -> MarkedLook {
    MarkedLook {
        light: p.light,
        scale: p.scale,
        foreground: p.foreground,
        prose: p.prose,
        border: p.border,
        line_height: super::rich_markdown::PROSE_LINE_HEIGHT,
        block_gap: super::markdown_style::paragraph_gap(p),
    }
}

thread_local! {
    /// Where each drawn section's top was last painted, by `"{message}#{section}"`, so a summary
    /// chip can scroll its section into view.
    static SECTION_TOPS: RefCell<HashMap<String, Pixels>> = RefCell::new(HashMap::new());
}

fn section_key(message: &str, section: usize) -> String {
    format!("{message}#{section}")
}

/// A layer behind a section that records its painted top.
pub(super) fn top_recorder(message: &str, section: usize) -> AnyElement {
    let key = section_key(message, section);
    canvas(
        move |bounds: Bounds<Pixels>, _, _| {
            SECTION_TOPS.with_borrow_mut(|tops| {
                if tops.len() > 512 {
                    tops.clear();
                }
                tops.insert(key.clone(), bounds.origin.y);
            });
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}

impl NativeChatView {
    /// A summary chip was pressed: bring its section to the top of the transcript.
    pub(super) fn jump_to_marked_section(
        &mut self,
        message: &str,
        section: usize,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(top) =
            SECTION_TOPS.with_borrow(|tops| tops.get(&section_key(message, section)).copied())
        else {
            return;
        };
        let scale = ChatAppearance::current(&self.snapshot).scale;
        let viewport = self.list.viewport_bounds();
        self.list
            .scroll_by(top - viewport.origin.y - px(16.0 * scale));
        cx.notify();
    }
}
