#ifndef NOVA_RENDER_BACKEND_H
#define NOVA_RENDER_BACKEND_H

#include <android/native_window.h>
#include <cstdint>
#include <string>
#include <vector>

#include "nova_core.h"

namespace nova {

enum class GraphicsBackendPreference : int32_t {
    AutoVulkanFirst = 0,
    OpenGLESOnly = 1,
};

/**
 * Quality and post-processing parameters for viewport and offline rendering (RN-2, RN-4).
 * Shared behind RenderBackend so a future GPU path-tracer (RN-9) consumes the exact same contract.
 */
struct RenderQualityConfig {
    int32_t qualityLevel = 2;       // 0=Low, 1=Medium, 2=High, 3=Ultra
    bool aaEnabled = true;          // FXAA / MSAA enable
    uint8_t msaaSamples = 4;        // 1, 2, 4
    bool ssaoEnabled = true;        // Screen-space Ambient Occlusion
    bool bloomEnabled = true;       // Subtle HDR bloom
    bool dofEnabled = false;        // Depth of Field (RN-4)
    float dofFocusDistance = 4.0f;  // Focal plane in meters
    float dofMaxAperture = 0.015f;  // Aperture diameter in meters
    float dofCocScale = 1.0f;       // Circle-of-confusion scale
    int32_t toneMapping = 0;        // 0=ACES, 1=Filmic, 2=PBRNeutral
};

/**
 * Abstract render backend interface (PRD Section 10 / RN-9).
 * Allows Filament (Vulkan / OpenGL ES) in v1.0 and future backends such as an offline path tracer.
 */
class RenderBackend {
public:
    virtual ~RenderBackend() = default;

    virtual bool initialize(GraphicsBackendPreference preference) = 0;
    virtual void shutdown() = 0;

    virtual bool attachSurface(ANativeWindow* window, int32_t width, int32_t height) = 0;
    virtual void resizeSurface(int32_t width, int32_t height) = 0;
    virtual void detachSurface() = 0;

    virtual bool renderFrame(const NovaCoreHandle* core, uint64_t frameTimeNanos) = 0;

    // Viewport dynamic resolution & final offline rendering (M6: RN-1..RN-9)
    virtual void setViewportAdaptiveQuality(bool enabled) = 0;
    virtual bool renderOffscreenStillRgba(const NovaCoreHandle* core,
                                          uint32_t width,
                                          uint32_t height,
                                          const RenderQualityConfig& cfg,
                                          std::vector<uint8_t>& outRgba,
                                          std::string& outError) = 0;
    virtual bool beginOffscreenSurface(ANativeWindow* window,
                                       int32_t width,
                                       int32_t height,
                                       const RenderQualityConfig& cfg,
                                       std::string& outError) = 0;
    virtual bool renderOffscreenSurfaceFrame(const NovaCoreHandle* core,
                                             uint64_t frameTimeNanos,
                                             int32_t width,
                                             int32_t height) = 0;
    virtual void endOffscreenSurface() = 0;

    virtual std::string activeBackendName() const = 0;
    virtual std::string lastError() const = 0;
    virtual bool isInitialized() const = 0;
    virtual bool hasSurface() const = 0;
};

} // namespace nova

#endif // NOVA_RENDER_BACKEND_H
