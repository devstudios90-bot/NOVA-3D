package com.nova3d.studio.viewport

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.provider.OpenableColumns
import android.view.PixelCopy
import android.view.Surface
import android.view.SurfaceView
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.nova3d.studio.nativebridge.NovaNativeBridge
import java.lang.ref.WeakReference
import java.nio.ByteBuffer
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.coroutines.resume
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext

data class SceneNodeSummary(
    val id: Long,
    val parentId: Long,
    val depth: Int,
    val kind: Int,
    val visible: Boolean,
    val selected: Boolean,
    val primarySelected: Boolean,
    val hasMesh: Boolean,
    val name: String,
)

data class NodeDetails(
    val id: Long,
    val parentId: Long,
    val kind: Int,
    val visible: Boolean,
    val selected: Boolean,
    val primarySelected: Boolean,
    val hasMesh: Boolean,
    val posX: Float,
    val posY: Float,
    val posZ: Float,
    val rotX: Float,
    val rotY: Float,
    val rotZ: Float,
    val scaleX: Float,
    val scaleY: Float,
    val scaleZ: Float,
    val colorR: Float,
    val colorG: Float,
    val colorB: Float,
    val colorA: Float,
    val metallic: Float,
    val roughness: Float,
    val materialId: Long,
    val lightKind: Int,
    val lightColorR: Float,
    val lightColorG: Float,
    val lightColorB: Float,
    val lightIntensity: Float,
    val lightCastShadows: Boolean,
    val lightSpotInnerDeg: Float = 20f,
    val lightSpotOuterDeg: Float = 40f,
    val lightRange: Float = 16f,
    val cameraFovDeg: Float = 50f,
)

enum class InterpolationUiMode(val code: Int, val label: String) {
    Linear(0, "Linear"),
    Constant(1, "Constant"),
    Bezier(2, "Bezier"),
    EaseIn(3, "Ease In"),
    EaseOut(4, "Ease Out"),
    EaseInOut(5, "Ease In-Out"),
}

data class KeyframeUi(
    val id: Long,
    val frame: Int,
    val value: Float,
    val interpolation: InterpolationUiMode,
    val inHandleX: Float,
    val inHandleY: Float,
    val outHandleX: Float,
    val outHandleY: Float,
)

data class AnimTrackUi(
    val id: Long,
    val targetKind: Int, // 0 = Node, 1 = Material
    val targetId: Long,
    val channel: Int,    // 0..22
    val keyframes: List<KeyframeUi>,
) {
    fun channelLabel(): String = when (channel) {
        0 -> "Loc X"
        1 -> "Loc Y"
        2 -> "Loc Z"
        3 -> "Rot X"
        4 -> "Rot Y"
        5 -> "Rot Z"
        6 -> "Scale X"
        7 -> "Scale Y"
        8 -> "Scale Z"
        9 -> "Light Int"
        10 -> "Light R"
        11 -> "Light G"
        12 -> "Light B"
        13 -> "Spot Cone"
        14 -> "Light Range"
        15 -> "Cam FOV"
        16 -> "Mat Color R"
        17 -> "Mat Color G"
        18 -> "Mat Color B"
        19 -> "Mat Opacity"
        20 -> "Metallic"
        21 -> "Roughness"
        22 -> "Emissive"
        else -> "Ch $channel"
    }
}

data class TimelineUiState(
    val startFrame: Int = 1,
    val endFrame: Int = 120,
    val fps: Float = 30f,
    val currentFrame: Float = 1f,
    val playing: Boolean = false,
    val looping: Boolean = true,
    val autoKey: Boolean = false,
    val activeCameraNodeId: Long = 0L,
)

enum class MaterialUiPreset(val code: Int, val label: String) {
    StudioBlue(0, "Studio Blue"),
    PolishedGold(1, "Polished Gold"),
    BrushedSteel(2, "Brushed Steel"),
    MatteTerracotta(3, "Terracotta"),
    FrostedGlass(4, "Frosted Glass"),
    NeonTech(5, "Neon Tech"),
    BrickMasonry(6, "Brick Masonry"),
    CrimsonCoat(7, "Crimson Coat"),
}

data class MaterialUiSpec(
    val id: Long,
    val name: String,
    val r: Float,
    val g: Float,
    val b: Float,
    val a: Float,
    val metallic: Float,
    val roughness: Float,
    val emR: Float,
    val emG: Float,
    val emB: Float,
    val emIntensity: Float,
    val aoStrength: Float,
    val alphaMode: Int,
    val albedoTexId: Long,
    val normalTexId: Long,
    val roughnessTexId: Long,
    val aoTexId: Long,
    val emissiveTexId: Long,
    val uvScaleX: Float,
    val uvScaleY: Float,
    val uvOffsetX: Float,
    val uvOffsetY: Float,
)

data class TextureUiSpec(
    val id: Long,
    val name: String,
    val width: Int,
    val height: Int,
    val status: Int, // 0=Builtin, 1=Loaded, 2=Missing
    val version: Long,
) {
    val isMissing: Boolean get() = status == 2
}

enum class EnvironmentUiPreset(val code: Int, val label: String) {
    StudioSoft(0, "Studio Soft"),
    StudioContrast(1, "Studio Contrast"),
    OutdoorDaylight(2, "Daylight"),
    GoldenSunset(3, "Golden Sunset"),
    NeonNight(4, "Neon Night"),
    NeutralGrey(5, "Neutral Grey"),
}

enum class LightingRigPreset(val code: Int, val label: String) {
    ThreePointStudio(0, "3-Point Studio"),
    ProductTurntable(1, "Product Turntable"),
    DramaticRim(2, "Dramatic Rim"),
    OutdoorSun(3, "Outdoor Sun"),
}

data class EnvironmentUiSpec(
    val preset: Int = 0,
    val rotationDeg: Float = 0f,
    val intensity: Float = 1.0f,
    val showSkybox: Boolean = false,
    val bloomEnabled: Boolean = false,
    val bloomStrength: Float = 0.14f,
    val exposure: Float = 1.0f,
    val previewSphere: Boolean = false,
)

enum class ProjectionUiMode(val code: Int, val label: String) {
    Perspective(0, "Persp"),
    Orthographic(1, "Ortho"),
}

enum class StandardViewPreset(val code: Int, val label: String) {
    Home(0, "Home"),
    Front(1, "Front"),
    Back(2, "Back"),
    Left(3, "Left"),
    Right(4, "Right"),
    Top(5, "Top"),
    Bottom(6, "Bottom"),
}

enum class ShadingUiMode(val code: Int, val label: String) {
    Solid(0, "Solid"),
    Wireframe(1, "Wire"),
    MaterialPreview(2, "Mat"),
    RenderPreview(3, "Render"),
}

enum class GizmoMode(val code: Int, val label: String) {
    Select(0, "Select"),
    Move(1, "Move"),
    Rotate(2, "Rotate"),
    Scale(3, "Scale"),
}

enum class GizmoSpace(val code: Int, val label: String) {
    World(0, "World"),
    Local(1, "Local"),
}

enum class InteractionUiMode(val code: Int, val label: String) {
    Object(0, "Object"),
    Edit(1, "Edit"),
}

enum class EditSelectUiMode(val code: Int, val label: String) {
    Vertex(0, "Vert"),
    Edge(1, "Edge"),
    Face(2, "Face"),
}

data class EditUiState(
    val interactionMode: InteractionUiMode = InteractionUiMode.Object,
    val editNodeId: Long = 0L,
    val selectMode: EditSelectUiMode = EditSelectUiMode.Face,
    val showFaceOrientation: Boolean = false,
    val smoothShading: Boolean = false,
    val totalVerts: Int = 0,
    val totalEdges: Int = 0,
    val totalFaces: Int = 0,
    val selectedVertClosure: Int = 0,
    val selectedVerts: Int = 0,
    val selectedEdges: Int = 0,
    val selectedFaces: Int = 0,
    val hasCentroid: Boolean = false,
    val centroidX: Float = 0f,
    val centroidY: Float = 0f,
    val centroidZ: Float = 0f,
) {
    val activeSelectedCount: Int
        get() = when (selectMode) {
            EditSelectUiMode.Vertex -> selectedVerts
            EditSelectUiMode.Edge -> selectedEdges
            EditSelectUiMode.Face -> selectedFaces
        }
}

