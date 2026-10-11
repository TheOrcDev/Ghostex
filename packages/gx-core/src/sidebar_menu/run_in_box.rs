//! The agent launcher's Run in a Box pages: the row on the agent list, the page of ready box
//! locations behind it, and the page of agents one location can run.
//!
//! CDXC:AgentBox 2026-10-01 WHY:
//! The pages are the launcher's own (`agentAccounts` commands answered by `LauncherAccounts`, a
//! back row carrying the launcher style), like the account page, so they keep the launcher's look
//! and its back navigation; a box launch is `projectAction: agent` with a `runLocation` and no
//! account, because the box signs in on its own. Only this computer's projects offer the pages:
//! the locations are the ones this computer's gxserver reported ready.
//!
//! SEE-ALSO: packages/gx-core/src/agentbox.rs, packages/gx-core/src/sidebar_accounts/launcher.rs.

use crate::agentbox::{agentbox_agent_family, AgentboxLocation};
use crate::keys::ProjectKey;

use super::agent_logos::colored_agent_logo;
use super::commands::MenuCommand;
use super::host::{LauncherAgent, MenuHost};
use super::item::MenuItem;

const RUN_IN_BOX_LABEL: &str = "Run in a Box";

/// The agents a box can run, in launcher order.
fn box_agents(host: &MenuHost) -> impl Iterator<Item = &LauncherAgent> {
    host.agents
        .iter()
        .filter(|agent| agentbox_agent_family(&agent.agent_id, agent.icon.as_deref()).is_some())
}

fn offers_boxes(group_id: &str, host: &MenuHost) -> bool {
    let local = ProjectKey::parse_sidebar_group_id(group_id)
        .is_none_or(|project| project.machine.is_local());
    local && !host.agentbox_locations.is_empty() && box_agents(host).next().is_some()
}

/// The agent list's Run in a Box row, when this computer has a ready box and an agent to run in it.
pub(crate) fn run_in_box_row(group_id: &str, host: &MenuHost) -> Option<MenuItem> {
    offers_boxes(group_id, host).then(|| MenuItem {
        label: Some(RUN_IN_BOX_LABEL.to_string()),
        icon: Some("box".to_string()),
        keep_open: true,
        command: Some(MenuCommand::agent_box_page(group_id, "box", None)),
        ..MenuItem::default()
    })
}

/// A page's first row: back to `action` (the agent list or the locations page).
fn back_row(group_id: &str, label: &str, action: &str) -> MenuItem {
    MenuItem {
        label: Some(label.to_string()),
        icon: Some("chevron-left".to_string()),
        agent_launcher: true,
        keep_open: true,
        command: Some(MenuCommand::agent_box_page(group_id, action, None)),
        ..MenuItem::default()
    }
}

fn hint(label: &str) -> MenuItem {
    MenuItem {
        label: Some(label.to_string()),
        disabled: true,
        ..MenuItem::default()
    }
}

/// The ready box locations, each opening its agents page.
pub(crate) fn run_in_box_locations_page(group_id: &str, host: &MenuHost) -> Vec<MenuItem> {
    let mut items = vec![back_row(group_id, RUN_IN_BOX_LABEL, "root")];
    if !offers_boxes(group_id, host) {
        items.push(hint(
            "No box is ready on this computer. Set one up in Settings > Cloud Boxes.",
        ));
        return items;
    }
    items.extend(host.agentbox_locations.iter().map(|location| MenuItem {
        label: Some(location.label.clone()),
        icon: Some(location.icon().to_string()),
        keep_open: true,
        command: Some(MenuCommand::agent_box_page(
            group_id,
            "boxAgents",
            Some(&location.run_location()),
        )),
        ..MenuItem::default()
    }));
    items
}

/// The agents `run_location` can run, each launching in that box.
pub(crate) fn run_in_box_agents_page(
    group_id: &str,
    host: &MenuHost,
    run_location: &str,
) -> Vec<MenuItem> {
    let location: Option<&AgentboxLocation> = host
        .agentbox_locations
        .iter()
        .find(|location| location.run_location() == run_location);
    let Some(location) = location.filter(|_| offers_boxes(group_id, host)) else {
        return run_in_box_locations_page(group_id, host);
    };
    let mut items = vec![back_row(group_id, &location.label, "box")];
    items.extend(box_agents(host).map(|agent| {
        let icon = agent.icon.as_deref();
        MenuItem {
            label: Some(agent.name.clone()),
            agent_icon: icon.map(str::to_string),
            image_data_url: icon.and_then(colored_agent_logo).map(str::to_string),
            icon: icon.is_none().then(|| "code".to_string()),
            command: Some(MenuCommand::agent_run_in_box(
                group_id,
                &agent.agent_id,
                run_location,
            )),
            ..MenuItem::default()
        }
    }));
    items.push(hint(
        "Runs in its own box with its own sign-in. Opens in the terminal.",
    ));
    items
}
