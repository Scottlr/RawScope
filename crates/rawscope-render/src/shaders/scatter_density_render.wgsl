struct RenderParams {
    grid_width: u32,
    grid_height: u32,
    max_bin_count: u32,
    transform_id: u32,
    palette_id: u32,
    presentation_id: u32,
    padding1: u32,
    padding2: u32,
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

fn density_intensity_value(count: f32, max_count: u32, transform_id: u32) -> f32 {
    if count <= 0.0 || max_count == 0u {
        return 0.0;
    }

    if transform_id == 0u {
        return clamp(count / f32(max_count), 0.0, 1.0);
    }

    let count_scale = log(count + 1.0);
    let max_count_scale = log(f32(max_count) + 1.0);
    return clamp(count_scale / max_count_scale, 0.0, 1.0);
}

fn palette_colour(
    intensity: f32,
    background: vec3<f32>,
    low_density: vec3<f32>,
    mid_density: vec3<f32>,
    high_density: vec3<f32>,
) -> vec3<f32> {
    if intensity <= 0.0 {
        return background;
    }

    let low_to_mid = smoothstep(0.0, 0.65, intensity);
    let mid_to_high = smoothstep(0.45, 1.0, intensity);
    let cool_colour = mix(low_density, mid_density, low_to_mid);
    return mix(cool_colour, high_density, mid_to_high);
}

fn density_colour(intensity: f32, palette_id: u32) -> vec3<f32> {
    if palette_id == 1u {
        return palette_colour(
            intensity,
            vec3<f32>(0.012, 0.015, 0.030),
            vec3<f32>(0.05, 0.16, 0.30),
            vec3<f32>(0.16, 0.46, 0.78),
            vec3<f32>(1.0, 0.58, 0.20),
        );
    }

    return palette_colour(
        intensity,
        vec3<f32>(0.015, 0.025, 0.035),
        vec3<f32>(0.02, 0.19, 0.28),
        vec3<f32>(0.08, 0.55, 0.58),
        vec3<f32>(1.0, 0.74, 0.30),
    );
}

fn clamped_bin_count(bin: vec2<i32>) -> f32 {
    let max_bin = vec2<i32>(i32(params.grid_width) - 1, i32(params.grid_height) - 1);
    let clamped_bin = clamp(bin, vec2<i32>(0), max_bin);
    let bin_index = u32(clamped_bin.y) * params.grid_width + u32(clamped_bin.x);
    return f32(counts[bin_index]);
}

fn reconstructed_intensity(grid_position: vec2<f32>) -> f32 {
    let base_bin = vec2<i32>(floor(grid_position));
    let blend = fract(grid_position);
    let count00 = clamped_bin_count(base_bin);
    let count10 = clamped_bin_count(base_bin + vec2<i32>(1, 0));
    let count01 = clamped_bin_count(base_bin + vec2<i32>(0, 1));
    let count11 = clamped_bin_count(base_bin + vec2<i32>(1, 1));
    let lower_count = mix(count00, count10, blend.x);
    let upper_count = mix(count01, count11, blend.x);
    let interpolated_count = mix(lower_count, upper_count, blend.y);
    return density_intensity_value(
        interpolated_count,
        params.max_bin_count,
        params.transform_id,
    );
}

fn topographic_intensity(grid_position: vec2<f32>) -> f32 {
    let cardinal_offset = 1.15;
    let diagonal_offset = vec2<f32>(cardinal_offset);
    let centre = reconstructed_intensity(grid_position) * 0.28;
    let cardinal = (
        reconstructed_intensity(grid_position + vec2<f32>(cardinal_offset, 0.0))
        + reconstructed_intensity(grid_position - vec2<f32>(cardinal_offset, 0.0))
        + reconstructed_intensity(grid_position + vec2<f32>(0.0, cardinal_offset))
        + reconstructed_intensity(grid_position - vec2<f32>(0.0, cardinal_offset))
    ) * 0.12;
    let diagonal = (
        reconstructed_intensity(grid_position + diagonal_offset)
        + reconstructed_intensity(grid_position - diagonal_offset)
        + reconstructed_intensity(
            grid_position + vec2<f32>(diagonal_offset.x, -diagonal_offset.y),
        )
        + reconstructed_intensity(
            grid_position + vec2<f32>(-diagonal_offset.x, diagonal_offset.y),
        )
    ) * 0.06;
    return centre + cardinal + diagonal;
}

fn topographic_colour(uv: vec2<f32>) -> vec3<f32> {
    let grid_size = vec2<f32>(f32(params.grid_width), f32(params.grid_height));
    let density_uv = vec2<f32>(uv.x, 1.0 - uv.y);
    let grid_position = density_uv * grid_size - vec2<f32>(0.5);
    let intensity = topographic_intensity(grid_position);
    if intensity <= 0.00001 {
        return density_colour(0.0, params.palette_id);
    }

    let sample_offset = 0.85;
    let left = topographic_intensity(grid_position - vec2<f32>(sample_offset, 0.0));
    let right = topographic_intensity(grid_position + vec2<f32>(sample_offset, 0.0));
    let lower = topographic_intensity(grid_position - vec2<f32>(0.0, sample_offset));
    let upper = topographic_intensity(grid_position + vec2<f32>(0.0, sample_offset));
    let normal = normalize(vec3<f32>((left - right) * 4.5, (lower - upper) * 4.5, 0.55));
    let light_direction = normalize(vec3<f32>(-0.45, -0.55, 0.72));
    let relief = 0.72 + 0.38 * max(dot(normal, light_direction), 0.0);

    let contour_coordinate = intensity * 10.0;
    let contour_phase = fract(contour_coordinate);
    let contour_distance = min(contour_phase, 1.0 - contour_phase);
    let contour_width = max(fwidth(contour_coordinate) * 0.72, 0.012);
    let contour_line = 1.0 - smoothstep(contour_width, contour_width * 1.8, contour_distance);
    let contour_strength = contour_line * smoothstep(0.06, 0.24, intensity) * 0.42;

    let background_colour = density_colour(0.0, params.palette_id);
    let shaded_density_colour = density_colour(intensity, params.palette_id) * relief;
    let field_visibility = smoothstep(0.015, 0.09, intensity);
    let base_colour = mix(background_colour, shaded_density_colour, field_visibility);
    let contour_colour = min(base_colour + vec3<f32>(0.22, 0.18, 0.08), vec3<f32>(1.0));
    return mix(base_colour, contour_colour, contour_strength);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let clamped_uv = clamp(input.uv, vec2<f32>(0.0), vec2<f32>(0.999999));
    if params.presentation_id == 1u {
        return vec4<f32>(topographic_colour(clamped_uv), 1.0);
    }

    let x_bin = min(u32(clamped_uv.x * f32(params.grid_width)), params.grid_width - 1u);
    let y_uv = 1.0 - clamped_uv.y;
    let y_bin = min(u32(y_uv * f32(params.grid_height)), params.grid_height - 1u);
    let bin_index = y_bin * params.grid_width + x_bin;
    let count = counts[bin_index];
    let intensity = density_intensity(count, params.max_bin_count, params.transform_id);
    return vec4<f32>(density_colour(intensity, params.palette_id), 1.0);
}
