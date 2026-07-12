//! Generation-owned typed rectangular dataset storage.

mod budget;
mod cell;
mod chunk;
mod schema;

use std::{error::Error, fmt};

use rawscope_core::{Generation, GenerationCounter, RowId};

use crate::DatasetIdentity;

pub use budget::{DatasetBudgetError, DatasetMemoryBudget, DatasetMemoryUsage};
pub use cell::{
    CellRef, CellState, DecodedCsvCell, InvalidCell, InvalidCellReason, NormalizedValue,
    SourceUnavailableReason, SourceValue, StoredCell,
};
pub use chunk::{ChunkValidationError, ColumnChunk, DatasetChunk};
pub use schema::{DatasetColumn, DatasetSchema, DatasetSchemaError, StoreColumnKind};

/// Marker preventing generations from different stores being mixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DatasetStoreOwner;

pub type DatasetGeneration = Generation<DatasetStoreOwner>;
pub type DatasetGenerationCounter = GenerationCounter<DatasetStoreOwner>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetStoreError {
    Chunk(ChunkValidationError),
    Budget(DatasetBudgetError),
    RowCountOverflow,
    MemoryOverflow,
}

impl fmt::Display for DatasetStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chunk(error) => write!(formatter, "invalid dataset chunk: {error}"),
            Self::Budget(error) => error.fmt(formatter),
            Self::RowCountOverflow => write!(formatter, "dataset row count overflowed u64"),
            Self::MemoryOverflow => write!(formatter, "dataset memory accounting overflowed u64"),
        }
    }
}

impl Error for DatasetStoreError {}

impl From<ChunkValidationError> for DatasetStoreError {
    fn from(error: ChunkValidationError) -> Self {
        Self::Chunk(error)
    }
}

/// Immutable typed rectangular store after successful builder completion.
#[derive(Debug, Clone, PartialEq)]
pub struct DatasetStore {
    identity: DatasetIdentity,
    schema: DatasetSchema,
    generation: DatasetGeneration,
    chunks: Vec<DatasetChunk>,
    row_count: u64,
    memory_usage: DatasetMemoryUsage,
}

impl DatasetStore {
    pub fn generation(&self) -> DatasetGeneration {
        self.generation
    }

    pub fn identity(&self) -> &DatasetIdentity {
        &self.identity
    }

    pub fn schema(&self) -> &DatasetSchema {
        &self.schema
    }

    pub fn row_count(&self) -> u64 {
        self.row_count
    }

    pub fn memory_usage(&self) -> DatasetMemoryUsage {
        self.memory_usage
    }

    pub fn chunks(&self) -> impl ExactSizeIterator<Item = &DatasetChunk> {
        self.chunks.iter()
    }

    pub fn source_value(
        &self,
        row: RowId,
        column: rawscope_core::ColumnId,
    ) -> Result<CellRef<'_>, DatasetAccessError> {
        let column_position = column.get() as usize;
        if self.schema.column(column).is_none() {
            return Err(DatasetAccessError::UnknownColumn { column });
        }
        let chunk = self
            .chunks
            .iter()
            .find(|chunk| {
                let start = chunk.row_id_start().0;
                let end = start.saturating_add(chunk.row_count() as u64);
                row.0 >= start && row.0 < end
            })
            .ok_or(DatasetAccessError::UnknownRow { row })?;
        let offset = usize::try_from(row.0 - chunk.row_id_start().0)
            .map_err(|_| DatasetAccessError::RowIndexOverflow { row })?;
        let column_chunk = chunk
            .columns()
            .get(column_position)
            .ok_or(DatasetAccessError::UnknownColumn { column })?;
        Ok(column_chunk
            .cells()
            .get(offset)
            .ok_or(DatasetAccessError::UnknownRow { row })?
            .as_cell_ref())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetAccessError {
    UnknownRow { row: RowId },
    UnknownColumn { column: rawscope_core::ColumnId },
    RowIndexOverflow { row: RowId },
}

impl fmt::Display for DatasetAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRow { row } => write!(formatter, "row {row:?} is not stored"),
            Self::UnknownColumn { column } => {
                write!(formatter, "column {column:?} is not in schema")
            }
            Self::RowIndexOverflow { row } => {
                write!(formatter, "row {row:?} cannot index this platform")
            }
        }
    }
}

impl Error for DatasetAccessError {}

pub struct DatasetStoreBuilder {
    identity: DatasetIdentity,
    schema: DatasetSchema,
    generation: DatasetGeneration,
    budget: DatasetMemoryBudget,
    chunks: Vec<DatasetChunk>,
    row_count: u64,
    memory_bytes: u64,
}

impl DatasetStoreBuilder {
    pub fn new(
        identity: DatasetIdentity,
        schema: DatasetSchema,
        budget: DatasetMemoryBudget,
        generations: &mut DatasetGenerationCounter,
    ) -> Self {
        Self {
            identity,
            schema,
            generation: generations.mint(),
            budget,
            chunks: Vec::new(),
            row_count: 0,
            memory_bytes: 0,
        }
    }

    pub fn append_chunk(&mut self, chunk: DatasetChunk) -> Result<(), DatasetStoreError> {
        let expected_row = RowId(self.row_count);
        chunk.validate_against(&self.schema, expected_row)?;
        let chunk_rows = chunk.row_count() as u64;
        let next_row_count = self
            .row_count
            .checked_add(chunk_rows)
            .ok_or(DatasetStoreError::RowCountOverflow)?;
        let chunk_bytes = chunk
            .estimated_bytes()
            .map_err(|_| DatasetStoreError::MemoryOverflow)?;
        let next_memory = self
            .memory_bytes
            .checked_add(chunk_bytes)
            .ok_or(DatasetStoreError::MemoryOverflow)?;
        if !self.budget.admits(next_memory) {
            return Err(DatasetStoreError::Budget(DatasetBudgetError {
                requested_bytes: next_memory,
                max_bytes: self.budget.max_bytes(),
            }));
        }

        self.chunks.push(chunk);
        self.row_count = next_row_count;
        self.memory_bytes = next_memory;
        Ok(())
    }

    pub fn finish(self) -> DatasetStore {
        DatasetStore {
            identity: self.identity,
            schema: self.schema,
            generation: self.generation,
            chunks: self.chunks,
            row_count: self.row_count,
            memory_usage: DatasetMemoryUsage::new(self.memory_bytes),
        }
    }
}
