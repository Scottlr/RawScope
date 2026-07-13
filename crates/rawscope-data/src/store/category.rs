//! Bounded, generation-owned category membership channels.

use std::{collections::BTreeMap, error::Error, fmt, num::NonZeroUsize, sync::Arc};

use rawscope_core::{ColumnId, RowId};

use super::{
    CellState, DatasetAccessError, DatasetGeneration, DatasetStore, NormalizedValue,
    StoreColumnKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CategoryValueId(u32);

impl CategoryValueId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryCodeKind {
    Value(CategoryValueId),
    Untracked,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryCodeLayout {
    pub tracked_value_count: u32,
    pub untracked_code: u32,
    pub missing_code: u32,
    pub invalid_code: u32,
}

impl CategoryCodeLayout {
    fn try_new(tracked_value_count: usize) -> Result<Self, CategoryIndexError> {
        let tracked_value_count = u32::try_from(tracked_value_count)
            .map_err(|_| CategoryIndexError::CodeSpaceOverflow)?;
        let untracked_code = tracked_value_count;
        let missing_code = untracked_code
            .checked_add(1)
            .ok_or(CategoryIndexError::CodeSpaceOverflow)?;
        let invalid_code = missing_code
            .checked_add(1)
            .ok_or(CategoryIndexError::CodeSpaceOverflow)?;
        Ok(Self {
            tracked_value_count,
            untracked_code,
            missing_code,
            invalid_code,
        })
    }

    pub fn decode(self, code: u32) -> CategoryCodeKind {
        if code < self.tracked_value_count {
            CategoryCodeKind::Value(CategoryValueId::new(code))
        } else if code == self.untracked_code {
            CategoryCodeKind::Untracked
        } else if code == self.missing_code {
            CategoryCodeKind::Missing
        } else {
            CategoryCodeKind::Invalid
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CategoryIndexValue {
    Utf8(Arc<str>),
    Bool(bool),
    I64(i64),
    U64(u64),
}

impl CategoryIndexValue {
    pub fn label(&self) -> Arc<str> {
        match self {
            Self::Utf8(value) => Arc::clone(value),
            Self::Bool(value) => Arc::<str>::from(value.to_string()),
            Self::I64(value) => Arc::<str>::from(value.to_string()),
            Self::U64(value) => Arc::<str>::from(value.to_string()),
        }
    }

    fn from_normalized(value: &NormalizedValue) -> Option<Self> {
        match value {
            NormalizedValue::Text(value) => Some(Self::Utf8(Arc::clone(value))),
            NormalizedValue::Bool(value) => Some(Self::Bool(*value)),
            NormalizedValue::I64(value) => Some(Self::I64(*value)),
            NormalizedValue::U64(value) => Some(Self::U64(*value)),
            NormalizedValue::F64(_) | NormalizedValue::TimestampMicros(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryIndexAccuracy {
    Exact,
    Estimated,
    Truncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedCategoryValue {
    pub id: CategoryValueId,
    pub value: CategoryIndexValue,
    pub label: Arc<str>,
    pub row_count: u64,
    pub accuracy: CategoryIndexAccuracy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryMembershipIndex {
    dataset_generation: DatasetGeneration,
    column_id: ColumnId,
    values: Arc<[IndexedCategoryValue]>,
    code_layout: CategoryCodeLayout,
    row_codes: Arc<[u32]>,
    untracked_row_count: u64,
    missing_row_count: u64,
    invalid_row_count: u64,
    accuracy: CategoryIndexAccuracy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryIndexError {
    UnknownColumn {
        column_id: ColumnId,
    },
    UnsupportedKind {
        column_id: ColumnId,
        kind: StoreColumnKind,
    },
    ZeroValueBudget,
    CodeSpaceOverflow,
    RowCountOverflow,
    DatasetAccess(DatasetAccessError),
}

impl fmt::Display for CategoryIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownColumn { column_id } => write!(
                formatter,
                "category column {column_id:?} is not in the dataset schema"
            ),
            Self::UnsupportedKind { column_id, kind } => write!(
                formatter,
                "category column {column_id:?} has unsupported kind {kind:?}"
            ),
            Self::ZeroValueBudget => formatter.write_str("category value budget must be positive"),
            Self::CodeSpaceOverflow => formatter.write_str("category code layout exceeds u32"),
            Self::RowCountOverflow => {
                formatter.write_str("category row count cannot fit this platform")
            }
            Self::DatasetAccess(error) => error.fmt(formatter),
        }
    }
}

impl Error for CategoryIndexError {}

impl From<DatasetAccessError> for CategoryIndexError {
    fn from(error: DatasetAccessError) -> Self {
        Self::DatasetAccess(error)
    }
}

impl CategoryMembershipIndex {
    pub fn build(
        store: &DatasetStore,
        column_id: ColumnId,
        max_values: NonZeroUsize,
    ) -> Result<Self, CategoryIndexError> {
        let column = store
            .schema()
            .column(column_id)
            .ok_or(CategoryIndexError::UnknownColumn { column_id })?;
        if !matches!(
            column.kind(),
            StoreColumnKind::Utf8
                | StoreColumnKind::Bool
                | StoreColumnKind::I64
                | StoreColumnKind::U64
        ) {
            return Err(CategoryIndexError::UnsupportedKind {
                column_id,
                kind: column.kind(),
            });
        }
        let mut candidates = BTreeMap::<CategoryIndexValue, u64>::new();
        let mut truncated = false;
        let row_count =
            usize::try_from(store.row_count()).map_err(|_| CategoryIndexError::RowCountOverflow)?;
        for row_number in 0..row_count {
            let row_id = RowId(row_number as u64);
            let cell = store.cell(row_id, column_id)?;
            let CellState::Value(value) = cell.normalized() else {
                continue;
            };
            let Some(value) = CategoryIndexValue::from_normalized(value) else {
                continue;
            };
            let next_count = candidates
                .get(&value)
                .copied()
                .unwrap_or_default()
                .saturating_add(1);
            if candidates.contains_key(&value) {
                candidates.insert(value, next_count);
            } else if candidates.len() < max_values.get() {
                candidates.insert(value, 1);
            } else {
                let Some((lowest_value, lowest_count)) = candidates
                    .iter()
                    .min_by_key(|(candidate, count)| (**count, *candidate))
                    .map(|(candidate, count)| (candidate.clone(), *count))
                else {
                    continue;
                };
                if next_count > lowest_count || (next_count == lowest_count && value < lowest_value)
                {
                    candidates.remove(&lowest_value);
                    candidates.insert(value, next_count);
                }
                truncated = true;
            }
        }
        let accuracy = if truncated {
            CategoryIndexAccuracy::Truncated
        } else {
            CategoryIndexAccuracy::Exact
        };
        let code_layout = CategoryCodeLayout::try_new(candidates.len())?;
        let values = candidates
            .keys()
            .enumerate()
            .map(|(index, value)| IndexedCategoryValue {
                id: CategoryValueId::new(index as u32),
                value: value.clone(),
                label: value.label(),
                row_count: 0,
                accuracy,
            })
            .collect::<Vec<_>>();
        let value_ids = values
            .iter()
            .map(|value| (value.value.clone(), value.id))
            .collect::<BTreeMap<_, _>>();
        let mut row_codes = Vec::with_capacity(row_count);
        let mut row_counts = vec![0_u64; values.len()];
        let mut untracked_row_count = 0_u64;
        let mut missing_row_count = 0_u64;
        let mut invalid_row_count = 0_u64;
        for row_number in 0..row_count {
            let row_id = RowId(row_number as u64);
            let cell = store.cell(row_id, column_id)?;
            let code = match cell.normalized() {
                CellState::Missing => {
                    missing_row_count = missing_row_count.saturating_add(1);
                    code_layout.missing_code
                }
                CellState::Invalid(_) => {
                    invalid_row_count = invalid_row_count.saturating_add(1);
                    code_layout.invalid_code
                }
                CellState::Value(value) => match CategoryIndexValue::from_normalized(value) {
                    Some(value) => match value_ids.get(&value).copied() {
                        Some(value_id) => {
                            row_counts[value_id.get() as usize] =
                                row_counts[value_id.get() as usize].saturating_add(1);
                            value_id.get()
                        }
                        None => {
                            untracked_row_count = untracked_row_count.saturating_add(1);
                            code_layout.untracked_code
                        }
                    },
                    None => {
                        invalid_row_count = invalid_row_count.saturating_add(1);
                        code_layout.invalid_code
                    }
                },
            };
            row_codes.push(code);
        }
        let values = values
            .into_iter()
            .zip(row_counts)
            .map(|(mut value, row_count)| {
                value.row_count = row_count;
                value
            })
            .collect::<Vec<_>>();
        Ok(Self {
            dataset_generation: store.generation(),
            column_id,
            values: values.into(),
            code_layout,
            row_codes: row_codes.into(),
            untracked_row_count,
            missing_row_count,
            invalid_row_count,
            accuracy,
        })
    }

    pub fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }
    pub const fn column_id(&self) -> ColumnId {
        self.column_id
    }
    pub fn values(&self) -> &[IndexedCategoryValue] {
        &self.values
    }
    pub const fn code_layout(&self) -> CategoryCodeLayout {
        self.code_layout
    }
    pub fn row_codes(&self) -> &[u32] {
        &self.row_codes
    }
    pub fn code(&self, row_id: RowId) -> Option<CategoryCodeKind> {
        self.row_codes
            .get(usize::try_from(row_id.0).ok()?)
            .copied()
            .map(|code| self.code_layout.decode(code))
    }
    pub fn untracked_row_count(&self) -> u64 {
        self.untracked_row_count
    }
    pub fn missing_row_count(&self) -> u64 {
        self.missing_row_count
    }
    pub fn invalid_row_count(&self) -> u64 {
        self.invalid_row_count
    }
    pub const fn accuracy(&self) -> CategoryIndexAccuracy {
        self.accuracy
    }
}
