//! Generation-bound source profiles with explicit bounded category accuracy.

use std::{collections::BTreeMap, error::Error, fmt, num::NonZeroUsize, sync::Arc};

use rawscope_core::ColumnId;
use rawscope_data::{DatasetGeneration, LoadedColumnKind, LoadedSourceTable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileConfig {
    max_category_values: NonZeroUsize,
}

impl ProfileConfig {
    pub fn new(max_category_values: usize) -> Result<Self, ProfileConfigError> {
        Ok(Self {
            max_category_values: NonZeroUsize::new(max_category_values)
                .ok_or(ProfileConfigError::ZeroCategoryBudget)?,
        })
    }

    pub const fn max_category_values(self) -> NonZeroUsize {
        self.max_category_values
    }
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            max_category_values: NonZeroUsize::new(256).expect("non-zero profile default"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileConfigError {
    ZeroCategoryBudget,
}

impl fmt::Display for ProfileConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("profile category budget must be positive")
    }
}

impl Error for ProfileConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileAccuracy {
    Exact,
    AtLeast,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryLeader {
    pub value: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnProfile {
    pub column_id: ColumnId,
    pub name: String,
    pub kind: LoadedColumnKind,
    pub valid_count: u64,
    pub missing_count: u64,
    pub invalid_count: u64,
    pub distinct_count: u64,
    pub category_accuracy: ProfileAccuracy,
    pub leaders: Arc<[CategoryLeader]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileIndex {
    dataset_generation: DatasetGeneration,
    columns: Arc<[ColumnProfile]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    ColumnIdOverflow { index: usize },
    RowCountOverflow,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColumnIdOverflow { index } => {
                write!(formatter, "profile column index {index} exceeds ColumnId")
            }
            Self::RowCountOverflow => formatter.write_str("profile count exceeded u64"),
        }
    }
}

impl Error for ProfileError {}

impl ProfileIndex {
    pub fn build(
        dataset_generation: DatasetGeneration,
        source: &LoadedSourceTable,
        config: ProfileConfig,
    ) -> Result<Self, ProfileError> {
        let columns = source
            .columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let column_id = ColumnId::new(
                    u32::try_from(index).map_err(|_| ProfileError::ColumnIdOverflow { index })?,
                );
                profile_column(source, index, column_id, column, config)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            dataset_generation,
            columns: columns.into(),
        })
    }

    pub const fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }

    pub fn columns(&self) -> &[ColumnProfile] {
        &self.columns
    }
}

fn profile_column(
    source: &LoadedSourceTable,
    column_index: usize,
    column_id: ColumnId,
    column: &rawscope_data::LoadedColumnSchema,
    config: ProfileConfig,
) -> Result<ColumnProfile, ProfileError> {
    let mut valid_count = 0_u64;
    let mut missing_count = 0_u64;
    let mut invalid_count = 0_u64;
    let mut distinct_count = 0_u64;
    let mut category_counts = BTreeMap::<String, u64>::new();
    let mut category_accuracy = ProfileAccuracy::Exact;
    let mut has_untracked_category = false;

    for row in &source.rows {
        let value = row
            .values
            .get(column_index)
            .map_or("", String::as_str)
            .trim();
        if value.is_empty() {
            missing_count = missing_count
                .checked_add(1)
                .ok_or(ProfileError::RowCountOverflow)?;
            continue;
        }
        let is_valid = match column.kind {
            LoadedColumnKind::Integer => {
                value.parse::<i64>().is_ok() || value.parse::<u64>().is_ok()
            }
            LoadedColumnKind::Float => value
                .parse::<f64>()
                .map(|number| number.is_finite())
                .unwrap_or(false),
            LoadedColumnKind::Empty | LoadedColumnKind::String | LoadedColumnKind::Unsupported => {
                true
            }
        };
        if !is_valid {
            invalid_count = invalid_count
                .checked_add(1)
                .ok_or(ProfileError::RowCountOverflow)?;
            continue;
        }
        valid_count = valid_count
            .checked_add(1)
            .ok_or(ProfileError::RowCountOverflow)?;
        if column.kind == LoadedColumnKind::String {
            if let Some(count) = category_counts.get_mut(value) {
                *count = count.checked_add(1).ok_or(ProfileError::RowCountOverflow)?;
            } else if category_counts.len() < config.max_category_values.get() {
                category_counts.insert(value.to_owned(), 1);
                distinct_count = distinct_count
                    .checked_add(1)
                    .ok_or(ProfileError::RowCountOverflow)?;
            } else {
                category_accuracy = ProfileAccuracy::AtLeast;
                if !has_untracked_category {
                    has_untracked_category = true;
                    distinct_count = distinct_count
                        .checked_add(1)
                        .ok_or(ProfileError::RowCountOverflow)?;
                }
            }
        }
    }

    let mut leaders = category_counts
        .into_iter()
        .map(|(value, count)| CategoryLeader { value, count })
        .collect::<Vec<_>>();
    leaders.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.value.cmp(&right.value))
    });
    Ok(ColumnProfile {
        column_id,
        name: column.name.clone(),
        kind: column.kind,
        valid_count,
        missing_count,
        invalid_count,
        distinct_count,
        category_accuracy,
        leaders: leaders.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_data::{DatasetGenerationCounter, LoadedColumnSchema, LoadedSourceRow};

    fn source() -> LoadedSourceTable {
        LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "lane".into(),
                kind: LoadedColumnKind::String,
            }],
            rows: ["beta", "alpha", "beta", "", "gamma"]
                .into_iter()
                .enumerate()
                .map(|(index, value)| LoadedSourceRow {
                    row_id: rawscope_core::RowId(index as u64),
                    values: vec![value.into()],
                })
                .collect(),
        }
    }

    #[test]
    fn profile_is_generation_bound_and_ties_are_deterministic() {
        let mut generations = DatasetGenerationCounter::default();
        let generation = generations.mint();
        let index =
            ProfileIndex::build(generation, &source(), ProfileConfig::new(2).unwrap()).unwrap();
        assert_eq!(index.dataset_generation(), generation);
        let column = &index.columns()[0];
        assert_eq!(column.missing_count, 1);
        assert_eq!(column.distinct_count, 3);
        assert_eq!(column.category_accuracy, ProfileAccuracy::AtLeast);
        assert_eq!(column.leaders[0].value, "beta");
        assert_eq!(column.leaders[1].value, "alpha");
    }

    #[test]
    fn zero_category_budget_is_rejected() {
        assert_eq!(
            ProfileConfig::new(0),
            Err(ProfileConfigError::ZeroCategoryBudget)
        );
    }
}
