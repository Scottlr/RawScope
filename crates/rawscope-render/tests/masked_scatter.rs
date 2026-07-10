use rawscope_core::{F32Range, RowId};
use rawscope_data::{
    build_visual_field_catalog, evaluate_filters, DatasetFilter, FilterEvaluation, FilterSet,
    LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, ScatterPointKind,
    ScatterPointRecord, VisualFieldCatalogConfig,
};
use rawscope_render::{
    scatter_marginal_summary, scatter_marginal_summary_masked, scatter_selection_comparison_masked,
    selected_region_summary_masked, ScatterBrushSelection,
};

#[test]
fn all_ones_mask_matches_unfiltered_cpu_density() {
    let points = points();
    let evaluation = evaluation(&["a", "b", "a", "b"], FilterSet::default());
    let range = F32Range::new(0.0, 10.0);

    let unfiltered = scatter_marginal_summary(&points, range, range, 5, 5);
    let masked =
        scatter_marginal_summary_masked(&points, &evaluation.mask, range, range, 5, 5).unwrap();

    assert_eq!(masked, unfiltered);
}

#[test]
fn masked_marginals_count_only_included_rows() {
    let points = points();
    let evaluation = category_evaluation(&["keep", "drop", "keep", "drop"]);
    let summary = scatter_marginal_summary_masked(
        &points,
        &evaluation.mask,
        F32Range::new(0.0, 10.0),
        F32Range::new(0.0, 10.0),
        5,
        5,
    )
    .unwrap();

    assert_eq!(summary.x_bins.iter().map(|bin| bin.count).sum::<u32>(), 2);
    assert_eq!(summary.y_bins.iter().map(|bin| bin.count).sum::<u32>(), 2);
}

#[test]
fn masked_brush_and_comparison_exclude_filtered_rows() {
    let points = points();
    let evaluation = category_evaluation(&["keep", "drop", "keep", "drop"]);
    let selection = ScatterBrushSelection {
        x_range: F32Range::new(0.0, 5.0),
        y_range: F32Range::new(0.0, 5.0),
    };

    let summary = selected_region_summary_masked(&points, &evaluation.mask, selection).unwrap();
    let comparison =
        scatter_selection_comparison_masked(&points, &evaluation.mask, selection).unwrap();

    assert_eq!(summary.selected_row_count, 1);
    assert_eq!(summary.total_row_count, 2);
    assert_eq!(comparison.selected_row_count, 1);
    assert_eq!(comparison.baseline_row_count, 4);
}

#[test]
fn masked_helpers_reject_misaligned_rows() {
    let points = points();
    let evaluation = evaluation(&["a", "b", "c"], FilterSet::default());
    let error = scatter_marginal_summary_masked(
        &points,
        &evaluation.mask,
        F32Range::new(0.0, 10.0),
        F32Range::new(0.0, 10.0),
        5,
        5,
    )
    .unwrap_err();

    assert_eq!(error.record_count, 4);
    assert_eq!(error.mask_len, 3);
}

fn category_evaluation(values: &[&str]) -> FilterEvaluation {
    let mut filters = FilterSet::default();
    filters.replace_for_column(DatasetFilter::Categories {
        column_name: "cohort".into(),
        included_values: vec!["keep".into()],
        include_missing: false,
    });
    evaluation(values, filters)
}

fn evaluation(values: &[&str], filters: FilterSet) -> FilterEvaluation {
    let source = LoadedSourceTable {
        columns: vec![LoadedColumnSchema {
            name: "cohort".into(),
            kind: LoadedColumnKind::String,
        }],
        rows: values
            .iter()
            .enumerate()
            .map(|(index, value)| LoadedSourceRow {
                row_id: RowId(index as u64),
                values: vec![(*value).into()],
            })
            .collect(),
    };
    let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
    evaluate_filters(&source, &catalog, &filters).unwrap()
}

fn points() -> Vec<ScatterPointRecord> {
    [(1.0, 1.0), (2.0, 2.0), (8.0, 8.0), (9.0, 9.0)]
        .into_iter()
        .enumerate()
        .map(|(index, (x, y))| ScatterPointRecord {
            row_id: RowId(index as u64),
            x,
            y,
            kind: ScatterPointKind::Unclassified,
        })
        .collect()
}
