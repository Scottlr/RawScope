//! GPU-side packing for scatter-density points.

use bytemuck::{Pod, Zeroable};
use rawscope_analysis::projection::ProjectedScatterPoint;
use rawscope_core::F32Range;
use rawscope_data::ScatterPointRecord;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuPoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuQuantization {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualPackingError {
    NonFinite { row_id: rawscope_core::RowId },
    OutsideDomain { row_id: rawscope_core::RowId },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackedScatterPoint {
    pub row_id: rawscope_core::RowId,
    pub x: f32,
    pub y: f32,
}

pub fn pack_scatter_points(
    points: &[ProjectedScatterPoint],
    quantization: GpuQuantization,
) -> Result<Vec<PackedScatterPoint>, VisualPackingError> {
    let x_span = quantization.x_max - quantization.x_min;
    let y_span = quantization.y_max - quantization.y_min;
    points
        .iter()
        .map(|point| {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(VisualPackingError::NonFinite {
                    row_id: point.row_id,
                });
            }
            if point.x < quantization.x_min
                || point.x > quantization.x_max
                || point.y < quantization.y_min
                || point.y > quantization.y_max
                || x_span <= 0.0
                || y_span <= 0.0
            {
                return Err(VisualPackingError::OutsideDomain {
                    row_id: point.row_id,
                });
            }
            let x = ((point.x - quantization.x_min) / x_span) as f32;
            let y = ((point.y - quantization.y_min) / y_span) as f32;
            if !x.is_finite() || !y.is_finite() {
                return Err(VisualPackingError::NonFinite {
                    row_id: point.row_id,
                });
            }
            Ok(PackedScatterPoint {
                row_id: point.row_id,
                x,
                y,
            })
        })
        .collect()
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

pub(crate) fn pack_points(points: &[ScatterPointRecord]) -> Vec<GpuPoint> {
    points
        .iter()
        .map(|point| GpuPoint {
            x: point.x,
            y: point.y,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_analysis::projection::ProjectedScatterPoint;
    use rawscope_core::RowId;
    use rawscope_data::ScatterPointKind;

    #[test]
    fn packing_rejects_non_finite_and_out_of_domain_points() {
        let quantization = GpuQuantization { x_min: 0.0, x_max: 1.0, y_min: 0.0, y_max: 1.0 };
        let non_finite = [ProjectedScatterPoint { row_id: RowId(1), x: f64::NAN, y: 0.5, kind: ScatterPointKind::Unclassified }];
        assert!(matches!(pack_scatter_points(&non_finite, quantization), Err(VisualPackingError::NonFinite { .. })));
        let outside = [ProjectedScatterPoint { row_id: RowId(2), x: 2.0, y: 0.5, kind: ScatterPointKind::Unclassified }];
        assert!(matches!(pack_scatter_points(&outside, quantization), Err(VisualPackingError::OutsideDomain { .. })));
    }
}
