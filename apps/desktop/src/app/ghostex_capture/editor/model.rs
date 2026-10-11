//! What the editor edits: the picture, the marks on it, and a pending crop box, all in the
//! picture's own pixels so the display size never changes what gets exported.

use std::sync::Arc;

use image::RgbaImage;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct P {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl P {
    pub(crate) fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned box from two corners in any order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Area {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) right: f32,
    pub(crate) bottom: f32,
}

impl Area {
    pub(crate) fn from_points(a: P, b: P) -> Self {
        Self {
            left: a.x.min(b.x),
            top: a.y.min(b.y),
            right: a.x.max(b.x),
            bottom: a.y.max(b.y),
        }
    }

    pub(crate) fn width(&self) -> f32 {
        self.right - self.left
    }

    pub(crate) fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub(crate) fn contains(&self, p: P) -> bool {
        p.x >= self.left && p.x <= self.right && p.y >= self.top && p.y <= self.bottom
    }

    pub(crate) fn clamp_to(&self, width: f32, height: f32) -> Self {
        Self {
            left: self.left.clamp(0.0, width),
            top: self.top.clamp(0.0, height),
            right: self.right.clamp(0.0, width),
            bottom: self.bottom.clamp(0.0, height),
        }
    }

    pub(crate) fn translated(&self, dx: f32, dy: f32) -> Self {
        Self {
            left: self.left + dx,
            top: self.top + dy,
            right: self.right + dx,
            bottom: self.bottom + dy,
        }
    }
}

/// Which corner or edge of a box a handle drags; `(x, y)` in -1..=1 steps (0 means that axis is
/// left alone).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Handle {
    pub(crate) x: i8,
    pub(crate) y: i8,
}

pub(crate) const BOX_HANDLES: [Handle; 8] = [
    Handle { x: -1, y: -1 },
    Handle { x: 0, y: -1 },
    Handle { x: 1, y: -1 },
    Handle { x: 1, y: 0 },
    Handle { x: 1, y: 1 },
    Handle { x: 0, y: 1 },
    Handle { x: -1, y: 1 },
    Handle { x: -1, y: 0 },
];

/// The handles at the middle of the picture's four sides that crop it from that side (top,
/// right, bottom, left).
///
/// CDXC:GhostexCapture 2026-09-30 DECISION:
/// User: "when i select the arrow i want to see 4 dots on the 4 sides centered on the image so i
/// can drag from any side to crop that way", and "i dont need crop to allow cropping outside": a
/// crop never reaches past the picture. This replaces the earlier decision that let the crop box
/// grow past the picture's edges to add #161616 space around it.
pub(crate) const EDGE_HANDLES: [Handle; 4] = [
    Handle { x: 0, y: -1 },
    Handle { x: 1, y: 0 },
    Handle { x: 0, y: 1 },
    Handle { x: -1, y: 0 },
];

pub(crate) fn handle_point(area: &Area, handle: Handle) -> P {
    let x = match handle.x {
        -1 => area.left,
        1 => area.right,
        _ => (area.left + area.right) / 2.0,
    };
    let y = match handle.y {
        -1 => area.top,
        1 => area.bottom,
        _ => (area.top + area.bottom) / 2.0,
    };
    P::new(x, y)
}

