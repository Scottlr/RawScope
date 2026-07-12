//! Canonical immutable selection snapshots for analytical consumers.

use std::{error::Error, fmt, sync::Arc};

use rawscope_core::{RowId, SelectionId};
use rawscope_data::DatasetGeneration;

use crate::cohort::CohortGeneration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSnapshot {
    dataset_generation: DatasetGeneration,
    cohort_generation: CohortGeneration,
    selection_id: SelectionId,
    row_ids: Arc<[RowId]>,
    selected_bins: Arc<[u32]>,
    samples: Arc<[RowId]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionSnapshotError {
    RowCountOverflow,
}

impl fmt::Display for SelectionSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RowCountOverflow => formatter.write_str("selected row count overflowed u64"),
        }
    }
}

impl Error for SelectionSnapshotError {}

impl SelectionSnapshot {
    pub fn from_parts(
        dataset_generation: DatasetGeneration,
        cohort_generation: CohortGeneration,
        selection_id: SelectionId,
        row_ids: impl IntoIterator<Item = RowId>,
        selected_bins: impl IntoIterator<Item = u32>,
        sample_limit: usize,
    ) -> Result<Self, SelectionSnapshotError> {
        let mut row_ids = row_ids.into_iter().collect::<Vec<_>>();
        row_ids.sort_unstable();
        row_ids.dedup();
        if u64::try_from(row_ids.len()).is_err() {
            return Err(SelectionSnapshotError::RowCountOverflow);
        }
        let samples = row_ids
            .iter()
            .copied()
            .take(sample_limit)
            .collect::<Vec<_>>();
        let mut selected_bins = selected_bins.into_iter().collect::<Vec<_>>();
        selected_bins.sort_unstable();
        selected_bins.dedup();
        Ok(Self {
            dataset_generation,
            cohort_generation,
            selection_id,
            row_ids: row_ids.into(),
            selected_bins: selected_bins.into(),
            samples: samples.into(),
        })
    }

    pub fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }

    pub fn cohort_generation(&self) -> CohortGeneration {
        self.cohort_generation
    }

    pub fn selection_id(&self) -> SelectionId {
        self.selection_id
    }

    pub fn row_ids(&self) -> &[RowId] {
        &self.row_ids
    }

    pub fn selected_bins(&self) -> &[u32] {
        &self.selected_bins
    }

    pub fn samples(&self) -> &[RowId] {
        &self.samples
    }

    pub fn selected_count(&self) -> u64 {
        self.row_ids.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_data::DatasetGenerationCounter;

    use crate::cohort::CohortGenerationCounter;

    #[test]
    fn snapshot_canonicalizes_membership_and_bounds_samples() {
        let snapshot = SelectionSnapshot::from_parts(
            DatasetGenerationCounter::default().mint(),
            CohortGenerationCounter::default().mint(),
            SelectionId(4),
            [RowId(9), RowId(2), RowId(2), RowId(7)],
            [8, 1, 8],
            2,
        )
        .unwrap();
        assert_eq!(snapshot.row_ids(), &[RowId(2), RowId(7), RowId(9)]);
        assert_eq!(snapshot.selected_bins(), &[1, 8]);
        assert_eq!(snapshot.samples(), &[RowId(2), RowId(7)]);
        assert_eq!(snapshot.selected_count(), 3);
    }

    #[test]
    fn zero_sample_limit_preserves_membership_and_bins_without_samples() {
        let snapshot = SelectionSnapshot::from_parts(
            DatasetGenerationCounter::default().mint(),
            CohortGenerationCounter::default().mint(),
            SelectionId(1),
            [RowId(3), RowId(1)],
            [4, 2],
            0,
        )
        .unwrap();
        assert_eq!(snapshot.row_ids(), &[RowId(1), RowId(3)]);
        assert_eq!(snapshot.selected_bins(), &[2, 4]);
        assert!(snapshot.samples().is_empty());
    }
}
