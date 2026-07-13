//! Runtime-neutral staged ingestion into the checked dataset store.

use std::{error::Error, fmt, num::NonZeroUsize};

use rawscope_core::{ColumnId, RowId};

use crate::{
    CellState, ColumnChunk, DatasetChunk, DatasetGenerationCounter, DatasetIdentity,
    DatasetMemoryBudget, DatasetSchema, DatasetStore, DatasetStoreBuilder, DatasetStoreError,
    DecodedCsvCell, InvalidCell, InvalidCellReason, LoadedColumnKind, LoadedSourceTable,
    NormalizedValue, StoreColumnKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkRowCount(NonZeroUsize);

impl ChunkRowCount {
    pub fn new(rows: usize) -> Option<Self> {
        NonZeroUsize::new(rows).map(Self)
    }

    pub const fn get(self) -> usize {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestionProgress {
    pub rows_processed: u64,
    pub total_rows: u64,
}

pub trait CancellationCheck {
    fn is_cancelled(&self) -> bool;
}

impl<F> CancellationCheck for F
where
    F: Fn() -> bool,
{
    fn is_cancelled(&self) -> bool {
        self()
    }
}

#[derive(Debug)]
pub struct IngestionRequest {
    pub identity: DatasetIdentity,
    pub source: LoadedSourceTable,
    pub chunk_rows: ChunkRowCount,
    pub memory_budget: DatasetMemoryBudget,
}

#[derive(Debug)]
pub struct IngestionPlan {
    identity: DatasetIdentity,
    source: LoadedSourceTable,
    schema: DatasetSchema,
    chunk_rows: ChunkRowCount,
    memory_budget: DatasetMemoryBudget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestionError {
    EmptySource,
    RowIdMismatch {
        expected: RowId,
        actual: RowId,
    },
    RowWidthMismatch {
        row_id: RowId,
        expected: usize,
        actual: usize,
    },
    Schema(crate::DatasetSchemaError),
    Store(DatasetStoreError),
    InvalidCell {
        row_id: RowId,
        column_id: ColumnId,
    },
    Cancelled {
        rows_processed: u64,
    },
    RowCountOverflow,
}

impl fmt::Display for IngestionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySource => formatter.write_str("ingestion source contains no rows"),
            Self::RowIdMismatch { expected, actual } => {
                write!(
                    formatter,
                    "row id {actual:?} is not the expected {expected:?}"
                )
            }
            Self::RowWidthMismatch {
                row_id,
                expected,
                actual,
            } => write!(
                formatter,
                "row {row_id:?} has {actual} cells; expected {expected}"
            ),
            Self::Schema(error) => write!(formatter, "invalid ingestion schema: {error}"),
            Self::Store(error) => write!(formatter, "store rejected ingestion chunk: {error}"),
            Self::InvalidCell { row_id, column_id } => {
                write!(formatter, "invalid cell at {row_id:?}/{column_id:?}")
            }
            Self::Cancelled { rows_processed } => {
                write!(formatter, "ingestion cancelled after {rows_processed} rows")
            }
            Self::RowCountOverflow => formatter.write_str("ingestion row count overflowed u64"),
        }
    }
}

impl Error for IngestionError {}

impl From<crate::DatasetSchemaError> for IngestionError {
    fn from(error: crate::DatasetSchemaError) -> Self {
        Self::Schema(error)
    }
}

impl From<DatasetStoreError> for IngestionError {
    fn from(error: DatasetStoreError) -> Self {
        Self::Store(error)
    }
}

impl IngestionPlan {
    pub fn prepare(request: IngestionRequest) -> Result<Self, IngestionError> {
        if request.source.rows.is_empty() {
            return Err(IngestionError::EmptySource);
        }
        let expected_width = request.source.columns.len();
        for (index, row) in request.source.rows.iter().enumerate() {
            let expected_row =
                RowId(u64::try_from(index).map_err(|_| IngestionError::RowCountOverflow)?);
            if row.row_id != expected_row {
                return Err(IngestionError::RowIdMismatch {
                    expected: expected_row,
                    actual: row.row_id,
                });
            }
            if row.values.len() != expected_width {
                return Err(IngestionError::RowWidthMismatch {
                    row_id: row.row_id,
                    expected: expected_width,
                    actual: row.values.len(),
                });
            }
        }
        let schema = DatasetSchema::try_new(request.source.columns.iter().enumerate().map(
            |(column_index, column)| {
                (
                    column.name.clone(),
                    store_kind(&request.source, column_index, column.kind),
                )
            },
        ))?;
        Ok(Self {
            identity: request.identity,
            source: request.source,
            schema,
            chunk_rows: request.chunk_rows,
            memory_budget: request.memory_budget,
        })
    }

    pub fn schema(&self) -> &DatasetSchema {
        &self.schema
    }

    pub fn execute<C, P>(
        self,
        cancel: &C,
        mut on_progress: P,
        generations: &mut DatasetGenerationCounter,
    ) -> Result<DatasetStore, IngestionError>
    where
        C: CancellationCheck,
        P: FnMut(IngestionProgress),
    {
        let total_rows =
            u64::try_from(self.source.rows.len()).map_err(|_| IngestionError::RowCountOverflow)?;
        let mut builder = DatasetStoreBuilder::new(
            self.identity.clone(),
            self.schema.clone(),
            self.memory_budget,
            generations,
        );
        let mut rows_processed = 0_u64;
        for chunk_start in (0..self.source.rows.len()).step_by(self.chunk_rows.get()) {
            if cancel.is_cancelled() {
                return Err(IngestionError::Cancelled { rows_processed });
            }
            let chunk_end = chunk_start
                .checked_add(self.chunk_rows.get())
                .ok_or(IngestionError::RowCountOverflow)?
                .min(self.source.rows.len());
            let chunk = self.build_chunk(chunk_start, chunk_end)?;
            builder.append_chunk(chunk)?;
            rows_processed =
                u64::try_from(chunk_end).map_err(|_| IngestionError::RowCountOverflow)?;
            on_progress(IngestionProgress {
                rows_processed,
                total_rows,
            });
        }
        Ok(builder.finish())
    }

    fn build_chunk(&self, start: usize, end: usize) -> Result<DatasetChunk, IngestionError> {
        let row_id_start =
            RowId(u64::try_from(start).map_err(|_| IngestionError::RowCountOverflow)?);
        let columns = self
            .source
            .columns
            .iter()
            .enumerate()
            .map(|(column_index, _column)| {
                let column_id = ColumnId::new(u32::try_from(column_index).unwrap_or(u32::MAX));
                let cells = self.source.rows[start..end]
                    .iter()
                    .map(|row| {
                        decoded_cell(
                            row.row_id,
                            column_id,
                            self.schema.columns()[column_index].kind(),
                            row.values[column_index].as_str(),
                        )
                        .into_stored_cell()
                    })
                    .collect();
                ColumnChunk::new(column_id, cells)
            })
            .collect();
        Ok(DatasetChunk::new(row_id_start, columns))
    }
}

fn store_kind(
    source: &LoadedSourceTable,
    column_index: usize,
    kind: LoadedColumnKind,
) -> StoreColumnKind {
    match kind {
        LoadedColumnKind::Integer => {
            let integer_is_signed = source.rows.iter().all(|row| {
                row.values.get(column_index).is_none_or(|value| {
                    value.trim().is_empty() || value.trim().parse::<i64>().is_ok()
                })
            });
            if integer_is_signed {
                StoreColumnKind::I64
            } else {
                StoreColumnKind::U64
            }
        }
        LoadedColumnKind::Float => StoreColumnKind::F64,
        LoadedColumnKind::Empty | LoadedColumnKind::String | LoadedColumnKind::Unsupported => {
            StoreColumnKind::Utf8
        }
    }
}

fn decoded_cell(
    row_id: RowId,
    column_id: ColumnId,
    kind: StoreColumnKind,
    raw: &str,
) -> DecodedCsvCell {
    let analytical = if raw.trim().is_empty() {
        CellState::Missing
    } else {
        match kind {
            StoreColumnKind::I64 => raw
                .trim()
                .parse::<i64>()
                .map(NormalizedValue::I64)
                .map(CellState::Value)
                .unwrap_or_else(|_| {
                    CellState::Invalid(InvalidCell::new(
                        row_id,
                        column_id,
                        InvalidCellReason::ParseFailure,
                    ))
                }),
            StoreColumnKind::U64 => raw
                .trim()
                .parse::<u64>()
                .map(NormalizedValue::U64)
                .map(CellState::Value)
                .unwrap_or_else(|_| {
                    CellState::Invalid(InvalidCell::new(
                        row_id,
                        column_id,
                        InvalidCellReason::ParseFailure,
                    ))
                }),
            StoreColumnKind::F64 => raw
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(NormalizedValue::F64)
                .map(CellState::Value)
                .unwrap_or_else(|| {
                    CellState::Invalid(InvalidCell::new(
                        row_id,
                        column_id,
                        InvalidCellReason::ParseFailure,
                    ))
                }),
            StoreColumnKind::TimestampMicros => raw
                .trim()
                .parse::<i64>()
                .map(NormalizedValue::TimestampMicros)
                .map(CellState::Value)
                .unwrap_or_else(|_| {
                    CellState::Invalid(InvalidCell::new(
                        row_id,
                        column_id,
                        InvalidCellReason::ParseFailure,
                    ))
                }),
            StoreColumnKind::Utf8 | StoreColumnKind::Bool => {
                CellState::Value(NormalizedValue::Text(raw.trim().into()))
            }
        }
    };
    DecodedCsvCell::new(raw, analytical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_decoding_keeps_raw_spelling_separate_from_normalized_value() {
        let stored = decoded_cell(RowId(4), ColumnId::new(2), StoreColumnKind::I64, "  0042 ")
            .into_stored_cell();

        assert_eq!(stored.raw_text(), Some("  0042 "));
        assert_eq!(
            stored.normalized(),
            &CellState::Value(NormalizedValue::I64(42))
        );
    }

    #[test]
    fn csv_invalid_and_missing_cells_keep_their_decoded_source_text() {
        let invalid = decoded_cell(RowId(5), ColumnId::new(1), StoreColumnKind::U64, " nope ")
            .into_stored_cell();
        let missing =
            decoded_cell(RowId(6), ColumnId::new(1), StoreColumnKind::F64, "  ").into_stored_cell();

        assert_eq!(invalid.raw_text(), Some(" nope "));
        assert!(matches!(invalid.normalized(), CellState::Invalid(_)));
        assert_eq!(missing.raw_text(), Some("  "));
        assert_eq!(missing.normalized(), &CellState::Missing);
    }

    #[test]
    fn timestamp_cells_keep_exact_micros_in_normalized_state() {
        let timestamp = decoded_cell(
            RowId(7),
            ColumnId::new(2),
            StoreColumnKind::TimestampMicros,
            "-9007199254740992",
        );
        assert_eq!(
            timestamp.analytical(),
            &CellState::Value(NormalizedValue::TimestampMicros(-9_007_199_254_740_992))
        );

        let invalid = decoded_cell(
            RowId(8),
            ColumnId::new(2),
            StoreColumnKind::TimestampMicros,
            "not-a-timestamp",
        );
        assert!(matches!(invalid.analytical(), CellState::Invalid(_)));
    }
}
