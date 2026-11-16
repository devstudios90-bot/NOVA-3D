#include <jni.h>
#include <android/native_window_jni.h>

#include <memory>
#include <mutex>
#include <string>
#include <vector>

#include "filament_backend.h"
#include "nova_core.h"

namespace {

struct NovaSession {
    std::mutex mutex;
    NovaCoreHandle* core = nullptr;
    std::unique_ptr<nova::RenderBackend> backend;
    ANativeWindow* offscreenWindow = nullptr;
    std::string lastError;

    ~NovaSession() {
        if (backend) {
            backend->shutdown();
            backend.reset();
        }
        if (offscreenWindow) {
            ANativeWindow_release(offscreenWindow);
            offscreenWindow = nullptr;
        }
        if (core) {
            nova_core_destroy(core);
            core = nullptr;
        }
    }
};

NovaSession* fromHandle(jlong ptr) {
    return reinterpret_cast<NovaSession*>(static_cast<intptr_t>(ptr));
}

void recordCoreError(NovaSession* session) {
    char buf[512] = {0};
    nova_core_last_error(buf, sizeof(buf));
    if (buf[0] != '\0') {
        session->lastError = buf;
    }
}

} // namespace

extern "C" {

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeCreateSession(
    JNIEnv* /*env*/, jobject /*thiz*/, jint backendPreference) {
    auto session = std::make_unique<NovaSession>();
    session->core = nova_core_create();
    if (!session->core) {
        return 0;
    }

    auto filament = std::make_unique<nova::FilamentBackend>();
    const auto pref = (backendPreference == 1)
                          ? nova::GraphicsBackendPreference::OpenGLESOnly
                          : nova::GraphicsBackendPreference::AutoVulkanFirst;

    if (!filament->initialize(pref)) {
        session->lastError = filament->lastError();
        // Keep session alive? No — report failure honestly by returning 0 if renderer couldn't init,
        // or store error. Returning 0 if engine fails is clean; we also log via Android logcat.
        return 0;
    }

    session->backend = std::move(filament);
    return static_cast<jlong>(reinterpret_cast<intptr_t>(session.release()));
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeDestroySession(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    {
        std::lock_guard<std::mutex> lock(session->mutex);
        if (session->backend) {
            session->backend->shutdown();
        }
    }
    delete session;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAttachSurface(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jobject surface, jint width, jint height) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !surface) return JNI_FALSE;

    ANativeWindow* window = ANativeWindow_fromSurface(env, surface);
    if (!window) {
        std::lock_guard<std::mutex> lock(session->mutex);
        session->lastError = "ANativeWindow_fromSurface returned null.";
        return JNI_FALSE;
    }

    std::lock_guard<std::mutex> lock(session->mutex);
    const bool ok = session->backend && session->backend->attachSurface(window, width, height);
    if (!ok && session->backend) {
        session->lastError = session->backend->lastError();
    }
    ANativeWindow_release(window);
    return ok ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeResizeSurface(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint width, jint height) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (session->backend) {
        session->backend->resizeSurface(width, height);
    }
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeDetachSurface(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (session->backend) {
        session->backend->detachSurface();
    }
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeRenderFrame(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong frameTimeNanos) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (!session->backend || !session->core) return JNI_FALSE;
    nova_core_anim_tick(session->core, static_cast<uint64_t>(frameTimeNanos));
    return session->backend->renderFrame(session->core, static_cast<uint64_t>(frameTimeNanos))
               ? JNI_TRUE
               : JNI_FALSE;
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeOrbit(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat deltaYawRad, jfloat deltaPitchRad) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_camera_orbit(session->core, deltaYawRad, deltaPitchRad) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativePan(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat normDx, jfloat normDy) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_camera_pan(session->core, normDx, normDy) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeZoom(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat factor) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_camera_zoom(session->core, factor) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetProjection(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint projectionMode) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_camera_set_projection(session->core, static_cast<uint32_t>(projectionMode)) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetStandardView(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint standardView) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_camera_set_standard_view(session->core, static_cast<uint32_t>(standardView)) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeFrameAll(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    nova_core_camera_frame_all(session->core);
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeFocusSelected(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    nova_core_camera_focus_selected(session->core);
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetGridAxes(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jboolean showGrid, jboolean showAxes) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    nova_core_scene_set_grid_axes(session->core, showGrid == JNI_TRUE, showAxes == JNI_TRUE);
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSelectNode(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_select_node(session->core, static_cast<uint64_t>(nodeId)) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT jfloatArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetViewportStats(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    jfloatArray arr = env->NewFloatArray(12);
    if (!arr) return nullptr;
    float values[12] = {0};

    auto* session = fromHandle(sessionPtr);
    if (session) {
        std::lock_guard<std::mutex> lock(session->mutex);
        NovaCameraSnapshot snap{};
        if (nova_core_camera_get_snapshot(session->core, &snap) == 0) {
            values[0] = static_cast<float>(snap.projection);
            values[1] = snap.distance;
            values[2] = snap.yaw_radians * 57.2957795f;
            values[3] = snap.pitch_radians * 57.2957795f;
            values[4] = static_cast<float>(nova_core_scene_node_count(session->core));
            bool grid = true, axes = true;
            nova_core_scene_get_grid_axes(session->core, &grid, &axes);
            values[5] = (grid ? 1.0f : 0.0f) + (axes ? 2.0f : 0.0f);
            values[6] = static_cast<float>(nova_core_get_shading_mode(session->core));
            NovaGizmoSnapshot gsnap{};
            if (nova_core_gizmo_get_snapshot(session->core, &gsnap) == 0) {
                values[7] = static_cast<float>(gsnap.mode);
                values[8] = static_cast<float>(gsnap.space);
            }
            values[9] = nova_core_can_undo(session->core) ? 1.0f : 0.0f;
            values[10] = nova_core_can_redo(session->core) ? 1.0f : 0.0f;
            values[11] = static_cast<float>(nova_core_scene_revision(session->core) & 0xFFFFFFULL);
        }
    }
    env->SetFloatArrayRegion(arr, 0, 12, values);
    return arr;
}

JNIEXPORT jstring JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetBackendName(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return env->NewStringUTF("Uninitialized");
    std::lock_guard<std::mutex> lock(session->mutex);
    const std::string name = session->backend ? session->backend->activeBackendName() : "None";
    return env->NewStringUTF(name.c_str());
}

JNIEXPORT jstring JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetLastError(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return env->NewStringUTF("");
    std::lock_guard<std::mutex> lock(session->mutex);
    return env->NewStringUTF(session->lastError.c_str());
}

JNIEXPORT jobjectArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetSceneNodeSummaries(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    jclass strCls = env->FindClass("java/lang/String");
    auto* session = fromHandle(sessionPtr);
    if (!session) {
        return env->NewObjectArray(0, strCls, nullptr);
    }

    std::lock_guard<std::mutex> lock(session->mutex);
    const size_t count = nova_core_scene_node_count(session->core);
    jobjectArray result = env->NewObjectArray(static_cast<jsize>(count), strCls, nullptr);

    for (size_t i = 0; i < count; ++i) {
        NovaNodeMeta meta{};
        if (nova_core_scene_get_node_meta(session->core, i, &meta) != 0) {
            continue;
        }
        char nameBuf[128] = {0};
        nova_core_scene_get_node_name(session->core, meta.id, nameBuf, sizeof(nameBuf));
        for (char& ch : nameBuf) {
            if (ch == '\0') break;
            if (ch == '|') ch = ' ';
        }
        // Format: id|parentId|depth|kind|visible|selected|primarySelected|hasMesh|name
        std::string line = std::to_string(meta.id) + "|" +
                           std::to_string(meta.parent_id) + "|" +
                           std::to_string(meta.depth) + "|" +
                           std::to_string(meta.kind) + "|" +
                           (meta.visible ? "1" : "0") + "|" +
                           (meta.selected ? "1" : "0") + "|" +
                           (meta.primary_selected ? "1" : "0") + "|" +
                           (meta.has_mesh ? "1" : "0") + "|" +
                           nameBuf;
        jstring jLine = env->NewStringUTF(line.c_str());
        env->SetObjectArrayElement(result, static_cast<jsize>(i), jLine);
        env->DeleteLocalRef(jLine);
    }
    return result;
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSelectNodeExt(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId, jboolean additive) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_select_node_ext(session->core, static_cast<uint64_t>(nodeId), additive == JNI_TRUE) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativePickAtNdc(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat ndcX, jfloat ndcY, jfloat aspect, jboolean additive) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint64_t outId = 0;
    if (nova_core_scene_pick_at_ndc(session->core, ndcX, ndcY, aspect, additive == JNI_TRUE, &outId) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jlong>(outId);
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetShadingMode(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint mode) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_set_shading_mode(session->core, static_cast<uint32_t>(mode)) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeResetEmptyScene(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_reset_empty(session->core) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeResetStarterScene(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_reset_starter(session->core) != 0) {
        recordCoreError(session);
    }
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAddPrimitive(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint primitiveType) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint64_t outId = 0;
    if (nova_core_scene_add_primitive(session->core, static_cast<uint32_t>(primitiveType), &outId) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jlong>(outId);
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeDuplicateSelected(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_duplicate_selected(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeDeleteSelected(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_delete_selected(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeRenameNode(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jlong nodeId, jstring name) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !name) return JNI_FALSE;
    const char* utf = env->GetStringUTFChars(name, nullptr);
    if (!utf) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    const int32_t rc = nova_core_scene_rename_node(session->core, static_cast<uint64_t>(nodeId), utf);
    env->ReleaseStringUTFChars(name, utf);
    if (rc != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeReparentNode(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong childId, jlong newParentId) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_reparent_node(
            session->core, static_cast<uint64_t>(childId), static_cast<uint64_t>(newParentId)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGroupSelected(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint64_t outId = 0;
    if (nova_core_scene_group_selected(session->core, &outId) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jlong>(outId);
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeUnparentSelected(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_unparent_selected(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeApplyTransformSelected(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_apply_transform_selected(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeOriginToGeometrySelected(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_origin_to_geometry_selected(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetVisibility(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId, jboolean visible) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_scene_set_visibility(
            session->core, static_cast<uint64_t>(nodeId), visible == JNI_TRUE) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetTransform(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId,
    jfloat posX, jfloat posY, jfloat posZ,
    jfloat rotX, jfloat rotY, jfloat rotZ,
    jfloat scaleX, jfloat scaleY, jfloat scaleZ) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    const float pos[3] = {posX, posY, posZ};
    const float rot[3] = {rotX, rotY, rotZ};
    const float scl[3] = {scaleX, scaleY, scaleZ};
    if (nova_core_scene_set_transform(session->core, static_cast<uint64_t>(nodeId), pos, rot, scl) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetMaterial(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId,
    jfloat r, jfloat g, jfloat b, jfloat a, jfloat metallic, jfloat roughness) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    const float rgba[4] = {r, g, b, a};
    if (nova_core_scene_set_material(
            session->core, static_cast<uint64_t>(nodeId), rgba, metallic, roughness) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetLight(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId,
    jint kind, jfloat r, jfloat g, jfloat b, jfloat intensity, jboolean castShadows) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    const float rgb[3] = {r, g, b};
    if (nova_core_scene_set_light(
            session->core, static_cast<uint64_t>(nodeId), static_cast<uint32_t>(kind),
            rgb, intensity, castShadows == JNI_TRUE) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetLightEx(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId,
    jint kind, jfloat r, jfloat g, jfloat b, jfloat intensity, jboolean castShadows,
    jfloat spotInnerDeg, jfloat spotOuterDeg, jfloat range) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    const float rgb[3] = {r, g, b};
    if (nova_core_scene_set_light_ex(
            session->core, static_cast<uint64_t>(nodeId), static_cast<uint32_t>(kind),
            rgb, intensity, castShadows == JNI_TRUE, spotInnerDeg, spotOuterDeg, range) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jfloatArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetNodeDetails(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jlong nodeId) {
    jfloatArray arr = env->NewFloatArray(32);
    if (!arr) return nullptr;
    float out[32] = {0};

    auto* session = fromHandle(sessionPtr);
    if (session && nodeId != 0) {
        std::lock_guard<std::mutex> lock(session->mutex);
        NovaNodeDetails details{};
        if (nova_core_scene_get_node_details(session->core, static_cast<uint64_t>(nodeId), &details) == 0) {
            out[0] = 1.0f;
            out[1] = static_cast<float>(details.kind);
            out[2] = details.visible ? 1.0f : 0.0f;
            out[3] = details.selected ? 1.0f : 0.0f;
            out[4] = details.primary_selected ? 1.0f : 0.0f;
            out[5] = details.has_mesh ? 1.0f : 0.0f;
            out[6] = details.position[0];
            out[7] = details.position[1];
            out[8] = details.position[2];
            out[9] = details.rotation_euler[0];
            out[10] = details.rotation_euler[1];
            out[11] = details.rotation_euler[2];
            out[12] = details.scale[0];
            out[13] = details.scale[1];
            out[14] = details.scale[2];
            out[15] = details.base_color[0];
            out[16] = details.base_color[1];
            out[17] = details.base_color[2];
            out[18] = details.base_color[3];
            out[19] = details.metallic;
            out[20] = details.roughness;
            out[21] = static_cast<float>(details.light_kind);
            out[22] = details.light_color[0];
            out[23] = details.light_color[1];
            out[24] = details.light_color[2];
            out[25] = details.light_intensity;
            out[26] = details.light_cast_shadows ? 1.0f : 0.0f;
            out[27] = static_cast<float>(details.parent_id);
            out[28] = static_cast<float>(details.material_id);
            out[29] = details.light_spot_inner;
            out[30] = details.light_spot_outer;
            out[31] = details.light_range;
        }
    }
    env->SetFloatArrayRegion(arr, 0, 32, out);
    return arr;
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetGizmoMode(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint mode) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    nova_core_gizmo_set_mode(session->core, static_cast<uint32_t>(mode));
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetGizmoSpace(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint space) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    nova_core_gizmo_set_space(session->core, static_cast<uint32_t>(space));
}

JNIEXPORT jint JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativePickGizmoAxis(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat ndcX, jfloat ndcY, jfloat aspect) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    return static_cast<jint>(nova_core_gizmo_pick_axis(session->core, ndcX, ndcY, aspect));
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeBeginGizmoDrag(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint axis, jfloat ndcX, jfloat ndcY, jfloat aspect) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_gizmo_begin_drag(
               session->core, static_cast<uint32_t>(axis), ndcX, ndcY, aspect)
               ? JNI_TRUE
               : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeUpdateGizmoDrag(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat ndcX, jfloat ndcY, jfloat aspect) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_gizmo_update_drag(session->core, ndcX, ndcY, aspect) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEndGizmoDrag(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_gizmo_end_drag(session->core) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeUndo(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_undo(session->core) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeRedo(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_redo(session->core) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jbyteArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSerializeProject(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return nullptr;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint8_t* ptr = nullptr;
    size_t len = 0;
    if (nova_core_project_serialize(session->core, &ptr, &len) != 0 || !ptr) {
        recordCoreError(session);
        return nullptr;
    }
    jbyteArray arr = env->NewByteArray(static_cast<jsize>(len));
    if (arr) {
        env->SetByteArrayRegion(arr, 0, static_cast<jsize>(len), reinterpret_cast<const jbyte*>(ptr));
    }
    nova_core_free_buffer(ptr, len);
    return arr;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeDeserializeProject(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jbyteArray data) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !data) return JNI_FALSE;
    const jsize len = env->GetArrayLength(data);
    if (len <= 0) {
        session->lastError = "Empty project file";
        return JNI_FALSE;
    }
    jbyte* bytes = env->GetByteArrayElements(data, nullptr);
    if (!bytes) return JNI_FALSE;

    std::lock_guard<std::mutex> lock(session->mutex);
    const int32_t rc = nova_core_project_deserialize(
        session->core, reinterpret_cast<const uint8_t*>(bytes), static_cast<size_t>(len));
    env->ReleaseByteArrayElements(data, bytes, JNI_ABORT);
    if (rc != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

// -----------------------------------------------------------------------------
// M3: Edit Mode, Topology Operations & Non-Destructive Modifiers
// -----------------------------------------------------------------------------

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetInteractionMode(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint mode) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_set_interaction_mode(session->core, static_cast<uint32_t>(mode)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetEditSelectMode(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint selectMode) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_set_select_mode(session->core, static_cast<uint32_t>(selectMode)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetFaceOrientation(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jboolean enabled) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_set_face_orientation(session->core, enabled == JNI_TRUE) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditClearOrSelectAll(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_edit_clear_or_select_all(session->core) == 0 ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jfloatArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetEditState(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    jfloatArray out = env->NewFloatArray(17);
    if (!out) return nullptr;
    float buf[17] = {0};
    if (session) {
        std::lock_guard<std::mutex> lock(session->mutex);
        NovaEditState st{};
        if (nova_core_edit_get_state(session->core, &st) == 0) {
            buf[0] = static_cast<float>(st.interaction_mode);
            buf[1] = static_cast<float>(st.edit_node_id);
            buf[2] = static_cast<float>(st.edit_select_mode);
            buf[3] = st.show_face_orientation ? 1.0f : 0.0f;
            buf[4] = st.smooth_shading ? 1.0f : 0.0f;
            buf[5] = static_cast<float>(st.total_verts);
            buf[6] = static_cast<float>(st.total_edges);
            buf[7] = static_cast<float>(st.total_faces);
            buf[8] = static_cast<float>(st.selected_vert_closure);
            buf[9] = static_cast<float>(st.selected_verts);
            buf[10] = static_cast<float>(st.selected_edges);
            buf[11] = static_cast<float>(st.selected_faces);
            buf[12] = st.has_centroid ? 1.0f : 0.0f;
            buf[13] = st.centroid_world[0];
            buf[14] = st.centroid_world[1];
            buf[15] = st.centroid_world[2];
            buf[16] = static_cast<float>(st.modifier_count);
        }
    }
    env->SetFloatArrayRegion(out, 0, 17, buf);
    return out;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditExtrude(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat distance) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_extrude(session->core, distance) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditInset(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat factor) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_inset(session->core, factor) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditBevel(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat width) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_bevel(session->core, width) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditLoopCut(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_loop_cut(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditSubdivide(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_subdivide(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditMerge(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_merge(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditDelete(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_delete(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditDissolveEdges(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_dissolve_edges(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditRecalculateNormals(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_recalculate_normals(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditFlipNormals(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_flip_normals(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditSetSmoothShading(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jboolean smooth) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_set_smooth_shading(session->core, smooth == JNI_TRUE) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEditTranslateSelected(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat dx, jfloat dy, jfloat dz) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_edit_translate_selected(session->core, dx, dy, dz) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jfloatArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetModifiers(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return env->NewFloatArray(0);
    std::lock_guard<std::mutex> lock(session->mutex);
    const size_t cnt = nova_core_modifier_count(session->core);
    jfloatArray out = env->NewFloatArray(static_cast<jsize>(cnt * 10));
    if (!out || cnt == 0) return out;
    std::vector<float> buf(cnt * 10, 0.0f);
    for (size_t i = 0; i < cnt; ++i) {
        NovaModifierInfo m{};
        if (nova_core_modifier_get(session->core, i, &m) == 0) {
            const size_t o = i * 10;
            buf[o + 0] = static_cast<float>(m.kind);
            buf[o + 1] = m.enabled ? 1.0f : 0.0f;
            buf[o + 2] = static_cast<float>(m.axis);
            buf[o + 3] = m.merge ? 1.0f : 0.0f;
            buf[o + 4] = static_cast<float>(m.levels);
            buf[o + 5] = m.thickness;
            buf[o + 6] = static_cast<float>(m.count);
            buf[o + 7] = m.offset[0];
            buf[o + 8] = m.offset[1];
            buf[o + 9] = m.offset[2];
        }
    }
    env->SetFloatArrayRegion(out, 0, static_cast<jsize>(buf.size()), buf.data());
    return out;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeModifierAdd(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint kind) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_modifier_add(session->core, static_cast<uint32_t>(kind)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeModifierRemove(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint index) {
    auto* session = fromHandle(sessionPtr);
    if (!session || index < 0) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_modifier_remove(session->core, static_cast<size_t>(index)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeModifierMove(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint index, jint delta) {
    auto* session = fromHandle(sessionPtr);
    if (!session || index < 0) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_modifier_move(session->core, static_cast<size_t>(index), delta) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeModifierUpdate(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint index,
    jint kind, jboolean enabled, jint axis, jboolean merge, jint levels,
    jfloat thickness, jint count, jfloat offX, jfloat offY, jfloat offZ) {
    auto* session = fromHandle(sessionPtr);
    if (!session || index < 0) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    NovaModifierInfo m{};
    m.kind = static_cast<uint32_t>(kind);
    m.enabled = (enabled == JNI_TRUE);
    m.axis = static_cast<uint32_t>(axis);
    m.merge = (merge == JNI_TRUE);
    m.levels = static_cast<uint32_t>(levels);
    m.thickness = thickness;
    m.count = static_cast<uint32_t>(count);
    m.offset[0] = offX;
    m.offset[1] = offY;
    m.offset[2] = offZ;
    if (nova_core_modifier_update(session->core, static_cast<size_t>(index), &m) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeModifierApplyAll(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_modifier_apply_all(session->core) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

// -----------------------------------------------------------------------------
// M4: Look Development — Materials, Textures, HDR Environment & Lighting Rigs
// -----------------------------------------------------------------------------

JNIEXPORT jobjectArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetMaterials(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    jclass stringCls = env->FindClass("java/lang/String");
    auto* session = fromHandle(sessionPtr);
    if (!session) {
        return env->NewObjectArray(0, stringCls, nullptr);
    }
    std::lock_guard<std::mutex> lock(session->mutex);
    const size_t count = nova_core_material_count(session->core);
    jobjectArray arr = env->NewObjectArray(static_cast<jsize>(count), stringCls, nullptr);
    char nameBuf[128]{};
    char row[512]{};
    for (size_t i = 0; i < count; ++i) {
        NovaMaterialInfo m{};
        if (nova_core_material_get_by_index(session->core, i, &m) != 0) continue;
        nameBuf[0] = '\0';
        nova_core_material_get_name(session->core, m.id, nameBuf, sizeof(nameBuf));
        for (char* p = nameBuf; *p; ++p) {
            if (*p == '\t' || *p == '\n' || *p == '\r') *p = ' ';
        }
        std::snprintf(
            row, sizeof(row),
            "%llu\t%s\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%u\t%llu\t%llu\t%llu\t%llu\t%llu\t%.4f\t%.4f\t%.4f\t%.4f",
            static_cast<unsigned long long>(m.id), nameBuf,
            m.base_color[0], m.base_color[1], m.base_color[2], m.base_color[3],
            m.metallic, m.roughness,
            m.emissive[0], m.emissive[1], m.emissive[2], m.emissive_intensity,
            m.ao_strength, m.alpha_mode,
            static_cast<unsigned long long>(m.albedo_tex_id),
            static_cast<unsigned long long>(m.normal_tex_id),
            static_cast<unsigned long long>(m.roughness_tex_id),
            static_cast<unsigned long long>(m.ao_tex_id),
            static_cast<unsigned long long>(m.emissive_tex_id),
            m.uv_scale[0], m.uv_scale[1], m.uv_offset[0], m.uv_offset[1]);
        jstring s = env->NewStringUTF(row);
        env->SetObjectArrayElement(arr, static_cast<jsize>(i), s);
        env->DeleteLocalRef(s);
    }
    return arr;
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetSelectedMaterialId(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    return static_cast<jlong>(nova_core_material_selected_id(session->core));
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSelectMaterial(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong matId) {
    auto* session = fromHandle(sessionPtr);
    if (!session || matId <= 0) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_material_select(session->core, static_cast<uint64_t>(matId)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeCreateMaterialPreset(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint presetIdx, jboolean assignSelected) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint64_t outId = 0;
    if (nova_core_material_create_preset(
            session->core, static_cast<uint32_t>(presetIdx), assignSelected == JNI_TRUE, &outId) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jlong>(outId);
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeDuplicateMaterial(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong matId) {
    auto* session = fromHandle(sessionPtr);
    if (!session || matId <= 0) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint64_t outId = 0;
    if (nova_core_material_duplicate(session->core, static_cast<uint64_t>(matId), &outId) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jlong>(outId);
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeRenameMaterial(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jlong matId, jstring name) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !name || matId <= 0) return JNI_FALSE;
    const char* utf = env->GetStringUTFChars(name, nullptr);
    std::lock_guard<std::mutex> lock(session->mutex);
    const int32_t rc = nova_core_material_rename(session->core, static_cast<uint64_t>(matId), utf);
    env->ReleaseStringUTFChars(name, utf);
    if (rc != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeDeleteMaterial(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong matId) {
    auto* session = fromHandle(sessionPtr);
    if (!session || matId <= 0) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_material_delete(session->core, static_cast<uint64_t>(matId)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeUpdateMaterial(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong matId,
    jfloat r, jfloat g, jfloat b, jfloat a,
    jfloat metallic, jfloat roughness,
    jfloat emR, jfloat emG, jfloat emB, jfloat emIntensity,
    jfloat aoStrength, jint alphaMode,
    jlong albedoTexId, jlong normalTexId, jlong roughnessTexId, jlong aoTexId, jlong emissiveTexId,
    jfloat uvScaleX, jfloat uvScaleY, jfloat uvOffX, jfloat uvOffY) {
    auto* session = fromHandle(sessionPtr);
    if (!session || matId <= 0) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    NovaMaterialInfo m{};
    m.id = static_cast<uint64_t>(matId);
    m.base_color[0] = r;
    m.base_color[1] = g;
    m.base_color[2] = b;
    m.base_color[3] = a;
    m.metallic = metallic;
    m.roughness = roughness;
    m.emissive[0] = emR;
    m.emissive[1] = emG;
    m.emissive[2] = emB;
    m.emissive_intensity = emIntensity;
    m.ao_strength = aoStrength;
    m.alpha_mode = static_cast<uint32_t>(alphaMode);
    m.albedo_tex_id = static_cast<uint64_t>(std::max<jlong>(0, albedoTexId));
    m.normal_tex_id = static_cast<uint64_t>(std::max<jlong>(0, normalTexId));
    m.roughness_tex_id = static_cast<uint64_t>(std::max<jlong>(0, roughnessTexId));
    m.ao_tex_id = static_cast<uint64_t>(std::max<jlong>(0, aoTexId));
    m.emissive_tex_id = static_cast<uint64_t>(std::max<jlong>(0, emissiveTexId));
    m.uv_scale[0] = uvScaleX;
    m.uv_scale[1] = uvScaleY;
    m.uv_offset[0] = uvOffX;
    m.uv_offset[1] = uvOffY;
    if (nova_core_material_update(session->core, &m) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAssignMaterialToSelected(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong matId) {
    auto* session = fromHandle(sessionPtr);
    if (!session || matId <= 0) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_material_assign_to_selected(session->core, static_cast<uint64_t>(matId)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jobjectArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetTextures(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    jclass stringCls = env->FindClass("java/lang/String");
    auto* session = fromHandle(sessionPtr);
    if (!session) {
        return env->NewObjectArray(0, stringCls, nullptr);
    }
    std::lock_guard<std::mutex> lock(session->mutex);
    const size_t count = nova_core_texture_count(session->core);
    jobjectArray arr = env->NewObjectArray(static_cast<jsize>(count), stringCls, nullptr);
    char nameBuf[128]{};
    char row[256]{};
    for (size_t i = 0; i < count; ++i) {
        NovaTextureMeta t{};
        if (nova_core_texture_get_meta_by_index(session->core, i, &t) != 0) continue;
        nameBuf[0] = '\0';
        nova_core_texture_get_name(session->core, t.id, nameBuf, sizeof(nameBuf));
        for (char* p = nameBuf; *p; ++p) {
            if (*p == '\t' || *p == '\n' || *p == '\r') *p = ' ';
        }
        std::snprintf(
            row, sizeof(row), "%llu\t%s\t%u\t%u\t%u\t%llu",
            static_cast<unsigned long long>(t.id), nameBuf, t.width, t.height, t.status,
            static_cast<unsigned long long>(t.version));
        jstring s = env->NewStringUTF(row);
        env->SetObjectArrayElement(arr, static_cast<jsize>(i), s);
        env->DeleteLocalRef(s);
    }
    return arr;
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeRegisterRgbaTexture(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jstring name, jstring sourceUri,
    jint width, jint height, jbyteArray rgbaBytes) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !name || !sourceUri || !rgbaBytes) return 0;
    const jsize len = env->GetArrayLength(rgbaBytes);
    jbyte* ptr = env->GetByteArrayElements(rgbaBytes, nullptr);
    if (!ptr) return 0;
    const char* nameUtf = env->GetStringUTFChars(name, nullptr);
    const char* uriUtf = env->GetStringUTFChars(sourceUri, nullptr);
    uint64_t outId = 0;
    {
        std::lock_guard<std::mutex> lock(session->mutex);
        if (nova_core_texture_register_rgba(
                session->core, nameUtf, uriUtf,
                static_cast<uint32_t>(width), static_cast<uint32_t>(height),
                reinterpret_cast<const uint8_t*>(ptr), static_cast<size_t>(len), &outId) != 0) {
            recordCoreError(session);
            outId = 0;
        }
    }
    env->ReleaseStringUTFChars(name, nameUtf);
    env->ReleaseStringUTFChars(sourceUri, uriUtf);
    env->ReleaseByteArrayElements(rgbaBytes, ptr, JNI_ABORT);
    return static_cast<jlong>(outId);
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeRegisterMissingTexture(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jstring name, jstring sourceUri) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !name || !sourceUri) return 0;
    const char* nameUtf = env->GetStringUTFChars(name, nullptr);
    const char* uriUtf = env->GetStringUTFChars(sourceUri, nullptr);
    uint64_t outId = 0;
    {
        std::lock_guard<std::mutex> lock(session->mutex);
        if (nova_core_texture_register_missing(session->core, nameUtf, uriUtf, &outId) != 0) {
            recordCoreError(session);
            outId = 0;
        }
    }
    env->ReleaseStringUTFChars(name, nameUtf);
    env->ReleaseStringUTFChars(sourceUri, uriUtf);
    return static_cast<jlong>(outId);
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeMarkTextureMissing(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong texId, jboolean missing) {
    auto* session = fromHandle(sessionPtr);
    if (!session || texId <= 0) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_texture_mark_missing(session->core, static_cast<uint64_t>(texId), missing == JNI_TRUE) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jobjectArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetMissingTextureWarnings(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    jclass stringCls = env->FindClass("java/lang/String");
    auto* session = fromHandle(sessionPtr);
    if (!session) {
        return env->NewObjectArray(0, stringCls, nullptr);
    }
    std::lock_guard<std::mutex> lock(session->mutex);
    const size_t count = nova_core_texture_missing_warning_count(session->core);
    jobjectArray arr = env->NewObjectArray(static_cast<jsize>(count), stringCls, nullptr);
    char buf[256]{};
    for (size_t i = 0; i < count; ++i) {
        buf[0] = '\0';
        if (nova_core_texture_get_missing_warning(session->core, i, buf, sizeof(buf)) == 0) {
            jstring s = env->NewStringUTF(buf);
            env->SetObjectArrayElement(arr, static_cast<jsize>(i), s);
            env->DeleteLocalRef(s);
        }
    }
    return arr;
}

JNIEXPORT jfloatArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetEnvironment(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    jfloatArray arr = env->NewFloatArray(8);
    if (!arr) return nullptr;
    float out[8] = {0};
    auto* session = fromHandle(sessionPtr);
    if (session) {
        std::lock_guard<std::mutex> lock(session->mutex);
        NovaEnvironmentInfo e{};
        if (nova_core_env_get(session->core, &e) == 0) {
            out[0] = static_cast<float>(e.preset);
            out[1] = e.rotation_deg;
            out[2] = e.intensity;
            out[3] = e.show_skybox ? 1.0f : 0.0f;
            out[4] = e.bloom_enabled ? 1.0f : 0.0f;
            out[5] = e.bloom_strength;
            out[6] = e.exposure;
            out[7] = e.material_preview_sphere ? 1.0f : 0.0f;
        }
    }
    env->SetFloatArrayRegion(arr, 0, 8, out);
    return arr;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetEnvironment(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint preset, jfloat rotationDeg,
    jfloat intensity, jboolean showSkybox, jboolean bloomEnabled, jfloat bloomStrength, jfloat exposure) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    NovaEnvironmentInfo e{};
    e.preset = static_cast<uint32_t>(preset);
    e.rotation_deg = rotationDeg;
    e.intensity = intensity;
    e.show_skybox = (showSkybox == JNI_TRUE);
    e.bloom_enabled = (bloomEnabled == JNI_TRUE);
    e.bloom_strength = bloomStrength;
    e.exposure = exposure;
    if (nova_core_env_set(session->core, &e) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeApplyLightingRig(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint rigIdx) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_lighting_apply_rig(session->core, static_cast<uint32_t>(rigIdx)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetMaterialPreviewSphere(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jboolean enabled) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    nova_core_material_preview_sphere_set(session->core, enabled == JNI_TRUE);
}

// -----------------------------------------------------------------------------
// M5: Animation — Timeline, Keyframes, Playback, Camera View & Graph Editor
// -----------------------------------------------------------------------------

JNIEXPORT jfloatArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetTimelineState(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    jfloatArray arr = env->NewFloatArray(9);
    if (!arr) return nullptr;
    float out[9] = {1.0f, 120.0f, 30.0f, 1.0f, 0.0f, 1.0f, 0.0f, 0.0f, 0.0f};
    auto* session = fromHandle(sessionPtr);
    if (session) {
        std::lock_guard<std::mutex> lock(session->mutex);
        NovaTimelineInfo tl{};
        if (nova_core_anim_get_timeline(session->core, &tl) == 0) {
            out[0] = static_cast<float>(tl.start_frame);
            out[1] = static_cast<float>(tl.end_frame);
            out[2] = tl.fps;
            out[3] = tl.current_frame;
            out[4] = tl.playing ? 1.0f : 0.0f;
            out[5] = tl.looping ? 1.0f : 0.0f;
            out[6] = tl.auto_key ? 1.0f : 0.0f;
            out[7] = static_cast<float>(tl.track_count);
            out[8] = static_cast<float>(tl.active_camera_node_id);
        }
    }
    env->SetFloatArrayRegion(arr, 0, 9, out);
    return arr;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetPlaying(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jboolean playing) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_anim_set_playing(session->core, playing == JNI_TRUE) == 0 ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetCurrentFrame(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jfloat frame) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_anim_set_frame(session->core, frame) == 0 ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetTimelineConfig(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint startFrame, jint endFrame,
    jfloat fps, jboolean looping, jboolean autoKey) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_anim_set_timeline(
               session->core,
               static_cast<int32_t>(startFrame),
               static_cast<int32_t>(endFrame),
               fps,
               looping == JNI_TRUE,
               autoKey == JNI_TRUE) == 0
               ? JNI_TRUE
               : JNI_FALSE;
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAnimInsertKeyframe(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint targetKind, jlong targetId,
    jint channel, jint frame, jfloat value, jint interpolation) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint64_t tid = 0, kid = 0;
    if (nova_core_anim_insert_keyframe(
            session->core,
            static_cast<uint32_t>(targetKind),
            static_cast<uint64_t>(targetId),
            static_cast<uint32_t>(channel),
            static_cast<int32_t>(frame),
            value,
            static_cast<uint32_t>(interpolation),
            &tid,
            &kid) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jlong>(kid);
}

JNIEXPORT jint JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAnimInsertNodeTrsKeys(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId, jint frame, jint mask,
    jint interpolation) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint32_t count = 0;
    if (nova_core_anim_insert_node_trs_keys(
            session->core,
            static_cast<uint64_t>(nodeId),
            static_cast<int32_t>(frame),
            static_cast<uint32_t>(mask),
            static_cast<uint32_t>(interpolation),
            &count) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jint>(count);
}

JNIEXPORT jint JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAnimInsertLightKeys(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId, jint frame,
    jint interpolation) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint32_t count = 0;
    if (nova_core_anim_insert_light_keys(
            session->core,
            static_cast<uint64_t>(nodeId),
            static_cast<int32_t>(frame),
            static_cast<uint32_t>(interpolation),
            &count) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jint>(count);
}

JNIEXPORT jint JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAnimInsertCameraKeys(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId, jint frame,
    jint interpolation) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint32_t count = 0;
    if (nova_core_anim_insert_camera_keys(
            session->core,
            static_cast<uint64_t>(nodeId),
            static_cast<int32_t>(frame),
            static_cast<uint32_t>(interpolation),
            &count) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jint>(count);
}

JNIEXPORT jint JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAnimInsertMaterialKeys(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong matId, jint frame,
    jint interpolation) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint32_t count = 0;
    if (nova_core_anim_insert_material_keys(
            session->core,
            static_cast<uint64_t>(matId),
            static_cast<int32_t>(frame),
            static_cast<uint32_t>(interpolation),
            &count) != 0) {
        recordCoreError(session);
        return 0;
    }
    return static_cast<jint>(count);
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAnimUpdateKeyframe(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong trackId, jlong keyId, jint frame,
    jfloat value, jint interpolation, jfloat inDx, jfloat inDy, jfloat outDx, jfloat outDy) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_anim_update_keyframe(
            session->core,
            static_cast<uint64_t>(trackId),
            static_cast<uint64_t>(keyId),
            static_cast<int32_t>(frame),
            value,
            static_cast<uint32_t>(interpolation),
            inDx, inDy, outDx, outDy) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAnimDeleteKeyframe(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong trackId, jlong keyId) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (nova_core_anim_delete_keyframe(
            session->core,
            static_cast<uint64_t>(trackId),
            static_cast<uint64_t>(keyId)) != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeAnimStepKeyframe(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jint direction) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_anim_step_keyframe(session->core, static_cast<int32_t>(direction)) == 0
               ? JNI_TRUE
               : JNI_FALSE;
}

JNIEXPORT jobjectArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetAnimTracks(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    jclass stringCls = env->FindClass("java/lang/String");
    auto* session = fromHandle(sessionPtr);
    if (!session) {
        return env->NewObjectArray(0, stringCls, nullptr);
    }
    std::lock_guard<std::mutex> lock(session->mutex);
    const uint32_t count = nova_core_anim_track_count(session->core);
    jobjectArray arr = env->NewObjectArray(static_cast<jsize>(count), stringCls, nullptr);
    for (uint32_t i = 0; i < count; ++i) {
        NovaAnimTrackMeta tm{};
        if (nova_core_anim_track_get(session->core, i, &tm) != 0) continue;
        std::string row;
        row.reserve(64 + tm.keyframe_count * 48);
        char header[96]{};
        std::snprintf(
            header, sizeof(header), "%llu\t%u\t%llu\t%u\t",
            static_cast<unsigned long long>(tm.track_id),
            tm.target_kind,
            static_cast<unsigned long long>(tm.target_id),
            tm.channel);
        row.append(header);
        for (uint32_t k = 0; k < tm.keyframe_count; ++k) {
            NovaKeyframeMeta km{};
            if (nova_core_anim_track_keyframe_get(session->core, i, k, &km) != 0) continue;
            if (k > 0) row.push_back(';');
            char kbuf[128]{};
            std::snprintf(
                kbuf, sizeof(kbuf), "%llu,%d,%.5f,%u,%.4f,%.4f,%.4f,%.4f",
                static_cast<unsigned long long>(km.id),
                km.frame,
                km.value,
                km.interpolation,
                km.in_handle[0], km.in_handle[1],
                km.out_handle[0], km.out_handle[1]);
            row.append(kbuf);
        }
        jstring s = env->NewStringUTF(row.c_str());
        env->SetObjectArrayElement(arr, static_cast<jsize>(i), s);
        env->DeleteLocalRef(s);
    }
    return arr;
}

JNIEXPORT jfloat JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetCameraFov(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 50.0f;
    std::lock_guard<std::mutex> lock(session->mutex);
    float fov = 50.0f;
    nova_core_camera_get_fov(session->core, static_cast<uint64_t>(nodeId), &fov);
    return fov;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetCameraFov(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong nodeId, jfloat fovDeg) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_camera_set_fov(session->core, static_cast<uint64_t>(nodeId), fovDeg) == 0
               ? JNI_TRUE
               : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetActiveCameraNode(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jlong camNodeId) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    return nova_core_camera_set_active_node(session->core, static_cast<uint64_t>(camNodeId)) == 0
               ? JNI_TRUE
               : JNI_FALSE;
}

JNIEXPORT jlong JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeGetActiveCameraNode(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return 0;
    std::lock_guard<std::mutex> lock(session->mutex);
    return static_cast<jlong>(nova_core_camera_get_active_node(session->core));
}

// ============================================================================
// M6 Offscreen Rendering, Quality & 3D Format I/O
// ============================================================================

static nova::RenderQualityConfig makeQualityConfig(
    jint qualityLevel,
    jboolean aaEnabled,
    jint msaaSamples,
    jboolean ssaoEnabled,
    jboolean bloomEnabled,
    jboolean dofEnabled,
    jfloat dofFocusDistance,
    jfloat dofMaxAperture,
    jfloat dofCocScale,
    jint toneMapping) {
    nova::RenderQualityConfig cfg;
    cfg.qualityLevel = qualityLevel;
    cfg.aaEnabled = (aaEnabled == JNI_TRUE);
    cfg.msaaSamples = static_cast<uint8_t>(std::max<jint>(1, std::min<jint>(4, msaaSamples)));
    cfg.ssaoEnabled = (ssaoEnabled == JNI_TRUE);
    cfg.bloomEnabled = (bloomEnabled == JNI_TRUE);
    cfg.dofEnabled = (dofEnabled == JNI_TRUE);
    cfg.dofFocusDistance = dofFocusDistance;
    cfg.dofMaxAperture = dofMaxAperture;
    cfg.dofCocScale = dofCocScale;
    cfg.toneMapping = toneMapping;
    return cfg;
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeSetViewportAdaptiveQuality(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr, jboolean enabled) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !session->backend) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    session->backend->setViewportAdaptiveQuality(enabled == JNI_TRUE);
}

JNIEXPORT jbyteArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeRenderStillRgba(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr,
    jint width, jint height,
    jint qualityLevel, jboolean aaEnabled, jint msaaSamples,
    jboolean ssaoEnabled, jboolean bloomEnabled,
    jboolean dofEnabled, jfloat dofFocusDistance, jfloat dofMaxAperture, jfloat dofCocScale,
    jint toneMapping) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !session->backend || width <= 0 || height <= 0) return nullptr;
    std::lock_guard<std::mutex> lock(session->mutex);

    const auto cfg = makeQualityConfig(
        qualityLevel, aaEnabled, msaaSamples, ssaoEnabled, bloomEnabled,
        dofEnabled, dofFocusDistance, dofMaxAperture, dofCocScale, toneMapping);

    std::vector<uint8_t> rgba;
    std::string err;
    if (!session->backend->renderOffscreenStillRgba(
            session->core, static_cast<uint32_t>(width), static_cast<uint32_t>(height),
            cfg, rgba, err)) {
        session->lastError = err.empty() ? "Still render failed" : err;
        return nullptr;
    }

    jbyteArray out = env->NewByteArray(static_cast<jsize>(rgba.size()));
    if (!out) {
        session->lastError = "Out of memory allocating still pixel buffer.";
        return nullptr;
    }
    env->SetByteArrayRegion(out, 0, static_cast<jsize>(rgba.size()),
                            reinterpret_cast<const jbyte*>(rgba.data()));
    return out;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeBeginVideoSurface(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr,
    jobject surface, jint width, jint height,
    jint qualityLevel, jboolean aaEnabled, jint msaaSamples,
    jboolean ssaoEnabled, jboolean bloomEnabled,
    jboolean dofEnabled, jfloat dofFocusDistance, jfloat dofMaxAperture, jfloat dofCocScale,
    jint toneMapping) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !session->backend || !surface) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);

    if (session->offscreenWindow) {
        session->backend->endOffscreenSurface();
        ANativeWindow_release(session->offscreenWindow);
        session->offscreenWindow = nullptr;
    }

    ANativeWindow* win = ANativeWindow_fromSurface(env, surface);
    if (!win) {
        session->lastError = "Unable to acquire ANativeWindow from MediaCodec Surface.";
        return JNI_FALSE;
    }

    const auto cfg = makeQualityConfig(
        qualityLevel, aaEnabled, msaaSamples, ssaoEnabled, bloomEnabled,
        dofEnabled, dofFocusDistance, dofMaxAperture, dofCocScale, toneMapping);

    std::string err;
    if (!session->backend->beginOffscreenSurface(win, width, height, cfg, err)) {
        ANativeWindow_release(win);
        session->lastError = err.empty() ? "Failed to bind video encoder Surface" : err;
        return JNI_FALSE;
    }
    session->offscreenWindow = win;
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeRenderVideoFrame(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr,
    jlong ptsNanos, jint width, jint height) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !session->backend) return JNI_FALSE;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (!session->backend->renderOffscreenSurfaceFrame(
            session->core, static_cast<uint64_t>(ptsNanos), width, height)) {
        session->lastError = session->backend->lastError();
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT void JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeEndVideoSurface(
    JNIEnv* /*env*/, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return;
    std::lock_guard<std::mutex> lock(session->mutex);
    if (session->backend) {
        session->backend->endOffscreenSurface();
    }
    if (session->offscreenWindow) {
        ANativeWindow_release(session->offscreenWindow);
        session->offscreenWindow = nullptr;
    }
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeImportObj(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jbyteArray bytes, jstring name) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !bytes) return JNI_FALSE;
    const jsize len = env->GetArrayLength(bytes);
    if (len <= 0) return JNI_FALSE;
    std::vector<uint8_t> buf(static_cast<size_t>(len));
    env->GetByteArrayRegion(bytes, 0, len, reinterpret_cast<jbyte*>(buf.data()));
    const char* nameChars = name ? env->GetStringUTFChars(name, nullptr) : nullptr;
    std::lock_guard<std::mutex> lock(session->mutex);
    const int32_t rc = nova_core_import_obj(
        session->core, buf.data(), buf.size(), nameChars ? nameChars : "Imported OBJ");
    if (nameChars) env->ReleaseStringUTFChars(name, nameChars);
    if (rc != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeImportGltf(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jbyteArray bytes, jstring name) {
    auto* session = fromHandle(sessionPtr);
    if (!session || !bytes) return JNI_FALSE;
    const jsize len = env->GetArrayLength(bytes);
    if (len <= 0) return JNI_FALSE;
    std::vector<uint8_t> buf(static_cast<size_t>(len));
    env->GetByteArrayRegion(bytes, 0, len, reinterpret_cast<jbyte*>(buf.data()));
    const char* nameChars = name ? env->GetStringUTFChars(name, nullptr) : nullptr;
    std::lock_guard<std::mutex> lock(session->mutex);
    const int32_t rc = nova_core_import_gltf(
        session->core, buf.data(), buf.size(), nameChars ? nameChars : "Imported glTF");
    if (nameChars) env->ReleaseStringUTFChars(name, nameChars);
    if (rc != 0) {
        recordCoreError(session);
        return JNI_FALSE;
    }
    return JNI_TRUE;
}

JNIEXPORT jbyteArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeExportObj(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return nullptr;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint8_t* ptr = nullptr;
    uintptr_t len = 0;
    if (nova_core_export_obj(session->core, &ptr, &len) != 0 || !ptr) {
        recordCoreError(session);
        return nullptr;
    }
    jbyteArray out = env->NewByteArray(static_cast<jsize>(len));
    if (out && len > 0) {
        env->SetByteArrayRegion(out, 0, static_cast<jsize>(len),
                                reinterpret_cast<const jbyte*>(ptr));
    }
    nova_core_free_buffer(ptr, len);
    return out;
}

JNIEXPORT jbyteArray JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeExportGltf(
    JNIEnv* env, jobject /*thiz*/, jlong sessionPtr, jboolean asGlb) {
    auto* session = fromHandle(sessionPtr);
    if (!session) return nullptr;
    std::lock_guard<std::mutex> lock(session->mutex);
    uint8_t* ptr = nullptr;
    uintptr_t len = 0;
    if (nova_core_export_gltf(session->core, asGlb == JNI_TRUE, &ptr, &len) != 0 || !ptr) {
        recordCoreError(session);
        return nullptr;
    }
    jbyteArray out = env->NewByteArray(static_cast<jsize>(len));
    if (out && len > 0) {
        env->SetByteArrayRegion(out, 0, static_cast<jsize>(len),
                                reinterpret_cast<const jbyte*>(ptr));
    }
    nova_core_free_buffer(ptr, len);
    return out;
}

JNIEXPORT jstring JNICALL
Java_com_nova3d_studio_nativebridge_NovaNativeBridge_nativeTakeIoWarnings(
    JNIEnv* env, jobject /*thiz*/, jlong /*sessionPtr*/) {
    char buf[4096] = {0};
    const size_t n = nova_core_io_take_warnings(buf, sizeof(buf));
    if (n == 0) {
        return env->NewStringUTF("");
    }
    return env->NewStringUTF(buf);
}

} // extern "C"
