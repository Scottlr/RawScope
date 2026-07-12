//! Immutable, generation-tagged filter cohorts.

use std::{error::Error, fmt, sync::Arc};

use rawscope_core::{Generation, GenerationCounter, RowId};
use rawscope_data::{
    evaluate_filters, DatasetFilter, DatasetGeneration, FilterError, FilterMask, FilterRevision,
    FilterSet, LoadedSourceTable, VisualFieldCatalog,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingValuePolicy {
    Exclude,
    Include,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidValuePolicy {
    RejectDataset,
    ExcludeWithDisclosure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CohortPolicy {
    pub missing: MissingValuePolicy,
    pub invalid: InvalidValuePolicy,
}

impl Default for CohortPolicy {
    fn default() -> Self {
        Self {
            missing: MissingValuePolicy::Exclude,
            invalid: InvalidValuePolicy::ExcludeWithDisclosure,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CohortOwner;

pub type CohortGeneration = Generation<CohortOwner>;
pub type CohortGenerationCounter = GenerationCounter<CohortOwner>;

#[derive(Debug, Clone, PartialEq)]
pub enum CohortError {
    Filter(FilterError),
    InvalidValue { row_id: RowId, column_name: String },
    RowCountOverflow,
}

impl fmt::Display for CohortError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Filter(error) => write!(formatter, "cohort filter failed: {error:?}"),
            Self::InvalidValue {
                row_id,
                column_name,
            } => write!(
                formatter,
                "invalid value in column '{column_name}' at row {row_id:?}"
            ),
            Self::RowCountOverflow => formatter.write_str("cohort row count overflowed u64"),
        }
    }
}

impl Error for CohortError {}

impl From<FilterError> for CohortError {
    fn from(error: FilterError) -> Self {
        Self::Filter(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CohortSnapshot {
    dataset_generation: DatasetGeneration,
    cohort_generation: CohortGeneration,
    filter_revision: FilterRevision,
    mask: Arc<[u32]>,
    included_row_ids: Arc<[RowId]>,
    included_row_count: u64,
    excluded_missing_count: u64,
    excluded_invalid_count: u64,
}

impl CohortSnapshot {
    pub fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }

    pub fn cohort_generation(&self) -> CohortGeneration {
        self.cohort_generation
    }

    pub fn filter_revision(&self) -> FilterRevision {
        self.filter_revision
    }

    pub fn mask(&self) -> &[u32] {
        &self.mask
    }

    pub fn filter_mask(&self) -> FilterMask {
        FilterMask::from_u32(self.mask.to_vec())
    }

    pub fn included_row_ids(&self) -> &[RowId] {
        &self.included_row_ids
    }

    pub fn included_row_count(&self) -> u64 {
        self.included_row_count
    }

    pub fn excluded_missing_count(&self) -> u64 {
        self.excluded_missing_count
    }

    pub fn excluded_invalid_count(&self) -> u64 {
        self.excluded_invalid_count
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CohortBuilder {
    filters: FilterSet,
    policy: CohortPolicy,
}

impl CohortBuilder {
    pub fn new(policy: CohortPolicy) -> Self {
        Self {
            filters: FilterSet::default(),
            policy,
        }
    }

    pub fn policy(&self) -> CohortPolicy {
        self.policy
    }

    pub fn filters(&self) -> &[DatasetFilter] {
        &self.filters.filters
    }

    pub fn replace_filter(&mut self, filter: DatasetFilter) {
        self.filters.replace_for_column(filter);
    }

    pub fn remove_column(&mut self, column_name: &str) -> bool {
        self.filters.remove_column(column_name)
    }

    pub fn clear(&mut self) {
        self.filters.clear();
    }

    pub fn evaluate(
        &self,
        source: &LoadedSourceTable,
        catalog: &VisualFieldCatalog,
        dataset_generation: DatasetGeneration,
        generations: &mut CohortGenerationCounter,
    ) -> Result<CohortSnapshot, CohortError> {
        let filters = filters_for_policy(&self.filters, self.policy.missing);
        if self.policy.invalid == InvalidValuePolicy::RejectDataset {
            reject_invalid_values(source, &filters)?;
        }
        let evaluation = evaluate_filters(source, catalog, &filters)?;
        let mask: Arc<[u32]> = evaluation.mask.as_gpu_u32_slice().to_vec().into();
        let included_row_ids: Arc<[RowId]> = source
            .rows
            .iter()
            .zip(mask.iter())
            .filter_map(|(row, included)| (*included == 1).then_some(row.row_id))
            .collect::<Vec<_>>()
            .into();
        let included_row_count =
            u64::try_from(included_row_ids.len()).map_err(|_| CohortError::RowCountOverflow)?;
        let excluded_count = u64::try_from(source.rows.len())
            .map_err(|_| CohortError::RowCountOverflow)?
            .saturating_sub(included_row_count);
        let (excluded_missing_count, excluded_invalid_count) =
            excluded_value_counts(source, &filters, &mask);
        Ok(CohortSnapshot {
            dataset_generation,
            cohort_generation: generations.mint(),
            filter_revision: filters.revision,
            mask,
            included_row_ids,
            included_row_count,
            excluded_missing_count,
            excluded_invalid_count: excluded_invalid_count.min(excluded_count),
        })
    }
}

fn filters_for_policy(filters: &FilterSet, policy: MissingValuePolicy) -> FilterSet {
    let include_missing = policy == MissingValuePolicy::Include;
    let revision = filters.revision;
    let filters = filters
        .filters
        .iter()
        .map(|filter| match filter {
            DatasetFilter::NumericRange {
                column_name,
                min_inclusive,
                max_inclusive,
                ..
            } => DatasetFilter::NumericRange {
                column_name: column_name.clone(),
                min_inclusive: *min_inclusive,
                max_inclusive: *max_inclusive,
                include_missing,
            },
            DatasetFilter::Categories {
                column_name,
                included_values,
                ..
            } => DatasetFilter::Categories {
                column_name: column_name.clone(),
                included_values: included_values.clone(),
                include_missing,
            },
        })
        .collect();
    FilterSet { filters, revision }
}

fn reject_invalid_values(
    source: &LoadedSourceTable,
    filters: &FilterSet,
) -> Result<(), CohortError> {
    for filter in &filters.filters {
        let DatasetFilter::NumericRange { column_name, .. } = filter else {
            continue;
        };
        let Some(column_index) = source
            .columns
            .iter()
            .position(|column| column.name == *column_name)
        else {
            continue;
        };
        for row in &source.rows {
            let value = row
                .values
                .get(column_index)
                .map(|value| value.trim())
                .unwrap_or("");
            if !value.is_empty()
                && value
                    .parse::<f64>()
                    .ok()
                    .is_none_or(|parsed| !parsed.is_finite())
            {
                return Err(CohortError::InvalidValue {
                    row_id: row.row_id,
                    column_name: column_name.clone(),
                });
            }
        }
    }
    Ok(())
}

fn excluded_value_counts(
    source: &LoadedSourceTable,
    filters: &FilterSet,
    mask: &[u32],
) -> (u64, u64) {
    source
        .rows
        .iter()
        .zip(mask.iter())
        .filter(|(_, included)| **included == 0)
        .fold((0, 0), |(missing, invalid), (row, _)| {
            let mut missing_row = false;
            let mut invalid_row = false;
            for filter in &filters.filters {
                let column_name = filter.column_name();
                let Some(column_index) = source
                    .columns
                    .iter()
                    .position(|column| column.name == column_name)
                else {
                    continue;
                };
                let value = row
                    .values
                    .get(column_index)
                    .map(|value| value.trim())
                    .unwrap_or("");
                if value.is_empty() {
                    missing_row = true;
                } else if matches!(filter, DatasetFilter::NumericRange { .. })
                    && value
                        .parse::<f64>()
                        .ok()
                        .is_none_or(|parsed| !parsed.is_finite())
                {
                    invalid_row = true;
                }
            }
            (
                missing + if missing_row { 1 } else { 0 },
                invalid + if invalid_row { 1 } else { 0 },
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_core::RowId;
    use rawscope_data::{
        DatasetGenerationCounter, LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow,
        VisualFieldCatalogConfig,
    };

    fn source() -> (LoadedSourceTable, VisualFieldCatalog) {
        let source = LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "rating".to_string(),
                kind: LoadedColumnKind::Float,
            }],
            rows: vec![
                LoadedSourceRow {
                    row_id: RowId(0),
                    values: vec![" 10.0 ".to_string()],
                },
                LoadedSourceRow {
                    row_id: RowId(1),
                    values: vec!["".to_string()],
                },
                LoadedSourceRow {
                    row_id: RowId(2),
                    values: vec!["oops".to_string()],
                },
            ],
        };
        let catalog =
            rawscope_data::build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
        (source, catalog)
    }

    #[test]
    fn cohort_snapshot_is_aligned_and_policy_counts_are_explicit() {
        let (source, catalog) = source();
        let mut builder = CohortBuilder::new(CohortPolicy::default());
        builder.replace_filter(DatasetFilter::NumericRange {
            column_name: "rating".to_string(),
            min_inclusive: 0.0,
            max_inclusive: 20.0,
            include_missing: false,
        });
        let mut generations = CohortGenerationCounter::default();
        let snapshot = builder
            .evaluate(
                &source,
                &catalog,
                DatasetGenerationCounter::default().mint(),
                &mut generations,
            )
            .unwrap();
        assert_eq!(snapshot.mask(), &[1, 0, 0]);
        assert_eq!(snapshot.included_row_ids(), &[RowId(0)]);
        assert_eq!(snapshot.excluded_missing_count(), 1);
        assert_eq!(snapshot.excluded_invalid_count(), 1);
    }

    #[test]
    fn reject_policy_surfaces_invalid_numeric_values() {
        let (source, catalog) = source();
        let mut builder = CohortBuilder::new(CohortPolicy {
            missing: MissingValuePolicy::Exclude,
            invalid: InvalidValuePolicy::RejectDataset,
        });
        builder.replace_filter(DatasetFilter::NumericRange {
            column_name: "rating".to_string(),
            min_inclusive: 0.0,
            max_inclusive: 20.0,
            include_missing: false,
        });
        let error = builder
            .evaluate(
                &source,
                &catalog,
                DatasetGenerationCounter::default().mint(),
                &mut CohortGenerationCounter::default(),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            CohortError::InvalidValue {
                row_id: RowId(2),
                ..
            }
        ));
    }
}
