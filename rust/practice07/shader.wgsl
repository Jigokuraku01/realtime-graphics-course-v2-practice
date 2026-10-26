struct VertexIn {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) texcoord: vec2f,
}

struct VertexOut {
    @builtin(position) clip_position: vec4f,
    @location(0) world_position: vec3f,
    @location(1) normal: vec3f,
}

struct CameraUniforms {
    viewProjection: mat4x4f,
    position: vec3f,
}

struct LightsUniforms {
    ambientIntensity: vec3f,
    dirLightDirection: vec3f,
    dirLightIntensity: f32,
    lightPosition: vec3f,
    brightness: f32,
    attenuation: vec3f,
}

struct Immediates {
    model: mat4x4f,
    albedo: vec3f,
    roughness: f32,
    specular_factor: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;
@group(1) @binding(0)
var<uniform> lights: LightsUniforms;

var<immediate> immediates: Immediates;

@vertex
fn vertexMain(in: VertexIn) -> VertexOut {
    let world_position = immediates.model * vec4f(in.position, 1.0);
    return VertexOut(
            camera.viewProjection * world_position,
            world_position.xyz,
            normalize((immediates.model * vec4f(in.normal, 0.0)).xyz),
        );
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    let normal = normalize(in.normal);
    let to_light = lights.lightPosition - in.world_position;
    let distance = length(to_light);
    let attenuation_factor = 1.0
        / (lights.attenuation.x + lights.attenuation.y * distance
            + lights.attenuation.z * distance * distance);
    let point_dir = to_light / max(distance, 0.0001);
    let to_camera = camera.position - in.world_position;
    let view_dir = to_camera / max(length(to_camera), 0.0001);

    let roughness = clamp(immediates.roughness, 0.05, 1.0);
    let power = 1.0 / (roughness * roughness) - 1.0;
    let specular_normalization = (power + 2.0) / (2.0 * 3.14159265);

    let dir_diffuse = max(0.0, dot(normal, lights.dirLightDirection));
    let half_dir_vector = lights.dirLightDirection + view_dir;
    let half_dir = half_dir_vector / max(length(half_dir_vector), 0.0001);
    let dir_specular = immediates.specular_factor * specular_normalization
        * pow(max(0.0, dot(normal, half_dir)), power);

    let point_diffuse = max(0.0, dot(normal, point_dir));
    let half_point_vector = point_dir + view_dir;
    let half_point = half_point_vector / max(length(half_point_vector), 0.0001);
    let point_specular = immediates.specular_factor * specular_normalization
        * pow(max(0.0, dot(normal, half_point)), power);

    let color = immediates.albedo * (
        lights.ambientIntensity
        + lights.dirLightIntensity * dir_diffuse * (1.0 + dir_specular)
        + lights.brightness * attenuation_factor * point_diffuse * (1.0 + point_specular)
    );

    return vec4f(color, 1.0);
}