enum class ModifierUiKind(val code: Int, val label: String) {
    Mirror(0, "Mirror"),
    Subdivision(1, "Subdivision"),
    Solidify(2, "Solidify"),
    Array(3, "Array"),
}

data class ModifierUiSpec(
    val index: Int,
    val kind: ModifierUiKind,
    val enabled: Boolean,
    val axis: Int,
    val merge: Boolean,
    val levels: Int,
    val thickness: Float,
    val count: Int,
    val offsetX: Float,
    val offsetY: Float,
    val offsetZ: Float,
)

enum class PrimitiveUiType(val code: Int, val label: String, val group: String) {
    Cube(0, "Cube", "Mesh"),
    UvSphere(1, "UV Sphere", "Mesh"),
    Icosphere(2, "Icosphere", "Mesh"),
    Cylinder(3, "Cylinder", "Mesh"),
    Cone(4, "Cone", "Mesh"),
    Torus(5, "Torus", "Mesh"),
    Plane(6, "Plane", "Mesh"),
    Circle(7, "Circle", "Mesh"),
    Empty(8, "Empty", "Helper"),
    Camera(9, "Camera", "Scene"),
    Light(10, "Light", "Scene"),
}

class ViewportController {
    private val renderThread = HandlerThread("NovaRenderThread").apply { start() }
    private val renderHandler = Handler(renderThread.looper)
    private val mainHandler = Handler(Looper.getMainLooper())
    private val frameInFlight = AtomicBoolean(false)

    @Volatile
    private var sessionPtr: Long = 0L
    private var lastFpsSampleNanos: Long = 0L
    private var frameCounter: Int = 0
    @Volatile
    private var pendingSurface: Surface? = null
    @Volatile
    private var pendingWidth: Int = 1
    @Volatile
    private var pendingHeight: Int = 1
    @Volatile
    private var surfaceViewRef: WeakReference<SurfaceView>? = null
    @Volatile
    private var destroyed: Boolean = false

    var isReady by mutableStateOf(false)
        private set
    var errorMessage by mutableStateOf<String?>(null)
        private set
    var statusBanner by mutableStateOf<String?>(null)
        private set
    var activeBackendName by mutableStateOf("Initializing")
        private set
    var projectionMode by mutableStateOf(ProjectionUiMode.Perspective)
        private set
    var shadingMode by mutableStateOf(ShadingUiMode.Solid)
        private set
    var gizmoMode by mutableStateOf(GizmoMode.Move)
        private set
    var gizmoSpace by mutableStateOf(GizmoSpace.World)
        private set
    var multiSelectEnabled by mutableStateOf(false)
        private set
    var canUndo by mutableStateOf(false)
        private set
    var canRedo by mutableStateOf(false)
        private set
    var cameraDistance by mutableFloatStateOf(5.5f)
        private set
    var cameraYawDeg by mutableFloatStateOf(37f)
        private set
    var cameraPitchDeg by mutableFloatStateOf(24f)
        private set
    var showGrid by mutableStateOf(true)
        private set
    var showAxes by mutableStateOf(true)
        private set
    var fps by mutableIntStateOf(0)
        private set
    var sceneNodes by mutableStateOf<List<SceneNodeSummary>>(emptyList())
        private set
    var selectedNodeDetails by mutableStateOf<NodeDetails?>(null)
        private set
    var editState by mutableStateOf(EditUiState())
        private set
    var modifiers by mutableStateOf<List<ModifierUiSpec>>(emptyList())
        private set
    var materials by mutableStateOf<List<MaterialUiSpec>>(emptyList())
        private set
    var selectedMaterialId by mutableStateOf(0L)
        private set
    var textures by mutableStateOf<List<TextureUiSpec>>(emptyList())
        private set
    var missingTextureWarnings by mutableStateOf<List<String>>(emptyList())
        private set
    var environment by mutableStateOf(EnvironmentUiSpec())
        private set
    var timeline by mutableStateOf(TimelineUiState())
        private set
    var animTracks by mutableStateOf<List<AnimTrackUi>>(emptyList())
        private set
    var selectedTrackId by mutableStateOf<Long?>(null)
    var selectedKeyframeId by mutableStateOf<Long?>(null)
    var defaultInterpolation by mutableStateOf(InterpolationUiMode.Bezier)

    // M6 Look/Render & 3D Format I/O state
    var adaptiveViewportQuality by mutableStateOf(true)
        private set
    var lastIoWarnings by mutableStateOf<List<String>>(emptyList())
        private set

    var projectTitle by mutableStateOf("Studio Starter")
        private set
    var currentFileName by mutableStateOf<String?>(null)
        private set
    var hasUnsavedChanges by mutableStateOf(false)
        private set

    var onSceneEdited: (() -> Unit)? = null

    fun registerSurfaceView(view: SurfaceView?) {
        surfaceViewRef = if (view != null) WeakReference(view) else null
    }

    fun clearStatusBanner() {
        statusBanner = null
    }

    fun clearError() {
        errorMessage = null
    }

    fun showStatus(msg: String) {
        statusBanner = msg
    }

    fun showError(msg: String) {
        errorMessage = msg
    }

    /**
     * Starts native session + Filament engine on the dedicated NovaRenderThread so that:
     * 1) the Android UI thread never blocks during initialization, and
     * 2) every Filament Engine / JobSystem / SwapChain / Renderer call stays on the same
     *    adopted thread.
     */
    fun startSessionAsync(backendPreference: Int = 0) {
        renderHandler.post {
            if (destroyed || sessionPtr != 0L) return@post
            if (!NovaNativeBridge.nativeLibraryLoaded) {
                val msg = "Native library failed to load: ${NovaNativeBridge.loadError ?: "unknown error"}"
                mainHandler.post {
                    errorMessage = msg
                    activeBackendName = "Error"
                    isReady = false
                }
                return@post
            }

            val ptr = NovaNativeBridge.nativeCreateSession(backendPreference)
            if (destroyed) {
                if (ptr != 0L) NovaNativeBridge.nativeDestroySession(ptr)
                return@post
            }
            if (ptr == 0L) {
                mainHandler.post {
                    errorMessage = "Failed to initialize 3D engine (Vulkan and OpenGL ES initialization failed)."
                    activeBackendName = "Error"
                    isReady = false
                }
                return@post
            }

            sessionPtr = ptr
            val backend = NovaNativeBridge.nativeGetBackendName(ptr)
            val surface = pendingSurface
            val width = pendingWidth
            val height = pendingHeight
            var attachError: String? = null
            if (surface != null && surface.isValid) {
                val ok = NovaNativeBridge.nativeAttachSurface(ptr, surface, width, height)
                if (!ok) {
                    val err = NovaNativeBridge.nativeGetLastError(ptr)
                    attachError = if (err.isNotBlank()) err else "Failed to attach viewport SwapChain."
                }
            }

            mainHandler.post {
                if (destroyed) return@post
                activeBackendName = backend
                errorMessage = attachError
                isReady = true
                refreshStateFromNative()
            }
        }
    }

    fun attachSurface(surface: Surface, width: Int, height: Int) {
        pendingSurface = surface
        pendingWidth = width
        pendingHeight = height
        renderHandler.post {
            val ptr = sessionPtr
            if (destroyed || ptr == 0L || !surface.isValid) return@post
            val ok = NovaNativeBridge.nativeAttachSurface(ptr, surface, width, height)
            val err = if (!ok) {
                NovaNativeBridge.nativeGetLastError(ptr).ifBlank {
                    "Failed to attach viewport SwapChain."
                }
            } else {
                null
            }
            mainHandler.post {
                if (!destroyed) errorMessage = err
            }
        }
    }

    fun resizeSurface(width: Int, height: Int) {
        pendingWidth = width
        pendingHeight = height
        renderHandler.post {
            val ptr = sessionPtr
            if (destroyed || ptr == 0L) return@post
            NovaNativeBridge.nativeResizeSurface(ptr, width, height)
        }
    }

