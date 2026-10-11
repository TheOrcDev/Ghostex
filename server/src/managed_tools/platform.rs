//! The operating system and CPU the tools are downloaded for.

use std::process::Stdio;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Os {
    Mac,
    Linux,
    Windows,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Arch {
    X64,
    Arm64,
}

pub(crate) fn os() -> Os {
    if cfg!(target_os = "macos") {
        Os::Mac
    } else if cfg!(windows) {
        Os::Windows
    } else {
        Os::Linux
    }
}

/// The machine's CPU. On a Mac an Intel gxserver running under Rosetta still reports Apple silicon,
/// so downloads match the hardware rather than the translated process.
pub(crate) fn arch() -> Option<Arch> {
    if os() == Os::Mac && std::env::consts::ARCH == "x86_64" && mac_has_arm64() {
        return Some(Arch::Arm64);
    }
    match std::env::consts::ARCH {
        "x86_64" => Some(Arch::X64),
        "aarch64" => Some(Arch::Arm64),
        _ => None,
    }
}

fn mac_has_arm64() -> bool {
    crate::platform::process::background_command("/usr/sbin/sysctl")
        .args(["-n", "hw.optional.arm64"])
        .stdin(Stdio::null())
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "1")
}

pub(crate) fn unsupported_cpu_reason() -> String {
    format!(
        "Ghostex can't install this on a {} processor.",
        std::env::consts::ARCH
    )
}

/// The GNU C library version on Linux (`None` on musl systems such as Alpine, or elsewhere).
pub(crate) fn glibc_version() -> Option<(u32, u32)> {
    if os() != Os::Linux {
        return None;
    }
    let parse = |text: &str| {
        text.split(|c: char| c.is_whitespace() || c == '-')
            .find_map(|token| {
                let mut parts = token.split('.');
                let major = parts.next()?.parse::<u32>().ok()?;
                let minor = parts.next()?.parse::<u32>().ok()?;
                (major == 2).then_some((major, minor))
            })
    };
    for (program, args) in [
        ("getconf", &["GNU_LIBC_VERSION"][..]),
        ("ldd", &["--version"][..]),
    ] {
        let Ok(output) = crate::platform::process::background_command(program)
            .args(args)
            .stdin(Stdio::null())
            .output()
        else {
            continue;
        };
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if text.to_lowercase().contains("musl") {
            return None;
        }
        if let Some(version) = text.lines().next().and_then(parse) {
            return Some(version);
        }
    }
    None
}

/// `Some(reason)` when a Linux download built against glibc cannot run here.
pub(crate) fn glibc_reason(label: &str, minimum: (u32, u32), example: &str) -> Option<String> {
    if os() != Os::Linux {
        return None;
    }
    match glibc_version() {
        Some(version) if version >= minimum => None,
        Some((major, minor)) => Some(format!(
            "{label} needs glibc {}.{} or newer ({example}); this system has {major}.{minor}.",
            minimum.0, minimum.1
        )),
        None => Some(format!(
            "{label} needs a glibc-based Linux ({example}); this system uses a different C library."
        )),
    }
}

/// Windows Subsystem for Linux, where there is no desktop password prompt.
pub(crate) fn is_wsl() -> bool {
    os() == Os::Linux
        && (std::env::var_os("WSL_DISTRO_NAME").is_some()
            || std::fs::read_to_string("/proc/sys/kernel/osrelease")
                .is_ok_and(|release| release.to_lowercase().contains("microsoft")))
}
