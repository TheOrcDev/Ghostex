//! A Mermaid diagram inside a Markdown document, drawn the way the Docs page drew it
//! (`packages/core-ui/mermaid/mermaid-diagram.tsx` (deleted 2026-10-01) and `mermaid.css`): a bordered card with a
//! toolbar (Diagram / Source, zoom out, fit, zoom in, copy source, expand) over a viewport that
//! fits the diagram to 416px high, scrolls when zoomed and pans by dragging. The editor keeps a
//! gap for it (`EditorState::set_mermaid_view_provider`, see `blocks.rs`).

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClickEvent, ClipboardItem, Context, CursorStyle, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement as _, Pixels, Point, Render, RenderImage, ScrollHandle,
    StatefulInteractiveElement as _, Styled as _, Window, div, img, point, px, svg,
};

use super::blocks::{mermaid_svg, rasterize_svg_scaled, svg_natural_size};
use crate::app::window::native_modal_kit::{
    ModalPalette, ModalSegmentedItem, modal_segmented_control,
};

const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 4.0;
const ZOOM_STEP: f32 = 1.25;
/// `.ghostex-mermaid-viewport`: 12px padding, at most 440px tall, at least 80px.
const VIEWPORT_PADDING: f32 = 12.0;
const VIEWPORT_MAX: f32 = 440.0;
const VIEWPORT_MIN: f32 = 80.0;
/// The height the diagram is fitted to (`frame.height`).
const FRAME_HEIGHT: f32 = 416.0;
/// `.ghostex-mermaid-toolbar`: 6px padding around 26px controls, and its bottom border.
const TOOLBAR_HEIGHT: f32 = 6.0 + 26.0 + 6.0 + 1.0;
/// `margin: 0.65rem 0`.
const CARD_MARGIN: f32 = 10.4;
const SOURCE_LINE: f32 = 19.2;
const COPIED_FOR: Duration = Duration::from_millis(1200);
const MAX_RASTER_SIDE: f32 = 8192.0;
const MAX_RASTER_PIXELS: f32 = 32_000_000.0;

/// What the card asks its host for.
pub(crate) type MermaidExpand = Rc<dyn Fn(String, &mut App)>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Diagram,
    Source,
}

enum Diagram {
    Rendering,
    Ready {
        svg: Arc<str>,
        width: f32,
        height: f32,
    },
    Failed(String),
}

/// The card's colours: the document's code tint and border, and its text.
#[derive(Clone, Copy)]
pub(crate) struct MermaidColors {
    pub(crate) light: bool,
    pub(crate) background: Hsla,
    pub(crate) border: Hsla,
    pub(crate) text: Hsla,
    pub(crate) muted: Hsla,
    pub(crate) hover: Hsla,
    pub(crate) mono: &'static str,
}

pub(crate) struct DocsMermaidWidget {
    source: String,
    colors: MermaidColors,
    mode: Mode,
    zoom: f32,
    diagram: Diagram,
    raster: Option<(Arc<RenderImage>, u32)>,
    raster_pending: bool,
    /// The card's width as last laid out, which the fitted size comes from.
    width: Rc<Cell<f32>>,
    scroll: ScrollHandle,
    drag: Option<(Point<Pixels>, Point<Pixels>)>,
    copied: bool,
    copied_generation: u64,
    expand: MermaidExpand,
    /// Read by the editor's view provider for the height to keep.
    layout: Rc<Cell<MermaidLayout>>,
}

