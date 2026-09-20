@vertex
fn vertexMain(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4f {
    var position = array<vec2f, 3>(
        vec2f(0.0, 0.5),
        vec2f(-0.5, -0.5),
        vec2f(0.5, -0.5),
    );
    return vec4f(position[vertex_index], 0.0, 1.0);
}

@fragment
fn fragmentMain(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let cell_size = 30.0;
    let ix = floor(position.x / cell_size);
    let iy = floor(position.y / cell_size);
    let is_black = (i32(ix) + i32(iy)) % 2 == 0;
    if is_black {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    } else {
        return vec4f(1.0, 1.0, 1.0, 1.0);
    }
}