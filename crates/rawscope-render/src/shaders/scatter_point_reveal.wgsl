struct RevealPoint {
    x: f32,
    y: f32,
    row_id_low: u32,
    row_id_high: u32,
};

struct Params {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    plot_width_px: f32,
    plot_height_px: f32,
    radius_px: f32,
    point_alpha: f32,
    emphasized_low: u32,
    emphasized_high: u32,
    has_emphasis: u32,
    padding: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) emphasized: f32,
};

@group(0) @binding(0)
var<storage, read> points: array<RevealPoint>;

@group(0) @binding(1)
var<uniform> params: Params;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );
    let point = points[instance_index];
    let corner = corners[vertex_index];
    let x_fraction = (point.x - params.x_min) / (params.x_max - params.x_min);
    let y_fraction = (params.y_max - point.y) / (params.y_max - params.y_min);
    let centre = vec2<f32>(x_fraction * 2.0 - 1.0, 1.0 - y_fraction * 2.0);
    let radius_clip = vec2<f32>(
        params.radius_px * 2.0 / params.plot_width_px,
        params.radius_px * 2.0 / params.plot_height_px,
    );
    let is_emphasized = params.has_emphasis == 1u
        && point.row_id_low == params.emphasized_low
        && point.row_id_high == params.emphasized_high;
    var output: VertexOutput;
    output.position = vec4<f32>(centre + corner * radius_clip, 0.0, 1.0);
    output.local = corner;
    output.emphasized = select(0.0, 1.0, is_emphasized);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(input.local);
    if distance > 1.0 {
        discard;
    }
    let edge = 1.0 - smoothstep(0.72, 1.0, distance);
    let neutral = vec3<f32>(0.78, 0.92, 0.96);
    let accent = vec3<f32>(1.0, 0.72, 0.24);
    let colour = mix(neutral, accent, input.emphasized);
    let alpha = mix(0.28, 0.82, edge) * params.point_alpha;
    return vec4<f32>(colour, alpha);
}
