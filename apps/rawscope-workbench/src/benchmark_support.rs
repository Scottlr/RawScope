//! Benchmark-facing helpers for workbench report-bundle writes.

use std::{
    error::Error,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use rawscope_evidence::{
    ScatterSelectionEvidenceV2, TimelineSelectionEvidenceV2, VisualFieldEvidenceV1,
};

use crate::app_report_bundle::{
    EvidenceReportBundlePaths, SCATTER_REPORT_BUNDLE_DIR_PREFIX, TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
    VISUAL_FIELD_REPORT_BUNDLE_DIR_PREFIX,
};

pub const BENCHMARK_METADATA_SCHEMA_VERSION: u32 = 1;
pub const BENCHMARK_UNKNOWN_VALUE: &str = "unknown";

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
    pub gpu_adapter: String,
    pub gpu_backend: String,
    pub gpu_driver: String,
    pub grid_width: u32,
    pub grid_height: u32,
    pub quality_tier: String,
    pub visible_category_layers: u32,
    pub active_bytes: Option<u64>,
    pub pending_bytes: Option<u64>,
    pub retiring_bytes: Option<u64>,
    pub derived_resources: Vec<String>,
    /// Filled by a benchmark result collector when quantiles are persisted;
    /// `None` prevents metadata from implying a measured value was available.
    pub pointer_submit_p50_ns: Option<u64>,
    pub pointer_submit_p95_ns: Option<u64>,
    pub settle_p50_ns: Option<u64>,
    pub settle_p95_ns: Option<u64>,
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
            gpu_adapter: BENCHMARK_UNKNOWN_VALUE.to_string(),
            gpu_backend: BENCHMARK_UNKNOWN_VALUE.to_string(),
            gpu_driver: BENCHMARK_UNKNOWN_VALUE.to_string(),
            grid_width: 0,
            grid_height: 0,
            quality_tier: BENCHMARK_UNKNOWN_VALUE.to_string(),
            visible_category_layers: 0,
            active_bytes: None,
            pending_bytes: None,
            retiring_bytes: None,
            derived_resources: Vec::new(),
            pointer_submit_p50_ns: None,
            pointer_submit_p95_ns: None,
            settle_p50_ns: None,
            settle_p95_ns: None,
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
        let only_one_grid_dimension_known = (self.grid_width == 0) ^ (self.grid_height == 0);
        if only_one_grid_dimension_known {
            return Err("benchmark grid dimensions must be both present or both unknown");
        }
        if let (Some(p50), Some(p95)) = (self.pointer_submit_p50_ns, self.pointer_submit_p95_ns) {
            if p50 > p95 {
                return Err("pointer benchmark p50 must not exceed p95");
            }
        }
        if let (Some(p50), Some(p95)) = (self.settle_p50_ns, self.settle_p95_ns) {
            if p50 > p95 {
                return Err("settle benchmark p50 must not exceed p95");
            }
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

/// Writes the canonical generic visual-field evidence artifact through the
/// same transactional report owner used by the workbench.
pub fn write_visual_field_report_bundle(
    output_dir: impl AsRef<Path>,
    evidence: &VisualFieldEvidenceV1,
    export_timestamp_unix_ms: u128,
    export_counter: u64,
) -> Result<(), Box<dyn Error>> {
    let bundle_paths = EvidenceReportBundlePaths::next_available(
        output_dir,
        VISUAL_FIELD_REPORT_BUNDLE_DIR_PREFIX,
        export_timestamp_unix_ms,
        export_counter,
    );
    bundle_paths.write_visual_field(evidence)
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

    #[test]
    fn benchmark_metadata_rejects_inconsistent_quantiles() {
        let mut metadata = BenchmarkRunMetadata::for_scenario(
            "interaction-v1",
            "fixture-v1",
            128,
            "workspace-default",
            10,
            "nanoseconds",
        );
        metadata.pointer_submit_p50_ns = Some(20);
        metadata.pointer_submit_p95_ns = Some(10);
        assert_eq!(
            metadata.validate(),
            Err("pointer benchmark p50 must not exceed p95")
        );
    }
}
