use std::{fs, path::PathBuf, sync::atomic::AtomicUsize, sync::atomic::Ordering};

use rawscope_adapters::LocalCsvSession;
use rawscope_session::{load_session_manifest, DatasetProfileId, ResolvedSessionView};

static NEXT_FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
fn local_csv_adapter_prepares_a_profiled_external_session() {
    let fixture = Fixture::new();
    let dataset_path = fixture.root.join("games.csv");
    fs::write(
        &dataset_path,
        "created_at,white_rating,black_rating,winner,game_id\n1,1500,1600,white,g-1\n",
    )
    .unwrap();

    let prepared = LocalCsvSession::scatter(&dataset_path, "white_rating", "black_rating")
        .profile("lichess-games")
        .display_name("Lichess games")
        .evidence_key("game_id")
        .limit(1)
        .prepare(fixture.root.join("session"))
        .unwrap();

    assert_eq!(
        prepared.dataset_path(),
        fs::canonicalize(dataset_path).unwrap()
    );
    assert_eq!(prepared.row_count(), None);

    let session = load_session_manifest(prepared.manifest_path()).unwrap();
    assert_eq!(
        session.dataset.display_name.as_deref(),
        Some("Lichess games")
    );
    assert_eq!(session.dataset.evidence_key.as_deref(), Some("game_id"));
    assert_eq!(session.dataset.limit, Some(1));
    assert_eq!(
        session.view,
        ResolvedSessionView::Scatter {
            x: "white_rating".to_string(),
            y: "black_rating".to_string(),
            profile: Some(DatasetProfileId::LichessGames),
        }
    );
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "rawscope-adapters-local-csv-test-{}-{id}",
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
