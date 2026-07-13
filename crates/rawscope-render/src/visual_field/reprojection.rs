//! Pure data-space reprojection from a completed density field to a display viewport.

use rawscope_analysis::visual_field::VisualFieldMapping;
use rawscope_core::F32Range;

use super::VisualFieldQuality;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualFieldViewport {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
    pub viewport_revision: u64,
    pub quality: VisualFieldQuality,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualFieldReprojection {
    pub source: VisualFieldViewport,
    pub display_x_range: F32Range,
    pub display_y_range: F32Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualFieldReprojectionError {
    MappingMismatch,
    GridMismatch,
}

impl VisualFieldReprojection {
    pub fn try_new(
        source: VisualFieldViewport,
        source_mapping: VisualFieldMapping,
        target: VisualFieldViewport,
        target_mapping: VisualFieldMapping,
        display_x_range: F32Range,
        display_y_range: F32Range,
    ) -> Result<Self, VisualFieldReprojectionError> {
        if source_mapping != target_mapping {
            return Err(VisualFieldReprojectionError::MappingMismatch);
        }
        if (source.grid_width, source.grid_height) != (target.grid_width, target.grid_height) {
            return Err(VisualFieldReprojectionError::GridMismatch);
        }
        Ok(Self {
            source,
            display_x_range,
            display_y_range,
        })
    }

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
    use rawscope_analysis::visual_field::{VisualFieldMapping, VisualFieldProjection};
    use rawscope_core::ColumnId;
    use rawscope_data::{DatasetSchema, StoreColumnKind};

    use super::*;
    fn source() -> VisualFieldViewport {
        VisualFieldViewport {
            x_range: F32Range::new(0.0, 100.0),
            y_range: F32Range::new(0.0, 100.0),
            grid_width: 256,
            grid_height: 256,
            viewport_revision: 1,
            quality: VisualFieldQuality::Exact,
        }
    }

    #[test]
    fn reprojection_maps_display_uv_through_data_space_to_source_uv() {
        let reprojection = VisualFieldReprojection {
            source: source(),
            display_x_range: F32Range::new(20.0, 70.0),
            display_y_range: F32Range::new(10.0, 60.0),
        };
        assert_eq!(reprojection.source_uv(0.5, 0.5), Some((0.45, 0.65)));
    }

    #[test]
    fn reprojection_marks_newly_uncovered_region_outside_source_field() {
        let reprojection = VisualFieldReprojection {
            source: source(),
            display_x_range: F32Range::new(80.0, 130.0),
            display_y_range: F32Range::new(0.0, 100.0),
        };
        assert_eq!(reprojection.source_uv(0.75, 0.5), None);
    }

    #[test]
    fn reprojection_requires_compatible_grid_and_mapping() {
        let schema =
            DatasetSchema::try_new([("x", StoreColumnKind::F64), ("y", StoreColumnKind::F64)])
                .unwrap();
        let mapping = VisualFieldMapping::try_new(
            &schema,
            VisualFieldProjection::NumericPair {
                x: ColumnId::new(0),
                y: ColumnId::new(1),
            },
            None,
        )
        .unwrap();
        let mismatched_mapping = VisualFieldMapping::try_new(
            &schema,
            VisualFieldProjection::NumericPair {
                x: ColumnId::new(1),
                y: ColumnId::new(0),
            },
            None,
        )
        .unwrap();
        let source = source();
        let target = VisualFieldViewport {
            grid_width: 128,
            ..source
        };
        assert_eq!(
            VisualFieldReprojection::try_new(
                source,
                mapping,
                source,
                mismatched_mapping,
                source.x_range,
                source.y_range,
            ),
            Err(VisualFieldReprojectionError::MappingMismatch)
        );
        assert_eq!(
            VisualFieldReprojection::try_new(
                source,
                mapping,
                target,
                mapping,
                source.x_range,
                source.y_range,
            ),
            Err(VisualFieldReprojectionError::GridMismatch)
        );
    }
}
