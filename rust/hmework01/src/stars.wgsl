struct StarInput {
    @location(0) direction: vec3f,
    @location(1) brightness: f32,
    @location(2) size_pixels: f32,
}

struct StarOutput {
    @builtin(position) position: vec4f,
    @location(0) brightness: f32,
}

struct Camera {
    right: vec4f,
    up: vec4f,
    forward: vec4f,
    projection: vec4f,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(input: StarInput, @builtin(vertex_index) vertex_index: u32) -> StarOutput {
    let corners = array<vec2f, 6>(
            vec2f(-1.0, -1.0),
            vec2f(1.0, -1.0),
            vec2f(-1.0, 1.0),
            vec2f(-1.0, 1.0),
            vec2f(1.0, -1.0),
            vec2f(1.0, 1.0),
        );
    var output: StarOutput;
    let w = dot(input.direction, camera.forward.xyz);
    let center = vec2f(
        dot(input.direction, camera.right.xyz),
        dot(input.direction, camera.up.xyz),
    ) * camera.projection.xy;
    let offset = corners[vertex_index] * input.size_pixels * camera.projection.zw * w;
    output.position = vec4f(center + offset, 0.0, w);
    output.brightness = input.brightness;
    return output;
}

@fragment
fn fs_main(input: StarOutput) -> @location(0) vec4f {
    return vec4f(vec3f(input.brightness), 1.0);
}
