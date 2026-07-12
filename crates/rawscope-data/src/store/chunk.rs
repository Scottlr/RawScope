//! Rectangular chunk contracts owned by `DatasetStore`.

use std::{error::Error, fmt};

use rawscope_core::{ColumnId, RowId};

use super::{DatasetColumn, DatasetSchema, StoredCell};

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnChunk {
    column_id: ColumnId,
    cells: Vec<StoredCell>,
}

impl ColumnChunk {
    pub fn new(column_id: ColumnId, cells: Vec<StoredCell>) -> Self {
        Self { column_id, cells }
    }

    pub fn column_id(&self) -> ColumnId {
        self.column_id
    }

    pub fn cells(&self) -> &[StoredCell] {
        &self.cells
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatasetChunk {
    row_id_start: RowId,
    columns: Vec<ColumnChunk>,
}

impl DatasetChunk {
    pub fn new(row_id_start: RowId, columns: Vec<ColumnChunk>) -> Self {
        Self {
            row_id_start,
            columns,
        }
    }

    pub fn row_id_start(&self) -> RowId {
        self.row_id_start
    }

    pub fn columns(&self) -> &[ColumnChunk] {
        &self.columns
    }

    pub fn row_count(&self) -> usize {
        self.columns.first().map_or(0, ColumnChunk::len)
    }

    pub fn estimated_bytes(&self) -> Result<u64, ChunkValidationError> {
        self.columns.iter().try_fold(0_u64, |total, column| {
            let cell_bytes = column.cells.iter().try_fold(0_u64, |subtotal, cell| {
                subtotal
                    .checked_add(cell.estimated_bytes())
                    .ok_or(ChunkValidationError::MemoryOverflow)
            })?;
            total
                .checked_add(cell_bytes)
                .ok_or(ChunkValidationError::MemoryOverflow)
        })
    }

    pub fn validate_against(
        &self,
        schema: &DatasetSchema,
        expected_row_id: RowId,
    ) -> Result<(), ChunkValidationError> {
        if self.columns.len() != schema.len() {
            return Err(ChunkValidationError::ColumnCountMismatch {
                expected: schema.len(),
                actual: self.columns.len(),
            });
        }
        if self.row_count() == 0 {
            return Err(ChunkValidationError::Empty);
        }
        if self.row_id_start != expected_row_id {
            return Err(ChunkValidationError::NonContiguousRows {
                expected: expected_row_id,
                actual: self.row_id_start,
            });
        }
        let row_count = self.row_count();
        for (position, column) in self.columns.iter().enumerate() {
            let expected = schema
                .columns()
                .get(position)
                .map(DatasetColumn::id)
                .ok_or(ChunkValidationError::ColumnCountMismatch {
                    expected: schema.len(),
                    actual: self.columns.len(),
                })?;
            if column.column_id != expected {
                return Err(ChunkValidationError::ColumnOrderMismatch {
                    position,
                    expected,
                    actual: column.column_id,
                });
            }
            if column.len() != row_count {
                return Err(ChunkValidationError::Ragged {
                    column_id: column.column_id,
                    expected: row_count,
                    actual: column.len(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkValidationError {
    Empty,
    ColumnCountMismatch {
        expected: usize,
        actual: usize,
    },
    ColumnOrderMismatch {
        position: usize,
        expected: ColumnId,
        actual: ColumnId,
    },
    Ragged {
        column_id: ColumnId,
        expected: usize,
        actual: usize,
    },
    NonContiguousRows {
        expected: RowId,
        actual: RowId,
    },
    MemoryOverflow,
}

impl fmt::Display for ChunkValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "dataset chunk must contain at least one row"),
            Self::ColumnCountMismatch { expected, actual } => {
                write!(
                    formatter,
                    "chunk has {actual} columns; schema requires {expected}"
                )
            }
            Self::ColumnOrderMismatch {
                position,
                expected,
                actual,
            } => write!(
                formatter,
                "chunk column {position} has id {actual:?}; expected {expected:?}"
            ),
            Self::Ragged {
                column_id,
                expected,
                actual,
            } => write!(
                formatter,
                "chunk column {column_id:?} has {actual} cells; expected {expected}"
            ),
            Self::NonContiguousRows { expected, actual } => write!(
                formatter,
                "chunk starts at row {actual:?}; next contiguous row is {expected:?}"
            ),
            Self::MemoryOverflow => write!(formatter, "chunk memory accounting overflowed u64"),
        }
    }
}

impl Error for ChunkValidationError {}
