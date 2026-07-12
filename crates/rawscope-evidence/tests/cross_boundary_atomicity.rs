use rawscope_analysis::{cohort::CohortGenerationCounter, selection::SelectionSnapshot};
use rawscope_core::{RowId, SelectionId};
use rawscope_data::{DatasetGeneration, DatasetGenerationCounter, DatasetIdentity};
use rawscope_evidence::{EvidenceContext, EvidenceDocument, EvidenceVisualContext};

fn context(
    dataset_generation: DatasetGeneration,
    cohort_generation: rawscope_analysis::cohort::CohortGeneration,
    cohort_included_row_count: u64,
) -> EvidenceContext {
    EvidenceContext {
        dataset_generation,
        cohort_generation,
        dataset_identity: DatasetIdentity::synthetic_scatter(
            42,
            usize::try_from(cohort_included_row_count).expect("fixture row count fits usize"),
        ),
        cohort_included_row_count,
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
fn evidence_boundary_preserves_complete_membership_beyond_sample_cap() {
    let dataset_generation = DatasetGenerationCounter::default().mint();
    let cohort_generation = CohortGenerationCounter::default().mint();
    let snapshot = SelectionSnapshot::from_parts(
        dataset_generation,
        cohort_generation,
        SelectionId(7),
        [RowId(9), RowId(2), RowId(7)],
        [8, 1, 8],
        1,
    )
    .expect("bounded selection snapshot should validate");

    let document = EvidenceDocument::from_selection(
        &snapshot,
        context(dataset_generation, cohort_generation, 3),
    )
    .expect("matching evidence context should commit");

    assert_eq!(document.selected_row_ids(), &[RowId(2), RowId(7), RowId(9)]);
    assert_eq!(document.selected_row_id_sample(), &[RowId(2)]);
    assert_eq!(document.selected_bins(), &[1, 8]);
    assert_eq!(document.selected_count(), 3);
}

#[test]
fn rejected_replacement_context_leaves_previous_document_unchanged() {
    let mut dataset_generations = DatasetGenerationCounter::default();
    let active_dataset = dataset_generations.mint();
    let pending_dataset = dataset_generations.mint();
    let cohort_generation = CohortGenerationCounter::default().mint();
    let snapshot = SelectionSnapshot::from_parts(
        active_dataset,
        cohort_generation,
        SelectionId(11),
        [RowId(4), RowId(6)],
        [3],
        2,
    )
    .expect("active selection snapshot should validate");
    let active_document =
        EvidenceDocument::from_selection(&snapshot, context(active_dataset, cohort_generation, 2))
            .expect("active evidence context should commit");
    let before_replacement = active_document.clone();

    let replacement =
        EvidenceDocument::from_selection(&snapshot, context(pending_dataset, cohort_generation, 2));

    assert!(replacement.is_err());
    assert_eq!(active_document, before_replacement);
}
