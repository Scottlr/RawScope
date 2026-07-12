use rawscope_core::{F32Range, RowId};
use rawscope_data::{
    ScatterPointKind, ScatterPointRecord, SyntheticDatasetMetadata, SyntheticPointCategory,
};
use rawscope_evidence::{
    ScatterSelectionEvidence, ScatterSelectionGeometry, SelectedPointSample,
    SelectionEvidenceConfig,
};

fn point(row_id: u64, x: f32, y: f32, category: SyntheticPointCategory) -> ScatterPointRecord {
    ScatterPointRecord {
        row_id: RowId(row_id),
        x,
        y,
        kind: ScatterPointKind::Synthetic(category),
    }
}

fn metadata() -> SyntheticDatasetMetadata {
    SyntheticDatasetMetadata::new(42, 5)
}

fn full_selection() -> ScatterSelectionGeometry {
    ScatterSelectionGeometry {
        x_range: F32Range::new(0.0, 100.0),
        y_range: F32Range::new(0.0, 100.0),
    }
}

#[test]
fn selected_row_ids_are_stable_lowest_row_ids() {
    let points = vec![
        point(9, 10.0, 10.0, SyntheticPointCategory::Background),
        point(2, 20.0, 20.0, SyntheticPointCategory::Cluster),
        point(5, 30.0, 30.0, SyntheticPointCategory::Outlier),
    ];

    let evidence = ScatterSelectionEvidence::from_points(
        &points,
        full_selection(),
        metadata(),
        5,
        SelectionEvidenceConfig { max_sample_size: 2 },
    );

    assert_eq!(evidence.selected_row_id_sample, vec![RowId(2), RowId(5)]);
}

#[test]
fn selected_record_sample_is_stable() {
    let points = vec![
        point(3, 30.0, 40.0, SyntheticPointCategory::Background),
        point(1, 10.0, 20.0, SyntheticPointCategory::Cluster),
    ];

    let evidence = ScatterSelectionEvidence::from_points(
        &points,
        full_selection(),
        metadata(),
        5,
        SelectionEvidenceConfig::default(),
    );

    assert_eq!(
        evidence.selected_record_sample,
        vec![
            SelectedPointSample {
                row_id: RowId(1),
                x: 10.0,
                y: 20.0,
                category: Some(SyntheticPointCategory::Cluster),
            },
            SelectedPointSample {
                row_id: RowId(3),
                x: 30.0,
                y: 40.0,
                category: Some(SyntheticPointCategory::Background),
            },
        ]
    );
}

#[test]
fn sample_size_limit_is_respected() {
    let points = vec![
        point(0, 10.0, 10.0, SyntheticPointCategory::Background),
        point(1, 20.0, 20.0, SyntheticPointCategory::Background),
        point(2, 30.0, 30.0, SyntheticPointCategory::Background),
    ];

    let evidence = ScatterSelectionEvidence::from_points(
        &points,
        full_selection(),
        metadata(),
        5,
        SelectionEvidenceConfig { max_sample_size: 2 },
    );

    assert_eq!(evidence.selected_record_sample.len(), 2);
    assert_eq!(evidence.selected_row_id_sample, vec![RowId(0), RowId(1)]);
}

#[test]
fn empty_selection_produces_zero_count_and_empty_sample() {
    let points = vec![point(0, 10.0, 10.0, SyntheticPointCategory::Background)];
    let selection = ScatterSelectionGeometry {
        x_range: F32Range::new(90.0, 100.0),
        y_range: F32Range::new(90.0, 100.0),
    };

    let evidence = ScatterSelectionEvidence::from_points(
        &points,
        selection,
        metadata(),
        5,
        SelectionEvidenceConfig::default(),
    );

    assert_eq!(evidence.selected_row_count, 0);
    assert_eq!(evidence.selected_percentage, 0.0);
    assert!(evidence.selected_row_id_sample.is_empty());
    assert!(evidence.selected_record_sample.is_empty());
    assert_eq!(evidence.selected_x_range, None);
    assert_eq!(evidence.selected_y_range, None);
}

#[test]
fn category_counts_and_top_category_are_deterministic() {
    let points = vec![
        point(0, 10.0, 10.0, SyntheticPointCategory::Cluster),
        point(1, 20.0, 20.0, SyntheticPointCategory::Cluster),
        point(2, 30.0, 30.0, SyntheticPointCategory::Outlier),
    ];

    let evidence = ScatterSelectionEvidence::from_points(
        &points,
        full_selection(),
        metadata(),
        5,
        SelectionEvidenceConfig::default(),
    );

    assert_eq!(evidence.category_counts.cluster, 2);
    assert_eq!(evidence.category_counts.background, 0);
    assert_eq!(evidence.category_counts.outlier, 1);
    assert_eq!(evidence.category_counts.unclassified, 0);
    assert_eq!(evidence.top_category, Some(SyntheticPointCategory::Cluster));
}

#[test]
fn min_max_selected_xy_are_correct() {
    let points = vec![
        point(0, 12.0, 90.0, SyntheticPointCategory::Background),
        point(1, 20.0, 40.0, SyntheticPointCategory::Cluster),
        point(2, 70.0, 30.0, SyntheticPointCategory::Outlier),
    ];

    let evidence = ScatterSelectionEvidence::from_points(
        &points,
        full_selection(),
        metadata(),
        5,
        SelectionEvidenceConfig::default(),
    );

    assert_eq!(evidence.selected_x_range, Some(F32Range::new(12.0, 70.0)));
    assert_eq!(evidence.selected_y_range, Some(F32Range::new(30.0, 90.0)));
}

#[test]
fn evidence_carries_dataset_metadata_and_brush_range() {
    let points = vec![point(0, 12.0, 90.0, SyntheticPointCategory::Background)];
    let selection = ScatterSelectionGeometry {
        x_range: F32Range::new(10.0, 20.0),
        y_range: F32Range::new(80.0, 95.0),
    };

    let evidence = ScatterSelectionEvidence::from_points(
        &points,
        selection,
        metadata(),
        20_000,
        SelectionEvidenceConfig::default(),
    );

    assert_eq!(evidence.dataset_metadata.seed, 42);
    assert_eq!(evidence.dataset_metadata.row_count, 5);
    assert_eq!(evidence.point_preset_row_count, 20_000);
    assert_eq!(evidence.brush_x_range, F32Range::new(10.0, 20.0));
    assert_eq!(evidence.brush_y_range, F32Range::new(80.0, 95.0));
}
