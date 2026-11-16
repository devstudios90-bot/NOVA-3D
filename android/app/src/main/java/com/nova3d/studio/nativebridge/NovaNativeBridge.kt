package com.nova3d.studio.nativebridge

import android.view.Surface

/**
 * Narrow JNI bridge to the C++ Filament renderer and Rust core engine.
 * All native scene mutations and rendering prep occur behind this boundary.
 */
object NovaNativeBridge {
    var nativeLibraryLoaded: Boolean = false
        private set
    var loadError: String? = null
        private set

    init {
        try {
            System.loadLibrary("c++_shared")
            System.loadLibrary("nova_renderer")
            nativeLibraryLoaded = true
        } catch (t: Throwable) {
            loadError = t.message ?: t::class.java.simpleName
            nativeLibraryLoaded = false
        }
    }

    external fun nativeCreateSession(backendPreference: Int): Long
    external fun nativeDestroySession(sessionPtr: Long)

    external fun nativeAttachSurface(sessionPtr: Long, surface: Surface, width: Int, height: Int): Boolean
    external fun nativeResizeSurface(sessionPtr: Long, width: Int, height: Int)
    external fun nativeDetachSurface(sessionPtr: Long)
    external fun nativeRenderFrame(sessionPtr: Long, frameTimeNanos: Long): Boolean

    external fun nativeOrbit(sessionPtr: Long, deltaYawRad: Float, deltaPitchRad: Float)
    external fun nativePan(sessionPtr: Long, normDx: Float, normDy: Float)
    external fun nativeZoom(sessionPtr: Long, factor: Float)
    external fun nativeSetProjection(sessionPtr: Long, projectionMode: Int)
    external fun nativeSetStandardView(sessionPtr: Long, standardView: Int)
    external fun nativeFrameAll(sessionPtr: Long)
    external fun nativeFocusSelected(sessionPtr: Long)
    external fun nativeSetGridAxes(sessionPtr: Long, showGrid: Boolean, showAxes: Boolean)
    external fun nativeSelectNode(sessionPtr: Long, nodeId: Long)
    external fun nativeSelectNodeExt(sessionPtr: Long, nodeId: Long, additive: Boolean)
    external fun nativePickAtNdc(
        sessionPtr: Long,
        ndcX: Float,
        ndcY: Float,
        aspect: Float,
        additive: Boolean,
    ): Long
    external fun nativeSetShadingMode(sessionPtr: Long, mode: Int)

    external fun nativeResetEmptyScene(sessionPtr: Long)
    external fun nativeResetStarterScene(sessionPtr: Long)
    external fun nativeAddPrimitive(sessionPtr: Long, primitiveType: Int): Long
    external fun nativeDuplicateSelected(sessionPtr: Long): Boolean
    external fun nativeDeleteSelected(sessionPtr: Long): Boolean
    external fun nativeRenameNode(sessionPtr: Long, nodeId: Long, name: String): Boolean
    external fun nativeReparentNode(sessionPtr: Long, childId: Long, newParentId: Long): Boolean
    external fun nativeGroupSelected(sessionPtr: Long): Long
    external fun nativeUnparentSelected(sessionPtr: Long): Boolean
    external fun nativeApplyTransformSelected(sessionPtr: Long): Boolean
    external fun nativeOriginToGeometrySelected(sessionPtr: Long): Boolean
    external fun nativeSetVisibility(sessionPtr: Long, nodeId: Long, visible: Boolean): Boolean
    external fun nativeSetTransform(
        sessionPtr: Long,
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
    ): Boolean
    external fun nativeSetMaterial(
        sessionPtr: Long,
        nodeId: Long,
        r: Float,
        g: Float,
        b: Float,
        a: Float,
        metallic: Float,
        roughness: Float,
    ): Boolean
    external fun nativeSetLight(
        sessionPtr: Long,
        nodeId: Long,
        kind: Int,
        r: Float,
        g: Float,
        b: Float,
        intensity: Float,
        castShadows: Boolean,
    ): Boolean
    external fun nativeSetLightEx(
        sessionPtr: Long,
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
    ): Boolean
    external fun nativeGetNodeDetails(sessionPtr: Long, nodeId: Long): FloatArray

    external fun nativeSetGizmoMode(sessionPtr: Long, mode: Int)
    external fun nativeSetGizmoSpace(sessionPtr: Long, space: Int)
    external fun nativePickGizmoAxis(
        sessionPtr: Long,
        ndcX: Float,
        ndcY: Float,
        aspect: Float,
    ): Int
    external fun nativeBeginGizmoDrag(
        sessionPtr: Long,
        axis: Int,
        ndcX: Float,
        ndcY: Float,
        aspect: Float,
    ): Boolean
    external fun nativeUpdateGizmoDrag(
        sessionPtr: Long,
        ndcX: Float,
        ndcY: Float,
        aspect: Float,
    ): Boolean
    external fun nativeEndGizmoDrag(sessionPtr: Long): Boolean

