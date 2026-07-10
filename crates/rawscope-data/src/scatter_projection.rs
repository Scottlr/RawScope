//! Truthful coordinate projections for paired numeric scatter fields.

use std::{error::Error, fmt};

use rawscope_core::F32Range;

use crate::ScatterPointRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScatterProjection {
    #[default]
    RawXY,
    MeanDifference,
}

impl ScatterProjection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::RawXY => "Raw",
            Self::MeanDifference => "Mean / difference",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterProjectionSpec {
    pub x_column: String,
    pub y_column: String,
}

impl ScatterProjectionSpec {
    pub fn new(x_column: impl Into<String>, y_column: impl Into<String>) -> Self {
        Self {
            x_column: x_column.into(),
            y_column: y_column.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterProjectionLabels {
    pub x_label: String,
    pub y_label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedScatterData {
    pub projection: ScatterProjection,
    pub labels: ScatterProjectionLabels,
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub points: Vec<ScatterPointRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScatterProjectionError {
    EmptyPoints,
    NonFiniteInput { row_id: u64 },
    NonFiniteResult { row_id: u64 },
}

impl fmt::Display for ScatterProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPoints => write!(formatter, "cannot project an empty scatter dataset"),
            Self::NonFiniteInput { row_id } => {
                write!(
                    formatter,
                    "scatter row {row_id} has non-finite input coordinates"
                )
            }
            Self::NonFiniteResult { row_id } => write!(
                formatter,
                "scatter row {row_id} projection is outside the finite f32 range"
            ),
        }
    }
}

impl Error for ScatterProjectionError {}

pub fn project_scatter_points(
    points: &[ScatterPointRecord],
    x_column: &str,
    y_column: &str,
    projection: ScatterProjection,
) -> Result<ProjectedScatterData, ScatterProjectionError> {
    if points.is_empty() {
        return Err(ScatterProjectionError::EmptyPoints);
    }

    let labels = projection_labels(x_column, y_column, projection);
    let mut projected = Vec::with_capacity(points.len());
    let mut x_min = f32::INFINITY;
    let mut x_max = f32::NEG_INFINITY;
    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;

    for point in points {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(ScatterProjectionError::NonFiniteInput {
                row_id: point.row_id.0,
            });
        }
        let (x, y) = project_coordinates(point, projection)?;
        x_min = x_min.min(x);
        x_max = x_max.max(x);
        y_min = y_min.min(y);
        y_max = y_max.max(y);
        projected.push(ScatterPointRecord {
            row_id: point.row_id,
            x,
            y,
            kind: point.kind,
        });
    }

    Ok(ProjectedScatterData {
        projection,
        labels,
        x_range: F32Range::from_bounds_expanded(x_min, x_max),
        y_range: F32Range::from_bounds_expanded(y_min, y_max),
        points: projected,
    })
}

fn projection_labels(
    x_column: &str,
    y_column: &str,
    projection: ScatterProjection,
) -> ScatterProjectionLabels {
    match projection {
        ScatterProjection::RawXY => ScatterProjectionLabels {
            x_label: x_column.to_string(),
            y_label: y_column.to_string(),
        },
        ScatterProjection::MeanDifference => ScatterProjectionLabels {
            x_label: format!("mean({x_column}, {y_column})"),
            y_label: format!("{x_column} - {y_column}"),
        },
    }
}

fn project_coordinates(
    point: &ScatterPointRecord,
    projection: ScatterProjection,
) -> Result<(f32, f32), ScatterProjectionError> {
    if projection == ScatterProjection::RawXY {
        return Ok((point.x, point.y));
    }
    let x = f64::from(point.x);
    let y = f64::from(point.y);
    let mean = ((x + y) / 2.0) as f32;
    let difference = (x - y) as f32;
    if !mean.is_finite() || !difference.is_finite() {
        return Err(ScatterProjectionError::NonFiniteResult {
            row_id: point.row_id.0,
        });
    }
    Ok((mean, difference))
}

#[cfg(test)]
mod tests {
    use rawscope_core::RowId;

    use crate::{ScatterPointKind, SyntheticPointCategory};

    use super::*;

    fn point(row_id: u64, x: f32, y: f32, kind: ScatterPointKind) -> ScatterPointRecord {
        ScatterPointRecord {
            row_id: RowId(row_id),
            x,
            y,
            kind,
        }
    }

    #[test]
    fn mean_difference_projection_uses_x_minus_y() {
        let projected = project_scatter_points(
            &[point(
                7,
                1_800.0,
                1_600.0,
                ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster),
            )],
            "white_rating",
            "black_rating",
            ScatterProjection::MeanDifference,
        )
        .unwrap();

        assert_eq!(projected.points[0].x, 1_700.0);
        assert_eq!(projected.points[0].y, 200.0);
        assert_eq!(projected.labels.x_label, "mean(white_rating, black_rating)");
        assert_eq!(projected.labels.y_label, "white_rating - black_rating");
    }

    #[test]
    fn projection_preserves_row_ids_order_and_kind() {
        let points = vec![
            point(
                9,
                3.0,
                1.0,
                ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier),
            ),
            point(
                2,
                4.0,
                2.0,
                ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster),
            ),
        ];
        let projected =
            project_scatter_points(&points, "x", "y", ScatterProjection::MeanDifference).unwrap();

        assert_eq!(projected.points[0].row_id, RowId(9));
        assert_eq!(
            projected.points[0].kind,
            ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier)
        );
        assert_eq!(projected.points[1].row_id, RowId(2));
        assert_eq!(
            projected.points[1].kind,
            ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster)
        );
    }

    #[test]
    fn raw_projection_preserves_coordinates() {
        let points = vec![point(
            1,
            f32::from_bits(0x3f80_0001),
            f32::from_bits(0xc020_0001),
            ScatterPointKind::Unclassified,
        )];
        let projected =
            project_scatter_points(&points, "a", "b", ScatterProjection::RawXY).unwrap();

        assert_eq!(projected.points[0].x.to_bits(), points[0].x.to_bits());
        assert_eq!(projected.points[0].y.to_bits(), points[0].y.to_bits());
        assert_eq!(projected.labels.x_label, "a");
        assert_eq!(projected.labels.y_label, "b");
    }
}
