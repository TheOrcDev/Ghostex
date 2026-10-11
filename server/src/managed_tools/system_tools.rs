//! The basic Linux tools agent installers assume: curl, CA certificates, unzip and git.

use std::{path::Path, time::Duration};

use super::{
    jobs::Log,
    platform::{self, Os},
    run,
    tools::{self, Status},
};

/// (command or marker, package name)
const PACKAGES: [(&str, &str); 4] = [
    ("curl", "curl"),
    ("ca-certificates", "ca-certificates"),
    ("unzip", "unzip"),
    ("git", "git"),
];

const CERTIFICATE_BUNDLES: [&str; 5] = [
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/ssl/ca-bundle.pem",
    "/etc/ssl/cert.pem",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
];

/// The packages of `wanted` (commands, plus "ca-certificates") this computer lacks.
pub(crate) fn missing(home: &Path, wanted: &[&str]) -> Vec<&'static str> {
    if platform::os() != Os::Linux {
        return Vec::new();
    }
    PACKAGES
        .iter()
        .filter(|(name, _)| wanted.contains(name))
        .filter(|(name, _)| {
            if *name == "ca-certificates" {
                !CERTIFICATE_BUNDLES
                    .iter()
                    .any(|path| Path::new(path).is_file())
            } else {
                tools::locate(name, home).is_none()
            }
        })
        .map(|(_, package)| *package)
        .collect()
}

fn all_missing(home: &Path) -> Vec<&'static str> {
    missing(home, &PACKAGES.map(|(name, _)| name))
}

fn package_command(packages: &[&str]) -> Option<String> {
    let list = packages.join(" ");
    let has = |name: &str| {
        ["/usr/bin", "/bin", "/usr/sbin", "/sbin"]
            .iter()
            .any(|dir| Path::new(dir).join(name).is_file())
    };
    Some(if has("apt-get") {
        format!("apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y {list}")
    } else if has("dnf") {
        format!("dnf install -y {list}")
    } else if has("yum") {
        format!("yum install -y {list}")
    } else if has("zypper") {
        format!("zypper --non-interactive install {list}")
    } else if has("pacman") {
        format!("pacman -Sy --noconfirm --needed {list}")
    } else if has("apk") {
        format!("apk add --no-cache {list}")
    } else {
        return None;
    })
}

/// `pkexec` shows the desktop's own password dialog; WSL and headless sessions have no dialog to
/// show, so their install runs in a terminal where `sudo` can ask.
pub(crate) fn can_install_in_background() -> bool {
    platform::os() == Os::Linux
        && !platform::is_wsl()
        && ["/usr/bin/pkexec", "/bin/pkexec"]
            .iter()
            .any(|path| Path::new(path).is_file())
        && (std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some())
}

/// The terminal command for computers without a password dialog.
pub(crate) fn terminal_command(home: &Path) -> Option<String> {
    let packages = all_missing(home);
    if packages.is_empty() {
        return None;
    }
    package_command(&packages).map(|command| format!("sudo sh -c '{command}'"))
}

pub(crate) fn status(home: &Path) -> Status {
    if platform::os() != Os::Linux {
        return Status::unsupported("System tools are installed by Ghostex only on Linux.".into());
    }
    let missing = all_missing(home);
    let command = package_command(if missing.is_empty() {
        &["curl"]
    } else {
        &missing
    });
    let background = can_install_in_background();
    let manager = command
        .as_deref()
        .and_then(|command| command.split_whitespace().next())
        .map(|word| if word == "apt-get" { "apt-get" } else { word })
        .unwrap_or("your package manager");
    let mut status = Status::new(if background {
        format!(
            "Installs {} with {manager}. Your computer asks for your password once.",
            missing_list(&missing)
        )
    } else {
        format!(
            "Opens a terminal that installs {} with sudo {manager}; type your password there.",
            missing_list(&missing)
        )
    });
    status.needs_password = true;
    if missing.is_empty() {
        status.executable = tools::locate("curl", home).map(|(path, _)| path);
        status.source = Some(tools::Source::System);
        status.detail = Some("curl, certificates, unzip and git are installed.".into());
    } else {
        status.detail = Some(format!("Missing: {}.", missing_list(&missing)));
        status.operations = vec!["install"];
        match command {
            None => {
                status.install_blocker = Some(format!(
                    "Ghostex doesn't know this Linux's package manager. Install {} yourself, then refresh.",
                    missing_list(&missing)
                ))
            }
            Some(_) if !background => status.terminal_command = terminal_command(home),
            Some(_) => {}
        }
    }
    status
}

fn missing_list(packages: &[&str]) -> String {
    packages
        .iter()
        .map(|package| {
            if *package == "ca-certificates" {
                "certificates"
            } else {
                package
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// CDXC:ManagedTools 2026-09-29 DECISION:
/// User (7A): when Linux lacks the basics agent installers assume (curl, CA certificates, unzip, git), one "Install missing system tools" click installs them with a single password prompt through the distribution's package manager. The prompt is polkit's (`pkexec`) as for Turn on SSH access; where there is none (WSL, no desktop session) the same command runs in a terminal where `sudo` asks.
pub(crate) fn install(home: &Path, log: &Log) -> Result<(), String> {
    let missing = all_missing(home);
    if missing.is_empty() {
        log.line("curl, certificates, unzip and git are already installed.");
        return Ok(());
    }
    let command =
        package_command(&missing).ok_or("Ghostex doesn't know this Linux's package manager.")?;
    if !can_install_in_background() {
        return Err(format!(
            "Run this in a terminal to install {}: sudo sh -c '{command}'",
            missing_list(&missing)
        ));
    }
    log.line(&format!(
        "Installing {} (asks for your password).",
        missing_list(&missing)
    ));
    let result = run::run(
        "pkexec",
        &["--disable-internal-agent", "/bin/sh", "-c", &command],
        &[],
        &[],
        log,
        Duration::from_secs(20 * 60),
    );
    if let Err(error) = result {
        return Err(if error.contains("126") {
            "The password prompt was cancelled.".to_string()
        } else if error.contains("127") {
            format!(
                "The administrator prompt did not authorize the install. If no password dialog appeared, run this in a terminal: sudo sh -c '{command}'"
            )
        } else {
            error
        });
    }
    crate::agent_hooks::probing::refresh_cli_environment(home);
    Ok(())
}
