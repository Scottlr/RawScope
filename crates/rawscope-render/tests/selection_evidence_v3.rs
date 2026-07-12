use std::path::PathBuf;

use rawscope_core::{F32Range, RowId, U64Range};
use rawscope_data::{
    DatasetIdentity, DatasetProfileId, LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow,
    LoadedSourceTable, ScatterPointKind, ScatterPointRecord, SyntheticDatasetMetadata,
    SyntheticEventType, SyntheticPointCategory, TimelineEventKind, TimelineEventRecord,
};
use rawscope_evidence::{
    scatter_selection_evidence_v2_json, scatter_selection_evidence_v2_markdown,
    ScatterEvidenceView, ScatterSelectionEvidence, ScatterSelectionEvidenceV2,
    ScatterSelectionGeometry, SelectionEvidenceConfig,
};
use rawscope_render::{
    scatter_aggregate_evidence_context, scatter_selection_comparison,
    scatter_selection_evidence_v3_json, scatter_selection_evidence_v3_markdown,
    timeline_aggregate_evidence_context, timeline_selection_comparison,
    timeline_selection_evidence_v2_json, timeline_selection_evidence_v2_markdown,
    timeline_selection_evidence_v3_markdown, AggregateBinSample, DensityEncoding,
    ScatterAggregateOverview, ScatterBrushSelection, ScatterSelectionEvidenceV3,
    SelectedRegionSummary, TimelineAggregateOverview, TimelineBrushSelection,
    TimelineEvidenceConfig, TimelineEvidenceView, TimelineLaneRange, TimelineSelectionEvidence,
    TimelineSelectionEvidenceV2, TimelineSelectionEvidenceV3, TimelineSelectionSummary,
};

fn scatter_points() -> Vec<ScatterPointRecord> {
    vec![
        ScatterPointRecord {
            row_id: RowId(0),
            x: 10.0,
            y: 20.0,
            kind: ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster),
        },
        ScatterPointRecord {
            row_id: RowId(1),
            x: 30.0,
            y: 40.0,
            kind: ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster),
        },
        ScatterPointRecord {
            row_id: RowId(2),
            x: 90.0,
            y: 95.0,
            kind: ScatterPointKind::Synthetic(SyntheticPointCategory::Background),
        },
    ]
}

fn scatter_selection() -> ScatterBrushSelection {
    ScatterBrushSelection {
        x_range: F32Range::new(0.0, 50.0),
        y_range: F32Range::new(0.0, 60.0),
    }
}

fn scatter_geometry() -> ScatterSelectionGeometry {
    ScatterSelectionGeometry {
        x_range: F32Range::new(0.0, 50.0),
        y_range: F32Range::new(0.0, 60.0),
    }
}

fn scatter_source_rows() -> LoadedSourceTable {
    LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "latency_ms".to_string(),
                kind: LoadedColumnKind::Float,
            },
            LoadedColumnSchema {
                name: "payload_size".to_string(),
                kind: LoadedColumnKind::Integer,
            },
        ],
        rows: vec![
            LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["10".to_string(), "512".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(1),
                values: vec!["30".to_string(), "256".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(2),
                values: vec!["90".to_string(), "128".to_string()],
            },
        ],
    }
}

fn scatter_evidence_v2() -> ScatterSelectionEvidenceV2 {
    let points = scatter_points();
    let v1 = ScatterSelectionEvidence::from_points(
        &points,
        scatter_geometry(),
        SyntheticDatasetMetadata::new(0, points.len()),
        points.len(),
        SelectionEvidenceConfig { max_sample_size: 2 },
    );
    let dataset_identity = DatasetIdentity::local_csv_scatter(
        PathBuf::from("C:/data/latency.csv"),
        points.len(),
        Some(points.len()),
        "latency_ms",
        "payload_size",
    );

    ScatterSelectionEvidenceV2::from_v1(
        &v1,
        dataset_identity,
        ScatterEvidenceView {
            x_range: F32Range::new(0.0, 50.0),
            y_range: F32Range::new(0.0, 60.0),
            grid_width: 256,
            grid_height: 128,
        },
        Some(&scatter_source_rows()),
    )
}

