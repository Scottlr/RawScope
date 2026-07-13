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
    split_fraction: f32,
    presentation: u32,
    shared_density_max_count: u32,
    density_transform: u32,
    max_support_share: f32,
    padding4: u32,
    padding5: u32,
    padding6: u32,
};

@group(0) @binding(0) var<storage, read> baseline_counts: array<u32>;
@group(0) @binding(1) var<storage, read> active_counts: array<u32>;
@group(0) @binding(2) var<storage, read_write> max_abs_fixed: array<atomic<u32>>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let bin_count = params.grid_width * params.grid_height;
    if id.x >= bin_count || params.baseline_total <= 0.0 || params.active_total <= 0.0 {
        return;
    }
    let baseline_share = f32(baseline_counts[id.x]) / params.baseline_total;
    let active_share = f32(active_counts[id.x]) / params.active_total;
    let encoded = u32(clamp(
        round(abs(active_share - baseline_share) * f32(params.fixed_point_scale)),
        0.0,
        4294967295.0,
    ));
    atomicMax(&max_abs_fixed[0], encoded);
}
