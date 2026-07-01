//! Evidence artifact export helpers for the workbench scatter-density demo.

use std::{
    error::Error,
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rawscope_render::{
    scatter_selection_evidence_json, scatter_selection_evidence_markdown, ScatterSelectionEvidence,
    SCATTER_SELECTION_EVIDENCE_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_SCHEMA_VERSION,
};
use tracing::{error, info, warn};

use crate::app::WorkbenchApp;

const EXPORT_DIR: &str = "target/rawscope-exports";
const EXPORT_MANIFEST_FILE_NAME: &str = "manifest.jsonl";
const SCATTER_SELECTION_FILE_STEM: &str = "scatter-selection";

impl WorkbenchApp {
    pub(crate) fn export_selection_evidence(&mut self) {
        let Some(evidence) = self.selection_evidence.clone() else {
            warn!(
                reason = "no finalized scatter selection evidence",
                "scatter selection evidence export skipped"
            );
            return;
        };

        let export_counter = self.next_evidence_export_counter();
        let export_timestamp_unix_ms = current_unix_timestamp_ms();
        let export_paths = ScatterSelectionExportPaths::next_available(
            EXPORT_DIR,
            export_timestamp_unix_ms,
            export_counter,
        );
        if let Err(err) = export_paths.write(&evidence) {
            error!(
                error = %err,
                json_path = %export_paths.json_path.display(),
                markdown_path = %export_paths.markdown_path.display(),
                manifest_path = %export_paths.manifest_path.display(),
                "failed to export scatter selection evidence"
            );
            return;
        }
        self.evidence_export_counter = export_paths.export_counter;

        info!(
            export_counter = export_paths.export_counter,
            json_path = %export_paths.json_path.display(),
            markdown_path = %export_paths.markdown_path.display(),
            manifest_path = %export_paths.manifest_path.display(),
            selected_row_count = evidence.selected_row_count,
            "RawScope scatter selection evidence exported"
        );
    }

    fn next_evidence_export_counter(&mut self) -> u64 {
        self.evidence_export_counter += 1;
        self.evidence_export_counter
    }
}

struct ScatterSelectionExportPaths {
    json_path: PathBuf,
    markdown_path: PathBuf,
    manifest_path: PathBuf,
    export_timestamp_unix_ms: u128,
    export_counter: u64,
}

impl ScatterSelectionExportPaths {
    fn next_available(
        output_dir: impl AsRef<Path>,
        export_timestamp_unix_ms: u128,
        initial_export_counter: u64,
    ) -> Self {
        let output_dir = output_dir.as_ref();
        let mut export_counter = initial_export_counter;
        loop {
            let paths = Self::new(output_dir, export_timestamp_unix_ms, export_counter);
            let candidate_paths_exist = paths.json_path.exists() || paths.markdown_path.exists();
            if !candidate_paths_exist {
                return paths;
            }
            export_counter += 1;
        }
    }

    fn new(
        output_dir: impl AsRef<Path>,
        export_timestamp_unix_ms: u128,
        export_counter: u64,
    ) -> Self {
        let output_dir = output_dir.as_ref();
        let file_stem =
            format!("{SCATTER_SELECTION_FILE_STEM}-{export_timestamp_unix_ms}-{export_counter}");
        let json_path = output_dir.join(format!("{file_stem}.json"));
        let markdown_path = output_dir.join(format!("{file_stem}.md"));
        let manifest_path = output_dir.join(EXPORT_MANIFEST_FILE_NAME);

        Self {
            json_path,
            markdown_path,
            manifest_path,
            export_timestamp_unix_ms,
            export_counter,
        }
    }

    fn write(&self, evidence: &ScatterSelectionEvidence) -> Result<(), Box<dyn Error>> {
        let Some(output_dir) = self.json_path.parent() else {
            return Err("scatter selection export path has no parent directory".into());
        };

        fs::create_dir_all(output_dir)?;
        let json = scatter_selection_evidence_json(evidence)?;
        let markdown = scatter_selection_evidence_markdown(evidence);
        fs::write(&self.json_path, json)?;
        fs::write(&self.markdown_path, markdown)?;
        self.append_manifest_record(evidence)?;

        Ok(())
    }

    fn append_manifest_record(
        &self,
        evidence: &ScatterSelectionEvidence,
    ) -> Result<(), Box<dyn Error>> {
        let manifest_record = ScatterSelectionManifestRecord {
            json_path: &self.json_path,
            markdown_path: &self.markdown_path,
            selected_row_count: evidence.selected_row_count,
            selected_percentage: evidence.selected_percentage,
            point_preset_row_count: evidence.point_preset_row_count,
            export_timestamp_unix_ms: self.export_timestamp_unix_ms,
            export_counter: self.export_counter,
        };
        let manifest_line = manifest_record.to_json_line();
        let mut manifest = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.manifest_path)?;
        writeln!(manifest, "{manifest_line}")?;

        Ok(())
    }
}

