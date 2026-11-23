struct Camera {
    viewProjection: mat4x4f,
    inverseViewProjection: mat4x4f,
    position: vec3f,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var linearSampler: sampler;
@group(0) @binding(5) var environmentTexture: texture_2d<f32>;

const PI: f32 = 3.141592653589793;
const QUAD = array<vec2f, 6>(
    vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
    vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0)
);

struct VertexOut {
    @builtin(position) clip_position: vec4f,
    @location(0) ndc: vec2f,
}

@vertex
fn vertexMain(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    let position = QUAD[vertex_index];
    return VertexOut(vec4f(position, 0.0, 1.0), position);
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    let far = camera.inverseViewProjection * vec4f(in.ndc, 1.0, 1.0);
    let world_position = far.xyz / far.w;
    let dir = normalize(world_position - camera.position);
    let environment_uv = vec2f(
        atan2(dir.z, dir.x) / PI * 0.5 + 0.5,
        -atan2(dir.y, length(dir.xz)) / PI + 0.5
    );
    return vec4f(textureSample(environmentTexture, linearSampler, environment_uv).rgb, 1.0);
}
