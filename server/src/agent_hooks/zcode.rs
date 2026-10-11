use std::{fs, path::Path, process::Command, time::Duration};

use serde_json::Value;

use crate::{domain::DomainStateError, platform::shell::command_shell};

#[cfg(not(windows))]
use super::probing::normalize_gxserver_process_path;
use super::{config::HookPaths, probing::run_command_stdout_with_timeout};

/// CDXC:AgentHooks 2026-09-15 WHY:
/// ZCode only creates its model defaults and setup marker when config.json does not exist.
/// Writing a hooks-only file first makes its runtime exit before /login is available.
/// Let the public launcher initialize its own defaults before merging Ghostex hooks.
pub(crate) fn ensure_zcode_config(
    path: &Path,
    hook_paths: &HookPaths,
) -> Result<(), DomainStateError> {
    ensure_initialized(path, || {
        let shell = command_shell();
        let mut command = Command::new(&shell.executable);
        command.args(shell.profileless_script_args("zcode --help"));
        command
            .current_dir(&hook_paths.home_dir)
            .env("HOME", &hook_paths.home_dir)
            .env("USERPROFILE", &hook_paths.home_dir)
            .env("NO_UPDATE_NOTIFIER", "1")
            .env("ZCODE_DISABLE_UPDATE_CHECK", "1");
        #[cfg(not(windows))]
        command.env(
            "PATH",
            normalize_gxserver_process_path(
                std::env::var("PATH").ok().as_deref(),
                &hook_paths.home_dir,
            ),
        );
        let _ = run_command_stdout_with_timeout(command, Duration::from_secs(10));
    })
}

fn ensure_initialized(path: &Path, bootstrap: impl FnOnce()) -> Result<(), DomainStateError> {
    if !path.exists() {
        bootstrap();
    }
    let initialized = fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .is_some_and(|config| {
            config
                .pointer("/model/main")
                .and_then(Value::as_str)
                .is_some_and(|model| !model.trim().is_empty())
        });
    if initialized {
        Ok(())
    } else {
        Err(DomainStateError::bad_request(format!(
            "ZCode's model defaults are missing from {}. Initialize or repair ZCode's configuration before installing hooks.",
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstraps_missing_config_but_never_replaces_existing_settings() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.json");
        let defaults = r#"{"model":{"main":"zai/glm-5.2"},"provider":{"zai":{"options":{}}}}"#;
        ensure_initialized(&path, || fs::write(&path, defaults).unwrap()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), defaults);
        ensure_initialized(&path, || panic!("existing config must not be bootstrapped")).unwrap();
        fs::write(&path, r#"{"hooks":{"enabled":true}}"#).unwrap();
        assert!(
            ensure_initialized(&path, || panic!("partial config must not be replaced")).is_err()
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            r#"{"hooks":{"enabled":true}}"#
        );
    }

    #[test]
    fn failed_bootstrap_does_not_create_a_hooks_only_config() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.json");
        assert!(ensure_initialized(&path, || {}).is_err());
        assert!(!path.exists());
    }
}
