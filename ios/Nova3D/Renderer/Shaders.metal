#include <metal_stdlib>
using namespace metal;

struct VertexInput {
    float3 position [[attribute(0)]];
    float3 normal   [[attribute(1)]];
    float2 uv       [[attribute(2)]];
    float4 color    [[attribute(3)]];
};

struct VertexOutput {
    float4 position [[position]];
    float3 worldPos;
    float3 normal;
    float2 uv;
    float4 color;
};

struct Uniforms {
    float4x4 modelMatrix;
    float4x4 viewProjectionMatrix;
    float3 cameraPosition;
};

vertex VertexOutput vertexShader(VertexInput in [[stage_in]],
                                 constant Uniforms &uniforms [[buffer(1)]]) {
    VertexOutput out;
    float4 worldPos = uniforms.modelMatrix * float4(in.position, 1.0);
    out.position = uniforms.viewProjectionMatrix * worldPos;
    out.worldPos = worldPos.xyz;
    out.normal = (uniforms.modelMatrix * float4(in.normal, 0.0)).xyz;
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

fragment float4 fragmentShader(VertexOutput in [[stage_in]]) {
    // Studio directional key light
    float3 lightDir = normalize(float3(0.55, 0.82, 0.45));
    float diff = max(dot(normalize(in.normal), lightDir), 0.15);
    float3 baseColor = in.color.rgb > 0.0 ? in.color.rgb : float3(0.65, 0.65, 0.68);
    float3 lit = baseColor * diff;
    return float4(lit, 1.0);
}

// Unlit vertex color shader for infinite grid, gizmo, and selection brackets
vertex VertexOutput unlitVertexShader(VertexInput in [[stage_in]],
                                      constant Uniforms &uniforms [[buffer(1)]]) {
    VertexOutput out;
    out.position = uniforms.viewProjectionMatrix * float4(in.position, 1.0);
    out.worldPos = in.position;
    out.normal = float3(0, 1, 0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

fragment float4 unlitFragmentShader(VertexOutput in [[stage_in]]) {
    return in.color;
}
