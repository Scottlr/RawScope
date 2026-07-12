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

#[test]
fn unchanged_cohort_evaluation_reuses_its_generation() {
    let (source, catalog) = source();
    let mut builder = CohortBuilder::new(CohortPolicy::default());
    let mut dataset_generations = DatasetGenerationCounter::default();
    let dataset_generation = dataset_generations.mint();
    let mut cohort_generations = CohortGenerationCounter::default();
    let first = builder
        .evaluate(
            &source,
            &catalog,
            dataset_generation,
            &mut cohort_generations,
        )
        .unwrap();
    let second = builder
        .evaluate(
            &source,
            &catalog,
            dataset_generation,
            &mut cohort_generations,
        )
        .unwrap();
    assert_eq!(first.cohort_generation(), second.cohort_generation());
    assert_eq!(first.filter_revision(), second.filter_revision());
}
