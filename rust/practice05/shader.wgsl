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
    var position = in.position;
    let beat = immediates.time * 2.0 * 6.2831853;
    let side_phase = select(3.1415927, 0.0, in.position.x >= 0.0);
    let step = sin(beat + side_phase);
    let foot_weight = (1.0 - smoothstep(-0.48, -0.12, in.position.y))
        * smoothstep(0.10, 0.22, abs(in.position.x));
    position.y += foot_weight * 0.12 * max(step, 0.0);
    position.z += foot_weight * 0.10 * step;

    return VertexOut(
            uniforms.viewProjection * uniforms.model * vec4f(position, 1.0),
            normalize((uniforms.model * vec4f(in.normal, 0.0)).xyz),
            in.texcoord,
        );
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    return textureSample(cowTexture, cowSampler, in.texcoord);
}
