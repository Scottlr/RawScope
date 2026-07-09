//! Local report-bundle paths, placeholder visual context, and manifest helpers.

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rawscope_data::DatasetSource;
use rawscope_render::{
    scatter_selection_evidence_v2_json, scatter_selection_evidence_v2_markdown,
    timeline_selection_evidence_v2_json, timeline_selection_evidence_v2_markdown,
    ScatterSelectionEvidenceV2, TimelineSelectionEvidenceV2,
    SCATTER_SELECTION_EVIDENCE_V2_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
    TIMELINE_SELECTION_EVIDENCE_V2_ARTIFACT_KIND, TIMELINE_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
};
use serde::Serialize;

pub(crate) const EXPORT_DIR: &str = "target/rawscope-exports";
pub(crate) const SCATTER_REPORT_BUNDLE_DIR_PREFIX: &str = "report-scatter";
pub(crate) const TIMELINE_REPORT_BUNDLE_DIR_PREFIX: &str = "report-timeline";

const BUNDLE_MANIFEST_FILE_NAME: &str = "manifest.json";
const EVIDENCE_JSON_FILE_NAME: &str = "evidence.json";
const EVIDENCE_MARKDOWN_FILE_NAME: &str = "evidence.md";
const VISUAL_CONTEXT_FILE_NAME: &str = "visual-context.txt";
const EVIDENCE_REPORT_BUNDLE_SCHEMA_VERSION: u32 = 1;
const SCATTER_REPORT_BUNDLE_ARTIFACT_KIND: &str = "scatter-evidence-report-bundle";
const TIMELINE_REPORT_BUNDLE_ARTIFACT_KIND: &str = "timeline-evidence-report-bundle";
const VISUAL_CONTEXT_KIND_PLACEHOLDER_TEXT: &str = "text-placeholder";
const VISUAL_CAPTURE_STATUS_DEFERRED: &str = "deferred";
const VISUAL_CAPTURE_DEFERRED_REASON: &str =
    "image capture is deferred until WGPU readback ownership and a narrow PNG dependency are approved";

pub(crate) struct EvidenceReportBundlePaths {
    pub(crate) bundle_dir: PathBuf,
    pub(crate) evidence_json_path: PathBuf,
    pub(crate) evidence_markdown_path: PathBuf,
    pub(crate) visual_context_path: PathBuf,
    pub(crate) manifest_path: PathBuf,
    pub(crate) export_timestamp_unix_ms: u128,
    pub(crate) export_counter: u64,
}

impl EvidenceReportBundlePaths {
    pub(crate) fn next_available(
        output_dir: impl AsRef<Path>,
        bundle_dir_prefix: &str,
        export_timestamp_unix_ms: u128,
        initial_export_counter: u64,
    ) -> Self {
        let output_dir = output_dir.as_ref();
        let mut export_counter = initial_export_counter;
        loop {
            let paths = Self::new(
                output_dir,
                bundle_dir_prefix,
                export_timestamp_unix_ms,
                export_counter,
            );
            if !paths.bundle_dir.exists() {
                return paths;
            }
            export_counter += 1;
        }
    }

    fn new(
        output_dir: impl AsRef<Path>,
        bundle_dir_prefix: &str,
        export_timestamp_unix_ms: u128,
        export_counter: u64,
    ) -> Self {
        let output_dir = output_dir.as_ref();
        let bundle_dir_name =
            format!("{bundle_dir_prefix}-{export_timestamp_unix_ms}-{export_counter}");
        let bundle_dir = output_dir.join(bundle_dir_name);
        let evidence_json_path = bundle_dir.join(EVIDENCE_JSON_FILE_NAME);
        let evidence_markdown_path = bundle_dir.join(EVIDENCE_MARKDOWN_FILE_NAME);
        let visual_context_path = bundle_dir.join(VISUAL_CONTEXT_FILE_NAME);
        let manifest_path = bundle_dir.join(BUNDLE_MANIFEST_FILE_NAME);

        Self {
            bundle_dir,
            evidence_json_path,
            evidence_markdown_path,
            visual_context_path,
            manifest_path,
            export_timestamp_unix_ms,
            export_counter,
        }
    }

