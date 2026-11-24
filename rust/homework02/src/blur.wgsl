@group(0) @binding(0) var sourceMoments: texture_2d_array<f32>;
@group(0) @binding(1) var sourceSampler: sampler;

var<immediate> params: vec4f;

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vertexMain(@builtin(vertex_index) index: u32) -> VertexOut {
    let vertices = array<vec2f, 3>(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
    let position = vertices[index];
    return VertexOut(vec4f(position, 0.0, 1.0), position * vec2f(0.5, -0.5) + vec2f(0.5));
}

@fragment
fn fragmentMain(input: VertexOut) -> @location(0) vec4f {
    let layer = i32(params.x);
    let center = textureSampleLevel(sourceMoments, sourceSampler,
        input.uv, layer, 0.0).rg * 0.20416369;

    let nearOffset = params.yz * 1.40733341;
    let nearWeight = 0.30400536;
    let nearPair = (
        textureSampleLevel(sourceMoments, sourceSampler,
            input.uv + nearOffset, layer, 0.0).rg
        + textureSampleLevel(sourceMoments, sourceSampler,
            input.uv - nearOffset, layer, 0.0).rg
    ) * nearWeight;

    let farOffset = params.yz * 3.29421495;
    let farWeight = 0.09391280;
    let farPair = (
        textureSampleLevel(sourceMoments, sourceSampler,
            input.uv + farOffset, layer, 0.0).rg
        + textureSampleLevel(sourceMoments, sourceSampler,
            input.uv - farOffset, layer, 0.0).rg
    ) * farWeight;
    return vec4f(center + nearPair + farPair, 0.0, 0.0);
}
