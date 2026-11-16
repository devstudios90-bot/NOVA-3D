#include "filament_backend.h"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <exception>
#include <unordered_set>
#include <vector>

#include <android/log.h>

#include <backend/DriverEnums.h>
#include <backend/PixelBufferDescriptor.h>
#include <filamat/MaterialBuilder.h>
#include <filament/Color.h>
#include <filament/ColorGrading.h>
#include <filament/LightManager.h>
#include <filament/MaterialEnums.h>
#include <filament/Options.h>
#include <filament/RenderableManager.h>
#include <filament/Skybox.h>
#include <filament/Texture.h>
#include <filament/TextureSampler.h>
#include <filament/ToneMapper.h>
#include <filament/TransformManager.h>
#include <filament/VertexBuffer.h>
#include <filament/Viewport.h>
#include <geometry/SurfaceOrientation.h>
#include <math/half.h>
#include <math/mat3.h>
#include <math/mat4.h>
#include <math/quat.h>
#include <math/vec3.h>
#include <math/vec4.h>
#include <utils/EntityManager.h>

#define NOVA_LOG_TAG "NovaFilament"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, NOVA_LOG_TAG, __VA_ARGS__)
#define LOGW(...) __android_log_print(ANDROID_LOG_WARN, NOVA_LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, NOVA_LOG_TAG, __VA_ARGS__)

