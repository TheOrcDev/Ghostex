package expo.modules.gpuiview

import android.content.Context
import android.util.Log
import java.io.File

/**
 * Fonts shipped in the APK for GPUI.
 *
 * Android 13+ draws emoji from a COLR v1 font that GPUI's text stack cannot rasterise, so the APK
 * carries a CBDT Noto Color Emoji (`assets/fonts/NotoColorEmoji.ttf`, staged by
 * `scripts/build-android.sh`). Assets are compressed inside the APK, so the file is copied to
 * app storage once per installed version; Rust then maps it read-only instead of holding a copy
 * in its heap.
 */
internal object BundledFonts {
  private const val TAG = "GpuiFonts"
  private const val EMOJI_ASSET = "fonts/NotoColorEmoji.ttf"

  fun emojiFontPath(context: Context): String? = try {
    val target = File(File(context.filesDir, "gpui-fonts"), "NotoColorEmoji.ttf")
    val stamp = File(target.parentFile, "NotoColorEmoji.version")
    val version = context.packageManager.getPackageInfo(context.packageName, 0).lastUpdateTime.toString()
    if (!target.isFile || !stamp.isFile || stamp.readText() != version) {
      target.parentFile?.mkdirs()
      val partial = File(target.parentFile, "NotoColorEmoji.ttf.partial")
      context.assets.open(EMOJI_ASSET).use { input -> partial.outputStream().use { input.copyTo(it) } }
      if (!partial.renameTo(target)) throw IllegalStateException("rename failed")
      stamp.writeText(version)
      Log.i(TAG, "extracted $EMOJI_ASSET (${target.length()} bytes)")
    }
    target.absolutePath
  } catch (error: Exception) {
    Log.w(TAG, "no bundled emoji font: $error")
    null
  }
}
