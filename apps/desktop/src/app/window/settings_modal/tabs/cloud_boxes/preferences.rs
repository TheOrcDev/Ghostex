//! The agent sign-in rows (boxes keep their own Claude sign-in) and the `agentboxDefaultLocation`
//! setting the New Thread picker starts on.
use super::super::super::catalog::SettingOption;
use super::super::super::fields::{
    ButtonVariant, ListItemStatus, RowSpec, reset_key, select_field, settings_button,
    settings_section,
};
use super::super::super::palette::SettingsPalette;
use super::model::{StatusSummary, location_options, provider_rows};
use super::{CloudBoxesTab, info_row};
use gpui::{AnyElement, Context, IntoElement, SharedString, Window};
use serde_json::Value;

const ICON_KEY: &str = "modals/settings/key.svg";
const LOCATION_KEY: &str = "agentboxDefaultLocation";

impl CloudBoxesTab {
    pub(super) fn sign_in_section(
        &mut self,
        p: &SettingsPalette,
        summary: &StatusSummary,
        show: impl Fn(&str) -> bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let blocked: Option<SharedString> = match summary.installed {
            Some(false) => Some("Install agentbox first.".into()),
            _ if !self.rpc_available(cx) => {
                Some("Ghostex could not reach its server on this computer.".into())
            }
            _ => None,
        };
        let mut rows = Vec::new();
        if show("agentboxClaudeSignIn") {
            rows.push(self.sign_in_row(
                p,
                "claude",
                "Claude in boxes",
                summary.claude_signed_in,
                "Claude needs its own one-time sign-in for boxes, so Claude on this computer stays signed in. Every box uses it.",
                blocked.clone(),
                cx,
            ));
        }
        if show("agentboxCodexSignIn") {
            rows.push(self.sign_in_row(
                p,
                "codex",
                "Codex in boxes",
                summary.codex_signed_in,
                "Boxes on this computer reuse your Codex sign-in. Sign in here if Codex is not signed in on this computer, or for cloud boxes.",
                blocked,
                cx,
            ));
        }
        settings_section(
            p,
            "Agent sign-in",
            Some("Your agent's settings and skills go with every box.".into()),
            None,
            rows,
        )
        .map(IntoElement::into_any_element)
    }

    #[allow(clippy::too_many_arguments)]
    fn sign_in_row(
        &mut self,
        p: &SettingsPalette,
        agent: &'static str,
        title: &str,
        signed_in: Option<bool>,
        tooltip: &str,
        blocked: Option<SharedString>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dot = match signed_in {
            Some(true) => ListItemStatus::Success,
            Some(false) => ListItemStatus::Warning,
            None => ListItemStatus::Neutral,
        };
        let button = settings_button(
            p,
            SharedString::from(format!("cloud-boxes-sign-in-{agent}")),
            if signed_in == Some(true) {
                "Sign In Again"
            } else {
                "Sign In"
            },
            Some(ICON_KEY),
            if signed_in == Some(true) {
                ButtonVariant::Ghost
            } else {
                ButtonVariant::Outline
            },
            blocked.is_some(),
            blocked,
            move |page: &mut Self, _window, cx| {
                page.run_terminal_command("agentLogin", &[("agent", agent)], cx)
            },
            cx,
        );
        let state = match signed_in {
            Some(true) => Some("Signed in".to_string()),
            Some(false) => Some("Not signed in".to_string()),
            None => None,
        };
        info_row(
            p,
            &format!("sign-in-{agent}"),
            Some(dot),
            ICON_KEY,
            title.to_string(),
            tooltip.to_string(),
            state,
            vec![button],
        )
    }

    /// `agentboxDefaultLocation`: where new threads run unless the user picks another place.
    pub(super) fn default_location_section(
        &mut self,
        p: &SettingsPalette,
        show: impl Fn(&str) -> bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !show(LOCATION_KEY) {
            return None;
        }
        let values = self.store.read(cx).values();
        let saved = values.string(LOCATION_KEY);
        let saved = if saved.trim().is_empty() {
            "local".to_string()
        } else {
            saved
        };
        let rows = provider_rows(self.status.as_ref());
        let options: Vec<SettingOption> = location_options(&rows, &saved)
            .into_iter()
            .map(|(label, value)| SettingOption { label, value })
            .collect();
        let row = select_field(
            self,
            p,
            LOCATION_KEY,
            RowSpec::new("Default location")
                .description("Where new threads run unless you pick another location.")
                .keyed(&values, LOCATION_KEY),
            Some(reset_key(LOCATION_KEY)),
            &options,
            &saved,
            Some(240.0),
            |page: &mut Self, value, _window, cx| {
                let store = page.store.clone();
                store.update(cx, |store, cx| {
                    store.update_setting(LOCATION_KEY, Value::String(value), cx)
                });
            },
            window,
            cx,
        );
        settings_section(
            p,
            "New threads",
            Some("Pick a box location once it shows Ready above.".into()),
            None,
            vec![row],
        )
        .map(IntoElement::into_any_element)
    }
}
