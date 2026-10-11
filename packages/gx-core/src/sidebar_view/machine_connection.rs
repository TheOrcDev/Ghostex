//! What a remote machine's connection state looks like in the sidebar: whether its tab spins or
//! turns red, and the notice its list draws while it is not connected.
//!
//! CDXC:RemoteMachines 2026-10-02 DECISION:
//! User: a remote machine that cannot connect showed an empty tab with a grey cloud and no message, which is bad UX. Every state that is not connected, connecting or plain `disconnected` is a failure (the cloud turns red, as the React sidebar's `isFailure` did), and the machine's list says why with Reconnect and Remote Settings buttons instead of drawing nothing.
//!
//! SEE-ALSO: apps/desktop/src/app/helpers/remote/types.rs (`GpuiRemoteGxserverConnectState`, the
//! wire words, and the terminal overlay that reads the same copy),
//! apps/desktop/src/app/native_sidebar/machines.rs and machine_notice.rs (the renderers).

use super::inputs::{MachineTabInput, MACHINE_STATE_CONNECTED};
use super::view::MachineNotice;

/// A connect attempt is running for the machine.
pub fn machine_state_is_busy(state: &str) -> bool {
    matches!(
        state,
        "connecting" | "installing" | "downloadingRemoteServerPackage"
    )
}

/// The machine's last connect attempt failed (`sshFailed`, `authFailed`, `tunnelFailed`, …).
pub fn machine_state_is_failure(state: &str) -> bool {
    !machine_state_is_busy(state) && !matches!(state, MACHINE_STATE_CONNECTED | "disconnected")
}

/// The title and the default explanation of a connection state, shared by the sidebar notice and
/// the terminal overlay of a remote session so the two never disagree about one machine.
pub fn machine_state_copy(state: &str) -> (&'static str, Option<&'static str>) {
    match state {
        "installing" => ("Installing gxserver…", None),
        "downloadingRemoteServerPackage" => ("Downloading server package…", None),
        "disconnected" => ("Not connected", None),
        "installApprovalRequired" => (
            "Remote setup needed",
            Some("Approve the gxserver install for this machine."),
        ),
        "installFailed" => (
            "Remote setup failed",
            Some("Reconnect the machine to try again."),
        ),
        "authFailed" => (
            "Cannot sign in to machine",
            Some("The machine rejected the saved SSH username, password or key."),
        ),
        "sshFailed" => ("Cannot reach machine", Some("SSH connection failed.")),
        "tunnelFailed" => ("Cannot reach machine", Some("SSH tunnel failed.")),
        "keychainFailed" => (
            "Cannot reach machine",
            Some("Saved credentials are unavailable."),
        ),
        "tokenUnavailable" => (
            "Cannot reach machine",
            Some("The remote gxserver token is unavailable."),
        ),
        "presentationSubscribeFailed" | "presentationStreamFailed" => (
            "Reconnecting to machine…",
            Some("The remote session stream dropped."),
        ),
        "unsupported" | "unsupportedRemotePlatform" => (
            "Machine unsupported",
            Some("This remote platform cannot host gxserver."),
        ),
        "invalid" => (
            "Machine unavailable",
            Some("The saved machine settings are incomplete."),
        ),
        "failed" => (
            "Cannot reach machine",
            Some("Reconnect the machine to try again."),
        ),
        _ => ("Connecting to machine…", None),
    }
}

/// The notice of the selected machine's list: none for this computer or a connected machine.
/// A failure explains itself with the host's sanitized reason when it has one.
pub(crate) fn machine_notice(machine: Option<&MachineTabInput>) -> Option<MachineNotice> {
    let machine = machine.filter(|machine| !machine.is_local() && !machine.is_connected())?;
    let (title, detail) = machine_state_copy(&machine.state);
    let failed = machine_state_is_failure(&machine.state);
    Some(MachineNotice {
        machine_id: machine.machine_id.clone(),
        title: title.to_string(),
        detail: if failed {
            machine.message.clone().or(detail.map(str::to_string))
        } else {
            detail.map(str::to_string)
        },
        busy: machine_state_is_busy(&machine.state),
        failed,
    })
}
