struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
}

const POSITIONS = array<vec2f, 3>(
    vec2f(0.0, 1.0),
    vec2f(-sqrt(0.75), -0.5),
    vec2f( sqrt(0.75), -0.5),
);

const COLORS = array<vec4f, 3>(
    vec4f(1.00, 0.29, 0.29, 1.0),
    vec4f(0.16, 0.72, 0.79, 1.0),
    vec4f(1.00, 0.84, 0.40, 1.0),
);

@vertex
fn vertexMain(@builtin(vertex_index) vertexIndex: u32) -> VertexOut {
    return VertexOut(
        vec4f(POSITIONS[vertexIndex], 0.0, 1.0),
        COLORS[vertexIndex]
    );
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    return in.color;
}
