use rawscope_core::RowId;
use rawscope_data::{LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable};
use rawscope_render::{dataset_diff_summary, DatasetDiffColumnStatus, DatasetDiffMissingnessDelta};

fn source_row(row_id: u64, values: &[&str]) -> LoadedSourceRow {
    LoadedSourceRow {
        row_id: RowId(row_id),
        values: values.iter().map(|value| (*value).to_string()).collect(),
    }
}

fn source_table(
    columns: &[(&str, LoadedColumnKind)],
    rows: &[LoadedSourceRow],
) -> LoadedSourceTable {
    LoadedSourceTable {
        columns: columns
            .iter()
            .map(|(name, kind)| LoadedColumnSchema {
                name: (*name).to_string(),
                kind: *kind,
            })
            .collect(),
        rows: rows.to_vec(),
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

fn missingness_delta<'a>(
    deltas: &'a [DatasetDiffMissingnessDelta],
    column_name: &str,
) -> &'a DatasetDiffMissingnessDelta {
    deltas
        .iter()
        .find(|delta| delta.column_name == column_name)
        .expect("missingness delta should exist")
}

#[test]
fn diff_reports_row_count_delta() {
    let before = source_table(
        &[("alpha", LoadedColumnKind::String)],
        &[source_row(0, &["a"]), source_row(1, &["b"])],
    );
    let after = source_table(
        &[("alpha", LoadedColumnKind::String)],
        &[
            source_row(0, &["a"]),
            source_row(1, &["b"]),
            source_row(2, &["c"]),
        ],
    );

    let summary = dataset_diff_summary(&before, &after);

    assert_eq!(summary.before_row_count, 2);
    assert_eq!(summary.after_row_count, 3);
    assert_eq!(summary.row_count_delta, 1);
}

#[test]
fn diff_reports_added_and_removed_columns() {
    let before = source_table(
        &[
            ("alpha", LoadedColumnKind::String),
            ("legacy", LoadedColumnKind::Integer),
        ],
        &[source_row(0, &["a", "1"])],
    );
    let after = source_table(
        &[
            ("alpha", LoadedColumnKind::String),
            ("beta", LoadedColumnKind::Float),
        ],
        &[source_row(0, &["a", "1.5"])],
    );

    let summary = dataset_diff_summary(&before, &after);

    assert!(summary.columns.iter().any(|column| {
        column.name == "alpha" && column.status == DatasetDiffColumnStatus::Unchanged
    }));
    assert!(summary.columns.iter().any(|column| {
        column.name == "beta"
            && column.status == DatasetDiffColumnStatus::Added
            && column.before_type.is_none()
            && column.after_type == Some(LoadedColumnKind::Float)
    }));
    assert!(summary.columns.iter().any(|column| {
        column.name == "legacy"
            && column.status == DatasetDiffColumnStatus::Removed
            && column.before_type == Some(LoadedColumnKind::Integer)
            && column.after_type.is_none()
    }));
}

#[test]
fn diff_reports_type_changed_column() {
    let before = source_table(
        &[("alpha", LoadedColumnKind::Integer)],
        &[source_row(0, &["1"])],
    );
    let after = source_table(
        &[("alpha", LoadedColumnKind::Float)],
        &[source_row(0, &["1.0"])],
    );

    let summary = dataset_diff_summary(&before, &after);
    let changed_column = summary
        .columns
        .iter()
        .find(|column| column.name == "alpha")
        .expect("column should exist");

    assert_eq!(changed_column.status, DatasetDiffColumnStatus::TypeChanged);
    assert_eq!(changed_column.before_type, Some(LoadedColumnKind::Integer));
    assert_eq!(changed_column.after_type, Some(LoadedColumnKind::Float));
}

#[test]
fn diff_reports_missingness_ratio_delta() {
    let before = source_table(
        &[
            ("alpha", LoadedColumnKind::String),
            ("beta", LoadedColumnKind::String),
        ],
        &[source_row(0, &["", "ok"]), source_row(1, &["filled", ""])],
    );
    let after = source_table(
        &[
            ("alpha", LoadedColumnKind::String),
            ("beta", LoadedColumnKind::String),
        ],
        &[
            source_row(0, &["", "ok"]),
            source_row(1, &["", "still ok"]),
            source_row(2, &["filled", ""]),
        ],
    );

    let summary = dataset_diff_summary(&before, &after);
    let alpha_delta = missingness_delta(&summary.missingness, "alpha");
    let beta_delta = missingness_delta(&summary.missingness, "beta");

    assert_eq!(alpha_delta.before_missing_count, 1);
    assert_eq!(alpha_delta.after_missing_count, 2);
    assert_eq!(alpha_delta.delta_missing_count, 1);
    assert_close(alpha_delta.before_missing_ratio, 0.5);
    assert_close(alpha_delta.after_missing_ratio, 2.0 / 3.0);
    assert_close(alpha_delta.delta_percentage_points, 16.666668);

    assert_eq!(beta_delta.before_missing_count, 1);
    assert_eq!(beta_delta.after_missing_count, 1);
    assert_eq!(beta_delta.delta_missing_count, 0);
    assert_close(beta_delta.before_missing_ratio, 0.5);
    assert_close(beta_delta.after_missing_ratio, 1.0 / 3.0);
    assert_close(beta_delta.delta_percentage_points, -16.666668);
}

#[test]
fn zero_row_diff_does_not_panic() {
    let before = source_table(&[("alpha", LoadedColumnKind::String)], &[]);
    let after = source_table(&[("alpha", LoadedColumnKind::String)], &[]);

    let summary = dataset_diff_summary(&before, &after);
    let alpha_delta = missingness_delta(&summary.missingness, "alpha");

    assert_eq!(summary.before_row_count, 0);
    assert_eq!(summary.after_row_count, 0);
    assert_eq!(summary.row_count_delta, 0);
    assert_close(alpha_delta.before_missing_ratio, 0.0);
    assert_close(alpha_delta.after_missing_ratio, 0.0);
    assert_close(alpha_delta.delta_percentage_points, 0.0);
}
