//! Optional natural-key validation for bounded row evidence.

use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    fmt,
};

use rawscope_core::{ColumnId, RowId};

use crate::{
    CellRef, DatasetGeneration, DatasetStore, LoadedSourceRow, LoadedSourceTable, SourceValue,
};

const INDEX_ENTRY_OVERHEAD_BYTES: u64 = 32;

/// Admission limit for the exact natural-key index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceKeyIndexBudget {
    max_bytes: u64,
}

impl EvidenceKeyIndexBudget {
    pub const fn new(max_bytes: u64) -> Self {
        Self { max_bytes }
    }

    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }
}

/// Validated source-column descriptor for a natural evidence key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetEvidenceKey {
    pub column_name: String,
    pub column_index: usize,
}

impl DatasetEvidenceKey {
    /// Validates a key against one immutable store generation.
    pub fn validate(
        store: &DatasetStore,
        column_id: ColumnId,
    ) -> Result<BoundDatasetEvidenceKey, EvidenceKeyValidationError> {
        let column = store.schema().column(column_id).ok_or_else(|| {
            EvidenceKeyValidationError::MissingColumn {
                column_name: format!("column-{column_id:?}"),
                available_columns: store
                    .schema()
                    .columns()
                    .iter()
                    .map(|column| column.name().to_string())
                    .collect(),
            }
        })?;
        let key = BoundDatasetEvidenceKey {
            generation: store.generation(),
            column_id,
            column_name: column.name().to_string(),
        };
        let mut first_row_by_value = HashMap::<String, RowId>::with_capacity(
            usize::try_from(store.row_count()).unwrap_or(0),
        );
        for row_number in 0..store.row_count() {
            let row_id = RowId(row_number);
            let value = match store.source_value(row_id, column_id).map_err(|_| {
                EvidenceKeyValidationError::MissingValue {
                    column_name: key.column_name.clone(),
                    row_id,
                }
            })? {
                CellRef::Value(SourceValue::Utf8(value)) => value.trim(),
                CellRef::Value(_)
                | CellRef::Missing
                | CellRef::Invalid(_)
                | CellRef::NotRetained(_) => {
                    return Err(EvidenceKeyValidationError::MissingValue {
                        column_name: key.column_name.clone(),
                        row_id,
                    });
                }
            };
            if value.is_empty() {
                return Err(EvidenceKeyValidationError::MissingValue {
                    column_name: key.column_name.clone(),
                    row_id,
                });
            }
            match first_row_by_value.entry(value.to_string()) {
                Entry::Occupied(entry) => {
                    return Err(EvidenceKeyValidationError::DuplicateValue {
                        column_name: key.column_name.clone(),
                        value: value.to_string(),
                        first: *entry.get(),
                        duplicate: row_id,
                    });
                }
                Entry::Vacant(entry) => {
                    entry.insert(row_id);
                }
            }
        }
        Ok(key)
    }

    /// Returns the original source value from one retained row in O(1) time.
    pub fn value<'a>(&self, row: &'a LoadedSourceRow) -> Option<&'a str> {
        row.values.get(self.column_index).map(String::as_str)
    }
}

/// A natural key bound to a specific immutable `DatasetStore` generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundDatasetEvidenceKey {
    generation: DatasetGeneration,
    column_id: ColumnId,
    column_name: String,
}

impl BoundDatasetEvidenceKey {
    pub fn generation(&self) -> DatasetGeneration {
        self.generation
    }

    pub fn column_id(&self) -> ColumnId {
        self.column_id
    }

    pub fn column_name(&self) -> &str {
        &self.column_name
    }

    pub fn value<'a>(
        &self,
        store: &'a DatasetStore,
        row_id: RowId,
    ) -> Result<CellRef<'a>, BoundEvidenceKeyError> {
        if store.generation() != self.generation {
            return Err(BoundEvidenceKeyError::GenerationMismatch {
                expected: self.generation,
                actual: store.generation(),
            });
        }
        store
            .source_value(row_id, self.column_id)
            .map_err(BoundEvidenceKeyError::Store)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundEvidenceKeyError {
    GenerationMismatch {
        expected: DatasetGeneration,
        actual: DatasetGeneration,
    },
    Store(crate::DatasetAccessError),
}

