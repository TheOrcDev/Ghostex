import UIKit

/// Keeps the React Native composer above the software keyboard without JavaScript.
///
/// iOS never resizes a window for the keyboard, and this app's screens pad their composer by the
/// bottom safe-area inset (`useSafeAreaInsets().bottom`). So while the keyboard is up the root view
/// controller's bottom safe area is extended to the keyboard's top edge: safe-area-context reports
/// the keyboard's height as the bottom inset, the composer's padding lifts it above the keyboard,
/// and flex shrinks the GPUI view above it, which GPUI follows with one resize per open or close
/// (the same as Android's `KeyboardResize`, which pads the content view by the keyboard inset).
///
/// CDXC:Mobile 2026-09-27 WHY: `installKeyboardResize()` is the JavaScript contract on both
/// platforms, so the keyboard is handled natively here instead of adding an iOS-only keyboard hook
/// to the screens (the real app lifts its composer with `useKeyboardInset`, which has the same
/// effect).
enum KeyboardResize {
  private static var installed = false

  static func install() {
    guard !installed else { return }
    installed = true
    let center = NotificationCenter.default
    center.addObserver(
      forName: UIResponder.keyboardWillChangeFrameNotification, object: nil, queue: .main
    ) { note in
      apply(note)
    }
    center.addObserver(
      forName: UIResponder.keyboardWillHideNotification, object: nil, queue: .main
    ) { note in
      apply(note, hiding: true)
    }
  }

  private static func apply(_ note: Notification, hiding: Bool = false) {
    guard let root = rootViewController(), let rootView = root.view,
      let window = rootView.window
    else { return }
    var overlap: CGFloat = 0
    if !hiding, let frame = note.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect {
      // The end frame is in screen coordinates; an undocked or floating keyboard (iPad) that
      // does not reach the bottom edge covers nothing the composer needs.
      let inWindow = window.convert(frame, from: window.screen.coordinateSpace)
      let inRoot = rootView.convert(inWindow, from: window)
      if inRoot.maxY >= rootView.bounds.maxY - 1 {
        overlap = max(0, rootView.bounds.maxY - inRoot.minY)
      }
    }
    // What the safe area would be without this adjustment (the home indicator strip).
    let base = rootView.safeAreaInsets.bottom - root.additionalSafeAreaInsets.bottom
    let extra = max(0, overlap - base)
    guard abs(root.additionalSafeAreaInsets.bottom - extra) >= 0.5 else { return }
    NSLog("[GpuiKeyboard] keyboard overlap %.0f pt: bottom safe area +%.0f", overlap, extra)
    root.additionalSafeAreaInsets.bottom = extra
  }

  private static func rootViewController() -> UIViewController? {
    let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
    let windows = scenes.flatMap(\.windows)
    let window = windows.first(where: \.isKeyWindow) ?? windows.first
    return window?.rootViewController
  }
}
