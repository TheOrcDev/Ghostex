//! Images pasted into the feedback message: what the relay accepts, and the thumbnail strip.

use super::state::GpuiFeedbackModalWindow;
use crate::app::window::native_modal_kit::*;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClipboardEntry, ClipboardItem, Context, Image, ImageFormat,
    InteractiveElement as _, IntoElement, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, StyledImage as _, WeakEntity, Window, div, img, px,
};
use std::sync::Arc;

/// The relay's limits (`LIMITS` in its `src/validate.ts`, checked again by gxserver).
pub(super) const FEEDBACK_MAX_IMAGES: usize = 5;
const FEEDBACK_IMAGE_MAX_BYTES: usize = 5 * 1024 * 1024;
/// The chat composer's thumbnail size.
const TILE_PX: f32 = 48.0;
const TILE_GROUP: &str = "feedback-image";

impl GpuiFeedbackModalWindow {
    /// The message field's paste hook: pasted images are attached, anything else is typed as text.
    /// The chat composer takes images through the same hook, which is how a browser paste reaches
    /// the web build too (native_chat/clipboard.rs).
    pub(super) fn paste_handler(
        modal: WeakEntity<Self>,
    ) -> impl Fn(&ClipboardItem, &mut Window, &mut App) -> bool + 'static {
        move |clipboard, _window, cx| {
            modal
                .update(cx, |modal, cx| modal.paste_images(clipboard, cx))
                .unwrap_or(false)
        }
    }

    fn paste_images(&mut self, clipboard: &ClipboardItem, cx: &mut Context<Self>) -> bool {
        let pasted: Vec<Image> = clipboard
            .entries
            .iter()
            .filter_map(|entry| match entry {
                ClipboardEntry::Image(image) => Some(image.clone()),
                _ => None,
            })
            .collect();
        if pasted.is_empty() {
            return false;
        }
        self.image_error = None;
        for image in pasted {
            if self.images.len() + self.pending_images >= FEEDBACK_MAX_IMAGES {
                self.image_error = Some(format!(
                    "You can attach up to {FEEDBACK_MAX_IMAGES} images."
                ));
                break;
            }
            self.pending_images += 1;
            let prepared = cx
                .background_executor()
                .spawn(async move { prepare_feedback_image(image) });
            cx.spawn(async move |this, cx| {
                let prepared = prepared.await;
                let _ = this.update(cx, |this, cx| {
                    this.pending_images -= 1;
                    match prepared {
                        Ok(image) => this.images.push(Arc::new(image)),
                        Err(error) => this.image_error = Some(error),
                    }
                    cx.notify();
                });
            })
            .detach();
        }
        cx.notify();
        true
    }

    /// The pasted images as 48px tiles, each with a remove button that shows while it is hovered,
    /// plus a dashed tile per image still being prepared.
    pub(super) fn render_images(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.images.is_empty() && self.pending_images == 0 {
            return None;
        }
        let p = self.palette;
        let mut row = div()
            .id("feedback-images")
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.0))
            .pt(px(4.0));
        for (index, image) in self.images.iter().enumerate() {
            row = row.child(
                div()
                    .relative()
                    .flex_shrink_0()
                    .group(TILE_GROUP)
                    .size(px(TILE_PX))
                    .child(
                        div()
                            .size_full()
                            .rounded(px(8.0))
                            .overflow_hidden()
                            .border_1()
                            .border_color(hsla(p.hairline))
                            .child(
                                img(image.clone())
                                    .size_full()
                                    .object_fit(gpui::ObjectFit::Cover),
                            ),
                    )
                    .child(
                        div()
                            .id(("feedback-image-remove", index))
                            .role(gpui::Role::Button)
                            .aria_label(format!("Remove image {}", index + 1))
                            .invisible()
                            .group_hover(TILE_GROUP, |style| style.visible())
                            .absolute()
                            .top(px(-5.0))
                            .right(px(-5.0))
                            .size(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_color(hsla(p.hairline))
                            .bg(hsla(p.solid_surface))
                            .cursor_pointer()
                            .hover(move |style| style.visible().bg(hsla(p.raised_hover)))
                            .when(!self.busy, |button| {
                                button.on_press(cx, move |this, _window, cx| {
                                    if index < this.images.len() {
                                        this.images.remove(index);
                                        this.image_error = None;
                                        cx.notify();
                                    }
                                })
                            })
                            .child(modal_icon("modals/kit/x.svg", 9.0, p.muted)),
                    ),
            );
        }
        for _ in 0..self.pending_images {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .size(px(TILE_PX))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.0))
                    .border_dashed()
                    .border_1()
                    .border_color(hsla(p.hairline))
                    .child(modal_spinner(p.muted)),
            );
        }
        Some(row.into_any_element())
    }
}

/// An image the relay accepts: PNG, JPEG or WebP of at most 5 MB. A larger PNG (a full-screen
/// Retina screenshot is often 6-12 MB) is scaled down until it fits instead of being refused.
fn prepare_feedback_image(image: Image) -> Result<Image, String> {
    match image.format() {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Webp => {}
        _ => return Err("Only PNG, JPEG and WebP images can be attached.".to_string()),
    }
    if image.bytes().len() <= FEEDBACK_IMAGE_MAX_BYTES {
        return Ok(image);
    }
    let too_large = || "Images must be 5 MB or smaller.".to_string();
    if image.format() != ImageFormat::Png {
        return Err(too_large());
    }
    let decoded = image::load_from_memory_with_format(image.bytes(), image::ImageFormat::Png)
        .map_err(|_| "That image could not be read.".to_string())?;
    // PNG size grows roughly with the pixel count, so the first try aims straight at the limit.
    let mut scale = (FEEDBACK_IMAGE_MAX_BYTES as f64 / image.bytes().len() as f64).sqrt() * 0.9;
    for _ in 0..4 {
        let width = ((decoded.width() as f64) * scale).round().max(1.0) as u32;
        let height = ((decoded.height() as f64) * scale).round().max(1.0) as u32;
        let mut encoded = std::io::Cursor::new(Vec::new());
        decoded
            .resize(width, height, image::imageops::FilterType::Triangle)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .map_err(|_| too_large())?;
        let bytes = encoded.into_inner();
        if bytes.len() <= FEEDBACK_IMAGE_MAX_BYTES {
            return Ok(Image::from_bytes(ImageFormat::Png, bytes));
        }
        scale *= 0.75;
    }
    Err(too_large())
}
