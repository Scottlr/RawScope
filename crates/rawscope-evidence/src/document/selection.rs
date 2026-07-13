//! Canonical selection membership and bounded row disclosure.

use std::{error::Error, fmt, sync::Arc};

use rawscope_analysis::selection::SelectionSnapshot;
use rawscope_core::{RowId, SelectionId};
use rawscope_data::{DatasetGeneration, DatasetIdentity};

use super::visual_field::{valid_visual_context, EvidenceContext, EvidenceVisualContext};
use rawscope_analysis::cohort::CohortGeneration;

const MAX_CANONICAL_SAMPLE_ROWS: usize = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceDocument {
    dataset_identity: DatasetIdentity,
    dataset_generation: DatasetGeneration,
    cohort_generation: CohortGeneration,
    selection_id: SelectionId,
    selected_row_ids: Arc<[RowId]>,
    selected_row_id_sample: Arc<[RowId]>,
    selected_bins: Arc<[u32]>,
    cohort_included_row_count: u64,
    source_rows_available: bool,
    visual: EvidenceVisualContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceValidationError {
    DatasetGenerationMismatch,
    CohortGenerationMismatch,
    SelectionCountExceedsCohort,
    SampleNotInSelection,
    SampleLimitExceeded,
    InvalidVisualContext,
}

impl fmt::Display for EvidenceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::DatasetGenerationMismatch => {
                "evidence dataset generation does not match selection"
            }
            Self::CohortGenerationMismatch => "evidence cohort generation does not match selection",
            Self::SelectionCountExceedsCohort => "evidence selection exceeds cohort membership",
            Self::SampleNotInSelection => {
                "evidence sample contains a row outside selection membership"
            }
            Self::SampleLimitExceeded => "evidence sample exceeds the canonical limit",
            Self::InvalidVisualContext => "evidence visual context is non-finite or empty",
        })
    }
}

impl Error for EvidenceValidationError {}

impl EvidenceDocument {
    pub fn from_selection(
        snapshot: &SelectionSnapshot,
        context: EvidenceContext,
    ) -> Result<Self, EvidenceValidationError> {
        if snapshot.dataset_generation() != context.dataset_generation {
            return Err(EvidenceValidationError::DatasetGenerationMismatch);
        }
        if snapshot.cohort_generation() != context.cohort_generation {
            return Err(EvidenceValidationError::CohortGenerationMismatch);
        }
        if snapshot.selected_count() > context.cohort_included_row_count {
            return Err(EvidenceValidationError::SelectionCountExceedsCohort);
        }
        if snapshot.samples().len() > MAX_CANONICAL_SAMPLE_ROWS {
            return Err(EvidenceValidationError::SampleLimitExceeded);
        }
        if snapshot
            .samples()
            .iter()
            .any(|row_id| snapshot.row_ids().binary_search(row_id).is_err())
        {
            return Err(EvidenceValidationError::SampleNotInSelection);
        }
        if !valid_visual_context(context.visual) {
            return Err(EvidenceValidationError::InvalidVisualContext);
        }
        Ok(Self {
            dataset_identity: context.dataset_identity,
            dataset_generation: context.dataset_generation,
            cohort_generation: context.cohort_generation,
            selection_id: snapshot.selection_id(),
            selected_row_ids: snapshot.row_ids().to_vec().into(),
            selected_row_id_sample: snapshot.samples().to_vec().into(),
            selected_bins: snapshot.selected_bins().to_vec().into(),
            cohort_included_row_count: context.cohort_included_row_count,
            source_rows_available: context.source_rows_available,
            visual: context.visual,
        })
    }

    pub fn dataset_identity(&self) -> &DatasetIdentity {
        &self.dataset_identity
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
    pub fn selected_row_ids(&self) -> &[RowId] {
        &self.selected_row_ids
    }
    pub fn selected_row_id_sample(&self) -> &[RowId] {
        &self.selected_row_id_sample
    }
    pub fn selected_bins(&self) -> &[u32] {
        &self.selected_bins
    }
    pub fn selected_count(&self) -> u64 {
        self.selected_row_ids.len() as u64
    }
    pub fn cohort_included_row_count(&self) -> u64 {
        self.cohort_included_row_count
    }
    pub fn source_rows_available(&self) -> bool {
        self.source_rows_available
    }
    pub fn visual_context(&self) -> EvidenceVisualContext {
        self.visual
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_analysis::cohort::CohortGenerationCounter;
    use rawscope_core::RowId;
    use rawscope_data::DatasetGenerationCounter;

    fn context(
        dataset_generation: DatasetGeneration,
        cohort_generation: CohortGeneration,
    ) -> EvidenceContext {
        EvidenceContext {
            dataset_generation,
            cohort_generation,
            dataset_identity: DatasetIdentity::synthetic_scatter(1, 10),
            cohort_included_row_count: 10,
            source_rows_available: true,
            visual: EvidenceVisualContext {
                x_min: 0.0,
                x_max: 1.0,
                y_min: 0.0,
                y_max: 1.0,
                grid_width: 16,
                grid_height: 16,
            },
        }
    }

    #[test]
    fn canonical_document_requires_matching_generations() {
        let mut dataset_generations = DatasetGenerationCounter::default();
        let dataset_generation = dataset_generations.mint();
        let cohort_generation = CohortGenerationCounter::default().mint();
        let snapshot = SelectionSnapshot::from_parts(
            dataset_generation,
            cohort_generation,
            rawscope_core::SelectionId(1),
            [RowId(2), RowId(3)],
            [4],
            8,
        )
        .unwrap();
        let other_generation = dataset_generations.mint();
        assert_eq!(
            EvidenceDocument::from_selection(
                &snapshot,
                context(other_generation, cohort_generation)
            ),
            Err(EvidenceValidationError::DatasetGenerationMismatch)
        );
    }

    #[test]
    fn canonical_document_rejects_invalid_visual_context() {
        let dataset_generation = DatasetGenerationCounter::default().mint();
        let cohort_generation = CohortGenerationCounter::default().mint();
        let snapshot = SelectionSnapshot::from_parts(
            dataset_generation,
            cohort_generation,
            rawscope_core::SelectionId(1),
            [RowId(2)],
            [],
            8,
        )
        .unwrap();
        let mut evidence_context = context(dataset_generation, cohort_generation);
        evidence_context.visual.x_max = f64::NAN;
        assert_eq!(
            EvidenceDocument::from_selection(&snapshot, evidence_context),
            Err(EvidenceValidationError::InvalidVisualContext)
        );
    }
}