struct ScatterSelectionManifestRecord<'a> {
    json_path: &'a Path,
    markdown_path: &'a Path,
    selected_row_count: usize,
    selected_percentage: f32,
    point_preset_row_count: usize,
    export_timestamp_unix_ms: u128,
    export_counter: u64,
}

impl ScatterSelectionManifestRecord<'_> {
    fn to_json_line(&self) -> String {
        let json_path = json_escape(&self.json_path.display().to_string());
        let markdown_path = json_escape(&self.markdown_path.display().to_string());
        format!(
            "{{\"artifact_kind\":\"{}\",\"schema_version\":{},\"json_path\":\"{}\",\"markdown_path\":\"{}\",\"selected_row_count\":{},\"selected_percentage\":{},\"point_preset_row_count\":{},\"export_timestamp_unix_ms\":{},\"export_counter\":{}}}",
            SCATTER_SELECTION_EVIDENCE_ARTIFACT_KIND,
            SCATTER_SELECTION_EVIDENCE_SCHEMA_VERSION,
            json_path,
            markdown_path,
            self.selected_row_count,
            self.selected_percentage,
            self.point_preset_row_count,
            self.export_timestamp_unix_ms,
            self.export_counter,
        )
    }
}

fn current_unix_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        json_escape, ScatterSelectionExportPaths, ScatterSelectionManifestRecord,
        EXPORT_MANIFEST_FILE_NAME,
    };

    #[test]
    fn export_paths_include_timestamp_and_counter() {
        let paths = ScatterSelectionExportPaths::new("target/rawscope-test-exports", 1234, 7);

        assert!(paths
            .json_path
            .ends_with(Path::new("scatter-selection-1234-7.json")));
        assert!(paths
            .markdown_path
            .ends_with(Path::new("scatter-selection-1234-7.md")));
        assert!(paths
            .manifest_path
            .ends_with(Path::new(EXPORT_MANIFEST_FILE_NAME)));
    }

    #[test]
    fn export_paths_skip_existing_collision() {
        let test_dir = unique_test_dir("collision");
        fs::create_dir_all(&test_dir).unwrap();
        let collided_path = test_dir.join("scatter-selection-1234-1.json");
        fs::write(&collided_path, "{}").unwrap();

        let paths = ScatterSelectionExportPaths::next_available(&test_dir, 1234, 1);

        assert!(paths
            .json_path
            .ends_with(Path::new("scatter-selection-1234-2.json")));
        fs::remove_dir_all(&test_dir).unwrap();
    }

    #[test]
    fn manifest_record_is_valid_json_line_with_expected_fields() {
        let record = ScatterSelectionManifestRecord {
            json_path: Path::new("target/rawscope-exports/scatter-selection-1234-1.json"),
            markdown_path: Path::new("target/rawscope-exports/scatter-selection-1234-1.md"),
            selected_row_count: 42,
            selected_percentage: 2.5,
            point_preset_row_count: 20_000,
            export_timestamp_unix_ms: 1234,
            export_counter: 1,
        };

        let line = record.to_json_line();
        assert!(line.starts_with('{'));
        assert!(line.ends_with('}'));
        assert!(line.contains("\"artifact_kind\":\"scatter-selection-evidence\""));
        assert!(line.contains("\"schema_version\":1"));
        assert!(line.contains("\"selected_row_count\":42"));
        assert!(line.contains("\"selected_percentage\":2.5"));
        assert!(line.contains("\"point_preset_row_count\":20000"));
        assert!(line.contains("\"export_timestamp_unix_ms\":1234"));
        assert!(line.contains("\"export_counter\":1"));
        assert!(line.contains("scatter-selection-1234-1.json"));
    }

    #[test]
    fn json_escape_escapes_path_sensitive_characters() {
        assert_eq!(json_escape("a\\b\"c\n"), "a\\\\b\\\"c\\n");
    }

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        let mut test_dir = std::env::temp_dir();
        test_dir.push(format!("rawscope-{name}-{}", std::process::id()));
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }
        test_dir
    }
}