    fun detachSurface() {
        pendingSurface = null
        if (destroyed) return
        if (Looper.myLooper() == renderThread.looper) {
            val ptr = sessionPtr
            if (ptr != 0L) NovaNativeBridge.nativeDetachSurface(ptr)
            return
        }
        val latch = CountDownLatch(1)
        val posted = renderHandler.post {
            try {
                val ptr = sessionPtr
                if (ptr != 0L) NovaNativeBridge.nativeDetachSurface(ptr)
            } finally {
                latch.countDown()
            }
        }
        if (posted) {
            latch.await(1500, TimeUnit.MILLISECONDS)
        }
    }

    fun renderFrame(frameTimeNanos: Long) {
        if (destroyed || sessionPtr == 0L) return
        if (!frameInFlight.compareAndSet(false, true)) return
        val posted = renderHandler.post {
            try {
                val ptr = sessionPtr
                if (!destroyed && ptr != 0L) {
                    val rendered = NovaNativeBridge.nativeRenderFrame(ptr, frameTimeNanos)
                    if (!rendered) {
                        val err = NovaNativeBridge.nativeGetLastError(ptr)
                        if (err.isNotBlank()) {
                            mainHandler.post {
                                if (!destroyed) errorMessage = err
                            }
                        }
                    }
                    if (timeline.playing) {
                        val tl = NovaNativeBridge.nativeGetTimelineState(ptr)
                        if (tl.size >= 9) {
                            val nowPlaying = tl[4] >= 0.5f
                            val nowFrame = tl[3]
                            val activeCam = tl[8].toLong()
                            mainHandler.post {
                                if (!destroyed) {
                                    val wasPlaying = timeline.playing
                                    timeline = timeline.copy(
                                        currentFrame = nowFrame,
                                        playing = nowPlaying,
                                        activeCameraNodeId = activeCam,
                                    )
                                    if (wasPlaying && !nowPlaying) {
                                        refreshStateFromNative()
                                    }
                                }
                            }
                        }
                    }
                    if (lastFpsSampleNanos == 0L) {
                        lastFpsSampleNanos = frameTimeNanos
                    }
                    frameCounter++
                    val elapsed = frameTimeNanos - lastFpsSampleNanos
                    if (elapsed >= 500_000_000L) {
                        val measuredFps = ((frameCounter * 1_000_000_000.0) / elapsed).toInt()
                        frameCounter = 0
                        lastFpsSampleNanos = frameTimeNanos
                        mainHandler.post {
                            if (!destroyed) fps = measuredFps
                        }
                    }
                }
            } finally {
                frameInFlight.set(false)
            }
        }
        if (!posted) {
            frameInFlight.set(false)
        }
    }

    suspend fun captureThumbnailBitmap(maxWidth: Int = 360): Bitmap? =
        suspendCancellableCoroutine { cont ->
            val sv = surfaceViewRef?.get()
            if (sv == null || !sv.holder.surface.isValid || sv.width <= 0 || sv.height <= 0) {
                cont.resume(null)
                return@suspendCancellableCoroutine
            }
            val scale = (maxWidth.toFloat() / sv.width.toFloat()).coerceAtMost(1f)
            val w = (sv.width * scale).toInt().coerceAtLeast(64)
            val h = (sv.height * scale).toInt().coerceAtLeast(64)
            val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
            try {
                PixelCopy.request(
                    sv,
                    bmp,
                    { result ->
                        if (result == PixelCopy.SUCCESS) {
                            cont.resume(bmp)
                        } else {
                            bmp.recycle()
                            cont.resume(null)
                        }
                    },
                    renderHandler,
                )
            } catch (_: Throwable) {
                bmp.recycle()
                cont.resume(null)
            }
        }

    fun orbit(deltaYawRad: Float, deltaPitchRad: Float) {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeOrbit(sessionPtr, deltaYawRad, deltaPitchRad)
        refreshStats()
    }

    fun pan(normDx: Float, normDy: Float) {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativePan(sessionPtr, normDx, normDy)
        refreshStats()
    }

    fun zoom(factor: Float) {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeZoom(sessionPtr, factor)
        refreshStats()
    }

    fun toggleProjection() {
        if (sessionPtr == 0L) return
        val next = if (projectionMode == ProjectionUiMode.Perspective) {
            ProjectionUiMode.Orthographic
        } else {
            ProjectionUiMode.Perspective
        }
        NovaNativeBridge.nativeSetProjection(sessionPtr, next.code)
        refreshStats()
    }

    @JvmName("applyShadingMode")
    fun setShadingMode(mode: ShadingUiMode) {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeSetShadingMode(sessionPtr, mode.code)
        refreshStats()
    }

    fun toggleShadingMode() {
        val next = ShadingUiMode.entries[(shadingMode.ordinal + 1) % ShadingUiMode.entries.size]
        setShadingMode(next)
    }

    fun setStandardView(preset: StandardViewPreset) {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeSetStandardView(sessionPtr, preset.code)
        refreshStats()
    }

    fun frameAll() {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeFrameAll(sessionPtr)
        refreshStats()
    }

    fun focusSelected() {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeFocusSelected(sessionPtr)
        refreshStats()
    }

    fun toggleGrid() {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeSetGridAxes(sessionPtr, !showGrid, showAxes)
        refreshStats()
    }

    fun toggleAxes() {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeSetGridAxes(sessionPtr, showGrid, !showAxes)
        refreshStats()
    }

    fun toggleMultiSelect() {
        multiSelectEnabled = !multiSelectEnabled
    }

    fun selectGizmoMode(mode: GizmoMode) {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeSetGizmoMode(sessionPtr, mode.code)
        refreshStats()
    }

    fun toggleGizmoSpace() {
        if (sessionPtr == 0L) return
        val next = if (gizmoSpace == GizmoSpace.World) GizmoSpace.Local else GizmoSpace.World
        NovaNativeBridge.nativeSetGizmoSpace(sessionPtr, next.code)
        refreshStats()
    }

    fun pickGizmoAxis(ndcX: Float, ndcY: Float, aspect: Float): Int {
        if (sessionPtr == 0L) return 0
        return NovaNativeBridge.nativePickGizmoAxis(sessionPtr, ndcX, ndcY, aspect)
    }

    fun beginGizmoDrag(axis: Int, ndcX: Float, ndcY: Float, aspect: Float): Boolean {
        if (sessionPtr == 0L) return false
        return NovaNativeBridge.nativeBeginGizmoDrag(sessionPtr, axis, ndcX, ndcY, aspect)
    }

    fun updateGizmoDrag(ndcX: Float, ndcY: Float, aspect: Float): Boolean {
        if (sessionPtr == 0L) return false
        val ok = NovaNativeBridge.nativeUpdateGizmoDrag(sessionPtr, ndcX, ndcY, aspect)
        if (ok) {
            refreshSelectedDetailsFromCurrent()
        }
        return ok
    }

    fun endGizmoDrag() {
        if (sessionPtr == 0L) return
        val changed = NovaNativeBridge.nativeEndGizmoDrag(sessionPtr)
        refreshStateFromNative()
        if (changed) {
            markSceneEdited()
        }
    }

    fun selectNode(nodeId: Long, additive: Boolean = multiSelectEnabled) {
        if (sessionPtr == 0L) return
        if (editState.interactionMode == InteractionUiMode.Edit && nodeId != editState.editNodeId) {
            NovaNativeBridge.nativeSetInteractionMode(sessionPtr, InteractionUiMode.Object.code)
        }
        NovaNativeBridge.nativeSelectNodeExt(sessionPtr, nodeId, additive)
        refreshStateFromNative()
    }

    fun pickNodeAt(ndcX: Float, ndcY: Float, aspect: Float) {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativePickAtNdc(sessionPtr, ndcX, ndcY, aspect, multiSelectEnabled)
        refreshStateFromNative()
    }

    fun addPrimitive(type: PrimitiveUiType) {
        if (sessionPtr == 0L) return
        val id = NovaNativeBridge.nativeAddPrimitive(sessionPtr, type.code)
        if (id != 0L) {
            refreshStateFromNative()
            markSceneEdited()
            statusBanner = "Added ${type.label}"
        } else {
            reportNativeError("Failed to add ${type.label}")
        }
    }

