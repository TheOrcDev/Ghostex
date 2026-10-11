//! Finding and closing the running app before its bundle is replaced.

use std::path::{Path, MAIN_SEPARATOR};
use std::process::Command;
use std::time::{Duration, Instant};

use super::Start;
use crate::bail;
use crate::util::{self, sleep_ms, Res};

impl Start {
    /// CDXC:Build 2026-06-25-13:56:
    /// The local GPUI rebuild command must fully close the exact dev bundle before replacing it. If AppleScript quit and SIGTERM leave a stale or slow GPUI process alive, escalate to SIGKILL and still verify the bundle has exited before building.
    ///
    /// CDXC:Build 2026-07-08-04:55:
    /// macOS GPUI starts build before closing the stable installed app. Only the old staged build bundle is closed before packaging, because build-macos-app.sh replaces that directory; the installed /Applications copy is closed only after a successful build.
    pub fn close_running_bundle(
        &self,
        bundle_path: &Path,
        action: &str,
        include_bundle_id: bool,
    ) -> Res {
        if self
            .running_bundle_pids(bundle_path, include_bundle_id)
            .is_empty()
        {
            return Ok(());
        }
        self.log
            .notice(&format!("Closing running {} {action}.", self.app_name));
        if include_bundle_id && self.is_darwin {
            let _ = util::output(Command::new("osascript").args([
                "-e",
                &format!("tell application id \"{}\" to quit", self.bundle_id),
            ]));
            if self.wait_for_bundle_exit(bundle_path, include_bundle_id, Duration::from_secs(8)) {
                return Ok(());
            }
        }
        let pids = self.running_bundle_pids(bundle_path, include_bundle_id);
        self.terminate_pids(&pids, false);
        if self.wait_for_bundle_exit(bundle_path, include_bundle_id, Duration::from_secs(8)) {
            return Ok(());
        }
        let pids = self.running_bundle_pids(bundle_path, include_bundle_id);
        self.log
            .notice(&format!("Force closing {} {action}.", self.app_name));
        self.terminate_pids(&pids, true);
        if !self.wait_for_bundle_exit(bundle_path, include_bundle_id, Duration::from_secs(2)) {
            bail!(
                "{} did not exit, refusing to continue while {} is still running.",
                self.app_name,
                bundle_path.display()
            );
        }
        Ok(())
    }

