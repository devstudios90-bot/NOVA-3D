package com.nova3d.studio.viewport

import android.view.Choreographer
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.viewinterop.AndroidView

/**
 * Interactive Filament viewport surface.
 *
 * Touch model (VP-7, VP-8, VP-9):
 * - Single-finger drag starting on a 3D gizmo axis: transforms selection without moving camera
 * - Single-finger tap (under touch slop): ray-picks scene object
 * - Single-finger drag on empty background: smooth spherical orbit
 * - Two-finger drag: screen-space pan
 * - Two-finger pinch: dolly / ortho zoom
 */
@Composable
fun ViewportSurface(
    controller: ViewportController,
    isActive: Boolean,
    modifier: Modifier = Modifier,
) {
    val frameCallback = remember(controller) {
        object : Choreographer.FrameCallback {
            var running = false
            override fun doFrame(frameTimeNanos: Long) {
                if (!running) return
                controller.renderFrame(frameTimeNanos)
                Choreographer.getInstance().postFrameCallback(this)
            }
        }
    }

    DisposableEffect(isActive, controller) {
        if (isActive) {
            frameCallback.running = true
            Choreographer.getInstance().postFrameCallback(frameCallback)
        }
        onDispose {
            frameCallback.running = false
            Choreographer.getInstance().removeFrameCallback(frameCallback)
        }
    }

    Box(
        modifier = modifier
            .fillMaxSize()
            .pointerInput(controller) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    val w = size.width.coerceAtLeast(1).toFloat()
                    val h = size.height.coerceAtLeast(1).toFloat()
                    val aspect = w / h
                    val downNdcX = (2f * down.position.x / w) - 1f
                    val downNdcY = 1f - (2f * down.position.y / h)

                    val hitAxis = controller.pickGizmoAxis(downNdcX, downNdcY, aspect)
                    var inGizmoDrag = if (hitAxis != 0) {
                        controller.beginGizmoDrag(hitAxis, downNdcX, downNdcY, aspect)
                    } else {
                        false
                    }

                    val touchSlop = viewConfiguration.touchSlop
                    var prevPointerCount = 1
                    var movedPastSlop = false
                    var hadMultiTouch = false

                    do {
                        val event = awaitPointerEvent()
                        val pressedChanges = event.changes.filter { it.pressed }
                        val pointerCount = pressedChanges.size
                        if (pointerCount >= 2) {
                            hadMultiTouch = true
                            if (inGizmoDrag) {
                                controller.endGizmoDrag()
                                inGizmoDrag = false
                            }
                        }

                        if (inGizmoDrag && pointerCount == 1 && prevPointerCount == 1) {
                            val change = pressedChanges.first()
                            if (change.positionChanged()) {
                                val nx = (2f * change.position.x / w) - 1f
                                val ny = 1f - (2f * change.position.y / h)
                                controller.updateGizmoDrag(nx, ny, aspect)
                                change.consume()
                            }
                        } else if (pointerCount == 1 && prevPointerCount == 1 && !hadMultiTouch) {
                            val change = pressedChanges.first()
                            val totalDist = (change.position - down.position).getDistance()
                            if (!movedPastSlop && totalDist > touchSlop) {
                                movedPastSlop = true
                            }
                            if (movedPastSlop && change.positionChanged()) {
                                val delta = change.position - change.previousPosition
                                val orbitSpeed = 3.2f / h
                                controller.orbit(
                                    deltaYawRad = -delta.x * orbitSpeed,
                                    deltaPitchRad = delta.y * orbitSpeed,
                                )
                                change.consume()
                            }
                        } else if (pointerCount >= 2 && prevPointerCount >= 2) {
                            val zoomChange = event.calculateZoom()
                            if (zoomChange != 1.0f && zoomChange > 0.01f) {
                                controller.zoom(1.0f / zoomChange)
                            }
                            val panChange = event.calculatePan()
                            if (panChange.x != 0f || panChange.y != 0f) {
                                controller.pan(
                                    normDx = panChange.x / h,
                                    normDy = panChange.y / h,
                                )
                            }
                            pressedChanges.forEach { if (it.positionChanged()) it.consume() }
                        }

                        prevPointerCount = pointerCount
                    } while (pressedChanges.isNotEmpty())

                    if (inGizmoDrag) {
                        controller.endGizmoDrag()
                    } else if (!hadMultiTouch && !movedPastSlop) {
                        controller.pickNodeAt(downNdcX, downNdcY, aspect)
                    }
                }
            },
    ) {
        AndroidView(
            modifier = Modifier.fillMaxSize(),
            factory = { context ->
                SurfaceView(context).apply {
                    controller.registerSurfaceView(this)
                    holder.addCallback(object : SurfaceHolder.Callback {
                        override fun surfaceCreated(holder: SurfaceHolder) {
                            val frame = holder.surfaceFrame
                            controller.attachSurface(
                                holder.surface,
                                frame.width().coerceAtLeast(1),
                                frame.height().coerceAtLeast(1),
                            )
                        }

                        override fun surfaceChanged(
                            holder: SurfaceHolder,
                            format: Int,
                            width: Int,
                            height: Int,
                        ) {
                            controller.resizeSurface(width.coerceAtLeast(1), height.coerceAtLeast(1))
                        }

                        override fun surfaceDestroyed(holder: SurfaceHolder) {
                            controller.detachSurface()
                        }
                    })
                }
            },
            onRelease = {
                controller.registerSurfaceView(null)
            },
        )
    }
}
