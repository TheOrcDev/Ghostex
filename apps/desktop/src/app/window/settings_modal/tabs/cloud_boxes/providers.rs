//! The "Where boxes run" section: one row per agentbox provider with its readiness and the setup
//! steps it needs (Set Up for Docker, Log In and Prepare for a cloud, Check for a registered
//! server), and the Add Server form that registers your own server over SSH.
use super::super::super::fields::{
    ButtonVariant, FieldStates, ListItemStatus, settings_button, settings_section,
    settings_text_input,
};
use super::super::super::palette::SettingsPalette;
use super::model::{
    ProviderKind, ProviderRow, StatusSummary, provider_rows, valid_alias, valid_ssh_target,
};
use super::{CloudBoxesTab, info_row};
use gpui::{
    AnyElement, Context, IntoElement, ParentElement as _, SharedString, Styled as _, Window, div,
    px,
};
use gpui_component::{h_flex, v_flex};

const ICON_BOX: &str = "modals/settings/box.svg";
const ICON_CLOUD: &str = "modals/settings/cloud.svg";
const ICON_SERVER: &str = "modals/settings/server.svg";
const ICON_KEY: &str = "modals/settings/key.svg";
const ICON_DOWNLOAD: &str = "modals/settings/download.svg";
const ICON_PLUS: &str = "modals/settings/plus.svg";
const ICON_TERMINAL: &str = "modals/settings/terminal-2.svg";

const ALIAS_FIELD: &str = "cloud-boxes-server-alias";
const SSH_FIELD: &str = "cloud-boxes-server-ssh";

/// A provider row's dot, its short state, and what its tooltip adds about that state.
fn provider_state(
    row: &ProviderRow,
    summary: &StatusSummary,
) -> (ListItemStatus, Option<String>, Option<String>) {
    let detail = row.detail.clone();
    match (row.kind, row.ready) {
        (ProviderKind::RemoteDockerSetup, _) | (_, None) => (ListItemStatus::Neutral, None, None),
        (ProviderKind::RemoteDocker, Some(true)) => {
            (ListItemStatus::Success, Some("Ready".into()), detail)
        }
        (_, Some(true)) => (ListItemStatus::Success, Some("Ready".into()), None),
        (ProviderKind::Local, Some(false)) if summary.docker_ready == Some(false) => (
            ListItemStatus::Warning,
            Some("Docker not running".into()),
            Some("Start Docker Desktop, OrbStack or Colima, then Refresh.".into()),
        ),
        (ProviderKind::Local, Some(false)) => {
            (ListItemStatus::Warning, Some("Not set up".into()), detail)
        }
        (ProviderKind::Cloud, Some(false)) if row.configured == Some(true) => (
            ListItemStatus::Warning,
            Some("Logged in, needs Prepare".into()),
            None,
        ),
        (ProviderKind::Cloud, Some(false)) => {
            (ListItemStatus::Neutral, Some("Not set up".into()), None)
        }
        (ProviderKind::RemoteDocker, Some(false)) => (
            ListItemStatus::Warning,
            Some("Not reachable".into()),
            detail,
        ),
    }
}

/// What a provider row's info icon explains: the provider and what its buttons do.
fn provider_tooltip(row: &ProviderRow, extra: Option<String>) -> String {
    let base = match row.kind {
        ProviderKind::Local => format!("{} Set Up builds the box image once.", row.description),
        ProviderKind::Cloud => format!(
            "{} Log In asks for an API token from your {} account; Prepare builds its base image once. Cloud boxes bill while they exist.",
            row.description, row.label
        ),
        ProviderKind::RemoteDocker => {
            format!("{} Check tests that boxes can run there.", row.description)
        }
        ProviderKind::RemoteDockerSetup => format!(
            "{} Add Server registers it by a name and its SSH address (user@host, host:port, or a name from ~/.ssh/config). The server needs Docker, and the terminal may ask you to trust its key.",
            row.description
        ),
    };
    match extra {
        Some(extra) => format!("{base} {extra}"),
        None => base,
    }
}

