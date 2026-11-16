#ifndef NOVA_FILAMENT_BACKEND_H
#define NOVA_FILAMENT_BACKEND_H

#include <cstdint>
#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

#include <filament/Camera.h>
#include <filament/ColorGrading.h>
#include <filament/Engine.h>
#include <filament/IndexBuffer.h>
#include <filament/IndirectLight.h>
#include <filament/Material.h>
#include <filament/MaterialInstance.h>
#include <filament/Renderer.h>
#include <filament/Scene.h>
#include <filament/Skybox.h>
#include <filament/SwapChain.h>
#include <filament/Texture.h>
#include <filament/TextureSampler.h>
#include <filament/VertexBuffer.h>
#include <filament/View.h>
#include <utils/Entity.h>

#include "render_backend.h"

namespace nova {

class FilamentBackend final : public RenderBackend {
public:
    FilamentBackend() = default;
    ~FilamentBackend() override;

    bool initialize(GraphicsBackendPreference preference) override;
    void shutdown() override;

    bool attachSurface(ANativeWindow* window, int32_t width, int32_t height) override;
    void resizeSurface(int32_t width, int32_t height) override;
    void detachSurface() override;

    bool renderFrame(const NovaCoreHandle* core, uint64_t frameTimeNanos) override;

    void setViewportAdaptiveQuality(bool enabled) override;
    bool renderOffscreenStillRgba(const NovaCoreHandle* core,
                                  uint32_t width,
                                  uint32_t height,
                                  const RenderQualityConfig& cfg,
                                  std::vector<uint8_t>& outRgba,
                                  std::string& outError) override;
    bool beginOffscreenSurface(ANativeWindow* window,
                               int32_t width,
                               int32_t height,
                               const RenderQualityConfig& cfg,
                               std::string& outError) override;
    bool renderOffscreenSurfaceFrame(const NovaCoreHandle* core,
                                     uint64_t frameTimeNanos,
                                     int32_t width,
                                     int32_t height) override;
    void endOffscreenSurface() override;

    std::string activeBackendName() const override;
    std::string lastError() const override;
    bool isInitialized() const override { return engine_ != nullptr; }
    bool hasSurface() const override { return swapChain_ != nullptr; }

private:
    struct GpuMeshNode {
        utils::Entity entity;
        filament::VertexBuffer* vertexBuffer = nullptr;
        filament::IndexBuffer* indexBuffer = nullptr;
        filament::MaterialInstance* materialInstance = nullptr;
        uint64_t meshVersion = 0;
        bool inScene = false;
        bool isTransparent = false;
    };

    struct GpuLightNode {
        utils::Entity entity;
        uint32_t lightKind = 0;
        bool inScene = false;
    };

    struct GpuTextureEntry {
        filament::Texture* texture = nullptr;
        uint64_t version = 0;
    };

    bool buildDefaultMaterials();
    bool buildDefaultTextures();
    void createStudioLighting();
    void createGridAndAxes();
    void createOverlayRenderable();
    void createPreviewSphere();
    void syncEnvironmentFromCore(const NovaCoreHandle* core, uint32_t shadingMode);
    filament::Texture* resolveGpuTexture(const NovaCoreHandle* core, uint64_t texId, bool isSrgb);
    void applyMaterialToInstance(
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
        uint64_t emissiveTexId);
    void destroyNodeResources(GpuMeshNode& node);
    void destroyLightResources(GpuLightNode& light);
    void destroyWireframeResources();
    void destroyEditOverlayResources();
    void rebuildWireframeOverlay(const NovaCoreHandle* core);
    void rebuildEditOverlay(const NovaCoreHandle* core);
    void updateDynamicOverlay(const NovaCoreHandle* core);
    void syncSceneFromCore(const NovaCoreHandle* core);
    void syncCameraFromCore(const NovaCoreHandle* core);

    filament::Engine* engine_ = nullptr;
    filament::Renderer* renderer_ = nullptr;
    filament::Scene* scene_ = nullptr;
    filament::View* view_ = nullptr;
    filament::Camera* camera_ = nullptr;
    filament::SwapChain* swapChain_ = nullptr;
    ANativeWindow* window_ = nullptr;

