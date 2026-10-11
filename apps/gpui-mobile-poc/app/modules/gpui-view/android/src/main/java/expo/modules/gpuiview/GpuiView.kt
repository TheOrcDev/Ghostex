package expo.modules.gpuiview

import android.content.Context
import android.graphics.PixelFormat
import android.graphics.SurfaceTexture
import android.os.SystemClock
import android.util.Log
import android.view.MotionEvent
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.TextureView
import android.view.View
import android.view.ViewConfiguration
import dev.ghostex.gpui.GpuiNative
import expo.modules.kotlin.AppContext
import expo.modules.kotlin.viewevent.EventDispatcher
import expo.modules.kotlin.views.ExpoView
import kotlin.math.hypot
import kotlin.math.min

/**
 * An Expo view whose whole area is drawn by GPUI.
 *
 * It hosts one child that produces an `android.view.Surface`: a [SurfaceView] by default (its own
 * compositor layer, cheapest to draw) or a [TextureView] (`surfaceType="texture"`: composed like a
 * normal view, so it moves and resizes in step with the React views around it, at some GPU cost).
 * The surface goes to Rust, which renders into it on the `gpui-main` thread; touches on this view
 * go to Rust whole (`MotionEvent` by `MotionEvent`) for GPUI's own gesture recognizer.
 */