fn scatter_evidence_v3() -> ScatterSelectionEvidenceV3 {
    let evidence_v2 = scatter_evidence_v2();
    let points = scatter_points();
    let summary = SelectedRegionSummary::from_points(&points, scatter_selection());
    let comparison = scatter_selection_comparison(&points, summary);
    let aggregate_context = scatter_aggregate_evidence_context(
        &scatter_overview_with_dense_context(evidence_v2.selected_row_id_sample[0]),
        &evidence_v2.selected_row_id_sample,
    );

    ScatterSelectionEvidenceV3::from_v2_with_presentation(
        &evidence_v2,
        DensityEncoding::scatter_default(),
        rawscope_render::ScatterDensityPresentation::TopographicField,
        comparison,
        aggregate_context,
        Some(DatasetProfileId::LichessGames),
    )
}

fn scatter_overview_with_dense_context(selected_row_id: RowId) -> ScatterAggregateOverview {
    let grid_width = 8;
    let grid_height = 4;
    let mut bins = vec![AggregateBinSample::default(); (grid_width * grid_height) as usize];
    for (index, bin) in bins.iter_mut().enumerate().take(20) {
        bin.count = (32 - index) as u32;
        bin.row_ids = vec![RowId(100 + index as u64), RowId(200 + index as u64)];
    }
    bins[19].count = 1;
    bins[19].row_ids = vec![selected_row_id];

    ScatterAggregateOverview {
        x_range: F32Range::new(0.0, 100.0),
        y_range: F32Range::new(0.0, 100.0),
        grid_width,
        grid_height,
        max_bin_count: 32,
        bins,
    }
}

fn timeline_events() -> Vec<TimelineEventRecord> {
    vec![
        TimelineEventRecord {
            row_id: RowId(0),
            timestamp: 100,
            lane: 0,
            value: 1.0,
            kind: TimelineEventKind::Synthetic(SyntheticEventType::Background),
        },
        TimelineEventRecord {
            row_id: RowId(1),
            timestamp: 140,
            lane: 1,
            value: 2.0,
            kind: TimelineEventKind::Synthetic(SyntheticEventType::Spike),
        },
        TimelineEventRecord {
            row_id: RowId(2),
            timestamp: 340,
            lane: 1,
            value: 3.0,
            kind: TimelineEventKind::Synthetic(SyntheticEventType::HighValueBand),
        },
        TimelineEventRecord {
            row_id: RowId(3),
            timestamp: 180,
            lane: 2,
            value: 4.0,
            kind: TimelineEventKind::Unclassified,
        },
    ]
}

fn timeline_selection() -> TimelineBrushSelection {
    TimelineBrushSelection {
        time_range: U64Range::new(90, 200),
        lane_range: TimelineLaneRange::new(0, 3),
    }
}

