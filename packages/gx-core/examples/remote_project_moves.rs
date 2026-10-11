//! Exercises the project moves and the Project Group / Space edits made on a REMOTE machine's tab,
//! which M4d part 2 blocker 8 took off the sidebar page.
//!
//! Usage: `cargo run --example remote_project_moves`
//!
//! It needs no recording: every case below is a command plus two documents in, a plan out. The
//! expected answers were derived by READING the shipped TypeScript, not by running it, because the
//! page that runs it is being deleted in this same milestone and the parity harness beside it
//! records only this computer's two bridges (`persistProjectCollections`, `persistSidebarSpaces`),
//! where a remote edit leaves through `post({ type: 'updateSidebarProjectCollections',
//! remoteMachineId })` instead:
//!
//! - `describeNativeSidebarMachine` (tooling/gx-core/sidebar-page-frozen/space-navigation.ts),
//!   whose `resolveProjectId` answers `remoteMachineContext.projectId`, the RAW id, on a remote
//!   machine;
//! - `saveNativeCollections` and `runNativeMembershipAction`
//!   (tooling/gx-core/sidebar-page-frozen/membership.ts) with
//!   `getRemoteProjectCollectionFamilyProjectIds`
//!   (packages/core-ui/sidebar-app/drag-drop-geometry.ts (deleted 2026-10-01));
//! - `reorderNativeSidebar` and `runNativeProjectDrop` (reorder.ts, project-drag.ts), whose order
//!   half posts the whole cross-machine `state.groupOrder` and is therefore refused by
//!   `syncWorkspaceGroupOrder`;
//! - `NativeSidebarMetadata.updateSpaces` and `adoptCollections`
//!   (tooling/gx-core/sidebar-page-frozen/metadata.ts), the second of which is where the
//!   per-machine `nextCollectionNumber` floor comes from.
//!
//! This is tooling, not a test suite; it prints what it found and fails the process on a
//! difference.

use std::process::ExitCode;

use ghostex_gx_core::{
    plan_project_move, CollectionsDocument, Core, MachineId, ProjectKey, ProjectWrite,
    SidebarInputs, SpacesDocument,
};
use serde_json::{json, Value};

const NOW_MS: u64 = 1_790_000_000_000;
const CREATE_MS: i64 = 1_790_000_000_000;
const MACHINE: &str = "remote-ab12";

/// `R01` is a worktree of `R00`, so a drag of either carries the family.
const WORKTREES: &[(&str, &str)] = &[("R01", "R00")];
const REMOTE_PROJECTS: &[&str] = &["R00", "R01", "R02", "R03"];
const LOCAL_PROJECTS: &[&str] = &["P00", "P01"];

