//! Linux remote credentials use the same system keyring backend as GPUI.

use crate::app::helpers::*;

const LABEL: &str = "Ghostex Remote";

/// CDXC:RemoteMachines 2026-09-23 WHY:
/// Connect and SSH askpass run on worker threads without a GPUI App context. Use GPUI's existing oo7 backend here so Linux passwords and daemon tokens remain in the desktop keyring rather than settings or temporary files.
fn save_remote_secret(machine_id: &str, kind: &str, secret: &str) -> anyhow::Result<()> {
    futures::executor::block_on(async {
        let keyring = oo7::Keyring::new().await?;
        keyring.unlock().await?;
        let attributes = [
            ("application", "ghostex"),
            ("remote-machine", machine_id),
            ("credential", kind),
        ];
        if secret.is_empty() {
            for item in keyring.search_items(&attributes).await? {
                item.delete().await?;
            }
        } else {
            keyring
                .create_item(LABEL, &attributes, secret.as_bytes(), true)
                .await?;
        }
        Ok(())
    })
}

pub(crate) fn gpui_save_remote_machine_password_to_keychain(
    remote_machine_id: &str,
    password: &str,
) -> GpuiRemoteSshPasswordKeychainResult {
    match save_remote_secret(remote_machine_id, "ssh-password", password) {
        Ok(()) => GpuiRemoteSshPasswordKeychainResult::Success,
        Err(_) => GpuiRemoteSshPasswordKeychainResult::Failed,
    }
}

pub(crate) fn gpui_save_remote_gxserver_token_to_keychain(
    remote_machine_id: &str,
    token: &str,
) -> GpuiRemoteTokenKeychainResult {
    match save_remote_secret(remote_machine_id, "gxserver-token", token) {
        Ok(()) => GpuiRemoteTokenKeychainResult::Success,
        Err(_) => GpuiRemoteTokenKeychainResult::Failed,
    }
}

pub(crate) fn gpui_read_remote_ssh_password_from_keychain(
    remote_machine_id: &str,
) -> Result<Vec<u8>, String> {
    let result: anyhow::Result<Vec<u8>> = futures::executor::block_on(async {
        let keyring = oo7::Keyring::new().await?;
        keyring.unlock().await?;
        let attributes = [
            ("application", "ghostex"),
            ("remote-machine", remote_machine_id),
            ("credential", "ssh-password"),
        ];
        for item in keyring.search_items(&attributes).await? {
            item.unlock().await?;
            let secret = item.secret().await?;
            if !secret.is_empty() {
                return Ok(secret.to_vec());
            }
        }
        anyhow::bail!("SSH password not found")
    });
    result.map_err(|_| {
        "Could not read the saved SSH password from secure system storage.".to_string()
    })
}
