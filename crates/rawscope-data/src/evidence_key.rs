//! Optional natural-key validation for bounded row evidence.

use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    fmt,
};

use rawscope_core::RowId;

use crate::{LoadedSourceRow, LoadedSourceTable};

/// Validated source-column descriptor for a natural evidence key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetEvidenceKey {
    pub column_name: String,
    pub column_index: usize,
}

impl DatasetEvidenceKey {
    /// Returns the original source value from one retained row in O(1) time.
    pub fn value<'a>(&self, row: &'a LoadedSourceRow) -> Option<&'a str> {
        row.values.get(self.column_index).map(String::as_str)
    }
}

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
        }
    }
}

impl Error for EvidenceKeyValidationError {}

/// Validates one source column as a unique, non-blank natural evidence key.
pub fn validate_evidence_key(
    source: &LoadedSourceTable,
    column_name: &str,
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
    let mut first_row_by_value = HashMap::<String, RowId>::with_capacity(source.rows.len());
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
            }
        }
    }
    Ok(key)
}
