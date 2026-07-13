struct Params {
    grid_width: u32,
    grid_height: u32,
    baseline_total: f32,
    active_total: f32,
    fixed_point_scale: u32,
    padding1: u32,
    padding2: u32,
    padding3: u32,
    source_x_min: f32,
    source_x_max: f32,
    source_y_min: f32,
    source_y_max: f32,
    display_x_min: f32,
    display_x_max: f32,
    display_y_min: f32,
    display_y_max: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var<storage, read> baseline_counts: array<u32>;
@group(0) @binding(1) var<storage, read> active_counts: array<u32>;
@group(0) @binding(2) var<storage, read> max_abs_fixed: array<u32>;
@group(0) @binding(3) var<uniform> params: Params;
@group(0) @binding(4) var palette_lut: texture_2d<f32>;
@group(0) @binding(5) var palette_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = vec2<f32>(
        position.x * 0.5 + 0.5,
        0.5 - position.y * 0.5,
    );
    return output;
}

fn source_uv_for_display(display_uv: vec2<f32>) -> vec3<f32> {
    let data_x = params.display_x_min + display_uv.x * (params.display_x_max - params.display_x_min);
    let data_y = params.display_y_max - display_uv.y * (params.display_y_max - params.display_y_min);
    let source_u = (data_x - params.source_x_min) / (params.source_x_max - params.source_x_min);
    let source_v = (params.source_y_max - data_y) / (params.source_y_max - params.source_y_min);
    let covered = source_u >= 0.0 && source_u <= 1.0 && source_v >= 0.0 && source_v <= 1.0;
    return vec3<f32>(source_u, source_v, select(0.0, 1.0, covered));
}

fn diverging_colour(value: f32) -> vec3<f32> {
    return textureSampleLevel(
        palette_lut,
        palette_sampler,
        vec2<f32>((clamp(value, -1.0, 1.0) + 1.0) * 0.5, 2.5 / 3.0),
        0.0,
    ).rgb;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let display_uv = clamp(input.uv, vec2<f32>(0.0), vec2<f32>(0.999999));
    let projected = source_uv_for_display(display_uv);
    if projected.z < 0.5 || params.baseline_total <= 0.0 || params.active_total <= 0.0 {
        return vec4<f32>(diverging_colour(0.0), 1.0);
    }
    let uv = clamp(projected.xy, vec2<f32>(0.0), vec2<f32>(0.999999));
    let x_bin = min(u32(uv.x * f32(params.grid_width)), params.grid_width - 1u);
    let y_bin = min(u32((1.0 - uv.y) * f32(params.grid_height)), params.grid_height - 1u);
    let index = y_bin * params.grid_width + x_bin;
    let baseline_share = f32(baseline_counts[index]) / params.baseline_total;
    let active_share = f32(active_counts[index]) / params.active_total;
    let max_abs = f32(max_abs_fixed[0]) / f32(params.fixed_point_scale);
    let normalized = select(0.0, clamp((active_share - baseline_share) / max_abs, -1.0, 1.0), max_abs > 0.0);
    return vec4<f32>(diverging_colour(normalized), 1.0);
}
