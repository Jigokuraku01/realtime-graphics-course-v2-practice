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
    @location(2) color: vec3f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) world_position: vec3f,
    @location(1) world_normal: vec3f,
    @location(2) color: vec3f,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_position = object.model * vec4f(input.position, 1.0);
    output.clip_position = scene.view_projection * world_position;
    output.world_position = world_position.xyz;
    output.world_normal = normalize((object.model * vec4f(input.normal, 0.0)).xyz);
    output.color = input.color * object.color.rgb;
    return output;
}

@fragment
fn fs_sun(input: VertexOutput) -> @location(0) vec4f {
    return vec4f(input.color, 1.0);
}

@fragment
fn fs_planet(input: VertexOutput) -> @location(0) vec4f {
    let light_direction = normalize(-input.world_position);
    let diffuse = max(dot(normalize(input.world_normal), light_direction), 0.0);
    let ambient = 0.08;
    return vec4f(input.color * (ambient + diffuse), 1.0);
}
