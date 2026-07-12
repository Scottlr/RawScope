use std::{fs, path::Path};

use rawscope_core::{F32Range, RowId, U64Range};
use rawscope_data::{
    DatasetIdentity, DatasetProfileId, ScatterPointKind, ScatterProjection, SyntheticEventType,
    SyntheticPointCategory,
};
use rawscope_evidence::{
    ScatterEvidenceView, ScatterSelectionEvidenceV2, ScatterSelectionKindCounts,
};
use rawscope_render::{
    scatter_selection_evidence_v4_json, AggregateEvidenceBin, ComparisonRatio, DensityEncoding,
    PointRevealEvidence, PointRevealMode, ScatterAggregateEvidenceContext, ScatterCohortEvidence,
    ScatterDensityMode, ScatterDensityPresentation, ScatterKindComparison,
    ScatterSelectionComparison, ScatterSelectionEvidenceV3, ScatterSelectionEvidenceV4,
    ScatterSelectionEvidenceV5, ScatterVisualQueryV4, SelectedEventTypeCounts,
    TimelineEvidenceView, TimelineLaneRange, TimelineSelectionEvidenceV2,
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

#[test]
fn write_scatter_bundle_v3_uses_enriched_visual_context() {
    let test_dir = unique_test_dir("scatter-write-v3");
    let bundle_paths = EvidenceReportBundlePaths::next_available(
        &test_dir,
        SCATTER_REPORT_BUNDLE_DIR_PREFIX,
        1234,
        1,
    );

    bundle_paths
        .write_scatter_v3(&sample_scatter_evidence_v3())
        .unwrap();

    let manifest = fs::read_to_string(&bundle_paths.manifest_path).unwrap();
    assert!(manifest.contains("\"evidence_schema_version\": 3"));
    assert!(manifest.contains("\"visual_context_kind\": \"text-visual-context\""));

    let evidence_json = fs::read_to_string(&bundle_paths.evidence_json_path).unwrap();
    assert!(evidence_json.contains("\"schema_version\": 3"));
    assert!(evidence_json.contains("\"density_encoding\""));

    let visual_context = fs::read_to_string(&bundle_paths.visual_context_path).unwrap();
    assert!(visual_context.contains("density_transform: log1p(count)"));
    assert!(visual_context.contains("density_palette: scatter sequential"));
    assert!(visual_context.contains("density_normalization: viewport max"));
    assert!(visual_context.contains("density_presentation: exact cells"));
    assert!(visual_context.contains("dataset_profile: lichess-games"));
    assert!(visual_context.contains("comparison_baseline: active_point_slice"));
    assert!(visual_context.contains("aggregate_context_bin_limit: 16"));
    assert!(visual_context.contains("capture_status: deferred"));

    fs::remove_dir_all(&test_dir).unwrap();
}

#[test]
fn write_scatter_bundle_v4_creates_manifest_and_evidence() {
    let test_dir = unique_test_dir("scatter-write-v4");
    let bundle_paths = EvidenceReportBundlePaths::next_available(
        &test_dir,
        SCATTER_REPORT_BUNDLE_DIR_PREFIX,
        1234,
        1,
    );
    bundle_paths
        .write_scatter_v4(&sample_scatter_evidence_v4())
        .unwrap();

    let manifest = fs::read_to_string(&bundle_paths.manifest_path).unwrap();
    assert!(manifest.contains("\"evidence_schema_version\": 4"));
    assert!(manifest.contains("rawscope.scatter-selection-evidence.v4"));
    let evidence = fs::read_to_string(&bundle_paths.evidence_json_path).unwrap();
    assert!(evidence.contains("\"visual_query\""));
    assert!(evidence.contains("\"cohort\""));
    let context = fs::read_to_string(&bundle_paths.visual_context_path).unwrap();
    assert!(context.contains("schema_version: 4"));
    assert!(context.contains("projection: Raw"));
    fs::remove_dir_all(&test_dir).unwrap();
}

#[test]
fn write_scatter_bundle_v5_creates_manifest_and_evidence() {
    let test_dir = unique_test_dir("scatter-write-v5");
    let bundle_paths = EvidenceReportBundlePaths::next_available(
        &test_dir,
        SCATTER_REPORT_BUNDLE_DIR_PREFIX,
        1234,
        1,
    );
    let evidence =
        ScatterSelectionEvidenceV5::from_v4(&sample_scatter_evidence_v4(), None, None).unwrap();
    bundle_paths.write_scatter_v5(&evidence).unwrap();

    let manifest = fs::read_to_string(&bundle_paths.manifest_path).unwrap();
    assert!(manifest.contains("\"evidence_schema_version\": 5"));
    assert!(manifest.contains("rawscope.scatter-selection-evidence.v5"));
    let evidence_json = fs::read_to_string(&bundle_paths.evidence_json_path).unwrap();
    assert!(evidence_json.contains("\"schema_version\": 5"));
    assert!(!evidence_json.contains("\"session_context\""));
    let context = fs::read_to_string(&bundle_paths.visual_context_path).unwrap();
    assert!(context.contains("schema_version: 5"));
    assert!(context.contains("session_context: false"));
    fs::remove_dir_all(&test_dir).unwrap();
}

#[test]
fn v4_bundle_captures_curated_active_state() {
    let json = scatter_selection_evidence_v4_json(&sample_scatter_evidence_v4()).unwrap();

    assert!(json.contains("\"variant\": \"raw_xy\""));
    assert!(json.contains("\"density_mode\": \"absolute_density\""));
    assert!(json.contains("\"density_presentation\": \"topographic_field\""));
    assert!(json.contains("\"mode\": \"auto\""));
    assert!(json.contains("\"included_row_count\": 20000"));
}

#[test]
fn write_timeline_bundle_v3_includes_dataset_profile_metadata() {
    let test_dir = unique_test_dir("timeline-write-v3");
    let bundle_paths = EvidenceReportBundlePaths::next_available(
        &test_dir,
        TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
        1234,
        1,
    );

    bundle_paths
        .write_timeline_v3(&sample_timeline_evidence_v3())
        .unwrap();

    let manifest = fs::read_to_string(&bundle_paths.manifest_path).unwrap();
    assert!(manifest.contains("\"evidence_schema_version\": 3"));
    assert!(manifest.contains("\"active_dataset_profile\": \"lichess-games\""));

    let evidence_json = fs::read_to_string(&bundle_paths.evidence_json_path).unwrap();
    assert!(evidence_json.contains("\"active_dataset_profile\": \"lichess-games\""));

    let visual_context = fs::read_to_string(&bundle_paths.visual_context_path).unwrap();
    assert!(visual_context.contains("dataset_profile: lichess-games"));
    assert!(visual_context.contains("comparison_baseline: active_event_slice"));

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
        point_kind_counts: ScatterSelectionKindCounts {
            cluster: 2,
            ..ScatterSelectionKindCounts::default()
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

fn sample_scatter_evidence_v3() -> ScatterSelectionEvidenceV3 {
    let evidence_v2 = sample_scatter_evidence();

    ScatterSelectionEvidenceV3::from_v2(
        &evidence_v2,
        DensityEncoding::scatter_default(),
        ScatterSelectionComparison {
            selected_row_count: 2,
            baseline_row_count: evidence_v2.dataset_identity.row_count,
            selected_percentage: evidence_v2.selected_percentage,
            point_kind_ratios: ScatterKindComparison {
                cluster: ComparisonRatio {
                    selected_count: 2,
                    baseline_count: 4,
                    selected_percentage: 100.0,
                    baseline_percentage: 20.0,
                    delta_percentage_points: 80.0,
                },
                background: ComparisonRatio {
                    selected_count: 0,
                    baseline_count: 10,
                    selected_percentage: 0.0,
                    baseline_percentage: 50.0,
                    delta_percentage_points: -50.0,
                },
                outlier: ComparisonRatio {
                    selected_count: 0,
                    baseline_count: 2,
                    selected_percentage: 0.0,
                    baseline_percentage: 10.0,
                    delta_percentage_points: -10.0,
                },
                unclassified: ComparisonRatio {
                    selected_count: 0,
                    baseline_count: 4,
                    selected_percentage: 0.0,
                    baseline_percentage: 20.0,
                    delta_percentage_points: -20.0,
                },
            },
        },
        ScatterAggregateEvidenceContext {
            bin_limit: 16,
            bins: vec![
                AggregateEvidenceBin {
                    bin_x: 4,
                    bin_y: 5,
                    count: 11,
                    row_id_sample: vec![3, 7],
                },
                AggregateEvidenceBin {
                    bin_x: 6,
                    bin_y: 2,
                    count: 9,
                    row_id_sample: vec![9, 13],
                },
            ],
        },
        Some(DatasetProfileId::LichessGames),
    )
}

fn sample_scatter_evidence_v4() -> ScatterSelectionEvidenceV4 {
    ScatterSelectionEvidenceV4::from_v3(
        &sample_scatter_evidence_v3(),
        ScatterVisualQueryV4 {
            x_range: F32Range::new(0.0, 100.0),
            y_range: F32Range::new(-25.0, 75.0),
            grid_width: 256,
            grid_height: 256,
            projection: ScatterProjection::RawXY,
            filters: vec![],
            density_mode: ScatterDensityMode::AbsoluteDensity,
            density_encoding: DensityEncoding::scatter_default(),
            density_presentation: ScatterDensityPresentation::TopographicField,
            difference: None,
            point_reveal: PointRevealEvidence {
                mode: PointRevealMode::Auto,
                eligible_count: 20_000,
                rendered_count: 0,
                sampled: false,
            },
            relief: None,
        },
        ScatterCohortEvidence {
            full_row_count: 20_000,
            included_row_count: 20_000,
            excluded_row_count: 0,
        },
        None,
    )
    .unwrap()
}

fn sample_timeline_evidence_v3() -> rawscope_render::TimelineSelectionEvidenceV3 {
    let evidence_v2 = sample_timeline_evidence();

    rawscope_render::TimelineSelectionEvidenceV3::from_v2(
        &evidence_v2,
        DensityEncoding::timeline_default(),
        rawscope_render::TimelineSelectionComparison {
            selected_event_count: evidence_v2.selected_event_count,
            baseline_event_count: evidence_v2.dataset_identity.row_count,
            selected_percentage: evidence_v2.selected_percentage,
            event_kind_ratios: rawscope_render::TimelineKindComparison {
                background: zero_ratio(),
                spike: zero_ratio(),
                stale_lane: zero_ratio(),
                high_value_band: zero_ratio(),
                unclassified: zero_ratio(),
            },
            lane_ratios: vec![],
        },
        rawscope_render::TimelineAggregateEvidenceContext {
            bin_limit: 16,
            bins: vec![],
        },
        Some(DatasetProfileId::LichessGames),
    )
}

fn zero_ratio() -> ComparisonRatio {
    ComparisonRatio {
        selected_count: 0,
        baseline_count: 0,
        selected_percentage: 0.0,
        baseline_percentage: 0.0,
        delta_percentage_points: 0.0,
    }
}
