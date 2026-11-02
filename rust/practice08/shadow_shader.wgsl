struct VertexIn {
    @location(0) position: vec3f,
}

struct ShadowImmediates {
    shadowProjection: mat4x4f,
}

var<immediate> immediates: ShadowImmediates;

@vertex
fn vertexMain(in: VertexIn) -> @builtin(position) vec4f {
    return immediates.shadowProjection * vec4f(in.position, 1.0);
}
