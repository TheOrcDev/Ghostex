package expo.modules.gpuiview

import dev.ghostex.gpui.GpuiNative
import expo.modules.kotlin.exception.CodedException
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition
import org.json.JSONObject

/**
 * `start(config)` and `command(json)` as synchronous JSI functions, the Activity's foreground and
 * background transitions, and the `GpuiView` native view.
 */
class GpuiViewModule : Module() {
  override fun definition() = ModuleDefinition {
    Name("GpuiView")

    Function("start") { configJson: String ->
      val context = appContext.reactContext
        ?: throw CodedException("GpuiNoContext", "React context is unavailable.", null)
      val activity = appContext.currentActivity
      val config = try {
        JSONObject(configJson)
      } catch (_: Exception) {
        JSONObject()
      }
      // Dev hook: `adb shell am start -n <pkg>/.MainActivity --es gpuiBackend gles` (or vulkan)
      // limits wgpu to one backend, so the two can be compared on a device.
      activity?.intent?.getStringExtra("gpuiBackend")?.let { config.put("backend", it) }
      // Dev hook: `--es gpuiContent demo` shows the synthetic list instead of the chat.
      activity?.intent?.getStringExtra("gpuiContent")?.let { config.put("content", it) }
      BundledFonts.emojiFontPath(context)?.let { config.put("emojiFontPath", it) }
      GpuiNative.nativeStart(activity, context.filesDir.absolutePath, config.toString())
    }

    Function("command") { json: String ->
      GpuiNative.nativeCommand(json)
    }

    // The launch Intent's string extras the JS side reads (`gpuiContent`, `gpuiSession`), so a test
    // can open a screen or a session with `adb shell am start ... --es <name> <value>`.
    Function("launchOptions") {
      val extras = appContext.currentActivity?.intent?.extras ?: return@Function emptyMap<String, String>()
      extras.keySet()
        .filter { it.startsWith("gpui") }
        .mapNotNull { key -> extras.getString(key)?.let { key to it } }
        .toMap()
    }

    // The chat's copy actions: GPUI writes its own clipboard, the phone's is this one.
    Function("setClipboardText") { text: String ->
      val context = appContext.reactContext ?: return@Function false
      val clipboard = context.getSystemService(android.content.Context.CLIPBOARD_SERVICE)
        as android.content.ClipboardManager
      clipboard.setPrimaryClip(android.content.ClipData.newPlainText("Ghostex", text))
      true
    }

    // Resize the React root for the software keyboard (see KeyboardResize).
    Function("installKeyboardResize") {
      val activity = appContext.currentActivity ?: return@Function false
      activity.runOnUiThread { KeyboardResize.install(activity) }
      true
    }

    OnActivityEntersForeground { GpuiNative.nativeResumed() }

    OnActivityEntersBackground { GpuiNative.nativePaused() }

    View(GpuiView::class) {
      Events("onGpuiEvent")

      Prop("surfaceType") { view: GpuiView, type: String? ->
        view.setSurfaceType(type)
      }
    }
  }
}
