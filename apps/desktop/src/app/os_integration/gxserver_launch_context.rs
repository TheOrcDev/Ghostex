//! Windows: replacing a gxserver that Windows started outside the desktop session.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::app::helpers::*;
use crate::support_logs::{self, GpuiSupportLog};
use crate::*;

/// The pid of the last gxserver the app set out to replace, so one that will not stop is not asked again on every reconnect.
static REPLACED_PID: Mutex<Option<u32>> = Mutex::new(None);
/// RedirectionGuard alone can also come from Windows policy, which a server the app starts would carry too; replacing such a server is tried once per run.
static REPLACED_FOR_REDIRECTION_TRUST: AtomicBool = AtomicBool::new(false);

/// What the running gxserver reported as `launchContext` when Windows started it where its agents are limited.
pub(crate) struct GpuiLimitedGxserverLaunch {
    pid: Option<u32>,
    /// Session 0 or a network logon (SSH); a gxserver the app starts never runs there.
    background: bool,
    launch_context: serde_json::Value,
}

/// The running local gxserver's launch context when it is limited (session 0, SSH, RedirectionGuard); `None` when it is not, or did not say (a WSL or older gxserver).
pub(crate) fn gpui_local_gxserver_limited_launch() -> Option<GpuiLimitedGxserverLaunch> {
    let health = gpui_gxserver_server_health(Duration::from_millis(1000)).ok()?;
    let launch_context = health.get("launchContext")?.clone();
    if launch_context
        .get("limited")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return None;
    }
    Some(GpuiLimitedGxserverLaunch {
        pid: health
            .get("pid")
            .and_then(serde_json::Value::as_u64)
            .and_then(|pid| u32::try_from(pid).ok()),
        background: launch_context
            .get("background")
            .and_then(serde_json::Value::as_bool)
            == Some(true),
        launch_context,
    })
}

/// The Windows session this app runs in.
fn current_session_id() -> Option<u32> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn ProcessIdToSessionId(pid: u32, session_id: *mut u32) -> i32;
    }
    let mut session_id = 0u32;
    (unsafe { ProcessIdToSessionId(std::process::id(), &mut session_id) } != 0)
        .then_some(session_id)
}

impl GhostexGpuiApp {
    /// CDXC:PlatformSupport 2026-10-04 DECISION:
    /// The user chose "Prevent and cure" (Q15 a). The cure: when the app starts or reconnects and the running gxserver reports that Windows started it in the background session (session 0, a network logon from SSH) or with RedirectionGuard enforced, the app replaces it at once with one it starts itself in the desktop session. Sessions keep running because the wmx daemons are separate processes; each keeps the old server's environment until its next Full Reload. Nothing is shown for it, it is only logged.
    /// SEE-ALSO: `server_placement` in server/src/platform/desktop_session.rs (the prevent half) and `LaunchContextReport` in server/src/protocol.rs.
    ///
    /// Returns whether a replacement started, so the bootstrap does not load from the server being replaced.
    pub(crate) fn replace_gpui_limited_gxserver(
        &mut self,
        limited: GpuiLimitedGxserverLaunch,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let skip = if limited.background {
            // An app that itself runs outside the desktop would start a server just like it.
            current_session_id().is_none_or(|session_id| session_id == 0)
        } else {
            REPLACED_FOR_REDIRECTION_TRUST.swap(true, Ordering::SeqCst)
        };
        let already_replaced = {
            let mut replaced = REPLACED_PID
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let already = limited.pid.is_some() && *replaced == limited.pid;
            if !skip && !already {
                *replaced = limited.pid;
            }
            already
        };
        support_logs::append(
            GpuiSupportLog::HostLifecycle,
            "gpui.gxserver.limitedLaunch",
            serde_json::json!({
                "warning": "gxserver runs outside the desktop session",
                "replacing": !skip && !already_replaced,
                "pid": limited.pid,
                "launchContext": limited.launch_context,
            }),
        );
        if skip || already_replaced {
            return false;
        }
        self.stop_gpui_local_gxserver(Some(false), cx);
        true
    }

    /// Runs on every full snapshot of this computer (a launch, a reconnect, a resync).
    pub(crate) fn check_gpui_local_gxserver_launch_context(
        &mut self,
        cx: &mut gpui::Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            let limited = cx
                .background_executor()
                .spawn(async { gpui_local_gxserver_limited_launch() })
                .await;
            if let Some(limited) = limited {
                let _ = this.update(cx, |this, cx| {
                    this.replace_gpui_limited_gxserver(limited, cx)
                });
            }
        })
        .detach();
    }
}
