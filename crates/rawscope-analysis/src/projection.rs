//! Precise, generation-shareable scatter projection output.

use std::sync::Arc;

use rawscope_core::RowId;
use rawscope_data::{ScatterPointKind, ScatterPointRecord};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedScatterPoint {
    pub row_id: RowId,
    pub x: f64,
    pub y: f64,
    pub kind: ScatterPointKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedScatterGeneration {
    points: Arc<[ProjectedScatterPoint]>,
}

impl ProjectedScatterGeneration {
    pub fn from_points(points: &[ScatterPointRecord]) -> Result<Self, ProjectionError> {
        let projected = points
            .iter()
            .map(|point| {
                if !point.x.is_finite() || !point.y.is_finite() {
                    return Err(ProjectionError::NonFinite {
                        row_id: point.row_id,
                    });
                }
                Ok(ProjectedScatterPoint {
                    row_id: point.row_id,
                    x: f64::from(point.x),
                    y: f64::from(point.y),
                    kind: point.kind,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            points: projected.into(),
        })
    }

    pub fn points(&self) -> Arc<[ProjectedScatterPoint]> {
        Arc::clone(&self.points)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    NonFinite { row_id: RowId },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_projection_shares_arc_storage() {
        let input = [ScatterPointRecord {
            row_id: RowId(4),
            x: 1.25,
            y: 2.5,
            kind: ScatterPointKind::Unclassified,
        }];
        let generation = ProjectedScatterGeneration::from_points(&input).unwrap();
        let first = generation.points();
        let second = generation.points();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(first[0].row_id, RowId(4));
    }
}
