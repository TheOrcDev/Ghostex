//! The desktop writes scenario-gated diagnostic files; the phone's chat sends the same calls to
//! `log` at debug level (logcat on Android), and no scenario is ever on.
use serde_json::Value;

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) enum GpuiSupportLog {
    SidebarRefresh,
    TerminalFocus,
    SessionChat,
}

pub(crate) fn append(log: GpuiSupportLog, event: &str, details: Value) {
    log::debug!("{log:?} {event} {details}");
}

pub(crate) fn append_for_scenario(
    log: GpuiSupportLog,
    _scenario_id: &str,
    event: &str,
    details: Value,
) {
    append(log, event, details);
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) enum GpuiDiagnosticScenario {
    SidebarRefresh,
    SessionChat,
    TerminalFocus,
}

pub(crate) fn scenario_enabled(_scenario: GpuiDiagnosticScenario) -> bool {
    false
}
