//! Pure data-space reprojection from a completed density field to a display viewport.

use rawscope_core::F32Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityQualityTier {
    Preview,
    Exact,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DensityFieldViewport {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
    pub viewport_revision: u64,
    pub quality: DensityQualityTier,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DensityReprojection {
    pub source: DensityFieldViewport,
    pub display_x_range: F32Range,
    pub display_y_range: F32Range,
}

impl DensityReprojection {
    pub fn source_uv(self, display_u: f32, display_v: f32) -> Option<(f32, f32)> {
        let data_x = self.display_x_range.min + display_u * self.display_x_range.span();
        let data_y = self.display_y_range.max - display_v * self.display_y_range.span();
        let source_u = (data_x - self.source.x_range.min) / self.source.x_range.span();
        let source_v = (self.source.y_range.max - data_y) / self.source.y_range.span();
        ((0.0..=1.0).contains(&source_u) && (0.0..=1.0).contains(&source_v))
            .then_some((source_u, source_v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> DensityFieldViewport {
        DensityFieldViewport {
            x_range: F32Range::new(0.0, 100.0),
            y_range: F32Range::new(0.0, 100.0),
            grid_width: 256,
            grid_height: 256,
            viewport_revision: 1,
            quality: DensityQualityTier::Exact,
        }
    }

    #[test]
    fn reprojection_maps_display_uv_through_data_space_to_source_uv() {
        let reprojection = DensityReprojection {
            source: source(),
            display_x_range: F32Range::new(20.0, 70.0),
            display_y_range: F32Range::new(10.0, 60.0),
        };
        assert_eq!(reprojection.source_uv(0.5, 0.5), Some((0.45, 0.65)));
    }

    #[test]
    fn reprojection_marks_newly_uncovered_region_outside_source_field() {
        let reprojection = DensityReprojection {
            source: source(),
            display_x_range: F32Range::new(80.0, 130.0),
            display_y_range: F32Range::new(0.0, 100.0),
        };
        assert_eq!(reprojection.source_uv(0.75, 0.5), None);
    }
}
