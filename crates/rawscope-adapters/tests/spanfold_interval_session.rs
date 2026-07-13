#![cfg(feature = "spanfold")]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use rawscope_adapters::{
    spanfold::{
        SpanfoldAdapterError, SpanfoldIntervalFamily, SpanfoldIntervalTransform,
        RAWSCOPE_SCATTER_DURATION_COLUMN, RAWSCOPE_SCATTER_START_COLUMN,
        SPANFOLD_EVIDENCE_KEY_COLUMN,
    },
    SessionAdapter,
};
use rawscope_data::load_scatter_dataset;
use rawscope_session::{load_session_manifest, ResolvedSessionView};
use spanfold::{
    ComparisonFinality, ComparisonResult, TemporalAxis, TemporalPoint, WindowHistoryFixture,
};

static NEXT_FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
fn core_transform_preserves_ranges_and_source_record_evidence() {
    let result = comparison_result();

    let dataset = SpanfoldIntervalTransform::default()
        .transform(&result)
        .expect("valid SpanFold comparison should transform");

    assert_eq!(dataset.plan_name(), "Provider QA");
    assert_eq!(dataset.temporal_axis(), TemporalAxis::ProcessingPosition);
    assert_eq!(dataset.clock(), None);
    assert_eq!(dataset.origin(), 1);
    assert_eq!(dataset.rows().len(), 5);

    let rows = dataset.rows();
    assert_interval(&rows[0], SpanfoldIntervalFamily::Overlap, 3, 5, 2.0, 2);
    assert_interval(&rows[1], SpanfoldIntervalFamily::Residual, 1, 3, 0.0, 2);
    assert_interval(&rows[2], SpanfoldIntervalFamily::Missing, 5, 6, 4.0, 1);
    assert_interval(&rows[3], SpanfoldIntervalFamily::Coverage, 1, 3, 0.0, 2);
    assert_interval(&rows[4], SpanfoldIntervalFamily::Coverage, 3, 5, 2.0, 2);

    assert_eq!(rows[0].target_record_ids, r#"["window-0000"]"#);
    assert_eq!(rows[0].against_record_ids, r#"["window-0001"]"#);
    assert_eq!(rows[3].target_magnitude, Some(2));
    assert_eq!(rows[3].covered_magnitude, Some(0));
    assert_eq!(rows[3].segment_coverage_ratio, Some(0.0));
    assert_eq!(rows[4].target_magnitude, Some(2));
    assert_eq!(rows[4].covered_magnitude, Some(2));
    assert_eq!(rows[4].segment_coverage_ratio, Some(1.0));

    let mut authoritative_metadata = Vec::new();
    authoritative_metadata.extend(
        result
            .overlap_rows_with_finality()
            .unwrap()
            .map(|entry| entry.metadata),
    );
    authoritative_metadata.extend(
        result
            .residual_rows_with_finality()
            .unwrap()
            .map(|entry| entry.metadata),
    );
    authoritative_metadata.extend(
        result
            .missing_rows_with_finality()
            .unwrap()
            .map(|entry| entry.metadata),
    );
    authoritative_metadata.extend(
        result
            .coverage_rows_with_finality()
            .unwrap()
            .map(|entry| entry.metadata),
    );

    for (row, metadata) in rows.iter().zip(authoritative_metadata) {
        assert_eq!(row.spanfold_row_id, metadata.row_id);
        assert_eq!(row.spanfold_finality, metadata.finality);
        assert_eq!(row.spanfold_finality_reason, metadata.reason);
        assert_eq!(row.spanfold_row_version, metadata.version);
        assert_eq!(row.spanfold_supersedes_row_id, metadata.supersedes_row_id);
    }
}

#[test]
fn all_interval_transform_maps_gap_side_and_containment_annotations() {
    let result = disconnected_comparison_result();

    let dataset = SpanfoldIntervalTransform::all_intervals()
        .transform(&result)
        .expect("every range-based comparator should transform");

    let family_labels = dataset
        .rows()
        .iter()
        .map(|row| row.row_family.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        family_labels,
        [
            "residual",
            "missing",
            "coverage",
            "gap",
            "symmetric_difference",
            "symmetric_difference",
            "containment",
        ]
    );

    let rows = dataset.rows();
    assert_eq!((rows[3].start, rows[3].end), (3, 5));
    assert_eq!(rows[4].side.as_deref(), Some("target"));
    assert_eq!(rows[5].side.as_deref(), Some("against"));
    assert_eq!(rows[6].status.as_deref(), Some("left_overhang"));
}

#[test]
fn transform_rejects_inconsistent_spanfold_row_metadata() {
    let mut result = comparison_result();
    result.row_finalities.pop();

    let error = SpanfoldIntervalTransform::default()
        .transform(&result)
        .expect_err("missing SpanFold metadata must not produce ordinal fallback ids");

    assert!(matches!(
        error,
        SpanfoldAdapterError::InconsistentRowMetadata(_)
    ));
}

#[test]
fn spanfold_adapter_prepares_a_session_consumed_by_the_rawscope_loader() {
    let fixture = Fixture::new();
    let bundle_dir = fixture.root.join("prepared-session");
    let transform = SpanfoldIntervalTransform::default();
    let result = comparison_result();

    let prepared = <SpanfoldIntervalTransform as SessionAdapter>::prepare_session(
        &transform,
        &result,
        &bundle_dir,
    )
    .expect("adapter should prepare a complete session");

    assert_eq!(prepared.row_count(), Some(5));
    let session = load_session_manifest(prepared.manifest_path())
        .expect("prepared manifest should satisfy session v1");
    assert_eq!(
        session.dataset.path,
        fs::canonicalize(prepared.dataset_path()).unwrap()
    );
    assert_eq!(
        session.dataset.evidence_key.as_deref(),
        Some(SPANFOLD_EVIDENCE_KEY_COLUMN)
    );
    assert!(matches!(
        session.view,
        ResolvedSessionView::Scatter { x, y, profile: None }
            if x == RAWSCOPE_SCATTER_START_COLUMN && y == RAWSCOPE_SCATTER_DURATION_COLUMN
    ));

    let loaded = load_scatter_dataset(
        prepared.dataset_path(),
        RAWSCOPE_SCATTER_START_COLUMN,
        RAWSCOPE_SCATTER_DURATION_COLUMN,
        None,
    )
    .expect("RawScope's real CSV loader should consume the prepared dataset");
    assert_eq!(loaded.points.len(), 5);
    assert_eq!(loaded.source_rows.rows.len(), 5);
    assert_eq!((loaded.points[0].x, loaded.points[0].y), (2.0, 2.0));
    assert_eq!((loaded.points[4].x, loaded.points[4].y), (2.0, 2.0));
    assert!(loaded
        .source_rows
        .column_names()
        .any(|name| name == "start"));
    assert!(loaded
        .source_rows
        .column_names()
        .any(|name| name == "target_record_ids"));
}

#[test]
fn provisional_identity_and_finality_survive_csv_and_rawscope_loading() {
    let fixture = Fixture::new();
    let bundle_dir = fixture.root.join("provisional-session");
    let result = provisional_comparison_result();
    let expected_metadata = result
        .residual_rows_with_finality()
        .unwrap()
        .map(|entry| entry.metadata.clone())
        .collect::<Vec<_>>();

    let prepared = SpanfoldIntervalTransform::default()
        .prepare_session(&result, &bundle_dir)
        .expect("provisional SpanFold rows should prepare normally");
    let loaded = load_scatter_dataset(
        prepared.dataset_path(),
        RAWSCOPE_SCATTER_START_COLUMN,
        RAWSCOPE_SCATTER_DURATION_COLUMN,
        None,
    )
    .expect("RawScope should load the materialized provisional rows");

    let column_names = loaded.source_rows.column_names().collect::<Vec<_>>();
    let row_id_column = column_index(&column_names, "spanfold_row_id");
    let finality_column = column_index(&column_names, "spanfold_finality");
    let reason_column = column_index(&column_names, "spanfold_finality_reason");
    let version_column = column_index(&column_names, "spanfold_row_version");
    let supersedes_column = column_index(&column_names, "spanfold_supersedes_row_id");

    assert_eq!(loaded.source_rows.rows.len(), expected_metadata.len());
    for (row, metadata) in loaded.source_rows.rows.iter().zip(expected_metadata) {
        assert_eq!(row.values[row_id_column], metadata.row_id);
        assert_eq!(row.values[finality_column], "Provisional");
        assert_eq!(metadata.finality, ComparisonFinality::Provisional);
        assert_eq!(row.values[reason_column], metadata.reason);
        assert_eq!(row.values[version_column], metadata.version.to_string());
        assert_eq!(
            row.values[supersedes_column],
            metadata.supersedes_row_id.unwrap_or_default()
        );
    }
}

fn comparison_result() -> ComparisonResult {
    let history = WindowHistoryFixture::new()
        .closed_window("DeviceOffline", "device-1", 1, 5, |window| {
            window.source("provider-a").partition("fleet-a")
        })
        .unwrap()
        .closed_window("DeviceOffline", "device-1", 3, 6, |window| {
            window.source("provider-b").partition("fleet-a")
        })
        .unwrap()
        .build();

    history
        .compare("Provider QA")
        .target_source("provider-a")
        .against_source("provider-b")
        .scope_window("DeviceOffline")
        .overlap()
        .residual()
        .missing()
        .coverage()
        .run()
}

fn disconnected_comparison_result() -> ComparisonResult {
    let history = WindowHistoryFixture::new()
        .closed_window("DeviceOffline", "device-1", 1, 3, |window| {
            window.source("provider-a").partition("fleet-a")
        })
        .unwrap()
        .closed_window("DeviceOffline", "device-1", 5, 7, |window| {
            window.source("provider-b").partition("fleet-a")
        })
        .unwrap()
        .build();

    history
        .compare("Disconnected Provider QA")
        .target_source("provider-a")
        .against_source("provider-b")
        .scope_window("DeviceOffline")
        .overlap()
        .residual()
        .missing()
        .coverage()
        .gap()
        .symmetric_difference()
        .containment()
        .run()
}

fn provisional_comparison_result() -> ComparisonResult {
    let history = WindowHistoryFixture::new()
        .open_window("DeviceOffline", "device-1", 1, |window| {
            window.source("provider-a").partition("fleet-a")
        })
        .unwrap()
        .closed_window("DeviceOffline", "device-1", 3, 5, |window| {
            window.source("provider-b").partition("fleet-a")
        })
        .unwrap()
        .build();

    history
        .compare("Live Provider QA")
        .target_source("provider-a")
        .against_source("provider-b")
        .scope_window("DeviceOffline")
        .residual()
        .clip_open_windows_to_position(10)
        .run_live(TemporalPoint::position(10))
}

fn column_index(column_names: &[&str], expected: &str) -> usize {
    column_names
        .iter()
        .position(|name| *name == expected)
        .unwrap_or_else(|| panic!("expected CSV column '{expected}'"))
}

fn assert_interval(
    row: &rawscope_adapters::spanfold::SpanfoldIntervalRow,
    family: SpanfoldIntervalFamily,
    start: i64,
    end: i64,
    start_offset: f64,
    duration: i64,
) {
    assert_eq!(row.row_family, family);
    assert_eq!((row.start, row.end), (start, end));
    assert_eq!(row.start_offset, start_offset);
    assert_eq!(row.duration, duration);
    assert_eq!(row.partition.as_deref(), Some("fleet-a"));
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "rawscope-adapters-spanfold-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
