struct OverlayParams {
    exact_rect: vec4<f32>,
    neighborhood_rect: vec4<f32>,
    plot_origin_px: vec2<f32>,
    plot_size_px: vec2<f32>,
    border_width_px: f32,
    crosshair_width_px: f32,
    presentation_alpha: f32,
    focus_kind: u32,
    padding: vec4<u32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> params: OverlayParams;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

fn inside(point: vec2<f32>, rect: vec4<f32>) -> bool {
    return point.x >= rect.x && point.x <= rect.z && point.y >= rect.y && point.y <= rect.w;
}

fn edge_distance(point: vec2<f32>, rect: vec4<f32>) -> f32 {
    return min(
        min(point.x - rect.x, rect.z - point.x),
        min(point.y - rect.y, rect.w - point.y),
    );
}

fn distance_to_rect(point: vec2<f32>, rect: vec4<f32>) -> f32 {
    let dx = max(max(rect.x - point.x, 0.0), point.x - rect.z);
    let dy = max(max(rect.y - point.y, 0.0), point.y - rect.w);
    return length(vec2<f32>(dx, dy));
}

fn focus_colour(kind: u32) -> vec3<f32> {
    if kind == 1u {
        return vec3<f32>(1.0, 0.72, 0.22);
    }
    return vec3<f32>(0.25, 0.95, 0.88);
}

fn is_corner_accent(point: vec2<f32>, rect: vec4<f32>, border_width: f32) -> bool {
    let near_left = abs(point.x - rect.x) <= border_width;
    let near_right = abs(point.x - rect.z) <= border_width;
    let near_top = abs(point.y - rect.y) <= border_width;
    let near_bottom = abs(point.y - rect.w) <= border_width;
    let near_corner_x = point.x <= rect.x + 5.0 || point.x >= rect.z - 5.0;
    let near_corner_y = point.y <= rect.y + 5.0 || point.y >= rect.w - 5.0;
    return (near_left || near_right) && near_corner_y
        || (near_top || near_bottom) && near_corner_x;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let point = input.position.xy - params.plot_origin_px;
    let in_plot = point.x >= 0.0
        && point.y >= 0.0
        && point.x <= params.plot_size_px.x
        && point.y <= params.plot_size_px.y;
    if !in_plot || params.presentation_alpha <= 0.0 {
        return vec4<f32>(0.0);
    }

    let in_exact = inside(point, params.exact_rect);
    let in_neighborhood = inside(point, params.neighborhood_rect);
    let exact_colour = focus_colour(params.focus_kind);
    let crosshair_colour = vec3<f32>(0.25, 0.95, 0.88);

    if in_exact {
        let exact_edge = edge_distance(point, params.exact_rect);
        if is_corner_accent(point, params.exact_rect, params.border_width_px) {
            return vec4<f32>(exact_colour, 1.0 * params.presentation_alpha);
        }
        if exact_edge <= params.border_width_px {
            return vec4<f32>(exact_colour, 0.96 * params.presentation_alpha);
        }
    }

    let exact_center = vec2<f32>(
        (params.exact_rect.x + params.exact_rect.z) * 0.5,
        (params.exact_rect.y + params.exact_rect.w) * 0.5,
    );
    let crosshair_hit = !in_exact
        && (abs(point.x - exact_center.x) <= params.crosshair_width_px * 0.5
            || abs(point.y - exact_center.y) <= params.crosshair_width_px * 0.5);
    if crosshair_hit {
        return vec4<f32>(crosshair_colour, 0.18 * params.presentation_alpha);
    }

    if in_neighborhood && !in_exact {
        let distance = distance_to_rect(point, params.exact_rect);
        let fade = 1.0 - smoothstep(0.0, 12.0, distance);
        return vec4<f32>(crosshair_colour, 0.08 * fade * params.presentation_alpha);
    }

    return vec4<f32>(0.0);
}
