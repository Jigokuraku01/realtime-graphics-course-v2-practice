struct VertexIn {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) texcoord: vec2f,
    @location(3) tangent: vec3f,
}

struct VertexOut {
    @builtin(position) clip_position: vec4f,
    @location(0) world_position: vec3f,
    @location(1) normal: vec3f,
    @location(2) texcoord: vec2f,
    @location(3) tangent: vec3f,
}

struct Camera {
    viewProjection: mat4x4f,
    inverseViewProjection: mat4x4f,
    position: vec3f,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var linearSampler: sampler;
@group(0) @binding(2) var albedoTexture: texture_2d<f32>;
@group(0) @binding(3) var normalTexture: texture_2d<f32>;
@group(0) @binding(4) var metalnessTexture: texture_2d<f32>;
@group(0) @binding(5) var environmentTexture: texture_2d<f32>;

const PI: f32 = 3.141592653589793;

@vertex
fn vertexMain(in: VertexIn) -> VertexOut {
    return VertexOut(
        camera.viewProjection * vec4f(in.position, 1.0),
        in.position,
        in.normal,
        in.texcoord,
        in.tangent
    );
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    let normal = normalize(in.normal);
    let tangent = normalize(in.tangent);
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3f(tangent, bitangent, normal);
    let real_normal = normalize(
        tbn * (textureSample(normalTexture, linearSampler, in.texcoord).rgb * 2.0 - vec3f(1.0))
    );

    let albedo = textureSample(albedoTexture, linearSampler, in.texcoord).rgb;
    let light_direction = normalize(vec3f(0.9, 0.4, 0.0));
    let color = (0.2 + 0.8 * max(0.0, dot(real_normal, light_direction))) * albedo;

    let direction_to_camera = normalize(camera.position - in.world_position);
    let dir = reflect(-direction_to_camera, real_normal);
    let environment_uv = vec2f(
        atan2(dir.z, dir.x) / PI * 0.5 + 0.5,
        -atan2(dir.y, length(dir.xz)) / PI + 0.5
    );
    let reflected_color = textureSample(environmentTexture, linearSampler, environment_uv).rgb;
    let metalness = textureSample(metalnessTexture, linearSampler, in.texcoord).r;
    return vec4f(mix(color, reflected_color, metalness), 1.0);
}
