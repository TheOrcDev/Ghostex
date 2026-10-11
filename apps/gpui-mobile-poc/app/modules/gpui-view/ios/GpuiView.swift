import ExpoModulesCore
import UIKit

/// An Expo view whose whole area is drawn by GPUI.
///
/// The view itself only hosts GPUI's window: `GpuiHost` adds GPUI's view controller as a child
/// (its view is the `CAMetalLayer` GPUI renders into and the view that receives the touches) and
/// keeps its frame equal to these bounds. React Native lays this view out; nothing else about it
/// is drawn. `surfaceType` is Android's choice between a `SurfaceView` and a `TextureView`; on iOS
/// there is one kind of surface, a Core Animation layer composed like any view, so it is ignored.
final class GpuiView: ExpoView {
  let onGpuiEvent = EventDispatcher()

  required init(appContext: AppContext? = nil) {
    super.init(appContext: appContext)
    clipsToBounds = true
    backgroundColor = UIColor(red: 0x18 / 255.0, green: 0x18 / 255.0, blue: 0x18 / 255.0, alpha: 1)
  }

  override func didMoveToSuperview() {
    super.didMoveToSuperview()
    if superview != nil {
      GpuiHost.shared.viewDidMount(self)
    } else {
      GpuiHost.shared.viewDidUnmount(self)
    }
  }

  override func didMoveToWindow() {
    super.didMoveToWindow()
    if window != nil {
      GpuiHost.shared.viewDidJoinWindow(self)
    } else {
      GpuiHost.shared.viewDidLeaveWindow(self)
    }
  }

  override func layoutSubviews() {
    super.layoutSubviews()
    GpuiHost.shared.layout(self)
  }

  func handleGpuiEvent(_ json: String) {
    onGpuiEvent(["json": json])
  }
}
