import GhostexGpuiMobile
import QuartzCore
import UIKit

/// The one GPUI window of the process and what drives it.
///
/// GPUI runs on the main thread on iOS. Its window is a `UIViewController` whose view is a
/// `CAMetalLayer`-backed `UIView` (gpui-mobile's embedded mode); this host adds that controller as
/// a child of whichever `GpuiView` is on screen, keeps its frame equal to the view's bounds, gives
/// GPUI frames from a `CADisplayLink` that pauses while GPUI has nothing to draw, and forwards the
/// application's lifecycle. Touches need nothing here: UIKit delivers them to GPUI's view, which
/// hands them to GPUI's gesture recognizer.
///
/// Like the Android host, GPUI launches when the first view joins a window after `start`, so the
/// `ready` event has a listener; it then lives for the process, across JavaScript reloads.
final class GpuiHost {
  static let shared = GpuiHost()

  /// gpui-mobile's `IosWindow`, the pointer every `gpui_ios_*` call takes. Main thread only.
  private var window: UnsafeMutableRawPointer?
  private var controller: UIViewController?
  /// The view GPUI's controller is attached to.
  private weak var host: GpuiView?
  /// Every mounted view, for events.
  private let views = NSHashTable<GpuiView>.weakObjects()
  private var displayLink: CADisplayLink?
  private var inBackground = false
  /// `start` has run; read on the main thread.
  private var started = false
  private let startLock = NSLock()
  private var callbackInstalled = false

  private init() {
    let center = NotificationCenter.default
    center.addObserver(
      self, selector: #selector(willEnterForeground),
      name: UIApplication.willEnterForegroundNotification, object: nil)
    center.addObserver(
      self, selector: #selector(didBecomeActive),
      name: UIApplication.didBecomeActiveNotification, object: nil)
    center.addObserver(
      self, selector: #selector(willResignActive),
      name: UIApplication.willResignActiveNotification, object: nil)
    center.addObserver(
      self, selector: #selector(didEnterBackground),
      name: UIApplication.didEnterBackgroundNotification, object: nil)
  }

  // MARK: - Start and commands (any thread)

  /// Starts the Rust host once per process. `false` when it was already running (a reloaded
  /// bundle); the first configuration stays in force.
  func start(filesDir: String, configJson: String) -> Bool {
    startLock.lock()
    if !callbackInstalled {
      callbackInstalled = true
      ghostex_gpui_set_event_callback(gpuiEventCallback, nil)
    }
    startLock.unlock()
    let first = ghostex_gpui_start(filesDir, configJson)
    DispatchQueue.main.async {
      self.started = true
      self.inBackground = UIApplication.shared.applicationState == .background
      self.attachPendingView()
    }
    return first
  }

  /// Queues one JSON command. Throws the library's message when it refused it.
  func command(_ json: String) throws {
    if let error = ghostex_gpui_command(json) {
      let message = String(cString: error)
      ghostex_gpui_string_free(error)
      throw GpuiCommandError(message: message)
    }
  }

  // MARK: - Views (main thread)

  /// Events go to a view from when React Native mounts it until it is unmounted, whether or not
  /// it is in a window: a full-screen modal (the session picker) takes the screen's views out of
  /// the window while it is up, and the chat's events must still reach JavaScript meanwhile.
  func viewDidMount(_ view: GpuiView) {
    views.add(view)
  }

  func viewDidUnmount(_ view: GpuiView) {
    views.remove(view)
  }

  /// GPUI draws into the view that is in a window; the frames stop while none is.
  func viewDidJoinWindow(_ view: GpuiView) {
    views.add(view)
    attach(view)
  }

  func viewDidLeaveWindow(_ view: GpuiView) {
    if host === view {
      displayLink?.isPaused = true
    }
  }

  /// The view's bounds changed: GPUI's view follows, and the frame at the new size is drawn in the
  /// same Core Animation transaction so the old drawable is never shown stretched or clipped.
  func layout(_ view: GpuiView, force: Bool = false) {
    guard view === host, let window, let controller else { return }
    let bounds = view.bounds
    guard bounds.width > 0, bounds.height > 0, force || controller.view.frame != bounds else {
      return
    }
    CATransaction.begin()
    CATransaction.setDisableActions(true)
    controller.view.frame = bounds
    gpui_ios_layout_view(window)
    if !inBackground {
      if gpui_ios_request_frame(window) {
        resumeFrames()
      }
    }
    CATransaction.commit()
  }

  private func attachPendingView() {
    let mounted = views.allObjects.filter { $0.window != nil }
    if let view = mounted.last, host == nil {
      attach(view)
    }
  }

  private func attach(_ view: GpuiView) {
    guard started, view.window != nil else { return }
    guard ensureWindow() != nil, let controller else { return }
    if controller.view.superview !== view {
      if controller.parent != nil {
        controller.willMove(toParent: nil)
        controller.view.removeFromSuperview()
        controller.removeFromParent()
      } else {
        controller.view.removeFromSuperview()
      }
      let parent = view.owningViewController
      parent?.addChild(controller)
      controller.view.frame = view.bounds
      view.addSubview(controller.view)
      controller.didMove(toParent: parent)
      NSLog("[GpuiView] attached GPUI to a %.0fx%.0f view", view.bounds.width, view.bounds.height)
    }
    host = view
    // A view joining a window may already have its final size; GPUI takes it and draws.
    layout(view, force: true)
    resumeFrames()
  }