class GpuiView(context: Context, appContext: AppContext) :
  ExpoView(context, appContext), GpuiNative.Listener {

  private val onGpuiEvent by EventDispatcher<Map<String, Any>>()

  private var surfaceType = SurfaceType.SURFACE
  private var child: View? = null

  private val touchSlop = ViewConfiguration.get(context).scaledTouchSlop
  private val pointerIds = IntArray(MAX_POINTERS)
  private val pointerXs = FloatArray(MAX_POINTERS)
  private val pointerYs = FloatArray(MAX_POINTERS)
  private var downX = 0f
  private var downY = 0f
  private var dragging = false

  private val scale: Float
    get() = resources.displayMetrics.density

  init {
    // The view itself draws nothing; GPUI owns every pixel of the child surface.
    setWillNotDraw(true)
    installChild()
  }

  // region props

  fun setSurfaceType(value: String?) {
    val next = if (value == "texture") SurfaceType.TEXTURE else SurfaceType.SURFACE
    if (next == surfaceType && child != null) return
    Log.i(TAG, "surface type $surfaceType -> $next")
    surfaceType = next
    installChild()
  }

  // endregion

  // region child surface

  private fun installChild() {
    child?.let {
      // Detaching destroys the old surface; its callback blocks until Rust has released it.
      removeView(it)
    }
    val view = when (surfaceType) {
      SurfaceType.SURFACE -> createSurfaceView()
      SurfaceType.TEXTURE -> createTextureView()
    }
    child = view
    addView(view, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
    layoutChild()
  }

  private fun createSurfaceView(): SurfaceView {
    val view = SurfaceView(context)
    view.holder.setFormat(PixelFormat.OPAQUE)
    view.holder.addCallback(object : SurfaceHolder.Callback2 {
      override fun surfaceCreated(holder: SurfaceHolder) {
        Log.i(TAG, "SurfaceView surfaceCreated")
        GpuiNative.nativeSurfaceChanged(holder.surface, scale)
      }

      override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        Log.i(TAG, "SurfaceView surfaceChanged ${width}x$height")
        GpuiNative.nativeSurfaceChanged(holder.surface, scale)
      }

      override fun surfaceDestroyed(holder: SurfaceHolder) {
        releaseSurface("SurfaceView")
      }

      override fun surfaceRedrawNeeded(holder: SurfaceHolder) = Unit

      // Holds the window's next frame until GPUI has drawn at the new size, so the old buffer is
      // not shown scaled. SurfaceView only joins that sync for resizes the window manager drives
      // (rotation, split screen); for a relayout the app makes itself (KeyboardResize) it does
      // not, and the old buffer shows scaled until GPUI's frame lands.
      override fun surfaceRedrawNeededAsync(holder: SurfaceHolder, drawingFinished: Runnable) {
        val frame = holder.surfaceFrame
        GpuiNative.awaitRedraw(frame.width(), frame.height(), drawingFinished)
      }
    })
    return view
  }

  private fun createTextureView(): TextureView {
    val view = TextureView(context)
    view.isOpaque = true
    view.surfaceTextureListener = object : TextureView.SurfaceTextureListener {
      private var surface: Surface? = null

      override fun onSurfaceTextureAvailable(texture: SurfaceTexture, width: Int, height: Int) {
        Log.i(TAG, "TextureView available ${width}x$height")
        val created = Surface(texture)
        surface = created
        GpuiNative.nativeSurfaceChanged(created, scale)
      }

      override fun onSurfaceTextureSizeChanged(texture: SurfaceTexture, width: Int, height: Int) {
        Log.i(TAG, "TextureView size changed ${width}x$height")
        surface?.let { GpuiNative.nativeSurfaceChanged(it, scale) }
      }

      override fun onSurfaceTextureDestroyed(texture: SurfaceTexture): Boolean {
        releaseSurface("TextureView")
        surface?.release()
        surface = null
        // Rust no longer uses the texture: let the TextureView release it.
        return true
      }

      override fun onSurfaceTextureUpdated(texture: SurfaceTexture) = Unit
    }
    return view
  }

  private fun releaseSurface(kind: String) {
    val started = SystemClock.elapsedRealtime()
    GpuiNative.nativeSurfaceDestroyed()
    Log.i(TAG, "$kind surface released in ${SystemClock.elapsedRealtime() - started} ms")
  }

  // React Native lays this view out but never measures or lays out its Android children
  // (ReactViewGroup-style parents ignore requestLayout), so the child is sized by hand.
  override fun onLayout(changed: Boolean, left: Int, top: Int, right: Int, bottom: Int) {
    layoutChild()
  }

  private fun layoutChild() {
    val view = child ?: return
    val width = width
    val height = height
    view.measure(
      MeasureSpec.makeMeasureSpec(width, MeasureSpec.EXACTLY),
      MeasureSpec.makeMeasureSpec(height, MeasureSpec.EXACTLY)
    )
    view.layout(0, 0, width, height)
  }

  // endregion

  // region touch

  override fun onInterceptTouchEvent(event: MotionEvent): Boolean = true

  override fun onTouchEvent(event: MotionEvent): Boolean {
    val count = min(event.pointerCount, MAX_POINTERS)
    for (index in 0 until count) {
      pointerIds[index] = event.getPointerId(index)
    }
    if (event.actionMasked == MotionEvent.ACTION_MOVE) {
      // CDXC:Mobile 2026-09-27 WHY: Android batches the touch samples of one frame into one
      // ACTION_MOVE (a 120 Hz digitizer gives two or more per 60 Hz frame). GPUI's recognizer
      // measures fling velocity from sample positions and times, so each historical sample goes
      // over with its own time instead of only the newest one stamped when it is processed.
      for (sample in 0 until event.historySize) {
        for (index in 0 until count) {
          pointerXs[index] = event.getHistoricalX(index, sample)
          pointerYs[index] = event.getHistoricalY(index, sample)
        }
        GpuiNative.nativeMotionEvent(
          MotionEvent.ACTION_MOVE,
          0,
          count,
          pointerIds,
          pointerXs,
          pointerYs,
          event.getHistoricalEventTime(sample)
        )
      }
    }
    for (index in 0 until count) {
      pointerXs[index] = event.getX(index)
      pointerYs[index] = event.getY(index)
    }
    when (event.actionMasked) {
      MotionEvent.ACTION_DOWN -> {
        downX = event.x
        downY = event.y
        dragging = false
      }
      MotionEvent.ACTION_MOVE -> {
        // Once the finger has moved past the slop GPUI's recognizer treats the contact as a pan:
        // from then on no ancestor (a pager, a navigator's swipe) may take the gesture away.
        if (!dragging && hypot(event.x - downX, event.y - downY) > touchSlop) {
          dragging = true
          parent?.requestDisallowInterceptTouchEvent(true)
        }
      }
      MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
        if (dragging) parent?.requestDisallowInterceptTouchEvent(false)
        dragging = false
      }
    }
    GpuiNative.nativeMotionEvent(
      event.actionMasked,
      event.actionIndex,
      count,
      pointerIds,
      pointerXs,
      pointerYs,
      event.eventTime
    )
    return true
  }

  // endregion

  // region events and lifecycle

  override fun onAttachedToWindow() {
    super.onAttachedToWindow()
    GpuiNative.addListener(this)
    appContext.currentActivity?.let { GpuiNative.nativeSetActivity(it) }
  }

  override fun onDetachedFromWindow() {
    GpuiNative.removeListener(this)
    super.onDetachedFromWindow()
  }

  override fun handleGpuiEvent(json: String) {
    onGpuiEvent(mapOf("json" to json))
  }

  // endregion

  private enum class SurfaceType { SURFACE, TEXTURE }

  companion object {
    private const val TAG = "GpuiView"
    private const val MAX_POINTERS = 16
  }
}
