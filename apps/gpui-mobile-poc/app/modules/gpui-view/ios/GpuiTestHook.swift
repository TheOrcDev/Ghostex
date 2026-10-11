import UIKit

/// Test-only remote control, off unless the app was launched with `--gpuiTestHook 1`.
///
/// The simulator has no command line for taps, so tests drive the app through the host protocol
/// instead: write a command object (or an array of them) to `<files>/test/command.json` in the
/// app's data container, then post the Darwin notification `dev.ghostex.gpuipoc.command`
/// (`xcrun simctl spawn <device> notifyutil -p dev.ghostex.gpuipoc.command`). Commands go to GPUI
/// exactly as JavaScript's `command()` sends them, except these, handled here:
///
/// - `__focusComposer`: the first text view on screen (the React Native composer) takes focus,
///   which opens the keyboard. `__insertText {text}` types into it; `__dismissKeyboard` closes it.
/// - `__rotate {orientation: "portrait" | "landscapeLeft" | "landscapeRight"}`.
///
/// With the hook on, every event is also logged (`[GpuiEvent] ...` in the unified log).
enum GpuiTestHook {
  static private(set) var enabled = false
  private static let notification = "dev.ghostex.gpuipoc.command"

  static func install() {
    DispatchQueue.main.async {
      guard !enabled else { return }
      enabled = true
      let center = CFNotificationCenterGetDarwinNotifyCenter()
      CFNotificationCenterAddObserver(
        center, nil,
        { _, _, _, _, _ in
          DispatchQueue.main.async { GpuiTestHook.run() }
        },
        notification as CFString, nil, .deliverImmediately)
      NSLog("[GpuiTestHook] listening for %@ (commands in %@)", notification, commandFile().path)
    }
  }

  static func commandFile() -> URL {
    URL(fileURLWithPath: LaunchOptions.filesDirectory())
      .appendingPathComponent("test", isDirectory: true)
      .appendingPathComponent("command.json")
  }

  private static func run() {
    guard let data = try? Data(contentsOf: commandFile()),
      let parsed = try? JSONSerialization.jsonObject(with: data)
    else {
      NSLog("[GpuiTestHook] no readable %@", commandFile().path)
      return
    }
    let commands = (parsed as? [[String: Any]]) ?? [(parsed as? [String: Any]) ?? [:]]
    for command in commands {
      perform(command)
    }
  }

  private static func perform(_ command: [String: Any]) {
    let type = command["type"] as? String ?? ""
    NSLog("[GpuiTestHook] %@", type)
    switch type {
    case "__focusComposer":
      firstTextInput(in: keyWindow())?.becomeFirstResponder()
    case "__insertText":
      let text = command["text"] as? String ?? ""
      (firstTextInput(in: keyWindow()) as? UIKeyInput)?.insertText(text)
    case "__dismissKeyboard":
      keyWindow()?.endEditing(true)
    case "__rotate":
      let mask: UIInterfaceOrientationMask
      switch command["orientation"] as? String {
      case "landscapeLeft": mask = .landscapeLeft
      case "landscapeRight": mask = .landscapeRight
      default: mask = .portrait
      }
      keyWindow()?.windowScene?.requestGeometryUpdate(.iOS(interfaceOrientations: mask)) { error in
        NSLog("[GpuiTestHook] rotation refused: %@", error.localizedDescription)
      }
    default:
      guard let data = try? JSONSerialization.data(withJSONObject: command),
        let json = String(data: data, encoding: .utf8)
      else { return }
      do {
        try GpuiHost.shared.command(json)
      } catch {
        NSLog("[GpuiTestHook] command rejected: %@", String(describing: error))
      }
    }
  }

  private static func keyWindow() -> UIWindow? {
    let windows = UIApplication.shared.connectedScenes
      .compactMap { $0 as? UIWindowScene }
      .flatMap(\.windows)
    return windows.first(where: \.isKeyWindow) ?? windows.first
  }

  private static func firstTextInput(in view: UIView?) -> UIView? {
    guard let view else { return nil }
    if view is UITextView || view is UITextField, !view.isHidden, view.alpha > 0.05 {
      return view
    }
    for child in view.subviews {
      if let found = firstTextInput(in: child) {
        return found
      }
    }
    return nil
  }
}
