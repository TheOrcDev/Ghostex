package dev.ghostex.gpui

import android.app.Activity
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.Log
import android.view.Surface
import java.util.concurrent.CopyOnWriteArraySet

/**
 * The JNI surface of `libghostex_gpui_mobile.so` (apps/gpui-mobile-poc/host, `src/android/entry.rs`).
 *
 * GPUI runs on its own process-lived `gpui-main` thread. Every `native*` call here only queues work
 * for it, except [nativeSurfaceDestroyed], which blocks until that thread has let go of the surface
 * (the caller may only return from `surfaceDestroyed` afterwards).
 *
 * Kept outside the Expo module's package so the class name, and with it the JNI symbol names, can
 * move into the real app unchanged.
 */
object GpuiNative {
  private const val TAG = "GpuiNative"

  init {
    System.loadLibrary("ghostex_gpui_mobile")
  }

  /** Starts GPUI once per process; `false` when it was already running. */
  @JvmStatic external fun nativeStart(activity: Activity?, filesDir: String, configJson: String): Boolean

  /** A new Activity instance took over (recreation); gpui-mobile's JNI helpers use it. */
  @JvmStatic external fun nativeSetActivity(activity: Activity)

  /** From `surfaceCreated` and `surfaceChanged`: a new surface is attached, the same one resized. */
  @JvmStatic external fun nativeSurfaceChanged(surface: Surface, scale: Float)

  /**
   * From `surfaceRedrawNeededAsync`: Rust calls [onRedrawDone] once a frame of `width` x `height`
   * is on screen. Use [awaitRedraw], which also owns the timeout.
   */
  @JvmStatic external fun nativeSurfaceRedrawNeeded(width: Int, height: Int)

  /** From `surfaceDestroyed`: returns once the render thread has released the surface. */
  @JvmStatic external fun nativeSurfaceDestroyed()

  /**
   * One whole MotionEvent, or one historical sample of a batched `ACTION_MOVE`: `action` is
   * `getActionMasked()`, the arrays hold `count` pointers in index order, positions in physical
   * pixels relative to the surface, and `eventTimeMs` is the sample's `getEventTime()`.
   */
  @JvmStatic external fun nativeMotionEvent(
    action: Int,
    actionIndex: Int,
    count: Int,
    ids: IntArray,
    xs: FloatArray,
    ys: FloatArray,
    eventTimeMs: Long
  ): Boolean

  @JvmStatic external fun nativeResumed()

  @JvmStatic external fun nativePaused()

  /** One JSON command object (`{"type": ...}`) for the root view. Throws on malformed JSON. */
  @JvmStatic external fun nativeCommand(json: String)

  /** Receives Rust events (JSON objects with a `type`) on the main thread. */
  fun interface Listener {
    fun handleGpuiEvent(json: String)
  }

  private val listeners = CopyOnWriteArraySet<Listener>()
  private val mainHandler = Handler(Looper.getMainLooper())

  fun addListener(listener: Listener) {
    listeners.add(listener)
  }

  fun removeListener(listener: Listener) {
    listeners.remove(listener)
  }

  private const val REDRAW_TIMEOUT_MS = 1000L
  private val redrawLock = Any()
  private var pendingRedraw: Runnable? = null
  private var pendingSince = 0L

  /**
   * Holds a resized SurfaceView's frame until GPUI has presented one of the new size, so the old
   * buffer is never shown stretched. `drawingFinished` runs on whichever thread finishes first:
   * `gpui-main` when the frame lands, or the main thread after [REDRAW_TIMEOUT_MS].
   */
  fun awaitRedraw(width: Int, height: Int, drawingFinished: Runnable) {
    val replaced = synchronized(redrawLock) {
      val previous = pendingRedraw
      pendingRedraw = drawingFinished
      pendingSince = SystemClock.elapsedRealtime()
      previous
    }
    replaced?.run()
    nativeSurfaceRedrawNeeded(width, height)
    mainHandler.postDelayed({
      val stale = synchronized(redrawLock) {
        val still = pendingRedraw === drawingFinished
        if (still) pendingRedraw = null
        still
      }
      if (stale) {
        Log.w(TAG, "no ${width}x$height frame after $REDRAW_TIMEOUT_MS ms; releasing the SurfaceView")
        drawingFinished.run()
      }
    }, REDRAW_TIMEOUT_MS)
  }

  /** Called by Rust on `gpui-main` when a frame of the awaited size has been presented. */
  @JvmStatic
  fun onRedrawDone(width: Int, height: Int) {
    val (done, since) = synchronized(redrawLock) {
      val pending = pendingRedraw
      pendingRedraw = null
      pending to pendingSince
    }
    if (done != null) {
      Log.i(TAG, "${width}x$height frame presented ${SystemClock.elapsedRealtime() - since} ms after the resize")
      done.run()
    }
  }

  /** Called by Rust on `gpui-main`; hops to the main thread and never blocks the caller. */
  @JvmStatic
  fun onRustEvent(json: String) {
    mainHandler.post {
      if (listeners.isEmpty()) {
        Log.d(TAG, "event with no listener: $json")
      }
      for (listener in listeners) listener.handleGpuiEvent(json)
    }
  }
}
