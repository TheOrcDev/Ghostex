//! The Actions page's data: the hydrate's `SidebarCommandButton`s, the editor draft
//! (`SettingsCommandDraft`), the title rules and the draft order kept while a reorder is in
//! flight (packages/core-ui/settings-modal/tabs/actions.tsx (deleted 2026-10-01) and drag-data.ts).
use serde_json::{Map, Value, json};

/// `SettingsCommandScope`.
///
/// CDXC:AgentLauncher 2026-08-01 SEE-ALSO: Global Actions (gxserver) and Project Actions (project metadata) are one implementation that differs only in the bridge message types and the copy (packages/core-ui/settings-modal/tabs/actions.tsx (deleted 2026-10-01)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CommandScope {
    Global,
    Project,
}

impl CommandScope {
    pub(crate) fn list_id(self) -> &'static str {
        match self {
            CommandScope::Global => "settings-global-actions",
            CommandScope::Project => "settings-actions",
        }
    }

    /// `createSettingsReorderRequestId`'s kind.
    fn reorder_kind(self) -> &'static str {
        match self {
            CommandScope::Global => "globalActions",
            CommandScope::Project => "actions",
        }
    }

    pub(crate) fn save_type(self) -> &'static str {
        match self {
            CommandScope::Global => "saveGlobalSidebarCommand",
            CommandScope::Project => "saveSidebarCommand",
        }
    }

    pub(crate) fn delete_type(self) -> &'static str {
        match self {
            CommandScope::Global => "deleteGlobalSidebarCommand",
            CommandScope::Project => "deleteSidebarCommand",
        }
    }

    pub(crate) fn order_type(self) -> &'static str {
        match self {
            CommandScope::Global => "syncGlobalSidebarCommandOrder",
            CommandScope::Project => "syncSidebarCommandOrder",
        }
    }
}

/// `SidebarActionType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionType {
    Terminal,
    Browser,
}

impl ActionType {
    pub(crate) fn parse(value: &str) -> Self {
        if value == "browser" {
            ActionType::Browser
        } else {
            ActionType::Terminal
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            ActionType::Terminal => "terminal",
            ActionType::Browser => "browser",
        }
    }
}

/// `SidebarCommandLink`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandLink {
    /// `integrated` or `external`.
    pub(crate) target: String,
    pub(crate) url: String,
}

/// `SidebarCommandButton` as the hydrate carries it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CommandButton {
    pub(crate) action_type: ActionType,
    pub(crate) close_terminal_on_exit: bool,
    pub(crate) command: Option<String>,
    pub(crate) command_id: String,
    pub(crate) icon: Option<String>,
    pub(crate) links: Option<Vec<CommandLink>>,
    pub(crate) name: String,
    pub(crate) play_completion_sound: bool,
    pub(crate) show_on_project_row: bool,
    pub(crate) url: Option<String>,
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool) == Some(true)
}

impl CommandButton {
    pub(crate) fn parse(value: &Value) -> Option<Self> {
        let command_id = text(value, "commandId").filter(|id| !id.is_empty())?;
        Some(Self {
            action_type: ActionType::parse(&text(value, "actionType").unwrap_or_default()),
            close_terminal_on_exit: flag(value, "closeTerminalOnExit"),
            command: text(value, "command"),
            command_id,
            icon: text(value, "icon"),
            links: value.get("links").and_then(Value::as_array).map(|links| {
                links
                    .iter()
                    .map(|link| CommandLink {
                        target: if text(link, "target").as_deref() == Some("external") {
                            "external".to_string()
                        } else {
                            "integrated".to_string()
                        },
                        url: text(link, "url").unwrap_or_default(),
                    })
                    .collect()
            }),
            name: text(value, "name").unwrap_or_default(),
            play_completion_sound: flag(value, "playCompletionSound"),
            show_on_project_row: flag(value, "showOnProjectRow"),
            url: text(value, "url"),
        })
    }

    /// `isSidebarCommandConfigured`.
    pub(crate) fn is_configured(&self) -> bool {
        match self.action_type {
            ActionType::Browser => self
                .url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty()),
            ActionType::Terminal => self
                .command
                .as_deref()
                .is_some_and(|command| !command.trim().is_empty()),
        }
    }

    /// `getActionTarget`: the first line of the command or URL.
    fn target(&self) -> Option<String> {
        let target = match self.action_type {
            ActionType::Browser => self.url.as_deref(),
            ActionType::Terminal => self.command.as_deref(),
        }?
        .trim();
        if target.is_empty() {
            return None;
        }
        target
            .split('\n')
            .next()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
    }

    /// `getActionTitle`.
    pub(crate) fn title(&self) -> String {
        let name = self.name.trim();
        if !name.is_empty() {
            return name.to_string();
        }
        self.target()
            .unwrap_or_else(|| "Untitled Action".to_string())
    }

    /// `getActionMeta`.
    pub(crate) fn meta(&self) -> String {
        let label = match self.action_type {
            ActionType::Browser => "Browser",
            ActionType::Terminal => "Terminal",
        };
        match self.target() {
            Some(target) => format!("{label} - {target}"),
            None => format!("{label} - Not configured"),
        }
    }

    /// `getSettingsCommandButtonTitle`.
    pub(crate) fn settings_title(&self) -> String {
        if let Some(name) = normalize_title(&self.name) {
            return name;
        }
        let target = self.command.as_deref().or(self.url.as_deref());
        target
            .and_then(normalize_title)
            .map(|target| target.chars().take(20).collect())
            .unwrap_or_default()
    }
}

