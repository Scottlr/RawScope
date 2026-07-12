use std::path::PathBuf;

use rawscope_core::{F32Range, RowId};
use rawscope_data::{
    DatasetIdentity, LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable,
    ScatterPointKind, ScatterPointRecord, SyntheticDatasetMetadata, SyntheticPointCategory,
};
use rawscope_render::{
    scatter_selection_evidence_json, scatter_selection_evidence_markdown,
    scatter_selection_evidence_v2_json, scatter_selection_evidence_v2_markdown,
    ScatterBrushSelection, ScatterEvidenceView, ScatterSelectionEvidence,
    ScatterSelectionEvidenceV2, SelectionEvidenceConfig,
};

fn point(row_id: u64, x: f32, y: f32, category: SyntheticPointCategory) -> ScatterPointRecord {
    ScatterPointRecord {
        row_id: RowId(row_id),
        x,
        y,
        kind: ScatterPointKind::Synthetic(category),
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

fn local_selection_evidence_v2() -> ScatterSelectionEvidenceV2 {
    let points = vec![
        ScatterPointRecord {
            row_id: RowId(0),
            x: 10.0,
            y: 20.0,
            kind: ScatterPointKind::Unclassified,
        },
        ScatterPointRecord {
            row_id: RowId(1),
            x: 30.0,
            y: 40.0,
            kind: ScatterPointKind::Unclassified,
        },
        ScatterPointRecord {
            row_id: RowId(2),
            x: 90.0,
            y: 95.0,
            kind: ScatterPointKind::Unclassified,
        },
    ];
    let selection = ScatterBrushSelection {
        x_range: F32Range::new(0.0, 50.0),
        y_range: F32Range::new(0.0, 60.0),
    };
    let v1 = ScatterSelectionEvidence::from_points(
        &points,
        selection,
        SyntheticDatasetMetadata::new(0, 3),
        3,
        SelectionEvidenceConfig { max_sample_size: 2 },
    );
    let dataset_identity = DatasetIdentity::local_csv_scatter(
        PathBuf::from("C:/data/latency.csv"),
        3,
        Some(3),
        "latency_ms",
        "payload_size",
    );
    let source_rows = LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "latency_ms".to_string(),
                kind: LoadedColumnKind::Float,
            },
            LoadedColumnSchema {
                name: "payload_size".to_string(),
                kind: LoadedColumnKind::Integer,
            },
            LoadedColumnSchema {
                name: "label".to_string(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: vec![
            LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["10".to_string(), "512".to_string(), "api".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(1),
                values: vec!["30".to_string(), "256".to_string(), "batch".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(2),
                values: vec!["90".to_string(), "128".to_string(), "tail".to_string()],
            },
        ],
    };

    ScatterSelectionEvidenceV2::from_v1(
        &v1,
        dataset_identity,
        ScatterEvidenceView {
            x_range: F32Range::new(0.0, 50.0),
            y_range: F32Range::new(0.0, 60.0),
            grid_width: 256,
            grid_height: 128,
        },
        Some(&source_rows),
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

#[test]
fn scatter_v2_json_contains_dataset_identity_and_bindings() {
    let json = scatter_selection_evidence_v2_json(&local_selection_evidence_v2()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["artifact_kind"], "scatter-selection-evidence");
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["dataset_identity"]["visual_kind"], "scatter");
    assert_eq!(value["dataset_identity"]["source"]["kind"], "local_csv");
    assert_eq!(value["dataset_identity"]["field_bindings"][0]["role"], "x");
    assert_eq!(
        value["dataset_identity"]["field_bindings"][0]["column_name"],
        "latency_ms"
    );
    assert_eq!(value["view"]["grid_width"], 256);
    assert_eq!(value["point_kind_counts"]["unclassified"], 2);
    assert_eq!(value["top_point_kind"], "unclassified");
    assert_eq!(
        value["source_columns"],
        serde_json::json!(["latency_ms", "payload_size", "label"])
    );
    assert_eq!(
        value["selected_source_row_sample"][0]["values"],
        serde_json::json!(["10", "512", "api"])
    );
}

#[test]
fn scatter_v2_markdown_contains_source_rows_for_local_csv() {
    let markdown = scatter_selection_evidence_v2_markdown(&local_selection_evidence_v2());

    assert!(markdown.contains("CPU-side selection evidence"));
    assert!(markdown.contains("Source: local_csv (latency.csv, limit 3)"));
    assert!(markdown.contains("Field bindings: x=latency_ms, y=payload_size"));
    assert!(markdown.contains("## Sampled Source Rows"));
    assert!(markdown.contains("Columns: latency_ms | payload_size | label"));
    assert!(markdown.contains("- Row 0: 10 | 512 | api"));
}
