struct Event {
    timestamp_offset: u32,
    lane: u32,
    is_before_time_range: u32,
    is_after_time_range: u32,
};

struct Params {
    time_span: u32,
    lane_count: u32,
    grid_width: u32,
    grid_height: u32,
    event_start: u32,
    dispatch_event_count: u32,
};

@group(0) @binding(0)
var<storage, read> events: array<Event>;

@group(0) @binding(1)
var<uniform> params: Params;

@group(0) @binding(2)
var<storage, read_write> counts: array<atomic<u32>>;

fn bin_time(timestamp_offset: u32, time_span: u32, bin_count: u32) -> u32 {
    if timestamp_offset == time_span {
        return bin_count - 1u;
    }

    let normalized = f32(timestamp_offset) / f32(time_span);
    let raw_bin = u32(floor(normalized * f32(bin_count)));
    return min(raw_bin, bin_count - 1u);
}

fn bin_lane(lane: u32, lane_count: u32, height: u32) -> u32 {
    let raw_bin = (lane * height) / lane_count;
    return min(raw_bin, height - 1u);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if global_id.x >= params.dispatch_event_count {
        return;
    }

    let event_index = params.event_start + global_id.x;
    let event = events[event_index];
    let event_is_outside_time = event.is_before_time_range != 0u || event.is_after_time_range != 0u;
    let event_is_outside_lane = event.lane >= params.lane_count;
    if event_is_outside_time || event_is_outside_lane {
        return;
    }

    let x_bin = bin_time(event.timestamp_offset, params.time_span, params.grid_width);
    let y_bin = bin_lane(event.lane, params.lane_count, params.grid_height);
    let bin_index = y_bin * params.grid_width + x_bin;
    atomicAdd(&counts[bin_index], 1u);
}