    pub(crate) fn write_scatter(
        &self,
        evidence: &ScatterSelectionEvidenceV2,
    ) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(&self.bundle_dir)?;
        fs::write(
            &self.evidence_json_path,
            scatter_selection_evidence_v2_json(evidence)?,
        )?;
        fs::write(
            &self.evidence_markdown_path,
            scatter_selection_evidence_v2_markdown(evidence),
        )?;
        fs::write(
            &self.visual_context_path,
            scatter_visual_context_placeholder(evidence),
        )?;

        let manifest_record = ScatterEvidenceReportBundleManifestRecord {
            artifact_kind: SCATTER_REPORT_BUNDLE_ARTIFACT_KIND,
            bundle_schema_version: EVIDENCE_REPORT_BUNDLE_SCHEMA_VERSION,
            evidence_artifact_kind: SCATTER_SELECTION_EVIDENCE_V2_ARTIFACT_KIND,
            evidence_schema_version: SCATTER_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
            bundle_dir: &self.bundle_dir,
            evidence_json_path: &self.evidence_json_path,
            evidence_markdown_path: &self.evidence_markdown_path,
            visual_context_path: &self.visual_context_path,
            visual_context_kind: VISUAL_CONTEXT_KIND_PLACEHOLDER_TEXT,
            visual_capture_status: VISUAL_CAPTURE_STATUS_DEFERRED,
            selected_row_count: evidence.selected_row_count,
            selected_percentage: evidence.selected_percentage,
            dataset_row_count: evidence.dataset_identity.row_count,
            export_timestamp_unix_ms: self.export_timestamp_unix_ms,
            export_counter: self.export_counter,
        };
        self.write_manifest_record(&manifest_record)?;

        Ok(())
    }

    pub(crate) fn write_timeline(
        &self,
        evidence: &TimelineSelectionEvidenceV2,
    ) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(&self.bundle_dir)?;
        fs::write(
            &self.evidence_json_path,
            timeline_selection_evidence_v2_json(evidence)?,
        )?;
        fs::write(
            &self.evidence_markdown_path,
            timeline_selection_evidence_v2_markdown(evidence),
        )?;
        fs::write(
            &self.visual_context_path,
            timeline_visual_context_placeholder(evidence),
        )?;

        let manifest_record = TimelineEvidenceReportBundleManifestRecord {
            artifact_kind: TIMELINE_REPORT_BUNDLE_ARTIFACT_KIND,
            bundle_schema_version: EVIDENCE_REPORT_BUNDLE_SCHEMA_VERSION,
            evidence_artifact_kind: TIMELINE_SELECTION_EVIDENCE_V2_ARTIFACT_KIND,
            evidence_schema_version: TIMELINE_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
            bundle_dir: &self.bundle_dir,
            evidence_json_path: &self.evidence_json_path,
            evidence_markdown_path: &self.evidence_markdown_path,
            visual_context_path: &self.visual_context_path,
            visual_context_kind: VISUAL_CONTEXT_KIND_PLACEHOLDER_TEXT,
            visual_capture_status: VISUAL_CAPTURE_STATUS_DEFERRED,
            selected_event_count: evidence.selected_event_count,
            selected_percentage: evidence.selected_percentage,
            dataset_row_count: evidence.dataset_identity.row_count,
            export_timestamp_unix_ms: self.export_timestamp_unix_ms,
            export_counter: self.export_counter,
        };
        self.write_manifest_record(&manifest_record)?;

        Ok(())
    }

    fn write_manifest_record(
        &self,
        manifest_record: &impl Serialize,
    ) -> Result<(), Box<dyn Error>> {
        let manifest_json = serde_json::to_string_pretty(manifest_record)?;
        fs::write(&self.manifest_path, manifest_json)?;
        Ok(())
    }
}

