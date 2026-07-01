use rawscope_core::{F32Range, RowId};
use rawscope_data::{SyntheticDatasetMetadata, SyntheticPointCategory, SyntheticPointRecord};
use rawscope_render::{
    scatter_selection_evidence_json, scatter_selection_evidence_markdown, ScatterBrushSelection,
    ScatterSelectionEvidence, SelectionEvidenceConfig,
};

fn point(row_id: u64, x: f32, y: f32, category: SyntheticPointCategory) -> SyntheticPointRecord {
    SyntheticPointRecord {
        row_id: RowId(row_id),
        x,
        y,
        category,
    }
}

fn selection_evidence() -> ScatterSelectionEvidence {
    let points = vec![
        point(7, 70.0, 80.0, SyntheticPointCategory::Outlier),
        point(1, 10.0, 20.0, SyntheticPointCategory::Cluster),
        point(3, 30.0, 40.0, SyntheticPointCategory::Cluster),
    ];
    let selection = ScatterBrushSelection {
        x_range: F32Range::new(0.0, 50.0),
        y_range: F32Range::new(0.0, 60.0),
    };

    ScatterSelectionEvidence::from_points(
        &points,
        selection,
        SyntheticDatasetMetadata::new(42, 3),
        20_000,
        SelectionEvidenceConfig { max_sample_size: 2 },
    )
}

#[test]
fn json_export_contains_versioned_evidence_shape() {
    let json = scatter_selection_evidence_json(&selection_evidence()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["artifact_kind"], "scatter-selection-evidence");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["dataset_metadata"]["seed"], 42);
    assert_eq!(value["dataset_metadata"]["row_count"], 3);
    assert_eq!(value["point_preset_row_count"], 20_000);
    assert_eq!(value["brush_range"]["x"]["min"], 0.0);
    assert_eq!(value["brush_range"]["x"]["max"], 50.0);
    assert_eq!(value["selected_row_count"], 2);
    assert_eq!(value["category_counts"]["cluster"], 2);
    assert_eq!(value["top_category"], "Cluster");
    assert_eq!(value["row_id_sample"], serde_json::json!([1, 3]));
    assert_eq!(value["selected_record_sample"][0]["row_id"], 1);
    assert_eq!(value["selected_record_sample"][0]["category"], "Cluster");
}

#[test]
fn markdown_export_contains_key_sections_and_stable_samples() {
    let markdown = scatter_selection_evidence_markdown(&selection_evidence());

    assert!(markdown.contains("# RawScope Scatter Selection Evidence"));
    assert!(markdown.contains("Synthetic CPU-side evidence"));
    assert!(markdown.contains("## Dataset"));
    assert!(markdown.contains("## Selected Region"));
    assert!(markdown.contains("## Category Counts"));
    assert!(markdown.contains("## Sampled Row IDs"));
    assert!(markdown.contains("## Sampled Records"));
    assert!(markdown.contains("1, 3"));
    assert!(markdown.contains("| 1 | 10.000000 | 20.000000 | Cluster |"));
    assert!(markdown.contains("| 3 | 30.000000 | 40.000000 | Cluster |"));
}

#[test]
fn markdown_export_handles_empty_selection() {
    let points = vec![point(0, 10.0, 20.0, SyntheticPointCategory::Cluster)];
    let selection = ScatterBrushSelection {
        x_range: F32Range::new(90.0, 100.0),
        y_range: F32Range::new(90.0, 100.0),
    };
    let evidence = ScatterSelectionEvidence::from_points(
        &points,
        selection,
        SyntheticDatasetMetadata::new(42, 1),
        20_000,
        SelectionEvidenceConfig::default(),
    );

    let markdown = scatter_selection_evidence_markdown(&evidence);

    assert!(markdown.contains("- Selected rows: 0"));
    assert!(markdown.contains("_No selected row ids._"));
    assert!(markdown.contains("| _none_ |  |  |  |"));
}
