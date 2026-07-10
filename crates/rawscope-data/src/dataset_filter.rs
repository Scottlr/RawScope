//! Deterministic AND-only filters over retained local source rows.

use std::{error::Error, fmt};

use rawscope_core::RowId;

use crate::{LoadedSourceTable, VisualFieldCatalog, VisualFieldSummary};

#[derive(Debug, Clone, PartialEq)]
pub enum DatasetFilter {
    NumericRange {
        column_name: String,
        min_inclusive: f64,
        max_inclusive: f64,
        include_missing: bool,
    },
    Categories {
        column_name: String,
        included_values: Vec<String>,
        include_missing: bool,
    },
}

impl DatasetFilter {
    pub fn column_name(&self) -> &str {
        match self {
            Self::NumericRange { column_name, .. } | Self::Categories { column_name, .. } => {
                column_name
            }
        }
    }

    fn normalize(&mut self) {
        if let Self::Categories {
            included_values, ..
        } = self
        {
            for value in included_values.iter_mut() {
                *value = value.trim().to_string();
            }
            included_values.sort();
            included_values.dedup();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FilterRevision(pub u64);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterSet {
    pub filters: Vec<DatasetFilter>,
    pub revision: FilterRevision,
}

impl FilterSet {
    pub fn replace_for_column(&mut self, mut filter: DatasetFilter) {
        filter.normalize();
        if let Some(existing) = self
            .filters
            .iter_mut()
            .find(|existing| existing.column_name() == filter.column_name())
        {
            if *existing != filter {
                *existing = filter;
                self.revision.0 += 1;
            }
        } else {
            self.filters.push(filter);
            self.revision.0 += 1;
        }
    }

    pub fn remove_column(&mut self, column_name: &str) -> bool {
        let original_len = self.filters.len();
        self.filters
            .retain(|filter| filter.column_name() != column_name);
        let changed = self.filters.len() != original_len;
        if changed {
            self.revision.0 += 1;
        }
        changed
    }

    pub fn clear(&mut self) {
        if !self.filters.is_empty() {
            self.filters.clear();
            self.revision.0 += 1;
        }
    }

    pub fn is_active(&self) -> bool {
        !self.filters.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterMask {
    included: Vec<u32>,
    included_count: usize,
}

impl FilterMask {
    pub fn all_included(row_count: usize) -> Self {
        Self {
            included: vec![1; row_count],
            included_count: row_count,
        }
    }

    pub fn includes(&self, row_id: RowId) -> bool {
        usize::try_from(row_id.0)
            .ok()
            .and_then(|index| self.included.get(index))
            .is_some_and(|value| *value == 1)
    }
    pub fn as_gpu_u32_slice(&self) -> &[u32] {
        &self.included
    }
    pub fn len(&self) -> usize {
        self.included.len()
    }
    pub fn is_empty(&self) -> bool {
        self.included.is_empty()
    }
    pub fn included_count(&self) -> usize {
        self.included_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterEvaluation {
    pub mask: FilterMask,
    pub included_count: usize,
    pub excluded_count: usize,
    pub revision: FilterRevision,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterError {
    UnknownColumn { column_name: String },
    IncompatibleColumnKind { column_name: String },
    InvalidNumericRange { column_name: String },
    NonContiguousRowIds { expected: RowId, actual: RowId },
    RowWidthMismatch { row_id: RowId },
}

pub fn evaluate_filters(
    source: &LoadedSourceTable,
    catalog: &VisualFieldCatalog,
    filters: &FilterSet,
) -> Result<FilterEvaluation, FilterError> {
    let resolved = filters
        .filters
        .iter()
        .map(|filter| resolve_filter(catalog, filter))
        .collect::<Result<Vec<_>, _>>()?;
    let mut included = Vec::with_capacity(source.rows.len());
    let mut included_count = 0;
    for (index, row) in source.rows.iter().enumerate() {
        let expected = RowId(index as u64);
        if row.row_id != expected {
            return Err(FilterError::NonContiguousRowIds {
                expected,
                actual: row.row_id,
            });
        }
        if row.values.len() != source.columns.len() {
            return Err(FilterError::RowWidthMismatch { row_id: row.row_id });
        }
        let matches = resolved.iter().all(|filter| filter.matches(&row.values));
        included.push(u32::from(matches));
        included_count += usize::from(matches);
    }
    Ok(FilterEvaluation {
        mask: FilterMask {
            included,
            included_count,
        },
        included_count,
        excluded_count: source.rows.len() - included_count,
        revision: filters.revision,
    })
}

enum ResolvedFilter {
    Numeric {
        index: usize,
        min: f64,
        max: f64,
        include_missing: bool,
    },
    Categories {
        index: usize,
        values: Vec<String>,
        include_missing: bool,
    },
}

impl ResolvedFilter {
    fn matches(&self, row: &[String]) -> bool {
        match self {
            Self::Numeric {
                index,
                min,
                max,
                include_missing,
            } => {
                let value = row[*index].trim();
                if value.is_empty() {
                    return *include_missing;
                }
                value
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite())
                    .map_or(*include_missing, |value| value >= *min && value <= *max)
            }
            Self::Categories {
                index,
                values,
                include_missing,
            } => {
                let value = row[*index].trim();
                if value.is_empty() {
                    *include_missing
                } else {
                    values
                        .binary_search_by(|candidate| candidate.as_str().cmp(value))
                        .is_ok()
                }
            }
        }
    }
}

fn resolve_filter(
    catalog: &VisualFieldCatalog,
    filter: &DatasetFilter,
) -> Result<ResolvedFilter, FilterError> {
    let column_name = filter.column_name();
    let field = catalog
        .fields
        .iter()
        .find(|field| field.column_name == column_name)
        .ok_or_else(|| FilterError::UnknownColumn {
            column_name: column_name.to_string(),
        })?;
    match filter {
        DatasetFilter::NumericRange {
            min_inclusive,
            max_inclusive,
            include_missing,
            ..
        } => {
            if !min_inclusive.is_finite()
                || !max_inclusive.is_finite()
                || min_inclusive > max_inclusive
            {
                return Err(FilterError::InvalidNumericRange {
                    column_name: column_name.to_string(),
                });
            }
            if !matches!(field.summary, VisualFieldSummary::Numeric(_)) {
                return Err(FilterError::IncompatibleColumnKind {
                    column_name: column_name.to_string(),
                });
            }
            Ok(ResolvedFilter::Numeric {
                index: field.column_index,
                min: *min_inclusive,
                max: *max_inclusive,
                include_missing: *include_missing,
            })
        }
        DatasetFilter::Categories {
            included_values,
            include_missing,
            ..
        } => {
            if !matches!(field.summary, VisualFieldSummary::Categorical(_)) {
                return Err(FilterError::IncompatibleColumnKind {
                    column_name: column_name.to_string(),
                });
            }
            let mut values = included_values
                .iter()
                .map(|value| value.trim().to_string())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            Ok(ResolvedFilter::Categories {
                index: field.column_index,
                values,
                include_missing: *include_missing,
            })
        }
    }
}

impl fmt::Display for FilterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownColumn { column_name } => {
                write!(formatter, "unknown filter column '{column_name}'")
            }
            Self::IncompatibleColumnKind { column_name } => write!(
                formatter,
                "filter kind is incompatible with column '{column_name}'"
            ),
            Self::InvalidNumericRange { column_name } => write!(
                formatter,
                "invalid inclusive numeric range for column '{column_name}'"
            ),
            Self::NonContiguousRowIds { expected, actual } => write!(
                formatter,
                "non-contiguous row ids: expected {} but found {}",
                expected.0, actual.0
            ),
            Self::RowWidthMismatch { row_id } => write!(
                formatter,
                "source row {} does not match the loaded schema width",
                row_id.0
            ),
        }
    }
}

impl Error for FilterError {}

#[cfg(test)]
#[path = "dataset_filter_tests.rs"]
mod tests;
