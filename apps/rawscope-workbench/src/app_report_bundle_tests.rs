use std::{fs, path::Path};

use rawscope_core::{F32Range, RowId, U64Range};
use rawscope_data::{
    DatasetIdentity, ScatterPointKind, SyntheticEventType, SyntheticPointCategory,
};
use rawscope_render::{
    ScatterEvidenceView, ScatterSelectionEvidenceV2, SelectedCategoryCounts,
    SelectedEventTypeCounts, TimelineEvidenceView, TimelineLaneRange, TimelineSelectionEvidenceV2,
};

use crate::app_report_bundle::{
    EvidenceReportBundlePaths, SCATTER_REPORT_BUNDLE_DIR_PREFIX, TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
};

#[test]
fn report_bundle_paths_include_timestamp_and_counter() {
    let test_dir = unique_test_dir("bundle-paths");
    let paths = EvidenceReportBundlePaths::next_available(
        &test_dir,
        SCATTER_REPORT_BUNDLE_DIR_PREFIX,
        1234,
        7,
    );

    assert!(paths
        .bundle_dir
        .ends_with(Path::new("report-scatter-1234-7")));
    assert!(paths
        .evidence_json_path
        .ends_with(Path::new("report-scatter-1234-7/evidence.json")));
    assert!(paths
        .evidence_markdown_path
        .ends_with(Path::new("report-scatter-1234-7/evidence.md")));
    assert!(paths
        .visual_context_path
        .ends_with(Path::new("report-scatter-1234-7/visual-context.txt")));
    assert!(paths
        .manifest_path
        .ends_with(Path::new("report-scatter-1234-7/manifest.json")));
}

#[test]
fn report_bundle_paths_skip_existing_collision() {
    let test_dir = unique_test_dir("bundle-collision");
    let collided_dir = test_dir.join("report-scatter-1234-1");
    fs::create_dir_all(&collided_dir).unwrap();

    let paths = EvidenceReportBundlePaths::next_available(
        &test_dir,
        SCATTER_REPORT_BUNDLE_DIR_PREFIX,
        1234,
        1,
    );

    assert!(paths
        .bundle_dir
        .ends_with(Path::new("report-scatter-1234-2")));
    fs::remove_dir_all(&test_dir).unwrap();
}

#[test]
fn timeline_report_bundle_paths_use_timeline_prefix() {
    let test_dir = unique_test_dir("timeline-paths");
    let paths = EvidenceReportBundlePaths::next_available(
        &test_dir,
        TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
        1234,
        7,
    );

    assert!(paths
        .bundle_dir
        .ends_with(Path::new("report-timeline-1234-7")));
    assert!(paths
        .evidence_json_path
        .ends_with(Path::new("report-timeline-1234-7/evidence.json")));
}

#[test]
fn write_scatter_bundle_creates_expected_files() {
    let test_dir = unique_test_dir("scatter-write");
    let bundle_paths = EvidenceReportBundlePaths::next_available(
        &test_dir,
        SCATTER_REPORT_BUNDLE_DIR_PREFIX,
        1234,
        1,
    );

    bundle_paths
        .write_scatter(&sample_scatter_evidence())
        .unwrap();

    assert!(bundle_paths.evidence_json_path.exists());
    assert!(bundle_paths.evidence_markdown_path.exists());
    assert!(bundle_paths.visual_context_path.exists());
    assert!(bundle_paths.manifest_path.exists());

    let manifest = fs::read_to_string(&bundle_paths.manifest_path).unwrap();
    assert!(manifest.contains("\"artifact_kind\": \"scatter-evidence-report-bundle\""));
    assert!(manifest.contains("\"visual_context_kind\": \"text-placeholder\""));

    let visual_context = fs::read_to_string(&bundle_paths.visual_context_path).unwrap();
    assert!(visual_context.contains("view_kind: scatter"));
    assert!(visual_context.contains("capture_status: deferred"));

    fs::remove_dir_all(&test_dir).unwrap();
}

