#![cfg(feature = "csv")]

use std::{
    fs,
    path::{Path, PathBuf},
};

use rawscope_data::{
    generate_synthetic_events, generate_synthetic_points, load_scatter_dataset,
    load_timeline_dataset, DatasetFieldRole, DatasetSource, DatasetSourceFormat,
    SyntheticEventConfig, SyntheticPointConfig, VisualDatasetKind,
};

#[test]
fn synthetic_scatter_identity_records_seed_and_row_count() {
    let dataset = generate_synthetic_points(SyntheticPointConfig::new(42, 128));

    assert_eq!(dataset.identity.visual_kind, VisualDatasetKind::Scatter);
    assert_eq!(dataset.identity.row_count, 128);
    assert_eq!(dataset.identity.lane_labels, Vec::<String>::new());
    assert_eq!(dataset.identity.field_bindings.len(), 2);
    assert_eq!(dataset.identity.field_bindings[0].role, DatasetFieldRole::X);
    assert_eq!(
        dataset.identity.field_bindings[0].column_name,
        "synthetic_x"
    );
    assert_eq!(dataset.identity.field_bindings[1].role, DatasetFieldRole::Y);
    assert_eq!(
        dataset.identity.field_bindings[1].column_name,
        "synthetic_y"
    );
    assert!(matches!(
        dataset.identity.source,
        DatasetSource::Synthetic {
            seed: 42,
            generator: "synthetic-points",
        }
    ));
    assert_eq!(
        dataset.identity.source_format(),
        DatasetSourceFormat::Synthetic
    );
    assert_eq!(dataset.identity.portable_source_label(), "synthetic-points");
}

#[test]
fn local_csv_scatter_identity_records_path_limit_and_bindings() {
    let path = write_fixture(
        "scatter_identity",
        "latency_ms,payload_size\n10.5,512\n20,1024\n",
    );

    let dataset = load_scatter_dataset(&path, "latency_ms", "payload_size", Some(1)).unwrap();

    assert_eq!(dataset.identity.visual_kind, VisualDatasetKind::Scatter);
    assert_eq!(dataset.identity.row_count, 1);
    assert_eq!(dataset.identity.lane_labels, Vec::<String>::new());
    assert_eq!(dataset.identity.field_bindings.len(), 2);
    assert_eq!(dataset.identity.field_bindings[0].role, DatasetFieldRole::X);
    assert_eq!(dataset.identity.field_bindings[0].column_name, "latency_ms");
    assert_eq!(dataset.identity.field_bindings[1].role, DatasetFieldRole::Y);
    assert_eq!(
        dataset.identity.field_bindings[1].column_name,
        "payload_size"
    );
    assert!(matches!(
        &dataset.identity.source,
        DatasetSource::LocalCsv {
            path: identity_path,
            limit: Some(1),
        } if identity_path == &path
    ));
    assert_eq!(dataset.identity.source_format(), DatasetSourceFormat::Csv);
    assert_eq!(
        dataset.identity.portable_source_label(),
        path.file_name().unwrap().to_string_lossy()
    );

    remove_fixture(&path);
}

#[test]
fn local_csv_timeline_identity_records_lane_labels() {
    let path = write_fixture(
        "timeline_identity",
        "timestamp,provider\n100,aws\n125,gcp\n150,aws\n",
    );

    let dataset = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();

    assert_eq!(dataset.identity.visual_kind, VisualDatasetKind::Timeline);
    assert_eq!(dataset.identity.row_count, 3);
    assert_eq!(dataset.identity.lane_labels, vec!["aws", "gcp"]);
    assert_eq!(dataset.identity.field_bindings.len(), 2);
    assert_eq!(
        dataset.identity.field_bindings[0].role,
        DatasetFieldRole::Time
    );
    assert_eq!(dataset.identity.field_bindings[0].column_name, "timestamp");
    assert_eq!(
        dataset.identity.field_bindings[1].role,
        DatasetFieldRole::Lane
    );
    assert_eq!(dataset.identity.field_bindings[1].column_name, "provider");
    assert!(matches!(
        &dataset.identity.source,
        DatasetSource::LocalCsv {
            path: identity_path,
            limit: None,
        } if identity_path == &path
    ));

    remove_fixture(&path);
}

#[test]
fn synthetic_timeline_identity_records_lane_labels() {
    let dataset = generate_synthetic_events(SyntheticEventConfig::new(99, 64));

    assert_eq!(dataset.identity.visual_kind, VisualDatasetKind::Timeline);
    assert_eq!(dataset.identity.row_count, 64);
    assert_eq!(dataset.identity.lane_labels.len(), 8);
    assert_eq!(dataset.identity.lane_labels[0], "lane-0");
    assert_eq!(dataset.identity.lane_labels[7], "lane-7");
    assert!(matches!(
        dataset.identity.source,
        DatasetSource::Synthetic {
            seed: 99,
            generator: "synthetic-events",
        }
    ));
}

fn write_fixture(name: &str, contents: &str) -> PathBuf {
    let path = fixture_path(name);
    fs::write(&path, contents).unwrap();
    path
}

fn remove_fixture(path: &Path) {
    fs::remove_file(path).unwrap();
}

fn fixture_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rawscope-{name}-{}-identity.csv",
        std::process::id()
    ));
    if path.exists() {
        fs::remove_file(&path).unwrap();
    }
    path
}