fn timeline_source_rows() -> LoadedSourceTable {
    LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "timestamp".to_string(),
                kind: LoadedColumnKind::Integer,
            },
            LoadedColumnSchema {
                name: "provider".to_string(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: vec![
            LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["100".to_string(), "aws".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(1),
                values: vec!["140".to_string(), "gcp".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(2),
                values: vec!["340".to_string(), "gcp".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(3),
                values: vec!["180".to_string(), "azure".to_string()],
            },
        ],
    }
}

fn timeline_evidence_v2() -> TimelineSelectionEvidenceV2 {
    let events = timeline_events();
    let v1 = TimelineSelectionEvidence::from_events(
        &events,
        timeline_selection(),
        3,
        SyntheticDatasetMetadata::new(0, events.len()),
        events.len(),
        TimelineEvidenceConfig { max_sample_size: 3 },
    );
    let dataset_identity = DatasetIdentity::local_csv_timeline(
        PathBuf::from("C:/data/timeline.csv"),
        events.len(),
        Some(events.len()),
        "timestamp",
        "provider",
        vec!["aws".to_string(), "gcp".to_string(), "azure".to_string()],
    );

    TimelineSelectionEvidenceV2::from_v1(
        &v1,
        dataset_identity,
        TimelineEvidenceView {
            time_range: U64Range::new(90, 200),
            full_time_range: U64Range::new(100, 340),
            lane_count: 3,
            grid_width: 256,
            grid_height: 64,
        },
        Some(&timeline_source_rows()),
    )
}

fn timeline_evidence_v3() -> TimelineSelectionEvidenceV3 {
    let evidence_v2 = timeline_evidence_v2();
    let events = timeline_events();
    let summary = TimelineSelectionSummary::from_events(&events, timeline_selection(), 3);
    let comparison = timeline_selection_comparison(&events, 3, &summary);
    let aggregate_context = timeline_aggregate_evidence_context(
        &timeline_overview_with_dense_context(evidence_v2.selected_row_id_sample[0]),
        &evidence_v2.selected_row_id_sample,
    );

    TimelineSelectionEvidenceV3::from_v2(
        &evidence_v2,
        DensityEncoding::timeline_default(),
        comparison,
        aggregate_context,
        Some(DatasetProfileId::LichessGames),
    )
}

fn timeline_overview_with_dense_context(selected_row_id: RowId) -> TimelineAggregateOverview {
    let grid_width = 8;
    let grid_height = 4;
    let mut bins = vec![AggregateBinSample::default(); (grid_width * grid_height) as usize];
    for (index, bin) in bins.iter_mut().enumerate().take(18) {
        bin.count = (24 - index) as u32;
        bin.row_ids = vec![RowId(300 + index as u64), RowId(400 + index as u64)];
    }
    bins[17].count = 2;
    bins[17].row_ids = vec![selected_row_id];

    TimelineAggregateOverview {
        time_range: U64Range::new(0, 500),
        lane_count: 3,
        grid_width,
        grid_height,
        max_bin_count: 24,
        bins,
    }
}

#[test]
fn scatter_v3_json_includes_density_encoding() {
    let json = scatter_selection_evidence_v3_json(&scatter_evidence_v3()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["schema_version"], 3);
    assert_eq!(value["view"]["density_encoding"]["transform"], "log1p");
    assert_eq!(
        value["view"]["density_encoding"]["palette"],
        "scatter_sequential"
    );
    assert_eq!(
        value["view"]["density_encoding"]["normalization"],
        "viewport_max"
    );
    assert_eq!(value["view"]["density_presentation"], "topographic_field");
}

#[test]
fn scatter_v3_json_includes_comparison_baseline() {
    let json = scatter_selection_evidence_v3_json(&scatter_evidence_v3()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["comparison"]["baseline"], "active_point_slice");
    assert_eq!(value["comparison"]["baseline_row_count"], 3);
}

#[test]
fn scatter_v3_json_includes_active_dataset_profile_when_present() {
    let json = scatter_selection_evidence_v3_json(&scatter_evidence_v3()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["active_dataset_profile"], "lichess-games");
}

#[test]
fn scatter_v3_json_bounds_aggregate_context_bins() {
    let json = scatter_selection_evidence_v3_json(&scatter_evidence_v3()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let bins = value["aggregate_context"]["bins"]
        .as_array()
        .expect("aggregate bins should serialize as an array");

    assert_eq!(value["aggregate_context"]["bin_limit"], 16);
    assert_eq!(bins.len(), 16);
    assert!(bins.iter().any(|bin| {
        bin["row_id_sample"]
            .as_array()
            .is_some_and(|row_ids| row_ids.iter().any(|row_id| row_id == 0))
    }));
}

#[test]
fn timeline_v3_markdown_names_encoding_and_baseline() {
    let markdown = timeline_selection_evidence_v3_markdown(&timeline_evidence_v3());

    assert!(
        markdown.contains("Density encoding: log1p(count) | timeline sequential | viewport max")
    );
    assert!(markdown.contains("Active dataset profile: lichess-games"));
    assert!(markdown.contains("Comparison baseline: active_event_slice (4 events)"));
}

#[test]
fn v2_export_functions_remain_callable() {
    let scatter_json = scatter_selection_evidence_v2_json(&scatter_evidence_v2()).unwrap();
    let scatter_markdown = scatter_selection_evidence_v2_markdown(&scatter_evidence_v2());
    let timeline_json = timeline_selection_evidence_v2_json(&timeline_evidence_v2()).unwrap();
    let timeline_markdown = timeline_selection_evidence_v2_markdown(&timeline_evidence_v2());

    assert!(scatter_json.contains("\"schema_version\": 2"));
    assert!(scatter_markdown.contains("# RawScope Scatter Selection Evidence"));
    assert!(timeline_json.contains("\"schema_version\": 2"));
    assert!(timeline_markdown.contains("# RawScope Timeline Selection Evidence"));
}

#[test]
fn scatter_v3_markdown_mentions_bounded_aggregate_context() {
    let markdown = scatter_selection_evidence_v3_markdown(&scatter_evidence_v3());

    assert!(markdown.contains("Included bins: 16 / 16"));
    assert!(markdown.contains("## Aggregate Context"));
}
