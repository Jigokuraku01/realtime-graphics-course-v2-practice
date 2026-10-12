struct VertexIn {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) texcoord: vec2f,
}

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) normal: vec3f,
    @location(1) texcoord: vec2f,
}

struct Immediates {
    time: f32,
}

struct Uniforms {
    model: mat4x4f,
    viewProjection: mat4x4f,
}

var<immediate> immediates: Immediates;

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var cowTexture: texture_2d<f32>;
@group(0) @binding(2)
var cowSampler: sampler;

@vertex
fn vertexMain(in: VertexIn) -> VertexOut {
    return VertexOut(
            uniforms.viewProjection * uniforms.model * vec4f(in.position, 1.0),
            normalize((uniforms.model * vec4f(in.normal, 0.0)).xyz),
            in.texcoord,
        );
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
        let offset = vec2f(immediates.time * 0.1, 0.0);
    let uv = in.texcoord + offset;

    return textureSample(cowTexture, cowSampler, uv);
}
