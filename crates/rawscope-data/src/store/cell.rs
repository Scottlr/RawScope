//! Raw source, normalized analytical, and invalid/missing cell contracts.

use std::{fmt, sync::Arc};

use rawscope_core::{ColumnId, RowId};

#[derive(Debug, Clone, PartialEq)]
pub enum SourceValue {
    Utf8(Arc<str>),
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    TimestampMicros(i64),
}

impl SourceValue {
    pub fn estimated_bytes(&self) -> u64 {
        match self {
            Self::Utf8(value) => value.len() as u64,
            Self::Bool(_) => 1,
            Self::I64(_) | Self::U64(_) | Self::F64(_) | Self::TimestampMicros(_) => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NormalizedValue {
    Text(Arc<str>),
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidCellReason {
    ParseFailure,
    TypeMismatch,
    OutOfRange,
    SourceUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidCell {
    row_id: RowId,
    column_id: ColumnId,
    reason: InvalidCellReason,
}

impl InvalidCell {
    pub const fn new(row_id: RowId, column_id: ColumnId, reason: InvalidCellReason) -> Self {
        Self {
            row_id,
            column_id,
            reason,
        }
    }

    pub const fn row_id(self) -> RowId {
        self.row_id
    }

    pub const fn column_id(self) -> ColumnId {
        self.column_id
    }

    pub const fn reason(self) -> InvalidCellReason {
        self.reason
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceUnavailableReason {
    RetentionDisabled,
    EvictedByBudget,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellState<T> {
    Value(T),
    Missing,
    Invalid(InvalidCell),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredCell {
    raw: Option<SourceValue>,
    normalized: CellState<NormalizedValue>,
}

impl StoredCell {
    pub fn value(raw: SourceValue, normalized: NormalizedValue) -> Self {
        Self {
            raw: Some(raw),
            normalized: CellState::Value(normalized),
        }
    }

    pub fn missing(raw: Option<SourceValue>) -> Self {
        Self {
            raw,
            normalized: CellState::Missing,
        }
    }

    pub fn invalid(raw: Option<SourceValue>, invalid: InvalidCell) -> Self {
        Self {
            raw,
            normalized: CellState::Invalid(invalid),
        }
    }

    pub fn raw(&self) -> Option<&SourceValue> {
        self.raw.as_ref()
    }

    pub fn normalized(&self) -> &CellState<NormalizedValue> {
        &self.normalized
    }

    pub fn estimated_bytes(&self) -> u64 {
        self.raw
            .as_ref()
            .map_or(0, SourceValue::estimated_bytes)
            .saturating_add(1)
    }

    pub fn as_cell_ref(&self) -> CellRef<'_> {
        match &self.normalized {
            CellState::Value(_) => self.raw.as_ref().map_or(
                CellRef::NotRetained(SourceUnavailableReason::RetentionDisabled),
                CellRef::Value,
            ),
            CellState::Missing => CellRef::Missing,
            CellState::Invalid(invalid) => CellRef::Invalid(invalid),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellRef<'a> {
    Value(&'a SourceValue),
    Missing,
    Invalid(&'a InvalidCell),
    NotRetained(SourceUnavailableReason),
}

impl fmt::Display for InvalidCellReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::ParseFailure => "parse failure",
            Self::TypeMismatch => "type mismatch",
            Self::OutOfRange => "out of range",
            Self::SourceUnavailable => "source unavailable",
        };
        formatter.write_str(label)
    }
}
