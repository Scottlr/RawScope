struct OverlayParams {
    min_x_px: f32,
    min_y_px: f32,
    max_x_px: f32,
    max_y_px: f32,
    screen_width_px: f32,
    screen_height_px: f32,
    border_width_px: f32,
    padding: f32,
    fill_rgba: vec4<f32>,
    border_rgba: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> params: OverlayParams;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0)
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let point = position.xy;
    let inside_x = point.x >= params.min_x_px && point.x <= params.max_x_px;
    let inside_y = point.y >= params.min_y_px && point.y <= params.max_y_px;
    let inside_rect = inside_x && inside_y;
    if !inside_rect {
        return vec4<f32>(0.0);
    }

    let near_left = point.x <= params.min_x_px + params.border_width_px;
    let near_right = point.x >= params.max_x_px - params.border_width_px;
    let near_top = point.y <= params.min_y_px + params.border_width_px;
    let near_bottom = point.y >= params.max_y_px - params.border_width_px;
    let on_border = near_left || near_right || near_top || near_bottom;
    if on_border {
        return params.border_rgba;
    }

    return params.fill_rgba;
}