  /// Launches GPUI on first use and wires its frame waker.
  private func ensureWindow() -> UnsafeMutableRawPointer? {
    if let window { return window }
    guard let created = ghostex_gpui_window() else {
      NSLog("[GpuiView] GPUI did not launch (see the GhostexGpui log)")
      return nil
    }
    guard let rawController = gpui_ios_view_controller(created) else {
      NSLog("[GpuiView] GPUI window has no view controller")
      return nil
    }
    window = created
    let controller = Unmanaged<UIViewController>.fromOpaque(rawController).takeUnretainedValue()
    controller.view.backgroundColor = .black
    controller.view.clipsToBounds = true
    self.controller = controller
    gpui_ios_set_frame_waker(created, gpuiFrameWaker, nil)
    return created
  }

  // MARK: - Frames

  fileprivate func resumeFrames() {
    guard window != nil, !inBackground else { return }
    if displayLink == nil {
      let link = CADisplayLink(target: DisplayLinkTarget(), selector: #selector(DisplayLinkTarget.tick))
      // ProMotion phones draw up to 120 Hz (Info.plist CADisableMinimumFrameDurationOnPhone).
      link.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
      // `.common`: keep ticking while UIKit tracks a touch or a scroll view scrolls.
      link.add(to: .main, forMode: .common)
      displayLink = link
    }
    displayLink?.isPaused = false
  }

  fileprivate func tick() {
    guard let window, !inBackground, host?.window != nil else {
      displayLink?.isPaused = true
      return
    }
    let wantsMore = gpui_ios_request_frame(window)
    if GpuiTestHook.enabled {
      tickStats.record(CACurrentMediaTime(), idle: !wantsMore)
    }
    if !wantsMore {
      // Idle: GPUI calls the waker when it wants frames again.
      displayLink?.isPaused = true
    }
  }

  /// Test hook only: display link ticks per second, next to GPUI's own `frameStats` (which counts
  /// the frames GPUI drew), to tell a blocked main thread from ticks where nothing changed.
  private var tickStats = TickStats()

  // MARK: - Events

  fileprivate func deliver(_ json: String) {
    if GpuiTestHook.enabled {
      NSLog("[GpuiEvent] %@", String(json.prefix(400)))
    }
    let targets = views.allObjects
    if targets.isEmpty {
      NSLog("[GpuiView] event with no listener: %@", String(json.prefix(200)))
      return
    }
    for view in targets {
      view.handleGpuiEvent(json)
    }
  }

  // MARK: - Lifecycle

  @objc private func willEnterForeground() {
    inBackground = false
    guard let window else { return }
    gpui_ios_will_enter_foreground(nil)
    // A frame for the snapshot UIKit shows while the app comes back.
    _ = gpui_ios_request_frame(window)
    resumeFrames()
  }

  @objc private func didBecomeActive() {
    inBackground = false
    guard window != nil else { return }
    gpui_ios_did_become_active(nil)
    resumeFrames()
  }

  @objc private func willResignActive() {
    guard window != nil else { return }
    gpui_ios_will_resign_active(nil)
  }

  /// A backgrounded app may not submit GPU work, so the display link stops here.
  @objc private func didEnterBackground() {
    inBackground = true
    displayLink?.isPaused = true
    guard window != nil else { return }
    gpui_ios_did_enter_background(nil)
  }
}

struct GpuiCommandError: Error, CustomStringConvertible {
  let message: String
  var description: String { message }
}

/// The display link's target, so the link (which retains its target) never retains the host.
private final class DisplayLinkTarget: NSObject {
  @objc func tick() {
    GpuiHost.shared.tick()
  }
}

/// Rust events arrive on the main thread, from inside a GPUI update; they are delivered on the
/// next turn of the main queue so nothing a listener does can re-enter GPUI.
private let gpuiEventCallback: ghostex_gpui_event_fn = { json, _ in
  guard let json else { return }
  let text = String(cString: json)
  DispatchQueue.main.async {
    GpuiHost.shared.deliver(text)
  }
}

/// GPUI wants frames again after the display link paused.
private let gpuiFrameWaker: @convention(c) (UnsafeMutableRawPointer?) -> Void = { _ in
  if Thread.isMainThread {
    GpuiHost.shared.resumeFrames()
  } else {
    DispatchQueue.main.async { GpuiHost.shared.resumeFrames() }
  }
}

extension UIView {
  /// The nearest view controller up the responder chain, the parent GPUI's controller joins.
  var owningViewController: UIViewController? {
    var responder: UIResponder? = next
    while let current = responder {
      if let controller = current as? UIViewController {
        return controller
      }
      responder = current.next
    }
    return nil
  }
}

/// Display link ticks, reported once per busy second (test hook only).
private struct TickStats {
  private var spanStart: CFTimeInterval = 0
  private var last: CFTimeInterval = 0
  private var ticks = 0
  private var worstGap: CFTimeInterval = 0

  mutating func record(_ now: CFTimeInterval, idle: Bool) {
    if ticks > 0, now - last > 0.25 {
      flush()
    }
    if ticks == 0 {
      spanStart = now
    } else {
      worstGap = max(worstGap, now - last)
    }
    ticks += 1
    last = now
    if now - spanStart >= 1 || idle {
      flush()
    }
  }

  private mutating func flush() {
    if ticks > 1 {
      NSLog(
        "[GpuiTicks] %d display link ticks in %.0f ms, worst gap %.1f ms", ticks,
        (last - spanStart) * 1000, worstGap * 1000)
    }
    ticks = 0
    worstGap = 0
  }
}
