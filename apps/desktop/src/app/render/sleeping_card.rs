//! The card every sleeping view, session and pane shows: what is asleep, and how to bring it back.

use gpui::AnyElement;
use gpui::FontWeight;
use gpui::IntoElement;
use gpui::ParentElement as _;
use gpui::Styled as _;
use gpui::div;
use gpui::prelude::FluentBuilder as _;
use gpui::px;
use gpui_component::v_flex;

use crate::app::helpers::*;
use crate::app::model::*;
use crate::*;

/// The size a caller draws the sleeping pill's icon at.
pub(crate) const SLEEPING_CARD_ICON_SIZE: f32 = 18.0;
const SLEEPING_CARD_WIDTH: f32 = 340.0;
/// The widest the sleeping pill grows to fit a long title before the title truncates.
const SLEEPING_PILL_MAX_WIDTH: f32 = 325.0;

/// CDXC:SessionSleep 2026-09-23 DECISION:
/// User picked the slim pill from the sleeping-card mockups ("this one is very nice"), then dropped its label: "remove 'is paused' just keep empty space there but reduce the space just slightly". Every sleeping view, Agents pane and command pane shows one rounded row, centred in its body, of the view's icon (the Browser's tab favicon), its title, a short empty gap and a Resume button. It supersedes the same day's centred card with the Browser's domain under the favicon; the pill has no domain line. Clicking anywhere resumes, not just the button: the card holds no input of its own, and the body it sits in owns the click (and key) that wakes it, so the Resume button is its look, not a separate target. When click-to-wake is off the button is replaced by "press any key", which is the only wake left.
/// Later the same day the user found the gap between title and Resume too big ("make this narrower by default ... get wider based on the text up to a point (still 325 px max width total)"): the pill hugs its title with a small gap, and past 325px the title truncates instead of the pill growing.
pub(crate) fn sleeping_card(
    icon: Option<AnyElement>,
    title: String,
    click_to_wake: bool,
) -> AnyElement {
    let ink = chrome_ink();
    let pill = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .max_w(px(SLEEPING_PILL_MAX_WIDTH))
        .min_w_0()
        .pl(px(14.0))
        .pr(px(6.0))
        .py(px(6.0))
        .rounded_full()
        .bg(view_card_wash())
        .when_some(icon, |this, icon| {
            this.child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(SLEEPING_CARD_ICON_SIZE))
                    .child(icon),
            )
        })
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_size(px(13.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(ink.opacity(0.9))
                .child(title),
        )
        // A small gap between the title and the button.
        .child(div().flex_shrink_0().w(px(4.0)))
        .when(click_to_wake, |this| {
            this.child(
                view_card_button("Resume")
                    .flex_shrink_0()
                    .h(px(28.0))
                    .rounded_full(),
            )
        })
        .when(!click_to_wake, |this| {
            this.child(
                div()
                    .flex_shrink_0()
                    .whitespace_nowrap()
                    .pr(px(8.0))
                    .text_size(px(12.5))
                    .text_color(ink.opacity(0.5))
                    .child("press any key"),
            )
        });
    // The wrapper keeps the pill inside a body narrower than its 325px cap.
    div()
        .flex()
        .max_w_full()
        .min_w_0()
        .child(pill)
        .into_any_element()
}

/// The frosted card frame every sleeping card is built on, shared by the other centred view states
/// (a view starting up, failing, or asking for setup, and the Linear and Jira setup page) so they
/// all read as the same card.
///
/// CDXC:Theming 2026-09-23 DECISION:
/// User: "make all these resume pages look like the other glassified ones we did (like linear first time for example)". Under window glass every sleeping view, Agents pane and command pane paints no page fill behind this card (the Code view's #0e0e0e included), so the card is a frosted wash over the blurred window like the Linear setup card, never a grey block on black.
pub(crate) fn view_card_frame() -> gpui::Div {
    v_flex()
        .w(px(SLEEPING_CARD_WIDTH))
        .max_w_full()
        .min_w_0()
        .items_center()
        .px(px(28.0))
        .pt(px(28.0))
        .pb(px(24.0))
        .rounded(px(16.0))
        /*
        CDXC:Theming 2026-09-23 DECISION:
        User: the sleeping card's glass is "simpler ... not overdone", like the chat's frosted message bubble: one flat wash of the ink, no gradient, lit edge, border or drop shadow. Supersedes the same day's gradient-and-shadow card.
        */
        .bg(view_card_wash())
}

/// The card's flat ink wash, shared by the sleeping pill.
fn view_card_wash() -> gpui::Rgba {
    chrome_ink().opacity(if chrome_uses_light_appearance() {
        0.04
    } else {
        0.06
    })
}

/// The card's soft action button, the Resume button's look.
pub(crate) fn view_card_button(label: impl Into<gpui::SharedString>) -> gpui::Div {
    let ink = chrome_ink();
    div()
        .flex()
        .h(px(30.0))
        .px(px(16.0))
        .items_center()
        .justify_center()
        .rounded(px(8.0))
        .bg(ink.opacity(0.08))
        .text_size(px(12.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ink.opacity(0.9))
        .cursor_pointer()
        .hover(move |this| this.bg(ink.opacity(0.14)))
        .child(label.into())
}

/// Centres a sleeping card over the body it belongs to without taking any of the body's input.
pub(crate) fn sleeping_card_layer(card: AnyElement) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p(px(16.0))
        .child(card)
        .into_any_element()
}

impl GhostexGpuiApp {
    /// The sleeping Browser's card names the focused pane's tab: its favicon, domain and title.
    pub(crate) fn render_browser_sleeping_card(&self) -> AnyElement {
        let tab = self
            .browser_tabs
            .active_tab_for_pane(self.browser_tabs.focused_pane)
            .filter(|tab| tab.state == BrowserTabState::Loaded);
        let title = tab
            .map(BrowserTab::display_title)
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "Browser".to_string());
        let favicon = browser_favicon_element(
            SLEEPING_CARD_ICON_SIZE,
            tab.and_then(|tab| tab.runtime_favicon_image.as_ref()),
            tab.and_then(|tab| tab.runtime_favicon_fetch.as_ref()),
        );
        sleeping_card(Some(favicon), title, true)
    }
}
