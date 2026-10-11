use super::*;
use crate::app::helpers::*;
use crate::*;
use gpui::{AnyElement, Entity, FontWeight, Subscription, Window, div, prelude::*, px};
use gpui_component::{
    h_flex,
    input::{Input, InputEvent, InputState},
    radio::Radio,
    v_flex,
};
use serde_json::{Value, json};

pub(crate) struct WebsiteHomeEditor {
    pub view_id: ExtensionId,
    pub project_id: String,
    input: Entity<InputState>,
    follow_parent: bool,
    error: Option<String>,
    _subscription: Subscription,
}

impl GhostexGpuiApp {
    pub(crate) fn open_website_home_editor(
        &mut self,
        id: ExtensionId,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(provider) = website_provider(id).filter(|provider| !provider.automatic()) else {
            return;
        };
        let Some(project_id) = self.active_project_id_for_view_scope() else {
            return;
        };
        let settings = shared_settings::shared_sidebar_settings_snapshot();
        let own = settings
            .object()
            .get("projectWebsiteViews")
            .and_then(|v| v.get(&provider.id))
            .and_then(|v| v.get("homes"))
            .and_then(|v| v.get(&project_id))
            .and_then(Value::as_str);
        let initial = self.website_home(provider, &project_id).unwrap_or_default();
        let follow_parent = own.is_none()
            && !initial.is_empty()
            && self.website_parent_project_id(&project_id).is_some();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(provider.placeholder.clone())
                .default_value(initial)
        });
        let subscription = cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Focus => {
                    this.pending_keyboard_handoff = None;
                    this.reclaim_gpui_root_for_chrome_input_focus();
                }
                InputEvent::Change => {
                    if let Some(editor) = this.project_views.website_editor.as_mut() {
                        editor.error = None;
                    }
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.save_website_home(window, cx),
                _ => {}
            },
        );
        self.project_views.website_editor = Some(WebsiteHomeEditor {
            view_id: id,
            project_id,
            input: input.clone(),
            follow_parent,
            error: None,
            _subscription: subscription,
        });
        self.open_view_tab(TitlebarMode::Extension(id), window, cx);
        self.pending_keyboard_handoff = None;
        self.update_project_workarea_runtime_cef_surface_visibility(cx);
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        {
            #[cfg(target_os = "macos")]
            self.begin_programmatic_focus();
            cef::focus_gpui_root_view(self.parent_ns_view);
            #[cfg(target_os = "macos")]
            self.end_programmatic_focus();
        }
        if !follow_parent {
            input.update(cx, |input, cx| input.focus(window, cx));
        }
        cx.notify();
    }

    fn choose_website_home(
        &mut self,
        url: Option<String>,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(editor) = self.project_views.website_editor.as_mut() {
            editor.follow_parent = url.is_none();
            editor.error = None;
            if let Some(url) = url {
                editor.input.update(cx, |input, cx| {
                    input.set_value(url, window, cx);
                    input.focus(window, cx);
                });
            }
        }
        cx.notify();
    }

    fn save_website_home(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let Some(editor) = self.project_views.website_editor.as_ref() else {
            return;
        };
        if self.active_project_id_for_view_scope().as_deref() != Some(&editor.project_id) {
            return;
        }
        let Some(provider) = website_provider(editor.view_id) else {
            return;
        };
        let project_id = editor.project_id.clone();
        let id = editor.view_id;
        let home = editor.input.read(cx).value().trim().to_string();
        let following =
            editor.follow_parent && self.website_parent_project_id(&project_id).is_some();
        if following
            && self
                .website_parent_project_id(&project_id)
                .and_then(|parent| self.website_home(provider, &parent))
                .is_none()
        {
            self.project_views.website_editor.as_mut().unwrap().error = Some("The parent project has no home yet. Choose a home for this worktree or set up the parent first.".into());
            cx.notify();
            return;
        }
        if !following && let Err(error) = provider.workspace(&home) {
            self.project_views.website_editor.as_mut().unwrap().error = Some(error);
            cx.notify();
            return;
        }
        let mut settings = shared_settings::shared_sidebar_settings_snapshot()
            .object()
            .clone();
        let all = settings
            .entry("projectWebsiteViews")
            .or_insert_with(|| json!({}));
        if !all.is_object() {
            *all = json!({});
        }
        let entry = all
            .as_object_mut()
            .unwrap()
            .entry(provider.id.clone())
            .or_insert_with(|| json!({}));
        if !entry.is_object() {
            *entry = json!({});
        }
        if !entry["homes"].is_object() {
            entry["homes"] = json!({});
        }
        if following {
            entry["homes"].as_object_mut().unwrap().remove(&project_id);
        } else {
            entry["homes"][&project_id] = json!(home);
            let workspace_key = provider.workspace(&home).unwrap().0;
            let mut previous = entry["workspaces"].as_array().cloned().unwrap_or_default();
            previous.retain(|value| {
                value
                    .as_str()
                    .and_then(|url| provider.workspace(url).ok())
                    .is_some_and(|(key, _)| key != workspace_key)
            });
            previous.insert(0, json!(home));
            entry["workspaces"] = json!(previous);
        }
        if let Err(error) = shared_settings::write_shared_sidebar_settings_object(settings) {
            self.project_views.website_editor.as_mut().unwrap().error =
                Some(format!("Could not save the home URL: {error:?}"));
            cx.notify();
            return;
        }
        let previous_home = self
            .custom_project_view_status(id)
            .and_then(|status| status["url"].as_str())
            .map(str::to_string);
        self.project_views.website_editor = None;
        self.ensure_project_workarea_runtime_cef_surfaces_for_current_context(cx);
        self.update_project_workarea_runtime_cef_surface_visibility(cx);
        if let Some(home) = self.website_home(provider, &project_id)
            && previous_home.as_deref() == Some(&home)
            && let Some(owned) = self
                .project_workarea_runtime_cef_surfaces
                .get(&ProjectWorkareaCefSurfaceSlotKey::Extension(id))
        {
            owned
                .surface
                .update(cx, |surface, _| surface.load_url(&home));
        }
        self.focus_project_editor_surface(TitlebarMode::Extension(id), window, cx);
        cx.notify();
    }

    pub(crate) fn render_website_home_setup(
        &mut self,
        id: ExtensionId,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) -> AnyElement {
        if !self.website_home_editor_is_open(id) {
            self.open_website_home_editor(id, window, cx);
        }
        let Some(editor) = self.project_views.website_editor.as_ref() else {
            return div().into_any_element();
        };
        let provider = website_provider(id).expect("website setup provider");
        let input = editor.input.clone();
        let value = input.read(cx).value().to_string();
        let following = editor.follow_parent;
        let error = editor.error.clone();
        let parent = self.website_parent_project_id(&editor.project_id);
        let parent_name = parent
            .as_ref()
            .and_then(|id| self.extension_projects.get(id))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "parent project".into());
        let can_cancel = self.website_home(provider, &editor.project_id).is_some();
        let settings = shared_settings::shared_sidebar_settings_snapshot();
        let saved = settings
            .object()
            .get("projectWebsiteViews")
            .and_then(|v| v.get(&provider.id))
            .and_then(|v| v.get("workspaces"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        /*
        CDXC:Theming 2026-09-23 DECISION:
        User: the Linear and Jira "Where should ... open?" page matches the native Automate view's style. It takes the Automate palette: no page fill under window glass, washed inputs and cards with hairline borders, and soft secondary buttons instead of a solid white one.
        */
        let p =
            crate::app::native_automate::AutomatePalette::resolve(window_glass_active_in(window));
        let background = p.page;
        let border = p.border;
        let muted = p.muted;
        let foreground = p.foreground;
        let card = p.card;
        let control = |key: String, label: String, cx: &mut gpui::Context<Self>| {
            crate::app::native_automate::secondary_button(
                &p,
                key,
                None,
                label,
                |_: &mut Self, _, _| {},
                cx,
            )
            .w_full()
            .justify_center()
        };
        // The same centred card the sleeping views and startup screens use, with the provider's icon.
        let mut body = v_flex()
            .w_full()
            .gap(px(14.))
            .font_family(p.font.clone())
            .text_color(foreground)
            .child(
                v_flex()
                    .items_center()
                    .gap(px(8.))
                    .text_center()
                    .child(titlebar_svg_icon(
                        TitlebarMode::Extension(id).tab_icon(),
                        34.0,
                        chrome_ink().opacity(0.8).into(),
                    ))
                    .child(
                        div()
                            .text_size(px(16.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Where should {} open?", provider.title)),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(muted)
                            .child("Paste the URL you want this view to open."),
                    ),
            );
        if parent.is_some() {
            body = body.child(
                v_flex()
                    .gap(px(8.))
                    .child(
                        Radio::new("website-follow-parent")
                            .label(format!("Follow {parent_name}"))
                            .checked(following)
                            .w_full()
                            .p(px(12.))
                            .border_1()
                            .border_color(border)
                            .bg(card)
                            .rounded(px(8.))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_website_home(None, window, cx)
                            })),
                    )
                    .child(
                        Radio::new("website-custom-home")
                            .label("Use a different home for this worktree")
                            .checked(!following)
                            .w_full()
                            .p(px(12.))
                            .border_1()
                            .border_color(border)
                            .bg(card)
                            .rounded(px(8.))
                            .on_click(cx.listener(|this, _, window, cx| {
                                let value = this
                                    .project_views
                                    .website_editor
                                    .as_ref()
                                    .map(|editor| editor.input.read(cx).value().to_string())
                                    .unwrap_or_default();
                                this.choose_website_home(Some(value), window, cx);
                            })),
                    ),
            );
        }
        if !following {
            if !saved.is_empty() {
                let mut choices = v_flex().gap(px(8.)).child(
                    div()
                        .text_size(px(12.))
                        .text_color(muted)
                        .child("Use a saved URL, or paste another"),
                );
                for (index, url) in saved.iter().filter_map(Value::as_str).enumerate() {
                    let Ok((_, label)) = provider.workspace(url) else {
                        continue;
                    };
                    let home = url.to_string();
                    choices = choices.child(
                        control(format!("website-workspace-{index}"), label, cx).on_click(
                            cx.listener(move |this, _, window, cx| {
                                this.choose_website_home(Some(home.clone()), window, cx)
                            }),
                        ),
                    );
                }
                body = body.child(choices);
            }
            body = body
                .child(
                    v_flex()
                        .gap(px(7.))
                        .child(div().text_size(px(12.)).child("Home URL"))
                        .child(
                            div()
                                .h(px(34.))
                                .px(px(10.))
                                .flex()
                                .items_center()
                                .rounded(px(8.))
                                .border_1()
                                .border_color(border)
                                .bg(card)
                                .child(
                                    Input::new(&input)
                                        .appearance(false)
                                        .bordered(false)
                                        .focus_bordered(false)
                                        .placeholder_color(muted)
                                        .w_full()
                                        .text_size(px(13.)),
                                ),
                        ),
                )
                .child(
                    div().text_size(px(12.)).text_color(muted).child(
                        provider
                            .workspace(&value)
                            .map(|(_, label)| format!("Workspace: {label}"))
                            .unwrap_or_else(|_| "Workspace is detected from your URL.".into()),
                    ),
                );
        }
        // User: the "Worktrees start with this home" note is not needed; a worktree's own card keeps
        // the line that says whether it follows its parent.
        if parent.is_some() {
            body = body.child(div().text_size(px(12.)).text_color(muted).child(if following {
                format!("This worktree will follow {parent_name}, including future home changes.")
            } else {
                "Only this worktree changes. Other worktrees keep their homes.".into()
            }));
        }
        if let Some(error) = error {
            body = body.child(div().text_size(px(12.)).text_color(p.danger).child(error));
        }
        let mut actions = h_flex().gap(px(8.));
        if can_cancel {
            actions = actions.child(
                control("website-cancel".into(), "Cancel".into(), cx).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.project_views.website_editor = None;
                        this.ensure_project_workarea_runtime_cef_surfaces_for_current_context(cx);
                        this.update_project_workarea_runtime_cef_surface_visibility(cx);
                        cx.notify();
                    },
                )),
            );
        }
        body = body.child(
            actions.child(
                control(
                    "website-save".into(),
                    format!("Save and open {}", provider.title),
                    cx,
                )
                .bg(p.selected)
                .on_click(cx.listener(|this, _, window, cx| this.save_website_home(window, cx))),
            ),
        );
        div()
            .id("website-home-setup")
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .bg(background)
            .p(px(16.))
            .flex()
            .items_center()
            .justify_center()
            .child(
                crate::app::render::sleeping_card::view_card_frame()
                    .w(px(440.))
                    .child(body),
            )
            .into_any_element()
    }
}