namespace nova {

namespace {

constexpr size_t kMaxOverlayVerts = 8192;
constexpr size_t kMaxOverlayIndices = 16384;

void freeOwnedBuffer(void* buffer, size_t /*size*/, void* /*user*/) {
    delete[] static_cast<uint8_t*>(buffer);
}

template <typename T>
filament::backend::BufferDescriptor makeOwnedDescriptor(const T* data, size_t count) {
    const size_t byteSize = count * sizeof(T);
    auto* copy = new uint8_t[byteSize];
    std::memcpy(copy, data, byteSize);
    return filament::backend::BufferDescriptor(copy, byteSize, freeOwnedBuffer);
}

struct LineVertex {
    float x, y, z;
    uint32_t colorAbgr;
};

uint32_t rgba(uint8_t r, uint8_t g, uint8_t b, uint8_t a = 255) {
    return (static_cast<uint32_t>(a) << 24) |
           (static_cast<uint32_t>(b) << 16) |
           (static_cast<uint32_t>(g) << 8) |
           static_cast<uint32_t>(r);
}

filament::math::float3 transformMatPoint(const float m[16], float x, float y, float z) {
    return {
        m[0] * x + m[4] * y + m[8] * z + m[12],
        m[1] * x + m[5] * y + m[9] * z + m[13],
        m[2] * x + m[6] * y + m[10] * z + m[14],
    };
}

void pushLine(std::vector<LineVertex>& verts, std::vector<uint32_t>& indices,
              const filament::math::float3& a, const filament::math::float3& b, uint32_t color) {
    if (verts.size() + 2 > kMaxOverlayVerts || indices.size() + 2 > kMaxOverlayIndices) {
        return;
    }
    const uint32_t base = static_cast<uint32_t>(verts.size());
    verts.push_back({a.x, a.y, a.z, color});
    verts.push_back({b.x, b.y, b.z, color});
    indices.push_back(base);
    indices.push_back(base + 1);
}

void pushWireBox(std::vector<LineVertex>& verts, std::vector<uint32_t>& indices,
                 const float m[16], const float bmin[3], const float bmax[3], uint32_t color) {
    filament::math::float3 c[8] = {
        transformMatPoint(m, bmin[0], bmin[1], bmin[2]),
        transformMatPoint(m, bmax[0], bmin[1], bmin[2]),
        transformMatPoint(m, bmax[0], bmax[1], bmin[2]),
        transformMatPoint(m, bmin[0], bmax[1], bmin[2]),
        transformMatPoint(m, bmin[0], bmin[1], bmax[2]),
        transformMatPoint(m, bmax[0], bmin[1], bmax[2]),
        transformMatPoint(m, bmax[0], bmax[1], bmax[2]),
        transformMatPoint(m, bmin[0], bmax[1], bmax[2]),
    };
    const int edges[12][2] = {
        {0, 1}, {1, 2}, {2, 3}, {3, 0},
        {4, 5}, {5, 6}, {6, 7}, {7, 4},
        {0, 4}, {1, 5}, {2, 6}, {3, 7},
    };
    for (const auto& e : edges) {
        pushLine(verts, indices, c[e[0]], c[e[1]], color);
    }
}

void pushSelectionBox(std::vector<LineVertex>& verts, std::vector<uint32_t>& indices,
                      const float m[16], const float bmin[3], const float bmax[3],
                      uint32_t frameColor, uint32_t cornerColor, bool primary) {
    filament::math::float3 c[8] = {
        transformMatPoint(m, bmin[0], bmin[1], bmin[2]),
        transformMatPoint(m, bmax[0], bmin[1], bmin[2]),
        transformMatPoint(m, bmax[0], bmax[1], bmin[2]),
        transformMatPoint(m, bmin[0], bmax[1], bmin[2]),
        transformMatPoint(m, bmin[0], bmin[1], bmax[2]),
        transformMatPoint(m, bmax[0], bmin[1], bmax[2]),
        transformMatPoint(m, bmax[0], bmax[1], bmax[2]),
        transformMatPoint(m, bmin[0], bmax[1], bmax[2]),
    };
    const int edges[12][2] = {
        {0, 1}, {1, 2}, {2, 3}, {3, 0},
        {4, 5}, {5, 6}, {6, 7}, {7, 4},
        {0, 4}, {1, 5}, {2, 6}, {3, 7},
    };
    // 1. Sleek perimeter bounding frame
    for (const auto& e : edges) {
        pushLine(verts, indices, c[e[0]], c[e[1]], frameColor);
    }

    // 2. Bold glowing corner brackets (Studio-grade selection)
    const int cornerAdj[8][3] = {
        {1, 3, 4}, {0, 2, 5}, {1, 3, 6}, {2, 0, 7},
        {5, 7, 0}, {4, 6, 1}, {5, 7, 2}, {6, 4, 3},
    };
    for (int i = 0; i < 8; ++i) {
        for (int k = 0; k < 3; ++k) {
            const int adj = cornerAdj[i][k];
            const filament::math::float3 dir = c[adj] - c[i];
            const float len = std::sqrt(dir.x * dir.x + dir.y * dir.y + dir.z * dir.z);
            const float bracketLen = std::min(len * 0.32f, 0.45f);
            if (len > 1e-4f) {
                const filament::math::float3 p = c[i] + dir * (bracketLen / len);
                pushLine(verts, indices, c[i], p, cornerColor);
            }
        }
    }

    // 3. Ground footprint projection cross for spatial depth on floor grid
    if (primary) {
        const float cx = 0.5f * (bmin[0] + bmax[0]);
        const float cz = 0.5f * (bmin[2] + bmax[2]);
        filament::math::float3 groundCenter = transformMatPoint(m, cx, bmin[1], cz);
        groundCenter.y = 0.005f;
        const float r = std::max(0.25f, std::min(1.5f, 0.35f * (bmax[0] - bmin[0] + bmax[2] - bmin[2])));
        pushLine(verts, indices, {groundCenter.x - r, groundCenter.y, groundCenter.z},
                                {groundCenter.x + r, groundCenter.y, groundCenter.z}, cornerColor);
        pushLine(verts, indices, {groundCenter.x, groundCenter.y, groundCenter.z - r},
                                {groundCenter.x, groundCenter.y, groundCenter.z + r}, cornerColor);
    }
}

filament::math::float3 normalizeVec(const filament::math::float3& v,
                                    const filament::math::float3& fallback) {
    const float len = std::sqrt(v.x * v.x + v.y * v.y + v.z * v.z);
    if (len > 1e-6f) {
        return {v.x / len, v.y / len, v.z / len};
    }
    return fallback;
}

filament::math::float3 crossVec(const filament::math::float3& a,
                                const filament::math::float3& b) {
    return {
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    };
}

void perpBasis(const filament::math::float3& dir,
               filament::math::float3& outU, filament::math::float3& outV) {
    const filament::math::float3 ref =
        (std::fabs(dir.y) < 0.9f) ? filament::math::float3{0, 1, 0}
                                  : filament::math::float3{1, 0, 0};
    outU = normalizeVec(crossVec(dir, ref), {1, 0, 0});
    outV = normalizeVec(crossVec(dir, outU), {0, 0, 1});
}

filament::math::float3 cubemapFaceDir(int face, float u, float v) {
    // OpenGL/Filament cubemap convention for faces +X,-X,+Y,-Y,+Z,-Z
    switch (face) {
        case 0: return normalizeVec({1.0f, -v, -u}, {1, 0, 0});
        case 1: return normalizeVec({-1.0f, -v, u}, {-1, 0, 0});
        case 2: return normalizeVec({u, 1.0f, v}, {0, 1, 0});
        case 3: return normalizeVec({u, -1.0f, -v}, {0, -1, 0});
        case 4: return normalizeVec({u, -v, 1.0f}, {0, 0, 1});
        default: return normalizeVec({-u, -v, -1.0f}, {0, 0, -1});
    }
}

float dotVec(const filament::math::float3& a, const filament::math::float3& b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

filament::math::float3 lerpVec(const filament::math::float3& a, const filament::math::float3& b, float t) {
    const float k = std::max(0.0f, std::min(1.0f, t));
    return {a.x + (b.x - a.x) * k, a.y + (b.y - a.y) * k, a.z + (b.z - a.z) * k};
}

// Procedural HDR environment radiance used for both skybox background and prefiltered IBL mip chain.
filament::math::float3 evalEnvRadiance(uint32_t preset, const filament::math::float3& dir, float blurFactor) {
    const float y = std::max(-1.0f, std::min(1.0f, dir.y));
    const float upT = std::max(0.0f, y);
    const float horizonT = std::pow(1.0f - std::fabs(y), 3.0f);
    const float groundT = std::max(0.0f, -y);
    const float specPow = std::max(3.0f, 90.0f * (1.0f - blurFactor * 0.92f));
    const float softPow = std::max(2.0f, 18.0f * (1.0f - blurFactor * 0.85f));
    const float highlightBoost = 1.0f - 0.65f * blurFactor;

    auto spot = [&](const filament::math::float3& lightDir, float power, float mult) {
        const float d = std::max(0.0f, dotVec(dir, normalizeVec(lightDir, {0, 1, 0})));
        return std::pow(d, power) * mult;
    };

    filament::math::float3 col{0.3f, 0.35f, 0.42f};
    switch (preset) {
        case 0: { // StudioSoft: neutral gradient + two softbox panels + cool floor bounce
            col = lerpVec({0.14f, 0.16f, 0.20f}, {0.58f, 0.66f, 0.78f}, 0.5f + 0.5f * y);
            const float box1 = spot({0.58f, 0.72f, 0.38f}, softPow, 2.2f * highlightBoost);
            const float box2 = spot({-0.68f, 0.45f, -0.52f}, softPow * 0.8f, 1.35f * highlightBoost);
            col.x += box1 * 1.0f + box2 * 0.78f;
            col.y += box1 * 0.97f + box2 * 0.88f;
            col.z += box1 * 0.92f + box2 * 1.05f;
            break;
        }
        case 1: { // StudioContrast: dark studio + punchy warm/cool key softboxes
            col = lerpVec({0.03f, 0.035f, 0.045f}, {0.16f, 0.19f, 0.24f}, upT);
            col.x += horizonT * 0.08f;
            col.y += horizonT * 0.09f;
            col.z += horizonT * 0.12f;
            const float key = spot({0.62f, 0.68f, 0.36f}, specPow * 0.55f, 3.8f * highlightBoost);
            const float rim = spot({-0.75f, 0.28f, -0.58f}, softPow, 2.1f * highlightBoost);
            col.x += key * 1.05f + rim * 0.45f;
            col.y += key * 0.96f + rim * 0.75f;
            col.z += key * 0.85f + rim * 1.15f;
            break;
        }
        case 2: { // OutdoorDaylight: clear blue zenith, bright horizon, sunlit disc, warm grass/earth bounce
            const filament::math::float3 sky = lerpVec({0.68f, 0.82f, 0.96f}, {0.18f, 0.45f, 0.95f}, std::pow(upT, 0.6f));
            const filament::math::float3 ground = lerpVec({0.26f, 0.24f, 0.20f}, {0.14f, 0.16f, 0.12f}, groundT);
            col = (y >= 0.0f) ? sky : ground;
            const float sun = spot({0.55f, 0.74f, 0.38f}, specPow, 5.2f * highlightBoost);
            const float halo = spot({0.55f, 0.74f, 0.38f}, 6.0f, 0.55f);
            col.x += (sun + halo) * 1.05f;
            col.y += (sun + halo) * 0.96f;
            col.z += (sun + halo) * 0.80f;
            break;
        }
        case 3: { // GoldenSunset: indigo upper sky, amber/magenta horizon, low golden sun
            const filament::math::float3 sky = lerpVec({1.15f, 0.42f, 0.18f}, {0.12f, 0.15f, 0.34f}, std::pow(upT, 0.45f));
            const filament::math::float3 ground = {0.12f, 0.08f, 0.09f};
            col = (y >= -0.05f) ? sky : lerpVec(sky, ground, std::min(1.0f, (-y - 0.05f) * 6.0f));
            const float sun = spot({0.72f, 0.20f, -0.66f}, specPow * 0.7f, 4.6f * highlightBoost);
            const float glow = spot({0.72f, 0.20f, -0.66f}, 4.0f, 0.65f);
            col.x += sun * 1.20f + glow * 1.0f;
            col.y += sun * 0.65f + glow * 0.42f;
            col.z += sun * 0.25f + glow * 0.18f;
            break;
        }
        case 4: { // NeonNight: deep midnight violet sky + cyan/magenta neon city strip horizon
            col = lerpVec({0.04f, 0.04f, 0.07f}, {0.03f, 0.02f, 0.09f}, upT);
            const float cyanGlow = spot({-0.80f, 0.14f, 0.58f}, softPow * 0.8f, 2.8f * highlightBoost);
            const float magGlow = spot({0.78f, 0.18f, -0.59f}, softPow * 0.8f, 2.6f * highlightBoost);
            col.x += horizonT * 0.22f + magGlow * 1.10f + cyanGlow * 0.12f;
            col.y += horizonT * 0.12f + magGlow * 0.18f + cyanGlow * 0.92f;
            col.z += horizonT * 0.35f + magGlow * 0.85f + cyanGlow * 1.15f;
            break;
        }
        default: { // NeutralGrey: balanced studio white/grey dome
            const float v = 0.40f + 0.18f * y;
            col = {v, v, v};
            const float top = spot({0.3f, 0.9f, 0.3f}, 6.0f, 0.85f * highlightBoost);
            col.x += top;
            col.y += top;
            col.z += top;
            break;
        }
    }
    return col;
}

filament::Texture* createProceduralEnvCubemap(
    filament::Engine* engine, uint32_t preset, uint32_t baseSize, uint8_t levels,
    float yRotationRad, filament::math::float3* outSh9) {
    auto* tex = filament::Texture::Builder()
                    .width(baseSize)
                    .height(baseSize)
                    .levels(levels)
                    .sampler(filament::Texture::Sampler::SAMPLER_CUBEMAP)
                    .format(filament::Texture::InternalFormat::RGBA16F)
                    .usage(filament::Texture::Usage::DEFAULT | filament::Texture::Usage::SAMPLEABLE)
                    .build(*engine);
    if (!tex) {
        return nullptr;
    }

    const float cRot = std::cos(yRotationRad);
    const float sRot = std::sin(yRotationRad);

    filament::math::float3 rawSh[9]{};
    float weightSum = 0.0f;

    for (uint8_t lvl = 0; lvl < levels; ++lvl) {
        const uint32_t sz = std::max(1u, baseSize >> lvl);
        const float blurFactor = (levels <= 1) ? 0.0f : static_cast<float>(lvl) / static_cast<float>(levels - 1);
        const size_t faceFloats = static_cast<size_t>(sz) * sz * 4;
        auto* halfBuf = new filament::math::half[faceFloats * 6];
        for (int face = 0; face < 6; ++face) {
            filament::math::half* dst = halfBuf + face * faceFloats;
            for (uint32_t py = 0; py < sz; ++py) {
                const float v = ((static_cast<float>(py) + 0.5f) / static_cast<float>(sz)) * 2.0f - 1.0f;
                for (uint32_t px = 0; px < sz; ++px) {
                    const float u = ((static_cast<float>(px) + 0.5f) / static_cast<float>(sz)) * 2.0f - 1.0f;
                    filament::math::float3 d = cubemapFaceDir(face, u, v);
                    if (yRotationRad != 0.0f) {
                        d = {d.x * cRot + d.z * sRot, d.y, -d.x * sRot + d.z * cRot};
                    }
                    const filament::math::float3 rad = evalEnvRadiance(preset, d, blurFactor);
                    const size_t idx = (static_cast<size_t>(py) * sz + px) * 4;
                    dst[idx + 0] = filament::math::half(rad.x);
                    dst[idx + 1] = filament::math::half(rad.y);
                    dst[idx + 2] = filament::math::half(rad.z);
                    dst[idx + 3] = filament::math::half(1.0f);

                    if (lvl == 0 && outSh9) {
                        const float distSq = 1.0f + u * u + v * v;
                        const float dOmega = 4.0f / (std::sqrt(distSq) * distSq);
                        weightSum += dOmega;
                        const float basis[9] = {
                            0.282095f,
                            -0.488603f * d.y,
                            0.488603f * d.z,
                            -0.488603f * d.x,
                            1.092548f * d.x * d.y,
                            -1.092548f * d.y * d.z,
                            0.315392f * (3.0f * d.z * d.z - 1.0f),
                            -1.092548f * d.x * d.z,
                            0.546274f * (d.x * d.x - d.y * d.y),
                        };
                        for (int k = 0; k < 9; ++k) {
                            const float w = basis[k] * dOmega;
                            rawSh[k].x += rad.x * w;
                            rawSh[k].y += rad.y * w;
                            rawSh[k].z += rad.z * w;
                        }
                    }
                }
            }
            auto* faceCopy = new filament::math::half[faceFloats];
            std::memcpy(faceCopy, dst, faceFloats * sizeof(filament::math::half));
            tex->setImage(
                *engine, lvl, 0, 0, static_cast<uint32_t>(face), sz, sz, 1,
                filament::Texture::PixelBufferDescriptor(
                    faceCopy, faceFloats * sizeof(filament::math::half),
                    filament::Texture::Format::RGBA, filament::Texture::Type::HALF,
                    freeOwnedBuffer));
        }
        delete[] halfBuf;
    }

    if (outSh9 && weightSum > 1e-4f) {
        const float norm = (4.0f * 3.14159265f) / weightSum;
        for (int k = 0; k < 9; ++k) {
            const float bandFactor = (k == 0) ? 1.0f : ((k <= 3) ? (2.0f / 3.0f) : 0.25f);
            outSh9[k].x = rawSh[k].x * norm * bandFactor;
            outSh9[k].y = rawSh[k].y * norm * bandFactor;
            outSh9[k].z = rawSh[k].z * norm * bandFactor;
        }
    }
    return tex;
}

} // namespace

FilamentBackend::~FilamentBackend() {
    shutdown();
}

bool FilamentBackend::initialize(GraphicsBackendPreference preference) {
    if (engine_) {
        return true;
    }
    lastError_.clear();

    const auto createWithBackend = [](filament::Engine::Backend backend) -> filament::Engine* {
        filament::Engine::Config cfg{};
        cfg.minCommandBufferSizeMB = 2;
        cfg.commandBufferSizeMB = 8;
        return filament::Engine::Builder()
            .backend(backend)
            .config(&cfg)
            .build();
    };

    if (preference == GraphicsBackendPreference::OpenGLESOnly) {
        LOGI("Attempting Filament Engine creation with OpenGL ES backend...");
        engine_ = createWithBackend(filament::Engine::Backend::OPENGL);
    } else {
        LOGI("Attempting Filament Engine creation with Vulkan backend...");
        engine_ = createWithBackend(filament::Engine::Backend::VULKAN);
        if (!engine_) {
            LOGW("Vulkan Engine creation failed; falling back to OpenGL ES...");
            engine_ = createWithBackend(filament::Engine::Backend::OPENGL);
        }
    }

    if (!engine_) {
        lastError_ = "Unable to initialize Google Filament Engine (Vulkan and OpenGL ES failed).";
        LOGE("%s", lastError_.c_str());
        return false;
    }

    LOGI("Filament Engine initialized using backend: %s", activeBackendName().c_str());

    renderer_ = engine_->createRenderer();
    scene_ = engine_->createScene();
    view_ = engine_->createView();

    cameraEntity_ = utils::EntityManager::get().create();
    camera_ = engine_->createCamera(cameraEntity_);

    view_->setScene(scene_);
    view_->setCamera(camera_);
    view_->setPostProcessingEnabled(true);
    // Interactive view renders both scene objects (layer 0 = 0x01) and editor overlays (layer 1 = 0x02)
    view_->setVisibleLayers(0x03, 0x03);
    setViewportAdaptiveQuality(viewportAdaptiveQuality_);

    filament::Renderer::ClearOptions clearOpts;
    clearOpts.clear = true;
    clearOpts.discard = true;
    clearOpts.clearColor = {0.051f, 0.055f, 0.067f, 1.0f};
    renderer_->setClearOptions(clearOpts);

    if (!buildDefaultMaterials() || !buildDefaultTextures()) {
        shutdown();
        return false;
    }

    createStudioLighting();
    createGridAndAxes();
    createOverlayRenderable();
    createPreviewSphere();

    return true;
}

void FilamentBackend::shutdown() {
    endOffscreenSurface();
    detachSurface();

    if (!engine_) {
        return;
    }

    for (auto& kv : gpuNodes_) {
        destroyNodeResources(kv.second);
    }
    gpuNodes_.clear();

    for (auto& kv : gpuLights_) {
        destroyLightResources(kv.second);
    }
    gpuLights_.clear();

    destroyWireframeResources();
    destroyEditOverlayResources();

    auto& em = utils::EntityManager::get();

    if (!overlayEntity_.isNull()) {
        scene_->remove(overlayEntity_);
        engine_->destroy(overlayEntity_);
        em.destroy(overlayEntity_);
        overlayEntity_ = {};
    }
    if (overlayVb_) { engine_->destroy(overlayVb_); overlayVb_ = nullptr; }
    if (overlayIb_) { engine_->destroy(overlayIb_); overlayIb_ = nullptr; }
    if (overlayMi_) { engine_->destroy(overlayMi_); overlayMi_ = nullptr; }

    if (!gridEntity_.isNull()) {
        if (gridAttached_) {
            scene_->remove(gridEntity_);
            gridAttached_ = false;
        }
        engine_->destroy(gridEntity_);
        em.destroy(gridEntity_);
        gridEntity_ = {};
    }
    if (gridVb_) { engine_->destroy(gridVb_); gridVb_ = nullptr; }
    if (gridIb_) { engine_->destroy(gridIb_); gridIb_ = nullptr; }
    if (gridMi_) { engine_->destroy(gridMi_); gridMi_ = nullptr; }

    if (!axesEntity_.isNull()) {
        if (axesAttached_) {
            scene_->remove(axesEntity_);
            axesAttached_ = false;
        }
        engine_->destroy(axesEntity_);
        em.destroy(axesEntity_);
        axesEntity_ = {};
    }
    if (axesVb_) { engine_->destroy(axesVb_); axesVb_ = nullptr; }
    if (axesIb_) { engine_->destroy(axesIb_); axesIb_ = nullptr; }
    if (axesMi_) { engine_->destroy(axesMi_); axesMi_ = nullptr; }

    if (!sunLightEntity_.isNull()) {
        if (sunLightAttached_) {
            scene_->remove(sunLightEntity_);
            sunLightAttached_ = false;
        }
        engine_->destroy(sunLightEntity_);
        em.destroy(sunLightEntity_);
        sunLightEntity_ = {};
    }
    if (indirectLight_) {
        scene_->setIndirectLight(nullptr);
        engine_->destroy(indirectLight_);
        indirectLight_ = nullptr;
    }
    if (iblCubemap_) {
        engine_->destroy(iblCubemap_);
        iblCubemap_ = nullptr;
    }
    if (skybox_) {
        scene_->setSkybox(nullptr);
        engine_->destroy(skybox_);
        skybox_ = nullptr;
    }
    if (skyCubemap_) {
        engine_->destroy(skyCubemap_);
        skyCubemap_ = nullptr;
    }
    if (!previewSphereEntity_.isNull()) {
        if (previewSphereAttached_) {
            scene_->remove(previewSphereEntity_);
            previewSphereAttached_ = false;
        }
        engine_->destroy(previewSphereEntity_);
        em.destroy(previewSphereEntity_);
        previewSphereEntity_ = {};
    }
    if (previewSphereVb_) { engine_->destroy(previewSphereVb_); previewSphereVb_ = nullptr; }
    if (previewSphereIb_) { engine_->destroy(previewSphereIb_); previewSphereIb_ = nullptr; }
    if (previewSphereMi_) { engine_->destroy(previewSphereMi_); previewSphereMi_ = nullptr; }

    for (auto& kv : gpuTextures_) {
        if (kv.second.texture) {
            engine_->destroy(kv.second.texture);
        }
    }
    gpuTextures_.clear();
    if (defaultWhiteTex_) { engine_->destroy(defaultWhiteTex_); defaultWhiteTex_ = nullptr; }
    if (defaultFlatNormalTex_) { engine_->destroy(defaultFlatNormalTex_); defaultFlatNormalTex_ = nullptr; }

    if (pbrMaterial_) {
        engine_->destroy(pbrMaterial_);
        pbrMaterial_ = nullptr;
    }
    if (pbrTransparentMaterial_) {
        engine_->destroy(pbrTransparentMaterial_);
        pbrTransparentMaterial_ = nullptr;
    }
    if (unlitLineMaterial_) {
        engine_->destroy(unlitLineMaterial_);
        unlitLineMaterial_ = nullptr;
    }
    if (overlayLineMaterial_) {
        engine_->destroy(overlayLineMaterial_);
        overlayLineMaterial_ = nullptr;
    }
    if (translucentTriMaterial_) {
        engine_->destroy(translucentTriMaterial_);
        translucentTriMaterial_ = nullptr;
    }

    syncedIblPreset_ = 0xFFFFFFFFu;
    syncedSkyPreset_ = 0xFFFFFFFFu;
    syncedSkyRotation_ = -9999.0f;
    syncedEnvVersion_ = 0;
    syncedEnvShading_ = 0xFFFFFFFFu;

    if (offscreenView_) {
        offscreenView_->setCamera(nullptr);
        offscreenView_->setScene(nullptr);
        engine_->destroy(offscreenView_);
        offscreenView_ = nullptr;
    }
    if (offscreenColorGrading_) {
        engine_->destroy(offscreenColorGrading_);
        offscreenColorGrading_ = nullptr;
    }
    if (view_) {
        view_->setCamera(nullptr);
        view_->setScene(nullptr);
        engine_->destroy(view_);
        view_ = nullptr;
    }
    if (scene_) {
        engine_->destroy(scene_);
        scene_ = nullptr;
    }
    if (renderer_) {
        engine_->destroy(renderer_);
        renderer_ = nullptr;
    }
    if (camera_) {
        engine_->destroyCameraComponent(cameraEntity_);
        em.destroy(cameraEntity_);
        cameraEntity_ = {};
        camera_ = nullptr;
    }

    filament::Engine::destroy(&engine_);
    engine_ = nullptr;
    syncedSceneRevision_ = 0;
}

bool FilamentBackend::attachSurface(ANativeWindow* window, int32_t width, int32_t height) {
    if (!engine_) {
        lastError_ = "Cannot attach surface before Filament Engine is initialized.";
        return false;
    }
    if (!window) {
        lastError_ = "Null ANativeWindow passed to attachSurface.";
        return false;
    }

    detachSurface();

    ANativeWindow_acquire(window);
    window_ = window;
    swapChain_ = engine_->createSwapChain(window_);
    if (!swapChain_) {
        ANativeWindow_release(window_);
        window_ = nullptr;
        lastError_ = "Filament failed to create SwapChain for ANativeWindow.";
        LOGE("%s", lastError_.c_str());
        return false;
    }

    resizeSurface(width, height);
    return true;
}

void FilamentBackend::resizeSurface(int32_t width, int32_t height) {
    width_ = std::max<int32_t>(1, width);
    height_ = std::max<int32_t>(1, height);
    if (view_) {
        view_->setViewport({0, 0, static_cast<uint32_t>(width_), static_cast<uint32_t>(height_)});
    }
}

void FilamentBackend::detachSurface() {
    if (engine_ && swapChain_) {
        engine_->destroy(swapChain_);
        swapChain_ = nullptr;
    }
    if (window_) {
        ANativeWindow_release(window_);
        window_ = nullptr;
    }
}

bool FilamentBackend::buildDefaultMaterials() {
    filamat::MaterialBuilder::init();

    const auto targetApis =
        (engine_->getBackend() == filament::Engine::Backend::VULKAN)
            ? filamat::MaterialBuilder::TargetApi::VULKAN
            : filamat::MaterialBuilder::TargetApi::OPENGL;

    const char* pbrGlsl = R"(
        void material(inout MaterialInputs material) {
            vec2 uv = getUV0() * materialParams.uvTransform.xy + materialParams.uvTransform.zw;
            if (materialParams.extraFlags.y < 0.5 && materialParams.mapFlags.y > 0.5) {
                vec3 n = texture(materialParams_normalMap, uv).xyz * 2.0 - 1.0;
                material.normal = n;
            }
            prepareMaterial(material);

            if (materialParams.extraFlags.y > 0.5) {
                // Solid / Wireframe clay shading mode (VP-6)
                material.baseColor = vec4(materialParams.baseColor.rgb * 0.85 + 0.12, 1.0);
                material.metallic = 0.0;
                material.roughness = 0.68;
                material.ambientOcclusion = 1.0;
                material.emissive = vec4(0.0);
                return;
            }

            vec4 albedoTex = (materialParams.mapFlags.x > 0.5)
                ? texture(materialParams_albedoMap, uv)
                : vec4(1.0);
            vec4 baseCol = materialParams.baseColor * albedoTex;
            float alpha = clamp(baseCol.a, 0.0, 1.0);
            if (materialParams.extraFlags.z > 0.5) {
                // Transparent blending uses premultiplied alpha in Filament
                material.baseColor = vec4(baseCol.rgb * alpha, alpha);
            } else {
                material.baseColor = vec4(baseCol.rgb, 1.0);
            }

            material.metallic = materialParams.metallic;
            float roughSample = (materialParams.mapFlags.z > 0.5)
                ? texture(materialParams_roughnessMap, uv).g
                : 1.0;
            material.roughness = clamp(materialParams.roughness * roughSample, 0.04, 1.0);

            float aoSample = (materialParams.mapFlags.w > 0.5)
                ? texture(materialParams_aoMap, uv).r
                : 1.0;
            material.ambientOcclusion = mix(1.0, aoSample, materialParams.aoStrength);

            vec3 emSample = (materialParams.extraFlags.x > 0.5)
                ? texture(materialParams_emissiveMap, uv).rgb
                : vec3(1.0);
            material.emissive = vec4(materialParams.emissive.rgb * emSample, 0.0);
        }
    )";

