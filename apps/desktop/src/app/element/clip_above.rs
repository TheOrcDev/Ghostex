//! A wrapper that stops its child painting above a given window y, for content that scrolls under
//! a see-through pinned row.

use gpui::{
    AnyElement, App, Bounds, ContentMask, Element, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Window, point,
};

/// Paints `child` only below `top` (window coordinates); `None` leaves it untouched.
pub(crate) fn clip_above(top: Option<Pixels>, child: impl IntoElement) -> ClipAbove {
    ClipAbove {
        top,
        child: child.into_any_element(),
    }
}

pub(crate) struct ClipAbove {
    top: Option<Pixels>,
    child: AnyElement,
}

impl Element for ClipAbove {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(top) = self.top.filter(|top| *top > bounds.top()) else {
            self.child.paint(window, cx);
            return;
        };
        let clipped = Bounds::from_corners(point(bounds.left(), top), bounds.bottom_right());
        window.with_content_mask(Some(ContentMask { bounds: clipped }), |window| {
            self.child.paint(window, cx);
        });
    }
}

impl IntoElement for ClipAbove {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}
