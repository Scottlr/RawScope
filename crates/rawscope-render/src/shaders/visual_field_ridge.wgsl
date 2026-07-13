struct Params {
    width: u32,
    height: u32,
    radius: u32,
    _padding: u32,
    x_step: f32,
    y_step: f32,
    sigma_squared: f32,
    strength_cutoff: f32,
    anisotropy_cutoff: f32,
    _padding_tail: vec3<f32>,
};

struct RidgeCell {
    strength: f32,
    tangent_x: f32,
    tangent_y: f32,
    _padding: f32,
};

@group(0) @binding(0)
var<storage, read> counts: array<u32>;
@group(0) @binding(1)
var<uniform> params: Params;
@group(0) @binding(2)
var<storage, read_write> horizontal: array<f32>;
@group(0) @binding(3)
var<storage, read_write> smoothed: array<f32>;
@group(0) @binding(4)
var<storage, read_write> candidates: array<vec4<f32>>;
@group(0) @binding(5)
var<storage, read_write> maximum_raw_bits: array<atomic<u32>>;
@group(0) @binding(6)
var<storage, read_write> cells: array<RidgeCell>;

fn index(x: u32, y: u32) -> u32 {
    return y * params.width + x;
}

fn clamp_i32(value: i32, limit: u32) -> u32 {
    return u32(clamp(value, 0, i32(limit) - 1));
}

fn kernel_weight(offset: i32, radius: u32) -> f32 {
    if radius == 1u {
        if offset == -1 || offset == 1 { return 0.25; }
        if offset == 0 { return 0.5; }
        return 0.0;
    }
    if radius == 2u {
        if offset == -2 || offset == 2 { return 0.0625; }
        if offset == -1 || offset == 1 { return 0.25; }
        if offset == 0 { return 0.375; }
        return 0.0;
    }
    if abs(offset) > 4 { return 0.0; }
    switch offset {
        case -4, 4: { return 0.00390625; }
        case -3, 3: { return 0.03125; }
        case -2, 2: { return 0.109375; }
        case -1, 1: { return 0.21875; }
        default: { return 0.2734375; }
    }
}

fn finite(value: f32) -> bool {
    return value == value && abs(value) < 3.402823e+38;
}

fn sample_smoothed(x_value: f32, y_value: f32) -> f32 {
    let x = clamp(x_value, 0.0, f32(params.width - 1u));
    let y = clamp(y_value, 0.0, f32(params.height - 1u));
    let x0 = u32(floor(x));
    let y0 = u32(floor(y));
    let x1 = min(x0 + 1u, params.width - 1u);
    let y1 = min(y0 + 1u, params.height - 1u);
    let tx = x - f32(x0);
    let ty = y - f32(y0);
    let top = mix(smoothed[index(x0, y0)], smoothed[index(x1, y0)], tx);
    let bottom = mix(smoothed[index(x0, y1)], smoothed[index(x1, y1)], tx);
    return mix(top, bottom, ty);
}

@compute @workgroup_size(64)
fn smooth_horizontal(@builtin(global_invocation_id) id: vec3<u32>) {
    let count = params.width * params.height;
    if id.x >= count { return; }
    let x = id.x % params.width;
    let y = id.x / params.width;
    var value = 0.0;
    for (var offset: i32 = -4; offset <= 4; offset++) {
        let sample_x = clamp_i32(i32(x) + offset, params.width);
        value += kernel_weight(offset, params.radius) * f32(counts[index(sample_x, y)]);
    }
    horizontal[id.x] = value;
}

@compute @workgroup_size(64)
fn smooth_vertical(@builtin(global_invocation_id) id: vec3<u32>) {
    let count = params.width * params.height;
    if id.x >= count { return; }
    let x = id.x % params.width;
    let y = id.x / params.width;
    var value = 0.0;
    for (var offset: i32 = -4; offset <= 4; offset++) {
        let sample_y = clamp_i32(i32(y) + offset, params.height);
        value += kernel_weight(offset, params.radius) * horizontal[index(x, sample_y)];
    }
    smoothed[id.x] = value;
}

struct EigenResult {
    lambda_min: f32,
    lambda_max: f32,
    normal_x: f32,
    normal_y: f32,
};

