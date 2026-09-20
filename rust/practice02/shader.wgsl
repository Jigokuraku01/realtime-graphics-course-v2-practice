struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
}

struct Immediates {
    transform: mat4x4f,
    view: mat4x4f,
}

const POSITIONS = array<vec2f, 18>(
    vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(1.0 / 2.0, sqrt(3.0) / 2.0),
    vec2f(0.0, 0.0), vec2f(1.0 / 2.0, sqrt(3.0) / 2.0), vec2f(-1.0 / 2.0, sqrt(3.0) / 2.0),
    vec2f(0.0, 0.0), vec2f(-1.0 / 2.0, sqrt(3.0) / 2.0), vec2f(-1.0, 0.0),
    vec2f(0.0, 0.0), vec2f(-1.0, 0.0), vec2f(-1.0 / 2.0, -sqrt(3.0) / 2.0),
    vec2f(0.0, 0.0), vec2f(-1.0 / 2.0, -sqrt(3.0) / 2.0), vec2f(1.0 / 2.0, -sqrt(3.0) / 2.0),
    vec2f(0.0, 0.0), vec2f(1.0 / 2.0, -sqrt(3.0) / 2.0), vec2f(1.0, 0.0),
);

const COLORS = array<vec4f, 18>(
    vec4f(1.00, 0.29, 0.29, 1.0), vec4f(1.00, 0.29, 0.29, 1.0), vec4f(0.16, 0.72, 0.79, 1.0),
    vec4f(1.00, 0.29, 0.29, 1.0), vec4f(0.16, 0.72, 0.79, 1.0), vec4f(0.16, 0.72, 0.79, 1.0),
    vec4f(1.00, 0.29, 0.29, 1.0), vec4f(0.16, 0.72, 0.79, 1.0), vec4f(1.00, 0.84, 0.40, 1.0),
    vec4f(1.00, 0.29, 0.29, 1.0), vec4f(1.00, 0.84, 0.40, 1.0), vec4f(1.00, 0.84, 0.40, 1.0),
    vec4f(1.00, 0.29, 0.29, 1.0), vec4f(1.00, 0.84, 0.40, 1.0), vec4f(1.00, 0.29, 0.29, 1.0),
    vec4f(1.00, 0.29, 0.29, 1.0), vec4f(1.00, 0.29, 0.29, 1.0), vec4f(1.00, 0.29, 0.29, 1.0),
);

var<immediate> immediates: Immediates;

@vertex
fn vertexMain(@builtin(vertex_index) vertexIndex: u32) -> VertexOut {
    let pos2d = POSITIONS[vertexIndex];
    let pos4d = vec4f(pos2d.xy, 0.0, 1.0);
    let rotated = immediates.transform * pos4d;
    let actual = immediates.view * rotated;
    return VertexOut(
        actual,
        COLORS[vertexIndex]
    );
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    return in.color;
}