fn main() -> ExitCode {
    let mut checks = Checks::default();
    let core = build_core();

    // 1. A drag on a remote tab is answered at all, where it used to be refused outright.
    let plan = remote_plan(&core, &move_group("R01", "R03", "before"));
    checks.some("a remote moveGroup is planned", plan.as_ref());

    // 2. The membership document it writes is keyed by the RAW project ids that machine's own
    //    daemon stores, never the scoped `remote:<machine>:project:<id>` form. This is the whole
    //    reason `resolveProjectId` exists.
    let collections = plan
        .as_ref()
        .and_then(|plan| first_collections(&plan.writes));
    checks.eq(
        "the remote membership is keyed by raw project ids",
        collection_of(collections.as_ref(), "R01"),
        Some("RC2".to_string()),
    );
    checks.eq(
        "no scoped project id reaches the remote document",
        collections
            .as_ref()
            .map(|document| document.to_storage_json().to_string().contains("remote:")),
        Some(false),
    );

    // 3. The order half is dropped: the page posted a list naming two machines and
    //    `syncWorkspaceGroupOrder` refused it, so a remote reorder has never saved an order.
    checks.eq(
        "a remote moveGroup writes no project order",
        plan.as_ref()
            .map(|plan| plan.writes.iter().any(is_group_order)),
        Some(false),
    );
    checks.eq(
        "and says so, for the host's counter",
        plan.as_ref().map(|plan| plan.dropped_group_order),
        Some(true),
    );

    // 4. The same gesture on THIS computer still posts its order. The guard on 3.
    let local = local_plan(&core, &move_group_local("P01", "P00", "after"));
    checks.eq(
        "a local moveGroup still writes its project order",
        local
            .as_ref()
            .map(|plan| plan.writes.iter().any(is_group_order)),
        Some(true),
    );
    checks.eq(
        "and is not marked as dropped",
        local.as_ref().map(|plan| plan.dropped_group_order),
        Some(false),
    );

    // 5. `moveToCollection` has no order arm at all, so a remote drop into a folder writes exactly
    //    what a local one writes.
    let into_folder = remote_plan(
        &core,
        &json!({
            "type": "moveToCollection",
            "sourceKind": "group",
            "sourceId": remote_group("R03"),
            "collectionId": "RC1",
        }),
    );
    checks.eq(
        "a remote moveToCollection writes one collections document",
        into_folder.as_ref().map(|plan| plan.writes.len()),
        Some(1),
    );
    checks.eq(
        "and files the project into the named folder",
        collection_of(
            into_folder
                .as_ref()
                .and_then(|plan| first_collections(&plan.writes))
                .as_ref(),
            "R03",
        ),
        Some("RC1".to_string()),
    );

    // 6. Add to Group > New Project Group takes the number from THAT machine's document, which is
    //    the per-machine monotonic overlay `adoptCollections` kept with its `Math.max`.
    let created = remote_plan(
        &core,
        &json!({
            "type": "projectMembership",
            "action": "createCollection",
            "groupId": remote_group("R03"),
        }),
    );
    checks.eq(
        "a remote New Project Group is numbered from that machine's document",
        created
            .as_ref()
            .and_then(|plan| first_collections(&plan.writes))
            .and_then(|document| {
                document
                    .state
                    .collections
                    .iter()
                    .find(|collection| collection.project_ids.iter().any(|id| id == "R03"))
                    .map(|collection| collection.title.clone())
            }),
        Some("Group 9".to_string()),
    );

    // 7. The Spaces submenu's New Space opens the dialog carrying the machine, because the result
    //    comes back to a host that has to know whose document to edit.
    let new_space = remote_plan(
        &core,
        &json!({
            "type": "spaceMembership",
            "projectId": "R03",
        }),
    );
    checks.eq(
        "New Space on a remote tab names the machine",
        new_space
            .as_ref()
            .and_then(|plan| plan.writes.first())
            .map(ProjectWrite::to_json)
            .map(|write| write["remoteMachineId"].clone()),
        Some(Value::from(MACHINE)),
    );
    checks.eq(
        "and the section it was opened from",
        new_space
            .as_ref()
            .and_then(|plan| plan.writes.first())
            .map(ProjectWrite::to_json)
            .map(|write| write["sectionKey"].clone()),
        Some(Value::from(format!("remote:{MACHINE}"))),
    );

    // 8. A Space tick edits that machine's Spaces document, again by raw project id.
    let ticked = remote_plan(
        &core,
        &json!({
            "type": "spaceMembership",
            "spaceId": "rs1",
            "projectId": "R03",
        }),
    );
    checks.eq(
        "a remote Space tick adds the raw project id",
        ticked
            .as_ref()
            .and_then(|plan| plan.writes.first())
            .and_then(|write| match write {
                ProjectWrite::EditSpaces { document } => document
                    .state
                    .spaces
                    .get("rs1")
                    .map(|space| space.member_project_ids.clone()),
                _ => None,
            }),
        Some(vec!["R03".to_string()]),
    );

    // 9. A tab naming a machine the store holds nothing for is still no answer at all, which is the
    //    hand-off the host counts. The refusal that went was the one on `LOCAL_MACHINE_ID`, not
    //    this one.
    let unknown = plan_project_move(
        &core,
        &inputs("remote-not-here"),
        &remote_collections(),
        Some(&remote_spaces()),
        &move_group("R01", "R03", "before"),
        CREATE_MS,
    );
    checks.eq(
        "a machine the store has not loaded is not answered",
        Some(unknown.is_none()),
        Some(true),
    );

    checks.report()
}