impl DocsMermaidWidget {
    pub(crate) fn new(
        source: String,
        colors: MermaidColors,
        expand: MermaidExpand,
        cx: &mut Context<Self>,
    ) -> Self {
        let text = source.clone();
        let light = colors.light;
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let result = background
                .spawn(async move {
                    let svg = mermaid_svg(&text, light)?;
                    let (width, height) = svg_natural_size(&svg)
                        .ok_or_else(|| "The diagram image could not be displayed.".to_string())?;
                    Ok::<_, String>((svg, width, height))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.diagram = match result {
                    Ok((svg, width, height)) => Diagram::Ready {
                        svg: svg.into(),
                        width,
                        height,
                    },
                    Err(error) => Diagram::Failed(error),
                };
                cx.notify();
            });
        })
        .detach();
        Self {
            source,
            colors,
            mode: Mode::Diagram,
            zoom: 1.0,
            diagram: Diagram::Rendering,
            raster: None,
            raster_pending: false,
            width: Rc::new(Cell::new(0.0)),
            scroll: ScrollHandle::new(),
            drag: None,
            copied: false,
            copied_generation: 0,
            expand,
            layout: Rc::new(Cell::new(MermaidLayout {
                source: false,
                zoom: 1.0,
                ratio: None,
                failed: false,
                source_lines: 1,
            })),
        }
    }

    /// The layout the card's height follows, shared with the editor's view provider.
    pub(crate) fn layout(&self) -> Rc<Cell<MermaidLayout>> {
        self.layout.clone()
    }

    fn current_layout(&self) -> MermaidLayout {
        MermaidLayout {
            source: self.mode == Mode::Source,
            zoom: self.zoom,
            ratio: match &self.diagram {
                Diagram::Ready { width, height, .. } => Some(width / height.max(1.0)),
                _ => None,
            },
            failed: matches!(self.diagram, Diagram::Failed(_)),
            source_lines: self.source.lines().count(),
        }
    }

    /// The fitted diagram size inside a card `card_width` wide, at the current zoom.
    fn display_size(&self, card_width: f32) -> Option<(f32, f32)> {
        let Diagram::Ready { width, height, .. } = &self.diagram else {
            return None;
        };
        let ratio = width / height.max(1.0);
        let frame_width = (card_width - 2.0 - VIEWPORT_PADDING * 2.0).max(1.0);
        let shown = self.zoom * frame_width.min(FRAME_HEIGHT * ratio);
        Some((shown, shown / ratio))
    }

    fn viewport_height(&self, card_width: f32) -> f32 {
        let total = self.current_layout().height(px(card_width));
        f32::from(total) - CARD_MARGIN * 2.0 - 2.0 - TOOLBAR_HEIGHT
    }

    fn set_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        cx.notify();
    }

    fn copy_source(&mut self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.source.clone()));
        self.copied = true;
        self.copied_generation = self.copied_generation.wrapping_add(1);
        let generation = self.copied_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                if this.copied_generation == generation {
                    this.copied = false;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn request_raster(&mut self, display_width: f32, scale_factor: f32, cx: &mut Context<Self>) {
        let Diagram::Ready { svg, width, height } = &self.diagram else {
            return;
        };
        let (natural_width, natural_height) = (*width, *height);
        let mut scale = display_width * scale_factor / natural_width;
        let limit = (MAX_RASTER_SIDE / (natural_width * scale))
            .min(MAX_RASTER_SIDE / (natural_height * scale))
            .min((MAX_RASTER_PIXELS / (natural_width * natural_height * scale * scale)).sqrt())
            .min(1.0);
        scale *= limit;
        let device_width = ((natural_width * scale / 8.0).round() * 8.0).max(8.0) as u32;
        if self.raster.as_ref().map(|(_, width)| *width) == Some(device_width)
            || self.raster_pending
        {
            return;
        }
        self.raster_pending = true;
        let scale = device_width as f32 / natural_width;
        let svg = svg.clone();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let image = background
                .spawn(async move { rasterize_svg_scaled(&svg, scale) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.raster_pending = false;
                match image {
                    Some(image) => {
                        if let Some((old, _)) = this.raster.replace((image, device_width)) {
                            cx.drop_image(old, None);
                        }
                    }
                    None => {
                        this.diagram =
                            Diagram::Failed("The diagram image could not be displayed.".into());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn button(
        &self,
        id: &'static str,
        icon: &'static str,
        label: &'static str,
        enabled: bool,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = self.colors;
        div()
            .id(id)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(26.0))
            .rounded(px(6.0))
            .when(enabled, |this| {
                this.cursor_pointer()
                    .hover(move |this| this.bg(colors.hover))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| on_click(this, cx)))
            })
            .when(!enabled, |this| this.opacity(0.4))
            .tooltip(move |window, cx| {
                gpui_component::tooltip::Tooltip::new(label).build(window, cx)
            })
            .child(svg().path(icon).size(px(15.0)).text_color(colors.text))
            .into_any_element()
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = ModalPalette::resolve(self.colors.light, None);
        let ready = self.raster.is_some() && matches!(self.diagram, Diagram::Ready { .. });
        let zoom = self.zoom;
        let mode = modal_segmented_control(
            &palette,
            "docs-mermaid-mode",
            &[
                ModalSegmentedItem {
                    icon: None,
                    label: "Diagram",
                },
                ModalSegmentedItem {
                    icon: None,
                    label: "Source",
                },
            ],
            usize::from(self.mode == Mode::Source),
            |this: &mut Self, index, _, cx| {
                this.mode = if index == 0 {
                    Mode::Diagram
                } else {
                    Mode::Source
                };
                cx.notify();
            },
            cx,
        );
        let mut actions = div().flex().items_center().gap(px(2.0));
        if self.mode == Mode::Diagram {
            actions = actions
                .child(self.button(
                    "docs-mermaid-zoom-out",
                    "modals/mermaid/minus.svg",
                    "Zoom out",
                    ready && zoom > MIN_ZOOM,
                    |this, cx| this.set_zoom(this.zoom / ZOOM_STEP, cx),
                    cx,
                ))
                .child(self.button(
                    "docs-mermaid-fit",
                    "modals/mermaid/focus-centered.svg",
                    "Fit diagram",
                    ready,
                    |this, cx| {
                        this.zoom = 1.0;
                        this.scroll.set_offset(point(px(0.0), px(0.0)));
                        cx.notify();
                    },
                    cx,
                ))
                .child(self.button(
                    "docs-mermaid-zoom-in",
                    "titlebar/plus.svg",
                    "Zoom in",
                    ready && zoom < MAX_ZOOM,
                    |this, cx| this.set_zoom(this.zoom * ZOOM_STEP, cx),
                    cx,
                ));
        }
        actions = actions
            .child(self.button(
                "docs-mermaid-copy",
                if self.copied {
                    "titlebar/check.svg"
                } else {
                    "titlebar/copy.svg"
                },
                if self.copied { "Copied" } else { "Copy source" },
                true,
                |this, cx| this.copy_source(cx),
                cx,
            ))
            .child(self.button(
                "docs-mermaid-expand",
                "files-view/t-arrows-maximize-2.svg",
                "Expand diagram",
                !matches!(self.diagram, Diagram::Rendering),
                |this, cx| (this.expand)(this.source.clone(), cx),
                cx,
            ));
        div()
            .flex_none()
            .h(px(TOOLBAR_HEIGHT))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.0))
            .px(px(6.0))
            .border_b_1()
            .border_color(self.colors.border)
            .child(div().w(px(160.0)).child(mode))
            .child(actions)
            .into_any_element()
    }

    fn status(&self, lines: Vec<(String, bool)>) -> AnyElement {
        let colors = self.colors;
        div()
            .p(px(VIEWPORT_PADDING))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .children(lines.into_iter().map(|(text, strong)| {
                div()
                    .text_color(if strong { colors.text } else { colors.muted })
                    .when(strong, |this| this.font_weight(gpui::FontWeight::SEMIBOLD))
                    .child(text)
            }))
            .into_any_element()
    }
}

