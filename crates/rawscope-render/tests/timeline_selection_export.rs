use std::path::PathBuf;

use rawscope_core::{RowId, U64Range};
use rawscope_data::{
    DatasetIdentity, LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable,
    SyntheticDatasetMetadata, SyntheticEventType, TimelineEventKind, TimelineEventRecord,
};
use rawscope_evidence::{
    timeline_selection_evidence_json, timeline_selection_evidence_markdown,
    timeline_selection_evidence_v2_json, TimelineEvidenceConfig, TimelineEvidenceView,
    TimelineLaneRange, TimelineSelectionEvidence, TimelineSelectionEvidenceV2,
};
use rawscope_render::{timeline_selection_evidence_from_events, TimelineBrushSelection};

fn event(
    row_id: u64,
    timestamp: u64,
    lane: u32,
    value: f32,
    event_type: SyntheticEventType,
) -> TimelineEventRecord {
    TimelineEventRecord {
        row_id: RowId(row_id),
        timestamp,
        lane,
        value,
        kind: TimelineEventKind::Synthetic(event_type),
    }
}

fn selection() -> TimelineBrushSelection {
    TimelineBrushSelection {
        time_range: U64Range::new(100, 300),
        lane_range: TimelineLaneRange::new(1, 3),
    }
}

fn timeline_evidence() -> TimelineSelectionEvidence {
    let events = vec![
        event(7, 120, 1, 10.0, SyntheticEventType::Background),
        event(1, 160, 2, 15.0, SyntheticEventType::Spike),
        event(3, 260, 2, 25.0, SyntheticEventType::Spike),
        event(9, 500, 1, 50.0, SyntheticEventType::StaleLane),
    ];

    timeline_selection_evidence_from_events(
        &events,
        selection(),
        4,
        SyntheticDatasetMetadata::new(42, events.len()),
        events.len(),
        TimelineEvidenceConfig { max_sample_size: 2 },
    )
}

fn local_timeline_evidence_v2() -> TimelineSelectionEvidenceV2 {
    let events = vec![
        TimelineEventRecord {
            row_id: RowId(0),
            timestamp: 100,
            lane: 0,
            value: 1.0,
            kind: TimelineEventKind::Unclassified,
        },
        TimelineEventRecord {
            row_id: RowId(1),
            timestamp: 140,
            lane: 1,
            value: 2.0,
            kind: TimelineEventKind::Unclassified,
        },
        TimelineEventRecord {
            row_id: RowId(2),
            timestamp: 340,
            lane: 1,
            value: 3.0,
            kind: TimelineEventKind::Unclassified,
        },
    ];
    let selection = TimelineBrushSelection {
        time_range: U64Range::new(90, 200),
        lane_range: TimelineLaneRange::new(0, 2),
    };
    let v1 = timeline_selection_evidence_from_events(
        &events,
        selection,
        2,
        SyntheticDatasetMetadata::new(0, 3),
        3,
        TimelineEvidenceConfig { max_sample_size: 2 },
    );
    let dataset_identity = DatasetIdentity::local_csv_timeline(
        PathBuf::from("C:/data/timeline.csv"),
        3,
        Some(3),
        "timestamp",
        "provider",
        vec!["aws".to_string(), "gcp".to_string()],
    );
    let source_rows = LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "timestamp".to_string(),
                kind: LoadedColumnKind::Integer,
            },
            LoadedColumnSchema {
                name: "provider".to_string(),
                kind: LoadedColumnKind::String,
            },
            LoadedColumnSchema {
                name: "status".to_string(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: vec![
            LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["100".to_string(), "aws".to_string(), "ok".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(1),
                values: vec!["140".to_string(), "gcp".to_string(), "late".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(2),
                values: vec!["340".to_string(), "gcp".to_string(), "tail".to_string()],
            },
        ],
    };

    TimelineSelectionEvidenceV2::from_v1(
        &v1,
        dataset_identity,
        TimelineEvidenceView {
            time_range: U64Range::new(90, 200),
            full_time_range: U64Range::new(100, 340),
            lane_count: 2,
            grid_width: 256,
            grid_height: 2,
        },
        Some(&source_rows),
    )
}

#[test]
fn json_export_contains_versioned_timeline_shape() {
    let json = timeline_selection_evidence_json(&timeline_evidence()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["artifact_kind"], "timeline-selection-evidence");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["dataset_metadata"]["seed"], 42);
    assert_eq!(value["dataset_metadata"]["row_count"], 4);
    assert_eq!(value["event_count"], 4);
    assert_eq!(value["selected_time_range"]["min"], 100);
    assert_eq!(value["selected_time_range"]["max"], 300);
    assert_eq!(value["selected_lane_range"]["start"], 1);
    assert_eq!(value["selected_lane_range"]["end_exclusive"], 3);
    assert_eq!(value["selected_event_count"], 3);
    assert_eq!(value["lane_counts"], serde_json::json!([0, 1, 2, 0]));
    assert_eq!(value["event_type_counts"]["spike"], 2);
    assert_eq!(value["top_lane"], 2);
    assert_eq!(value["top_event_type"], "Spike");
    assert_eq!(value["row_id_sample"], serde_json::json!([1, 3]));
    assert_eq!(value["selected_event_sample"][0]["row_id"], 1);
    assert_eq!(value["selected_event_sample"][0]["event_type"], "Spike");
}

