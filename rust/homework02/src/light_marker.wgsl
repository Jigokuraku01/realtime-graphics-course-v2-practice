struct Frame {
    viewProjection: mat4x4f,
    sunProjection: mat4x4f,
    cameraPosition: vec4f,
    ambient: vec4f,
    sunDirection: vec4f,
    sunColor: vec4f,
    pointPositionRange: vec4f,
    pointColor: vec4f,
}

@group(0) @binding(0) var<uniform> frame: Frame;

@vertex
fn vertexMain(@location(0) localPosition: vec3f) -> @builtin(position) vec4f {
    let radius = frame.pointPositionRange.w * 0.0025;
    let worldPosition = frame.pointPositionRange.xyz + localPosition * radius;
    return frame.viewProjection * vec4f(worldPosition, 1.0);
}

@fragment
fn fragmentMain() -> @location(0) vec4f {
    return vec4f(1.0, 0.82, 0.42, 1.0);
}
