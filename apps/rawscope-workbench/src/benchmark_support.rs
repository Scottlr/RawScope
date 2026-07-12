//! Benchmark-facing helpers for workbench report-bundle writes.

use std::{
    error::Error,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use rawscope_evidence::{ScatterSelectionEvidenceV2, TimelineSelectionEvidenceV2};

use crate::app_report_bundle::{
    EvidenceReportBundlePaths, SCATTER_REPORT_BUNDLE_DIR_PREFIX, TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
};

pub const BENCHMARK_METADATA_SCHEMA_VERSION: u32 = 1;

/// Reproducibility context attached to a product benchmark scenario.
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkRunMetadata {
    pub schema_version: u32,
    pub scenario_id: String,
    pub fixture_version: String,
    pub command: String,
    pub dataset_rows: u64,
    pub feature_set: String,
    pub source_revision: String,
    pub dirty_worktree: bool,
    pub operating_system: String,
    pub architecture: String,
    pub rust_toolchain: String,
    pub date_unix_seconds: u64,
    pub sample_count: u64,
    pub metric_unit: String,
}

impl BenchmarkRunMetadata {
    pub fn for_scenario(
        scenario_id: impl Into<String>,
        fixture_version: impl Into<String>,
        dataset_rows: u64,
        feature_set: impl Into<String>,
        sample_count: u64,
        metric_unit: impl Into<String>,
    ) -> Self {
        let date_unix_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        Self {
            schema_version: BENCHMARK_METADATA_SCHEMA_VERSION,
            scenario_id: scenario_id.into(),
            fixture_version: fixture_version.into(),
            command: std::env::args().collect::<Vec<_>>().join(" "),
            dataset_rows,
            feature_set: feature_set.into(),
            source_revision: option_env!("RAWSCOPE_GIT_REVISION")
                .unwrap_or("unknown")
                .to_string(),
            dirty_worktree: option_env!("RAWSCOPE_GIT_DIRTY")
                .is_some_and(|value| value.eq_ignore_ascii_case("true")),
            operating_system: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            rust_toolchain: option_env!("RAWSCOPE_RUST_TOOLCHAIN")
                .unwrap_or("unknown")
                .to_string(),
            date_unix_seconds,
            sample_count,
            metric_unit: metric_unit.into(),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != BENCHMARK_METADATA_SCHEMA_VERSION {
            return Err("unsupported benchmark metadata schema version");
        }
        if self.scenario_id.trim().is_empty() {
            return Err("benchmark scenario id must not be empty");
        }
        if self.fixture_version.trim().is_empty() {
            return Err("benchmark fixture version must not be empty");
        }
        if self.command.trim().is_empty() {
            return Err("benchmark command must not be empty");
        }
        if self.feature_set.trim().is_empty() {
            return Err("benchmark feature set must not be empty");
        }
        if self.metric_unit.trim().is_empty() {
            return Err("benchmark metric unit must not be empty");
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Writes one scatter report bundle into `output_dir` using the workbench bundle owner.
pub fn write_scatter_report_bundle(
    output_dir: impl AsRef<Path>,
    evidence: &ScatterSelectionEvidenceV2,
    export_timestamp_unix_ms: u128,
    export_counter: u64,
) -> Result<(), Box<dyn Error>> {
    let bundle_paths = EvidenceReportBundlePaths::next_available(
        output_dir,
        SCATTER_REPORT_BUNDLE_DIR_PREFIX,
        export_timestamp_unix_ms,
        export_counter,
    );
    bundle_paths.write_scatter(evidence)
}

/// Writes one timeline report bundle into `output_dir` using the workbench bundle owner.
pub fn write_timeline_report_bundle(
    output_dir: impl AsRef<Path>,
    evidence: &TimelineSelectionEvidenceV2,
    export_timestamp_unix_ms: u128,
    export_counter: u64,
) -> Result<(), Box<dyn Error>> {
    let bundle_paths = EvidenceReportBundlePaths::next_available(
        output_dir,
        TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
        export_timestamp_unix_ms,
        export_counter,
    );
    bundle_paths.write_timeline(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_metadata_requires_reproducibility_fields() {
        let metadata = BenchmarkRunMetadata::for_scenario(
            "selection-v1",
            "fixture-v1",
            128,
            "workspace-default",
            10,
            "rows_per_second",
        );
        assert!(metadata.validate().is_ok());
        assert!(metadata.to_json().unwrap().contains("selection-v1"));
    }

    #[test]
    fn benchmark_metadata_rejects_empty_metric_units() {
        let mut metadata = BenchmarkRunMetadata::for_scenario(
            "selection-v1",
            "fixture-v1",
            128,
            "workspace-default",
            10,
            "rows_per_second",
        );
        metadata.metric_unit.clear();
        assert_eq!(
            metadata.validate(),
            Err("benchmark metric unit must not be empty")
        );
    }
}
