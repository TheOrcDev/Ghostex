import ExpoModulesCore
import UIKit

/// `start(config)` and `command(json)` as synchronous JSI functions, the launch options, the
/// clipboard, the keyboard, and the `GpuiView` native view: the same module the Android side
/// defines (`android/.../GpuiViewModule.kt`), so the JavaScript runs unchanged.
public class GpuiViewModule: Module {
  public func definition() -> ModuleDefinition {
    Name("GpuiView")

    // Runs on the JavaScript thread; GPUI itself launches on the main thread when the first view
    // joins a window (GpuiHost).
    Function("start") { (configJson: String) -> Bool in
      var config =
        (try? JSONSerialization.jsonObject(with: Data(configJson.utf8))) as? [String: Any] ?? [:]
      let options = LaunchOptions.current
      // Dev hook, as on Android: `--gpuiContent demo` shows the synthetic list instead of the chat.
      if let content = options["gpuiContent"] {
        config["content"] = content
      }
      let json =
        (try? JSONSerialization.data(withJSONObject: config)).flatMap {
          String(data: $0, encoding: .utf8)
        } ?? "{}"
      if options["gpuiTestHook"] != nil {
        GpuiTestHook.install()
      }
      return GpuiHost.shared.start(filesDir: LaunchOptions.filesDirectory(), configJson: json)
    }

    Function("command") { (json: String) throws in
      do {
        try GpuiHost.shared.command(json)
      } catch let error as GpuiCommandError {
        throw Exception(name: "GpuiCommandRejected", description: error.message)
      }
    }

    // The launch arguments and environment the JavaScript side reads (`gpuiContent`,
    // `gpuiSession`), so a test can open a screen or a session:
    // `xcrun simctl launch <device> dev.ghostex.gpuipoc --gpuiSession <projectId:sessionId>`.
    Function("launchOptions") { () -> [String: String] in
      LaunchOptions.current
    }

    // The chat's copy actions arrive as `copy` events; the system clipboard is this one.
    Function("setClipboardText") { (text: String) -> Bool in
      DispatchQueue.main.async {
        UIPasteboard.general.string = text
      }
      return true
    }

    // Android pads the Activity's content for the keyboard; iOS extends the root view
    // controller's bottom safe area to the keyboard (KeyboardResize).
    Function("installKeyboardResize") { () -> Bool in
      DispatchQueue.main.async {
        KeyboardResize.install()
      }
      return true
    }

    View(GpuiView.self) {
      Events("onGpuiEvent")

      // Android's SurfaceView / TextureView switch; iOS has one kind of surface.
      Prop("surfaceType") { (_: GpuiView, _: String?) in }
    }
  }
}

/// Where the app's own files go, and the `gpui*` launch options.
enum LaunchOptions {
  /// `gpui*` options from the process arguments (`--gpuiSession value`, `-gpuiSession value` or
  /// `--gpuiSession=value`) and the environment (`SIMCTL_CHILD_gpuiSession=value` for
  /// `simctl launch`); arguments win.
  static let current: [String: String] = {
    var options: [String: String] = [:]
    for (key, value) in ProcessInfo.processInfo.environment where key.hasPrefix("gpui") {
      options[key] = value
    }
    let arguments = Array(ProcessInfo.processInfo.arguments.dropFirst())
    var index = 0
    while index < arguments.count {
      let argument = arguments[index]
      index += 1
      let name = argument.drop(while: { $0 == "-" })
      guard argument.hasPrefix("-"), name.hasPrefix("gpui") else { continue }
      if let equals = name.firstIndex(of: "=") {
        options[String(name[..<equals])] = String(name[name.index(after: equals)...])
      } else if index < arguments.count {
        options[String(name)] = arguments[index]
        index += 1
      }
    }
    return options
  }()

  /// Application Support: private to the app, backed up, not shown in the Files app.
  static func filesDirectory() -> String {
    let manager = FileManager.default
    let base =
      manager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? URL(fileURLWithPath: NSTemporaryDirectory())
    let directory = base.appendingPathComponent("gpui", isDirectory: true)
    try? manager.createDirectory(at: directory, withIntermediateDirectories: true)
    return directory.path
  }
}
