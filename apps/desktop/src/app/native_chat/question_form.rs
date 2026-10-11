//! A question form an agent wrote as a ```questions block, drawn where the block sits in its reply:
//! every question at once, choices picked with a click, a note or text box per question, and one
//! Send button. The rules (picks, Other, what is sent, the answered state) are the chat core's
//! (`questionForms`, packages/gx-chat-core/src/questions/forms.rs); this file only lays them out.
//!
//! CDXC:SessionChat 2026-10-11 SEE-ALSO:
//! The phone draws the same form in apps/mobile/app/src/chat/native/transcript/QuestionForm.tsx; the grammar is packages/gx-chat-core/src/transcript/question_forms.rs. Change them together.

use std::collections::HashSet;

use super::{appearance::ChatAppearance, state::NativeChatView, transcript::text};
use crate::app::native_chat::cursor::ChatCursor as _;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, AppContext as _, Context, Entity, FontWeight, Hsla, InteractiveElement as _,
    IntoElement, KeyDownEvent, ParentElement as _, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Window, div, px, rgb, svg,
};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use serde_json::{Value, json};

/// The line that opens a marked form; the form's id follows it (`NATIVE_QUESTIONS_OPEN` in
/// packages/gx-chat-core/src/transcript/native_markdown.rs).
pub(super) const OPEN: &str = "\u{E000}questions:";
pub(super) const CLOSE: &str = "\u{E000}/questions";

/// One open form's note or text box.
pub(super) struct FormInput {
    pub(super) input: Entity<TextareaState>,
    _subscription: Subscription,
    /// The placeholder it shows, which follows the picks.
    placeholder: String,
    /// It holds the reader's text: typed into, or filled from the saved draft once.
    settled: bool,
}

fn error_color(p: &ChatAppearance) -> Hsla {
    rgb(if p.light { 0xb91c1c } else { 0xfb7185 }).into()
}

fn placeholder(question: &Value, draft: &Value) -> String {
    if question["text"] == true {
        let own = text(question, "placeholder");
        return if own.is_empty() {
            "Your answer (optional)".to_string()
        } else {
            own
        };
    }
    if draft["otherPicked"] == true {
        "What would you prefer instead? (optional)".to_string()
    } else {
        "Note (optional)".to_string()
    }
}

