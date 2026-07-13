use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use rawscope_session::{
    load_session_manifest, parse_session_manifest, parse_session_manifest_versioned,
    session_manifest_json, session_manifest_json_v2, DatasetProfileId, ParsedSessionManifest,
    RawScopeSessionManifestV1, RawScopeSessionManifestV2, ResolvedSessionView,
    SessionColumnBinding, SessionDataFormat, SessionDatasetV1, SessionManifestError,
    SessionProfileHint, SessionSource, SessionViewV1, SessionViewV2,
    RAWSCOPE_SESSION_ARTIFACT_KIND, RAWSCOPE_SESSION_SCHEMA_VERSION,
    RAWSCOPE_SESSION_SCHEMA_VERSION_V2,
};

static NEXT_FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
fn relative_dataset_path_resolves_from_manifest_directory() {
    let fixture = Fixture::new();
    let data_path = fixture.root.join("nested").join("games.csv");
    fs::create_dir_all(data_path.parent().unwrap()).unwrap();
    fs::write(&data_path, "x,y\n1,2\n").unwrap();
    let manifest_path = fixture.root.join("analysis.rawscope.json");
    write_manifest(
        &manifest_path,
        RawScopeSessionManifestV1 {
            artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
            schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION,
            dataset: SessionDatasetV1 {
                path: PathBuf::from("nested/games.csv"),
                format: SessionDataFormat::Csv,
                display_name: Some("Games".to_string()),
                limit: Some(10),
                evidence_key: None,
            },
            view: SessionViewV1::Scatter {
                x: "x".to_string(),
                y: "y".to_string(),
                profile: None,
            },
        },
    );

    let session = load_session_manifest(&manifest_path).unwrap();

    assert_eq!(session.dataset.path, fs::canonicalize(data_path).unwrap());
    assert_eq!(session.dataset.display_name.as_deref(), Some("Games"));
    assert_eq!(session.dataset.limit, Some(10));
}

#[test]
fn session_manifest_round_trips_scatter_and_timeline_views() {
    let manifests = [
        RawScopeSessionManifestV1 {
            artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
            schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION,
            dataset: SessionDatasetV1 {
                path: PathBuf::from("games.parquet"),
                format: SessionDataFormat::Parquet,
                display_name: None,
                limit: None,
                evidence_key: Some("game_id".to_string()),
            },
            view: SessionViewV1::Scatter {
                x: "white_rating".to_string(),
                y: "black_rating".to_string(),
                profile: Some("lichess-games".to_string()),
            },
        },
        RawScopeSessionManifestV1 {
            artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
            schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION,
            dataset: SessionDatasetV1 {
                path: PathBuf::from("events.csv"),
                format: SessionDataFormat::Csv,
                display_name: Some("Events".to_string()),
                limit: None,
                evidence_key: None,
            },
            view: SessionViewV1::Timeline {
                time: "created_at".to_string(),
                lane: "winner".to_string(),
                profile: Some("lichess-games".to_string()),
            },
        },
    ];

    for expected in manifests {
        let json = session_manifest_json(&expected).unwrap();
        let decoded = parse_session_manifest(&json).unwrap();
        assert_eq!(decoded, expected);
    }
}

#[test]
fn unknown_field_and_unsupported_version_are_rejected() {
    let unknown_field = r#"{
        "artifact_kind": "rawscope.session",
        "schema_version": 1,
        "dataset": {"path":"data.csv","format":"csv","unexpected":true},
        "view": {"kind":"scatter","x":"x","y":"y"}
    }"#;
    assert!(matches!(
        parse_session_manifest(unknown_field),
        Err(SessionManifestError::JsonParse { .. })
    ));

    let unsupported_version = r#"{
        "artifact_kind": "rawscope.session",
        "schema_version": 2,
        "dataset": {"path":"data.csv","format":"csv"},
        "view": {"kind":"scatter","x":"x","y":"y"}
    }"#;
    assert!(matches!(
        parse_session_manifest(unsupported_version),
        Err(SessionManifestError::UnsupportedSchemaVersion { actual: 2, .. })
    ));
}

#[test]
fn session_v1_golden_bytes_remain_unchanged() {
    let manifest = manifest("data.csv", SessionDataFormat::Csv, None);
    let json = session_manifest_json(&manifest).unwrap();
    assert_eq!(
        json,
        concat!(
            "{\n",
            "  \"artifact_kind\": \"rawscope.session\",\n",
            "  \"schema_version\": 1,\n",
            "  \"dataset\": {\n",
            "    \"path\": \"data.csv\",\n",
            "    \"format\": \"csv\"\n",
            "  },\n",
            "  \"view\": {\n",
            "    \"kind\": \"scatter\",\n",
            "    \"x\": \"x\",\n",
            "    \"y\": \"y\"\n",
            "  }\n",
            "}\n"
        )
    );
}

