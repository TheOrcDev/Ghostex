/// The desktop hands native keyboard focus back from its Chromium child views here. A page has one keyboard owner, the canvas, so there is nothing to reclaim.
pub(super) fn reclaim_keyboard_focus(_window: &gpui::Window) {}