impl Render for DocsMermaidWidget {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = self.colors;
        let card_width = self.width.get();
        // A change the editor's gap must follow (drawn, zoomed, Source) redraws the window, so
        // the editor asks the provider again.
        let layout = self.current_layout();
        if self.layout.replace(layout) != layout {
            window.refresh();
        }
        let toolbar = self.toolbar(cx);
        let viewport_height = self.viewport_height(card_width);
        let body: AnyElement = match self.mode {
            Mode::Source => div()
                .id("docs-mermaid-source")
                .h(px(viewport_height))
                .overflow_scroll()
                .p(px(VIEWPORT_PADDING))
                .font_family(colors.mono)
                .text_size(px(12.0))
                .line_height(px(SOURCE_LINE))
                .children(self.source.lines().map(|line| {
                    div().whitespace_nowrap().child(if line.is_empty() {
                        " ".to_string()
                    } else {
                        line.to_string()
                    })
                }))
                .into_any_element(),
            Mode::Diagram => {
                let display = (card_width > 0.0)
                    .then(|| self.display_size(card_width))
                    .flatten();
                if let Some((width, _)) = display {
                    self.request_raster(width, window.scale_factor(), cx);
                }
                let content = match (&self.diagram, display, &self.raster) {
                    (Diagram::Failed(error), _, _) => self.status(vec![
                        ("Could not render diagram".to_string(), true),
                        (error.clone(), false),
                        (
                            "Open Source to inspect the Mermaid text.".to_string(),
                            false,
                        ),
                    ]),
                    (_, Some((width, height)), Some((image, _))) => img(image.clone())
                        .flex_none()
                        .w(px(width))
                        .h(px(height))
                        .mx_auto()
                        .into_any_element(),
                    (Diagram::Ready { .. }, _, _) => {
                        self.status(vec![("Composing diagram…".to_string(), false)])
                    }
                    _ => self.status(vec![("Rendering diagram…".to_string(), false)]),
                };
                div()
                    .id("docs-mermaid-viewport")
                    .h(px(viewport_height))
                    .overflow_scroll()
                    .track_scroll(&self.scroll)
                    .p(px(VIEWPORT_PADDING))
                    .cursor(if self.drag.is_some() {
                        CursorStyle::ClosedHand
                    } else {
                        CursorStyle::OpenHand
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _, cx| {
                            this.drag = Some((event.position, this.scroll.offset()));
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        let Some((start, offset)) = this.drag else {
                            return;
                        };
                        if event.pressed_button != Some(MouseButton::Left) {
                            this.drag = None;
                            cx.notify();
                            return;
                        }
                        let max = this.scroll.max_offset();
                        let next = offset + (event.position - start);
                        this.scroll.set_offset(point(
                            next.x.clamp(-max.x, px(0.0)),
                            next.y.clamp(-max.y, px(0.0)),
                        ));
                        cx.notify();
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.drag = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.drag = None;
                            cx.notify();
                        }),
                    )
                    .child(content)
                    .into_any_element()
            }
        };
        let width = self.width.clone();
        let view = cx.weak_entity();
        div().size_full().py(px(CARD_MARGIN)).child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(px(8.0))
                .border_1()
                .border_color(colors.border)
                .bg(colors.background)
                .text_color(colors.text)
                .text_size(px(13.0))
                .line_height(px(19.5))
                .on_children_prepainted(move |bounds, _, cx| {
                    let measured = bounds
                        .first()
                        .map_or(0.0, |bounds| f32::from(bounds.size.width) + 2.0);
                    if (width.get() - measured).abs() > 0.5 {
                        width.set(measured);
                        let view = view.clone();
                        cx.defer(move |cx| {
                            let _ = view.update(cx, |_, cx| cx.notify());
                        });
                    }
                })
                .child(toolbar)
                .child(body),
        )
    }
}

/// What the card's height depends on, shared with the editor's view provider (which has no
/// `App` to read the card with).
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct MermaidLayout {
    source: bool,
    zoom: f32,
    /// The diagram's width over its height, once drawn.
    ratio: Option<f32>,
    failed: bool,
    source_lines: usize,
}

impl MermaidLayout {
    /// The height the editor keeps for the card at `width`, margins included.
    pub(crate) fn height(self, width: Pixels) -> Pixels {
        let width = f32::from(width);
        let content = if self.source {
            self.source_lines.max(1) as f32 * SOURCE_LINE
        } else if let Some(ratio) = self.ratio {
            let frame_width = (width - 2.0 - VIEWPORT_PADDING * 2.0).max(1.0);
            self.zoom * frame_width.min(FRAME_HEIGHT * ratio) / ratio
        } else if self.failed {
            3.0 * 19.5 + 12.0
        } else {
            19.5
        };
        let viewport = (content + VIEWPORT_PADDING * 2.0).clamp(VIEWPORT_MIN, VIEWPORT_MAX);
        px(CARD_MARGIN * 2.0 + 2.0 + TOOLBAR_HEIGHT + viewport)
    }
}