/// One project move on the remote tab, against that machine's two documents.
fn remote_plan(core: &Core, command: &Value) -> Option<ghostex_gx_core::ProjectMovePlan> {
    plan_project_move(
        core,
        &inputs(MACHINE),
        &remote_collections(),
        Some(&remote_spaces()),
        command,
        CREATE_MS,
    )
}

/// The same, on this computer's tab and this computer's documents.
fn local_plan(core: &Core, command: &Value) -> Option<ghostex_gx_core::ProjectMovePlan> {
    plan_project_move(
        core,
        &inputs("local"),
        &local_collections(),
        Some(&remote_spaces()),
        command,
        CREATE_MS,
    )
}

fn move_group(group: &str, target: &str, position: &str) -> Value {
    json!({
        "type": "moveGroup",
        "groupId": remote_group(group),
        "targetGroupId": remote_group(target),
        "position": position,
    })
}

fn move_group_local(group: &str, target: &str, position: &str) -> Value {
    json!({
        "type": "moveGroup",
        "groupId": ProjectKey::local(group).to_sidebar_group_id(),
        "targetGroupId": ProjectKey::local(target).to_sidebar_group_id(),
        "position": position,
    })
}

fn remote_group(project_id: &str) -> String {
    ProjectKey::remote(MACHINE, project_id).to_sidebar_group_id()
}

fn is_group_order(write: &ProjectWrite) -> bool {
    matches!(write, ProjectWrite::GroupOrder { .. })
}

fn first_collections(writes: &[ProjectWrite]) -> Option<CollectionsDocument> {
    writes.iter().find_map(|write| match write {
        ProjectWrite::EditCollections { document } => Some(document.clone()),
        _ => None,
    })
}

fn collection_of(document: Option<&CollectionsDocument>, project_id: &str) -> Option<String> {
    document?
        .state
        .collections
        .iter()
        .find(|collection| collection.project_ids.iter().any(|id| id == project_id))
        .map(|collection| collection.collection_id.clone())
}

fn inputs(selected_machine_id: &str) -> SidebarInputs {
    let mut inputs = SidebarInputs::default();
    inputs.ui.selected_machine_id = selected_machine_id.to_string();
    inputs.settings.sidebar_spaces_enabled = true;
    inputs
}

/// The remote machine's own collections document: raw project ids, and a counter this app has never
/// touched, so a create has to take the number from here.
fn remote_collections() -> CollectionsDocument {
    CollectionsDocument::from_echo_json(&json!({
        "collections": {
            "RC1": { "collectionId": "RC1", "title": "Group 1", "color": "#7c6df2", "projectIds": ["R00"] },
            "RC2": { "collectionId": "RC2", "title": "Group 2", "color": "#3aa675", "projectIds": ["R03"] },
        },
        "order": ["RC1", "RC2"],
        "nextCollectionNumber": 9,
    }))
    .expect("the remote collections fixture parses")
}

/// This computer's, which shares no id with the one above so a document mix-up cannot pass.
fn local_collections() -> CollectionsDocument {
    CollectionsDocument::from_storage_json(&json!({
        "collections": [
            { "collectionId": "LC1", "title": "Group 1", "color": "#7c6df2", "projectIds": ["P00"] },
        ],
        "nextCollectionNumber": 2,
    }))
}

fn remote_spaces() -> SpacesDocument {
    SpacesDocument::from_echo_json(&json!({
        "order": ["rs1"],
        "spaces": {
            "rs1": {
                "spaceId": "rs1", "name": "Remote", "color": "#3f8fc7", "icon": "stack",
                "memberProjectIds": [], "memberCollectionIds": [],
            },
        },
    }))
    .expect("the remote Spaces fixture parses")
}

