struct Params {
    grid_width: u32,
    grid_height: u32,
    _padding: vec2<u32>,
    plot_scale_x: f32,
    plot_scale_y: f32,
    mark_length: f32,
    opacity: f32,
};

struct RidgeCell {
    strength: f32,
    tangent_x: f32,
    tangent_y: f32,
    _padding: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) strength: f32,
};

@group(0) @binding(0)
var<storage, read> cells: array<RidgeCell>;
@group(0) @binding(1)
var<uniform> params: Params;

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
    let cell = cells[instance_index];
    let x = f32(instance_index % params.grid_width) + 0.5;
    let y = f32(instance_index / params.grid_width) + 0.5;
    let center = vec2<f32>(
        x / f32(params.grid_width) * 2.0 - 1.0,
        1.0 - y / f32(params.grid_height) * 2.0,
    );
    let scaled_tangent = vec2<f32>(
        cell.tangent_x * params.plot_scale_x,
        cell.tangent_y * params.plot_scale_y,
    );
    let tangent_length = max(length(scaled_tangent), 1.0e-6);
    let tangent = scaled_tangent / tangent_length;
    let normal = vec2<f32>(-tangent.y, tangent.x);
    let half_length = params.mark_length * 0.5;
    let half_thickness = params.mark_length * 0.12;
    let position = center
        + tangent * (local.x * half_length)
        + normal * (local.y * half_thickness);
    return VertexOutput(
        vec4<f32>(position, 0.0, 1.0),
        cell.strength,
    );
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let strength = clamp(input.strength, 0.0, 1.0);
    let color = vec3<f32>(0.93, 0.77, 0.35);
    return vec4<f32>(color, strength * params.opacity);
}