    external fun nativeUndo(sessionPtr: Long): Boolean
    external fun nativeRedo(sessionPtr: Long): Boolean

    external fun nativeSerializeProject(sessionPtr: Long): ByteArray?
    external fun nativeDeserializeProject(sessionPtr: Long, data: ByteArray): Boolean

    external fun nativeGetViewportStats(sessionPtr: Long): FloatArray
    external fun nativeGetBackendName(sessionPtr: Long): String
    external fun nativeGetLastError(sessionPtr: Long): String
    external fun nativeGetSceneNodeSummaries(sessionPtr: Long): Array<String>

    // M3: Edit Mode, Topology Operations & Modifier Stack
    external fun nativeSetInteractionMode(sessionPtr: Long, mode: Int): Boolean
    external fun nativeSetEditSelectMode(sessionPtr: Long, selectMode: Int): Boolean
    external fun nativeSetFaceOrientation(sessionPtr: Long, enabled: Boolean): Boolean
    external fun nativeEditClearOrSelectAll(sessionPtr: Long): Boolean
    external fun nativeGetEditState(sessionPtr: Long): FloatArray

    external fun nativeEditExtrude(sessionPtr: Long, distance: Float): Boolean
    external fun nativeEditInset(sessionPtr: Long, factor: Float): Boolean
    external fun nativeEditBevel(sessionPtr: Long, width: Float): Boolean
    external fun nativeEditLoopCut(sessionPtr: Long): Boolean
    external fun nativeEditSubdivide(sessionPtr: Long): Boolean
    external fun nativeEditMerge(sessionPtr: Long): Boolean
    external fun nativeEditDelete(sessionPtr: Long): Boolean
    external fun nativeEditDissolveEdges(sessionPtr: Long): Boolean
    external fun nativeEditRecalculateNormals(sessionPtr: Long): Boolean
    external fun nativeEditFlipNormals(sessionPtr: Long): Boolean
    external fun nativeEditSetSmoothShading(sessionPtr: Long, smooth: Boolean): Boolean
    external fun nativeEditTranslateSelected(sessionPtr: Long, dx: Float, dy: Float, dz: Float): Boolean

    external fun nativeGetModifiers(sessionPtr: Long): FloatArray
    external fun nativeModifierAdd(sessionPtr: Long, kind: Int): Boolean
    external fun nativeModifierRemove(sessionPtr: Long, index: Int): Boolean
    external fun nativeModifierMove(sessionPtr: Long, index: Int, delta: Int): Boolean
    external fun nativeModifierUpdate(
        sessionPtr: Long,
        index: Int,
        kind: Int,
        enabled: Boolean,
        axis: Int,
        merge: Boolean,
        levels: Int,
        thickness: Float,
        count: Int,
        offX: Float,
        offY: Float,
        offZ: Float,
    ): Boolean
    external fun nativeModifierApplyAll(sessionPtr: Long): Boolean

    // M4: Look Development — Materials, Textures, HDR Environment & Lighting Rigs
    external fun nativeGetMaterials(sessionPtr: Long): Array<String>
    external fun nativeGetSelectedMaterialId(sessionPtr: Long): Long
    external fun nativeSelectMaterial(sessionPtr: Long, matId: Long): Boolean
    external fun nativeCreateMaterialPreset(sessionPtr: Long, presetIdx: Int, assignSelected: Boolean): Long
    external fun nativeDuplicateMaterial(sessionPtr: Long, matId: Long): Long
    external fun nativeRenameMaterial(sessionPtr: Long, matId: Long, name: String): Boolean
    external fun nativeDeleteMaterial(sessionPtr: Long, matId: Long): Boolean
    external fun nativeUpdateMaterial(
        sessionPtr: Long,
        matId: Long,
        r: Float,
        g: Float,
        b: Float,
        a: Float,
        metallic: Float,
        roughness: Float,
        emR: Float,
        emG: Float,
        emB: Float,
        emIntensity: Float,
        aoStrength: Float,
        alphaMode: Int,
        albedoTexId: Long,
        normalTexId: Long,
        roughnessTexId: Long,
        aoTexId: Long,
        emissiveTexId: Long,
        uvScaleX: Float,
        uvScaleY: Float,
        uvOffX: Float,
        uvOffY: Float,
    ): Boolean
    external fun nativeAssignMaterialToSelected(sessionPtr: Long, matId: Long): Boolean

    external fun nativeGetTextures(sessionPtr: Long): Array<String>
    external fun nativeRegisterRgbaTexture(
        sessionPtr: Long,
        name: String,
        sourceUri: String,
        width: Int,
        height: Int,
        rgbaBytes: ByteArray,
    ): Long
    external fun nativeRegisterMissingTexture(sessionPtr: Long, name: String, sourceUri: String): Long
    external fun nativeMarkTextureMissing(sessionPtr: Long, texId: Long, missing: Boolean): Boolean
    external fun nativeGetMissingTextureWarnings(sessionPtr: Long): Array<String>

