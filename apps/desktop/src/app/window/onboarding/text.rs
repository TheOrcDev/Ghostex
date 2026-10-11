//! One line of stage text with CSS letter spacing and, for the second heading line, a per-glyph
//! gradient.
//!
//! CDXC:Onboarding 2026-09-28 WHY:
//! GPUI text has no tracking control, and the onboarding's type depends on it: the eyebrows and
//! labels are tracked out (0.22-0.26em), the headings tracked in (-0.04em). The shaped glyph
//! advances are shifted per character instead, so kerning and native rasterisation stay intact.
//! The heading's second line is CSS gradient text (`background-clip: text`); GPUI paints one colour
//! per run, so each character takes the gradient's colour at its own position.
use super::stage::mix;
use gpui::{
    App, Bounds, Element, ElementId, FontWeight, GlobalElementId, Hsla, InspectorElementId,
    IntoElement, LayoutId, Pixels, ShapedLine, SharedString, Style, TextAlign, TextRun, Window,
    font, px, size,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) enum TextPaint {
    Solid(Hsla),
    /// Stops along the line's width (0..1).
    Gradient(Vec<(f32, Hsla)>),
}

pub(crate) struct TrackedText {
    text: SharedString,
    family: &'static str,
    weight: FontWeight,
    size: Pixels,
    line_height: Pixels,
    spacing: Pixels,
    paint: TextPaint,
}

/// `text` on one line at `size`, `line_height` tall, with `spacing` added after every character.
pub(crate) fn tracked(
    text: impl Into<SharedString>,
    family: &'static str,
    weight: f32,
    size: Pixels,
    line_height: Pixels,
    spacing: Pixels,
    color: Hsla,
) -> TrackedText {
    let (family, weight) = super::fonts::face(family, weight);
    TrackedText {
        text: text.into(),
        family,
        weight: FontWeight(weight),
        size,
        line_height,
        spacing,
        paint: TextPaint::Solid(color),
    }
}

impl TrackedText {
    pub(crate) fn gradient(mut self, stops: Vec<(f32, Hsla)>) -> Self {
        self.paint = TextPaint::Gradient(stops);
        self
    }
}

fn gradient_color(stops: &[(f32, Hsla)], at: f32) -> Hsla {
    let Some(first) = stops.first() else {
        return gpui::white();
    };
    if at <= first.0 {
        return first.1;
    }
    for pair in stops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if at <= b.0 {
            let span = (b.0 - a.0).max(1e-6);
            return mix(a.1, b.1, (at - a.0) / span);
        }
    }
    stops.last().map(|stop| stop.1).unwrap_or(first.1)
}

impl IntoElement for TrackedText {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for TrackedText {
    type RequestLayoutState = ShapedLine;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut text_font = font(self.family);
        text_font.weight = self.weight;
        let base = TextRun {
            len: self.text.len(),
            font: text_font.clone(),
            color: match &self.paint {
                TextPaint::Solid(color) => *color,
                TextPaint::Gradient(stops) => stops.first().map(|s| s.1).unwrap_or(gpui::white()),
            },
            ..Default::default()
        };
        let char_count = self.text.chars().count().max(1);
        let runs: Vec<TextRun> = match &self.paint {
            TextPaint::Solid(_) => vec![base],
            TextPaint::Gradient(_) => Vec::new(),
        };
        // A first shaping measures where each character sits; the gradient runs are laid on it.
        let mut line = window.text_system().shape_line(
            self.text.clone(),
            self.size,
            &[base_run(&self.text, &text_font)],
            None,
        );
        if let TextPaint::Gradient(stops) = &self.paint {
            let total = line.width + self.spacing * char_count as f32;
            let mut positions = vec![px(0.0); self.text.len() + 1];
            for run in &line.runs {
                for glyph in &run.glyphs {
                    if glyph.index < positions.len() {
                        positions[glyph.index] = glyph.position.x;
                    }
                }
            }
            let mut gradient_runs = Vec::new();
            for (char_index, (byte_index, ch)) in self.text.char_indices().enumerate() {
                let x = positions[byte_index] + self.spacing * char_index as f32;
                let center = x + self.size * 0.28;
                let at = if total > px(0.0) {
                    (center / total).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                gradient_runs.push(TextRun {
                    len: ch.len_utf8(),
                    font: text_font.clone(),
                    color: gradient_color(stops, at),
                    ..Default::default()
                });
            }
            line =
                window
                    .text_system()
                    .shape_line(self.text.clone(), self.size, &gradient_runs, None);
        } else {
            line = window
                .text_system()
                .shape_line(self.text.clone(), self.size, &runs, None);
        }
        if self.spacing != px(0.0) {
            let mut layout = gpui::LineLayout {
                font_size: line.font_size,
                width: line.width,
                ascent: line.ascent,
                descent: line.descent,
                runs: line.runs.clone(),
                len: line.len(),
            };
            for run in &mut layout.runs {
                for glyph in &mut run.glyphs {
                    let before = self.text[..glyph.index.min(self.text.len())]
                        .chars()
                        .count();
                    glyph.position.x += self.spacing * before as f32;
                }
            }
            layout.width += self.spacing * char_count as f32;
            *line = Arc::new(layout);
        }
        let style = Style {
            size: size(line.width.into(), self.line_height.into()),
            flex_shrink: 0.0,
            ..Default::default()
        };
        (window.request_layout(style, [], cx), line)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        line: &mut Self::RequestLayoutState,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let _ = line.paint(
            bounds.origin,
            self.line_height,
            TextAlign::Left,
            None,
            window,
            cx,
        );
    }
}

fn base_run(text: &str, text_font: &gpui::Font) -> TextRun {
    TextRun {
        len: text.len(),
        font: text_font.clone(),
        color: gpui::white(),
        ..Default::default()
    }
}
