//! GPU-side packing for scatter-density points.

use bytemuck::{Pod, Zeroable};
use rawscope_core::F32Range;
use rawscope_data::SyntheticPointRecord;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuPoint {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct ScatterParams {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    grid_width: u32,
    grid_height: u32,
    point_start: u32,
    dispatch_point_count: u32,
}

impl ScatterParams {
    pub(crate) fn new(
        x_range: F32Range,
        y_range: F32Range,
        grid_width: u32,
        grid_height: u32,
        point_start: u32,
        dispatch_point_count: u32,
    ) -> Self {
        Self {
            x_min: x_range.min,
            x_max: x_range.max,
            y_min: y_range.min,
            y_max: y_range.max,
            grid_width,
            grid_height,
            point_start,
            dispatch_point_count,
        }
    }
}

pub(crate) fn pack_points(points: &[SyntheticPointRecord]) -> Vec<GpuPoint> {
    points
        .iter()
        .map(|point| GpuPoint {
            x: point.x,
            y: point.y,
        })
        .collect()
}