    external fun nativeGetEnvironment(sessionPtr: Long): FloatArray
    external fun nativeSetEnvironment(
        sessionPtr: Long,
        preset: Int,
        rotationDeg: Float,
        intensity: Float,
        showSkybox: Boolean,
        bloomEnabled: Boolean,
        bloomStrength: Float,
        exposure: Float,
    ): Boolean
    external fun nativeApplyLightingRig(sessionPtr: Long, rigIdx: Int): Boolean
    external fun nativeSetMaterialPreviewSphere(sessionPtr: Long, enabled: Boolean)

    // M5: Animation — Timeline, Keyframes, Playback, Camera View & Graph Editor
    external fun nativeGetTimelineState(sessionPtr: Long): FloatArray
    external fun nativeSetPlaying(sessionPtr: Long, playing: Boolean): Boolean
    external fun nativeSetCurrentFrame(sessionPtr: Long, frame: Float): Boolean
    external fun nativeSetTimelineConfig(
        sessionPtr: Long,
        startFrame: Int,
        endFrame: Int,
        fps: Float,
        looping: Boolean,
        autoKey: Boolean,
    ): Boolean
    external fun nativeAnimInsertKeyframe(
        sessionPtr: Long,
        targetKind: Int,
        targetId: Long,
        channel: Int,
        frame: Int,
        value: Float,
        interpolation: Int,
    ): Long
    external fun nativeAnimInsertNodeTrsKeys(
        sessionPtr: Long,
        nodeId: Long,
        frame: Int,
        mask: Int,
        interpolation: Int,
    ): Int
    external fun nativeAnimInsertLightKeys(
        sessionPtr: Long,
        nodeId: Long,
        frame: Int,
        interpolation: Int,
    ): Int
    external fun nativeAnimInsertCameraKeys(
        sessionPtr: Long,
        nodeId: Long,
        frame: Int,
        interpolation: Int,
    ): Int
    external fun nativeAnimInsertMaterialKeys(
        sessionPtr: Long,
        matId: Long,
        frame: Int,
        interpolation: Int,
    ): Int
    external fun nativeAnimUpdateKeyframe(
        sessionPtr: Long,
        trackId: Long,
        keyId: Long,
        frame: Int,
        value: Float,
        interpolation: Int,
        inDx: Float,
        inDy: Float,
        outDx: Float,
        outDy: Float,
    ): Boolean
    external fun nativeAnimDeleteKeyframe(sessionPtr: Long, trackId: Long, keyId: Long): Boolean
    external fun nativeAnimStepKeyframe(sessionPtr: Long, direction: Int): Boolean
    external fun nativeGetAnimTracks(sessionPtr: Long): Array<String>
    external fun nativeGetCameraFov(sessionPtr: Long, nodeId: Long): Float
    external fun nativeSetCameraFov(sessionPtr: Long, nodeId: Long, fovDeg: Float): Boolean
    external fun nativeSetActiveCameraNode(sessionPtr: Long, camNodeId: Long): Boolean
    external fun nativeGetActiveCameraNode(sessionPtr: Long): Long

    // M6 Offscreen Rendering & 3D Format Import/Export
    external fun nativeSetViewportAdaptiveQuality(sessionPtr: Long, enabled: Boolean)
    external fun nativeRenderStillRgba(
        sessionPtr: Long,
        width: Int,
        height: Int,
        qualityLevel: Int,
        aaEnabled: Boolean,
        msaaSamples: Int,
        ssaoEnabled: Boolean,
        bloomEnabled: Boolean,
        dofEnabled: Boolean,
        dofFocusDistance: Float,
        dofMaxAperture: Float,
        dofCocScale: Float,
        toneMapping: Int,
    ): ByteArray?

    external fun nativeBeginVideoSurface(
        sessionPtr: Long,
        surface: Surface,
        width: Int,
        height: Int,
        qualityLevel: Int,
        aaEnabled: Boolean,
        msaaSamples: Int,
        ssaoEnabled: Boolean,
        bloomEnabled: Boolean,
        dofEnabled: Boolean,
        dofFocusDistance: Float,
        dofMaxAperture: Float,
        dofCocScale: Float,
        toneMapping: Int,
    ): Boolean

    external fun nativeRenderVideoFrame(
        sessionPtr: Long,
        ptsNanos: Long,
        width: Int,
        height: Int,
    ): Boolean

    external fun nativeEndVideoSurface(sessionPtr: Long)

    external fun nativeImportObj(sessionPtr: Long, bytes: ByteArray, name: String): Boolean
    external fun nativeImportGltf(sessionPtr: Long, bytes: ByteArray, name: String): Boolean
    external fun nativeExportObj(sessionPtr: Long): ByteArray?
    external fun nativeExportGltf(sessionPtr: Long, asGlb: Boolean): ByteArray?
    external fun nativeTakeIoWarnings(sessionPtr: Long): String
}
