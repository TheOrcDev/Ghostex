//! The answers a reader sent from an agent's HTML page, drawn as a card instead of the JSON message
//! gxserver typed into the session (`pageAnswers`, packages/gx-chat-core/src/transcript/page_answers.rs).
//!
//! CDXC:SessionChat 2026-10-11 SEE-ALSO:
//! The phone draws the same card in apps/mobile/app/src/chat/native/transcript/PageAnswersCard.tsx; the page's own card reads Answered in visual.rs. Change them together.

use super::{appearance::ChatAppearance, state::NativeChatView, transcript::text};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, FontWeight, Hsla, IntoElement, ParentElement as _, Styled as _, div, px, rgb, svg,
};
use serde_json::Value;

/// The green the chat's charts use for good news, which also marks an answered page.
pub(super) fn answered_color(p: &ChatAppearance) -> Hsla {
    rgb(if p.light { 0x059669 } else { 0x34d399 }).into()
}

impl NativeChatView {
    pub(super) fn page_answers_card(&self, answers: &Value, p: &ChatAppearance) -> AnyElement {
        let s = p.scale;
        let good = answered_color(p);
        let header = div()
            .flex()
            .items_start()
            .gap(px(8.0 * s))
            .child(
                svg()
                    .path("titlebar/check.svg")
                    .size(px(14.0 * s))
                    .mt(px(4.0 * s))
                    .text_color(good)
                    .flex_shrink_0(),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(p.foreground)
                    .child(format!("You answered “{}”", text(answers, "title"))),
            )
            .into_any_element();
        let mut body: Vec<AnyElement> = answers["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| {
                let label = text(row, "label");
                let note = text(row, "note");
                // CDXC:SessionChat 2026-10-11 DECISION: User: "make it show the Q above the Answer so they both have more width area": each row is the question (muted) on its own line, the answer (bold) below it at the card's full width, then its note. The phone (PageAnswersCard.tsx) stacks the same way.
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0 * s))
                    .w_full()
                    .min_w_0()
                    .when(!label.is_empty(), |this| {
                        this.child(
                            div()
                                .text_size(px(13.0 * s))
                                .line_height(px(18.5 * s))
                                .text_color(p.muted)
                                .child(label),
                        )
                    })
                    .child(
                        div()
                            .text_size(px(14.0 * s))
                            .line_height(px(20.0 * s))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.foreground)
                            .child(text(row, "value")),
                    )
                    .when(!note.is_empty(), |this| {
                        this.child(
                            div()
                                .text_size(px(12.5 * s))
                                .line_height(px(18.0 * s))
                                .text_color(p.muted)
                                .child(note),
                        )
                    })
                    .into_any_element()
            })
            .collect();
        if let Some(more) = answers["more"].as_u64().filter(|more| *more > 0) {
            body.push(
                div()
                    .text_size(px(12.0 * s))
                    .text_color(p.muted)
                    .child(format!("and {more} more"))
                    .into_any_element(),
            );
        }
        self.status_card_with_header(header, body, Vec::new(), p)
    }
}
