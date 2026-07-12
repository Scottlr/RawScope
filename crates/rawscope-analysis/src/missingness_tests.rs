use super::*;
use crate::cohort::{CohortBuilder, CohortGenerationCounter, CohortPolicy};
use rawscope_core::RowId;
use rawscope_data::{
    DatasetFilter, DatasetGenerationCounter, LoadedColumnSchema, LoadedSourceRow,
    VisualFieldCatalogConfig,
};

#[test]
fn missingness_index_is_generation_bound_and_keeps_invalid_separate() {
    let source = LoadedSourceTable {
        columns: vec![LoadedColumnSchema {
            name: "score".to_string(),
            kind: LoadedColumnKind::Float,
        }],
        rows: vec![
            LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(1),
                values: vec!["bad".to_string()],
            },
        ],
    };
    let catalog =
        rawscope_data::build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
    let mut cohort_generations = CohortGenerationCounter::default();
    let cohort = CohortBuilder::new(CohortPolicy::default())
        .evaluate(
            &source,
            &catalog,
            DatasetGenerationCounter::default().mint(),
            &mut cohort_generations,
        )
        .unwrap();
    let index = MissingnessIndex::build(&source, &cohort).unwrap();
    assert_eq!(index.columns()[0].missing_count, 1);
    assert_eq!(index.columns()[0].invalid_count, 1);

    let mut filtered = CohortBuilder::new(CohortPolicy::default());
    filtered.replace_filter(DatasetFilter::NumericRange {
        column_name: "score".to_string(),
        min_inclusive: 0.0,
        max_inclusive: 1.0,
        include_missing: false,
    });
    let filtered = filtered
        .evaluate(
            &source,
            &catalog,
            cohort.dataset_generation(),
            &mut cohort_generations,
        )
        .unwrap();
    let index = MissingnessIndex::build(&source, &filtered).unwrap();
    assert_eq!(index.columns()[0].included_count, 0);
    assert_eq!(index.cohort_generation(), filtered.cohort_generation());
}
