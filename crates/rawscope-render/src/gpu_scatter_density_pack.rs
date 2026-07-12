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

/// Source-domain and normalized-error disclosure for GPU coordinate packing.
///
/// The error bounds are conservative one-ULP bounds for the f32 normalized
/// coordinate representation, expressed back in each source domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuQuantizationDisclosure {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub x_scale: f64,
    pub y_scale: f64,
    pub max_abs_error_x: f64,
    pub max_abs_error_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualPackingError {
    InvalidDomain,
    NonFinite { row_id: rawscope_core::RowId },
    OutsideDomain { row_id: rawscope_core::RowId },
}

impl GpuQuantization {
    pub fn validate(self) -> Result<(), VisualPackingError> {
        let domains = [(self.x_min, self.x_max), (self.y_min, self.y_max)];
        if domains
            .iter()
            .any(|(min, max)| !min.is_finite() || !max.is_finite() || max <= min)
        {
            return Err(VisualPackingError::InvalidDomain);
        }
        Ok(())
    }

    /// Returns the validated source-domain/precision disclosure for packed coordinates.
    pub fn disclosure(self) -> Result<GpuQuantizationDisclosure, VisualPackingError> {
        self.validate()?;
        let x_span = self.x_max - self.x_min;
        let y_span = self.y_max - self.y_min;
        let normalized_ulp = f64::from(f32::EPSILON);
        Ok(GpuQuantizationDisclosure {
            x_min: self.x_min,
            x_max: self.x_max,
            y_min: self.y_min,
            y_max: self.y_max,
            x_scale: 1.0 / x_span,
            y_scale: 1.0 / y_span,
            max_abs_error_x: x_span * normalized_ulp,
            max_abs_error_y: y_span * normalized_ulp,
        })
    }
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
    let _disclosure = quantization.disclosure()?;
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
    use std::mem::{align_of, size_of};

    use super::*;
    use rawscope_analysis::projection::ProjectedScatterPoint;
    use rawscope_core::RowId;
    use rawscope_data::ScatterPointKind;

    #[test]
    fn scatter_gpu_abis_match_wgsl_scalar_layout() {
        assert_eq!(size_of::<GpuPoint>(), 8);
        assert_eq!(align_of::<GpuPoint>(), 4);
        assert_eq!(size_of::<ScatterParams>(), 32);
        assert_eq!(align_of::<ScatterParams>(), 4);
        assert_eq!(std::mem::offset_of!(GpuPoint, x), 0);
        assert_eq!(std::mem::offset_of!(GpuPoint, y), 4);
        assert_eq!(std::mem::offset_of!(ScatterParams, x_min), 0);
        assert_eq!(std::mem::offset_of!(ScatterParams, x_max), 4);
        assert_eq!(std::mem::offset_of!(ScatterParams, y_min), 8);
        assert_eq!(std::mem::offset_of!(ScatterParams, y_max), 12);
        assert_eq!(std::mem::offset_of!(ScatterParams, grid_width), 16);
        assert_eq!(std::mem::offset_of!(ScatterParams, grid_height), 20);
        assert_eq!(std::mem::offset_of!(ScatterParams, point_start), 24);
        assert_eq!(
            std::mem::offset_of!(ScatterParams, dispatch_point_count),
            28
        );
    }

    #[test]
    fn packing_rejects_non_finite_and_out_of_domain_points() {
        let quantization = GpuQuantization {
            x_min: 0.0,
            x_max: 1.0,
            y_min: 0.0,
            y_max: 1.0,
        };
        let non_finite = [ProjectedScatterPoint {
            row_id: RowId(1),
            x: f64::NAN,
            y: 0.5,
            kind: ScatterPointKind::Unclassified,
        }];
        assert!(matches!(
            pack_scatter_points(&non_finite, quantization),
            Err(VisualPackingError::NonFinite { .. })
        ));
        let outside = [ProjectedScatterPoint {
            row_id: RowId(2),
            x: 2.0,
            y: 0.5,
            kind: ScatterPointKind::Unclassified,
        }];
        assert!(matches!(
            pack_scatter_points(&outside, quantization),
            Err(VisualPackingError::OutsideDomain { .. })
        ));
    }

    #[test]
    fn packing_rejects_invalid_quantization_domains_before_rows() {
        let invalid = GpuQuantization {
            x_min: 1.0,
            x_max: 1.0,
            y_min: 0.0,
            y_max: 1.0,
        };
        assert_eq!(invalid.validate(), Err(VisualPackingError::InvalidDomain));
        assert_eq!(
            pack_scatter_points(&[], invalid),
            Err(VisualPackingError::InvalidDomain)
        );
    }

    #[test]
    fn quantization_disclosure_reports_domain_scale_and_error_bounds() {
        let quantization = GpuQuantization {
            x_min: -10.0,
            x_max: 10.0,
            y_min: 100.0,
            y_max: 140.0,
        };

        let disclosure = quantization.disclosure().unwrap();

        assert_eq!(disclosure.x_scale, 0.05);
        assert_eq!(disclosure.y_scale, 0.025);
        assert_eq!(disclosure.max_abs_error_x, 20.0 * f64::from(f32::EPSILON));
        assert_eq!(disclosure.max_abs_error_y, 40.0 * f64::from(f32::EPSILON));
    }

    #[test]
    fn packed_points_preserve_row_identity_at_the_quantization_boundary() {
        let points = [ProjectedScatterPoint {
            row_id: RowId(42),
            x: 1.0,
            y: 3.0,
            kind: ScatterPointKind::Unclassified,
        }];
        let packed = pack_scatter_points(
            &points,
            GpuQuantization {
                x_min: 0.0,
                x_max: 2.0,
                y_min: 0.0,
                y_max: 6.0,
            },
        )
        .unwrap();

        assert_eq!(packed[0].row_id, RowId(42));
        assert_eq!(packed[0].x, 0.5);
        assert_eq!(packed[0].y, 0.5);
    }
}
