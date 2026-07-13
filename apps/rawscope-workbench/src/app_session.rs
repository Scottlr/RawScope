//! Workbench startup projection for versioned local session manifests.

use std::{io, path::PathBuf};

use rawscope_data::DatasetProfileId as DataProfileId;
use rawscope_data::{validate_evidence_key, DatasetEvidenceKey, LoadedSourceTable};
use rawscope_session::{
    load_session_manifest, ResolvedRawScopeSession, ResolvedSessionView, SessionDataFormat,
};

use crate::{
    app::WorkbenchApp,
    cli::{WorkbenchArgs, WorkbenchInput},
    demo::DemoMode,
};

pub(crate) use crate::app_session_visual_field::{
    resolve_visual_field_mapping, validate_category_binding,
};

/// Startup values shared by `main` and the workbench application state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkbenchStartup {
    pub(crate) demo_mode: DemoMode,
    pub(crate) input: Option<WorkbenchInput>,
    pub(crate) compare_input: Option<PathBuf>,
    pub(crate) session: Option<PendingSessionContext>,
}

/// Session metadata retained until its loaded source table can validate the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingSessionContext {
    pub(crate) manifest_path: PathBuf,
    pub(crate) schema_version: u32,
    pub(crate) display_name: Option<String>,
    pub(crate) data_format: SessionDataFormat,
    pub(crate) evidence_key: Option<String>,
}

/// Session metadata that is safe to use after the source table has loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveSessionContext {
    pub(crate) manifest_path: PathBuf,
    pub(crate) schema_version: u32,
    pub(crate) display_name: Option<String>,
    pub(crate) data_format: SessionDataFormat,
    pub(crate) evidence_key: Option<DatasetEvidenceKey>,
}

/// Resolves direct CLI arguments or one validated session manifest into startup state.
pub(crate) fn resolve_workbench_startup(
    args: WorkbenchArgs,
) -> Result<WorkbenchStartup, io::Error> {
    let Some(session_path) = args.session_path else {
        return Ok(WorkbenchStartup {
            demo_mode: args.demo_mode,
            input: args.input,
            compare_input: args.compare_input,
            session: None,
        });
    };

    let session = load_session_manifest(&session_path).map_err(|source| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "failed to load RawScope session manifest '{}': {source}",
                session_path.display()
            ),
        )
    })?;
    startup_from_session(session)
}

/// Projects syntax-only CLI arguments into initial app state without reading a manifest.
pub(crate) fn startup_without_session(args: WorkbenchArgs) -> WorkbenchStartup {
    WorkbenchStartup {
        demo_mode: args.demo_mode,
        input: args.input,
        compare_input: args.compare_input,
        session: None,
    }
}

fn startup_from_session(session: ResolvedRawScopeSession) -> Result<WorkbenchStartup, io::Error> {
    let ResolvedRawScopeSession {
        manifest_path,
        schema_version,
        dataset,
        view,
    } = session;
    let pending_session = PendingSessionContext {
        manifest_path,
        schema_version,
        display_name: dataset.display_name,
        data_format: dataset.format,
        evidence_key: dataset.evidence_key,
    };

    let (demo_mode, input) = match view {
        ResolvedSessionView::NumericPair {
            x,
            y,
            category,
            profile,
        } => (
            DemoMode::Scatter,
            WorkbenchInput::Scatter {
                path: dataset.path,
                x_column: Some(x),
                y_column: Some(y),
                category_column: category,
                projection: crate::cli::VisualFieldProjectionKind::NumericPair,
                limit: session_row_limit(dataset.limit)?,
                profile: profile.map(contract_profile_to_data),
            },
        ),
        ResolvedSessionView::TimelineLane {
            time,
            lane,
            profile,
        } => (
            DemoMode::Timeline,
            WorkbenchInput::Timeline {
                path: dataset.path,
                time_column: Some(time),
                lane_column: Some(lane),
                limit: session_row_limit(dataset.limit)?,
                profile: profile.map(contract_profile_to_data),
            },
        ),
        ResolvedSessionView::TimeValue {
            time,
            value,
            category,
            profile,
        } => (
            DemoMode::Scatter,
            WorkbenchInput::Scatter {
                path: dataset.path,
                x_column: Some(time),
                y_column: Some(value),
                category_column: category,
                projection: crate::cli::VisualFieldProjectionKind::TimeValue,
                limit: session_row_limit(dataset.limit)?,
                profile: profile.map(contract_profile_to_data),
            },
        ),
    };

    Ok(WorkbenchStartup {
        demo_mode,
        input: Some(input),
        compare_input: None,
        session: Some(pending_session),
    })
}

fn session_row_limit(limit: Option<u64>) -> Result<Option<usize>, io::Error> {
    limit
        .map(|value| {
            usize::try_from(value).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "session row limit does not fit this platform",
                )
            })
        })
        .transpose()
}

fn contract_profile_to_data(profile: rawscope_session::DatasetProfileId) -> DataProfileId {
    match profile {
        rawscope_session::DatasetProfileId::LichessGames => DataProfileId::LichessGames,
    }
}

