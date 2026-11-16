struct VertexIn {
    @builtin(vertex_index) vertexIndex: u32,
    @location(0) position: vec3f,
}

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) texcoord: vec2f,
}

struct Camera {
    viewProjection: mat4x4f,
    position: vec3f,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var particleTexture: texture_2d<f32>;
@group(0) @binding(2) var particleSampler: sampler;

const EXTENT = 0.05;

const QUAD_VERTICES = array<vec2f, 6>(
    vec2f(-1.0, -1.0),
    vec2f( 1.0, -1.0),
    vec2f(-1.0,  1.0),
    vec2f(-1.0,  1.0),
    vec2f( 1.0, -1.0),
    vec2f( 1.0,  1.0),
);

@vertex
fn vertexMain(in: VertexIn) -> VertexOut {
    let vertex = QUAD_VERTICES[in.vertexIndex];
    let position = in.position + vec3f(vertex * EXTENT, 0.0);
    return VertexOut(
        camera.viewProjection * vec4f(position, 1.0),
        vertex * 0.5 + vec2f(0.5)
    );
}

@fragment
fn fragmentMain() -> @location(0) vec4f {
    return vec4f(1.0, 1.0, 1.0, 1.0);
}