    auto configurePbrBuilder = [&](const char* name, filament::BlendingMode blending) {
        return filamat::MaterialBuilder()
            .name(name)
            .platform(filamat::MaterialBuilder::Platform::MOBILE)
            .targetApi(targetApis)
            .optimization(filamat::MaterialBuilder::Optimization::NONE)
            .shading(filament::Shading::LIT)
            .blending(blending)
            .depthWrite(blending == filament::BlendingMode::OPAQUE)
            .require(filament::VertexAttribute::UV0)
            .parameter("baseColor", filamat::MaterialBuilder::UniformType::FLOAT4)
            .parameter("metallic", filamat::MaterialBuilder::UniformType::FLOAT)
            .parameter("roughness", filamat::MaterialBuilder::UniformType::FLOAT)
            .parameter("emissive", filamat::MaterialBuilder::UniformType::FLOAT4)
            .parameter("aoStrength", filamat::MaterialBuilder::UniformType::FLOAT)
            .parameter("uvTransform", filamat::MaterialBuilder::UniformType::FLOAT4)
            .parameter("mapFlags", filamat::MaterialBuilder::UniformType::FLOAT4)
            .parameter("extraFlags", filamat::MaterialBuilder::UniformType::FLOAT4)
            .parameter("albedoMap", filamat::MaterialBuilder::SamplerType::SAMPLER_2D,
                       filamat::MaterialBuilder::SamplerFormat::FLOAT)
            .parameter("normalMap", filamat::MaterialBuilder::SamplerType::SAMPLER_2D,
                       filamat::MaterialBuilder::SamplerFormat::FLOAT)
            .parameter("roughnessMap", filamat::MaterialBuilder::SamplerType::SAMPLER_2D,
                       filamat::MaterialBuilder::SamplerFormat::FLOAT)
            .parameter("aoMap", filamat::MaterialBuilder::SamplerType::SAMPLER_2D,
                       filamat::MaterialBuilder::SamplerFormat::FLOAT)
            .parameter("emissiveMap", filamat::MaterialBuilder::SamplerType::SAMPLER_2D,
                       filamat::MaterialBuilder::SamplerFormat::FLOAT)
            .material(pbrGlsl)
            .build(engine_->getJobSystem());
    };

    filamat::Package pbrPkg = configurePbrBuilder("NovaDefaultPBR", filament::BlendingMode::OPAQUE);
    filamat::Package pbrTransPkg = configurePbrBuilder("NovaTransparentPBR", filament::BlendingMode::TRANSPARENT);

    filamat::Package linePkg =
        filamat::MaterialBuilder()
            .name("NovaUnlitVertexColor")
            .platform(filamat::MaterialBuilder::Platform::MOBILE)
            .targetApi(targetApis)
            .optimization(filamat::MaterialBuilder::Optimization::NONE)
            .shading(filament::Shading::UNLIT)
            .require(filament::VertexAttribute::COLOR)
            .material(R"(
                void material(inout MaterialInputs material) {
                    prepareMaterial(material);
                    material.baseColor = getColor();
                }
            )")
            .build(engine_->getJobSystem());

    filamat::Package overlayLinePkg =
        filamat::MaterialBuilder()
            .name("NovaOverlayVertexColor")
            .platform(filamat::MaterialBuilder::Platform::MOBILE)
            .targetApi(targetApis)
            .optimization(filamat::MaterialBuilder::Optimization::NONE)
            .shading(filament::Shading::UNLIT)
            .depthWrite(false)
            .depthCulling(false)
            .require(filament::VertexAttribute::COLOR)
            .material(R"(
                void material(inout MaterialInputs material) {
                    prepareMaterial(material);
                    material.baseColor = getColor();
                }
            )")
            .build(engine_->getJobSystem());

    filamat::Package triOverlayPkg =
        filamat::MaterialBuilder()
            .name("NovaTranslucentOverlayTri")
            .platform(filamat::MaterialBuilder::Platform::MOBILE)
            .targetApi(targetApis)
            .optimization(filamat::MaterialBuilder::Optimization::NONE)
            .shading(filament::Shading::UNLIT)
            .blending(filament::BlendingMode::TRANSPARENT)
            .depthWrite(false)
            .depthCulling(true)
            .require(filament::VertexAttribute::COLOR)
            .material(R"(
                void material(inout MaterialInputs material) {
                    prepareMaterial(material);
                    material.baseColor = getColor();
                }
            )")
            .build(engine_->getJobSystem());

    filamat::MaterialBuilder::shutdown();

    if (!pbrPkg.isValid() || !pbrTransPkg.isValid() || !linePkg.isValid() ||
        !overlayLinePkg.isValid() || !triOverlayPkg.isValid()) {
        lastError_ = "Failed to compile default Filament materials via filamat.";
        LOGE("%s", lastError_.c_str());
        return false;
    }

    pbrMaterial_ = filament::Material::Builder()
                       .package(pbrPkg.getData(), pbrPkg.getSize())
                       .build(*engine_);

    pbrTransparentMaterial_ = filament::Material::Builder()
                                  .package(pbrTransPkg.getData(), pbrTransPkg.getSize())
                                  .build(*engine_);

    unlitLineMaterial_ = filament::Material::Builder()
                             .package(linePkg.getData(), linePkg.getSize())
                             .build(*engine_);

    overlayLineMaterial_ = filament::Material::Builder()
                               .package(overlayLinePkg.getData(), overlayLinePkg.getSize())
                               .build(*engine_);

    translucentTriMaterial_ = filament::Material::Builder()
                                  .package(triOverlayPkg.getData(), triOverlayPkg.getSize())
                                  .build(*engine_);

    if (!pbrMaterial_ || !pbrTransparentMaterial_ || !unlitLineMaterial_ ||
        !overlayLineMaterial_ || !translucentTriMaterial_) {
        lastError_ = "Failed to instantiate Filament Material objects.";
        LOGE("%s", lastError_.c_str());
        return false;
    }