#[test]
fn write_timeline_bundle_creates_expected_files() {
    let test_dir = unique_test_dir("timeline-write");
    let bundle_paths = EvidenceReportBundlePaths::next_available(
        &test_dir,
        TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
        1234,
        1,
    );

    bundle_paths
        .write_timeline(&sample_timeline_evidence())
        .unwrap();

    assert!(bundle_paths.evidence_json_path.exists());
    assert!(bundle_paths.evidence_markdown_path.exists());
    assert!(bundle_paths.visual_context_path.exists());
    assert!(bundle_paths.manifest_path.exists());

    let manifest = fs::read_to_string(&bundle_paths.manifest_path).unwrap();
    assert!(manifest.contains("\"artifact_kind\": \"timeline-evidence-report-bundle\""));
    assert!(manifest.contains("\"visual_context_kind\": \"text-placeholder\""));

    let visual_context = fs::read_to_string(&bundle_paths.visual_context_path).unwrap();
    assert!(visual_context.contains("view_kind: timeline"));
    assert!(visual_context.contains("capture_status: deferred"));

    fs::remove_dir_all(&test_dir).unwrap();
}

fn unique_test_dir(name: &str) -> std::path::PathBuf {
    let mut test_dir = std::env::temp_dir();
    test_dir.push(format!("rawscope-{name}-{}", std::process::id()));
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir).unwrap();
    }
    test_dir
}

fn sample_scatter_evidence() -> ScatterSelectionEvidenceV2 {
    ScatterSelectionEvidenceV2 {
        dataset_identity: DatasetIdentity::synthetic_scatter(42, 20_000),
        view: ScatterEvidenceView {
            x_range: F32Range::new(0.0, 100.0),
            y_range: F32Range::new(-25.0, 75.0),
            grid_width: 256,
            grid_height: 256,
        },
        selected_row_count: 2,
        selected_percentage: 0.01,
        selected_row_id_sample: vec![RowId(3), RowId(7)],
        selected_record_sample: vec![],
        selected_source_column_names: vec![],
        selected_source_row_sample: vec![],
        point_kind_counts: SelectedCategoryCounts {
            cluster: 2,
            ..SelectedCategoryCounts::default()
        },
        top_point_kind: Some(ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster)),
        selected_x_range: Some(F32Range::new(10.0, 11.0)),
        selected_y_range: Some(F32Range::new(20.0, 21.0)),
        brush_x_range: F32Range::new(9.5, 11.5),
        brush_y_range: F32Range::new(19.5, 21.5),
    }
}

fn sample_timeline_evidence() -> TimelineSelectionEvidenceV2 {
    TimelineSelectionEvidenceV2 {
        dataset_identity: DatasetIdentity::synthetic_timeline(42, 20_000, 4),
        view: TimelineEvidenceView {
            time_range: U64Range::new(1_000, 2_000),
            full_time_range: U64Range::new(0, 10_000),
            lane_count: 4,
            grid_width: 256,
            grid_height: 128,
        },
        selected_event_count: 3,
        selected_percentage: 0.015,
        selected_time_range: U64Range::new(1_100, 1_400),
        selected_lane_range: TimelineLaneRange {
            start: 1,
            end_exclusive: 3,
        },
        selected_row_id_sample: vec![RowId(5), RowId(9), RowId(11)],
        selected_event_sample: vec![],
        selected_source_column_names: vec![],
        selected_source_row_sample: vec![],
        lane_counts: vec![0, 2, 1, 0],
        event_kind_counts: SelectedEventTypeCounts {
            background: 1,
            spike: 2,
            ..SelectedEventTypeCounts::default()
        },
        top_lane: Some(1),
        top_event_kind: Some(rawscope_data::TimelineEventKind::Synthetic(
            SyntheticEventType::Spike,
        )),
        selected_timestamp_range: Some(U64Range::new(1_100, 1_350)),
        selected_value_range: Some((2.0, 8.0)),
    }
}