    utils::Entity cameraEntity_;
    utils::Entity sunLightEntity_;
    bool sunLightAttached_ = false;
    filament::IndirectLight* indirectLight_ = nullptr;
    filament::Texture* iblCubemap_ = nullptr;
    filament::Skybox* skybox_ = nullptr;
    filament::Texture* skyCubemap_ = nullptr;
    uint32_t syncedIblPreset_ = 0xFFFFFFFFu;
    uint32_t syncedSkyPreset_ = 0xFFFFFFFFu;
    float syncedSkyRotation_ = -9999.0f;
    uint64_t syncedEnvVersion_ = 0;
    uint32_t syncedEnvShading_ = 0xFFFFFFFFu;

    filament::Material* pbrMaterial_ = nullptr;
    filament::Material* pbrTransparentMaterial_ = nullptr;
    filament::Material* unlitLineMaterial_ = nullptr;
    filament::Material* overlayLineMaterial_ = nullptr;
    filament::Material* translucentTriMaterial_ = nullptr;

    filament::Texture* defaultWhiteTex_ = nullptr;
    filament::Texture* defaultFlatNormalTex_ = nullptr;
    filament::TextureSampler defaultSampler_{};
    std::unordered_map<uint64_t, GpuTextureEntry> gpuTextures_;

    // Live Shaderball Preview Sphere for Materials tab (MT-4)
    utils::Entity previewSphereEntity_;
    filament::VertexBuffer* previewSphereVb_ = nullptr;
    filament::IndexBuffer* previewSphereIb_ = nullptr;
    filament::MaterialInstance* previewSphereMi_ = nullptr;
    bool previewSphereTransparent_ = false;
    bool previewSphereAttached_ = false;

    // Grid & axes entities
    utils::Entity gridEntity_;
    filament::VertexBuffer* gridVb_ = nullptr;
    filament::IndexBuffer* gridIb_ = nullptr;
    filament::MaterialInstance* gridMi_ = nullptr;
    bool gridAttached_ = false;

    utils::Entity axesEntity_;
    filament::VertexBuffer* axesVb_ = nullptr;
    filament::IndexBuffer* axesIb_ = nullptr;
    filament::MaterialInstance* axesMi_ = nullptr;
    bool axesAttached_ = false;

    // Dynamic selection outlines, object icons, and transform gizmos (channel 4, no depth cull)
    utils::Entity overlayEntity_;
    filament::VertexBuffer* overlayVb_ = nullptr;
    filament::IndexBuffer* overlayIb_ = nullptr;
    filament::MaterialInstance* overlayMi_ = nullptr;

    // Optional wireframe shading overlay (depth-tested)
    utils::Entity wireframeEntity_;
    filament::VertexBuffer* wireframeVb_ = nullptr;
    filament::IndexBuffer* wireframeIb_ = nullptr;
    filament::MaterialInstance* wireframeMi_ = nullptr;

    // M3 Edit mode topology overlay & Face Orientation visualization
    utils::Entity editLineEntity_;
    filament::VertexBuffer* editLineVb_ = nullptr;
    filament::IndexBuffer* editLineIb_ = nullptr;
    filament::MaterialInstance* editLineMi_ = nullptr;

    utils::Entity editTriEntity_;
    filament::VertexBuffer* editTriVb_ = nullptr;
    filament::IndexBuffer* editTriIb_ = nullptr;
    filament::MaterialInstance* editTriMi_ = nullptr;

    std::unordered_map<uint64_t, GpuMeshNode> gpuNodes_;
    std::unordered_map<uint64_t, GpuLightNode> gpuLights_;
    uint64_t syncedSceneRevision_ = 0;
    int32_t width_ = 1;
    int32_t height_ = 1;
    std::string lastError_;

    // Offscreen final-render resources (RN-1..RN-6)
    filament::View* offscreenView_ = nullptr;
    filament::SwapChain* offscreenSwapChain_ = nullptr;
    filament::ColorGrading* offscreenColorGrading_ = nullptr;
    bool viewportAdaptiveQuality_ = true;

    void applyQualityToView(filament::View* targetView, const RenderQualityConfig& cfg, bool offscreen);
    void syncCameraForExtent(const NovaCoreHandle* core, int32_t w, int32_t h, float focusDist, bool allowPreviewSphere);
    void syncSceneForPass(const NovaCoreHandle* core, bool offscreenClean);
};

} // namespace nova

#endif // NOVA_FILAMENT_BACKEND_H
