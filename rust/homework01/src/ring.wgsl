struct SceneUniform {
    view_projection: mat4x4f,
    camera_position: vec4f,
};

struct ObjectUniform {
    model: mat4x4f,
    color: vec4f,
};

@group(0) @binding(0) var<uniform> scene: SceneUniform;
@group(1) @binding(0) var<uniform> object: ObjectUniform;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) radial: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) world_position: vec3f,
    @location(1) world_normal: vec3f,
    @location(2) radial: f32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_position = object.model * vec4f(input.position, 1.0);
    output.clip_position = scene.view_projection * world_position;
    output.world_position = world_position.xyz;
    output.world_normal = normalize((object.model * vec4f(input.normal, 0.0)).xyz);
    output.radial = input.radial;
    return output;
}

// этот про тень планеты на кольцо
@fragment
fn fs_ring(input: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4f {
    let side = select(-1.0, 1.0, front_facing);
    let normal = normalize(input.world_normal) * side;
    let light_direction = normalize(-input.world_position);
    let to_planet = object.model[3].xyz - input.world_position;
    let along_light = dot(to_planet, light_direction);
    let closest = to_planet - along_light * light_direction;
    let planet_radius = length(object.model[0].xyz);
    let in_shadow = along_light > 0.0 && along_light < length(input.world_position)
        && dot(closest, closest) < planet_radius * planet_radius;
    let diffuse = select(max(dot(normal, light_direction), 0.0), 0.0, in_shadow);
    let ambient = 0.12;
    return vec4f(object.color.rgb * (ambient + diffuse), object.color.a);
}
 
// этот про ореол вокруг Солнца
@fragment
fn fs_glow(input: VertexOutput) -> @location(0) vec4f {
    let fade = 1.0 - smoothstep(0.0, 1.0, input.radial);
    let alpha = object.color.a * fade * fade;
    return vec4f(object.color.rgb, alpha);
}
