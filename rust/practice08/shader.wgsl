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
    lightDirection: vec3f,
    lightIntensity: vec3f,
    shadowProjection: mat4x4f,
}

struct Immediates {
    model: mat4x4f,
    albedo: vec3f,
    roughness: f32,
    specularFactor: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;
@group(1) @binding(0)
var<uniform> lights: LightsUniforms;
@group(1) @binding(1)
var shadowMap: texture_depth_2d;
@group(1) @binding(2)
var shadowMapSampler: sampler;
@group(1) @binding(3)
var shadowCompareSampler: sampler_comparison;

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

const PI = 3.1415926535;

fn computeLight(
    normal: vec3f,
    lightDirection: vec3f,
    lightIntensity: vec3f,
    viewDirection: vec3f,
) -> vec3f {
    let halfway = normalize(lightDirection + viewDirection);
    let power = 1.0 / max(0.0001, immediates.roughness * immediates.roughness) - 1.0;
    return max(0.0, dot(normal, lightDirection))
        * (1.0
            + immediates.specularFactor * (power + 2.0) / (2.0 * PI)
            * pow(max(0.0, dot(halfway, normal)), power)) * lightIntensity;
}

const SHADOW_BIAS = 0.001;

fn shadowVisibility(worldPosition: vec3f) -> f32 {
    let projected = lights.shadowProjection * vec4f(worldPosition, 1.0);
    let shadowCoord = projected.xyz / projected.w;
    let uv = vec2f(0.5 + 0.5 * shadowCoord.x, 0.5 - 0.5 * shadowCoord.y);

    let visibility = textureSampleCompare(
            shadowMap,
            shadowCompareSampler,
            uv,
            shadowCoord.z - SHADOW_BIAS,
        );
    let insideMap =
        projected.w > 0.0 && all(shadowCoord.xy >= vec2f(-1.0)) && all(shadowCoord.xy <= vec2f(1.0))
        && shadowCoord.z >= 0.0 && shadowCoord.z <= 1.0;
    return select(1.0, visibility, insideMap);
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    let normal = normalize(in.normal);

    let viewDirection = normalize(camera.position - in.world_position);
    let halfway = normalize(lights.lightDirection + viewDirection);
    let specularPower = 1.0 / max(0.0001, immediates.roughness * immediates.roughness) - 1.0;
    let cosine = max(0.0, dot(normal, lights.lightDirection));

    let directLight = immediates.albedo * lights.lightIntensity * cosine
        + lights.lightIntensity * immediates.specularFactor * (specularPower + 2.0) / (2.0 * PI)
        * pow(max(0.0, dot(halfway, normal)), specularPower);
    let color = immediates.albedo * lights.ambientIntensity
        + shadowVisibility(in.world_position) * directLight;

    return vec4f(color, 1.0);
}
