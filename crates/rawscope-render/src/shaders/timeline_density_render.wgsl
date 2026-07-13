struct RenderParams {
    grid_width: u32,
    grid_height: u32,
    lane_count: u32,
    max_bin_count: u32,
    transform_id: u32,
    palette_id: u32,
    padding0: u32,
    padding1: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var<storage, read> counts: array<u32>;

@group(0) @binding(1)
var<uniform> params: RenderParams;

@group(0) @binding(2)
var palette_lut: texture_2d<f32>;

@group(0) @binding(3)
var palette_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0)
    );

    let position = positions[vertex_index];

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    return output;
}

fn density_intensity(count: u32, max_count: u32, transform_id: u32) -> f32 {
    if count == 0u || max_count == 0u {
        return 0.0;
    }

    if transform_id == 0u {
        return clamp(f32(count) / f32(max_count), 0.0, 1.0);
    }

    let count_scale = log(f32(count) + 1.0);
    let max_count_scale = log(f32(max_count) + 1.0);
    return clamp(count_scale / max_count_scale, 0.0, 1.0);
}

fn density_colour(intensity: f32, palette_id: u32) -> vec3<f32> {
    return textureSampleLevel(
        palette_lut,
        palette_sampler,
        vec2<f32>(clamp(intensity, 0.0, 1.0), (f32(palette_id) + 0.5) / 3.0),
        0.0,
    ).rgb;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let clamped_uv = clamp(input.uv, vec2<f32>(0.0), vec2<f32>(0.999999));
    let x_bin = min(u32(clamped_uv.x * f32(params.grid_width)), params.grid_width - 1u);
    let lane_position = (1.0 - clamped_uv.y) * f32(params.lane_count);
    let lane = min(u32(lane_position), params.lane_count - 1u);
    let lane_fraction = fract(lane_position);
    let y_bin = min((lane * params.grid_height) / params.lane_count, params.grid_height - 1u);
    let bin_index = y_bin * params.grid_width + x_bin;
    let count = counts[bin_index];
    let intensity = density_intensity(count, params.max_bin_count, params.transform_id);
    let lane_gap = lane_fraction < 0.035 || lane_fraction > 0.965;
    if lane_gap {
        return vec4<f32>(0.008, 0.011, 0.022, 1.0);
    }
    if count == 0u {
        let alternating_background = select(
            vec3<f32>(0.012, 0.015, 0.030),
            vec3<f32>(0.016, 0.021, 0.040),
            lane % 2u == 1u,
        );
        return vec4<f32>(alternating_background, 1.0);
    }
    return vec4<f32>(density_colour(intensity, params.palette_id), 1.0);
}
