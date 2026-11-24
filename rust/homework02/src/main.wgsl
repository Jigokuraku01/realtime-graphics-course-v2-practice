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

struct Material {
    ambient: vec4f,
    diffuse: vec4f,
    specular: vec4f,
    params: vec4f,
}

struct VertexIn {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
}

struct VertexOut {
    @builtin(position) clipPosition: vec4f,
    @location(0) worldPosition: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
}

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var sunMoments: texture_2d<f32>;
@group(0) @binding(2) var pointMoments: texture_cube<f32>;
@group(0) @binding(3) var momentSampler: sampler;

@group(1) @binding(0) var<uniform> material: Material;
@group(1) @binding(1) var albedoMap: texture_2d<f32>;
@group(1) @binding(2) var opacityMap: texture_2d<f32>;
@group(1) @binding(3) var materialSampler: sampler;

const PI: f32 = 3.14159265359;
const BLEED_REDUCTION: f32 = 0.2;

fn linearToSrgb(linear: vec3f) -> vec3f {
    let color = max(linear, vec3f(0.0));
    return select(
        1.055 * pow(color, vec3f(1.0 / 2.4)) - 0.055,
        12.92 * color,
        color <= vec3f(0.0031308),
    );
}

@vertex
fn vertexMain(input: VertexIn) -> VertexOut {
    return VertexOut(
        frame.viewProjection * vec4f(input.position, 1.0),
        input.position,
        input.normal,
        input.uv,
    );
}

fn visibility(moments: vec2f, receiverDepth: f32) -> f32 {
    let distance = receiverDepth - moments.x - 0.00025;
    if distance <= 0.0 {
        return 1.0;
    }
    let variance = max(moments.y - moments.x * moments.x, 0.0);
    let bound = variance / max(variance + distance * distance, 0.000001);
    return clamp((bound - BLEED_REDUCTION) / (1.0 - BLEED_REDUCTION), 0.0, 1.0);
}

fn sunVisibility(position: vec3f) -> f32 {
    let projected = frame.sunProjection * vec4f(position, 1.0);
    let ndc = projected.xyz / projected.w;
    if any(abs(ndc.xy) > vec2f(1.0)) || ndc.z < 0.0 || ndc.z > 1.0 {
        return 1.0;
    }
    let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
    return visibility(textureSampleLevel(sunMoments, momentSampler, uv, 0.0).rg, ndc.z);
}

fn pointVisibility(position: vec3f) -> f32 {
    let relative = position - frame.pointPositionRange.xyz;
    let distance = length(relative);
    if distance >= frame.pointPositionRange.w {
        return 1.0;
    }
    return visibility(
        textureSampleLevel(pointMoments, momentSampler, relative, 0.0).rg,
        distance / frame.pointPositionRange.w,
    );
}

fn phong(normal: vec3f, lightDirection: vec3f, viewDirection: vec3f,
         intensity: vec3f, albedo: vec3f) -> vec3f {
    let cosine = max(dot(normal, lightDirection), 0.0);
    if cosine <= 0.0 {
        return vec3f(0.0);
    }
    let power = max(material.params.x, 1.0);
    let reflected = reflect(-lightDirection, normal);
    let specular = material.specular.rgb * ((power + 2.0) / (2.0 * PI))
        * pow(max(dot(reflected, viewDirection), 0.0), power);
    return intensity * (albedo + specular) * cosine;
}

@fragment
fn fragmentMain(input: VertexOut) -> @location(0) vec4f {
    if material.params.w > 0.5 {
        let opacityTexel = textureSample(opacityMap, materialSampler, input.uv);
        let opacity = min(opacityTexel.r, opacityTexel.a);
        if opacity < 0.5 {
            discard;
        }
    }

    var albedo = material.diffuse.rgb;
    if material.params.y > 0.5 {
        albedo = textureSample(albedoMap, materialSampler, input.uv).rgb;
    }

    let normal = normalize(input.normal);
    let viewDirection = normalize(frame.cameraPosition.xyz - input.worldPosition);
    let sun = phong(normal, normalize(frame.sunDirection.xyz), viewDirection,
        frame.sunColor.rgb, albedo) * sunVisibility(input.worldPosition);
    let pointVector = frame.pointPositionRange.xyz - input.worldPosition;
    let distance = length(pointVector);
    let pointDirection = pointVector / max(distance, 0.0001);
    let attenuation = 1.0 / (1.0 + 2.0 * distance / frame.pointPositionRange.w
        + 2.0 * distance * distance / (frame.pointPositionRange.w * frame.pointPositionRange.w));
    let point = phong(normal, pointDirection, viewDirection,
        frame.pointColor.rgb * attenuation, albedo) * pointVisibility(input.worldPosition);
    let color = albedo * frame.ambient.rgb + sun + point;
    let exposed = max(color, vec3f(0.0)) * 1.4;
    let mapped = exposed / (vec3f(1.0) + exposed);
    return vec4f(linearToSrgb(mapped), 1.0);
}