impl WorkbenchApp {
    pub(crate) fn activate_session_context(
        &mut self,
        source: &LoadedSourceTable,
    ) -> Result<(), io::Error> {
        let Some(pending) = self.workbench_state.pending_session.as_ref() else {
            self.workbench_state.active_session = None;
            return Ok(());
        };
        let evidence_key = pending
            .evidence_key
            .as_deref()
            .map(|column_name| validate_evidence_key(source, column_name))
            .transpose()
            .map_err(|source| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "failed to validate session evidence key for '{}': {source}",
                        pending.manifest_path.display()
                    ),
                )
            })?;
        self.workbench_state.active_session = Some(ActiveSessionContext {
            manifest_path: pending.manifest_path.clone(),
            schema_version: pending.schema_version,
            display_name: pending.display_name.clone(),
            data_format: pending.data_format,
            evidence_key,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::{fs, path::PathBuf};

    use rawscope_core::RowId;
    use rawscope_data::{
        DatasetProfileId, DatasetSchema, LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow,
        StoreColumnKind,
    };
    use rawscope_session::{
        session_manifest_json, session_manifest_json_v2, RawScopeSessionManifestV1,
        RawScopeSessionManifestV2, ResolvedSessionView, SessionColumnBinding, SessionDataFormat,
        SessionDatasetV1, SessionSource, SessionViewV1, SessionViewV2,
        RAWSCOPE_SESSION_ARTIFACT_KIND, RAWSCOPE_SESSION_SCHEMA_VERSION,
        RAWSCOPE_SESSION_SCHEMA_VERSION_V2,
    };

    use super::*;

    static NEXT_FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn session_scatter_maps_to_existing_workbench_input() {
        let fixture = write_session(
            SessionViewV1::Scatter {
                x: "white_rating".to_string(),
                y: "black_rating".to_string(),
                profile: Some("lichess-games".to_string()),
            },
            Some("Matchmaking"),
        );
        let args = WorkbenchArgs::parse([
            "--session".to_string(),
            fixture.manifest.to_string_lossy().to_string(),
        ])
        .unwrap();

        let startup = resolve_workbench_startup(args).unwrap();

        assert_eq!(startup.demo_mode, DemoMode::Scatter);
        assert_eq!(
            startup.input,
            Some(WorkbenchInput::Scatter {
                path: fs::canonicalize(&fixture.data).unwrap(),
                x_column: Some("white_rating".to_string()),
                y_column: Some("black_rating".to_string()),
                category_column: None,
                projection: crate::cli::VisualFieldProjectionKind::NumericPair,
                limit: None,
                profile: Some(DatasetProfileId::LichessGames),
            })
        );
        assert_eq!(
            startup.session.as_ref().unwrap().display_name.as_deref(),
            Some("Matchmaking")
        );
    }

    #[test]
    fn session_timeline_maps_to_existing_workbench_input() {
        let fixture = write_session(
            SessionViewV1::Timeline {
                time: "created_at".to_string(),
                lane: "winner".to_string(),
                profile: Some("lichess-games".to_string()),
            },
            None,
        );
        let args = WorkbenchArgs::parse([
            "--session".to_string(),
            fixture.manifest.to_string_lossy().to_string(),
        ])
        .unwrap();

        let startup = resolve_workbench_startup(args).unwrap();

        assert_eq!(startup.demo_mode, DemoMode::Timeline);
        assert!(matches!(
            startup.input,
            Some(WorkbenchInput::Timeline {
                time_column: Some(time),
                lane_column: Some(lane),
                ..
            }) if time == "created_at" && lane == "winner"
        ));
    }

    #[test]
    fn session_v2_numeric_pair_preserves_category_for_generic_controller_path() {
        let fixture = write_v2_session();
        let args = WorkbenchArgs::parse([
            "--session".to_string(),
            fixture.manifest.to_string_lossy().to_string(),
        ])
        .unwrap();

        let startup = resolve_workbench_startup(args).unwrap();

        assert_eq!(
            startup.input,
            Some(WorkbenchInput::Scatter {
                path: fs::canonicalize(&fixture.data).unwrap(),
                x_column: Some("x".to_string()),
                y_column: Some("y".to_string()),
                category_column: Some("segment".to_string()),
                projection: crate::cli::VisualFieldProjectionKind::NumericPair,
                limit: None,
                profile: None,
            })
        );
    }

    #[test]
    fn session_v2_time_value_preserves_time_role_for_scatter_field_path() {
        let fixture = write_v2_session();
        let manifest_value = RawScopeSessionManifestV2 {
            artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
            schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION_V2,
            source: SessionSource {
                path: PathBuf::from("games.csv"),
                format: SessionDataFormat::Csv,
                display_name: None,
                limit: None,
                evidence_key: None,
            },
            view: SessionViewV2::TimeValue {
                time: SessionColumnBinding::new("x"),
                value: SessionColumnBinding::new("y"),
                category: Some(SessionColumnBinding::new("segment")),
                profile: None,
            },
        };
        fs::write(
            &fixture.manifest,
            session_manifest_json_v2(&manifest_value).unwrap(),
        )
        .unwrap();

        let args = WorkbenchArgs::parse([
            "--session".to_string(),
            fixture.manifest.to_string_lossy().to_string(),
        ])
        .unwrap();
        let startup = resolve_workbench_startup(args).unwrap();

        assert!(matches!(
            startup.input,
            Some(WorkbenchInput::Scatter {
                projection: crate::cli::VisualFieldProjectionKind::TimeValue,
                category_column: Some(category),
                ..
            }) if category == "segment"
        ));
    }

    #[test]
    fn session_evidence_key_is_validated_after_load() {
        let fixture = write_session_with_key("game_id");
        let args = WorkbenchArgs::parse([
            "--session".to_string(),
            fixture.manifest.to_string_lossy().to_string(),
        ])
        .unwrap();
        let startup = resolve_workbench_startup(args).unwrap();
        let mut app = WorkbenchApp::new(startup);
        let source = rawscope_data::LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "game_id".to_string(),
                kind: LoadedColumnKind::String,
            }],
            rows: vec![LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["g-1".to_string()],
            }],
        };

        app.activate_session_context(&source).unwrap();

        assert_eq!(
            app.workbench_state
                .active_session
                .as_ref()
                .and_then(|session| session.evidence_key.as_ref())
                .and_then(|key| key.value(&source.rows[0])),
            Some("g-1")
        );
    }

    #[test]
    fn incompatible_session_category_is_rejected_before_render_setup() {
        let source = rawscope_data::LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "category".to_string(),
                kind: LoadedColumnKind::Float,
            }],
            rows: Vec::new(),
        };

        let error = validate_category_binding(&source, Some("category"))
            .expect_err("numeric category columns are not categorical bindings");
        assert!(error.to_string().contains("incompatible source kind"));
    }

    #[test]
    fn time_value_session_intent_resolves_through_the_generic_mapping_constructor() {
        let schema = DatasetSchema::try_new([
            ("observed_at", StoreColumnKind::TimestampMicros),
            ("value", StoreColumnKind::F64),
            ("segment", StoreColumnKind::Utf8),
        ])
        .unwrap();
        let view = ResolvedSessionView::TimeValue {
            time: "observed_at".to_string(),
            value: "value".to_string(),
            category: Some("segment".to_string()),
            profile: None,
        };

        let mapping = resolve_visual_field_mapping(&view, &schema).unwrap();

        assert!(matches!(
            mapping.projection(),
            rawscope_analysis::visual_field::VisualFieldProjection::TimeValue { .. }
        ));
        assert!(mapping.category().is_some());
    }

    struct Fixture {
        root: PathBuf,
        data: PathBuf,
        manifest: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    fn write_session(view: SessionViewV1, display_name: Option<&str>) -> Fixture {
        write_session_with_options(view, display_name, None)
    }

    fn write_session_with_key(column: &str) -> Fixture {
        write_session_with_options(
            SessionViewV1::Scatter {
                x: "white_rating".to_string(),
                y: "black_rating".to_string(),
                profile: Some("lichess-games".to_string()),
            },
            None,
            Some(column),
        )
    }

    fn write_session_with_options(
        view: SessionViewV1,
        display_name: Option<&str>,
        evidence_key: Option<&str>,
    ) -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "rawscope-workbench-session-test-{}-{}-{}",
            std::process::id(),
            display_name.unwrap_or("default"),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let data = root.join("games.csv");
        fs::write(
            &data,
            "created_at,white_rating,black_rating,winner,game_id\n1,1500,1600,white,g-1\n",
        )
        .unwrap();
        let manifest = root.join("analysis.rawscope.json");
        let manifest_value = RawScopeSessionManifestV1 {
            artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
            schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION,
            dataset: SessionDatasetV1 {
                path: PathBuf::from("games.csv"),
                format: SessionDataFormat::Csv,
                display_name: display_name.map(str::to_string),
                limit: None,
                evidence_key: evidence_key.map(str::to_string),
            },
            view,
        };
        fs::write(&manifest, session_manifest_json(&manifest_value).unwrap()).unwrap();
        Fixture {
            root,
            data,
            manifest,
        }
    }

    fn write_v2_session() -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "rawscope-workbench-session-v2-test-{}-{}",
            std::process::id(),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let data = root.join("games.csv");
        fs::write(&data, "x,y,segment\n1,2,a\n").unwrap();
        let manifest = root.join("analysis.rawscope.json");
        let manifest_value = RawScopeSessionManifestV2 {
            artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
            schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION_V2,
            source: SessionSource {
                path: PathBuf::from("games.csv"),
                format: SessionDataFormat::Csv,
                display_name: None,
                limit: None,
                evidence_key: None,
            },
            view: SessionViewV2::NumericPair {
                x: SessionColumnBinding::new("x"),
                y: SessionColumnBinding::new("y"),
                category: Some(SessionColumnBinding::new("segment")),
                profile: None,
            },
        };
        fs::write(
            &manifest,
            session_manifest_json_v2(&manifest_value).unwrap(),
        )
        .unwrap();
        Fixture {
            root,
            data,
            manifest,
        }
    }
}
