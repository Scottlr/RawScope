use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use rawscope_session::{
    session_manifest_json, RawScopeSessionManifestV1, SessionDataFormat, SessionDatasetV1,
    SessionViewV1, WorkbenchLaunchError, WorkbenchLauncher, RAWSCOPE_SESSION_ARTIFACT_KIND,
    RAWSCOPE_SESSION_SCHEMA_VERSION,
};

static NEXT_FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
fn explicit_missing_workbench_path_fails_after_session_validation() {
    let fixture = Fixture::new();
    let dataset_path = fixture.root.join("data.csv");
    fs::write(&dataset_path, "x,y\n1,2\n").unwrap();
    let manifest_path = fixture.root.join("analysis.rawscope.json");
    write_manifest(&manifest_path);
    let missing_executable = fixture.root.join("missing-rawscope-workbench");

    let error = WorkbenchLauncher::new()
        .with_executable(&missing_executable)
        .launch(&manifest_path)
        .expect_err("a configured executable path must exist");

    assert!(matches!(
        error,
        WorkbenchLaunchError::ExecutableMetadata { path, .. } if path == missing_executable
    ));
}

fn write_manifest(path: &Path) {
    let manifest = RawScopeSessionManifestV1 {
        artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
        schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION,
        dataset: SessionDatasetV1 {
            path: PathBuf::from("data.csv"),
            format: SessionDataFormat::Csv,
            display_name: None,
            limit: None,
            evidence_key: None,
        },
        view: SessionViewV1::Scatter {
            x: "x".to_string(),
            y: "y".to_string(),
            profile: None,
        },
    };
    fs::write(path, session_manifest_json(&manifest).unwrap()).unwrap();
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "rawscope-session-launcher-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
