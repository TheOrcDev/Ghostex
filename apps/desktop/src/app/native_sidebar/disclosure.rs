use super::model::NativeSidebarSnapshot;
use crate::GhostexGpuiApp;
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, px};
use gpui_component::v_flex;
use std::{collections::HashMap, time::Duration};
use web_time::Instant;

#[derive(Default)]
pub(crate) struct SidebarDisclosures {
    items: HashMap<String, Disclosure>,
}
struct Disclosure {
    collapsed: bool,
    height: f32,
    started: Instant,
    duration: Duration,
    from: f32,
    session_ids: Vec<String>,
}
impl Disclosure {
    fn fraction(&self) -> f32 {
        if self.duration.is_zero() {
            return if self.collapsed { 0.0 } else { 1.0 };
        }
        let t = (self.started.elapsed().as_secs_f32() / self.duration.as_secs_f32()).min(1.0);
        let eased = super::space_gesture::bezier(t, 0.22, 1.0, 0.36, 1.0);
        self.from + ((if self.collapsed { 0.0 } else { 1.0 }) - self.from) * eased
    }
    fn animating(&self) -> bool {
        self.started.elapsed() < self.duration
    }
}
impl SidebarDisclosures {
    pub(crate) fn sync(&mut self, snapshot: &NativeSidebarSnapshot, reduce_motion: bool) {
        let duration = if reduce_motion {
            Duration::ZERO
        } else {
            Duration::from_millis(
                snapshot.hud["settings"]["sidebarCollapseAnimationDurationMs"]
                    .as_u64()
                    .unwrap_or_default(),
            )
        };
        // CDXC:Sidebar 2026-09-20 WHY:
        // Keyed rather than a list scanned per held item: with the sidebar list built in Rust this
        // runs on every change the list draws, a focus step during a held hotkey included, and the
        // scan was one comparison per drawn heading per held heading.
        let mut next: HashMap<String, (bool, Vec<String>)> = HashMap::new();
        for group in &snapshot.groups {
            next.insert(
                format!("group:{}", group.group_id),
                (group.collapsed, Vec::new()),
            );
            for section in &group.sections {
                next.insert(
                    format!("section:{}:{}", group.group_id, section.id),
                    (section.collapsed, section.session_ids.clone()),
                );
            }
        }
        for collection in &snapshot.collections {
            next.insert(
                format!("collection:{}", collection.collection_id),
                (collection.collapsed, Vec::new()),
            );
        }
        self.items.retain(|key, _| next.contains_key(key));
        for (key, (collapsed, ids)) in next {
            if let Some(item) = self.items.get_mut(&key) {
                if item.collapsed != collapsed {
                    item.from = item.fraction();
                    item.started = Instant::now();
                    item.duration = duration;
                    item.collapsed = collapsed;
                }
                if !collapsed {
                    item.session_ids = ids;
                }
            } else {
                self.items.insert(
                    key,
                    Disclosure {
                        collapsed,
                        height: 0.0,
                        started: Instant::now(),
                        duration: Duration::ZERO,
                        from: if collapsed { 0.0 } else { 1.0 },
                        session_ids: ids,
                    },
                );
            }
        }
    }
    pub(crate) fn present(&self, key: &str, collapsed: bool) -> bool {
        !collapsed || self.items.get(key).is_some_and(Disclosure::animating)
    }
    pub(crate) fn section_ids<'a>(
        &'a self,
        key: &str,
        current: &'a [String],
        collapsed: bool,
    ) -> &'a [String] {
        if collapsed {
            self.items
                .get(key)
                .map(|item| item.session_ids.as_slice())
                .unwrap_or(current)
        } else {
            current
        }
    }
}
impl GhostexGpuiApp {
    pub(crate) fn render_native_disclosure(
        &self,
        key: String,
        content: AnyElement,
        cx: &mut gpui::Context<Self>,
    ) -> AnyElement {
        let item = self.native_sidebar.disclosures.items.get(&key);
        let animated = item.is_some_and(Disclosure::animating);
        let fraction = item.map(Disclosure::fraction).unwrap_or(1.0);
        let height = item.map(|item| item.height).unwrap_or_default();
        let view = cx.entity().clone();
        let mut wrapper =
            div()
                .w_full()
                .flex_shrink_0()
                .on_children_prepainted(move |bounds, window, cx| {
                    if let Some(bounds) = bounds.first() {
                        view.update(cx, |app, cx| {
                            if let Some(item) = app.native_sidebar.disclosures.items.get_mut(&key) {
                                let height = f32::from(bounds.size.height);
                                if (item.height - height).abs() > 0.01 {
                                    item.height = height;
                                    cx.notify();
                                }
                                if item.animating() {
                                    window.request_animation_frame();
                                    cx.notify();
                                }
                            }
                        });
                    }
                });
        if animated {
            wrapper = wrapper
                .max_h(px(height * fraction))
                .overflow_hidden()
                .opacity(fraction);
        }
        wrapper
            .child(v_flex().w_full().flex_shrink_0().child(content))
            .into_any_element()
    }
}
