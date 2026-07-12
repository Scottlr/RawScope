//! Scatter evidence v5 report writing and local session visual context.

use std::{error::Error, fs};

use rawscope_render::{
    scatter_selection_evidence_v5_json, scatter_selection_evidence_v5_markdown,
    ScatterSelectionEvidenceV5, SCATTER_SELECTION_EVIDENCE_V5_ARTIFACT_KIND,
    SCATTER_SELECTION_EVIDENCE_V5_SCHEMA_VERSION,
};
use serde::Serialize;

use crate::app_report_bundle::EvidenceReportBundlePaths;

const SCATTER_REPORT_BUNDLE_ARTIFACT_KIND: &str = "scatter-evidence-report-bundle";
const VISUAL_CONTEXT_KIND: &str = "text-visual-query-context";

impl EvidenceReportBundlePaths {
    pub(crate) fn write_scatter_v5(
        &self,
        evidence: &ScatterSelectionEvidenceV5,
    ) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(&self.bundle_dir)?;
        fs::write(
            &self.evidence_json_path,
            scatter_selection_evidence_v5_json(evidence)?,
        )?;
        fs::write(
            &self.evidence_markdown_path,
            scatter_selection_evidence_v5_markdown(evidence)?,
        )?;
        fs::write(
            &self.visual_context_path,
            scatter_visual_context_v5(evidence),
        )?;
        let manifest = ScatterEvidenceV5BundleManifest {
            artifact_kind: SCATTER_REPORT_BUNDLE_ARTIFACT_KIND,
            bundle_schema_version: 1,
            evidence_artifact_kind: SCATTER_SELECTION_EVIDENCE_V5_ARTIFACT_KIND,
            evidence_schema_version: SCATTER_SELECTION_EVIDENCE_V5_SCHEMA_VERSION,
            active_dataset_profile: evidence
                .active_dataset_profile
                .map(|profile| profile.as_str()),
            evidence_json_file: file_name(&self.evidence_json_path),
            evidence_markdown_file: file_name(&self.evidence_markdown_path),
            visual_context_file: file_name(&self.visual_context_path),
            visual_context_kind: VISUAL_CONTEXT_KIND,
            selected_row_count: evidence.selected_row_count,
            dataset_row_count: evidence.dataset_identity.row_count,
            included_row_count: evidence.cohort.included_row_count,
            export_timestamp_unix_ms: self.export_timestamp_unix_ms,
            export_counter: self.export_counter,
        };
        fs::write(
            &self.manifest_path,
            serde_json::to_string_pretty(&manifest)?,
        )?;
        Ok(())
    }
}

#[derive(Serialize)]
struct ScatterEvidenceV5BundleManifest<'a> {
    artifact_kind: &'static str,
    bundle_schema_version: u32,
    evidence_artifact_kind: &'static str,
    evidence_schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_dataset_profile: Option<&'static str>,
    evidence_json_file: &'a str,
    evidence_markdown_file: &'a str,
    visual_context_file: &'a str,
    visual_context_kind: &'static str,
    selected_row_count: usize,
    dataset_row_count: usize,
    included_row_count: usize,
    export_timestamp_unix_ms: u128,
    export_counter: u64,
}

fn scatter_visual_context_v5(evidence: &ScatterSelectionEvidenceV5) -> String {
    let query = &evidence.visual_query;
    let mut context = format!(
        "RawScope visual query context\n\
view_kind: scatter\n\
schema_version: {}\n\
projection: {}\n\
density_mode: {}\n\
density_presentation: {}\n\
density_transform: {}\n\
density_palette: {}\n\
density_normalization: {}\n\
grid_size: {}x{}\n\
active_filter_count: {}\n\
full_row_count: {}\n\
included_row_count: {}\n\
excluded_row_count: {}\n\
session_context: {}\n\
pinned_inspection: {}\n",
        evidence.schema_version,
        query.projection.label(),
        query.density_mode.label(),
        query.density_presentation.evidence_label(),
        query.density_encoding.transform.label(),
        query.density_encoding.palette.label(),
        query.density_encoding.normalization.label(),
        query.grid_width,
        query.grid_height,
        query.filters.len(),
        evidence.cohort.full_row_count,
        evidence.cohort.included_row_count,
        evidence.cohort.excluded_row_count,
        evidence.session_context.is_some(),
        evidence.pinned_inspection.is_some(),
    );
    if let Some(context_value) = &evidence.session_context {
        context.push_str(&format!(
            "session_artifact_kind: {}\nsession_schema_version: {}\ndata_format: {}\nevidence_key_column: {}\n",
            context_value.session_artifact_kind,
            context_value.session_schema_version,
            context_value.data_format.label(),
            context_value.evidence_key_column.as_deref().unwrap_or("not provided"),
        ));
    }
    if let Some(pin) = &evidence.pinned_inspection {
        context.push_str(&format!(
            "pinned_bin: ({}, {})\npinned_rows: {}\npinned_active_share: {:.6}\npinned_occupied_density_percentile: {}\npinned_neighborhood_rows: {}\npinned_neighborhood_share: {:.6}\npinned_row_id_sample_size: {}\npinned_sample_limit: {}\npinned_evidence_key_sample_size: {}\n",
            pin.bin_x,
            pin.bin_y,
            pin.row_count,
            pin.active_share,
            pin.occupied_density_percentile
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "not available".to_string()),
            pin.neighborhood_row_count,
            pin.neighborhood_share,
            pin.row_id_sample.len(),
            pin.sample_limit,
            pin.evidence_key_values.len(),
        ));
    }
    context
}

fn file_name(path: &std::path::Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .expect("report bundle paths use UTF-8 file names")
}
