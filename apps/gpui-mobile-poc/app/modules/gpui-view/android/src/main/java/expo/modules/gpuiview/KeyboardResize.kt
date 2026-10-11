package expo.modules.gpuiview

import android.app.Activity
import android.util.Log
import android.view.View
import android.view.ViewGroup
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import java.lang.ref.WeakReference

/**
 * `adjustResize` for an edge-to-edge window.
 *
 * CDXC:Mobile 2026-09-27 WHY: React Native 0.86 runs edge-to-edge, where
 * `windowSoftInputMode="adjustResize"` no longer resizes the window, so the keyboard covered the
 * composer and the GPUI view never shrank. Padding the content view by the IME inset restores the
 * resize without moving keyboard handling into JavaScript.
 *
 * React Native 0.86 draws edge-to-edge (`edgeToEdgeEnabled=true`, and Android 15 enforces it for
 * apps targeting SDK 35), and an edge-to-edge window is not resized for the software keyboard:
 * `windowSoftInputMode="adjustResize"` has no effect, the keyboard simply covers the composer and
 * the bottom of the GPUI view. This restores the resize: the Activity's content view is padded by
 * the keyboard's height, so React Native lays its root out in the space above the keyboard and
 * flex shrinks the GPUI view.
 *
 * The padding is the whole keyboard inset, navigation bar included: safe-area-context measures
 * the root against the system bars, so once the root ends above the keyboard its bottom inset
 * drops to zero, and padding only the part above the bar would leave the composer that much
 * under the keyboard. Like `adjustResize` it applies the keyboard's final height once rather than
 * following the slide frame by frame, so the GPUI surface is resized once per open or close.
 */
internal object KeyboardResize {
  private const val TAG = "GpuiKeyboard"
  private var installedOn: WeakReference<View>? = null

  fun install(activity: Activity) {
    val content = activity.findViewById<ViewGroup>(android.R.id.content) ?: return
    if (installedOn?.get() === content) return
    installedOn = WeakReference(content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val bottom = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      if (view.paddingBottom != bottom) {
        Log.i(TAG, "keyboard ${bottom}px: content bottom padding $bottom")
        view.setPadding(view.paddingLeft, view.paddingTop, view.paddingRight, bottom)
      }
      // Not consumed: React Native and safe-area-context still read the system bar insets.
      insets
    }
    ViewCompat.requestApplyInsets(content)
  }
}
