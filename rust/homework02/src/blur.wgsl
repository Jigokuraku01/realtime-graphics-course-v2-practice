@group(0) @binding(0) var sourceMoments: texture_2d_array<f32>;
@group(0) @binding(1) var sourceSampler: sampler;

// x = source layer, yz = one-pixel offset in the blur direction
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
    let weights = array<f32, 9>(
        0.02763055, 0.06628225, 0.12383154, 0.18017382, 0.20416369,
        0.18017382, 0.12383154, 0.06628225, 0.02763055,
    );
    var result = vec2f(0.0);
    for (var i = 0; i < 9; i++) {
        let offset = f32(i - 4) * params.yz;
        result += textureSampleLevel(sourceMoments, sourceSampler,
            input.uv + offset, i32(params.x), 0.0).rg * weights[i];
    }
    return vec4f(result, 0.0, 0.0);
}