    return true;
}

bool FilamentBackend::buildDefaultTextures() {
    auto create1x1 = [&](uint8_t r, uint8_t g, uint8_t b, uint8_t a) -> filament::Texture* {
        auto* tex = filament::Texture::Builder()
                        .width(1)
                        .height(1)
                        .levels(1)
                        .sampler(filament::Texture::Sampler::SAMPLER_2D)
                        .format(filament::Texture::InternalFormat::RGBA8)
                        .usage(filament::Texture::Usage::DEFAULT | filament::Texture::Usage::SAMPLEABLE)
                        .build(*engine_);
        if (tex) {
            auto* px = new uint8_t[4]{r, g, b, a};
            tex->setImage(*engine_, 0,
                          filament::Texture::PixelBufferDescriptor(
                              px, 4, filament::Texture::Format::RGBA,
                              filament::Texture::Type::UBYTE, freeOwnedBuffer));
        }
        return tex;
    };

    defaultWhiteTex_ = create1x1(255, 255, 255, 255);
    defaultFlatNormalTex_ = create1x1(128, 128, 255, 255);
    defaultSampler_ = filament::TextureSampler(
        filament::TextureSampler::MinFilter::LINEAR_MIPMAP_LINEAR,
        filament::TextureSampler::MagFilter::LINEAR,
        filament::TextureSampler::WrapMode::REPEAT);

    if (!defaultWhiteTex_ || !defaultFlatNormalTex_) {
        lastError_ = "Failed to create fallback Filament textures.";
        return false;
    }
    return true;
}

void FilamentBackend::createPreviewSphere() {
    const int rings = 28;
    const int sectors = 48;
    const float radius = 1.25f;
    std::vector<float> pos;
    std::vector<filament::math::float3> norms;
    std::vector<filament::math::float2> uvs;
    std::vector<uint32_t> indices;

    for (int r = 0; r <= rings; ++r) {
        const float v = static_cast<float>(r) / static_cast<float>(rings);
        const float theta = v * 3.14159265f;
        const float st = std::sin(theta);
        const float ct = std::cos(theta);
        for (int s = 0; s <= sectors; ++s) {
            const float u = static_cast<float>(s) / static_cast<float>(sectors);
            const float phi = u * 2.0f * 3.14159265f;
            const float sp = std::sin(phi);
            const float cp = std::cos(phi);
            const float nx = cp * st;
            const float ny = ct;
            const float nz = sp * st;
            pos.push_back(nx * radius);
            pos.push_back(ny * radius);
            pos.push_back(nz * radius);
            norms.push_back({nx, ny, nz});
            uvs.push_back({u * 2.0f, v});
        }
    }
    for (int r = 0; r < rings; ++r) {
        for (int s = 0; s < sectors; ++s) {
            const uint32_t i0 = static_cast<uint32_t>(r * (sectors + 1) + s);
            const uint32_t i1 = i0 + 1;
            const uint32_t i2 = i0 + (sectors + 1);
            const uint32_t i3 = i2 + 1;
            indices.push_back(i0);
            indices.push_back(i1);
            indices.push_back(i2);
            indices.push_back(i1);
            indices.push_back(i3);
            indices.push_back(i2);
        }
    }

    const size_t vCount = norms.size();
    auto* orientation =
        filament::geometry::SurfaceOrientation::Builder()
            .vertexCount(vCount)
            .normals(norms.data())
            .uvs(uvs.data())
            .positions(reinterpret_cast<const filament::math::float3*>(pos.data()))
            .triangleCount(indices.size() / 3)
            .triangles(reinterpret_cast<const filament::math::uint3*>(indices.data()))
            .build();
    std::vector<filament::math::short4> tangents(vCount, {0, 0, 0, 32767});
    if (orientation) {
        orientation->getQuats(tangents.data(), vCount);
        delete orientation;
    }

    previewSphereVb_ =
        filament::VertexBuffer::Builder()
            .vertexCount(static_cast<uint32_t>(vCount))
            .bufferCount(3)
            .attribute(filament::VertexAttribute::POSITION, 0,
                       filament::VertexBuffer::AttributeType::FLOAT3, 0, sizeof(float) * 3)
            .attribute(filament::VertexAttribute::TANGENTS, 1,
                       filament::VertexBuffer::AttributeType::SHORT4, 0, sizeof(filament::math::short4))
            .normalized(filament::VertexAttribute::TANGENTS)
            .attribute(filament::VertexAttribute::UV0, 2,
                       filament::VertexBuffer::AttributeType::FLOAT2, 0, sizeof(float) * 2)
            .build(*engine_);
    previewSphereVb_->setBufferAt(*engine_, 0, makeOwnedDescriptor(pos.data(), pos.size()));
    previewSphereVb_->setBufferAt(*engine_, 1, makeOwnedDescriptor(tangents.data(), tangents.size()));
    previewSphereVb_->setBufferAt(*engine_, 2, makeOwnedDescriptor(uvs.data(), uvs.size()));

    previewSphereIb_ =
        filament::IndexBuffer::Builder()
            .indexCount(static_cast<uint32_t>(indices.size()))
            .bufferType(filament::IndexBuffer::IndexType::UINT)
            .build(*engine_);
    previewSphereIb_->setBuffer(*engine_, makeOwnedDescriptor(indices.data(), indices.size()));

    previewSphereMi_ = pbrMaterial_->createInstance();
    previewSphereTransparent_ = false;
    previewSphereEntity_ = utils::EntityManager::get().create();
    filament::Box bbox;
    bbox.set({-radius, -radius, -radius}, {radius, radius, radius});
    filament::RenderableManager::Builder(1)
        .boundingBox(bbox)
        .layerMask(0xFF, 0x02)
        .material(0, previewSphereMi_)
        .geometry(0, filament::RenderableManager::PrimitiveType::TRIANGLES,
                  previewSphereVb_, previewSphereIb_)
        .castShadows(true)
        .receiveShadows(true)
        .build(*engine_, previewSphereEntity_);
    previewSphereAttached_ = false;
}

filament::Texture* FilamentBackend::resolveGpuTexture(
    const NovaCoreHandle* core, uint64_t texId, bool isSrgb) {
    if (texId == 0 || !core || !engine_) {
        return nullptr;
    }
    NovaTextureMeta meta{};
    if (nova_core_texture_get_meta(core, texId, &meta) != 0) {
        return nullptr;
    }
    const uint64_t cacheKey = (texId << 1) | (isSrgb ? 1ULL : 0ULL);
    auto it = gpuTextures_.find(cacheKey);
    if (it != gpuTextures_.end() && it->second.version == meta.version && it->second.texture) {
        return it->second.texture;
    }

    size_t reqBytes = 0;
    if (nova_core_texture_copy_rgba(core, texId, nullptr, 0, &reqBytes) != 0 || reqBytes == 0) {
        return nullptr;
    }
    auto* pixels = new uint8_t[reqBytes];
    if (nova_core_texture_copy_rgba(core, texId, pixels, reqBytes, &reqBytes) != 0) {
        delete[] pixels;
        return nullptr;
    }

    const uint32_t maxDim = std::max(1u, std::max(meta.width, meta.height));
    uint8_t levels = 1;
    uint32_t d = maxDim;
    while (d > 1 && levels < 10) {
        d >>= 1;
        ++levels;
    }

    if (it != gpuTextures_.end() && it->second.texture) {
        engine_->destroy(it->second.texture);
        it->second.texture = nullptr;
    }

    const auto fmt = isSrgb ? filament::Texture::InternalFormat::SRGB8_A8
                            : filament::Texture::InternalFormat::RGBA8;
    auto* tex = filament::Texture::Builder()
                    .width(meta.width)
                    .height(meta.height)
                    .levels(levels)
                    .sampler(filament::Texture::Sampler::SAMPLER_2D)
                    .format(fmt)
                    .usage(filament::Texture::Usage::DEFAULT |
                           filament::Texture::Usage::SAMPLEABLE |
                           filament::Texture::Usage::GEN_MIPMAPPABLE)
                    .build(*engine_);
    if (!tex) {
        delete[] pixels;
        return nullptr;
    }

    tex->setImage(
        *engine_, 0,
        filament::Texture::PixelBufferDescriptor(
            pixels, reqBytes, filament::Texture::Format::RGBA,
            filament::Texture::Type::UBYTE, freeOwnedBuffer));
    tex->generateMipmaps(*engine_);

    gpuTextures_[cacheKey] = GpuTextureEntry{tex, meta.version};
    return tex;
}

void FilamentBackend::applyMaterialToInstance(
    const NovaCoreHandle* core,
    filament::MaterialInstance* mi,
    uint32_t shadingMode,
    const float baseColor[4],
    float metallic,
    float roughness,
    const float emissive[3],
    float emissiveIntensity,
    float aoStrength,
    bool isSelected,
    const float uvScale[2],
    const float uvOffset[2],
    uint64_t albedoTexId,
    uint64_t normalTexId,
    uint64_t roughnessTexId,
    uint64_t aoTexId,
    uint64_t emissiveTexId) {
    if (!mi) return;
    const auto srgbToLinear = [](float c) -> float {
        const float v = std::max(0.0f, c);
        return (v <= 0.04045f) ? (v / 12.92f) : std::pow((v + 0.055f) / 1.055f, 2.4f);
    };
    filament::math::float4 color{
        srgbToLinear(baseColor[0]),
        srgbToLinear(baseColor[1]),
        srgbToLinear(baseColor[2]),
        baseColor[3]};
    if (isSelected) {
        color.x = std::min(1.0f, color.x * 1.12f + 0.025f);
        color.y = std::min(1.0f, color.y * 1.12f + 0.025f);
        color.z = std::min(1.0f, color.z * 1.12f + 0.025f);
    }
    const bool isClay = (shadingMode <= 1);
    const bool isTransparent = (baseColor[3] < 0.995f);

    auto* albedoTex = (!isClay && albedoTexId > 0) ? resolveGpuTexture(core, albedoTexId, true) : nullptr;
    auto* normalTex = (!isClay && normalTexId > 0) ? resolveGpuTexture(core, normalTexId, false) : nullptr;
    auto* roughTex = (!isClay && roughnessTexId > 0) ? resolveGpuTexture(core, roughnessTexId, false) : nullptr;
    auto* aoTex = (!isClay && aoTexId > 0) ? resolveGpuTexture(core, aoTexId, false) : nullptr;
    auto* emissiveTex = (!isClay && emissiveTexId > 0) ? resolveGpuTexture(core, emissiveTexId, true) : nullptr;

    mi->setParameter("baseColor", color);
    mi->setParameter("metallic", metallic);
    mi->setParameter("roughness", roughness);
    mi->setParameter("emissive", filament::math::float4{
        srgbToLinear(emissive[0]) * emissiveIntensity,
        srgbToLinear(emissive[1]) * emissiveIntensity,
        srgbToLinear(emissive[2]) * emissiveIntensity,
        0.0f});
    mi->setParameter("aoStrength", aoStrength);
    mi->setParameter("uvTransform", filament::math::float4{
        uvScale[0], uvScale[1], uvOffset[0], uvOffset[1]});
    mi->setParameter("mapFlags", filament::math::float4{
        albedoTex ? 1.0f : 0.0f,
        normalTex ? 1.0f : 0.0f,
        roughTex ? 1.0f : 0.0f,
        aoTex ? 1.0f : 0.0f});
    mi->setParameter("extraFlags", filament::math::float4{
        emissiveTex ? 1.0f : 0.0f,
        isClay ? 1.0f : 0.0f,
        isTransparent ? 1.0f : 0.0f,
        0.0f});

    mi->setParameter("albedoMap", albedoTex ? albedoTex : defaultWhiteTex_, defaultSampler_);
    mi->setParameter("normalMap", normalTex ? normalTex : defaultFlatNormalTex_, defaultSampler_);
    mi->setParameter("roughnessMap", roughTex ? roughTex : defaultWhiteTex_, defaultSampler_);
    mi->setParameter("aoMap", aoTex ? aoTex : defaultWhiteTex_, defaultSampler_);
    mi->setParameter("emissiveMap", emissiveTex ? emissiveTex : defaultWhiteTex_, defaultSampler_);
}

void FilamentBackend::syncEnvironmentFromCore(const NovaCoreHandle* core, uint32_t shadingMode) {
    if (!engine_ || !scene_ || !view_ || !core) return;

    NovaEnvironmentInfo env{};
    if (nova_core_env_get(core, &env) != 0) {
        return;
    }

    // In MaterialPreview (2) / Solid (0) / Wireframe (1), keep dark crisp slate background unless
    // in RenderPreview (3) where user's HDR skybox setting takes over, OR if user explicitly turned Show Skybox on!
    const uint32_t activePreset = (shadingMode == 0 || shadingMode == 1) ? 5u : env.preset;
    const float iblIntensity =
        (shadingMode == 0 || shadingMode == 1) ? 18000.0f : (20000.0f * std::max(0.05f, env.intensity));

    if (!indirectLight_ || syncedIblPreset_ != activePreset) {
        filament::math::float3 sh9[9]{};
        filament::Texture* newIbl = createProceduralEnvCubemap(engine_, activePreset, 32, 6, 0.0f, sh9);
        if (newIbl) {
            auto* nextInd = filament::IndirectLight::Builder()
                                .reflections(newIbl)
                                .irradiance(3, sh9)
                                .intensity(iblIntensity)
                                .build(*engine_);
            if (nextInd) {
                scene_->setIndirectLight(nextInd);
                if (indirectLight_) engine_->destroy(indirectLight_);
                if (iblCubemap_) engine_->destroy(iblCubemap_);
                indirectLight_ = nextInd;
                iblCubemap_ = newIbl;
                syncedIblPreset_ = activePreset;
            } else {
                engine_->destroy(newIbl);
            }
        }
    }

    const float rotRad = env.rotation_deg * (3.14159265f / 180.0f);
    if (indirectLight_) {
        indirectLight_->setIntensity(iblIntensity);
        indirectLight_->setRotation(filament::math::mat3f::rotation(rotRad, filament::math::float3{0, 1, 0}));
    }

    const bool wantSkybox = env.show_skybox && shadingMode >= 2;
    if (wantSkybox) {
        if (!skybox_ || syncedSkyPreset_ != activePreset ||
            std::fabs(syncedSkyRotation_ - env.rotation_deg) > 0.25f) {
            filament::Texture* nextSkyTex = createProceduralEnvCubemap(
                engine_, activePreset, 48, 1, -rotRad, nullptr);
            if (nextSkyTex) {
                auto* nextSky = filament::Skybox::Builder()
                                    .environment(nextSkyTex)
                                    .intensity(28000.0f * std::max(0.2f, env.intensity))
                                    .build(*engine_);
                if (nextSky) {
                    scene_->setSkybox(nextSky);
                    if (skybox_) engine_->destroy(skybox_);
                    if (skyCubemap_) engine_->destroy(skyCubemap_);
                    skybox_ = nextSky;
                    skyCubemap_ = nextSkyTex;
                    syncedSkyPreset_ = activePreset;
                    syncedSkyRotation_ = env.rotation_deg;
                } else {
                    engine_->destroy(nextSkyTex);
                }
            }
        }
    } else if (skybox_) {
        scene_->setSkybox(nullptr);
        engine_->destroy(skybox_);
        skybox_ = nullptr;
        if (skyCubemap_) {
            engine_->destroy(skyCubemap_);
            skyCubemap_ = nullptr;
        }
        syncedSkyPreset_ = 0xFFFFFFFFu;
    }

    if (camera_) {
        camera_->setExposure(16.0f, 1.0f / 125.0f, 100.0f * std::max(0.1f, env.exposure));
    }

    filament::BloomOptions bloom{};
    bloom.enabled = env.bloom_enabled && shadingMode >= 2;
    bloom.strength = std::max(0.0f, std::min(1.0f, env.bloom_strength));
    view_->setBloomOptions(bloom);

    syncedEnvVersion_ = env.version;
    syncedEnvShading_ = shadingMode;
}

void FilamentBackend::createStudioLighting() {
    sunLightEntity_ = utils::EntityManager::get().create();
    filament::LightManager::Builder(filament::LightManager::Type::DIRECTIONAL)
        .color({1.0f, 0.96f, 0.90f})
        .intensity(62000.0f)
        .direction({-0.55f, -0.82f, -0.45f})
        .castShadows(true)
        .build(*engine_, sunLightEntity_);
    scene_->addEntity(sunLightEntity_);
    sunLightAttached_ = true;

    const filament::math::float3 shBands[9] = {
        {0.58f, 0.62f, 0.68f},
        {0.18f, 0.20f, 0.24f},
        {0.00f, 0.00f, 0.00f},
        {-0.08f, -0.07f, -0.06f},
        {0.00f, 0.00f, 0.00f},
        {0.02f, 0.02f, 0.03f},
        {0.05f, 0.05f, 0.06f},
        {0.00f, 0.00f, 0.00f},
        {0.03f, 0.03f, 0.04f},
    };
    indirectLight_ = filament::IndirectLight::Builder()
                         .irradiance(3, shBands)
                         .intensity(20000.0f)
                         .build(*engine_);
    scene_->setIndirectLight(indirectLight_);
}

void FilamentBackend::createGridAndAxes() {
    std::vector<LineVertex> gridVerts;
    std::vector<uint32_t> gridIndices;

    // 1. Minor fine grid lines (step 1.0, range -18 to +18, fading smoothly)
    const int fineExt = 18;
    for (int i = -fineExt; i <= fineExt; ++i) {
        if (i == 0 || i % 5 == 0) continue;
        const float f = static_cast<float>(i);
        const float distRatio = std::fabs(f) / static_cast<float>(fineExt);
        const float alphaFade = (1.0f - distRatio * 0.8f);
        const uint8_t rVal = static_cast<uint8_t>(40 * alphaFade);
        const uint8_t gVal = static_cast<uint8_t>(45 * alphaFade);
        const uint8_t bVal = static_cast<uint8_t>(58 * alphaFade);
        const uint32_t col = rgba(rVal, gVal, bVal);

        // X lines
        const uint32_t b0 = static_cast<uint32_t>(gridVerts.size());
        gridVerts.push_back({-static_cast<float>(fineExt), 0.0f, f, rgba(16, 18, 24)});
        gridVerts.push_back({0.0f, 0.0f, f, col});
        gridVerts.push_back({static_cast<float>(fineExt), 0.0f, f, rgba(16, 18, 24)});
        gridIndices.push_back(b0); gridIndices.push_back(b0 + 1);
        gridIndices.push_back(b0 + 1); gridIndices.push_back(b0 + 2);

        // Z lines
        const uint32_t b1 = static_cast<uint32_t>(gridVerts.size());
        gridVerts.push_back({f, 0.0f, -static_cast<float>(fineExt), rgba(16, 18, 24)});
        gridVerts.push_back({f, 0.0f, 0.0f, col});
        gridVerts.push_back({f, 0.0f, static_cast<float>(fineExt), rgba(16, 18, 24)});
        gridIndices.push_back(b1); gridIndices.push_back(b1 + 1);
        gridIndices.push_back(b1 + 1); gridIndices.push_back(b1 + 2);
    }

    // 2. Major infinite grid lines (step 5.0, range -60 to +60, multi-segment horizon fade)
    const int majExt = 60;
    for (int i = -majExt; i <= majExt; i += 5) {
        if (i == 0) continue;
        const float f = static_cast<float>(i);
        const float distRatio = std::fabs(f) / static_cast<float>(majExt);
        const float alphaFade = (1.0f - distRatio * 0.85f);
        const uint8_t rVal = static_cast<uint8_t>(72 * alphaFade);
        const uint8_t gVal = static_cast<uint8_t>(82 * alphaFade);
        const uint8_t bVal = static_cast<uint8_t>(106 * alphaFade);
        const uint32_t midCol = rgba(rVal, gVal, bVal);
        const uint32_t farCol = rgba(22, 25, 34);

        // X lines
        const uint32_t b0 = static_cast<uint32_t>(gridVerts.size());
        gridVerts.push_back({-static_cast<float>(majExt), 0.0f, f, rgba(14, 15, 20)});
        gridVerts.push_back({-25.0f, 0.0f, f, farCol});
        gridVerts.push_back({0.0f, 0.0f, f, midCol});
        gridVerts.push_back({25.0f, 0.0f, f, farCol});
        gridVerts.push_back({static_cast<float>(majExt), 0.0f, f, rgba(14, 15, 20)});
        for (uint32_t k = 0; k < 4; ++k) {
            gridIndices.push_back(b0 + k);
            gridIndices.push_back(b0 + k + 1);
        }

        // Z lines
        const uint32_t b1 = static_cast<uint32_t>(gridVerts.size());
        gridVerts.push_back({f, 0.0f, -static_cast<float>(majExt), rgba(14, 15, 20)});
        gridVerts.push_back({f, 0.0f, -25.0f, farCol});
        gridVerts.push_back({f, 0.0f, 0.0f, midCol});
        gridVerts.push_back({f, 0.0f, 25.0f, farCol});
        gridVerts.push_back({f, 0.0f, static_cast<float>(majExt), rgba(14, 15, 20)});
        for (uint32_t k = 0; k < 4; ++k) {
            gridIndices.push_back(b1 + k);
            gridIndices.push_back(b1 + k + 1);
        }
    }

    gridVb_ = filament::VertexBuffer::Builder()
                  .vertexCount(static_cast<uint32_t>(gridVerts.size()))
                  .bufferCount(1)
                  .attribute(filament::VertexAttribute::POSITION, 0,
                             filament::VertexBuffer::AttributeType::FLOAT3, 0, sizeof(LineVertex))
                  .attribute(filament::VertexAttribute::COLOR, 0,
                             filament::VertexBuffer::AttributeType::UBYTE4,
                             offsetof(LineVertex, colorAbgr), sizeof(LineVertex))
                  .normalized(filament::VertexAttribute::COLOR)
                  .build(*engine_);
    gridVb_->setBufferAt(*engine_, 0, makeOwnedDescriptor(gridVerts.data(), gridVerts.size()));

    gridIb_ = filament::IndexBuffer::Builder()
                  .indexCount(static_cast<uint32_t>(gridIndices.size()))
                  .bufferType(filament::IndexBuffer::IndexType::UINT)
                  .build(*engine_);
    gridIb_->setBuffer(*engine_, makeOwnedDescriptor(gridIndices.data(), gridIndices.size()));

    gridMi_ = unlitLineMaterial_->createInstance();
    gridEntity_ = utils::EntityManager::get().create();
    filament::RenderableManager::Builder(1)
        .boundingBox({{-70.0f, -0.02f, -70.0f}, {70.0f, 0.02f, 70.0f}})
        .layerMask(0xFF, 0x02)
        .material(0, gridMi_)
        .geometry(0, filament::RenderableManager::PrimitiveType::LINES, gridVb_, gridIb_)
        .culling(false)
        .castShadows(false)
        .receiveShadows(false)
        .build(*engine_, gridEntity_);
    scene_->addEntity(gridEntity_);
    gridAttached_ = true;

    // Ground Coordinate Floor Axes (Infinite style, blending into horizon)
    std::vector<LineVertex> axesVerts;
    std::vector<uint32_t> axesIndices;
    const float axExt = 80.0f;

    auto addAxisSeg = [&](float x0, float y0, float z0, uint32_t c0,
                          float x1, float y1, float z1, uint32_t c1) {
        const uint32_t b = static_cast<uint32_t>(axesVerts.size());
        axesVerts.push_back({x0, y0, z0, c0});
        axesVerts.push_back({x1, y1, z1, c1});
        axesIndices.push_back(b);
        axesIndices.push_back(b + 1);
    };

    // X Axis (Red Floor Line)
    addAxisSeg(0.0f, 0.001f, 0.0f, rgba(255, 68, 88), 30.0f, 0.001f, 0.0f, rgba(230, 55, 75));
    addAxisSeg(30.0f, 0.001f, 0.0f, rgba(230, 55, 75), axExt, 0.001f, 0.0f, rgba(65, 20, 28));
    addAxisSeg(0.0f, 0.001f, 0.0f, rgba(135, 42, 54), -30.0f, 0.001f, 0.0f, rgba(80, 26, 34));
    addAxisSeg(-30.0f, 0.001f, 0.0f, rgba(80, 26, 34), -axExt, 0.001f, 0.0f, rgba(22, 12, 16));

    // Z Axis (Blue Floor Line)
    addAxisSeg(0.0f, 0.001f, 0.0f, rgba(48, 168, 255), 0.0f, 0.001f, 30.0f, rgba(40, 135, 225));
    addAxisSeg(0.0f, 0.001f, 30.0f, rgba(40, 135, 225), 0.0f, 0.001f, axExt, rgba(22, 55, 110));
    addAxisSeg(0.0f, 0.001f, 0.0f, rgba(38, 85, 155), 0.0f, 0.001f, -30.0f, rgba(26, 55, 105));
    addAxisSeg(0.0f, 0.001f, -30.0f, rgba(26, 55, 105), 0.0f, 0.001f, -axExt, rgba(14, 20, 42));

    // Y Axis (Height indicator at origin)
    addAxisSeg(0.0f, 0.0f, 0.0f, rgba(46, 224, 110), 0.0f, 4.0f, 0.0f, rgba(24, 135, 58));

    axesVb_ = filament::VertexBuffer::Builder()
                  .vertexCount(static_cast<uint32_t>(axesVerts.size()))
                  .bufferCount(1)
                  .attribute(filament::VertexAttribute::POSITION, 0,
                             filament::VertexBuffer::AttributeType::FLOAT3, 0, sizeof(LineVertex))
                  .attribute(filament::VertexAttribute::COLOR, 0,
                             filament::VertexBuffer::AttributeType::UBYTE4,
                             offsetof(LineVertex, colorAbgr), sizeof(LineVertex))
                  .normalized(filament::VertexAttribute::COLOR)
                  .build(*engine_);
    axesVb_->setBufferAt(*engine_, 0, makeOwnedDescriptor(axesVerts.data(), axesVerts.size()));

    axesIb_ = filament::IndexBuffer::Builder()
                  .indexCount(static_cast<uint32_t>(axesIndices.size()))
                  .bufferType(filament::IndexBuffer::IndexType::UINT)
                  .build(*engine_);
    axesIb_->setBuffer(*engine_, makeOwnedDescriptor(axesIndices.data(), axesIndices.size()));

    axesMi_ = unlitLineMaterial_->createInstance();
    axesEntity_ = utils::EntityManager::get().create();
    filament::RenderableManager::Builder(1)
        .boundingBox({{-90.0f, -0.02f, -90.0f}, {90.0f, 5.0f, 90.0f}})
        .layerMask(0xFF, 0x02)
        .material(0, axesMi_)
        .geometry(0, filament::RenderableManager::PrimitiveType::LINES, axesVb_, axesIb_)
        .culling(false)
        .castShadows(false)
        .receiveShadows(false)
        .build(*engine_, axesEntity_);
    scene_->addEntity(axesEntity_);
    axesAttached_ = true;
}

void FilamentBackend::createOverlayRenderable() {
    std::vector<LineVertex> initVerts(kMaxOverlayVerts, {0.0f, 0.0f, 0.0f, 0});
    std::vector<uint32_t> initIndices(kMaxOverlayIndices, 0);

    overlayVb_ = filament::VertexBuffer::Builder()
                     .vertexCount(static_cast<uint32_t>(kMaxOverlayVerts))
                     .bufferCount(1)
                     .attribute(filament::VertexAttribute::POSITION, 0,
                                filament::VertexBuffer::AttributeType::FLOAT3, 0, sizeof(LineVertex))
                     .attribute(filament::VertexAttribute::COLOR, 0,
                                filament::VertexBuffer::AttributeType::UBYTE4,
                                offsetof(LineVertex, colorAbgr), sizeof(LineVertex))
                     .normalized(filament::VertexAttribute::COLOR)
                     .build(*engine_);
    overlayVb_->setBufferAt(*engine_, 0, makeOwnedDescriptor(initVerts.data(), initVerts.size()));

    overlayIb_ = filament::IndexBuffer::Builder()
                     .indexCount(static_cast<uint32_t>(kMaxOverlayIndices))
                     .bufferType(filament::IndexBuffer::IndexType::UINT)
                     .build(*engine_);
    overlayIb_->setBuffer(*engine_, makeOwnedDescriptor(initIndices.data(), initIndices.size()));

    overlayMi_ = overlayLineMaterial_->createInstance();
    overlayEntity_ = utils::EntityManager::get().create();
    filament::RenderableManager::Builder(1)
        .boundingBox({{-100.0f, -100.0f, -100.0f}, {100.0f, 100.0f, 100.0f}})
        .layerMask(0xFF, 0x02)
        .material(0, overlayMi_)
        .geometry(0, filament::RenderableManager::PrimitiveType::LINES, overlayVb_, overlayIb_, 0, 0)
        .culling(false)
        .priority(7)
        .channel(4)
        .castShadows(false)
        .receiveShadows(false)
        .build(*engine_, overlayEntity_);

    auto& rcm = engine_->getRenderableManager();
    rcm.setInstanceCount(rcm.getInstance(overlayEntity_), 0);
    scene_->addEntity(overlayEntity_);
}

void FilamentBackend::destroyNodeResources(GpuMeshNode& node) {
    if (!engine_) return;
    if (!node.entity.isNull()) {
        if (node.inScene) {
            scene_->remove(node.entity);
            node.inScene = false;
        }
        engine_->destroy(node.entity);
        utils::EntityManager::get().destroy(node.entity);
        node.entity = {};
    }
    if (node.vertexBuffer) {
        engine_->destroy(node.vertexBuffer);
        node.vertexBuffer = nullptr;
    }
    if (node.indexBuffer) {
        engine_->destroy(node.indexBuffer);
        node.indexBuffer = nullptr;
    }
    if (node.materialInstance) {
        engine_->destroy(node.materialInstance);
        node.materialInstance = nullptr;
    }
}

void FilamentBackend::destroyLightResources(GpuLightNode& light) {
    if (!engine_) return;
    if (!light.entity.isNull()) {
        if (light.inScene) {
            scene_->remove(light.entity);
            light.inScene = false;
        }
        engine_->destroy(light.entity);
        utils::EntityManager::get().destroy(light.entity);
        light.entity = {};
    }
}

void FilamentBackend::destroyWireframeResources() {
    if (!engine_) return;
    if (!wireframeEntity_.isNull()) {
        scene_->remove(wireframeEntity_);
        engine_->destroy(wireframeEntity_);
        utils::EntityManager::get().destroy(wireframeEntity_);
        wireframeEntity_ = {};
    }
    if (wireframeVb_) { engine_->destroy(wireframeVb_); wireframeVb_ = nullptr; }
    if (wireframeIb_) { engine_->destroy(wireframeIb_); wireframeIb_ = nullptr; }
    if (wireframeMi_) { engine_->destroy(wireframeMi_); wireframeMi_ = nullptr; }
}

void FilamentBackend::destroyEditOverlayResources() {
    if (!engine_) return;
    if (!editLineEntity_.isNull()) {
        scene_->remove(editLineEntity_);
        engine_->destroy(editLineEntity_);
        utils::EntityManager::get().destroy(editLineEntity_);
        editLineEntity_ = {};
    }
    if (editLineVb_) { engine_->destroy(editLineVb_); editLineVb_ = nullptr; }
    if (editLineIb_) { engine_->destroy(editLineIb_); editLineIb_ = nullptr; }
    if (editLineMi_) { engine_->destroy(editLineMi_); editLineMi_ = nullptr; }

    if (!editTriEntity_.isNull()) {
        scene_->remove(editTriEntity_);
        engine_->destroy(editTriEntity_);
        utils::EntityManager::get().destroy(editTriEntity_);
        editTriEntity_ = {};
    }
    if (editTriVb_) { engine_->destroy(editTriVb_); editTriVb_ = nullptr; }
    if (editTriIb_) { engine_->destroy(editTriIb_); editTriIb_ = nullptr; }
    if (editTriMi_) { engine_->destroy(editTriMi_); editTriMi_ = nullptr; }
}

void FilamentBackend::rebuildEditOverlay(const NovaCoreHandle* core) {
    destroyEditOverlayResources();
    if (nova_core_material_preview_sphere_get(core)) {
        return;
    }
    NovaEditOverlayView view{};
    if (nova_core_edit_get_overlay(core, &view) != 0) {
        return;
    }

    auto toCol = [](const float c[4]) -> uint32_t {
        auto clampU8 = [](float v) -> uint8_t {
            const float cl = std::max(0.0f, std::min(1.0f, v));
            return static_cast<uint8_t>(cl * 255.0f);
        };
        return rgba(clampU8(c[0]), clampU8(c[1]), clampU8(c[2]), clampU8(c[3]));
    };

    if (view.line_count > 0 && view.lines_ptr != nullptr && unlitLineMaterial_) {
        std::vector<LineVertex> verts;
        std::vector<uint32_t> indices;
        verts.reserve(view.line_count * 2);
        indices.reserve(view.line_count * 2);
        for (size_t i = 0; i < view.line_count; ++i) {
            const auto& seg = view.lines_ptr[i];
            const uint32_t col = toCol(seg.color);
            const uint32_t base = static_cast<uint32_t>(verts.size());
            verts.push_back({seg.a[0], seg.a[1], seg.a[2], col});
            verts.push_back({seg.b[0], seg.b[1], seg.b[2], col});
            indices.push_back(base);
            indices.push_back(base + 1);
        }
        editLineVb_ = filament::VertexBuffer::Builder()
                          .vertexCount(static_cast<uint32_t>(verts.size()))
                          .bufferCount(1)
                          .attribute(filament::VertexAttribute::POSITION, 0,
                                     filament::VertexBuffer::AttributeType::FLOAT3, 0, sizeof(LineVertex))
                          .attribute(filament::VertexAttribute::COLOR, 0,
                                     filament::VertexBuffer::AttributeType::UBYTE4,
                                     offsetof(LineVertex, colorAbgr), sizeof(LineVertex))
                          .normalized(filament::VertexAttribute::COLOR)
                          .build(*engine_);
        editLineVb_->setBufferAt(*engine_, 0, makeOwnedDescriptor(verts.data(), verts.size()));
        editLineIb_ = filament::IndexBuffer::Builder()
                          .indexCount(static_cast<uint32_t>(indices.size()))
                          .bufferType(filament::IndexBuffer::IndexType::UINT)
                          .build(*engine_);
        editLineIb_->setBuffer(*engine_, makeOwnedDescriptor(indices.data(), indices.size()));
        editLineMi_ = unlitLineMaterial_->createInstance();
        editLineEntity_ = utils::EntityManager::get().create();
        filament::RenderableManager::Builder(1)
            .boundingBox({{-100.0f, -100.0f, -100.0f}, {100.0f, 100.0f, 100.0f}})
            .layerMask(0xFF, 0x02)
            .material(0, editLineMi_)
            .geometry(0, filament::RenderableManager::PrimitiveType::LINES, editLineVb_, editLineIb_)
            .culling(false)
            .castShadows(false)
            .receiveShadows(false)
            .build(*engine_, editLineEntity_);
        scene_->addEntity(editLineEntity_);
    }

    if (view.tri_count > 0 && view.tris_ptr != nullptr && translucentTriMaterial_) {
        std::vector<LineVertex> verts;
        std::vector<uint32_t> indices;
        verts.reserve(view.tri_count * 3);
        indices.reserve(view.tri_count * 3);
        for (size_t i = 0; i < view.tri_count; ++i) {
            const auto& tri = view.tris_ptr[i];
            const uint32_t col = toCol(tri.color);
            const uint32_t base = static_cast<uint32_t>(verts.size());
            verts.push_back({tri.v0[0], tri.v0[1], tri.v0[2], col});
            verts.push_back({tri.v1[0], tri.v1[1], tri.v1[2], col});
            verts.push_back({tri.v2[0], tri.v2[1], tri.v2[2], col});
            indices.push_back(base);
            indices.push_back(base + 1);
            indices.push_back(base + 2);
        }
        editTriVb_ = filament::VertexBuffer::Builder()
                         .vertexCount(static_cast<uint32_t>(verts.size()))
                         .bufferCount(1)
                         .attribute(filament::VertexAttribute::POSITION, 0,
                                    filament::VertexBuffer::AttributeType::FLOAT3, 0, sizeof(LineVertex))
                         .attribute(filament::VertexAttribute::COLOR, 0,
                                    filament::VertexBuffer::AttributeType::UBYTE4,
                                    offsetof(LineVertex, colorAbgr), sizeof(LineVertex))
                         .normalized(filament::VertexAttribute::COLOR)
                         .build(*engine_);
        editTriVb_->setBufferAt(*engine_, 0, makeOwnedDescriptor(verts.data(), verts.size()));
        editTriIb_ = filament::IndexBuffer::Builder()
                         .indexCount(static_cast<uint32_t>(indices.size()))
                         .bufferType(filament::IndexBuffer::IndexType::UINT)
                         .build(*engine_);
        editTriIb_->setBuffer(*engine_, makeOwnedDescriptor(indices.data(), indices.size()));
        editTriMi_ = translucentTriMaterial_->createInstance();
        editTriEntity_ = utils::EntityManager::get().create();
        filament::RenderableManager::Builder(1)
            .boundingBox({{-100.0f, -100.0f, -100.0f}, {100.0f, 100.0f, 100.0f}})
            .layerMask(0xFF, 0x02)
            .material(0, editTriMi_)
            .geometry(0, filament::RenderableManager::PrimitiveType::TRIANGLES, editTriVb_, editTriIb_)
            .culling(true)
            .castShadows(false)
            .receiveShadows(false)
            .build(*engine_, editTriEntity_);
        scene_->addEntity(editTriEntity_);
    }
}

void FilamentBackend::rebuildWireframeOverlay(const NovaCoreHandle* core) {
    destroyWireframeResources();
    if (nova_core_get_shading_mode(core) != 1 || nova_core_material_preview_sphere_get(core)) {
        return;
    }

    std::vector<LineVertex> verts;
    std::vector<uint32_t> indices;
    const uint32_t wireCol = rgba(190, 224, 255);
    const size_t count = nova_core_scene_node_count(core);
    for (size_t i = 0; i < count; ++i) {
        NovaNodeMeta meta{};
        if (nova_core_scene_get_node_meta(core, i, &meta) != 0 || !meta.visible || !meta.has_mesh) {
            continue;
        }
        NovaMeshView mesh{};
        if (nova_core_scene_get_node_mesh(core, meta.id, &mesh) != 0 || mesh.vertex_count == 0) {
            continue;
        }
        const uint32_t baseVert = static_cast<uint32_t>(verts.size());
        for (size_t v = 0; v < mesh.vertex_count; ++v) {
            const float px = mesh.positions_ptr[v * 3 + 0];
            const float py = mesh.positions_ptr[v * 3 + 1];
            const float pz = mesh.positions_ptr[v * 3 + 2];
            const auto wp = transformMatPoint(meta.model_matrix, px, py, pz);
            verts.push_back({wp.x, wp.y, wp.z, wireCol});
        }
        for (size_t t = 0; t + 2 < mesh.index_count; t += 3) {
            const uint32_t i0 = baseVert + mesh.indices_ptr[t + 0];
            const uint32_t i1 = baseVert + mesh.indices_ptr[t + 1];
            const uint32_t i2 = baseVert + mesh.indices_ptr[t + 2];
            indices.push_back(i0); indices.push_back(i1);
            indices.push_back(i1); indices.push_back(i2);
            indices.push_back(i2); indices.push_back(i0);
        }
    }

    if (verts.empty() || indices.empty()) {
        return;
    }

    wireframeVb_ = filament::VertexBuffer::Builder()
                       .vertexCount(static_cast<uint32_t>(verts.size()))
                       .bufferCount(1)
                       .attribute(filament::VertexAttribute::POSITION, 0,
                                  filament::VertexBuffer::AttributeType::FLOAT3, 0, sizeof(LineVertex))
                       .attribute(filament::VertexAttribute::COLOR, 0,
                                  filament::VertexBuffer::AttributeType::UBYTE4,
                                  offsetof(LineVertex, colorAbgr), sizeof(LineVertex))
                       .normalized(filament::VertexAttribute::COLOR)
                       .build(*engine_);
    wireframeVb_->setBufferAt(*engine_, 0, makeOwnedDescriptor(verts.data(), verts.size()));

    wireframeIb_ = filament::IndexBuffer::Builder()
                       .indexCount(static_cast<uint32_t>(indices.size()))
                       .bufferType(filament::IndexBuffer::IndexType::UINT)
                       .build(*engine_);
    wireframeIb_->setBuffer(*engine_, makeOwnedDescriptor(indices.data(), indices.size()));

    wireframeMi_ = unlitLineMaterial_->createInstance();
    wireframeEntity_ = utils::EntityManager::get().create();
    filament::RenderableManager::Builder(1)
        .boundingBox({{-100.0f, -100.0f, -100.0f}, {100.0f, 100.0f, 100.0f}})
        .layerMask(0xFF, 0x02)
        .material(0, wireframeMi_)
        .geometry(0, filament::RenderableManager::PrimitiveType::LINES, wireframeVb_, wireframeIb_)
        .culling(false)
        .castShadows(false)
        .receiveShadows(false)
        .build(*engine_, wireframeEntity_);
    scene_->addEntity(wireframeEntity_);
}

void FilamentBackend::updateDynamicOverlay(const NovaCoreHandle* core) {
    if (!overlayVb_ || !overlayIb_ || overlayEntity_.isNull()) {
        return;
    }
    if (nova_core_material_preview_sphere_get(core)) {
        auto& rcm = engine_->getRenderableManager();
        rcm.setInstanceCount(rcm.getInstance(overlayEntity_), 0);
        return;
    }

    std::vector<LineVertex> verts;
    std::vector<uint32_t> indices;
    verts.reserve(512);
    indices.reserve(768);

    const uint32_t selPrimaryFrameCol    = rgba(138, 92, 255, 190); // Electric Violet Master Accent
    const uint32_t selPrimaryCornerCol   = rgba(228, 204, 255, 255); // Radiant Corner Highlight
    const uint32_t selSecondaryFrameCol  = rgba(56, 189, 248, 170); // Cool Cyan
    const uint32_t selSecondaryCornerCol = rgba(190, 235, 255, 255);
    const uint32_t iconLightCol = rgba(255, 224, 110);
    const uint32_t iconCamCol = rgba(164, 210, 255);
    const uint32_t iconEmptyCol = rgba(160, 172, 188);

    const bool inEditMode = (nova_core_get_interaction_mode(core) == 1);
    const uint64_t activeCamId = nova_core_camera_get_active_node(core);
    const size_t count = nova_core_scene_node_count(core);
    for (size_t i = 0; i < count; ++i) {
        NovaNodeMeta meta{};
        if (nova_core_scene_get_node_meta(core, i, &meta) != 0) {
            continue;
        }
        if (!meta.visible || (activeCamId != 0 && meta.id == activeCamId)) {
            continue;
        }

        if (meta.selected && !inEditMode) {
            const uint32_t frameCol = meta.primary_selected ? selPrimaryFrameCol : selSecondaryFrameCol;
            const uint32_t cornerCol = meta.primary_selected ? selPrimaryCornerCol : selSecondaryCornerCol;
            pushSelectionBox(verts, indices, meta.model_matrix, meta.bounds_min, meta.bounds_max,
                             frameCol, cornerCol, meta.primary_selected);
        }

        // Render lightweight markers for non-mesh nodes (Empty=0, Light=2, Camera=3)
        if (meta.kind == 0) {
            const uint32_t col = meta.selected ? selPrimaryCornerCol : iconEmptyCol;
            const auto o = transformMatPoint(meta.model_matrix, 0, 0, 0);
            const auto px = transformMatPoint(meta.model_matrix, 0.25f, 0, 0);
            const auto nx = transformMatPoint(meta.model_matrix, -0.25f, 0, 0);
            const auto py = transformMatPoint(meta.model_matrix, 0, 0.25f, 0);
            const auto ny = transformMatPoint(meta.model_matrix, 0, -0.25f, 0);
            const auto pz = transformMatPoint(meta.model_matrix, 0, 0, 0.25f);
            const auto nz = transformMatPoint(meta.model_matrix, 0, 0, -0.25f);
            pushLine(verts, indices, nx, px, col);
            pushLine(verts, indices, ny, py, col);
            pushLine(verts, indices, nz, pz, col);
            (void)o;
        } else if (meta.kind == 2) {
            const uint32_t col = meta.selected ? selPrimaryCornerCol : iconLightCol;
            const float r = 0.24f;
            const auto top = transformMatPoint(meta.model_matrix, 0, r, 0);
            const auto bot = transformMatPoint(meta.model_matrix, 0, -r, 0);
            const auto a = transformMatPoint(meta.model_matrix, r, 0, 0);
            const auto b = transformMatPoint(meta.model_matrix, -r, 0, 0);
            const auto c = transformMatPoint(meta.model_matrix, 0, 0, r);
            const auto d = transformMatPoint(meta.model_matrix, 0, 0, -r);
            pushLine(verts, indices, top, a, col); pushLine(verts, indices, top, b, col);
            pushLine(verts, indices, top, c, col); pushLine(verts, indices, top, d, col);
            pushLine(verts, indices, bot, a, col); pushLine(verts, indices, bot, b, col);
            pushLine(verts, indices, bot, c, col); pushLine(verts, indices, bot, d, col);
            pushLine(verts, indices, a, c, col); pushLine(verts, indices, c, b, col);
            pushLine(verts, indices, b, d, col); pushLine(verts, indices, d, a, col);
        } else if (meta.kind == 3) {
            const uint32_t col = meta.selected ? selPrimaryCornerCol : iconCamCol;
            const auto o = transformMatPoint(meta.model_matrix, 0, 0, 0);
            const auto f0 = transformMatPoint(meta.model_matrix, -0.24f, 0.16f, -0.45f);
            const auto f1 = transformMatPoint(meta.model_matrix, 0.24f, 0.16f, -0.45f);
            const auto f2 = transformMatPoint(meta.model_matrix, 0.24f, -0.16f, -0.45f);
            const auto f3 = transformMatPoint(meta.model_matrix, -0.24f, -0.16f, -0.45f);
            pushLine(verts, indices, o, f0, col); pushLine(verts, indices, o, f1, col);
            pushLine(verts, indices, o, f2, col); pushLine(verts, indices, o, f3, col);
            pushLine(verts, indices, f0, f1, col); pushLine(verts, indices, f1, f2, col);
            pushLine(verts, indices, f2, f3, col); pushLine(verts, indices, f3, f0, col);
        }
    }

    // 3D Transform Gizmo (Move / Rotate / Scale)
    NovaGizmoSnapshot gizmo{};
    if (nova_core_gizmo_get_snapshot(core, &gizmo) == 0 && gizmo.visible) {
        const filament::math::float3 pivot{gizmo.pivot[0], gizmo.pivot[1], gizmo.pivot[2]};
        const filament::math::float3 axes[3] = {
            {gizmo.axis_x[0], gizmo.axis_x[1], gizmo.axis_x[2]},
            {gizmo.axis_y[0], gizmo.axis_y[1], gizmo.axis_y[2]},
            {gizmo.axis_z[0], gizmo.axis_z[1], gizmo.axis_z[2]},
        };
        const float s = gizmo.scale;
        const float bold = s * 0.016f;
        const uint32_t activeCol = rgba(255, 235, 64);
        const uint32_t cols[3] = {
            (gizmo.active_axis == 1) ? activeCol : rgba(255, 68, 88),
            (gizmo.active_axis == 2) ? activeCol : rgba(46, 224, 110),
            (gizmo.active_axis == 3) ? activeCol : rgba(48, 168, 255),
        };
        const uint32_t centerCol = (gizmo.active_axis == 4) ? activeCol : rgba(250, 250, 255);

        if (gizmo.mode == 1) {
            // Move Gizmo: 3D conical arrowheads + multi-rail stems + planar handles + center octahedron
            for (int a = 0; a < 3; ++a) {
                const filament::math::float3 start = pivot + axes[a] * (s * 0.12f);
                const filament::math::float3 back = pivot + axes[a] * (s * 0.74f);
                const filament::math::float3 tip = pivot + axes[a] * (s * 1.05f);
                filament::math::float3 u, v;
                perpBasis(axes[a], u, v);

                // Thick 3D stem (4 rails + center)
                pushLine(verts, indices, start, back, cols[a]);
                pushLine(verts, indices, start + u * bold, back + u * bold, cols[a]);
                pushLine(verts, indices, start - u * bold, back - u * bold, cols[a]);
                pushLine(verts, indices, start + v * bold, back + v * bold, cols[a]);
                pushLine(verts, indices, start - v * bold, back - v * bold, cols[a]);

                // Mid stem reinforcement cross
                const auto mid1 = pivot + axes[a] * (s * 0.38f);
                const auto mid2 = pivot + axes[a] * (s * 0.58f);
                pushLine(verts, indices, mid1 - u * bold, mid1 + u * bold, cols[a]);
                pushLine(verts, indices, mid1 - v * bold, mid1 + v * bold, cols[a]);
                pushLine(verts, indices, mid2 - u * bold, mid2 + u * bold, cols[a]);
                pushLine(verts, indices, mid2 - v * bold, mid2 + v * bold, cols[a]);

                // 3D 8-sided Conical Arrowhead
                const float r = s * 0.088f;
                constexpr int kConeSides = 8;
                constexpr float kTwoPi = 6.283185307f;
                filament::math::float3 ringPts[kConeSides];
                for (int k = 0; k < kConeSides; ++k) {
                    const float th = (static_cast<float>(k) / kConeSides) * kTwoPi;
                    ringPts[k] = back + (u * std::cos(th) + v * std::sin(th)) * r;
                }
                for (int k = 0; k < kConeSides; ++k) {
                    const int nextK = (k + 1) % kConeSides;
                    pushLine(verts, indices, ringPts[k], ringPts[nextK], cols[a]); // base rim
                    pushLine(verts, indices, ringPts[k], tip, cols[a]);            // cone ridge
                    pushLine(verts, indices, ringPts[k], back, cols[a]);           // base cap spoke
                }
            }

            // Dual-axis planar movement handles (XY, XZ, YZ mini squares)
            const float p0 = s * 0.22f;
            const float p1 = s * 0.38f;
            const int pairs[3][2] = {{0, 1}, {0, 2}, {1, 2}};
            const uint32_t planeCols[3] = {
                rgba(255, 215, 80),  // XY
                rgba(215, 90, 255),  // XZ
                rgba(80, 230, 220),  // YZ
            };
            for (int p = 0; p < 3; ++p) {
                const auto a0 = axes[pairs[p][0]];
                const auto a1 = axes[pairs[p][1]];
                const auto corner0 = pivot + a0 * p1 + a1 * p0;
                const auto corner = pivot + a0 * p1 + a1 * p1;
                const auto corner1 = pivot + a0 * p0 + a1 * p1;
                pushLine(verts, indices, pivot + a0 * p1, corner, planeCols[p]);
                pushLine(verts, indices, corner, pivot + a1 * p1, planeCols[p]);
                pushLine(verts, indices, pivot + a0 * p0 + a1 * p0, corner0, planeCols[p]);
                pushLine(verts, indices, pivot + a0 * p0 + a1 * p0, corner1, planeCols[p]);
            }

            // Glowing Center Octahedron Handle
            const float cr = s * 0.085f;
            const filament::math::float3 cPts[6] = {
                pivot + axes[0] * cr, pivot - axes[0] * cr,
                pivot + axes[1] * cr, pivot - axes[1] * cr,
                pivot + axes[2] * cr, pivot - axes[2] * cr,
            };
            for (int k = 0; k < 2; ++k) {
                for (int m = 2; m < 4; ++m) {
                    for (int n = 4; n < 6; ++n) {
                        pushLine(verts, indices, cPts[k], cPts[m], centerCol);
                        pushLine(verts, indices, cPts[m], cPts[n], centerCol);
                        pushLine(verts, indices, cPts[n], cPts[k], centerCol);
                    }
                }
            }
        } else if (gizmo.mode == 2) {
            // Rotate Gizmo: 64-segment double-concentric rings + cardinal ticks
            const float ringR = s * 0.88f;
            constexpr int kSegs = 64;
            constexpr float kTwoPi = 6.283185307f;
            for (int a = 0; a < 3; ++a) {
                const auto u = axes[(a + 1) % 3];
                const auto v = axes[(a + 2) % 3];
                const auto n = axes[a] * (bold * 1.1f);
                for (int k = 0; k < kSegs; ++k) {
                    const float t0 = (static_cast<float>(k) / kSegs) * kTwoPi;
                    const float t1 = (static_cast<float>(k + 1) / kSegs) * kTwoPi;
                    const auto dir0 = u * std::cos(t0) + v * std::sin(t0);
                    const auto dir1 = u * std::cos(t1) + v * std::sin(t1);
                    const auto p0 = pivot + dir0 * ringR;
                    const auto p1 = pivot + dir1 * ringR;
                    pushLine(verts, indices, p0, p1, cols[a]);
                    pushLine(verts, indices, p0 + n, p1 + n, cols[a]);
                    pushLine(verts, indices, p0 - n, p1 - n, cols[a]);
                    if (k % 16 == 0) {
                        pushLine(verts, indices, p0 - n * 1.8f, p0 + n * 1.8f, cols[a]);
                    }
                }
            }
            // Screen-space outer tracker ring
            const uint32_t outerRingCol = rgba(160, 175, 205, 120);
            const float outR = s * 1.02f;
            for (int k = 0; k < 48; ++k) {
                const float t0 = (static_cast<float>(k) / 48) * kTwoPi;
                const float t1 = (static_cast<float>(k + 1) / 48) * kTwoPi;
                const auto p0 = pivot + (axes[0] * std::cos(t0) + axes[2] * std::sin(t0)) * outR;
                const auto p1 = pivot + (axes[0] * std::cos(t1) + axes[2] * std::sin(t1)) * outR;
                pushLine(verts, indices, p0, p1, outerRingCol);
            }
        } else if (gizmo.mode == 3) {
            // Scale Gizmo: 3D cube handles at tips + bold stems + center cube
            for (int a = 0; a < 3; ++a) {
                const auto start = pivot + axes[a] * (s * 0.12f);
                const auto tip = pivot + axes[a] * (s * 0.95f);
                filament::math::float3 u, v;
                perpBasis(axes[a], u, v);
                pushLine(verts, indices, start, tip, cols[a]);
                pushLine(verts, indices, start + u * bold, tip + u * bold, cols[a]);
                pushLine(verts, indices, start - u * bold, tip - u * bold, cols[a]);
                pushLine(verts, indices, start + v * bold, tip + v * bold, cols[a]);
                pushLine(verts, indices, start - v * bold, tip - v * bold, cols[a]);

                // Solid 3D Cube handle at axis tip
                const float r = s * 0.065f;
                const auto c = tip + axes[a] * r;
                for (int sx : {-1, 1}) {
                    for (int sy : {-1, 1}) {
                        const auto p0 = c + u * (r * sx) + v * (r * sy) - axes[a] * r;
                        const auto p1 = c + u * (r * sx) + v * (r * sy) + axes[a] * r;
                        pushLine(verts, indices, p0, p1, cols[a]);
                    }
                }
                for (int sa : {-1, 1}) {
                    const auto faceCenter = c + axes[a] * (r * sa);
                    const auto c0 = faceCenter - u * r - v * r;
                    const auto c1 = faceCenter + u * r - v * r;
                    const auto c2 = faceCenter + u * r + v * r;
                    const auto c3 = faceCenter - u * r + v * r;
                    pushLine(verts, indices, c0, c1, cols[a]);
                    pushLine(verts, indices, c1, c2, cols[a]);
                    pushLine(verts, indices, c2, c3, cols[a]);
                    pushLine(verts, indices, c3, c0, cols[a]);
                    pushLine(verts, indices, c0, c2, cols[a]);
                }
            }

            // Planar scale brackets
            const float p0 = s * 0.22f;
            const float p1 = s * 0.36f;
            const int pairs[3][2] = {{0, 1}, {0, 2}, {1, 2}};
            const uint32_t planeCols[3] = {rgba(255, 215, 80), rgba(215, 90, 255), rgba(80, 230, 220)};
            for (int p = 0; p < 3; ++p) {
                const auto a0 = axes[pairs[p][0]];
                const auto a1 = axes[pairs[p][1]];
                const auto corner = pivot + a0 * p1 + a1 * p1;
                pushLine(verts, indices, pivot + a0 * p1, corner, planeCols[p]);
                pushLine(verts, indices, corner, pivot + a1 * p1, planeCols[p]);
            }

            // Solid Central Cube handle
            const float cr = s * 0.08f;
            for (int sign : {-1, 1}) {
                const auto yOff = axes[1] * (cr * static_cast<float>(sign));
                const auto c0 = pivot + yOff - axes[0] * cr - axes[2] * cr;
                const auto c1 = pivot + yOff + axes[0] * cr - axes[2] * cr;
                const auto c2 = pivot + yOff + axes[0] * cr + axes[2] * cr;
                const auto c3 = pivot + yOff - axes[0] * cr + axes[2] * cr;
                pushLine(verts, indices, c0, c1, centerCol);
                pushLine(verts, indices, c1, c2, centerCol);
                pushLine(verts, indices, c2, c3, centerCol);
                pushLine(verts, indices, c3, c0, centerCol);
                pushLine(verts, indices, c0, c2, centerCol);
            }
            for (int sx : {-1, 1}) {
                for (int sz : {-1, 1}) {
                    const auto p0 = pivot + axes[0] * (cr * sx) + axes[2] * (cr * sz) - axes[1] * cr;
                    const auto p1 = pivot + axes[0] * (cr * sx) + axes[2] * (cr * sz) + axes[1] * cr;
                    pushLine(verts, indices, p0, p1, centerCol);
                }
            }
        }
    }

    auto& rcm = engine_->getRenderableManager();
    const auto instance = rcm.getInstance(overlayEntity_);
    if (indices.empty()) {
        rcm.setInstanceCount(instance, 0);
        return;
    }

    overlayVb_->setBufferAt(*engine_, 0, makeOwnedDescriptor(verts.data(), verts.size()));
    overlayIb_->setBuffer(*engine_, makeOwnedDescriptor(indices.data(), indices.size()));
    rcm.setGeometryAt(instance, 0, filament::RenderableManager::PrimitiveType::LINES,
                      overlayVb_, overlayIb_, 0, indices.size());
    rcm.setInstanceCount(instance, 1);
}

void FilamentBackend::syncSceneFromCore(const NovaCoreHandle* core) {
    syncSceneForPass(core, false);
}

void FilamentBackend::syncSceneForPass(const NovaCoreHandle* core, bool offscreenClean) {
    const uint64_t rev = nova_core_scene_revision(core);
    if (!offscreenClean && rev == syncedSceneRevision_) {
        return;
    }

    const uint32_t rawShadingMode = nova_core_get_shading_mode(core);
    const uint32_t shadingMode = offscreenClean ? std::max(2u, rawShadingMode) : rawShadingMode;
    const bool previewSphereMode = offscreenClean ? false : nova_core_material_preview_sphere_get(core);
    syncEnvironmentFromCore(core, shadingMode);

    bool showGrid = !previewSphereMode;
    bool showAxes = !previewSphereMode;
    if (!previewSphereMode) {
        nova_core_scene_get_grid_axes(core, &showGrid, &showAxes);
    }

    if (!gridEntity_.isNull()) {
        if (showGrid && !gridAttached_) {
            scene_->addEntity(gridEntity_);
            gridAttached_ = true;
        } else if (!showGrid && gridAttached_) {
            scene_->remove(gridEntity_);
            gridAttached_ = false;
        }
    }
    if (!axesEntity_.isNull()) {
        if (showAxes && !axesAttached_) {
            scene_->addEntity(axesEntity_);
            axesAttached_ = true;
        } else if (!showAxes && axesAttached_) {
            scene_->remove(axesEntity_);
            axesAttached_ = false;
        }
    }

    const size_t count = nova_core_scene_node_count(core);
    auto& tcm = engine_->getTransformManager();
    auto& rcm = engine_->getRenderableManager();
    std::unordered_set<uint64_t> seenMeshIds;
    std::unordered_set<uint64_t> seenLightIds;
    bool hasUserDirectionalLight = false;

    for (size_t i = 0; i < count; ++i) {
        NovaNodeMeta meta{};
        if (nova_core_scene_get_node_meta(core, i, &meta) != 0) {
            continue;
        }

        // Handle user Light nodes
        if (meta.kind == 2) {
            seenLightIds.insert(meta.id);
            if (meta.visible && meta.light_kind == 0) {
                hasUserDirectionalLight = true;
            }
            const float innerRad = std::max(0.01f, meta.light_spot_inner) * (3.14159265f / 180.0f);
            const float outerRad = std::max(innerRad + 0.02f, meta.light_spot_outer * (3.14159265f / 180.0f));
            const float falloffRange = std::max(0.5f, meta.light_range);
            const filament::math::float3 pos{
                meta.model_matrix[12], meta.model_matrix[13], meta.model_matrix[14]};
            const filament::math::float3 fwd = normalizeVec(
                {-meta.model_matrix[8], -meta.model_matrix[9], -meta.model_matrix[10]},
                {-0.55f, -0.82f, -0.45f});

            auto lit = gpuLights_.find(meta.id);
            if (lit != gpuLights_.end() && lit->second.lightKind != meta.light_kind) {
                destroyLightResources(lit->second);
                gpuLights_.erase(lit);
                lit = gpuLights_.end();
            }
            if (lit == gpuLights_.end()) {
                GpuLightNode gl;
                gl.lightKind = meta.light_kind;
                gl.entity = utils::EntityManager::get().create();
                const auto fType = (meta.light_kind == 0)
                                       ? filament::LightManager::Type::DIRECTIONAL
                                       : (meta.light_kind == 2)
                                             ? filament::LightManager::Type::SPOT
                                             : filament::LightManager::Type::POINT;
                filament::LightManager::Builder(fType)
                    .color({meta.light_color[0], meta.light_color[1], meta.light_color[2]})
                    .intensity(meta.light_intensity)
                    .position(pos)
                    .direction(fwd)
                    .falloff(falloffRange)
                    .spotLightCone(innerRad, outerRad)
                    .castShadows(meta.light_cast_shadows)
                    .build(*engine_, gl.entity);
                if (meta.visible) {
                    scene_->addEntity(gl.entity);
                    gl.inScene = true;
                }
                gpuLights_[meta.id] = gl;
            } else {
                auto& lm = engine_->getLightManager();
                const auto inst = lm.getInstance(lit->second.entity);
                lm.setColor(inst, {meta.light_color[0], meta.light_color[1], meta.light_color[2]});
                lm.setIntensity(inst, meta.light_intensity);
                lm.setPosition(inst, pos);
                lm.setDirection(inst, fwd);
                lm.setFalloff(inst, falloffRange);
                lm.setSpotLightCone(inst, innerRad, outerRad);

                if (meta.visible && !lit->second.inScene) {
                    scene_->addEntity(lit->second.entity);
                    lit->second.inScene = true;
                } else if (!meta.visible && lit->second.inScene) {
                    scene_->remove(lit->second.entity);
                    lit->second.inScene = false;
                }
            }
        }

        if (!meta.has_mesh) {
            continue;
        }
        seenMeshIds.insert(meta.id);

        auto it = gpuNodes_.find(meta.id);
        if (it == gpuNodes_.end() || it->second.meshVersion != meta.mesh_version) {
            if (it != gpuNodes_.end()) {
                destroyNodeResources(it->second);
                gpuNodes_.erase(it);
            }

            NovaMeshView mesh{};
            if (nova_core_scene_get_node_mesh(core, meta.id, &mesh) != 0) {
                continue;
            }

            GpuMeshNode gpuNode;
            gpuNode.meshVersion = meta.mesh_version;
            gpuNode.entity = utils::EntityManager::get().create();
            gpuNode.isTransparent = (meta.alpha_mode == 1 || meta.base_color[3] < 0.995f);

            auto* orientation =
                filament::geometry::SurfaceOrientation::Builder()
                    .vertexCount(mesh.vertex_count)
                    .normals(reinterpret_cast<const filament::math::float3*>(mesh.normals_ptr))
                    .uvs(reinterpret_cast<const filament::math::float2*>(mesh.uvs_ptr))
                    .positions(reinterpret_cast<const filament::math::float3*>(mesh.positions_ptr))
                    .triangleCount(mesh.index_count / 3)
                    .triangles(reinterpret_cast<const filament::math::uint3*>(mesh.indices_ptr))
                    .build();

            std::vector<filament::math::short4> tangents(mesh.vertex_count);
            if (orientation) {
                orientation->getQuats(tangents.data(), mesh.vertex_count);
                delete orientation;
            } else {
                for (auto& q : tangents) {
                    q = {0, 0, 0, 32767};
                }
            }

            gpuNode.vertexBuffer =
                filament::VertexBuffer::Builder()
                    .vertexCount(static_cast<uint32_t>(mesh.vertex_count))
                    .bufferCount(3)
                    .attribute(filament::VertexAttribute::POSITION, 0,
                               filament::VertexBuffer::AttributeType::FLOAT3, 0,
                               sizeof(float) * 3)
                    .attribute(filament::VertexAttribute::TANGENTS, 1,
                               filament::VertexBuffer::AttributeType::SHORT4, 0,
                               sizeof(filament::math::short4))
                    .normalized(filament::VertexAttribute::TANGENTS)
                    .attribute(filament::VertexAttribute::UV0, 2,
                               filament::VertexBuffer::AttributeType::FLOAT2, 0,
                               sizeof(float) * 2)
                    .build(*engine_);

            gpuNode.vertexBuffer->setBufferAt(
                *engine_, 0, makeOwnedDescriptor(mesh.positions_ptr, mesh.vertex_count * 3));
            gpuNode.vertexBuffer->setBufferAt(
                *engine_, 1, makeOwnedDescriptor(tangents.data(), tangents.size()));
            gpuNode.vertexBuffer->setBufferAt(
                *engine_, 2, makeOwnedDescriptor(mesh.uvs_ptr, mesh.vertex_count * 2));

            gpuNode.indexBuffer =
                filament::IndexBuffer::Builder()
                    .indexCount(static_cast<uint32_t>(mesh.index_count))
                    .bufferType(filament::IndexBuffer::IndexType::UINT)
                    .build(*engine_);
            gpuNode.indexBuffer->setBuffer(
                *engine_, makeOwnedDescriptor(mesh.indices_ptr, mesh.index_count));

            gpuNode.materialInstance = (gpuNode.isTransparent ? pbrTransparentMaterial_ : pbrMaterial_)->createInstance();

            filament::Box bbox;
            const filament::math::float3 bMin{mesh.bounds_min[0], mesh.bounds_min[1], mesh.bounds_min[2]};
            const filament::math::float3 bMax{mesh.bounds_max[0], mesh.bounds_max[1], mesh.bounds_max[2]};
            bbox.set(bMin, bMax);

            filament::RenderableManager::Builder(1)
                .boundingBox(bbox)
                .material(0, gpuNode.materialInstance)
                .geometry(0, filament::RenderableManager::PrimitiveType::TRIANGLES,
                          gpuNode.vertexBuffer, gpuNode.indexBuffer)
                .castShadows(!gpuNode.isTransparent)
                .receiveShadows(true)
                .build(*engine_, gpuNode.entity);

            if (meta.visible && !previewSphereMode) {
                scene_->addEntity(gpuNode.entity);
                gpuNode.inScene = true;
            }
            gpuNodes_[meta.id] = gpuNode;
            it = gpuNodes_.find(meta.id);
        }

        GpuMeshNode& gpuNode = it->second;
        const bool nodeVisible = meta.visible && !previewSphereMode;
        if (nodeVisible && !gpuNode.inScene) {
            scene_->addEntity(gpuNode.entity);
            gpuNode.inScene = true;
        } else if (!nodeVisible && gpuNode.inScene) {
            scene_->remove(gpuNode.entity);
            gpuNode.inScene = false;
        }

        const bool wantTrans = (meta.alpha_mode == 1 || meta.base_color[3] < 0.995f) && (shadingMode >= 2);
        if (wantTrans != gpuNode.isTransparent) {
            filament::MaterialInstance* oldMi = gpuNode.materialInstance;
            filament::MaterialInstance* nextMi = (wantTrans ? pbrTransparentMaterial_ : pbrMaterial_)->createInstance();
            const auto rInst = rcm.getInstance(gpuNode.entity);
            rcm.setMaterialInstanceAt(rInst, 0, nextMi);
            rcm.setCastShadows(rInst, !wantTrans);
            gpuNode.materialInstance = nextMi;
            gpuNode.isTransparent = wantTrans;
            if (oldMi) {
                engine_->destroy(oldMi);
            }
        }

        applyMaterialToInstance(
            core, gpuNode.materialInstance, shadingMode,
            meta.base_color, meta.metallic, meta.roughness,
            meta.emissive, meta.emissive_intensity, meta.ao_strength,
            meta.selected && !offscreenClean, meta.uv_scale, meta.uv_offset,
            meta.albedo_tex_id, meta.normal_tex_id, meta.roughness_tex_id,
            meta.ao_tex_id, meta.emissive_tex_id);

        filament::math::mat4f mat;
        std::memcpy(&mat, meta.model_matrix, sizeof(float) * 16);
        tcm.setTransform(tcm.getInstance(gpuNode.entity), mat);
    }

    // Update live Shaderball Preview Sphere when Materials tab preview is active (MT-4)
    if (!previewSphereEntity_.isNull()) {
        if (previewSphereMode) {
            NovaMaterialInfo matInfo{};
            const uint64_t selMatId = nova_core_material_selected_id(core);
            if (nova_core_material_get(core, selMatId, &matInfo) == 0) {
                const bool wantSphereTrans =
                    (matInfo.alpha_mode == 1 || matInfo.base_color[3] < 0.995f);
                if (wantSphereTrans != previewSphereTransparent_) {
                    filament::MaterialInstance* oldMi = previewSphereMi_;
                    filament::MaterialInstance* nextMi =
                        (wantSphereTrans ? pbrTransparentMaterial_ : pbrMaterial_)->createInstance();
                    const auto rInst = rcm.getInstance(previewSphereEntity_);
                    rcm.setMaterialInstanceAt(rInst, 0, nextMi);
                    rcm.setCastShadows(rInst, !wantSphereTrans);
                    previewSphereMi_ = nextMi;
                    previewSphereTransparent_ = wantSphereTrans;
                    if (oldMi) {
                        engine_->destroy(oldMi);
                    }
                }
                applyMaterialToInstance(
                    core, previewSphereMi_, std::max(2u, shadingMode),
                    matInfo.base_color, matInfo.metallic, matInfo.roughness,
                    matInfo.emissive, matInfo.emissive_intensity, matInfo.ao_strength,
                    false, matInfo.uv_scale, matInfo.uv_offset,
                    matInfo.albedo_tex_id, matInfo.normal_tex_id, matInfo.roughness_tex_id,
                    matInfo.ao_tex_id, matInfo.emissive_tex_id);
            }
            if (!previewSphereAttached_) {
                scene_->addEntity(previewSphereEntity_);
                previewSphereAttached_ = true;
            }
        } else if (previewSphereAttached_) {
            scene_->remove(previewSphereEntity_);
            previewSphereAttached_ = false;
        }
    }

    // Remove GPU nodes deleted from Rust core
    for (auto it = gpuNodes_.begin(); it != gpuNodes_.end();) {
        if (seenMeshIds.find(it->first) == seenMeshIds.end()) {
            destroyNodeResources(it->second);
            it = gpuNodes_.erase(it);
        } else {
            ++it;
        }
    }
    for (auto it = gpuLights_.begin(); it != gpuLights_.end();) {
        if (seenLightIds.find(it->first) == seenLightIds.end()) {
            destroyLightResources(it->second);
            it = gpuLights_.erase(it);
        } else {
            ++it;
        }
    }

    // Keep fallback studio sun unless user placed an explicit Directional Light
    if (!sunLightEntity_.isNull()) {
        if (hasUserDirectionalLight && sunLightAttached_) {
            scene_->remove(sunLightEntity_);
            sunLightAttached_ = false;
        } else if (!hasUserDirectionalLight && !sunLightAttached_) {
            scene_->addEntity(sunLightEntity_);
            sunLightAttached_ = true;
        }
    }

    if (offscreenClean) {
        syncedSceneRevision_ = 0;
    } else {
        rebuildWireframeOverlay(core);
        rebuildEditOverlay(core);
        syncedSceneRevision_ = rev;
    }
}

void FilamentBackend::syncCameraFromCore(const NovaCoreHandle* core) {
    syncCameraForExtent(core, width_, height_, 0.0f, true);
}

void FilamentBackend::syncCameraForExtent(const NovaCoreHandle* core,
                                          int32_t w,
                                          int32_t h,
                                          float focusDist,
                                          bool allowPreviewSphere) {
    NovaCameraSnapshot snap{};
    if (nova_core_camera_get_snapshot(core, &snap) != 0 || !camera_) {
        return;
    }

    const double aspect =
        static_cast<double>(std::max<int32_t>(1, w)) /
        static_cast<double>(std::max<int32_t>(1, h));

    if (snap.projection == 1) {
        const double halfH = static_cast<double>(snap.ortho_half_height);
        const double halfW = halfH * aspect;
        camera_->setProjection(filament::Camera::Projection::ORTHO,
                               -halfW, halfW, -halfH, halfH,
                               static_cast<double>(snap.near_plane),
                               static_cast<double>(snap.far_plane));
    } else {
        camera_->setProjection(static_cast<double>(snap.fov_y_degrees),
                               aspect,
                               static_cast<double>(snap.near_plane),
                               static_cast<double>(snap.far_plane),
                               filament::Camera::Fov::VERTICAL);
    }

    if (focusDist > 0.0f) {
        camera_->setFocusDistance(std::max(0.1f, focusDist));
    }

    if (allowPreviewSphere && nova_core_material_preview_sphere_get(core)) {
        const float d = std::max(2.6f, std::min(6.2f, snap.distance * 0.72f));
        const float cp = std::cos(snap.pitch_radians);
        const float sp = std::sin(snap.pitch_radians);
        const float sy = std::sin(snap.yaw_radians);
        const float cy = std::cos(snap.yaw_radians);
        camera_->lookAt(
            {d * cp * sy, d * sp, d * cp * cy},
            {0.0f, 0.0f, 0.0f},
            {0.0f, 1.0f, 0.0f});
    } else {
        camera_->lookAt(
            {snap.eye[0], snap.eye[1], snap.eye[2]},
            {snap.target[0], snap.target[1], snap.target[2]},
            {snap.up[0], snap.up[1], snap.up[2]});
    }
}

bool FilamentBackend::renderFrame(const NovaCoreHandle* core, uint64_t frameTimeNanos) {
    if (!engine_ || !renderer_ || !swapChain_ || !view_ || !core) {
        return false;
    }

    try {
        syncSceneFromCore(core);
        syncCameraFromCore(core);
        updateDynamicOverlay(core);

        if (renderer_->beginFrame(swapChain_, frameTimeNanos)) {
            renderer_->render(view_);
            renderer_->endFrame();
        }
        return true;
    } catch (const std::exception& e) {
        lastError_ = e.what();
        LOGE("Filament renderFrame failed: %s", lastError_.c_str());
        return false;
    }
}

void FilamentBackend::setViewportAdaptiveQuality(bool enabled) {
    viewportAdaptiveQuality_ = enabled;
    if (!view_) {
        return;
    }
    filament::View::DynamicResolutionOptions dyn{};
    dyn.enabled = enabled;
    dyn.minScale = enabled ? filament::math::float2{0.68f, 0.68f} : filament::math::float2{1.0f, 1.0f};
    dyn.maxScale = {1.0f, 1.0f};
    dyn.sharpness = 0.85f;
    dyn.quality = enabled ? filament::View::QualityLevel::MEDIUM : filament::View::QualityLevel::LOW;
    view_->setDynamicResolutionOptions(dyn);
}

void FilamentBackend::applyQualityToView(filament::View* targetView,
                                         const RenderQualityConfig& cfg,
                                         bool offscreen) {
    if (!engine_ || !targetView) {
        return;
    }
    const int level = std::clamp(cfg.qualityLevel, 0, 3);
    const auto qLevel = (level == 0) ? filament::View::QualityLevel::LOW
                      : (level == 1) ? filament::View::QualityLevel::MEDIUM
                      : (level == 2) ? filament::View::QualityLevel::HIGH
                                     : filament::View::QualityLevel::ULTRA;

    targetView->setAntiAliasing(cfg.aaEnabled
                                    ? filament::View::AntiAliasing::FXAA
                                    : filament::View::AntiAliasing::NONE);

    filament::View::MultiSampleAntiAliasingOptions msaa{};
    msaa.enabled = cfg.aaEnabled && cfg.msaaSamples > 1 && level >= 1;
    msaa.sampleCount = std::clamp<uint8_t>(cfg.msaaSamples, 1, 4);
    msaa.customResolve = (level >= 2);
    targetView->setMultiSampleAntiAliasingOptions(msaa);

    filament::View::AmbientOcclusionOptions ao{};
    ao.enabled = cfg.ssaoEnabled;
    ao.aoType = (level >= 2)
                    ? filament::View::AmbientOcclusionOptions::AmbientOcclusionType::GTAO
                    : filament::View::AmbientOcclusionOptions::AmbientOcclusionType::SAO;
    ao.radius = 0.35f;
    ao.power = 1.15f;
    ao.intensity = (level >= 2) ? 1.1f : 0.9f;
    ao.resolution = (level >= 3) ? 1.0f : 0.5f;
    ao.quality = qLevel;
    ao.lowPassFilter = (level >= 2) ? filament::View::QualityLevel::HIGH : filament::View::QualityLevel::MEDIUM;
    ao.upsampling = (level >= 2) ? filament::View::QualityLevel::HIGH : filament::View::QualityLevel::LOW;
    ao.bentNormals = (level >= 3);
    targetView->setAmbientOcclusionOptions(ao);

    filament::View::BloomOptions bloom{};
    bloom.enabled = cfg.bloomEnabled;
    bloom.strength = (level >= 2) ? 0.11f : 0.08f;
    bloom.resolution = (level >= 3) ? 512u : 384u;
    bloom.levels = (level >= 2) ? 6 : 5;
    targetView->setBloomOptions(bloom);

    filament::View::DepthOfFieldOptions dof{};
    dof.enabled = cfg.dofEnabled;
    dof.cocScale = std::clamp(cfg.dofCocScale, 0.1f, 3.0f);
    dof.maxApertureDiameter = std::clamp(cfg.dofMaxAperture, 0.001f, 0.25f);
    dof.nativeResolution = (level >= 3);
    dof.foregroundRingCount = (level >= 2) ? 5 : 3;
    dof.backgroundRingCount = (level >= 2) ? 5 : 3;
    targetView->setDepthOfFieldOptions(dof);
    if (cfg.dofEnabled && camera_) {
        camera_->setFocusDistance(std::max(0.1f, cfg.dofFocusDistance));
    }

    filament::View::RenderQuality rq{};
    rq.hdrColorBuffer = (level >= 2)
                            ? filament::View::QualityLevel::HIGH
                            : filament::View::QualityLevel::MEDIUM;
    targetView->setRenderQuality(rq);

    targetView->setShadowType((level >= 3)   ? filament::View::ShadowType::PCSS
                              : (level >= 1) ? filament::View::ShadowType::PCF
                                             : filament::View::ShadowType::VSM);
    targetView->setDithering(cfg.aaEnabled
                                 ? filament::View::Dithering::TEMPORAL
                                 : filament::View::Dithering::NONE);

    if (offscreen) {
        filament::View::DynamicResolutionOptions dyn{};
        dyn.enabled = false;
        targetView->setDynamicResolutionOptions(dyn);

        if (offscreenColorGrading_) {
            targetView->setColorGrading(nullptr);
            engine_->destroy(offscreenColorGrading_);
            offscreenColorGrading_ = nullptr;
        }
        std::unique_ptr<filament::ToneMapper> mapper;
        if (cfg.toneMapping == 1) {
            mapper = std::make_unique<filament::FilmicToneMapper>();
        } else if (cfg.toneMapping == 2) {
            mapper = std::make_unique<filament::PBRNeutralToneMapper>();
        } else {
            mapper = std::make_unique<filament::ACESToneMapper>();
        }
        offscreenColorGrading_ = filament::ColorGrading::Builder()
                                     .quality(filament::ColorGrading::QualityLevel::HIGH)
                                     .toneMapper(mapper.get())
                                     .build(*engine_);
        targetView->setColorGrading(offscreenColorGrading_);
    }
}

bool FilamentBackend::renderOffscreenStillRgba(const NovaCoreHandle* core,
                                               uint32_t width,
                                               uint32_t height,
                                               const RenderQualityConfig& cfg,
                                               std::vector<uint8_t>& outRgba,
                                               std::string& outError) {
    outError.clear();
    if (!engine_ || !renderer_ || !scene_ || !camera_ || !core) {
        outError = "Renderer is not initialized.";
        return false;
    }
    if (width == 0 || height == 0 || width > 4096 || height > 4096) {
        outError = "Unsupported still render dimensions.";
        return false;
    }
    if (offscreenSwapChain_) {
        outError = "Another offscreen render session is already active.";
        return false;
    }

    filament::SwapChain* headlessSwap =
        engine_->createSwapChain(width, height, filament::SwapChain::CONFIG_READABLE);
    if (!headlessSwap) {
        outError = "Failed to allocate headless readable SwapChain for offscreen capture.";
        return false;
    }

    try {
        if (!offscreenView_) {
            offscreenView_ = engine_->createView();
            offscreenView_->setScene(scene_);
            offscreenView_->setCamera(camera_);
            offscreenView_->setPostProcessingEnabled(true);
            // Clean render pass: ONLY layer 0 (real scene meshes), never editor overlays (layer 1)
            offscreenView_->setVisibleLayers(0x03, 0x01);
        }

        applyQualityToView(offscreenView_, cfg, true);
        offscreenView_->setViewport({0, 0, width, height});

        syncSceneForPass(core, true);
        syncCameraForExtent(core, static_cast<int32_t>(width), static_cast<int32_t>(height),
                            cfg.dofEnabled ? cfg.dofFocusDistance : 0.0f, false);

        const size_t totalBytes = static_cast<size_t>(width) * static_cast<size_t>(height) * 4u;
        std::vector<uint8_t> raw(totalBytes, 0);
        struct CaptureState {
            bool completed = false;
        } capState;

        filament::backend::PixelBufferDescriptor pbd(
            raw.data(), totalBytes,
            filament::backend::PixelDataFormat::RGBA,
            filament::backend::PixelDataType::UBYTE,
            1, 0, 0, width,
            [](void* /*buf*/, size_t /*sz*/, void* user) {
                static_cast<CaptureState*>(user)->completed = true;
            },
            &capState);

        static uint64_t sStillSeq = 1000000000ULL;
        sStillSeq += 16666667ULL;

        if (!renderer_->beginFrame(headlessSwap, sStillSeq)) {
            engine_->destroy(headlessSwap);
            outError = "Renderer declined headless frame start.";
            syncedSceneRevision_ = 0;
            syncCameraFromCore(core);
            return false;
        }
        renderer_->render(offscreenView_);
        renderer_->readPixels(0, 0, width, height, std::move(pbd));
        renderer_->endFrame();
        engine_->flushAndWait();

        engine_->destroy(headlessSwap);
        headlessSwap = nullptr;

        if (!capState.completed) {
            outError = "GPU readback callback did not complete cleanly.";
            syncCameraFromCore(core);
            return false;
        }

        // OpenGL ES readPixels returns bottom-up rows; Vulkan readPixels is already top-down.
        if (engine_->getBackend() == filament::Engine::Backend::OPENGL) {
            outRgba.resize(totalBytes);
            const size_t rowStride = static_cast<size_t>(width) * 4u;
            for (uint32_t y = 0; y < height; ++y) {
                const uint32_t srcY = height - 1u - y;
                std::memcpy(outRgba.data() + static_cast<size_t>(y) * rowStride,
                            raw.data() + static_cast<size_t>(srcY) * rowStride,
                            rowStride);
            }
        } else {
            outRgba = std::move(raw);
        }

        // Restore interactive viewport state
        syncedSceneRevision_ = 0;
        syncCameraFromCore(core);
        return true;
    } catch (const std::exception& e) {
        if (headlessSwap) {
            engine_->destroy(headlessSwap);
        }
        outError = e.what();
        syncedSceneRevision_ = 0;
        syncCameraFromCore(core);
        return false;
    }
}

bool FilamentBackend::beginOffscreenSurface(ANativeWindow* window,
                                            int32_t width,
                                            int32_t height,
                                            const RenderQualityConfig& cfg,
                                            std::string& outError) {
    outError.clear();
    if (!engine_ || !renderer_ || !scene_ || !camera_ || !window) {
        outError = "Invalid renderer state or encoder Surface.";
        return false;
    }
    endOffscreenSurface();

    offscreenSwapChain_ = engine_->createSwapChain(window);
    if (!offscreenSwapChain_) {
        outError = "Failed to create Filament SwapChain for MediaCodec Surface.";
        return false;
    }

    if (!offscreenView_) {
        offscreenView_ = engine_->createView();
        offscreenView_->setScene(scene_);
        offscreenView_->setCamera(camera_);
        offscreenView_->setPostProcessingEnabled(true);
        offscreenView_->setVisibleLayers(0x03, 0x01);
    }

    applyQualityToView(offscreenView_, cfg, true);
    offscreenView_->setViewport({0, 0,
                                 static_cast<uint32_t>(std::max(1, width)),
                                 static_cast<uint32_t>(std::max(1, height))});
    return true;
}

bool FilamentBackend::renderOffscreenSurfaceFrame(const NovaCoreHandle* core,
                                                  uint64_t frameTimeNanos,
                                                  int32_t width,
                                                  int32_t height) {
    if (!engine_ || !renderer_ || !offscreenSwapChain_ || !offscreenView_ || !core) {
        return false;
    }
    try {
        syncSceneForPass(core, true);
        syncCameraForExtent(core, width, height, 0.0f, false);
        if (renderer_->beginFrame(offscreenSwapChain_, frameTimeNanos)) {
            renderer_->render(offscreenView_);
            renderer_->endFrame();
        }
        engine_->flushAndWait();
        return true;
    } catch (const std::exception& e) {
        lastError_ = e.what();
        LOGE("Offscreen video frame failed: %s", lastError_.c_str());
        return false;
    }
}

void FilamentBackend::endOffscreenSurface() {
    if (!engine_) {
        return;
    }
    if (offscreenSwapChain_) {
        engine_->flushAndWait();
        engine_->destroy(offscreenSwapChain_);
        offscreenSwapChain_ = nullptr;
    }
    syncedSceneRevision_ = 0;
}

std::string FilamentBackend::activeBackendName() const {
    if (!engine_) {
        return "Uninitialized";
    }
    switch (engine_->getBackend()) {
        case filament::Engine::Backend::VULKAN:
            return "Vulkan";
        case filament::Engine::Backend::OPENGL:
            return "OpenGL ES";
        default:
            return "Other";
    }
}

std::string FilamentBackend::lastError() const {
    return lastError_;
}

} // namespace nova
