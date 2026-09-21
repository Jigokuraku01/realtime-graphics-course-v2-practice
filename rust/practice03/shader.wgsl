struct VertexIn {
    @location(0) position: vec2f,
    @location(1) color: vec4f,
}

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
}

var<immediate> view: mat4x4f;

@vertex
fn vertexMain(in: VertexIn) -> VertexOut {
    return VertexOut(
        view * vec4f(in.position, 0.0, 1.0),
        in.color
    );
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    return in.color;
}
