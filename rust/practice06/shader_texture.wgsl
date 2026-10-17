@group(0) @binding(0)
var input_texture: texture_2d<f32>;
@group(0) @binding(1)
var linear_sampler: sampler;
@group(0) @binding(2)
var nearest_sampler: sampler;

struct Immediates {
    center: vec2f,
    extent: vec2f,
    mode: i32,
    time: f32,
}

var<immediate> immediates: Immediates;

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) texcoord: vec2f,
}

const VERTICES = array<vec2f, 6>(
        vec2f(-1.0, -1.0),
        vec2f(1.0, -1.0),
        vec2f(-1.0, 1.0),
        vec2f(-1.0, 1.0),
        vec2f(1.0, -1.0),
        vec2f(1.0, 1.0),
    );

@vertex
fn vertexMain(@builtin(vertex_index) index: u32) -> VertexOut {
    let vertex = VERTICES[index];

    return VertexOut(
            vec4f(immediates.center + immediates.extent * vertex, 0.0, 1.0),
            vec2f(0.5) + vec2f(0.5, -0.5) * vertex,
        );
}

fn gaussianBlur(in: VertexOut) -> vec4f {
    const N = 10;
    const sigma = 6.0;
    var sum = vec4f(0.0);
    var sum_weights = 0.0;
    let size = vec2f(textureDimensions(input_texture, 0));
    for(var x = -N; x <= N; x += 1) {
        for(var y = -N; y <= N; y += 1) {
            let offset = vec2f(vec2i(x, y));
            let weight = exp(-dot(offset, offset) / 2.0 / (sigma * sigma));
            sum += textureSampleLevel(
                    input_texture,
                    nearest_sampler,
                    in.texcoord + offset / size,
                    0,
                ) * weight;
            sum_weights += weight;
        }
    }
    return sum / sum_weights;
}

@fragment
fn fragmentMain(in: VertexOut) -> @location(0) vec4f {
    if immediates.mode == 3 {
        return gaussianBlur(in);
    }
    var texcoord = in.texcoord;
    if immediates.mode == 2 {
        texcoord += vec2(sin(in.texcoord.y * 50.0 + immediates.time) * 0.01, 0.0);
    }
    let sampled = textureSample(input_texture, linear_sampler, texcoord);
    var color = sampled.rgb;
    if immediates.mode == 1 {
        color = floor(color * 4.0) / 3.0;
    }
    return vec4f(color, sampled.a);
}