/// This computer plus one remote machine, both with projects, because the remote section must be
/// built from the REMOTE machine's presentation and not from whatever this computer holds.
fn build_core() -> Core {
    let mut core = Core::default();
    core.handle_raw_frame(
        MachineId::Local,
        &snapshot_frame(LOCAL_PROJECTS).to_string(),
        NOW_MS,
    )
    .expect("the local frame parses");
    core.handle_raw_frame(
        MachineId::Remote(MACHINE.to_string()),
        &snapshot_frame(REMOTE_PROJECTS).to_string(),
        NOW_MS,
    )
    .expect("the remote frame parses");
    core
}

fn snapshot_frame(project_ids: &[&str]) -> Value {
    json!({
        "type": "presentationSnapshot",
        "protocolVersion": ghostex_gx_core::protocol::GXSERVER_PROTOCOL_VERSION,
        "serverId": "fixture",
        "revision": 1,
        "snapshot": {
            "revision": 1,
            "generatedAt": "2026-09-21T00:00:00.000Z",
            "projects": project_ids.iter().map(|id| project(id)).collect::<Vec<_>>(),
            "groups": project_ids.iter().map(|id| group(id)).collect::<Vec<_>>(),
            "sessions": project_ids
                .iter()
                .flat_map(|id| ["A", "B"].map(|session_id| session(id, session_id)))
                .collect::<Vec<_>>(),
        },
    })
}

fn project(project_id: &str) -> Value {
    let mut value = json!({
        "projectId": project_id,
        "title": project_id,
        "path": format!("/tmp/{project_id}"),
        "pathState": "available",
        "groupIds": [format!("{project_id}:active")],
        "sortKey": format!("1:{project_id}"),
        "createdAt": "2026-06-29T13:10:42.091Z",
        "updatedAt": "2026-09-15T01:18:43.055Z",
    });
    if let Some((_, parent)) = WORKTREES.iter().find(|(child, _)| *child == project_id) {
        value["worktree"] = json!({
            "branch": format!("wt/{project_id}"),
            "name": project_id,
            "parentProjectId": parent,
            "parentProjectName": parent,
            "parentProjectPath": format!("/tmp/{parent}"),
        });
    }
    value
}

fn group(project_id: &str) -> Value {
    json!({
        "groupId": format!("{project_id}:active"),
        "projectId": project_id,
        "title": "Active",
        "sessionIds": ["A", "B"],
        "sortKey": format!("1:{project_id}:active"),
    })
}

fn session(project_id: &str, session_id: &str) -> Value {
    json!({
        "sessionId": session_id,
        "projectId": project_id,
        "groupId": format!("{project_id}:active"),
        "kind": "agent",
        "surface": "workspace",
        "zmxName": format!("S90-{project_id}-{session_id}"),
        "sortKey": format!("000000000000:0:2:{project_id}:{session_id}"),
        "visibleInSidebarByDefault": true,
        "isPinned": false,
        "alias": format!("Session {project_id} {session_id}"),
        "activity": "idle",
        "lifecycleState": "running",
        "lastInteractionAt": "2026-09-15T01:18:43.055Z",
        "createdAt": "2026-09-15T01:00:00.000Z",
        "updatedAt": "2026-09-15T01:18:43.055Z",
    })
}

/// The checks, printed in order, with the process failing on the first difference recorded.
#[derive(Default)]
struct Checks {
    failures: usize,
    total: usize,
}

impl Checks {
    fn eq<T: std::fmt::Debug + PartialEq>(&mut self, what: &str, found: T, expected: T) {
        self.total += 1;
        if found == expected {
            println!("ok   {what}");
            return;
        }
        self.failures += 1;
        println!("FAIL {what}\n       expected {expected:?}\n       found    {found:?}");
    }

    fn some<T>(&mut self, what: &str, value: Option<&T>) {
        self.total += 1;
        match value.is_some() {
            true => println!("ok   {what}"),
            false => {
                self.failures += 1;
                println!("FAIL {what}\n       expected a plan, found none");
            }
        }
    }

    fn report(self) -> ExitCode {
        println!(
            "remote project moves: {} checks, {} failures",
            self.total, self.failures
        );
        match self.failures {
            0 => ExitCode::SUCCESS,
            _ => ExitCode::FAILURE,
        }
    }
}
