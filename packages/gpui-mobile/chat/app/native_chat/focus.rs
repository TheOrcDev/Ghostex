/// The desktop hands native keyboard focus back from its Chromium child views here. The phone's
/// surface has no Chromium child and no keyboard of its own (the composer is React Native), so
/// there is nothing to reclaim.
pub(super) fn reclaim_keyboard_focus(_window: &gpui::Window) {}
