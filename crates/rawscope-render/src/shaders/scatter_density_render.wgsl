struct RenderParams {
    grid_width: u32,
    grid_height: u32,
    max_bin_count: u32,
    transform_id: u32,
    palette_id: u32,
    presentation_id: u32,
    padding1: u32,
    padding2: u32,
    source_x_min: f32,
    source_x_max: f32,
    source_y_min: f32,
    source_y_max: f32,
    display_x_min: f32,
    display_x_max: f32,
    display_y_min: f32,
    display_y_max: f32,
    relief_height_strength: f32,
    relief_normal_radius_bins: u32,
    relief_light_azimuth_radians: f32,
    relief_light_elevation_radians: f32,
    relief_ambient_strength: f32,
    relief_shadow_strength: f32,
    relief_contour_strength: f32,
    relief_padding: f32,
    previous_max_bin_count: u32,
    previous_transform_id: u32,
    previous_palette_id: u32,
    previous_presentation_id: u32,
    previous_source_x_min: f32,
    previous_source_x_max: f32,
    previous_source_y_min: f32,
    previous_source_y_max: f32,
    transition_progress: f32,
    transition_padding1: f32,
    transition_padding2: f32,
    transition_padding3: f32,
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
var<storage, read> max_counts: array<u32>;

@group(0) @binding(3)
var<storage, read> previous_counts: array<u32>;

@group(0) @binding(4)
var palette_lut: texture_2d<f32>;

@group(0) @binding(5)
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
    output.uv = vec2<f32>(
        position.x * 0.5 + 0.5,
        0.5 - position.y * 0.5,
    );
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

fn density_colour(intensity: f32, palette_id: u32) -> vec3<f32> {
    return textureSampleLevel(
        palette_lut,
        palette_sampler,
        vec2<f32>(clamp(intensity, 0.0, 1.0), (f32(palette_id) + 0.5) / 3.0),
        0.0,
    ).rgb;
}

fn clamped_bin_count(bin: vec2<i32>, previous: bool) -> f32 {
    let max_bin = vec2<i32>(i32(params.grid_width) - 1, i32(params.grid_height) - 1);
    let clamped_bin = clamp(bin, vec2<i32>(0), max_bin);
    let bin_index = u32(clamped_bin.y) * params.grid_width + u32(clamped_bin.x);
    return select(f32(counts[bin_index]), f32(previous_counts[bin_index]), previous);
}

fn reconstructed_intensity(grid_position: vec2<f32>, previous: bool) -> f32 {
    let base_bin = vec2<i32>(floor(grid_position));
    let blend = fract(grid_position);
    let count00 = clamped_bin_count(base_bin, previous);
    let count10 = clamped_bin_count(base_bin + vec2<i32>(1, 0), previous);
    let count01 = clamped_bin_count(base_bin + vec2<i32>(0, 1), previous);
    let count11 = clamped_bin_count(base_bin + vec2<i32>(1, 1), previous);
    let lower_count = mix(count00, count10, blend.x);
    let upper_count = mix(count01, count11, blend.x);
    let interpolated_count = mix(lower_count, upper_count, blend.y);
    return density_intensity_value(
        interpolated_count,
        select(max_counts[0], params.previous_max_bin_count, previous),
        select(params.transform_id, params.previous_transform_id, previous),
    );
}

fn topographic_intensity(grid_position: vec2<f32>, previous: bool) -> f32 {
    let cardinal_offset = 1.15;
    let diagonal_offset = vec2<f32>(cardinal_offset);
    let centre = reconstructed_intensity(grid_position, previous) * 0.28;
    let cardinal = (
        reconstructed_intensity(grid_position + vec2<f32>(cardinal_offset, 0.0), previous)
        + reconstructed_intensity(grid_position - vec2<f32>(cardinal_offset, 0.0), previous)
        + reconstructed_intensity(grid_position + vec2<f32>(0.0, cardinal_offset), previous)
        + reconstructed_intensity(grid_position - vec2<f32>(0.0, cardinal_offset), previous)
    ) * 0.12;
    let diagonal = (
        reconstructed_intensity(grid_position + diagonal_offset, previous)
        + reconstructed_intensity(grid_position - diagonal_offset, previous)
        + reconstructed_intensity(
            grid_position + vec2<f32>(diagonal_offset.x, -diagonal_offset.y), previous,
        )
        + reconstructed_intensity(
            grid_position + vec2<f32>(-diagonal_offset.x, diagonal_offset.y), previous,
        )
    ) * 0.06;
    return centre + cardinal + diagonal;
}

fn topographic_colour(uv: vec2<f32>, previous: bool) -> vec3<f32> {
    let grid_size = vec2<f32>(f32(params.grid_width), f32(params.grid_height));
    let density_uv = vec2<f32>(uv.x, 1.0 - uv.y);
    let grid_position = density_uv * grid_size - vec2<f32>(0.5);
    let intensity = topographic_intensity(grid_position, previous);
    let palette_id = select(params.palette_id, params.previous_palette_id, previous);
    if intensity <= 0.00001 {
        return density_colour(0.0, palette_id);
    }

    let sample_offset = 0.85;
    let left = topographic_intensity(grid_position - vec2<f32>(sample_offset, 0.0), previous);
    let right = topographic_intensity(grid_position + vec2<f32>(sample_offset, 0.0), previous);
    let lower = topographic_intensity(grid_position - vec2<f32>(0.0, sample_offset), previous);
    let upper = topographic_intensity(grid_position + vec2<f32>(0.0, sample_offset), previous);
    let normal = normalize(vec3<f32>((left - right) * 4.5, (lower - upper) * 4.5, 0.55));
    let light_direction = normalize(vec3<f32>(-0.45, -0.55, 0.72));
    let relief = 0.72 + 0.38 * max(dot(normal, light_direction), 0.0);

    let contour_coordinate = intensity * 10.0;
    let contour_phase = fract(contour_coordinate);
    let contour_distance = min(contour_phase, 1.0 - contour_phase);
    let contour_width = max(fwidth(contour_coordinate) * 0.72, 0.012);
    let contour_line = 1.0 - smoothstep(contour_width, contour_width * 1.8, contour_distance);
    let contour_strength = contour_line * smoothstep(0.06, 0.24, intensity) * 0.42;

    let background_colour = density_colour(0.0, palette_id);
    let shaded_density_colour = density_colour(intensity, palette_id) * relief;
    let field_visibility = smoothstep(0.015, 0.09, intensity);
    let base_colour = mix(background_colour, shaded_density_colour, field_visibility);
    let contour_colour = min(base_colour + vec3<f32>(0.22, 0.18, 0.08), vec3<f32>(1.0));
    return mix(base_colour, contour_colour, contour_strength);
}

fn relief_colour(uv: vec2<f32>, previous: bool) -> vec3<f32> {
    let grid_size = vec2<f32>(f32(params.grid_width), f32(params.grid_height));
    let density_uv = vec2<f32>(uv.x, 1.0 - uv.y);
    let position = density_uv * grid_size - vec2<f32>(0.5);
    let intensity = topographic_intensity(position, previous);
    let palette_id = select(params.palette_id, params.previous_palette_id, previous);
    if intensity <= 0.00001 || params.grid_width <= 1u || params.grid_height <= 1u {
        return density_colour(intensity, palette_id);
    }

    let fine_radius = f32(max(params.relief_normal_radius_bins, 1u));
    let coarse_radius = min(fine_radius * 2.0, 8.0);
    let fine_x = topographic_intensity(position - vec2<f32>(fine_radius, 0.0), previous)
        - topographic_intensity(position + vec2<f32>(fine_radius, 0.0), previous);
    let fine_y = topographic_intensity(position - vec2<f32>(0.0, fine_radius), previous)
        - topographic_intensity(position + vec2<f32>(0.0, fine_radius), previous);
    let coarse_x = topographic_intensity(position - vec2<f32>(coarse_radius, 0.0), previous)
        - topographic_intensity(position + vec2<f32>(coarse_radius, 0.0), previous);
    let coarse_y = topographic_intensity(position - vec2<f32>(0.0, coarse_radius), previous)
        - topographic_intensity(position + vec2<f32>(0.0, coarse_radius), previous);
    let gradient = (vec2<f32>(fine_x, fine_y) * 0.68
        + vec2<f32>(coarse_x, coarse_y) * 0.32) * params.relief_height_strength;
    let normal = normalize(vec3<f32>(gradient, 1.0));

    let horizontal_light = vec2<f32>(
        cos(params.relief_light_azimuth_radians),
        sin(params.relief_light_azimuth_radians),
    );
    let light = normalize(vec3<f32>(
        horizontal_light * cos(params.relief_light_elevation_radians),
        sin(params.relief_light_elevation_radians),
    ));
    let diffuse = max(dot(normal, light), 0.0);

    var horizon_occlusion = 0.0;
    for (var step = 1u; step <= 8u; step = step + 1u) {
        let distance = f32(step) * fine_radius;
        let sample = topographic_intensity(position + horizontal_light * distance, previous);
        let horizon = intensity + distance * tan(params.relief_light_elevation_radians) * 0.012;
        horizon_occlusion = max(horizon_occlusion, smoothstep(horizon, horizon + 0.035, sample));
    }

    let visibility = smoothstep(0.018, 0.12, intensity);
    let lighting = params.relief_ambient_strength
        + (1.0 - params.relief_ambient_strength) * diffuse;
    let shadow = 1.0 - horizon_occlusion * params.relief_shadow_strength;
    let base = density_colour(intensity, palette_id);
    var shaded = base * mix(1.0, lighting * shadow, visibility);

    let contour_coordinate = intensity * 12.0;
    let contour_phase = fract(contour_coordinate);
    let contour_distance = min(contour_phase, 1.0 - contour_phase);
    let contour_width = max(fwidth(contour_coordinate) * 0.65, 0.012);
    let contour = 1.0 - smoothstep(contour_width, contour_width * 1.8, contour_distance);
    shaded *= 1.0 - contour * params.relief_contour_strength * visibility;
    return clamp(shaded, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn source_uv_for_display(display_uv: vec2<f32>, previous: bool) -> vec3<f32> {
    let data_x = params.display_x_min + display_uv.x * (params.display_x_max - params.display_x_min);
    let data_y = params.display_y_max - display_uv.y * (params.display_y_max - params.display_y_min);
    let source_x_min = select(params.source_x_min, params.previous_source_x_min, previous);
    let source_x_max = select(params.source_x_max, params.previous_source_x_max, previous);
    let source_y_min = select(params.source_y_min, params.previous_source_y_min, previous);
    let source_y_max = select(params.source_y_max, params.previous_source_y_max, previous);
    let source_u = (data_x - source_x_min) / (source_x_max - source_x_min);
    let source_v = (source_y_max - data_y) / (source_y_max - source_y_min);
    let covered = source_u >= 0.0 && source_u <= 1.0 && source_v >= 0.0 && source_v <= 1.0;
    return vec3<f32>(source_u, source_v, select(0.0, 1.0, covered));
}

fn presentation_colour(display_uv: vec2<f32>, previous: bool) -> vec3<f32> {
    let palette_id = select(params.palette_id, params.previous_palette_id, previous);
    let presentation_id = select(params.presentation_id, params.previous_presentation_id, previous);
    let transform_id = select(params.transform_id, params.previous_transform_id, previous);
    let max_bin_count = select(max_counts[0], params.previous_max_bin_count, previous);
    let projected = source_uv_for_display(display_uv, previous);
    if projected.z < 0.5 {
        return density_colour(0.0, palette_id);
    }
    let uv = clamp(projected.xy, vec2<f32>(0.0), vec2<f32>(0.999999));
    if presentation_id == 1u {
        return topographic_colour(uv, previous);
    }
    if presentation_id == 2u {
        return relief_colour(uv, previous);
    }
    let x_bin = min(u32(uv.x * f32(params.grid_width)), params.grid_width - 1u);
    let y_bin = min(u32((1.0 - uv.y) * f32(params.grid_height)), params.grid_height - 1u);
    let bin_index = y_bin * params.grid_width + x_bin;
    let count = select(counts[bin_index], previous_counts[bin_index], previous);
    return density_colour(density_intensity(count, max_bin_count, transform_id), palette_id);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let display_uv = clamp(input.uv, vec2<f32>(0.0), vec2<f32>(0.999999));
    let previous = presentation_colour(display_uv, true);
    let current = presentation_colour(display_uv, false);
    return vec4<f32>(mix(previous, current, params.transition_progress), 1.0);
}