    fun duplicateSelected() {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeDuplicateSelected(sessionPtr)) {
            refreshStateFromNative()
            markSceneEdited()
        } else {
            reportNativeError("Select an object to duplicate")
        }
    }

    fun deleteSelected() {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeDeleteSelected(sessionPtr)) {
            refreshStateFromNative()
            markSceneEdited()
        } else {
            reportNativeError("Select an object to delete")
        }
    }

    fun renameNode(nodeId: Long, newName: String) {
        if (sessionPtr == 0L) return
        val trimmed = newName.trim()
        if (trimmed.isEmpty()) return
        if (NovaNativeBridge.nativeRenameNode(sessionPtr, nodeId, trimmed)) {
            refreshStateFromNative()
            markSceneEdited()
        } else {
            reportNativeError("Failed to rename object")
        }
    }

    fun reparentNode(childId: Long, newParentId: Long) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeReparentNode(sessionPtr, childId, newParentId)) {
            refreshStateFromNative()
            markSceneEdited()
        } else {
            reportNativeError("Invalid parent assignment")
        }
    }

    fun groupSelected() {
        if (sessionPtr == 0L) return
        val id = NovaNativeBridge.nativeGroupSelected(sessionPtr)
        if (id != 0L) {
            refreshStateFromNative()
            markSceneEdited()
            statusBanner = "Grouped selection"
        } else {
            reportNativeError("Select objects to group")
        }
    }

    fun unparentSelected() {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeUnparentSelected(sessionPtr)) {
            refreshStateFromNative()
            markSceneEdited()
        } else {
            reportNativeError("Selected object has no parent")
        }
    }

    fun applyTransformSelected() {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeApplyTransformSelected(sessionPtr)) {
            refreshStateFromNative()
            markSceneEdited()
            statusBanner = "Applied transforms to mesh geometry"
        } else {
            reportNativeError("Select a mesh object to apply transforms")
        }
    }

    fun originToGeometrySelected() {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeOriginToGeometrySelected(sessionPtr)) {
            refreshStateFromNative()
            markSceneEdited()
            statusBanner = "Recentered origin to geometry"
        } else {
            reportNativeError("Select a mesh object to recenter origin")
        }
    }

    fun setNodeVisibility(nodeId: Long, visible: Boolean) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetVisibility(sessionPtr, nodeId, visible)) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun setNodeTransform(
        nodeId: Long,
        posX: Float,
        posY: Float,
        posZ: Float,
        rotX: Float,
        rotY: Float,
        rotZ: Float,
        scaleX: Float,
        scaleY: Float,
        scaleZ: Float,
    ) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetTransform(
                sessionPtr, nodeId,
                posX, posY, posZ,
                rotX, rotY, rotZ,
                scaleX, scaleY, scaleZ,
            )
        ) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun setNodeMaterial(
        nodeId: Long,
        r: Float,
        g: Float,
        b: Float,
        a: Float,
        metallic: Float,
        roughness: Float,
    ) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetMaterial(sessionPtr, nodeId, r, g, b, a, metallic, roughness)) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun setNodeLight(
        nodeId: Long,
        kind: Int,
        r: Float,
        g: Float,
        b: Float,
        intensity: Float,
        castShadows: Boolean,
    ) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetLight(sessionPtr, nodeId, kind, r, g, b, intensity, castShadows)) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun setNodeLightEx(
        nodeId: Long,
        kind: Int,
        r: Float,
        g: Float,
        b: Float,
        intensity: Float,
        castShadows: Boolean,
        spotInnerDeg: Float,
        spotOuterDeg: Float,
        range: Float,
    ) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetLightEx(
                sessionPtr, nodeId, kind, r, g, b, intensity, castShadows,
                spotInnerDeg, spotOuterDeg, range,
            )
        ) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun undo() {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeUndo(sessionPtr)) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun redo() {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeRedo(sessionPtr)) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun newEmptyProject(title: String = "Untitled Scene") {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeResetEmptyScene(sessionPtr)
        projectTitle = title
        currentFileName = null
        hasUnsavedChanges = false
        refreshStateFromNative()
    }

    fun newStarterProject(title: String = "Studio Starter") {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeResetStarterScene(sessionPtr)
        projectTitle = title
        currentFileName = null
        hasUnsavedChanges = false
        refreshStateFromNative()
    }

    fun serializeProjectBytes(): ByteArray? {
        if (sessionPtr == 0L) return null
        return NovaNativeBridge.nativeSerializeProject(sessionPtr)
    }

    fun loadProjectBytes(
        title: String,
        fileName: String?,
        bytes: ByteArray,
        markDirty: Boolean = false,
    ): Boolean {
        if (sessionPtr == 0L) return false
        val ok = NovaNativeBridge.nativeDeserializeProject(sessionPtr, bytes)
        if (!ok) {
            reportNativeError("Could not open .nova3d file (invalid or unsupported format)")
            return false
        }
        projectTitle = title
        currentFileName = fileName
        hasUnsavedChanges = markDirty
        refreshStateFromNative()
        return true
    }

    fun markSaved(title: String, fileName: String) {
        projectTitle = title
        currentFileName = fileName
        hasUnsavedChanges = false
        statusBanner = "Saved \"$title\""
    }

    fun destroy() {
        destroyed = true
        pendingSurface = null
        surfaceViewRef = null
        isReady = false
        val ptr = sessionPtr
        sessionPtr = 0L
        renderHandler.post {
            if (ptr != 0L) {
                NovaNativeBridge.nativeDestroySession(ptr)
            }
            renderThread.quitSafely()
        }
    }

    private fun markSceneEdited() {
        hasUnsavedChanges = true
        onSceneEdited?.invoke()
    }

    private fun reportNativeError(fallback: String) {
        val err = if (sessionPtr != 0L) NovaNativeBridge.nativeGetLastError(sessionPtr) else ""
        errorMessage = err.ifBlank { fallback }
    }

    private fun refreshStateFromNative() {
        refreshStats()
        refreshNodes()
        refreshEditAndModifiers()
        refreshLookDev()
        refreshAnimation()
    }

    private fun refreshEditAndModifiers() {
        if (sessionPtr == 0L) return
        val es = NovaNativeBridge.nativeGetEditState(sessionPtr)
        if (es.size >= 17) {
            val iMode = if (es[0].toInt() == 1) InteractionUiMode.Edit else InteractionUiMode.Object
            val sMode = EditSelectUiMode.entries.firstOrNull { it.code == es[2].toInt() } ?: EditSelectUiMode.Face
            editState = EditUiState(
                interactionMode = iMode,
                editNodeId = es[1].toLong(),
                selectMode = sMode,
                showFaceOrientation = es[3] >= 0.5f,
                smoothShading = es[4] >= 0.5f,
                totalVerts = es[5].toInt(),
                totalEdges = es[6].toInt(),
                totalFaces = es[7].toInt(),
                selectedVertClosure = es[8].toInt(),
                selectedVerts = es[9].toInt(),
                selectedEdges = es[10].toInt(),
                selectedFaces = es[11].toInt(),
                hasCentroid = es[12] >= 0.5f,
                centroidX = es[13],
                centroidY = es[14],
                centroidZ = es[15],
            )
        }
        val rawMods = NovaNativeBridge.nativeGetModifiers(sessionPtr)
        val count = rawMods.size / 10
        val parsedMods = ArrayList<ModifierUiSpec>(count)
        for (i in 0 until count) {
            val o = i * 10
            val kind = ModifierUiKind.entries.firstOrNull { it.code == rawMods[o].toInt() } ?: ModifierUiKind.Mirror
            parsedMods.add(
                ModifierUiSpec(
                    index = i,
                    kind = kind,
                    enabled = rawMods[o + 1] >= 0.5f,
                    axis = rawMods[o + 2].toInt(),
                    merge = rawMods[o + 3] >= 0.5f,
                    levels = rawMods[o + 4].toInt(),
                    thickness = rawMods[o + 5],
                    count = rawMods[o + 6].toInt(),
                    offsetX = rawMods[o + 7],
                    offsetY = rawMods[o + 8],
                    offsetZ = rawMods[o + 9],
                )
            )
        }
        modifiers = parsedMods
    }

    // M3 Actions
    fun setInteractionMode(mode: InteractionUiMode) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetInteractionMode(sessionPtr, mode.code)) {
            refreshStateFromNative()
            statusBanner = if (mode == InteractionUiMode.Edit) "Edit Mode" else "Object Mode"
        } else {
            reportNativeError("Select a mesh object before entering Edit Mode")
        }
    }

    fun toggleInteractionMode() {
        val next = if (editState.interactionMode == InteractionUiMode.Edit) {
            InteractionUiMode.Object
        } else {
            InteractionUiMode.Edit
        }
        setInteractionMode(next)
    }

    fun setEditSelectMode(mode: EditSelectUiMode) {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeSetEditSelectMode(sessionPtr, mode.code)
        refreshStateFromNative()
    }

    fun toggleFaceOrientation() {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeSetFaceOrientation(sessionPtr, !editState.showFaceOrientation)
        refreshStateFromNative()
    }

    fun editClearOrSelectAll() {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeEditClearOrSelectAll(sessionPtr)
        refreshStateFromNative()
    }

    private fun runMeshEdit(actionName: String, op: (Long) -> Boolean) {
        val ptr = sessionPtr
        if (ptr == 0L) return
        if (op(ptr)) {
            refreshStateFromNative()
            markSceneEdited()
            statusBanner = actionName
        } else {
            reportNativeError("$actionName requires an active element selection")
        }
    }

    fun editExtrude(distance: Float = 0.35f) = runMeshEdit("Extrude") {
        NovaNativeBridge.nativeEditExtrude(it, distance)
    }

    fun editInset(factor: Float = 0.25f) = runMeshEdit("Inset") {
        NovaNativeBridge.nativeEditInset(it, factor)
    }

    fun editBevel(width: Float = 0.12f) = runMeshEdit("Bevel") {
        NovaNativeBridge.nativeEditBevel(it, width)
    }

    fun editLoopCut() = runMeshEdit("Loop Cut") {
        NovaNativeBridge.nativeEditLoopCut(it)
    }

    fun editSubdivide() = runMeshEdit("Subdivide") {
        NovaNativeBridge.nativeEditSubdivide(it)
    }

    fun editMerge() = runMeshEdit("Merge Vertices") {
        NovaNativeBridge.nativeEditMerge(it)
    }

    fun editDelete() = runMeshEdit("Delete Geometry") {
        NovaNativeBridge.nativeEditDelete(it)
    }

    fun editDissolveEdges() = runMeshEdit("Dissolve Edges") {
        NovaNativeBridge.nativeEditDissolveEdges(it)
    }

    fun editRecalculateNormals() = runMeshEdit("Recalculate Normals") {
        NovaNativeBridge.nativeEditRecalculateNormals(it)
    }

    fun editFlipNormals() = runMeshEdit("Flip Normals") {
        NovaNativeBridge.nativeEditFlipNormals(it)
    }

    fun editSetSmoothShading(smooth: Boolean) = runMeshEdit(if (smooth) "Shade Smooth" else "Shade Flat") {
        NovaNativeBridge.nativeEditSetSmoothShading(it, smooth)
    }

    fun editNudgeSelected(dx: Float, dy: Float, dz: Float) = runMeshEdit("Move Elements") {
        NovaNativeBridge.nativeEditTranslateSelected(it, dx, dy, dz)
    }

    fun addModifier(kind: ModifierUiKind) = runMeshEdit("Added ${kind.label}") {
        NovaNativeBridge.nativeModifierAdd(it, kind.code)
    }

    fun removeModifier(index: Int) = runMeshEdit("Removed Modifier") {
        NovaNativeBridge.nativeModifierRemove(it, index)
    }

    fun moveModifier(index: Int, delta: Int) = runMeshEdit("Reordered Modifier") {
        NovaNativeBridge.nativeModifierMove(it, index, delta)
    }

    fun updateModifier(spec: ModifierUiSpec) = runMeshEdit("Updated ${spec.kind.label}") {
        NovaNativeBridge.nativeModifierUpdate(
            it,
            spec.index,
            spec.kind.code,
            spec.enabled,
            spec.axis,
            spec.merge,
            spec.levels,
            spec.thickness,
            spec.count,
            spec.offsetX,
            spec.offsetY,
            spec.offsetZ,
        )
    }

    fun applyModifierStack() = runMeshEdit("Applied Modifiers") {
        NovaNativeBridge.nativeModifierApplyAll(it)
    }

    private fun refreshStats() {
        if (sessionPtr == 0L) return
        val stats = NovaNativeBridge.nativeGetViewportStats(sessionPtr)
        if (stats.size >= 11) {
            projectionMode = if (stats[0].toInt() == 1) {
                ProjectionUiMode.Orthographic
            } else {
                ProjectionUiMode.Perspective
            }
            cameraDistance = stats[1]
            cameraYawDeg = stats[2]
            cameraPitchDeg = stats[3]
            val flags = stats[5].toInt()
            showGrid = (flags and 1) != 0
            showAxes = (flags and 2) != 0
            shadingMode = ShadingUiMode.entries.firstOrNull { it.code == stats[6].toInt() }
                ?: ShadingUiMode.MaterialPreview
            gizmoMode = GizmoMode.entries.firstOrNull { it.code == stats[7].toInt() } ?: GizmoMode.Move
            gizmoSpace = if (stats[8].toInt() == 1) GizmoSpace.Local else GizmoSpace.World
            canUndo = stats[9] >= 0.5f
            canRedo = stats[10] >= 0.5f
        }
    }

    private fun refreshNodes() {
        if (sessionPtr == 0L) return
        val raw = NovaNativeBridge.nativeGetSceneNodeSummaries(sessionPtr)
        val parsed = raw.mapNotNull { entry ->
            val parts = entry.split('|', limit = 9)
            if (parts.size == 9) {
                SceneNodeSummary(
                    id = parts[0].toLongOrNull() ?: return@mapNotNull null,
                    parentId = parts[1].toLongOrNull() ?: 0L,
                    depth = parts[2].toIntOrNull() ?: 0,
                    kind = parts[3].toIntOrNull() ?: 0,
                    visible = parts[4] == "1",
                    selected = parts[5] == "1",
                    primarySelected = parts[6] == "1",
                    hasMesh = parts[7] == "1",
                    name = parts[8],
                )
            } else {
                null
            }
        }
        sceneNodes = parsed
        val primaryId = parsed.firstOrNull { it.primarySelected }?.id
            ?: parsed.firstOrNull { it.selected }?.id
        selectedNodeDetails = primaryId?.let { fetchNodeDetails(it) }
    }

    private fun refreshSelectedDetailsFromCurrent() {
        val currentId = selectedNodeDetails?.id ?: return
        selectedNodeDetails = fetchNodeDetails(currentId)
    }

    private fun fetchNodeDetails(nodeId: Long): NodeDetails? {
        if (sessionPtr == 0L || nodeId == 0L) return null
        val d = NovaNativeBridge.nativeGetNodeDetails(sessionPtr, nodeId)
        if (d.size < 28 || d[0] < 0.5f) return null
        return NodeDetails(
            id = nodeId,
            parentId = d[27].toLong(),
            kind = d[1].toInt(),
            visible = d[2] >= 0.5f,
            selected = d[3] >= 0.5f,
            primarySelected = d[4] >= 0.5f,
            hasMesh = d[5] >= 0.5f,
            posX = d[6],
            posY = d[7],
            posZ = d[8],
            rotX = d[9],
            rotY = d[10],
            rotZ = d[11],
            scaleX = d[12],
            scaleY = d[13],
            scaleZ = d[14],
            colorR = d[15],
            colorG = d[16],
            colorB = d[17],
            colorA = d[18],
            metallic = d[19],
            roughness = d[20],
            materialId = if (d.size > 28) d[28].toLong() else 0L,
            lightKind = d[21].toInt(),
            lightColorR = d[22],
            lightColorG = d[23],
            lightColorB = d[24],
            lightIntensity = d[25],
            lightCastShadows = d[26] >= 0.5f,
            lightSpotInnerDeg = if (d.size > 29) d[29] else 20f,
            lightSpotOuterDeg = if (d.size > 30) d[30] else 40f,
            lightRange = if (d.size > 31) d[31] else 16f,
            cameraFovDeg = if (d[1].toInt() == 3) NovaNativeBridge.nativeGetCameraFov(sessionPtr, nodeId) else 50f,
        )
    }

    // --- M4: Look Development (Materials, Textures, Environment & Lighting Rigs) ---

    fun selectMaterial(matId: Long) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSelectMaterial(sessionPtr, matId)) {
            refreshLookDev()
        }
    }

    fun createMaterialFromPreset(preset: MaterialUiPreset, assignSelected: Boolean = true) {
        if (sessionPtr == 0L) return
        val newId = NovaNativeBridge.nativeCreateMaterialPreset(sessionPtr, preset.code, assignSelected)
        if (newId > 0L) {
            refreshStateFromNative()
            markSceneEdited()
            showStatus(
                if (assignSelected && selectedNodeDetails?.hasMesh == true) {
                    "Created & assigned '${preset.label}'"
                } else {
                    "Added '${preset.label}' to Material Library"
                },
            )
        }
    }

    fun duplicateMaterial(matId: Long) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeDuplicateMaterial(sessionPtr, matId) > 0L) {
            refreshStateFromNative()
            markSceneEdited()
            showStatus("Duplicated material")
        }
    }

    fun renameMaterial(matId: Long, newName: String) {
        if (sessionPtr == 0L) return
        val trimmed = newName.trim()
        if (trimmed.isEmpty()) return
        if (NovaNativeBridge.nativeRenameMaterial(sessionPtr, matId, trimmed)) {
            refreshLookDev()
            markSceneEdited()
        }
    }

    fun deleteMaterial(matId: Long) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeDeleteMaterial(sessionPtr, matId)) {
            refreshStateFromNative()
            markSceneEdited()
            showStatus("Deleted material")
        } else {
            showError(NovaNativeBridge.nativeGetLastError(sessionPtr))
        }
    }

    fun updateMaterial(spec: MaterialUiSpec) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeUpdateMaterial(
                sessionPtr,
                spec.id,
                spec.r,
                spec.g,
                spec.b,
                spec.a,
                spec.metallic,
                spec.roughness,
                spec.emR,
                spec.emG,
                spec.emB,
                spec.emIntensity,
                spec.aoStrength,
                spec.alphaMode,
                spec.albedoTexId,
                spec.normalTexId,
                spec.roughnessTexId,
                spec.aoTexId,
                spec.emissiveTexId,
                spec.uvScaleX,
                spec.uvScaleY,
                spec.uvOffsetX,
                spec.uvOffsetY,
            )
        ) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun assignMaterialToSelected(matId: Long) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeAssignMaterialToSelected(sessionPtr, matId)) {
            refreshStateFromNative()
            markSceneEdited()
            val name = materials.firstOrNull { it.id == matId }?.name ?: "Material"
            showStatus("Assigned '$name' to selection")
        } else {
            showStatus("Select a mesh object first")
        }
    }

    fun markTextureMissing(texId: Long, missing: Boolean) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeMarkTextureMissing(sessionPtr, texId, missing)) {
            refreshLookDev()
            markSceneEdited()
            showStatus(if (missing) "Simulated missing texture (checker fallback)" else "Texture restored")
        }
    }

    fun registerMissingTestTexture(name: String, sourceUri: String): Long {
        if (sessionPtr == 0L) return 0L
        val id = NovaNativeBridge.nativeRegisterMissingTexture(sessionPtr, name, sourceUri)
        if (id > 0L) {
            refreshLookDev()
            markSceneEdited()
        }
        return id
    }

    suspend fun importImageTextureAsync(context: Context, uri: Uri, attachAsAlbedoToSelectedMat: Boolean = true) {
        if (sessionPtr == 0L) return
        val decoded = withContext(Dispatchers.IO) {
            runCatching {
                var displayName = "Imported Image"
                context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
                    if (c.moveToFirst()) {
                        val idx = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                        if (idx >= 0) {
                            displayName = c.getString(idx)?.substringBeforeLast('.') ?: displayName
                        }
                    }
                }
                val raw = context.contentResolver.openInputStream(uri)?.use { stream ->
                    BitmapFactory.decodeStream(stream)
                } ?: return@runCatching null

                val maxDim = 512
                val scale = (maxDim.toFloat() / raw.width.coerceAtLeast(raw.height).toFloat()).coerceAtMost(1f)
                val targetW = (raw.width * scale).toInt().coerceAtLeast(1)
                val targetH = (raw.height * scale).toInt().coerceAtLeast(1)
                val argb = if (targetW != raw.width || targetH != raw.height || raw.config != Bitmap.Config.ARGB_8888) {
                    val scaled = Bitmap.createScaledBitmap(raw, targetW, targetH, true)
                    val copy = scaled.copy(Bitmap.Config.ARGB_8888, false)
                    if (scaled !== raw) scaled.recycle()
                    raw.recycle()
                    copy
                } else {
                    raw
                }
                val bytes = ByteArray(argb.width * argb.height * 4)
                argb.copyPixelsToBuffer(ByteBuffer.wrap(bytes))
                val w = argb.width
                val h = argb.height
                argb.recycle()
                Triple(displayName, Pair(w, h), bytes)
            }.getOrNull()
        }

        if (decoded == null) {
            showError("Unable to decode image file")
            return
        }
        val (name, dims, rgba) = decoded
        val newTexId = NovaNativeBridge.nativeRegisterRgbaTexture(
            sessionPtr, name, uri.toString(), dims.first, dims.second, rgba,
        )
        if (newTexId <= 0L) {
            showError("Failed to register texture in project")
            return
        }
        if (attachAsAlbedoToSelectedMat) {
            val mat = materials.firstOrNull { it.id == selectedMaterialId }
            if (mat != null) {
                updateMaterial(mat.copy(albedoTexId = newTexId))
            }
        }
        refreshLookDev()
        markSceneEdited()
        showStatus("Imported '${name}' (${dims.first}x${dims.second})")
    }

    fun updateEnvironment(spec: EnvironmentUiSpec) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetEnvironment(
                sessionPtr,
                spec.preset,
                spec.rotationDeg,
                spec.intensity,
                spec.showSkybox,
                spec.bloomEnabled,
                spec.bloomStrength,
                spec.exposure,
            )
        ) {
            refreshLookDev()
            markSceneEdited()
        }
    }

    fun applyLightingRig(rig: LightingRigPreset) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeApplyLightingRig(sessionPtr, rig.code)) {
            refreshStateFromNative()
            markSceneEdited()
            showStatus("Applied ${rig.label} rig")
        }
    }

    fun setMaterialPreviewSphere(enabled: Boolean) {
        if (sessionPtr == 0L) return
        NovaNativeBridge.nativeSetMaterialPreviewSphere(sessionPtr, enabled)
        refreshLookDev()
    }

    fun refreshLookDev() {
        if (sessionPtr == 0L) return
        val rawMats = NovaNativeBridge.nativeGetMaterials(sessionPtr)
        materials = rawMats.mapNotNull { row ->
            val p = row.split('\t')
            if (p.size < 23) return@mapNotNull null
            MaterialUiSpec(
                id = p[0].toLongOrNull() ?: return@mapNotNull null,
                name = p[1],
                r = p[2].toFloatOrNull() ?: 0.8f,
                g = p[3].toFloatOrNull() ?: 0.8f,
                b = p[4].toFloatOrNull() ?: 0.8f,
                a = p[5].toFloatOrNull() ?: 1.0f,
                metallic = p[6].toFloatOrNull() ?: 0.0f,
                roughness = p[7].toFloatOrNull() ?: 0.4f,
                emR = p[8].toFloatOrNull() ?: 0f,
                emG = p[9].toFloatOrNull() ?: 0f,
                emB = p[10].toFloatOrNull() ?: 0f,
                emIntensity = p[11].toFloatOrNull() ?: 0f,
                aoStrength = p[12].toFloatOrNull() ?: 1f,
                alphaMode = p[13].toIntOrNull() ?: 0,
                albedoTexId = p[14].toLongOrNull() ?: 0L,
                normalTexId = p[15].toLongOrNull() ?: 0L,
                roughnessTexId = p[16].toLongOrNull() ?: 0L,
                aoTexId = p[17].toLongOrNull() ?: 0L,
                emissiveTexId = p[18].toLongOrNull() ?: 0L,
                uvScaleX = p[19].toFloatOrNull() ?: 1f,
                uvScaleY = p[20].toFloatOrNull() ?: 1f,
                uvOffsetX = p[21].toFloatOrNull() ?: 0f,
                uvOffsetY = p[22].toFloatOrNull() ?: 0f,
            )
        }
        selectedMaterialId = NovaNativeBridge.nativeGetSelectedMaterialId(sessionPtr)

        val rawTex = NovaNativeBridge.nativeGetTextures(sessionPtr)
        textures = rawTex.mapNotNull { row ->
            val p = row.split('\t')
            if (p.size < 6) return@mapNotNull null
            TextureUiSpec(
                id = p[0].toLongOrNull() ?: return@mapNotNull null,
                name = p[1],
                width = p[2].toIntOrNull() ?: 64,
                height = p[3].toIntOrNull() ?: 64,
                status = p[4].toIntOrNull() ?: 0,
                version = p[5].toLongOrNull() ?: 1L,
            )
        }
        missingTextureWarnings = NovaNativeBridge.nativeGetMissingTextureWarnings(sessionPtr).toList()

        val env = NovaNativeBridge.nativeGetEnvironment(sessionPtr)
        if (env.size >= 8) {
            environment = EnvironmentUiSpec(
                preset = env[0].toInt(),
                rotationDeg = env[1],
                intensity = env[2],
                showSkybox = env[3] >= 0.5f,
                bloomEnabled = env[4] >= 0.5f,
                bloomStrength = env[5],
                exposure = env[6],
                previewSphere = env[7] >= 0.5f,
            )
        }
    }

    // --- M5: Animation — Timeline, Keyframes, Playback, Camera View & Graph Editor ---

    fun setPlaying(playing: Boolean) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetPlaying(sessionPtr, playing)) {
            refreshAnimation()
            if (!playing) refreshStateFromNative()
        }
    }

    fun togglePlaying() {
        setPlaying(!timeline.playing)
    }

    fun rewindToStart() {
        setPlaying(false)
        setCurrentFrame(timeline.startFrame.toFloat())
    }

    fun setCurrentFrame(frame: Float) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetCurrentFrame(sessionPtr, frame)) {
            refreshStateFromNative()
        }
    }

    fun stepKeyframe(direction: Int) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeAnimStepKeyframe(sessionPtr, direction)) {
            refreshStateFromNative()
        }
    }

    fun updateTimelineConfig(
        startFrame: Int = timeline.startFrame,
        endFrame: Int = timeline.endFrame,
        fps: Float = timeline.fps,
        looping: Boolean = timeline.looping,
        autoKey: Boolean = timeline.autoKey,
    ) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetTimelineConfig(sessionPtr, startFrame, endFrame, fps, looping, autoKey)) {
            refreshAnimation()
            markSceneEdited()
        }
    }

    fun keySelectedNodeTrs(mask: Int = 7) {
        val node = selectedNodeDetails ?: return
        if (sessionPtr == 0L) return
        val frame = timeline.currentFrame.toInt()
        val count = NovaNativeBridge.nativeAnimInsertNodeTrsKeys(
            sessionPtr, node.id, frame, mask, defaultInterpolation.code,
        )
        if (count > 0) {
            refreshStateFromNative()
            markSceneEdited()
            showStatus("Keyed ${count} channel(s) @ F$frame")
        }
    }

    fun keySelectedLight() {
        val node = selectedNodeDetails ?: return
        if (sessionPtr == 0L || node.kind != 2) return
        val frame = timeline.currentFrame.toInt()
        val count = NovaNativeBridge.nativeAnimInsertLightKeys(
            sessionPtr, node.id, frame, defaultInterpolation.code,
        )
        if (count > 0) {
            refreshStateFromNative()
            markSceneEdited()
            showStatus("Keyed Light (${count}ch) @ F$frame")
        }
    }

    fun keySelectedCamera() {
        val node = selectedNodeDetails ?: return
        if (sessionPtr == 0L || node.kind != 3) return
        val frame = timeline.currentFrame.toInt()
        val count = NovaNativeBridge.nativeAnimInsertCameraKeys(
            sessionPtr, node.id, frame, defaultInterpolation.code,
        )
        if (count > 0) {
            refreshStateFromNative()
            markSceneEdited()
            showStatus("Keyed Camera (${count}ch) @ F$frame")
        }
    }

    fun keyMaterial(matId: Long) {
        if (sessionPtr == 0L || matId == 0L) return
        val frame = timeline.currentFrame.toInt()
        val count = NovaNativeBridge.nativeAnimInsertMaterialKeys(
            sessionPtr, matId, frame, defaultInterpolation.code,
        )
        if (count > 0) {
            refreshStateFromNative()
            markSceneEdited()
            showStatus("Keyed Material (${count}ch) @ F$frame")
        }
    }

    fun insertKeyframe(
        targetKind: Int,
        targetId: Long,
        channel: Int,
        frame: Int,
        value: Float,
        interpolation: InterpolationUiMode = defaultInterpolation,
    ) {
        if (sessionPtr == 0L) return
        val kid = NovaNativeBridge.nativeAnimInsertKeyframe(
            sessionPtr, targetKind, targetId, channel, frame, value, interpolation.code,
        )
        if (kid > 0L) {
            refreshStateFromNative()
            selectedKeyframeId = kid
            markSceneEdited()
        }
    }

    fun updateKeyframe(
        trackId: Long,
        keyId: Long,
        frame: Int,
        value: Float,
        interpolation: InterpolationUiMode,
        inDx: Float,
        inDy: Float,
        outDx: Float,
        outDy: Float,
    ) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeAnimUpdateKeyframe(
                sessionPtr, trackId, keyId, frame, value, interpolation.code,
                inDx, inDy, outDx, outDy,
            )
        ) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun deleteKeyframe(trackId: Long, keyId: Long) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeAnimDeleteKeyframe(sessionPtr, trackId, keyId)) {
            if (selectedKeyframeId == keyId) selectedKeyframeId = null
            refreshStateFromNative()
            markSceneEdited()
            showStatus("Deleted keyframe")
        }
    }

    fun setCameraNodeFov(nodeId: Long, fovDeg: Float) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetCameraFov(sessionPtr, nodeId, fovDeg)) {
            refreshStateFromNative()
            markSceneEdited()
        }
    }

    fun setActiveCameraNode(camNodeId: Long) {
        if (sessionPtr == 0L) return
        if (NovaNativeBridge.nativeSetActiveCameraNode(sessionPtr, camNodeId)) {
            refreshStateFromNative()
            val label = if (camNodeId == 0L) {
                "Orbit View"
            } else {
                sceneNodes.firstOrNull { it.id == camNodeId }?.name ?: "Camera"
            }
            showStatus("View: $label")
        }
    }

    fun toggleCameraView() {
        val active = timeline.activeCameraNodeId
        if (active != 0L) {
            setActiveCameraNode(0L)
            return
        }
        val selectedCam = selectedNodeDetails?.takeIf { it.kind == 3 }?.id
        val firstCam = sceneNodes.firstOrNull { it.kind == 3 }?.id
        val targetCam = selectedCam ?: firstCam
        if (targetCam != null) {
            setActiveCameraNode(targetCam)
        } else {
            showStatus("No Camera node in scene — Add > Camera first")
        }
    }

    fun trackTargetName(track: AnimTrackUi): String {
        return if (track.targetKind == 0) {
            sceneNodes.firstOrNull { it.id == track.targetId }?.name ?: "Node #${track.targetId}"
        } else {
            materials.firstOrNull { it.id == track.targetId }?.name ?: "Mat #${track.targetId}"
        }
    }

    fun refreshAnimation() {
        if (sessionPtr == 0L) return
        val tl = NovaNativeBridge.nativeGetTimelineState(sessionPtr)
        if (tl.size >= 9) {
            timeline = TimelineUiState(
                startFrame = tl[0].toInt(),
                endFrame = tl[1].toInt(),
                fps = tl[2],
                currentFrame = tl[3],
                playing = tl[4] >= 0.5f,
                looping = tl[5] >= 0.5f,
                autoKey = tl[6] >= 0.5f,
                activeCameraNodeId = tl[8].toLong(),
            )
        }
        val rawTracks = NovaNativeBridge.nativeGetAnimTracks(sessionPtr)
        val parsed = rawTracks.mapNotNull { row ->
            val p = row.split('\t', limit = 5)
            if (p.size < 5) return@mapNotNull null
            val tid = p[0].toLongOrNull() ?: return@mapNotNull null
            val tk = p[1].toIntOrNull() ?: 0
            val targetId = p[2].toLongOrNull() ?: return@mapNotNull null
            val ch = p[3].toIntOrNull() ?: 0
            val kfs = if (p[4].isBlank()) {
                emptyList()
            } else {
                p[4].split(';').mapNotNull { kStr ->
                    val kp = kStr.split(',')
                    if (kp.size < 8) return@mapNotNull null
                    val interpCode = kp[3].toIntOrNull() ?: 0
                    KeyframeUi(
                        id = kp[0].toLongOrNull() ?: return@mapNotNull null,
                        frame = kp[1].toIntOrNull() ?: 1,
                        value = kp[2].toFloatOrNull() ?: 0f,
                        interpolation = InterpolationUiMode.entries.firstOrNull { it.code == interpCode }
                            ?: InterpolationUiMode.Linear,
                        inHandleX = kp[4].toFloatOrNull() ?: -0.35f,
                        inHandleY = kp[5].toFloatOrNull() ?: 0f,
                        outHandleX = kp[6].toFloatOrNull() ?: 0.35f,
                        outHandleY = kp[7].toFloatOrNull() ?: 0f,
                    )
                }
            }
            AnimTrackUi(tid, tk, targetId, ch, kfs)
        }
        animTracks = parsed
        if (selectedTrackId == null || parsed.none { it.id == selectedTrackId }) {
            selectedTrackId = parsed.firstOrNull()?.id
        }
    }

    // ========================================================================
    // M6 Offscreen Rendering Thread Marshal & 3D Asset Import / Export
    // ========================================================================

    fun clearIoWarnings() {
        lastIoWarnings = emptyList()
    }

    fun setAdaptiveViewport(enabled: Boolean) {
        adaptiveViewportQuality = enabled
        renderHandler.post {
            if (destroyed || sessionPtr == 0L) return@post
            NovaNativeBridge.nativeSetViewportAdaptiveQuality(sessionPtr, enabled)
        }
    }

    suspend fun <T> runOnRenderThread(block: (sessionPtr: Long) -> T): Result<T> =
        suspendCancellableCoroutine { cont ->
            if (destroyed) {
                cont.resume(Result.failure(IllegalStateException("Controller destroyed")))
                return@suspendCancellableCoroutine
            }
            val posted = renderHandler.post {
                if (destroyed || sessionPtr == 0L) {
                    cont.resume(Result.failure(IllegalStateException("3D engine session is not active yet")))
                    return@post
                }
                try {
                    cont.resume(Result.success(block(sessionPtr)))
                } catch (t: Throwable) {
                    cont.resume(Result.failure(t))
                }
            }
            if (!posted) {
                cont.resume(Result.failure(IllegalStateException("Render thread unavailable")))
            }
        }

    fun refreshAfterExternalEdit() {
        refreshStateFromNative()
        markSceneEdited()
    }

    fun import3dModelAsync(
        context: Context,
        uri: Uri,
        onComplete: ((Boolean) -> Unit)? = null,
    ) {
        val displayName = resolveModelDisplayName(context, uri)
        val lower = displayName.lowercase(java.util.Locale.US)
        val isObj = lower.endsWith(".obj")
        val isGltf = lower.endsWith(".glb") || lower.endsWith(".gltf")

        renderHandler.post {
            if (destroyed || sessionPtr == 0L) {
                mainHandler.post {
                    errorMessage = "3D engine is not ready yet."
                    onComplete?.invoke(false)
                }
                return@post
            }

            val bytes = try {
                context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
            } catch (e: Exception) {
                null
            }
            if (bytes == null || bytes.isEmpty()) {
                mainHandler.post {
                    errorMessage = "Unable to read 3D asset file \"$displayName\"."
                    onComplete?.invoke(false)
                }
                return@post
            }

            val cleanTitle = displayName.substringBeforeLast('.').ifBlank { "Imported Asset" }
            val ok = when {
                isObj -> NovaNativeBridge.nativeImportObj(sessionPtr, bytes, cleanTitle)
                isGltf -> NovaNativeBridge.nativeImportGltf(sessionPtr, bytes, cleanTitle)
                else -> {
                    // Auto-sniff GLB binary magic ("glTF") or JSON '{' vs OBJ text
                    val isGlbMagic = bytes.size >= 4 &&
                        bytes[0] == 0x67.toByte() && bytes[1] == 0x6C.toByte() &&
                        bytes[2] == 0x54.toByte() && bytes[3] == 0x46.toByte()
                    val firstNonWs = bytes.firstOrNull { it > 0x20.toByte() }
                    if (isGlbMagic || firstNonWs == '{'.code.toByte()) {
                        NovaNativeBridge.nativeImportGltf(sessionPtr, bytes, cleanTitle)
                    } else {
                        NovaNativeBridge.nativeImportObj(sessionPtr, bytes, cleanTitle)
                    }
                }
            }

            val warningsRaw = NovaNativeBridge.nativeTakeIoWarnings(sessionPtr)
            val err = if (!ok) NovaNativeBridge.nativeGetLastError(sessionPtr) else ""
            if (ok) {
                NovaNativeBridge.nativeFrameAll(sessionPtr)
            }
            val parsedWarnings = warningsRaw.lines().map { it.trim() }.filter { it.isNotEmpty() }
            mainHandler.post {
                if (ok) {
                    lastIoWarnings = parsedWarnings
                    refreshStateFromNative()
                    markSceneEdited()
                    statusBanner = if (parsedWarnings.isEmpty()) {
                        "Imported \"$displayName\""
                    } else {
                        "Imported \"$displayName\" (${parsedWarnings.size} conversion note${if (parsedWarnings.size == 1) "" else "s"})"
                    }
                } else {
                    errorMessage = err.ifBlank { "Unsupported or corrupt 3D asset \"$displayName\"" }
                }
                onComplete?.invoke(ok)
            }
        }
    }

    suspend fun export3dModelBytes(format: String): Pair<ByteArray?, List<String>> =
        suspendCancellableCoroutine { cont ->
            val posted = renderHandler.post {
                if (destroyed || sessionPtr == 0L) {
                    cont.resume(Pair(null, emptyList()))
                    return@post
                }
                val bytes = when (format.lowercase(java.util.Locale.US)) {
                    "obj" -> NovaNativeBridge.nativeExportObj(sessionPtr)
                    "gltf" -> NovaNativeBridge.nativeExportGltf(sessionPtr, false)
                    else -> NovaNativeBridge.nativeExportGltf(sessionPtr, true)
                }
                val warningsRaw = NovaNativeBridge.nativeTakeIoWarnings(sessionPtr)
                val warnings = warningsRaw.lines().map { it.trim() }.filter { it.isNotEmpty() }
                val err = if (bytes == null) NovaNativeBridge.nativeGetLastError(sessionPtr) else ""
                mainHandler.post {
                    if (bytes != null) {
                        lastIoWarnings = warnings
                    } else if (err.isNotBlank()) {
                        errorMessage = err
                    }
                }
                cont.resume(Pair(bytes, warnings))
            }
            if (!posted) {
                cont.resume(Pair(null, emptyList()))
            }
        }

    private fun resolveModelDisplayName(context: Context, uri: Uri): String {
        context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
            ?.use { c ->
                val idx = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (idx >= 0 && c.moveToFirst()) {
                    val v = c.getString(idx)
                    if (!v.isNullOrBlank()) return v
                }
            }
        return uri.lastPathSegment?.substringAfterLast('/') ?: "asset"
    }
}
