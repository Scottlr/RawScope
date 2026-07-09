//! Benchmark-facing helpers for workbench report-bundle writes.

use std::{error::Error, path::Path};

use rawscope_render::{ScatterSelectionEvidenceV2, TimelineSelectionEvidenceV2};

use crate::app_report_bundle::{
    EvidenceReportBundlePaths, SCATTER_REPORT_BUNDLE_DIR_PREFIX, TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
};

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
