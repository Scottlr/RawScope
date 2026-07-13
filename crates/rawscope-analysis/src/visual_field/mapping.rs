//! Checked column bindings for generic two-dimensional visual fields.

use rawscope_core::ColumnId;
use rawscope_data::{DatasetFieldBinding, DatasetFieldRole, DatasetSchema, StoreColumnKind};

/// Maximum number of visible category composition layers, including reserved
/// Other/Missing/Invalid buckets.
pub const MAX_CATEGORY_COMPOSITION_LAYERS: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisualAxisKind {
    Numeric,
    TimestampMicros,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisualFieldProjection {
    NumericPair { x: ColumnId, y: ColumnId },
    TimeValue { time: ColumnId, value: ColumnId },
}

impl VisualFieldProjection {
    pub const fn x_column(self) -> ColumnId {
        match self {
            Self::NumericPair { x, .. } | Self::TimeValue { time: x, .. } => x,
        }
    }

    pub const fn y_column(self) -> ColumnId {
        match self {
            Self::NumericPair { y, .. } | Self::TimeValue { value: y, .. } => y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisualFieldMapping {
    projection: VisualFieldProjection,
    category: Option<ColumnId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualFieldMappingError {
    MissingColumn {
        column_id: ColumnId,
    },
    DuplicateAxes {
        column_id: ColumnId,
    },
    AxisKindMismatch {
        column_id: ColumnId,
        expected: VisualAxisKind,
    },
    CategoryKindMismatch {
        column_id: ColumnId,
    },
    CategoryMatchesAxis {
        column_id: ColumnId,
    },
    CategoryRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisualFieldMode {
    Density,
    CategoryComposition,
    CohortComparison,
    DensityRidges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualFieldModeSupport {
    Supported,
    CategoryRequired,
}

impl VisualFieldMapping {
    pub fn try_new(
        schema: &DatasetSchema,
        projection: VisualFieldProjection,
        category: Option<ColumnId>,
    ) -> Result<Self, VisualFieldMappingError> {
        let mapping = Self {
            projection,
            category,
        };
        mapping.validate_against(schema)?;
        Ok(mapping)
    }

    pub fn validate_against(self, schema: &DatasetSchema) -> Result<(), VisualFieldMappingError> {
        let x = self.projection.x_column();
        let y = self.projection.y_column();
        if x == y {
            return Err(VisualFieldMappingError::DuplicateAxes { column_id: x });
        }

        let x_kind = schema
            .column(x)
            .ok_or(VisualFieldMappingError::MissingColumn { column_id: x })?
            .kind();
        let y_kind = schema
            .column(y)
            .ok_or(VisualFieldMappingError::MissingColumn { column_id: y })?
            .kind();
        match self.projection {
            VisualFieldProjection::NumericPair { .. } => {
                require_numeric(x, x_kind)?;
                require_numeric(y, y_kind)?;
            }
            VisualFieldProjection::TimeValue { time, value } => {
                if x_kind != StoreColumnKind::TimestampMicros {
                    return Err(VisualFieldMappingError::AxisKindMismatch {
                        column_id: time,
                        expected: VisualAxisKind::TimestampMicros,
                    });
                }
                require_numeric(value, y_kind)?;
            }
        }

        if let Some(category) = self.category {
            let category_kind = schema
                .column(category)
                .ok_or(VisualFieldMappingError::MissingColumn {
                    column_id: category,
                })?
                .kind();
            if category == x || category == y {
                return Err(VisualFieldMappingError::CategoryMatchesAxis {
                    column_id: category,
                });
            }
            if !is_category(category_kind) {
                return Err(VisualFieldMappingError::CategoryKindMismatch {
                    column_id: category,
                });
            }
        }
        Ok(())
    }

    pub const fn projection(self) -> VisualFieldProjection {
        self.projection
    }

    pub const fn category(self) -> Option<ColumnId> {
        self.category
    }

    pub const fn x_column(self) -> ColumnId {
        self.projection.x_column()
    }

    pub const fn y_column(self) -> ColumnId {
        self.projection.y_column()
    }

    pub fn dataset_field_bindings(
        self,
        schema: &DatasetSchema,
    ) -> Result<Vec<DatasetFieldBinding>, VisualFieldMappingError> {
        self.validate_against(schema)?;
        let mut bindings = Vec::with_capacity(3);
        let column_name = |column_id| {
            schema
                .column(column_id)
                .map(|column| column.name().to_string())
                .ok_or(VisualFieldMappingError::MissingColumn { column_id })
        };
        match self.projection {
            VisualFieldProjection::NumericPair { x, y } => {
                bindings.push(DatasetFieldBinding::new(
                    DatasetFieldRole::X,
                    column_name(x)?,
                ));
                bindings.push(DatasetFieldBinding::new(
                    DatasetFieldRole::Y,
                    column_name(y)?,
                ));
            }
            VisualFieldProjection::TimeValue { time, value } => {
                bindings.push(DatasetFieldBinding::new(
                    DatasetFieldRole::Time,
                    column_name(time)?,
                ));
                bindings.push(DatasetFieldBinding::new(
                    DatasetFieldRole::Value,
                    column_name(value)?,
                ));
            }
        }
        if let Some(category) = self.category {
            bindings.push(DatasetFieldBinding::new(
                DatasetFieldRole::Category,
                column_name(category)?,
            ));
        }
        Ok(bindings)
    }

    pub const fn supports(self, mode: VisualFieldMode) -> VisualFieldModeSupport {
        match mode {
            VisualFieldMode::CategoryComposition if self.category.is_none() => {
                VisualFieldModeSupport::CategoryRequired
            }
            VisualFieldMode::Density
            | VisualFieldMode::CategoryComposition
            | VisualFieldMode::CohortComparison
            | VisualFieldMode::DensityRidges => VisualFieldModeSupport::Supported,
        }
    }

    pub const fn require_mode(self, mode: VisualFieldMode) -> Result<(), VisualFieldMappingError> {
        match self.supports(mode) {
            VisualFieldModeSupport::Supported => Ok(()),
            VisualFieldModeSupport::CategoryRequired => {
                Err(VisualFieldMappingError::CategoryRequired)
            }
        }
    }
}

fn require_numeric(
    column_id: ColumnId,
    kind: StoreColumnKind,
) -> Result<(), VisualFieldMappingError> {
    if is_numeric(kind) {
        Ok(())
    } else {
        Err(VisualFieldMappingError::AxisKindMismatch {
            column_id,
            expected: VisualAxisKind::Numeric,
        })
    }
}

const fn is_numeric(kind: StoreColumnKind) -> bool {
    matches!(
        kind,
        StoreColumnKind::I64 | StoreColumnKind::U64 | StoreColumnKind::F64
    )
}

const fn is_category(kind: StoreColumnKind) -> bool {
    matches!(
        kind,
        StoreColumnKind::Utf8 | StoreColumnKind::Bool | StoreColumnKind::I64 | StoreColumnKind::U64
    )
}