    fn wait_for_bundle_exit(
        &self,
        bundle_path: &Path,
        include_bundle_id: bool,
        timeout: Duration,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self
                .running_bundle_pids(bundle_path, include_bundle_id)
                .is_empty()
            {
                return true;
            }
            sleep_ms(100);
        }
        self.running_bundle_pids(bundle_path, include_bundle_id)
            .is_empty()
    }

    fn running_bundle_pids(&self, bundle_path: &Path, include_bundle_id: bool) -> Vec<String> {
        let mut pids = if include_bundle_id {
            self.pids_by_bundle_id()
        } else {
            Vec::new()
        };
        for pid in self.pids_by_bundle_path(bundle_path) {
            if !pids.contains(&pid) {
                pids.push(pid);
            }
        }
        pids
    }

    /// CDXC:Build 2026-09-23 WHY:
    /// This runs every 100ms while the app quits and launches. Asking System Events through osascript took ~0.27s per call; lsappinfo reads the same Launch Services table in ~0.01s.
    pub fn pids_by_bundle_id(&self) -> Vec<String> {
        if !self.is_darwin {
            return Vec::new();
        }
        let Some(found) = util::stdout_if_ok(
            Command::new("lsappinfo").args(["find", &format!("bundleid={}", self.bundle_id)]),
        ) else {
            return Vec::new();
        };
        let asns = asns(&found);
        if asns.is_empty() {
            return Vec::new();
        }
        let Some(info) = util::stdout_if_ok(
            Command::new("lsappinfo")
                .args(["info", "-only", "pid"])
                .args(&asns),
        ) else {
            return Vec::new();
        };
        pid_values(&info)
    }

    pub fn pids_by_bundle_path(&self, bundle_path: &Path) -> Vec<String> {
        if self.targets_windows {
            return self.windows_app_pids();
        }
        let Some(listing) =
            util::stdout_if_ok(Command::new("ps").args(["-axo", "pid=,args=", "-ww"]))
        else {
            return Vec::new();
        };
        let code_server_store =
            format!("{}{MAIN_SEPARATOR}", self.code_server_store_root.display());
        listing
            .lines()
            .filter_map(|line| {
                let line = line.trim_start();
                let (pid, args) = line.split_once(char::is_whitespace)?;
                let args = args.trim_start();
                if pid.is_empty() || !pid.bytes().all(|b| b.is_ascii_digit()) || args.is_empty() {
                    return None;
                }
                let belongs = self.command_line_belongs_to_bundle(args, bundle_path)
                    // The installed app's code-server runs from the local-start store, not from inside the bundle.
                    || (self.is_darwin && bundle_path == self.installed_app_path && args.contains(&code_server_store));
                belongs.then(|| pid.to_string())
            })
            .collect()
    }

    fn command_line_belongs_to_bundle(&self, command_line: &str, bundle_path: &Path) -> bool {
        if self.is_darwin {
            // CDXC:Build 2026-07-10:
            // A macOS app bundle also contains long-lived gxserver and zmx executables under Contents/Resources. Those processes deliberately survive a GPUI app restart, so bundle-path ownership must include only the main UI executable and CEF helper app executables. Otherwise the graceful-quit wait can never finish while persistence is working, and its SIGTERM/SIGKILL escalation kills the very zmx sessions the relaunched app is meant to reattach.
            let main_executable = bundle_path
                .join("Contents")
                .join("MacOS")
                .join(&self.app_name);
            if runs_executable(command_line, &main_executable) {
                return true;
            }
            let helpers_root = format!(
                "{}{MAIN_SEPARATOR}",
                bundle_path.join("Contents").join("Frameworks").display()
            );
            let Some(relative) = command_line.strip_prefix(&helpers_root) else {
                return false;
            };
            let marker =
                format!(".app{MAIN_SEPARATOR}Contents{MAIN_SEPARATOR}MacOS{MAIN_SEPARATOR}");
            let Some(marker_index) = relative.find(&marker) else {
                return false;
            };
            let helper_name = &relative[..marker_index];
            if !helper_name.starts_with(&format!("{} Helper", self.app_name)) {
                return false;
            }
            let helper_executable = bundle_path
                .join("Contents")
                .join("Frameworks")
                .join(format!("{helper_name}.app"))
                .join("Contents")
                .join("MacOS")
                .join(helper_name);
            return runs_executable(command_line, &helper_executable);
        }
        // Flat Linux layout: match only the app binaries (Ghostex, the release package's ghostex-gpui-runtime, and ghostex-gpui-cef-helper). The gxserver daemon and zmx sessions live under the same directory and must survive a rebuild.
        ["Ghostex", "ghostex-gpui-runtime", "ghostex-gpui-cef-helper"]
            .iter()
            .any(|name| runs_executable(command_line, &bundle_path.join(name)))
    }

    fn terminate_pids(&self, pids: &[String], force: bool) {
        if self.targets_windows {
            self.terminate_windows_pids(pids, force);
        } else {
            #[cfg(unix)]
            for pid in pids {
                if let Ok(pid) = pid.parse::<i32>() {
                    util::kill_pid(pid, force);
                }
            }
        }
    }
}

fn runs_executable(command_line: &str, executable: &Path) -> bool {
    let executable = executable.display().to_string();
    command_line == executable || command_line.starts_with(&format!("{executable} "))
}

/// Every `ASN:0x<hex>-0x<hex>` in lsappinfo output, which prints them as `ASN:0x0-0xb58b58-"Ghostex":`.
fn asns(text: &str) -> Vec<String> {
    let hex_run = |s: &str| s.find(|c: char| !c.is_ascii_hexdigit()).unwrap_or(s.len());
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(index) = rest.find("ASN:") {
        rest = &rest[index + 4..];
        let Some(first) = rest.strip_prefix("0x") else {
            continue;
        };
        let first_len = hex_run(first);
        let Some(second) = first[first_len..].strip_prefix("-0x") else {
            continue;
        };
        let second_len = hex_run(second);
        if first_len > 0 && second_len > 0 {
            found.push(format!(
                "ASN:0x{}-0x{}",
                &first[..first_len],
                &second[..second_len]
            ));
        }
    }
    found
}

/// Every `pid = 123` / `"pid"=123` value in lsappinfo info output.
fn pid_values(info: &str) -> Vec<String> {
    let mut pids = Vec::new();
    let mut rest = info;
    while let Some(index) = rest.find("pid") {
        let after = rest[index + 3..].trim_start_matches('"').trim_start();
        rest = &rest[index + 3..];
        let Some(after) = after.strip_prefix('=') else {
            continue;
        };
        let digits: String = after
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            pids.push(digits);
        }
    }
    pids
}
