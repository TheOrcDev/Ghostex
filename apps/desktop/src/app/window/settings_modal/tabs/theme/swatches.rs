//! The theme colour squares (`ThemeSwatchGrid` in theme-simple-controls.tsx (deleted 2026-10-01), `.theme-colour-*` in
//! packages/core-ui/styles/settings-theme.css). GPUI has no radial gradients, so each square's
//! `swatchStyle` background (two accent glows over a 150deg gradient) and its 160deg white sheen are
//! drawn into a small bitmap once per colour, appearance and Colourfulness step.
use super::colours::{Rgb, SwatchPaint};
use gpui::RenderImage;
use std::sync::Arc;

/// The bitmap's side in pixels; the square is drawn about 37px wide at 1x and 56px at 1.5x.
const SIZE: u32 = 64;

fn over(color: &mut [f64; 3], source: [f64; 3], alpha: f64) {
    for index in 0..3 {
        color[index] += (source[index] - color[index]) * alpha;
    }
}

/// `radial-gradient(<rx> <ry> at <cx> <cy>, accent <alpha> 0%, accent 0 <end>)` at `(x, y)`, as the
/// alpha of the accent there (premultiplied interpolation makes it linear in alpha).
fn radial(x: f64, y: f64, center: (f64, f64), radii: (f64, f64), alpha: f64, end: f64) -> f64 {
    let distance = (((x - center.0) / radii.0).powi(2) + ((y - center.1) / radii.1).powi(2)).sqrt();
    alpha * (1.0 - (distance / end).clamp(0.0, 1.0))
}

/// Where `(x, y)` falls on a CSS linear gradient of `angle` degrees in a unit square (0 at the
/// start edge, 1 at the end edge).
fn linear(x: f64, y: f64, angle: f64) -> f64 {
    let (sin, cos) = angle.to_radians().sin_cos();
    let length = sin.abs() + cos.abs();
    ((x - 0.5) * sin - (y - 0.5) * cos) / length + 0.5
}

/// The square for one colour at one step.
pub(super) fn swatch_image(paint: &SwatchPaint) -> Option<Arc<RenderImage>> {
    let Rgb(top) = paint.top;
    let Rgb(base) = paint.base;
    let Rgb(accent) = paint.accent;
    let mut bgra = vec![0u8; (SIZE * SIZE * 4) as usize];
    for row in 0..SIZE {
        let y = (row as f64 + 0.5) / SIZE as f64;
        for column in 0..SIZE {
            let x = (column as f64 + 0.5) / SIZE as f64;
            // linear-gradient(150deg, top 0%, base 88%)
            let t = (linear(x, y, 150.0) / 0.88).clamp(0.0, 1.0);
            let mut color = [0, 1, 2].map(|index| top[index] + (base[index] - top[index]) * t);
            // radial-gradient(90% 70% at 100% 100%, accent rim 0%, transparent 70%) under
            // radial-gradient(130% 100% at 20% 8%, accent glow 0%, transparent 62%).
            over(
                &mut color,
                accent,
                radial(x, y, (1.0, 1.0), (0.9, 0.7), paint.rim, 0.7),
            );
            over(
                &mut color,
                accent,
                radial(x, y, (0.2, 0.08), (1.3, 1.0), paint.glow, 0.62),
            );
            // `::after`: linear-gradient(160deg, rgba(255,255,255,.16) 0%, transparent 42%).
            let sheen = linear(x, y, 160.0);
            over(
                &mut color,
                [255.0; 3],
                0.16 * (1.0 - (sheen / 0.42).clamp(0.0, 1.0)),
            );
            let index = ((row * SIZE + column) * 4) as usize;
            bgra[index] = color[2].round().clamp(0.0, 255.0) as u8;
            bgra[index + 1] = color[1].round().clamp(0.0, 255.0) as u8;
            bgra[index + 2] = color[0].round().clamp(0.0, 255.0) as u8;
            bgra[index + 3] = 255;
        }
    }
    let buffer = image::RgbaImage::from_raw(SIZE, SIZE, bgra)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}