/// The hud's command list at `key` (`commands` or `globalCommands`).
pub(crate) fn commands_from_hud(hud: Option<&Value>, key: &str) -> Vec<CommandButton> {
    hud.and_then(|hud| hud.get(key))
        .and_then(Value::as_array)
        .map(|commands| commands.iter().filter_map(CommandButton::parse).collect())
        .unwrap_or_default()
}

/// `normalizeSettingsCommandTitle`: trimmed, runs of whitespace collapsed.
pub(crate) fn normalize_title(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    (!normalized.is_empty()).then_some(normalized)
}

/// `getSettingsCommandTitleKey`.
pub(crate) fn title_key(value: &str) -> String {
    normalize_title(value)
        .map(|title| title.to_lowercase())
        .unwrap_or_default()
}

/// `getSettingsCommandDraftTitle`.
pub(crate) fn draft_title(action_type: ActionType, command: &str, name: &str, url: &str) -> String {
    if let Some(name) = normalize_title(name) {
        return name;
    }
    let target = match action_type {
        ActionType::Browser => url,
        ActionType::Terminal => command,
    };
    normalize_title(target)
        .map(|target| target.chars().take(20).collect())
        .unwrap_or_default()
}

/// `mergeIds`.
pub(crate) fn merge_ids(draft: &[String], synced: &[String]) -> Vec<String> {
    let mut merged: Vec<String> = draft
        .iter()
        .filter(|id| synced.contains(id))
        .cloned()
        .collect();
    for id in synced {
        if !merged.contains(id) {
            merged.push(id.clone());
        }
    }
    merged
}

/// `reconcileDraftIds`: the draft order while it still differs from the synced one.
pub(crate) fn reconcile_draft_ids(
    draft: Option<&[String]>,
    synced: &[String],
) -> Option<Vec<String>> {
    let draft = draft?;
    let next = merge_ids(draft, synced);
    (next != synced).then_some(next)
}

/// `useSettingsCommandOrder`: the commands in the draft order when one is pending.
pub(crate) fn ordered_commands(
    commands: &[CommandButton],
    draft: Option<&[String]>,
) -> Vec<CommandButton> {
    let synced: Vec<String> = commands
        .iter()
        .map(|command| command.command_id.clone())
        .collect();
    let ids = match draft {
        Some(draft) => merge_ids(draft, &synced),
        None => synced,
    };
    ids.iter()
        .filter_map(|id| {
            commands
                .iter()
                .find(|command| &command.command_id == id)
                .cloned()
        })
        .collect()
}

/// `createSettingsReorderRequestId`.
pub(crate) fn reorder_request_id(scope: CommandScope) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let millis = now.as_millis();
    let mut noise = (now.as_nanos() as u64) ^ 0x2545_f491_4f6c_dd1d;
    let mut random = String::new();
    for _ in 0..6 {
        random.push(std::char::from_digit((noise % 36) as u32, 36).unwrap_or('0'));
        noise /= 36;
    }
    format!(
        "settings-{}-{}-{random}",
        scope.reorder_kind(),
        base36(millis)
    )
}

fn base36(mut value: u128) -> String {
    let mut digits = Vec::new();
    loop {
        digits.push(std::char::from_digit((value % 36) as u32, 36).unwrap_or('0'));
        value /= 36;
        if value == 0 {
            break;
        }
    }
    digits.iter().rev().collect()
}

/// `SettingsCommandDraft` as it is saved (`getDraft()` of the editor), in the save message.
#[allow(clippy::too_many_arguments)]
pub(crate) fn save_message(
    scope: CommandScope,
    action_type: ActionType,
    close_terminal_on_exit: bool,
    command: &str,
    command_id: Option<&str>,
    icon: &str,
    links: &[CommandLink],
    name: &str,
    play_completion_sound: bool,
    show_on_project_row: bool,
    url: &str,
) -> Value {
    let terminal = action_type == ActionType::Terminal;
    let mut message = Map::new();
    message.insert("actionType".into(), json!(action_type.id()));
    message.insert(
        "closeTerminalOnExit".into(),
        json!(terminal && close_terminal_on_exit),
    );
    if terminal {
        message.insert("command".into(), json!(command.trim()));
    }
    if let Some(command_id) = command_id {
        message.insert("commandId".into(), json!(command_id));
    }
    message.insert("icon".into(), json!(icon));
    if terminal {
        let links: Vec<Value> = links
            .iter()
            .map(|link| json!({ "target": link.target, "url": link.url.trim() }))
            .filter(|link| {
                link.get("url")
                    .and_then(Value::as_str)
                    .is_some_and(|url| !url.is_empty())
            })
            .collect();
        message.insert("links".into(), Value::Array(links));
    }
    message.insert("name".into(), json!(name.trim()));
    message.insert(
        "playCompletionSound".into(),
        json!(terminal && play_completion_sound),
    );
    message.insert("showOnProjectRow".into(), json!(show_on_project_row));
    if !terminal {
        message.insert("url".into(), json!(url.trim()));
    }
    message.insert("type".into(), json!(scope.save_type()));
    Value::Object(message)
}