impl CloudBoxesTab {
    pub(super) fn providers_section(
        &mut self,
        p: &SettingsPalette,
        summary: &StatusSummary,
        show: impl Fn(&str) -> bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let rows = provider_rows(self.status.as_ref());
        // Steps need agentbox; until it is installed every button says so.
        let blocked: Option<SharedString> = match summary.installed {
            Some(false) => Some("Install agentbox first.".into()),
            _ if !self.rpc_available(cx) => {
                Some("Ghostex could not reach its server on this computer.".into())
            }
            _ => None,
        };
        let mut elements: Vec<AnyElement> = Vec::new();
        for row in &rows {
            let key = match row.kind {
                ProviderKind::RemoteDocker | ProviderKind::RemoteDockerSetup => {
                    "agentboxRemoteDocker"
                }
                _ => "agentboxProviders",
            };
            if !show(key) {
                continue;
            }
            elements.push(self.provider_row(p, row, summary, blocked.clone(), cx));
            if row.kind == ProviderKind::RemoteDockerSetup && self.adding_server {
                elements.push(self.server_form(p, blocked.is_some(), window, cx));
            }
        }
        settings_section(
            p,
            "Where boxes run",
            Some("Docker on this computer is free. Cloud boxes bill while they exist.".into()),
            None,
            elements,
        )
        .map(IntoElement::into_any_element)
    }

    fn provider_row(
        &mut self,
        p: &SettingsPalette,
        row: &ProviderRow,
        summary: &StatusSummary,
        blocked: Option<SharedString>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (dot, state, extra) = provider_state(row, summary);
        let tooltip = provider_tooltip(row, extra);
        let ready = row.ready == Some(true);
        // A finished step stays available but quiet.
        let variant = if ready {
            ButtonVariant::Ghost
        } else {
            ButtonVariant::Outline
        };
        let disabled = blocked.is_some();
        let id = row.id.clone();
        let mut controls: Vec<AnyElement> = Vec::new();
        match row.kind {
            ProviderKind::Local => {
                controls.push(settings_button(
                    p,
                    SharedString::from(format!("cloud-boxes-provider-{id}-setup")),
                    "Set Up",
                    Some(ICON_DOWNLOAD),
                    variant,
                    disabled,
                    blocked.clone(),
                    move |page: &mut Self, _window, cx| {
                        page.run_terminal_command("setup", &[("provider", "docker")], cx)
                    },
                    cx,
                ));
            }
            ProviderKind::Cloud => {
                let login_id = id.clone();
                controls.push(settings_button(
                    p,
                    SharedString::from(format!("cloud-boxes-provider-{id}-login")),
                    "Log In",
                    Some(ICON_KEY),
                    if row.configured == Some(true) {
                        ButtonVariant::Ghost
                    } else {
                        ButtonVariant::Outline
                    },
                    disabled,
                    blocked.clone(),
                    move |page: &mut Self, _window, cx| {
                        page.run_terminal_command("login", &[("provider", login_id.as_str())], cx)
                    },
                    cx,
                ));
                let prepare_id = id.clone();
                let needs_login = row.configured == Some(false);
                controls.push(settings_button(
                    p,
                    SharedString::from(format!("cloud-boxes-provider-{id}-prepare")),
                    "Prepare",
                    Some(ICON_BOX),
                    variant,
                    disabled || needs_login,
                    Some(blocked.clone().unwrap_or_else(|| "Log in first.".into())),
                    move |page: &mut Self, _window, cx| {
                        page.run_terminal_command(
                            "prepare",
                            &[("provider", prepare_id.as_str())],
                            cx,
                        )
                    },
                    cx,
                ));
            }
            ProviderKind::RemoteDocker => {
                let alias = row.alias().unwrap_or_default().to_string();
                controls.push(settings_button(
                    p,
                    SharedString::from(format!("cloud-boxes-provider-{id}-check")),
                    "Check",
                    Some(ICON_TERMINAL),
                    variant,
                    disabled,
                    blocked.clone(),
                    move |page: &mut Self, _window, cx| {
                        page.run_terminal_command(
                            "remoteDockerDoctor",
                            &[("host", alias.as_str())],
                            cx,
                        )
                    },
                    cx,
                ));
            }
            ProviderKind::RemoteDockerSetup => {
                if !self.adding_server {
                    controls.push(settings_button(
                        p,
                        "cloud-boxes-add-server",
                        "Add Server",
                        Some(ICON_PLUS),
                        ButtonVariant::Outline,
                        disabled,
                        blocked.clone(),
                        |page: &mut Self, _window, cx| {
                            page.adding_server = true;
                            cx.notify();
                        },
                        cx,
                    ));
                }
            }
        }
        let icon = match row.kind {
            ProviderKind::Local => ICON_BOX,
            ProviderKind::Cloud => ICON_CLOUD,
            ProviderKind::RemoteDocker | ProviderKind::RemoteDockerSetup => ICON_SERVER,
        };
        info_row(
            p,
            &format!("provider-{}", row.id),
            Some(dot),
            icon,
            row.label.clone(),
            tooltip,
            state,
            controls,
        )
    }

