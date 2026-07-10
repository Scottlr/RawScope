use rawscope_data::{
    dataset_profile, parse_dataset_profile_id, validate_dataset_profile, DatasetProfileId,
    LoadedColumnKind, LoadedColumnSchema,
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
