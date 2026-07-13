struct RenderParams {
    grid_width: u32,
    grid_height: u32,
    layer_count: u32,
    _padding: u32,
    max_count: f32,
    density_floor: f32,
    opacity: f32,
    _padding_params: u32,
    neutral_rgba: vec4<f32>,
    _padding_tail: vec4<u32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) rgba: vec4<f32>,
};

@group(0) @binding(0)
var<storage, read> layer_counts: array<u32>;
@group(0) @binding(1)
var<storage, read> palette: array<vec4<f32>>;
@group(0) @binding(2)
var<uniform> params: RenderParams;

fn composition_purity(bin_index: u32, total: f32) -> f32 {
    if (total <= 0.0 || params.layer_count <= 1u) {
        return select(0.0, 1.0, total > 0.0);
    }
    var entropy = 0.0;
    var layer_index = 0u;
    while (layer_index < params.layer_count) {
        let count = f32(layer_counts[layer_index * params.grid_width * params.grid_height + bin_index]);
        if (count > 0.0) {
            let share = count / total;
            entropy -= share * log(share);
        }
        layer_index += 1u;
    }
    return clamp(1.0 - entropy / log(f32(params.layer_count)), 0.0, 1.0);
}

fn composition_color(bin_index: u32) -> vec4<f32> {
    var total = 0.0;
    var dominant_count = 0u;
    var dominant_layer = 0u;
    var layer_index = 0u;
    while (layer_index < params.layer_count) {
        let count = layer_counts[layer_index * params.grid_width * params.grid_height + bin_index];
        total += f32(count);
        if (count > dominant_count) {
            dominant_count = count;
            dominant_layer = layer_index;
        }
        layer_index += 1u;
    }
    if (total <= 0.0) {
        return vec4<f32>(params.neutral_rgba.rgb, 0.0);
    }
    let purity = composition_purity(bin_index, total);
    let density = clamp(total / max(params.max_count, 1.0), 0.0, 1.0);
    let lightness = params.density_floor
        + (1.0 - params.density_floor) * sqrt(density);
    let hue = palette[dominant_layer].rgb;
    let mixed = mix(params.neutral_rgba.rgb, hue, purity) * lightness;
    return vec4<f32>(mixed, params.opacity * density);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let local = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    )[vertex_index];
    let x = f32(instance_index % params.grid_width);
    let y = f32(instance_index / params.grid_width);
    let center = vec2<f32>(
        (x + 0.5) / f32(params.grid_width) * 2.0 - 1.0,
        1.0 - (y + 0.5) / f32(params.grid_height) * 2.0,
    );
    let half_size = vec2<f32>(
        1.0 / f32(params.grid_width),
        1.0 / f32(params.grid_height),
    );
    let position = center + local * half_size;
    let bin_index = y * params.grid_width + x;
    return VertexOutput(
        vec4<f32>(position, 0.0, 1.0),
        composition_color(bin_index),
    );
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.rgba;
}
