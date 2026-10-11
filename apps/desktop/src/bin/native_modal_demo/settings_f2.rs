//! Preview fixtures for the Remote, Projects, Agents and Actions Settings pages: the hydrate
//! fields those pages read (`hud.agents`, `hud.commands`, `hud.globalCommands`,
//! `hud.projectSettingsProjects`), the scripted gxserver of the Remote story, the agent CLI
//! endpoint, and the answers to the messages the pages post (`requestAgentHookStatus`,
//! `probeRemoteGxserverInstall`, ...). The fixtures of each page live in their own file.
//!
//! States: see each fixture file (`remote*`, `projects*`, `agents*`, `actions*`).
use serde_json::{Map, Value, json};

#[path = "settings_f2_actions.rs"]
mod actions;
#[path = "settings_f2_agents.rs"]
mod agents;
#[path = "settings_f2_remote.rs"]
mod remote;

/// Settings the page states start from, on top of the story's `modalSettings`.
pub(super) fn story_settings(state: &str, settings: &mut Map<String, Value>) {
    remote::story_settings(state, settings);
    agents::story_settings(state, settings);
    actions::story_settings(state, settings);
}

/// Adds the hud fields these pages read to a `sidebarState` hydrate.
pub(super) fn extend_sidebar_state(message: &mut Value) {
    if !message["hud"].is_object() {
        message["hud"] = json!({});
    }
    agents::extend_sidebar_state(message);
    actions::extend_sidebar_state(message);
}

/// The `open` message fields of a page state, when the state is one of these pages'.
pub(super) fn open_message(state: &str) -> Option<Value> {
    remote::open_message(state)
        .or_else(|| agents::open_message(state))
        .or_else(|| actions::open_message(state))
}

/// Whether the preview host can reach gxserver (`tailcatRpc` defined) in this state.
pub(super) fn gxserver_rpc_available(state: &str) -> bool {
    remote::gxserver_rpc_available(state) && agents::gxserver_rpc_available(state)
}

/// A scripted gxserver answer.
pub(super) fn rpc(path: &str, params: &Value) -> Result<Value, String> {
    if path == "/api/agentCliMaintenance" {
        return agents::rpc(params);
    }
    remote::rpc(path, params)
}

/// Payloads the app would push back after a posted message (`agentHookStatus` after
/// `requestAgentHookStatus`, `remoteGxserverInstallState` after `probeRemoteGxserverInstall`).
pub(super) fn answers(message: &Value) -> Vec<Value> {
    let mut payloads = agents::answers(message);
    payloads.extend(remote::answers(message));
    payloads
}

/// `GHOSTEX_NATIVE_MODAL_DEMO_DRAG="x,y;x,y;..."` (window points): after the page settles, a
/// press at the first point, a pointer drag through the rest, and (with
/// `GHOSTEX_NATIVE_MODAL_DEMO_DRAG_RELEASE=1`) a release at the last, dispatched to the window
/// without focusing it, so a screenshot catches a reorder mid-drag or just after the drop.
pub(super) fn after_open(window: gpui::WindowHandle<gpui_component::Root>, cx: &mut gpui::App) {
    let script = std::env::var("GHOSTEX_NATIVE_MODAL_DEMO_DRAG").unwrap_or_default();
    let points: Vec<gpui::Point<gpui::Pixels>> = script
        .split(';')
        .filter_map(|pair| {
            let mut parts = pair.split(',').map(|n| n.trim().parse::<f32>().ok());
            Some(gpui::point(
                gpui::px(parts.next()??),
                gpui::px(parts.next()??),
            ))
        })
        .collect();
    if points.is_empty() {
        return;
    }
    let release = std::env::var("GHOSTEX_NATIVE_MODAL_DEMO_DRAG_RELEASE").is_ok_and(|v| v == "1");
    cx.spawn(async move |cx| {
        let executor = cx.background_executor().clone();
        let pause = |ms: u64| executor.timer(std::time::Duration::from_millis(ms));
        pause(1500).await;
        let left = gpui::MouseButton::Left;
        let modifiers = gpui::Modifiers::default();
        for (step, position) in points.iter().copied().enumerate() {
            let _ = window.update(cx, |_, window, cx| {
                if step == 0 {
                    window.dispatch_event(
                        gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                            position,
                            pressed_button: None,
                            modifiers,
                        }),
                        cx,
                    );
                    window.dispatch_event(
                        gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                            button: left,
                            position,
                            modifiers,
                            click_count: 1,
                            first_mouse: false,
                        }),
                        cx,
                    );
                } else {
                    window.dispatch_event(
                        gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                            position,
                            pressed_button: Some(left),
                            modifiers,
                        }),
                        cx,
                    );
                }
            });
            pause(40).await;
        }
        // `GHOSTEX_NATIVE_MODAL_DEMO_DRAG_ESCAPE=1`: Escape mid-drag (the drag is cancelled).
        if std::env::var("GHOSTEX_NATIVE_MODAL_DEMO_DRAG_ESCAPE").is_ok_and(|v| v == "1") {
            let any_window: gpui::AnyWindowHandle = window.into();
            let _ = any_window.update(cx, |_, window, cx| {
                if let Ok(keystroke) = gpui::Keystroke::parse("escape") {
                    window.dispatch_keystroke(keystroke, cx);
                }
            });
            pause(40).await;
        }
        if release {
            let position = *points
                .last()
                .unwrap_or(&gpui::point(gpui::px(0.0), gpui::px(0.0)));
            let _ = window.update(cx, |_, window, cx| {
                window.dispatch_event(
                    gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                        button: left,
                        position,
                        modifiers,
                        click_count: 1,
                    }),
                    cx,
                );
            });
        }
    })
    .detach();
}
