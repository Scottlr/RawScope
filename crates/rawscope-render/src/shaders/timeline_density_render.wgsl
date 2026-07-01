struct RenderParams {
    grid_width: u32,
    grid_height: u32,
    max_bin_count: u32,
    padding: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var<storage, read> counts: array<u32>;

@group(0) @binding(1)
var<uniform> params: RenderParams;

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

fn density_intensity(count: u32, max_count: u32) -> f32 {
    if count == 0u || max_count == 0u {
        return 0.0;
    }

    let count_scale = log(f32(count) + 1.0);
    let max_count_scale = log(f32(max_count) + 1.0);
    return clamp(count_scale / max_count_scale, 0.0, 1.0);
}

fn density_colour(intensity: f32) -> vec3<f32> {
    let background = vec3<f32>(0.012, 0.015, 0.030);
    let low_density = vec3<f32>(0.05, 0.16, 0.30);
    let mid_density = vec3<f32>(0.16, 0.46, 0.78);
    let high_density = vec3<f32>(1.0, 0.58, 0.20);

    if intensity <= 0.0 {
        return background;
    }

    let low_to_mid = smoothstep(0.0, 0.65, intensity);
    let mid_to_high = smoothstep(0.45, 1.0, intensity);
    let cool_colour = mix(low_density, mid_density, low_to_mid);
    return mix(cool_colour, high_density, mid_to_high);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let clamped_uv = clamp(input.uv, vec2<f32>(0.0), vec2<f32>(0.999999));
    let x_bin = min(u32(clamped_uv.x * f32(params.grid_width)), params.grid_width - 1u);
    let y_bin = min(u32(clamped_uv.y * f32(params.grid_height)), params.grid_height - 1u);
    let bin_index = y_bin * params.grid_width + x_bin;
    let count = counts[bin_index];
    let intensity = density_intensity(count, params.max_bin_count);
    return vec4<f32>(density_colour(intensity), 1.0);
}
