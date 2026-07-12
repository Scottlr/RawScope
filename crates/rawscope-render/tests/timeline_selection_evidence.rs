use rawscope_core::{RowId, U64Range};
use rawscope_data::{
    SyntheticDatasetMetadata, SyntheticEventType, TimelineEventKind, TimelineEventRecord,
};
use rawscope_evidence::{TimelineEvidenceConfig, TimelineLaneRange, TimelineSelectionEvidence};
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

fn events() -> Vec<TimelineEventRecord> {
    vec![
        event(20, 120, 1, 10.0, SyntheticEventType::Background),
        event(5, 180, 1, 20.0, SyntheticEventType::Spike),
        event(12, 250, 2, 30.0, SyntheticEventType::Spike),
        event(2, 260, 4, 40.0, SyntheticEventType::HighValueBand),
        event(3, 400, 1, 50.0, SyntheticEventType::StaleLane),
        event(1, 150, 2, 15.0, SyntheticEventType::Background),
    ]
}

fn evidence_with_config(config: TimelineEvidenceConfig) -> TimelineSelectionEvidence {
    let source_events = events();
    timeline_selection_evidence_from_events(
        &source_events,
        selection(),
        5,
        SyntheticDatasetMetadata::new(42, source_events.len()),
        source_events.len(),
        config,
    )
}

#[test]
fn selected_row_ids_are_stable_lowest_row_ids() {
    let evidence = evidence_with_config(TimelineEvidenceConfig::default());

    assert_eq!(
        evidence.selected_row_id_sample,
        vec![RowId(1), RowId(5), RowId(12), RowId(20)]
    );
    assert_eq!(evidence.selected_event_sample[0].row_id, RowId(1));
}

#[test]
fn sample_size_limit_is_respected() {
    let evidence = evidence_with_config(TimelineEvidenceConfig { max_sample_size: 2 });

    assert_eq!(evidence.selected_row_id_sample, vec![RowId(1), RowId(5)]);
    assert_eq!(evidence.selected_event_sample.len(), 2);
}

#[test]
fn empty_selection_produces_zero_count_and_empty_sample() {
    let source_events = events();
    let empty_selection = TimelineBrushSelection {
        time_range: U64Range::new(900, 950),
        lane_range: TimelineLaneRange::new(0, 1),
    };

    let evidence = timeline_selection_evidence_from_events(
        &source_events,
        empty_selection,
        5,
        SyntheticDatasetMetadata::new(42, source_events.len()),
        source_events.len(),
        TimelineEvidenceConfig::default(),
    );

    assert_eq!(evidence.selected_event_count, 0);
    assert_eq!(evidence.selected_row_id_sample, Vec::<RowId>::new());
    assert_eq!(evidence.selected_event_sample, vec![]);
    assert_eq!(evidence.selected_timestamp_range, None);
    assert_eq!(evidence.selected_value_range, None);
}

#[test]
fn lane_counts_and_event_type_counts_are_correct() {
    let evidence = evidence_with_config(TimelineEvidenceConfig::default());

    assert_eq!(evidence.lane_counts, vec![0, 2, 2, 0, 0]);
    assert_eq!(evidence.event_type_counts.background, 2);
    assert_eq!(evidence.event_type_counts.spike, 2);
    assert_eq!(evidence.event_type_counts.stale_lane, 0);
    assert_eq!(evidence.event_type_counts.high_value_band, 0);
    assert_eq!(evidence.event_type_counts.unclassified, 0);
}

#[test]
fn top_lane_and_top_event_type_are_deterministic() {
    let evidence = evidence_with_config(TimelineEvidenceConfig::default());

    assert_eq!(evidence.top_lane, Some(2));
    assert_eq!(evidence.top_event_type, Some(SyntheticEventType::Spike));
}

#[test]
fn min_max_timestamp_and_value_are_correct() {
    let evidence = evidence_with_config(TimelineEvidenceConfig::default());

    assert_eq!(
        evidence.selected_timestamp_range,
        Some(U64Range::new(120, 250))
    );
    assert_eq!(evidence.selected_value_range, Some((10.0, 30.0)));
}

#[test]
fn metadata_and_selection_are_preserved() {
    let evidence = evidence_with_config(TimelineEvidenceConfig::default());

    assert_eq!(
        evidence.dataset_metadata,
        SyntheticDatasetMetadata::new(42, 6)
    );
    assert_eq!(evidence.event_count, 6);
    assert_eq!(evidence.selected_event_count, 4);
    assert!((evidence.selected_percentage - 66.66667).abs() < 0.001);
    assert_eq!(evidence.selected_time_range, U64Range::new(100, 300));
    assert_eq!(evidence.selected_lane_range, TimelineLaneRange::new(1, 3));
}

#[test]
fn evidence_uses_data_anchored_time_and_lane_selection() {
    let source_events = vec![
        event(0, 110, 0, 1.0, SyntheticEventType::Background),
        event(1, 150, 1, 2.0, SyntheticEventType::Background),
        event(2, 250, 2, 3.0, SyntheticEventType::Spike),
        event(3, 350, 2, 4.0, SyntheticEventType::Spike),
    ];
    let selection = TimelineBrushSelection {
        time_range: U64Range::new(140, 300),
        lane_range: TimelineLaneRange::new(1, 3),
    };

    let evidence = timeline_selection_evidence_from_events(
        &source_events,
        selection,
        3,
        SyntheticDatasetMetadata::new(7, source_events.len()),
        source_events.len(),
        TimelineEvidenceConfig::default(),
    );

    assert_eq!(evidence.selected_row_id_sample, vec![RowId(1), RowId(2)]);
}
