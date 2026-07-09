use rawscope_core::{RowId, U64Range};
use rawscope_data::{
    SyntheticDatasetMetadata, SyntheticEventType, TimelineEventKind, TimelineEventRecord,
};
use rawscope_render::{
    timeline_selection_evidence_json, timeline_selection_evidence_markdown, TimelineBrushSelection,
    TimelineEvidenceConfig, TimelineLaneRange, TimelineSelectionEvidence,
};

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

    TimelineSelectionEvidence::from_events(
        &events,
        selection(),
        4,
        SyntheticDatasetMetadata::new(42, events.len()),
        events.len(),
        TimelineEvidenceConfig { max_sample_size: 2 },
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
    let evidence = TimelineSelectionEvidence::from_events(
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
