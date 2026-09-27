struct VertexIn {
    @location(0) position: vec2f,
    @location(1) color: vec4f,
    @location(2) distance: f32,
}

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
    @location(1) distance: f32,
}

struct Immediates {
    view: mat4x4f,
    is_dotted: u32,
    time: f32,
}

var<immediate> immediates: Immediates;

@vertex
fn vertexMain(in: VertexIn) -> VertexOut {
    return VertexOut(
        immediates.view * vec4f(in.position, 0.0, 1.0),
        in.color,
        in.distance,
    );
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    if immediates.is_dotted == 0u {
        return in.color;
    }

    let dash_length = 100.0;
    let gap_length = 100.0;
    let speed = 200.0;
    let period = dash_length + gap_length;

    let phase = fract((in.distance - immediates.time * speed) / period) * period;

    if phase < dash_length {
        discard;
    }

    return in.color;
}