pub(crate) fn drag_handle(area: &Area, handle: Handle, to: P) -> Area {
    let mut next = *area;
    match handle.x {
        -1 => next.left = to.x,
        1 => next.right = to.x,
        _ => {}
    }
    match handle.y {
        -1 => next.top = to.y,
        1 => next.bottom = to.y,
        _ => {}
    }
    Area::from_points(P::new(next.left, next.top), P::new(next.right, next.bottom))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tool {
    Pointer,
    Crop,
    Arrow,
    Text,
    Rect,
}

pub(crate) const COLORS: [u32; 4] = [0xef4444, 0xfacc15, 0x3b82f6, 0xffffff];

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Mark {
    Arrow {
        from: P,
        to: P,
        color: u32,
    },
    Rect {
        area: Area,
        color: u32,
    },
    Text {
        at: P,
        text: String,
        color: u32,
        /// A dark rounded box behind the text.
        background: bool,
    },
}

impl Mark {
    pub(crate) fn translate(&mut self, dx: f32, dy: f32) {
        match self {
            Mark::Arrow { from, to, .. } => {
                *from = P::new(from.x + dx, from.y + dy);
                *to = P::new(to.x + dx, to.y + dy);
            }
            Mark::Rect { area, .. } => *area = area.translated(dx, dy),
            Mark::Text { at, .. } => *at = P::new(at.x + dx, at.y + dy),
        }
    }
}

/// Sizes of marks in picture pixels, from their size in points on screen.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Metrics {
    /// Picture pixels per point of the screen it came from.
    pub(crate) px_per_pt: f32,
}

impl Metrics {
    pub(crate) fn stroke(&self) -> f32 {
        3.0 * self.px_per_pt
    }

    pub(crate) fn text_size(&self) -> f32 {
        20.0 * self.px_per_pt
    }

    pub(crate) fn arrow_head(&self) -> f32 {
        16.0 * self.px_per_pt
    }

    /// The box a text mark occupies (an estimate: the export draws the real glyphs).
    pub(crate) fn text_area(&self, at: P, text: &str) -> Area {
        let size = self.text_size();
        let widest = text
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
            .max(1);
        let lines = text.lines().count().max(1);
        Area {
            left: at.x,
            top: at.y,
            right: at.x + widest as f32 * size * 0.58 + size * 0.4,
            bottom: at.y + lines as f32 * size * 1.25,
        }
    }
}

fn distance_to_segment(p: P, a: P, b: P) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0.0, 1.0)
    };
    let (x, y) = (a.x + t * dx, a.y + t * dy);
    ((p.x - x).powi(2) + (p.y - y).powi(2)).sqrt()
}

/// The topmost mark under `p`, within `slop` picture pixels.
pub(crate) fn hit_mark(marks: &[Mark], metrics: Metrics, p: P, slop: f32) -> Option<usize> {
    marks.iter().enumerate().rev().find_map(|(index, mark)| {
        let hit = match mark {
            Mark::Arrow { from, to, .. } => distance_to_segment(p, *from, *to) <= slop,
            Mark::Rect { area, .. } => {
                let outer = Area {
                    left: area.left - slop,
                    top: area.top - slop,
                    right: area.right + slop,
                    bottom: area.bottom + slop,
                };
                let inner = Area {
                    left: area.left + slop,
                    top: area.top + slop,
                    right: area.right - slop,
                    bottom: area.bottom - slop,
                };
                outer.contains(p)
                    && !(inner.width() > 0.0 && inner.height() > 0.0 && inner.contains(p))
            }
            Mark::Text { at, text, .. } => metrics.text_area(*at, text).contains(p),
        };
        hit.then_some(index)
    })
}

/// What a press inside the picture grabbed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Drag {
    /// Drawing a new crop box from its first corner.
    CropNew {
        from: P,
    },
    CropHandle {
        handle: Handle,
        start: Area,
    },
    CropMove {
        from: P,
        start: Area,
    },
    /// One of the Pointer's side handles, cropping the picture from that side on release.
    EdgeCrop {
        handle: Handle,
    },
    /// Drawing a new arrow or rectangle; the mark is already in the list at `index`.
    Draw {
        index: usize,
        from: P,
    },
    MoveMark {
        index: usize,
        from: P,
        start: Mark,
    },
    ArrowEnd {
        index: usize,
        head: bool,
    },
    RectHandle {
        index: usize,
        handle: Handle,
        start: Area,
    },
}

