struct ShadowFrame {
    viewProjection: mat4x4f,
    pointPosition: vec4f,
    params: vec4f,
}

struct Material {
    ambient: vec4f,
    diffuse: vec4f,
    specular: vec4f,
    params: vec4f,
}

struct VertexIn {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
}

struct VertexOut {
    @builtin(position) clipPosition: vec4f,
    @location(0) worldPosition: vec3f,
    @location(1) uv: vec2f,
}

@group(0) @binding(0) var<uniform> shadow: ShadowFrame;
@group(1) @binding(0) var<uniform> material: Material;
@group(1) @binding(1) var albedoMap: texture_2d<f32>;
@group(1) @binding(2) var opacityMap: texture_2d<f32>;
@group(1) @binding(3) var materialSampler: sampler;

@vertex
fn vertexMain(input: VertexIn) -> VertexOut {
    return VertexOut(
        shadow.viewProjection * vec4f(input.position, 1.0),
        input.position,
        input.uv,
    );
}

@fragment
fn fragmentMain(input: VertexOut) -> @location(0) vec4f {
    var depth = input.clipPosition.z;
    if shadow.params.x > 0.5 {
        depth = length(input.worldPosition - shadow.pointPosition.xyz) / shadow.params.y;
    }
    let depthDx = dpdx(depth);
    let depthDy = dpdy(depth);
    if material.params.w > 0.5 {
        let opacityTexel = textureSample(opacityMap, materialSampler, input.uv);
        let opacity = min(opacityTexel.r, opacityTexel.a);
        if opacity < 0.5 {
            discard;
        }
    }
    return vec4f(depth, depth * depth + 0.25 * (depthDx * depthDx + depthDy * depthDy), 0.0, 0.0);
}
