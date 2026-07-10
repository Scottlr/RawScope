use rawscope_core::RowId;
use rawscope_data::{
    available_profile_filter_hints, build_visual_field_catalog, dataset_profile,
    parse_dataset_profile_id, validate_dataset_profile, DatasetProfileId, LoadedColumnKind,
    LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, VisualFieldCatalogConfig,
};

fn schema(columns: &[(&str, LoadedColumnKind)]) -> Vec<LoadedColumnSchema> {
    columns
        .iter()
        .map(|(name, kind)| LoadedColumnSchema {
            name: (*name).to_string(),
            kind: *kind,
        })
        .collect()
}

#[test]
fn parse_known_profile_id() {
    let parsed = parse_dataset_profile_id("lichess-games").unwrap();

    assert_eq!(parsed, DatasetProfileId::LichessGames);
    assert_eq!(parsed.as_str(), "lichess-games");
}

#[test]
fn reject_unknown_profile_id() {
    let err =
        parse_dataset_profile_id("mystery-games").expect_err("unknown dataset profile should fail");

    assert!(err.to_string().contains("mystery-games"));
    assert!(err.to_string().contains("lichess-games"));
}

#[test]
fn validate_profile_accepts_required_columns() {
    let profile = dataset_profile(DatasetProfileId::LichessGames);
    let columns = schema(&[
        ("created_at", LoadedColumnKind::Integer),
        ("white_rating", LoadedColumnKind::Integer),
        ("black_rating", LoadedColumnKind::Integer),
        ("winner", LoadedColumnKind::String),
        ("termination", LoadedColumnKind::String),
    ]);

    validate_dataset_profile(profile, &columns).unwrap();
}

#[test]
fn validate_profile_reports_missing_columns() {
    let profile = dataset_profile(DatasetProfileId::LichessGames);
    let columns = schema(&[
        ("created_at", LoadedColumnKind::Integer),
        ("white_rating", LoadedColumnKind::Integer),
    ]);

    let err = validate_dataset_profile(profile, &columns)
        .expect_err("profile validation should report missing columns");

    assert_eq!(
        err.missing_columns,
        vec!["black_rating".to_string(), "winner".to_string()]
    );
    assert!(err
        .to_string()
        .contains("missing required columns: black_rating, winner"));
}

#[test]
fn validate_profile_reports_type_mismatch() {
    let profile = dataset_profile(DatasetProfileId::LichessGames);
    let columns = schema(&[
        ("created_at", LoadedColumnKind::String),
        ("white_rating", LoadedColumnKind::Integer),
        ("black_rating", LoadedColumnKind::Float),
        ("winner", LoadedColumnKind::String),
    ]);

    let err = validate_dataset_profile(profile, &columns)
        .expect_err("profile validation should report type mismatches");

    assert_eq!(err.type_mismatches.len(), 2);
    assert!(err
        .to_string()
        .contains("created_at expected integer but found string"));
    assert!(err
        .to_string()
        .contains("black_rating expected integer but found float"));
}

#[test]
fn lichess_optional_hints_do_not_change_profile_validation() {
    let profile = dataset_profile(DatasetProfileId::LichessGames);
    let required_only = schema(&[
        ("created_at", LoadedColumnKind::Integer),
        ("white_rating", LoadedColumnKind::Integer),
        ("black_rating", LoadedColumnKind::Integer),
        ("winner", LoadedColumnKind::String),
    ]);

    validate_dataset_profile(profile, &required_only).unwrap();
    assert!(profile.scatter_hints.show_equality_guide);
    assert!(profile.scatter_hints.supports_mean_difference);
    assert_eq!(profile.required_columns.len(), 4);
}

#[test]
fn lichess_profile_recommends_but_does_not_default_projection() {
    let profile = dataset_profile(DatasetProfileId::LichessGames);

    assert!(profile.scatter_hints.supports_mean_difference);
    assert_eq!(
        rawscope_data::ScatterProjection::default(),
        rawscope_data::ScatterProjection::RawXY
    );
}

#[test]
fn lichess_profile_defaults_are_curated_for_visual_analysis() {
    let defaults = dataset_profile(DatasetProfileId::LichessGames).scatter_defaults;

    assert!(defaults.show_equality_guide);
    assert!(defaults.suggest_mean_difference);
    assert_eq!(
        &defaults.preferred_filter_columns[..3],
        &["winner", "category", "time_control"]
    );
}

#[test]
fn available_hints_skip_missing_columns() {
    let profile = dataset_profile(DatasetProfileId::LichessGames);
    let source = LoadedSourceTable {
        columns: schema(&[
            ("winner", LoadedColumnKind::String),
            ("category", LoadedColumnKind::Integer),
            ("opening", LoadedColumnKind::String),
        ]),
        rows: vec![LoadedSourceRow {
            row_id: RowId(0),
            values: vec!["white".into(), "1".into(), "Sicilian".into()],
        }],
    };
    let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());

    let hints = available_profile_filter_hints(profile, &catalog);

    assert_eq!(
        hints
            .iter()
            .map(|hint| hint.column_name)
            .collect::<Vec<_>>(),
        vec!["winner", "opening"]
    );
}
