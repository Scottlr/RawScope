use rawscope_core::RowId;
use rawscope_data::{LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable};
use rawscope_render::{missingness_grid, missingness_selection_summary, MissingnessSelection};

fn source_table_fixture() -> LoadedSourceTable {
    LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "alpha".to_string(),
                kind: LoadedColumnKind::String,
            },
            LoadedColumnSchema {
                name: "beta".to_string(),
                kind: LoadedColumnKind::String,
            },
            LoadedColumnSchema {
                name: "gamma".to_string(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: vec![
            LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["".to_string(), "  ".to_string(), "ok".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(1),
                values: vec!["1".to_string(), "".to_string(), "ok".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(2),
                values: vec!["2".to_string(), "value".to_string(), " ".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(3),
                values: vec!["3".to_string(), "value".to_string(), "ok".to_string()],
            },
        ],
    }
}

#[test]
fn missingness_grid_counts_empty_and_whitespace_cells_deterministically() {
    let source_table = source_table_fixture();

    let grid = missingness_grid(&source_table, 2);

    let bucket_zero_alpha = grid.cell(0, 0).expect("bucket 0 col 0 should exist");
    let bucket_zero_beta = grid.cell(0, 1).expect("bucket 0 col 1 should exist");
    let bucket_one_gamma = grid.cell(1, 2).expect("bucket 1 col 2 should exist");

    assert_eq!(grid.row_bucket_count, 2);
    assert_eq!(grid.column_count, 3);
    assert_eq!(bucket_zero_alpha.missing_count, 1);
    assert_eq!(bucket_zero_alpha.total_count, 2);
    assert_eq!(bucket_zero_beta.missing_count, 2);
    assert_eq!(bucket_zero_beta.total_count, 2);
    assert_eq!(bucket_one_gamma.missing_count, 1);
    assert_eq!(bucket_one_gamma.total_count, 2);
}

#[test]
fn missingness_selection_summary_returns_sorted_row_ids_and_column_names() {
    let source_table = source_table_fixture();

    let summary =
        missingness_selection_summary(&source_table, 2, MissingnessSelection::new(0, 1, 1, 2));

    assert_eq!(summary.selected_missing_count, 2);
    assert_eq!(summary.selected_total_count, 2);
    assert_eq!(summary.selected_row_ids, vec![RowId(0), RowId(1)]);
    assert_eq!(summary.column_names, vec!["beta".to_string()]);
}

#[test]
fn missingness_selection_summary_accumulates_across_multiple_columns_and_buckets() {
    let source_table = source_table_fixture();

    let summary =
        missingness_selection_summary(&source_table, 2, MissingnessSelection::new(0, 2, 1, 3));

    assert_eq!(summary.selected_missing_count, 3);
    assert_eq!(summary.selected_total_count, 8);
    assert_eq!(summary.selected_row_ids, vec![RowId(0), RowId(1), RowId(2)]);
    assert_eq!(
        summary.column_names,
        vec!["beta".to_string(), "gamma".to_string()]
    );
}
