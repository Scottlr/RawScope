//! Generation-bound missing/invalid column aggregates.

use std::{error::Error, fmt, sync::Arc};

use rawscope_core::ColumnId;
use rawscope_data::{LoadedColumnKind, LoadedSourceTable};

use crate::cohort::{CohortGeneration, CohortSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingnessColumn {
    pub column_id: ColumnId,
    pub missing_count: u64,
    pub invalid_count: u64,
    pub included_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingnessIndex {
    dataset_generation: rawscope_data::DatasetGeneration,
    cohort_generation: CohortGeneration,
    columns: Arc<[MissingnessColumn]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingnessError {
    RowMaskMismatch { rows: usize, mask: usize },
    ColumnIdOverflow { index: usize },
}

impl fmt::Display for MissingnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RowMaskMismatch { rows, mask } => {
                write!(
                    formatter,
                    "missingness rows ({rows}) do not match cohort mask ({mask})"
                )
            }
            Self::ColumnIdOverflow { index } => {
                write!(
                    formatter,
                    "column index {index} does not fit the stable column-id space"
                )
            }
        }
    }
}

impl Error for MissingnessError {}

impl MissingnessIndex {
    pub fn build(
        source: &LoadedSourceTable,
        cohort: &CohortSnapshot,
    ) -> Result<Self, MissingnessError> {
        if source.rows.len() != cohort.mask().len() {
            return Err(MissingnessError::RowMaskMismatch {
                rows: source.rows.len(),
                mask: cohort.mask().len(),
            });
        }
        let columns = source
            .columns
            .iter()
            .enumerate()
            .map(|(column_index, column)| {
                let column_id = ColumnId::new(u32::try_from(column_index).map_err(|_| {
                    MissingnessError::ColumnIdOverflow {
                        index: column_index,
                    }
                })?);
                let mut missing_count = 0;
                let mut invalid_count = 0;
                let mut included_count = 0;
                for (row, included) in source.rows.iter().zip(cohort.mask()) {
                    if *included != 1 {
                        continue;
                    }
                    included_count += 1;
                    let value = row
                        .values
                        .get(column_index)
                        .map(String::as_str)
                        .unwrap_or("");
                    if value.trim().is_empty() {
                        missing_count += 1;
                    } else if is_invalid(column.kind, value) {
                        invalid_count += 1;
                    }
                }
                Ok(MissingnessColumn {
                    column_id,
                    missing_count,
                    invalid_count,
                    included_count,
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into();
        Ok(Self {
            dataset_generation: cohort.dataset_generation(),
            cohort_generation: cohort.cohort_generation(),
            columns,
        })
    }

    pub fn dataset_generation(&self) -> rawscope_data::DatasetGeneration {
        self.dataset_generation
    }

    pub fn cohort_generation(&self) -> CohortGeneration {
        self.cohort_generation
    }

    pub fn columns(&self) -> &[MissingnessColumn] {
        &self.columns
    }
}

fn is_invalid(kind: LoadedColumnKind, value: &str) -> bool {
    match kind {
        LoadedColumnKind::Integer => value.parse::<i64>().is_err() && value.parse::<u64>().is_err(),
        LoadedColumnKind::Float => value
            .parse::<f64>()
            .map(|parsed| !parsed.is_finite())
            .unwrap_or(true),
        LoadedColumnKind::Empty | LoadedColumnKind::String | LoadedColumnKind::Unsupported => false,
    }
}

#[cfg(test)]
#[path = "missingness_tests.rs"]
mod tests;
