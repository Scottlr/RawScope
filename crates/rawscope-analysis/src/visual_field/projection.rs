//! Immutable visual-field projection generations built from typed store cells.

use std::{error::Error, fmt, sync::Arc};

use rawscope_core::RowId;
use rawscope_data::{
    CellState, DatasetAccessError, DatasetGeneration, DatasetStore, NormalizedValue,
    StoreColumnKind,
};

use super::{
    TimeAxisTransform, TimeValueProjectionError, VisualFieldMapping, VisualFieldProjection,
};
use crate::inspection::F64Domain;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedVisualPoint {
    pub row_id: RowId,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisualAxisDomain {
    I64 { min: i64, max: i64 },
    U64 { min: u64, max: u64 },
    F64(F64Domain),
    TimestampMicros { min: i64, max: i64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedVisualFieldGeneration {
    dataset_generation: DatasetGeneration,
    mapping: VisualFieldMapping,
    points: Arc<[ProjectedVisualPoint]>,
    /// Exact timestamps aligned with `points` for a time-value projection.
    ///
    /// `ProjectedVisualPoint::x` remains the source-domain f64 compatibility
    /// coordinate.  GPU consumers must use `gpu_points`, which derives the
    /// bounded coordinate from these exact integers instead of an epoch-scale
    /// f32 cast.
    timestamp_micros: Option<Arc<[i64]>>,
    time_axis_transform: Option<TimeAxisTransform>,
    x_domain: VisualAxisDomain,
    y_domain: VisualAxisDomain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualFieldProjectionError {
    Mapping(super::VisualFieldMappingError),
    DatasetAccess(DatasetAccessError),
    MissingValue {
        row_id: RowId,
        column_id: rawscope_core::ColumnId,
    },
    InvalidValue {
        row_id: RowId,
        column_id: rawscope_core::ColumnId,
    },
    UnexpectedValueKind {
        row_id: RowId,
    },
    NonFinite {
        row_id: RowId,
    },
    TimeValue(TimeValueProjectionError),
    NoValidPoints,
    NonIncreasingDomain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualFieldRowPolicy {
    ExcludeMissingOrInvalid,
    RejectMissingOrInvalid,
}

impl ProjectedVisualFieldGeneration {
    pub fn from_store(
        store: &DatasetStore,
        mapping: VisualFieldMapping,
    ) -> Result<Self, VisualFieldProjectionError> {
        Self::from_store_with_policy(
            store,
            mapping,
            VisualFieldRowPolicy::ExcludeMissingOrInvalid,
        )
    }

    pub fn from_store_with_policy(
        store: &DatasetStore,
        mapping: VisualFieldMapping,
        row_policy: VisualFieldRowPolicy,
    ) -> Result<Self, VisualFieldProjectionError> {
        mapping
            .validate_against(store.schema())
            .map_err(VisualFieldProjectionError::Mapping)?;

        let mut points = Vec::new();
        let mut timestamp_micros = Vec::new();
        let mut x_values = Vec::new();
        let mut y_values = Vec::new();
        let row_count = store.row_count();
        let x_column = mapping.x_column();
        let y_column = mapping.y_column();
        let x_kind = column_kind(store, x_column)?;
        let y_kind = column_kind(store, y_column)?;
        for row_number in 0..row_count {
            let row_id = RowId(row_number);
            let x = match read_axis_value(store, row_id, x_column, x_kind)? {
                AxisRead::Value(value) => value,
                AxisRead::Missing => match row_policy {
                    VisualFieldRowPolicy::ExcludeMissingOrInvalid => continue,
                    VisualFieldRowPolicy::RejectMissingOrInvalid => {
                        return Err(VisualFieldProjectionError::MissingValue {
                            row_id,
                            column_id: x_column,
                        })
                    }
                },
                AxisRead::Invalid => match row_policy {
                    VisualFieldRowPolicy::ExcludeMissingOrInvalid => continue,
                    VisualFieldRowPolicy::RejectMissingOrInvalid => {
                        return Err(VisualFieldProjectionError::InvalidValue {
                            row_id,
                            column_id: x_column,
                        })
                    }
                },
            };
            let y = match read_axis_value(store, row_id, y_column, y_kind)? {
                AxisRead::Value(value) => value,
                AxisRead::Missing => match row_policy {
                    VisualFieldRowPolicy::ExcludeMissingOrInvalid => continue,
                    VisualFieldRowPolicy::RejectMissingOrInvalid => {
                        return Err(VisualFieldProjectionError::MissingValue {
                            row_id,
                            column_id: y_column,
                        })
                    }
                },
                AxisRead::Invalid => match row_policy {
                    VisualFieldRowPolicy::ExcludeMissingOrInvalid => continue,
                    VisualFieldRowPolicy::RejectMissingOrInvalid => {
                        return Err(VisualFieldProjectionError::InvalidValue {
                            row_id,
                            column_id: y_column,
                        })
                    }
                },
            };
            if !matches_projection_kind(mapping.projection(), x, y) {
                return Err(VisualFieldProjectionError::UnexpectedValueKind { row_id });
            }
            let x_f64 = x
                .as_f64()
                .ok_or(VisualFieldProjectionError::NonFinite { row_id })?;
            let y_f64 = y
                .as_f64()
                .ok_or(VisualFieldProjectionError::NonFinite { row_id })?;
            points.push(ProjectedVisualPoint {
                row_id,
                x: x_f64,
                y: y_f64,
            });
            if matches!(
                mapping.projection(),
                VisualFieldProjection::TimeValue { .. }
            ) {
                let AxisValue::TimestampMicros(timestamp) = x else {
                    return Err(VisualFieldProjectionError::UnexpectedValueKind { row_id });
                };
                timestamp_micros.push(timestamp);
            }
            x_values.push(x);
            y_values.push(y);
        }

        if points.is_empty() {
            return Err(VisualFieldProjectionError::NoValidPoints);
        }

        let x_domain = domain_for_values(x_values)?;
        let y_domain = domain_for_values(y_values)?;
        let (timestamp_micros, time_axis_transform) = match x_domain {
            VisualAxisDomain::TimestampMicros { min, max } => {
                let transform = TimeAxisTransform::from_domain(min, max)
                    .map_err(VisualFieldProjectionError::TimeValue)?;
                (Some(timestamp_micros.into()), Some(transform))
            }
            _ => (None, None),
        };
        Ok(Self {
            dataset_generation: store.generation(),
            mapping,
            points: points.into(),
            timestamp_micros,
            time_axis_transform,
            x_domain,
            y_domain,
        })
    }

    pub fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }

    pub const fn mapping(&self) -> VisualFieldMapping {
        self.mapping
    }

    pub fn points(&self) -> Arc<[ProjectedVisualPoint]> {
        Arc::clone(&self.points)
    }

    /// Returns exact timestamps aligned with the projected points, if this is
    /// a time-value field.
    pub fn timestamp_micros(&self) -> Option<Arc<[i64]>> {
        self.timestamp_micros.as_ref().map(Arc::clone)
    }

    /// Returns the checked timestamp transform for a time-value field.
    pub const fn time_axis_transform(&self) -> Option<TimeAxisTransform> {
        self.time_axis_transform
    }

    /// Produces bounded coordinates suitable for f32 GPU upload.
    ///
    /// The analytical `points` accessor remains source-domain compatible for
    /// numeric-pair callers.  Time-value coordinates are derived from the
    /// exact aligned timestamp storage here, so epoch-scale values never pass
    /// through an intermediate f32 representation.
    pub fn gpu_points(&self) -> Arc<[ProjectedVisualPoint]> {
        let (Some(transform), Some(timestamps)) =
            (self.time_axis_transform, self.timestamp_micros.as_ref())
        else {
            return Arc::clone(&self.points);
        };
        self.points
            .iter()
            .zip(timestamps.iter())
            .map(|(point, timestamp)| ProjectedVisualPoint {
                row_id: point.row_id,
                x: transform.normalized(*timestamp),
                y: point.y,
            })
            .collect::<Vec<_>>()
            .into()
    }

    pub const fn x_domain(&self) -> VisualAxisDomain {
        self.x_domain
    }

    pub const fn y_domain(&self) -> VisualAxisDomain {
        self.y_domain
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AxisValue {
    I64(i64),
    U64(u64),
    F64(f64),
    TimestampMicros(i64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AxisRead {
    Value(AxisValue),
    Missing,
    Invalid,
}

impl AxisValue {
    fn as_f64(self) -> Option<f64> {
        let value = match self {
            Self::I64(value) | Self::TimestampMicros(value) => value as f64,
            Self::U64(value) => value as f64,
            Self::F64(value) => value,
        };
        value.is_finite().then_some(value)
    }
}

fn read_axis_value(
    store: &DatasetStore,
    row_id: RowId,
    column_id: rawscope_core::ColumnId,
    expected_kind: StoreColumnKind,
) -> Result<AxisRead, VisualFieldProjectionError> {
    let cell = store
        .cell(row_id, column_id)
        .map_err(VisualFieldProjectionError::DatasetAccess)?;
    match cell.normalized() {
        CellState::Missing => Ok(AxisRead::Missing),
        CellState::Invalid(_) => Ok(AxisRead::Invalid),
        CellState::Value(normalized) => match normalized {
            NormalizedValue::I64(value) if expected_kind == StoreColumnKind::I64 => {
                Ok(AxisRead::Value(AxisValue::I64(*value)))
            }
            NormalizedValue::U64(value) if expected_kind == StoreColumnKind::U64 => {
                Ok(AxisRead::Value(AxisValue::U64(*value)))
            }
            NormalizedValue::F64(value) if expected_kind == StoreColumnKind::F64 => {
                Ok(AxisRead::Value(AxisValue::F64(*value)))
            }
            NormalizedValue::TimestampMicros(value)
                if expected_kind == StoreColumnKind::TimestampMicros =>
            {
                Ok(AxisRead::Value(AxisValue::TimestampMicros(*value)))
            }
            _ => Err(VisualFieldProjectionError::UnexpectedValueKind { row_id }),
        },
    }
}

fn column_kind(
    store: &DatasetStore,
    column_id: rawscope_core::ColumnId,
) -> Result<StoreColumnKind, VisualFieldProjectionError> {
    store
        .schema()
        .column(column_id)
        .map(|column| column.kind())
        .ok_or(VisualFieldProjectionError::Mapping(
            super::VisualFieldMappingError::MissingColumn { column_id },
        ))
}

fn matches_projection_kind(projection: VisualFieldProjection, x: AxisValue, y: AxisValue) -> bool {
    match projection {
        VisualFieldProjection::NumericPair { .. } => {
            matches!(x, AxisValue::I64(_) | AxisValue::U64(_) | AxisValue::F64(_))
                && matches!(y, AxisValue::I64(_) | AxisValue::U64(_) | AxisValue::F64(_))
        }
        VisualFieldProjection::TimeValue { .. } => {
            matches!(x, AxisValue::TimestampMicros(_))
                && matches!(y, AxisValue::I64(_) | AxisValue::U64(_) | AxisValue::F64(_))
        }
    }
}

fn domain_for_values(
    values: Vec<AxisValue>,
) -> Result<VisualAxisDomain, VisualFieldProjectionError> {
    let first = values
        .first()
        .copied()
        .ok_or(VisualFieldProjectionError::NoValidPoints)?;
    match first {
        AxisValue::I64(_) => {
            let (min, max) = values
                .iter()
                .fold((i64::MAX, i64::MIN), |(min, max), value| {
                    let AxisValue::I64(value) = value else {
                        return (min, max);
                    };
                    (min.min(*value), max.max(*value))
                });
            Ok(VisualAxisDomain::I64 { min, max })
        }
        AxisValue::U64(_) => {
            let (min, max) = values.iter().fold((u64::MAX, 0), |(min, max), value| {
                let AxisValue::U64(value) = value else {
                    return (min, max);
                };
                (min.min(*value), max.max(*value))
            });
            Ok(VisualAxisDomain::U64 { min, max })
        }
        AxisValue::F64(_) => {
            let (min, max) =
                values
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                        let AxisValue::F64(value) = value else {
                            return (min, max);
                        };
                        (min.min(*value), max.max(*value))
                    });
            let (domain_min, domain_max) = if min == max {
                singleton_f64_bounds(min).ok_or(VisualFieldProjectionError::NonIncreasingDomain)?
            } else {
                (min, max)
            };
            if !domain_min.is_finite() || !domain_max.is_finite() {
                return Err(VisualFieldProjectionError::NonIncreasingDomain);
            }
            F64Domain::try_new(domain_min, domain_max)
                .map(VisualAxisDomain::F64)
                .map_err(|_| VisualFieldProjectionError::NonIncreasingDomain)
        }
        AxisValue::TimestampMicros(_) => {
            let (min, max) = values
                .iter()
                .fold((i64::MAX, i64::MIN), |(min, max), value| {
                    let AxisValue::TimestampMicros(value) = value else {
                        return (min, max);
                    };
                    (min.min(*value), max.max(*value))
                });
            Ok(VisualAxisDomain::TimestampMicros { min, max })
        }
    }
}

fn singleton_f64_bounds(value: f64) -> Option<(f64, f64)> {
    if value == 0.0 {
        return Some((-1.0, 1.0));
    }
    let delta = (value.abs() * 0.01).max(1.0);
    let expanded = (value - delta, value + delta);
    if expanded.0.is_finite() && expanded.1.is_finite() && expanded.0 < expanded.1 {
        return Some(expanded);
    }
    let fallback = if value.is_sign_positive() {
        (value - value.abs() * 0.01, value)
    } else {
        (value, value + value.abs() * 0.01)
    };
    (fallback.0.is_finite() && fallback.1.is_finite() && fallback.0 < fallback.1)
        .then_some(fallback)
}

impl fmt::Display for VisualFieldProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Mapping(_) => "visual-field mapping is invalid",
            Self::DatasetAccess(_) => "visual-field store access failed",
            Self::MissingValue { .. } => "visual-field row is missing an axis value",
            Self::InvalidValue { .. } => "visual-field row has an invalid axis value",
            Self::UnexpectedValueKind { .. } => "visual-field axis value has an unexpected kind",
            Self::NonFinite { .. } => "visual-field axis value is not finite",
            Self::TimeValue(_) => "time-value timestamp transform is invalid",
            Self::NoValidPoints => "visual-field projection has no valid points",
            Self::NonIncreasingDomain => "visual-field projection domain is not increasing",
        })
    }
}

impl Error for VisualFieldProjectionError {}