#[derive(Serialize)]
struct ScatterEvidenceReportBundleManifestRecord<'a> {
    artifact_kind: &'static str,
    bundle_schema_version: u32,
    evidence_artifact_kind: &'static str,
    evidence_schema_version: u32,
    bundle_dir: &'a Path,
    evidence_json_path: &'a Path,
    evidence_markdown_path: &'a Path,
    visual_context_path: &'a Path,
    visual_context_kind: &'static str,
    visual_capture_status: &'static str,
    selected_row_count: usize,
    selected_percentage: f32,
    dataset_row_count: usize,
    export_timestamp_unix_ms: u128,
    export_counter: u64,
}

#[derive(Serialize)]
struct TimelineEvidenceReportBundleManifestRecord<'a> {
    artifact_kind: &'static str,
    bundle_schema_version: u32,
    evidence_artifact_kind: &'static str,
    evidence_schema_version: u32,
    bundle_dir: &'a Path,
    evidence_json_path: &'a Path,
    evidence_markdown_path: &'a Path,
    visual_context_path: &'a Path,
    visual_context_kind: &'static str,
    visual_capture_status: &'static str,
    selected_event_count: usize,
    selected_percentage: f32,
    dataset_row_count: usize,
    export_timestamp_unix_ms: u128,
    export_counter: u64,
}

fn scatter_visual_context_placeholder(evidence: &ScatterSelectionEvidenceV2) -> String {
    format!(
        "RawScope visual context placeholder\n\
view_kind: scatter\n\
dataset_source: {}\n\
dataset_rows: {}\n\
grid_size: {}x{}\n\
view_x_range: {:.6}..{:.6}\n\
view_y_range: {:.6}..{:.6}\n\
brush_x_range: {:.6}..{:.6}\n\
brush_y_range: {:.6}..{:.6}\n\
selected_row_count: {}\n\
selected_percentage: {:.6}\n\
capture_status: {}\n\
capture_reason: {}\n",
        dataset_source_label(&evidence.dataset_identity.source),
        evidence.dataset_identity.row_count,
        evidence.view.grid_width,
        evidence.view.grid_height,
        evidence.view.x_range.min,
        evidence.view.x_range.max,
        evidence.view.y_range.min,
        evidence.view.y_range.max,
        evidence.brush_x_range.min,
        evidence.brush_x_range.max,
        evidence.brush_y_range.min,
        evidence.brush_y_range.max,
        evidence.selected_row_count,
        evidence.selected_percentage,
        VISUAL_CAPTURE_STATUS_DEFERRED,
        VISUAL_CAPTURE_DEFERRED_REASON,
    )
}

fn timeline_visual_context_placeholder(evidence: &TimelineSelectionEvidenceV2) -> String {
    format!(
        "RawScope visual context placeholder\n\
view_kind: timeline\n\
dataset_source: {}\n\
dataset_rows: {}\n\
grid_size: {}x{}\n\
view_time_range: {}..{}\n\
full_time_range: {}..{}\n\
lane_count: {}\n\
selected_time_range: {}..{}\n\
selected_lane_range: {}..{}\n\
selected_event_count: {}\n\
selected_percentage: {:.6}\n\
capture_status: {}\n\
capture_reason: {}\n",
        dataset_source_label(&evidence.dataset_identity.source),
        evidence.dataset_identity.row_count,
        evidence.view.grid_width,
        evidence.view.grid_height,
        evidence.view.time_range.min,
        evidence.view.time_range.max,
        evidence.view.full_time_range.min,
        evidence.view.full_time_range.max,
        evidence.view.lane_count,
        evidence.selected_time_range.min,
        evidence.selected_time_range.max,
        evidence.selected_lane_range.start,
        evidence.selected_lane_range.end_exclusive,
        evidence.selected_event_count,
        evidence.selected_percentage,
        VISUAL_CAPTURE_STATUS_DEFERRED,
        VISUAL_CAPTURE_DEFERRED_REASON,
    )
}

fn dataset_source_label(source: &DatasetSource) -> String {
    match source {
        DatasetSource::Synthetic { seed, generator } => {
            format!("synthetic:{generator}:seed={seed}")
        }
        DatasetSource::LocalCsv { path, limit } => format!(
            "local_csv:{}{}",
            path.display(),
            limit
                .map(|value| format!(":limit={value}"))
                .unwrap_or_default()
        ),
    }
}

pub(crate) fn current_unix_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
