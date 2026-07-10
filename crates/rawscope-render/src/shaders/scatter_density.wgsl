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

@group(0) @binding(0)
var<storage, read> points: array<Point>;

@group(0) @binding(1)
var<uniform> params: Params;

@group(0) @binding(2)
var<storage, read_write> counts: array<atomic<u32>>;

@group(0) @binding(3)
var<storage, read_write> max_count: array<atomic<u32>>;

@group(0) @binding(4)
var<storage, read> filter_mask: array<u32>;

fn bin_f32(value: f32, range_min: f32, range_max: f32, bin_count: u32) -> u32 {
    if value == range_max {
        return bin_count - 1u;
    }

    let normalized = (value - range_min) / (range_max - range_min);
    let raw_bin = u32(floor(normalized * f32(bin_count)));
    return min(raw_bin, bin_count - 1u);
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
    let point_is_outside_x = point.x < params.x_min || point.x > params.x_max;
    let point_is_outside_y = point.y < params.y_min || point.y > params.y_max;
    if point_is_outside_x || point_is_outside_y {
        return;
    }

    let x_bin = bin_f32(point.x, params.x_min, params.x_max, params.grid_width);
    let y_bin = bin_f32(point.y, params.y_min, params.y_max, params.grid_height);
    let bin_index = y_bin * params.grid_width + x_bin;
    let previous = atomicAdd(&counts[bin_index], 1u);
    atomicMax(&max_count[0], previous + 1u);
}
