//! Run on macOS with a display: cargo test --release --features shader-canvas-probe --test shader-canvas.
//! Exercises the terminal's real shader preflight without allocating giant textures.
//! Includes the same module tree as terminal-element-demo; keep those imports in sync.
#![allow(dead_code)]

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("SKIP: terminal shader effects are macOS-only");
}

#[cfg(target_os = "macos")]
#[path = "../src/ghostty_kit.rs"]
mod ghostty_kit;
#[cfg(target_os = "macos")]
#[path = "../src/ghostty_vt/mod.rs"]
mod ghostty_vt;
#[cfg(target_os = "macos")]
#[path = "../src/hotkey_label.rs"]
mod hotkey_label;
#[cfg(target_os = "macos")]
#[path = "../src/shared_settings/mod.rs"]
mod shared_settings;
#[cfg(target_os = "macos")]
#[path = "../src/support_logs.rs"]
mod support_logs;
#[cfg(target_os = "macos")]
#[path = "../src/terminal_environment.rs"]
mod terminal_environment;
#[cfg(target_os = "macos")]
#[path = "../src/terminal_model.rs"]
mod terminal_model;
#[cfg(target_os = "macos")]
#[path = "../src/terminal_scrollbar_reveal.rs"]
mod terminal_scrollbar_reveal;
#[cfg(target_os = "macos")]
#[path = "../src/terminal_shaders.rs"]
mod terminal_shaders;
#[cfg(target_os = "macos")]
#[path = "../src/terminal_wheel.rs"]
mod terminal_wheel;

#[cfg(target_os = "macos")]
mod terminal_element {
    include!("../src/terminal_element.rs");

    pub fn probe(terminal: &Entity<TerminalView>, window: &mut Window, cx: &mut App) -> usize {
        use crate::terminal_shaders::{ShaderAnimation, TerminalShaderSettings};
        let scale = window.scale_factor();
        let limit = gpui::MAX_SHADER_EFFECT_TEXTURE_SIZE as f32;
        let layout = TerminalLayout::background_only(
            CellMetrics {
                cell_width: px(8.),
                line_height: px(17.),
            },
            gpui::black(),
        );
        let cases = [
            ("zero-width", 0., 100., false),
            ("zero-height", 100., 0., false),
            ("negative-width", -1., 100., false),
            ("negative-height", 100., -1., false),
            ("too-wide", limit + 1., 100., false),
            ("too-tall", 100., limit + 1., false),
            ("exact-limit", limit, limit, true),
            ("ordinary", 800., 600., true),
        ];
        let mut failures = 0;
        for (name_mode, mode, focused, animates) in [
            ("always-unfocused", ShaderAnimation::Always, false, true),
            (
                "focused",
                ShaderAnimation::Focused,
                true,
                window.is_window_active(),
            ),
            ("focused-unfocused", ShaderAnimation::Focused, false, false),
            ("off", ShaderAnimation::Off, true, false),
        ] {
            terminal.update(cx, |view, _| {
                view.settings.shaders = Some(TerminalShaderSettings {
                    sources: vec![Arc::<str>::from("probe: never compiled")].into(),
                    animation: mode,
                    enabled: true,
                    background_alpha: 0.5,
                });
                view.focused = focused;
                view.refresh_snapshot();
            });
            for (name, width, height, supported) in cases {
                // Drain unrelated callbacks so the next count belongs to this preflight.
                window.simulate_next_frame(cx);
                let bounds = Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(width / scale), px(height / scale)),
                );
                let effect = TerminalElement::new(terminal.clone())
                    .shader_effect(bounds, &layout, window, cx);
                let animation_requests = window.simulate_next_frame(cx);
                let expected_requests = usize::from(supported && animates);
                let shader_ready = effect.as_ref().is_some_and(|(shader, _)| shader.is_some());
                let background_alpha = effect.map(|(_, alpha)| alpha);
                let passed = shader_ready == supported
                    && animation_requests == expected_requests
                    && background_alpha == Some(0.5);
                println!(
                    "{}",
                    serde_json::json!({
                        "case": name, "mode": name_mode, "scale": scale,
                        "effect": shader_ready, "background_alpha": background_alpha,
                        "animation_requests": animation_requests, "passed": passed
                    })
                );
                failures += usize::from(!passed);
            }
        }
        failures
    }
}

#[cfg(target_os = "macos")]
fn main() {
    use gpui::{App, AppContext as _, Context, IntoElement, Render, Window, px, size};
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(30));
        eprintln!("FAIL: native shader canvas probe timed out");
        std::process::exit(124);
    });
    struct Probe {
        terminal: gpui::Entity<terminal_element::TerminalView>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let terminal = self.terminal.clone();
            gpui::canvas(
                |_, _, _| (),
                move |_, _, window, cx| {
                    let failures = terminal_element::probe(&terminal, window, cx);
                    // GPUI's native run loop exits the process on quit; return the probe result there.
                    cx.defer(move |_| std::process::exit(if failures == 0 { 0 } else { 1 }));
                },
            )
            .size_full()
        }
    }
    use gpui::Styled as _;
    gpui_platform::application().run(move |cx: &mut App| {
        cx.open_window(
            gpui::WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::centered(
                    None,
                    size(px(100.), px(100.)),
                    cx,
                ))),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Shader canvas regression".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |_, cx| {
                let terminal = cx.new(|cx| {
                    terminal_element::TerminalView::spawn(
                        terminal_model::TerminalSpawnConfig {
                            program: "/bin/cat".into(),
                            args: vec![],
                            env: vec![],
                            cwd: None,
                            cols: 80,
                            rows: 24,
                            cell_width_px: 8,
                            cell_height_px: 17,
                            max_scrollback: 100,
                        },
                        Default::default(),
                        Box::new(|_, _, _, _| {}),
                        cx,
                    )
                    .unwrap()
                });
                cx.new(|_| Probe { terminal })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