fn eigen_for_minor_direction(dxx: f32, dxy: f32, dyy: f32) -> EigenResult {
    let half_trace = (dxx + dyy) * 0.5;
    let half_difference = (dxx - dyy) * 0.5;
    let discriminant = sqrt(half_difference * half_difference + dxy * dxy);
    let lambda_min = half_trace - discriminant;
    let lambda_max = half_trace + discriminant;
    if !finite(discriminant) || discriminant <= 1.0e-12 { return EigenResult(lambda_min, lambda_max, 0.0, 0.0); }
    let candidate_a = vec2<f32>(dxy, lambda_min - dxx);
    let candidate_b = vec2<f32>(lambda_min - dyy, dxy);
    let candidate = select(candidate_b, candidate_a, dot(candidate_a, candidate_a) >= dot(candidate_b, candidate_b));
    let length = length(candidate);
    if !finite(length) || length <= 1.0e-12 { return EigenResult(lambda_min, lambda_max, 0.0, 0.0); }
    return EigenResult(lambda_min, lambda_max, candidate.x / length, candidate.y / length);
}

@compute @workgroup_size(64)
fn candidate(@builtin(global_invocation_id) id: vec3<u32>) {
    let count = params.width * params.height;
    if id.x >= count { return; }
    let x = f32(id.x % params.width);
    let y = f32(id.x / params.width);
    let center = sample_smoothed(x, y);
    let left = sample_smoothed(x - 1.0, y);
    let right = sample_smoothed(x + 1.0, y);
    let up = sample_smoothed(x, y - 1.0);
    let down = sample_smoothed(x, y + 1.0);
    let upper_left = sample_smoothed(x - 1.0, y - 1.0);
    let upper_right = sample_smoothed(x + 1.0, y - 1.0);
    let lower_left = sample_smoothed(x - 1.0, y + 1.0);
    let lower_right = sample_smoothed(x + 1.0, y + 1.0);
    let dxx = (right - 2.0 * center + left) / (params.x_step * params.x_step);
    let dyy = (down - 2.0 * center + up) / (params.y_step * params.y_step);
    let dxy = (lower_right - lower_left - upper_right + upper_left) / (4.0 * params.x_step * params.y_step);
    let eigen = eigen_for_minor_direction(dxx, dxy, dyy);
    let eigengap = abs(eigen.lambda_max - eigen.lambda_min);
    let anisotropy = eigengap / (abs(eigen.lambda_max) + abs(eigen.lambda_min) + 1.0e-12);
    let plus = sample_smoothed(x + eigen.normal_x, y + eigen.normal_y);
    let minus = sample_smoothed(x - eigen.normal_x, y - eigen.normal_y);
    let local_maximum = !(eigen.normal_x == 0.0 && eigen.normal_y == 0.0)
        && center >= plus && center >= minus && (center > plus || center > minus);
    let is_candidate = eigen.lambda_min < 0.0
        && finite(anisotropy)
        && anisotropy >= params.anisotropy_cutoff
        && eigengap > 1.0e-12
        && local_maximum;
    let raw_strength = select(0.0, params.sigma_squared * max(-eigen.lambda_min, 0.0), is_candidate);
    candidates[id.x] = vec4<f32>(raw_strength, eigen.normal_x, eigen.normal_y, 0.0);
}

@compute @workgroup_size(64)
fn reduce_maximum(@builtin(global_invocation_id) id: vec3<u32>) {
    let count = params.width * params.height;
    if id.x >= count { return; }
    let raw_strength = candidates[id.x].x;
    if raw_strength > 0.0 && finite(raw_strength) {
        atomicMax(&maximum_raw_bits[0], bitcast<u32>(raw_strength));
    }
}

@compute @workgroup_size(64)
fn normalize(@builtin(global_invocation_id) id: vec3<u32>) {
    let count = params.width * params.height;
    if id.x >= count { return; }
    let maximum_raw = bitcast<f32>(atomicLoad(&maximum_raw_bits[0]));
    let candidate_value = candidates[id.x];
    if maximum_raw <= 0.0 || !finite(maximum_raw) || candidate_value.x <= 0.0 {
        cells[id.x] = RidgeCell(0.0, 0.0, 0.0, 0.0);
        return;
    }
    let strength = clamp(candidate_value.x / maximum_raw, 0.0, 1.0);
    if !finite(strength) || strength < params.strength_cutoff {
        cells[id.x] = RidgeCell(0.0, 0.0, 0.0, 0.0);
        return;
    }
    var tangent = vec2<f32>(-candidate_value.z, candidate_value.y);
    if tangent.x < 0.0 || (tangent.x == 0.0 && tangent.y < 0.0) { tangent = -tangent; }
    let tangent_length = length(tangent);
    if !finite(tangent_length) || tangent_length <= 1.0e-12 {
        cells[id.x] = RidgeCell(0.0, 0.0, 0.0, 0.0);
        return;
    }
    tangent /= tangent_length;
    cells[id.x] = RidgeCell(strength, tangent.x, tangent.y, 0.0);
}