#[test]
fn markdown_export_contains_key_sections_and_stable_samples() {
    let markdown = timeline_selection_evidence_markdown(&timeline_evidence());

    assert!(markdown.contains("# RawScope Timeline Selection Evidence"));
    assert!(markdown.contains("Synthetic CPU-side evidence"));
    assert!(markdown.contains("## Dataset"));
    assert!(markdown.contains("## Selected Region"));
    assert!(markdown.contains("## Lane Counts"));
    assert!(markdown.contains("## Event-Type Counts"));
    assert!(markdown.contains("## Sampled Row IDs"));
    assert!(markdown.contains("## Sampled Events"));
    assert!(markdown.contains("1, 3"));
    assert!(markdown.contains("| 1 | 160 | 2 | 15.000000 | Spike |"));
    assert!(markdown.contains("| 3 | 260 | 2 | 25.000000 | Spike |"));
}

#[test]
fn markdown_export_handles_empty_selection() {
    let events = vec![event(0, 120, 1, 10.0, SyntheticEventType::Background)];
    let empty_selection = TimelineBrushSelection {
        time_range: U64Range::new(900, 950),
        lane_range: TimelineLaneRange::new(0, 1),
    };
    let evidence = timeline_selection_evidence_from_events(
        &events,
        empty_selection,
        2,
        SyntheticDatasetMetadata::new(42, events.len()),
        events.len(),
        TimelineEvidenceConfig::default(),
    );

    let markdown = timeline_selection_evidence_markdown(&evidence);

    assert!(markdown.contains("- Selected events: 0"));
    assert!(markdown.contains("_No selected row ids._"));
    assert!(markdown.contains("| _none_ |  |  |  |  |"));
}

#[test]
fn timeline_v2_json_contains_lane_labels() {
    let json = timeline_selection_evidence_v2_json(&local_timeline_evidence_v2()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["artifact_kind"], "timeline-selection-evidence");
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["dataset_identity"]["source"]["kind"], "local_csv");
    assert_eq!(
        value["dataset_identity"]["lane_labels"],
        serde_json::json!(["aws", "gcp"])
    );
    assert_eq!(value["event_kind_counts"]["unclassified"], 2);
    assert_eq!(value["top_event_kind"], "unclassified");
    assert_eq!(
        value["selected_source_row_sample"][1]["values"],
        serde_json::json!(["140", "gcp", "late"])
    );
}

#[test]
fn timeline_v2_json_contains_view_range() {
    let json = timeline_selection_evidence_v2_json(&local_timeline_evidence_v2()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["view"]["time_range"]["min"], 90);
    assert_eq!(value["view"]["time_range"]["max"], 200);
    assert_eq!(value["view"]["full_time_range"]["min"], 100);
    assert_eq!(value["view"]["full_time_range"]["max"], 340);
    assert_eq!(value["view"]["grid_width"], 256);
}
