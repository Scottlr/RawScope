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

/// Closed source numeric set accepted at the CPU projection boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectionNumeric {
    I64(i64),
    U64(u64),
    F64(f64),
}

impl ProjectionNumeric {
    fn as_f64(self) -> Option<f64> {
        let value = match self {
            Self::I64(value) => value as f64,
            Self::U64(value) => value as f64,
            Self::F64(value) => value,
        };
        value.is_finite().then_some(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreciseScatterInput {
    pub row_id: RowId,
    pub x: ProjectionNumeric,
    pub y: ProjectionNumeric,
    pub kind: ScatterPointKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedScatterGeneration {
    points: Arc<[ProjectedScatterPoint]>,
}

impl ProjectedScatterGeneration {
    /// Projects typed source numerics once into the CPU f64 analysis domain.
    pub fn from_numeric_inputs(inputs: &[PreciseScatterInput]) -> Result<Self, ProjectionError> {
        let projected = inputs
            .iter()
            .map(|input| {
                let x = input.x.as_f64().ok_or(ProjectionError::NonFinite {
                    row_id: input.row_id,
                })?;
                let y = input.y.as_f64().ok_or(ProjectionError::NonFinite {
                    row_id: input.row_id,
                })?;
                Ok(ProjectedScatterPoint {
                    row_id: input.row_id,
                    x,
                    y,
                    kind: input.kind,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            points: projected.into(),
        })
    }

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

    #[test]
    fn typed_numeric_inputs_preserve_integer_boundaries_until_projection() {
        let inputs = [PreciseScatterInput {
            row_id: RowId(9),
            x: ProjectionNumeric::U64(u64::MAX),
            y: ProjectionNumeric::I64(i64::MIN),
            kind: ScatterPointKind::Unclassified,
        }];
        let generation = ProjectedScatterGeneration::from_numeric_inputs(&inputs).unwrap();
        let point = generation.points()[0];
        assert_eq!(point.x, u64::MAX as f64);
        assert_eq!(point.y, i64::MIN as f64);
    }

    #[test]
    fn typed_projection_rejects_non_finite_f64() {
        let inputs = [PreciseScatterInput {
            row_id: RowId(3),
            x: ProjectionNumeric::F64(f64::INFINITY),
            y: ProjectionNumeric::F64(0.0),
            kind: ScatterPointKind::Unclassified,
        }];
        assert_eq!(
            ProjectedScatterGeneration::from_numeric_inputs(&inputs),
            Err(ProjectionError::NonFinite { row_id: RowId(3) })
        );
    }
}