impl fmt::Display for BoundEvidenceKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GenerationMismatch { expected, actual } => write!(
                formatter,
                "evidence key belongs to generation {:?}, received {:?}",
                expected, actual
            ),
            Self::Store(error) => error.fmt(formatter),
        }
    }
}

impl Error for BoundEvidenceKeyError {}

/// Failure while validating a requested natural evidence key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceKeyValidationError {
    MissingColumn {
        column_name: String,
        available_columns: Vec<String>,
    },
    MissingValue {
        column_name: String,
        row_id: RowId,
    },
    DuplicateValue {
        column_name: String,
        value: String,
        first: RowId,
        duplicate: RowId,
    },
    BudgetExceeded {
        requested_bytes: u64,
        max_bytes: u64,
    },
}

impl fmt::Display for EvidenceKeyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingColumn {
                column_name,
                available_columns,
            } => write!(
                formatter,
                "missing evidence key column '{column_name}'; available columns: {}",
                available_columns.join(", ")
            ),
            Self::MissingValue { column_name, row_id } => write!(
                formatter,
                "evidence key column '{column_name}' is blank for row {row_id:?}"
            ),
            Self::DuplicateValue {
                column_name,
                value,
                first,
                duplicate,
            } => write!(
                formatter,
                "evidence key column '{column_name}' contains duplicate value '{value}' in rows {first:?} and {duplicate:?}"
            ),
            Self::BudgetExceeded {
                requested_bytes,
                max_bytes,
            } => write!(
                formatter,
                "evidence key index requires {requested_bytes} bytes, exceeding budget of {max_bytes} bytes"
            ),
        }
    }
}

impl Error for EvidenceKeyValidationError {}

/// Validates one source column as a unique, non-blank natural evidence key.
pub fn validate_evidence_key(
    source: &LoadedSourceTable,
    column_name: &str,
) -> Result<DatasetEvidenceKey, EvidenceKeyValidationError> {
    validate_evidence_key_with_budget(source, column_name, EvidenceKeyIndexBudget::new(u64::MAX))
}

/// Validates a natural key while admitting its exact index within a byte budget.
pub fn validate_evidence_key_with_budget(
    source: &LoadedSourceTable,
    column_name: &str,
    budget: EvidenceKeyIndexBudget,
) -> Result<DatasetEvidenceKey, EvidenceKeyValidationError> {
    let Some(column_index) = source
        .columns
        .iter()
        .position(|column| column.name == column_name)
    else {
        return Err(EvidenceKeyValidationError::MissingColumn {
            column_name: column_name.to_string(),
            available_columns: source.column_names().map(str::to_string).collect(),
        });
    };

    let key = DatasetEvidenceKey {
        column_name: column_name.to_string(),
        column_index,
    };
    let mut first_row_by_value = HashMap::<String, RowId>::new();
    let mut index_bytes = 0_u64;
    for row in &source.rows {
        let Some(value) = key.value(row) else {
            return Err(EvidenceKeyValidationError::MissingValue {
                column_name: key.column_name.clone(),
                row_id: row.row_id,
            });
        };
        let normalized_value = value.trim();
        if normalized_value.is_empty() {
            return Err(EvidenceKeyValidationError::MissingValue {
                column_name: key.column_name.clone(),
                row_id: row.row_id,
            });
        }
        match first_row_by_value.entry(normalized_value.to_string()) {
            Entry::Occupied(entry) => {
                return Err(EvidenceKeyValidationError::DuplicateValue {
                    column_name: key.column_name.clone(),
                    value: normalized_value.to_string(),
                    first: *entry.get(),
                    duplicate: row.row_id,
                });
            }
            Entry::Vacant(entry) => {
                entry.insert(row.row_id);
                index_bytes = index_bytes
                    .checked_add(
                        u64::try_from(normalized_value.len())
                            .unwrap_or(u64::MAX)
                            .saturating_add(INDEX_ENTRY_OVERHEAD_BYTES),
                    )
                    .ok_or(EvidenceKeyValidationError::BudgetExceeded {
                        requested_bytes: u64::MAX,
                        max_bytes: budget.max_bytes(),
                    })?;
                if index_bytes > budget.max_bytes() {
                    return Err(EvidenceKeyValidationError::BudgetExceeded {
                        requested_bytes: index_bytes,
                        max_bytes: budget.max_bytes(),
                    });
                }
            }
        }
    }
    Ok(key)
}
