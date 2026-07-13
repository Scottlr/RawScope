struct Point {
    x: f32,
    y: f32,
};

struct Params {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    grid_width: u32,
    grid_height: u32,
    point_start: u32,
    dispatch_point_count: u32,
};

struct SpecialLayerParams {
    untracked_layer: u32,
    missing_layer: u32,
    invalid_layer: u32,
    layer_count: u32,
};

@group(0) @binding(0) var<storage, read> points: array<Point>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var<storage, read> total_counts: array<u32>;
@group(0) @binding(3) var<storage, read> filter_mask: array<u32>;
@group(0) @binding(4) var<storage, read> category_codes: array<u32>;
@group(0) @binding(5) var<storage, read> value_to_layer: array<u32>;
@group(0) @binding(6) var<storage, read> special_layers: SpecialLayerParams;
@group(0) @binding(7) var<storage, read_write> layer_counts: array<atomic<u32>>;

const NO_CATEGORY_LAYER: u32 = 0xffffffffu;

fn bin_f32(value: f32, range_min: f32, range_max: f32, bin_count: u32) -> u32 {
    if value == range_max {
        return bin_count - 1u;
    }
    let normalized = (value - range_min) / (range_max - range_min);
    let raw_bin = u32(floor(normalized * f32(bin_count)));
    return min(raw_bin, bin_count - 1u);
}

fn resolve_layer(code: u32) -> u32 {
    if code < arrayLength(&value_to_layer) {
        return value_to_layer[code];
    }
    return special_layers.invalid_layer;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if global_id.x >= params.dispatch_point_count {
        return;
    }
    let point_index = params.point_start + global_id.x;
    if filter_mask[point_index] == 0u {
        return;
    }
    let point = points[point_index];
    if point.x < params.x_min || point.x > params.x_max ||
       point.y < params.y_min || point.y > params.y_max {
        return;
    }
    let x_bin = bin_f32(point.x, params.x_min, params.x_max, params.grid_width);
    let y_bin = bin_f32(point.y, params.y_min, params.y_max, params.grid_height);
    let bin_index = y_bin * params.grid_width + x_bin;
    // Keep the total field as the shared coordinate/mask contract. The
    // composition pass never allocates or increments a duplicate total.
    let _exact_total = total_counts[bin_index];
    let layer_id = resolve_layer(category_codes[point_index]);
    if layer_id == NO_CATEGORY_LAYER || layer_id >= special_layers.layer_count {
        return;
    }
    let layer_index = layer_id * (params.grid_width * params.grid_height) + bin_index;
    atomicAdd(&layer_counts[layer_index], 1u);
}