impl NativeChatView {
    /// Gives each open form's text boxes an input before the transcript is drawn (the segment
    /// renderer has no window to make one), keeps their placeholders in step with the picks, and
    /// drops the inputs of forms that were answered or left the transcript.
    pub(super) fn sync_form_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let forms = self.snapshot["questionForms"].clone();
        let mut wanted = HashSet::new();
        for (id, form) in forms.as_object().into_iter().flatten() {
            if form["state"] != "open" {
                continue;
            }
            for (index, question) in form["questions"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                let key = format!("{id}:{index}");
                let draft = &form["drafts"][index];
                let hint = placeholder(question, draft);
                wanted.insert(key.clone());
                let value = text(draft, "other");
                if let Some(entry) = self.form_inputs.get_mut(&key) {
                    if entry.placeholder != hint {
                        entry.placeholder = hint.clone();
                        entry
                            .input
                            .update(cx, |state, cx| state.set_placeholder(hint, window, cx));
                    }
                    // The saved draft is read as the form appears, so its text can land after the
                    // box was made; it fills the box once, never over what the reader typed.
                    if !entry.settled && !value.is_empty() {
                        entry.settled = true;
                        entry
                            .input
                            .update(cx, |state, cx| state.set_value(value, window, cx));
                    }
                    continue;
                }
                let settled = !value.is_empty();
                let input = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .auto_grow(1, 6)
                        .placeholder(hint.clone())
                        .default_value(value)
                });
                let typed_key = key.clone();
                let form_id = id.clone();
                let subscription = cx.subscribe_in(
                    &input,
                    window,
                    move |this, input, event: &InputEvent, window, cx| match event {
                        InputEvent::Change => {
                            if let Some(entry) = this.form_inputs.get_mut(&typed_key) {
                                entry.settled = true;
                            }
                            this.invoke(
                                json!({"type":"formText","form":form_id,"question":index,"text":input.read(cx).value().to_string()}),
                                cx,
                            )
                        }
                        // Ctrl+Enter (Cmd+Enter on macOS) sends the form from its note or text box.
                        InputEvent::PressEnter {
                            secondary: true, ..
                        } => this.invoke(json!({"type":"formSend","form":form_id}), cx),
                        InputEvent::Focus => super::focus::reclaim_keyboard_focus(window),
                        _ => {}
                    },
                );
                self.form_inputs.insert(
                    key,
                    FormInput {
                        input,
                        _subscription: subscription,
                        placeholder: hint,
                        settled,
                    },
                );
            }
        }
        self.form_inputs.retain(|key, _| wanted.contains(key));
    }

    /// The form `id` names, or its source as code when the document does not carry it (a form in
    /// a subagent's transcript).
    pub(super) fn question_form(
        &self,
        id: &str,
        fence: &str,
        p: &ChatAppearance,
        cx: &Context<Self>,
    ) -> AnyElement {
        let form = &self.snapshot["questionForms"][id];
        if !form.is_object() {
            return self.text_view(
                format!("question-form-source:{id}"),
                fence.to_string(),
                &Value::Null,
                p,
                cx,
            );
        }
        let s = p.scale;
        let phase = text(form, "state");
        let enabled = phase == "open";
        let title = text(form, "title");
        let error = text(form, "error");
        let good = super::page_answers::answered_color(p);
        let header = div()
            .flex()
            .items_start()
            .gap(px(8.0 * s))
            .child(
                svg()
                    .path("titlebar/help-circle.svg")
                    .size(px(14.0 * s))
                    .mt(px(4.0 * s))
                    .text_color(p.muted)
                    .flex_shrink_0(),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(p.foreground)
                    .child(title),
            )
            .when(phase == "answered", |header| {
                header.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0 * s))
                        .flex_shrink_0()
                        .mt(px(1.0 * s))
                        .px(px(8.0 * s))
                        .py(px(2.0 * s))
                        .rounded_full()
                        .bg(good.opacity(0.14))
                        .text_size(px(11.0 * s))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(good)
                        .child(
                            svg()
                                .path("titlebar/check.svg")
                                .size(px(11.0 * s))
                                .text_color(good),
                        )
                        .child("Answered"),
                )
            })
            .into_any_element();
        if !error.is_empty() {
            let body = vec![
                div()
                    .text_color(error_color(p))
                    .child(format!("These questions could not be shown: {error}"))
                    .into_any_element(),
                self.text_view(
                    format!("question-form-source:{id}"),
                    fence.to_string(),
                    &Value::Null,
                    p,
                    cx,
                ),
            ];
            return self.status_card_with_header(header, body, Vec::new(), p);
        }
        let other_label = text(form, "otherLabel");
        let body: Vec<AnyElement> = form["questions"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(index, question)| {
                self.form_question(
                    id,
                    index,
                    question,
                    &form["drafts"][index],
                    &other_label,
                    enabled,
                    p,
                    cx,
                )
            })
            .collect();
        let send_error = text(form, "sendError");
        let can_send = form["canSend"] == true;
        let status = match phase.as_str() {
            "answered" => "Sent to the agent.".to_string(),
            "queued" => "Queued: it goes to the agent once it's ready.".to_string(),
            "sending" => "Sending…".to_string(),
            _ if !send_error.is_empty() => send_error,
            _ => "Questions you leave blank are sent as unanswered.".to_string(),
        };
        let status_color = if enabled && !text(form, "sendError").is_empty() {
            error_color(p)
        } else {
            p.muted
        };
        let send_id = SharedString::from(format!("question-form-send:{id}"));
        let send_label = match phase.as_str() {
            "answered" => "Sent".to_string(),
            "queued" => "Queued".to_string(),
            _ => text(form, "submit"),
        };
        let form_id = id.to_string();
        let actions = vec![
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(12.5 * s))
                .text_color(status_color)
                .child(status)
                .into_any_element(),
            div()
                .id(send_id)
                .role(gpui::Role::Button)
                .aria_label(send_label.clone())
                .h(px(28.0 * s))
                .px(px(12.0 * s))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.0 * s))
                .border_1()
                .border_color(p.control_border)
                .text_color(p.foreground)
                .when(p.light, |button| button.bg(p.background))
                .when(!can_send, |button| button.opacity(0.5))
                .when(can_send, |button| {
                    button
                        .chat_cursor_pointer()
                        .hover(|style| style.bg(p.input))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.invoke(json!({"type":"formSend","form":form_id}), cx)
                        }))
                })
                .child(send_label)
                .into_any_element(),
        ];
        // CDXC:SessionChat 2026-10-11 DECISION: User: Ctrl+Enter (Cmd+Enter on macOS) presses the form's send button from anywhere in it, a note box included; the page kit (server/src/visual_pages/kit.rs) does the same.
        let send_form = id.to_string();
        div()
            .w_full()
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "enter" && event.keystroke.modifiers.secondary() {
                    this.invoke(json!({"type":"formSend","form":send_form}), cx);
                    cx.stop_propagation();
                }
            }))
            .child(self.status_card_with_header(header, body, actions, p))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn form_question(
        &self,
        id: &str,
        index: usize,
        question: &Value,
        draft: &Value,
        other_label: &str,
        enabled: bool,
        p: &ChatAppearance,
        cx: &Context<Self>,
    ) -> AnyElement {
        let s = p.scale;
        let tag = text(question, "tag");
        let detail = text(question, "detail");
        let multiple = question["multiple"] == true;
        let picked: Vec<u64> = draft["indices"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
            .collect();
        let mut column = div()
            .flex()
            .flex_col()
            .gap(px(6.0 * s))
            .w_full()
            .min_w_0()
            .child(
                // CDXC:SessionChat 2026-10-11 DECISION: User picked from the looks page: a question's tag is a filled chip (no border) placed above its title, "b, but pls make it show like eyebrow (in terms of position)"; and choices keep the radio or checkbox with the letter as a muted keycap on the right (look A). The phone (QuestionForm.tsx) and the page kit (server/src/visual_pages/kit.rs) match.
                div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(4.0 * s))
                    .when(!tag.is_empty(), |header| {
                        header.child(
                            div()
                                .px(px(7.0 * s))
                                .rounded_full()
                                .bg(p.muted.opacity(0.18))
                                .font_family(super::fonts::CHAT_MONO)
                                .text_size(px(10.5 * s))
                                .line_height(px(18.0 * s))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tag),
                        )
                    })
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.foreground)
                            .child(text(question, "title")),
                    ),
            )
            .when(!detail.is_empty(), |column| {
                column.child(
                    div()
                        .text_size(px(13.0 * s))
                        .line_height(px(18.5 * s))
                        .text_color(p.card_muted)
                        .child(detail),
                )
            });
        if question["text"] != true {
            if multiple {
                column = column.child(
                    div()
                        .text_size(px(12.0 * s))
                        .text_color(p.muted)
                        .child("Pick any that apply."),
                );
            }
            let choices = question["choices"].as_array().cloned().unwrap_or_default();
            for (choice_index, choice) in choices.iter().enumerate() {
                column = column.child(self.form_choice_row(
                    format!("question-form-choice:{id}:{index}:{choice_index}"),
                    choice_index,
                    text(choice, "label"),
                    text(choice, "detail"),
                    choice["recommended"] == true,
                    picked.contains(&(choice_index as u64)),
                    multiple,
                    false,
                    enabled,
                    json!({"type":"formChoice","form":id,"question":index,"choice":choice_index}),
                    p,
                    cx,
                ));
            }
            column = column.child(self.form_choice_row(
                format!("question-form-choice:{id}:{index}:other"),
                choices.len(),
                other_label.to_string(),
                String::new(),
                false,
                draft["otherPicked"] == true,
                multiple,
                true,
                enabled,
                json!({"type":"formChoice","form":id,"question":index,"choice":choices.len()}),
                p,
                cx,
            ));
        }
        let written = text(draft, "other");
        match self.form_inputs.get(&format!("{id}:{index}")) {
            Some(FormInput { input, .. }) if enabled => {
                column = column.child(
                    div()
                        .w_full()
                        .px(px(10.0 * s))
                        .py(px(6.0 * s))
                        .rounded(px(8.0 * s))
                        .border_1()
                        .border_color(p.control_border)
                        .when(p.light, |field| field.bg(p.background))
                        .child(
                            Textarea::new(input)
                                .aria_label("Note")
                                .placeholder_color(p.muted.opacity(0.6))
                                .caret_color(p.foreground)
                                .appearance(false)
                                .bordered(false)
                                .focus_bordered(false)
                                .p_0()
                                .line_height(px(20.0 * s))
                                .text_color(p.foreground)
                                .text_size(px(13.5 * s)),
                        ),
                );
            }
            _ if !written.trim().is_empty() => {
                let shown = if question["text"] == true {
                    written
                } else {
                    format!("Note: {written}")
                };
                column = column.child(
                    div()
                        .text_size(px(13.5 * s))
                        .line_height(px(19.0 * s))
                        .text_color(p.card_muted)
                        .child(shown),
                );
            }
            _ if question["text"] == true && !enabled => {
                column = column.child(
                    div()
                        .text_size(px(13.5 * s))
                        .text_color(p.muted)
                        .child("Not answered"),
                );
            }
            _ => {}
        }
        column.into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn form_choice_row(
        &self,
        id: String,
        number: usize,
        label: String,
        detail: String,
        recommended: bool,
        picked: bool,
        multiple: bool,
        other: bool,
        enabled: bool,
        action: Value,
        p: &ChatAppearance,
        cx: &Context<Self>,
    ) -> AnyElement {
        let s = p.scale;
        let letter = if number < 26 {
            ((b'a' + number as u8) as char).to_string()
        } else {
            (number + 1).to_string()
        };
        let mark = div()
            .flex_shrink_0()
            .mt(px(3.0 * s))
            .size(px(14.0 * s))
            .flex()
            .items_center()
            .justify_center()
            .border(px(1.5 * s))
            .border_color(if picked { p.control_primary } else { p.muted })
            .when(multiple, |mark| mark.rounded(px(4.0 * s)))
            .when(!multiple, |mark| mark.rounded_full())
            .when(picked && multiple, |mark| {
                mark.bg(p.control_primary).child(
                    svg()
                        .path("titlebar/check.svg")
                        .size(px(10.0 * s))
                        .text_color(p.background),
                )
            })
            .when(picked && !multiple, |mark| {
                mark.child(div().size(px(6.0 * s)).rounded_full().bg(p.control_primary))
            });
        div()
            .id(SharedString::from(id))
            .when(enabled, |row| row.tab_index(0))
            .role(if multiple {
                gpui::Role::CheckBox
            } else {
                gpui::Role::RadioButton
            })
            .aria_label(label.clone())
            .flex()
            .items_start()
            .gap(px(10.0 * s))
            .w_full()
            .px(px(12.0 * s))
            .py(px(8.0 * s))
            .rounded(px(8.0 * s))
            .border_1()
            .border_color(if picked {
                p.control_primary.opacity(0.3)
            } else {
                p.control_border
            })
            .text_color(p.foreground)
            .when(p.light, |row| row.bg(p.background))
            .when(picked, |row| row.bg(p.control_primary.opacity(0.1)))
            .when(!enabled && !picked, |row| row.opacity(0.5))
            .when(enabled, |row| {
                row.chat_cursor_pointer().when(!picked, |row| {
                    row.hover(|style| {
                        style.bg(if p.light {
                            p.input
                        } else {
                            p.input_border.opacity(0.3)
                        })
                    })
                })
            })
            .child(mark)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.0 * s))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.0 * s))
                            .text_size(px(14.0 * s))
                            .line_height(px(19.25 * s))
                            .child(
                                div()
                                    .min_w_0()
                                    .when(other && !picked, |label| label.text_color(p.card_muted))
                                    .child(label.clone()),
                            )
                            .when(recommended, |line| {
                                line.child(
                                    div()
                                        .flex_shrink_0()
                                        .px(px(6.0 * s))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(p.control_primary.opacity(0.6))
                                        .text_size(px(10.5 * s))
                                        .line_height(px(16.0 * s))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.control_primary)
                                        .child("Recommended"),
                                )
                            }),
                    )
                    .when(!detail.is_empty(), |column| {
                        column.child(
                            div()
                                .text_size(px(13.0 * s))
                                .line_height(px(18.0 * s))
                                .text_color(p.card_muted)
                                .child(detail),
                        )
                    }),
            )
            // The letter the reader can refer to ("do b"), as a muted keycap at the row's end.
            .child(
                div()
                    .flex_shrink_0()
                    .h(px(18.0 * s))
                    .min_w(px(18.0 * s))
                    .px(px(4.0 * s))
                    .mt(px(0.5 * s))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0 * s))
                    .border_1()
                    .border_color(p.control_border.opacity(0.6))
                    .bg(p.background.opacity(0.4))
                    .font_family(super::fonts::CHAT_MONO)
                    .text_size(px(11.0 * s))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if picked { p.foreground } else { p.muted })
                    .child(letter),
            )
            .when(enabled, |row| {
                row.on_click(cx.listener(move |this, _, _, cx| this.invoke(action.clone(), cx)))
            })
            .into_any_element()
    }
}