    /// The Add Server form: a name for the server and how SSH reaches it, then `Add` runs
    /// `agentbox remote-docker add <name> <ssh>` in a terminal (it may ask to trust the host key).
    fn server_form(
        &mut self,
        p: &SettingsPalette,
        blocked: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let alias_id = SharedString::from(ALIAS_FIELD);
        let ssh_id = SharedString::from(SSH_FIELD);
        let alias_value = self.server_alias.clone();
        let ssh_value = self.server_ssh.clone();
        let alias_input = FieldStates::text_state(
            self,
            &alias_id,
            &alias_value,
            Some("my-server"),
            |page: &mut Self, text, _window, cx| {
                page.server_alias = text.trim().to_string();
                cx.notify();
            },
            window,
            cx,
        );
        let ssh_input = FieldStates::text_state(
            self,
            &ssh_id,
            &ssh_value,
            Some("user@host, host:port, or a name from ~/.ssh/config"),
            |page: &mut Self, text, _window, cx| {
                page.server_ssh = text.trim().to_string();
                cx.notify();
            },
            window,
            cx,
        );
        let alias = self.server_alias.clone();
        let ssh = self.server_ssh.clone();
        let alias_ok = valid_alias(&alias);
        let ssh_ok = valid_ssh_target(&ssh);
        let problem = if !alias.is_empty() && !alias_ok {
            Some("Use letters, digits, dots, dashes or underscores for the name.")
        } else if !ssh.is_empty() && !ssh_ok {
            Some("SSH takes user@host, host:port, or a name from ~/.ssh/config.")
        } else {
            None
        };
        let label = |text: &str| {
            div()
                .w(px(56.0))
                .flex_shrink_0()
                .text_size(px(13.0))
                .text_color(gpui::Hsla::from(p.muted))
                .child(text.to_string())
        };
        let cancel = settings_button(
            p,
            "cloud-boxes-add-server-cancel",
            "Cancel",
            None,
            ButtonVariant::Ghost,
            false,
            None,
            |page: &mut Self, _window, cx| {
                page.adding_server = false;
                cx.notify();
            },
            cx,
        );
        let add = settings_button(
            p,
            "cloud-boxes-add-server-confirm",
            "Add",
            Some(ICON_PLUS),
            ButtonVariant::Outline,
            blocked || !alias_ok || !ssh_ok,
            Some(if blocked {
                "Install agentbox first.".into()
            } else {
                "Fill in a name and how SSH reaches the server.".into()
            }),
            move |page: &mut Self, _window, cx| {
                page.run_terminal_command(
                    "remoteDockerAdd",
                    &[("alias", alias.as_str()), ("ssh", ssh.as_str())],
                    cx,
                );
                page.adding_server = false;
                page.server_alias.clear();
                page.server_ssh.clear();
                cx.notify();
            },
            cx,
        );
        let form = v_flex()
            .w_full()
            .gap(px(10.0))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap(px(10.0))
                    .child(label("Name"))
                    .child(settings_text_input(
                        p,
                        &alias_input,
                        None,
                        false,
                        window,
                        cx,
                    )),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap(px(10.0))
                    .child(label("SSH"))
                    .child(settings_text_input(p, &ssh_input, None, true, window, cx)),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(13.0))
                            .text_color(gpui::Hsla::from(p.destructive))
                            .children(problem),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .items_center()
                            .gap(px(8.0))
                            .child(cancel)
                            .child(add),
                    ),
            );
        div()
            .w_full()
            .px(px(20.0))
            .py(px(14.0))
            .child(form)
            .into_any_element()
    }
}