/// A picture as the editor left it: cropped, with its marks still separate, so it can be edited
/// again from the prompt box.
#[derive(Clone)]
pub(crate) struct EditSource {
    pub(crate) image: Arc<RgbaImage>,
    pub(crate) marks: Vec<Mark>,
    pub(crate) px_per_pt: f32,
}

/// The document one editor window works on.
pub(crate) struct EditorDoc {
    pub(crate) image: Arc<RgbaImage>,
    pub(crate) metrics: Metrics,
    pub(crate) tool: Tool,
    pub(crate) color: u32,
    /// Whether new text gets a background box.
    pub(crate) text_background: bool,
    pub(crate) marks: Vec<Mark>,
    /// The marks the picture had when the editor opened; changing them counts as an edit.
    pub(crate) opened_marks: Vec<Mark>,
    pub(crate) selected: Option<usize>,
    pub(crate) crop: Option<Area>,
    pub(crate) drag: Option<Drag>,
    pub(crate) editing_text: Option<usize>,
    pub(crate) undo: Vec<(Arc<RgbaImage>, Vec<Mark>)>,
}

impl EditorDoc {
    pub(crate) fn new(image: Arc<RgbaImage>, px_per_pt: f32) -> Self {
        Self {
            image,
            metrics: Metrics {
                px_per_pt: px_per_pt.max(0.5),
            },
            // CDXC:GhostexCapture 2026-09-30 DECISION:
            // User: "i want crop always selected by default to begin with"; dragging draws a crop
            // box that can be resized afterwards, and Enter with no box switches to the Pointer so a
            // second Enter adds the picture to the prompt.
            tool: Tool::Crop,
            color: COLORS[0],
            text_background: false,
            marks: Vec::new(),
            opened_marks: Vec::new(),
            selected: None,
            crop: None,
            drag: None,
            editing_text: None,
            undo: Vec::new(),
        }
    }

    pub(crate) fn size(&self) -> (f32, f32) {
        (self.image.width() as f32, self.image.height() as f32)
    }

    /// The whole picture as a box.
    pub(crate) fn full(&self) -> Area {
        let (width, height) = self.size();
        Area {
            left: 0.0,
            top: 0.0,
            right: width,
            bottom: height,
        }
    }

    pub(crate) fn checkpoint(&mut self) {
        self.undo.push((self.image.clone(), self.marks.clone()));
        if self.undo.len() > 50 {
            self.undo.remove(0);
        }
    }

    pub(crate) fn undo(&mut self) {
        if let Some((image, marks)) = self.undo.pop() {
            self.image = image;
            self.marks = marks;
            self.selected = None;
            self.crop = None;
            self.editing_text = None;
            self.drag = None;
        }
    }

    /// Crops the picture to the pending box and moves the marks with it.
    pub(crate) fn apply_crop(&mut self) -> bool {
        let Some(crop) = self.crop.take() else {
            return false;
        };
        if crop.width() < 2.0 || crop.height() < 2.0 {
            return false;
        }
        let (width, height) = self.size();
        let crop = crop.clamp_to(width, height);
        if crop.width() < 2.0 || crop.height() < 2.0 || crop == self.full() {
            return false;
        }
        self.checkpoint();
        self.image = Arc::new(
            image::imageops::crop_imm(
                self.image.as_ref(),
                crop.left.round() as u32,
                crop.top.round() as u32,
                crop.width().round() as u32,
                crop.height().round() as u32,
            )
            .to_image(),
        );
        for mark in &mut self.marks {
            mark.translate(-crop.left.round(), -crop.top.round());
        }
        self.selected = None;
        true
    }

    /// Ends text editing, dropping a label that stayed empty.
    pub(crate) fn finish_text(&mut self) {
        let Some(index) = self.editing_text.take() else {
            return;
        };
        if let Some(Mark::Text { text, .. }) = self.marks.get(index)
            && text.trim().is_empty()
        {
            self.marks.remove(index);
            self.selected = None;
        }
    }
}