#[test]
fn session_v2_numeric_pair_and_time_value_resolve_to_version_neutral_bindings() {
    let numeric = RawScopeSessionManifestV2 {
        artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
        schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION_V2,
        source: SessionSource {
            path: PathBuf::from("data.parquet"),
            format: SessionDataFormat::Parquet,
            display_name: None,
            limit: None,
            evidence_key: None,
        },
        view: SessionViewV2::NumericPair {
            x: SessionColumnBinding::new("x"),
            y: SessionColumnBinding::new("y"),
            category: Some(SessionColumnBinding::new("segment")),
            profile: Some(SessionProfileHint("lichess-games".to_string())),
        },
    };
    let json = session_manifest_json_v2(&numeric).unwrap();
    assert!(matches!(
        parse_session_manifest_versioned(&json).unwrap(),
        ParsedSessionManifest::V2(_)
    ));

    let fixture = Fixture::new();
    let data_path = fixture.root.join("data.parquet");
    fs::write(&data_path, b"placeholder").unwrap();
    let manifest_path = fixture.root.join("analysis.rawscope.json");
    fs::write(&manifest_path, json).unwrap();
    assert!(matches!(
        load_session_manifest(&manifest_path).unwrap().view,
        ResolvedSessionView::NumericPair {
            x,
            y,
            category: Some(category),
            profile: Some(DatasetProfileId::LichessGames),
        } if x == "x" && y == "y" && category == "segment"
    ));

    let time_value = RawScopeSessionManifestV2 {
        view: SessionViewV2::TimeValue {
            time: SessionColumnBinding::new("observed_at"),
            value: SessionColumnBinding::new("value"),
            category: None,
            profile: None,
        },
        ..numeric
    };
    let time_json = session_manifest_json_v2(&time_value).unwrap();
    let time_manifest_path = fixture.root.join("time.rawscope.json");
    fs::write(&time_manifest_path, time_json).unwrap();
    assert!(matches!(
        load_session_manifest(&time_manifest_path).unwrap().view,
        ResolvedSessionView::TimeValue { time, value, category: None, profile: None }
            if time == "observed_at" && value == "value"
    ));
}

#[test]
fn unknown_session_version_reports_supported_versions() {
    let error = parse_session_manifest_versioned(
        r#"{"artifact_kind":"rawscope.session","schema_version":99}"#,
    )
    .expect_err("unknown versions should be rejected");
    assert!(matches!(
        error,
        SessionManifestError::UnsupportedSchemaVersions {
            actual: 99,
            supported: &[1, 2]
        }
    ));
}

#[test]
fn declared_format_must_match_local_file_extension() {
    let fixture = Fixture::new();
    let data_path = fixture.root.join("games.csv");
    fs::write(&data_path, "x,y\n1,2\n").unwrap();
    let manifest_path = fixture.root.join("analysis.rawscope.json");
    write_manifest(
        &manifest_path,
        manifest("games.csv", SessionDataFormat::Parquet, None),
    );

    let error = load_session_manifest(&manifest_path).expect_err("format mismatch should fail");

    assert!(matches!(
        error,
        SessionManifestError::FormatExtensionMismatch {
            format: SessionDataFormat::Parquet,
            ..
        }
    ));
}

#[test]
fn session_rejects_uri_and_zero_limit() {
    let uri = manifest(
        "https://example.test/data.csv",
        SessionDataFormat::Csv,
        None,
    );
    let fixture = Fixture::new();
    let manifest_path = fixture.root.join("analysis.rawscope.json");
    write_manifest(&manifest_path, uri);
    let uri_error = load_session_manifest(&manifest_path)
        .expect_err("URI paths should be rejected before filesystem access");
    assert!(matches!(uri_error, SessionManifestError::UriPath { .. }));

    let zero_limit = manifest("data.csv", SessionDataFormat::Csv, Some(0));
    let zero_limit_json = serde_json::to_string(&zero_limit).unwrap();
    let zero_limit_error =
        parse_session_manifest(&zero_limit_json).expect_err("zero limits should be rejected");
    assert!(matches!(
        zero_limit_error,
        SessionManifestError::InvalidField {
            field: "dataset.limit",
            ..
        }
    ));
}

#[test]
fn resolved_profile_is_typed_without_changing_view_bindings() {
    let fixture = Fixture::new();
    let data_path = fixture.root.join("games.csv");
    fs::write(
        &data_path,
        "created_at,white_rating,black_rating,winner\n1,1500,1600,white\n",
    )
    .unwrap();
    let manifest_path = fixture.root.join("analysis.rawscope.json");
    write_manifest(
        &manifest_path,
        RawScopeSessionManifestV1 {
            artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
            schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION,
            dataset: SessionDatasetV1 {
                path: PathBuf::from("games.csv"),
                format: SessionDataFormat::Csv,
                display_name: None,
                limit: None,
                evidence_key: None,
            },
            view: SessionViewV1::Scatter {
                x: "white_rating".to_string(),
                y: "black_rating".to_string(),
                profile: Some("lichess-games".to_string()),
            },
        },
    );

    let session = load_session_manifest(&manifest_path).unwrap();

    assert!(matches!(
        session.view,
        ResolvedSessionView::NumericPair {
            x,
            y,
            category: None,
            profile: Some(DatasetProfileId::LichessGames),
        } if x == "white_rating" && y == "black_rating"
    ));
}

fn manifest(
    path: &str,
    format: SessionDataFormat,
    limit: Option<u64>,
) -> RawScopeSessionManifestV1 {
    RawScopeSessionManifestV1 {
        artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
        schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION,
        dataset: SessionDatasetV1 {
            path: PathBuf::from(path),
            format,
            display_name: None,
            limit,
            evidence_key: None,
        },
        view: SessionViewV1::Scatter {
            x: "x".to_string(),
            y: "y".to_string(),
            profile: None,
        },
    }
}

fn write_manifest(path: &Path, manifest: RawScopeSessionManifestV1) {
    fs::write(path, session_manifest_json(&manifest).unwrap()).unwrap();
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("rawscope-session-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
