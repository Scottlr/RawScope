//! Local report-bundle paths, placeholder visual context, and manifest helpers.

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rawscope_data::{DatasetProfileId, DatasetSource};
use rawscope_evidence::{
    scatter_selection_evidence_v2_json, scatter_selection_evidence_v2_markdown,
    ScatterSelectionEvidenceV2, SCATTER_SELECTION_EVIDENCE_V2_ARTIFACT_KIND,
    SCATTER_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
};
use rawscope_render::{
    scatter_selection_evidence_v3_json, scatter_selection_evidence_v3_markdown,
    timeline_selection_evidence_v2_json, timeline_selection_evidence_v2_markdown,
    timeline_selection_evidence_v3_json, timeline_selection_evidence_v3_markdown,
    ScatterSelectionEvidenceV3, TimelineSelectionEvidenceV2, TimelineSelectionEvidenceV3,
    SCATTER_SELECTION_EVIDENCE_V3_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
    TIMELINE_SELECTION_EVIDENCE_V2_ARTIFACT_KIND, TIMELINE_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
    TIMELINE_SELECTION_EVIDENCE_V3_ARTIFACT_KIND, TIMELINE_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
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
const VISUAL_CONTEXT_KIND_SUMMARY_TEXT: &str = "text-visual-context";
const VISUAL_CAPTURE_STATUS_DEFERRED: &str = "deferred";
const VISUAL_CAPTURE_DEFERRED_REASON: &str =
    "image capture is deferred until WGPU readback ownership and a narrow PNG dependency are approved";
const SCATTER_COMPARISON_BASELINE_LABEL: &str = "active_point_slice";
const TIMELINE_COMPARISON_BASELINE_LABEL: &str = "active_event_slice";

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
        while output_dir
            .join(format!(
                "{bundle_dir_prefix}-{export_timestamp_unix_ms}-{export_counter}"
            ))
            .exists()
        {
            export_counter = export_counter.saturating_add(1);
        }
        Self::new(
            output_dir,
            bundle_dir_prefix,
            export_timestamp_unix_ms,
            export_counter,
        )
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
        self.transactional(|staged| staged.write_scatter_unstaged(evidence))
    }

    fn write_scatter_unstaged(
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
            active_dataset_profile: None,
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
        self.transactional(|staged| staged.write_timeline_unstaged(evidence))
    }

    fn write_timeline_unstaged(
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
            active_dataset_profile: None,
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

    pub(crate) fn write_scatter_v3(
        &self,
        evidence: &ScatterSelectionEvidenceV3,
    ) -> Result<(), Box<dyn Error>> {
        self.transactional(|staged| staged.write_scatter_v3_unstaged(evidence))
    }

    fn write_scatter_v3_unstaged(
        &self,
        evidence: &ScatterSelectionEvidenceV3,
    ) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(&self.bundle_dir)?;
        fs::write(
            &self.evidence_json_path,
            scatter_selection_evidence_v3_json(evidence)?,
        )?;
        fs::write(
            &self.evidence_markdown_path,
            scatter_selection_evidence_v3_markdown(evidence),
        )?;
        fs::write(
            &self.visual_context_path,
            scatter_visual_context_v3(evidence),
        )?;

        let manifest_record = ScatterEvidenceReportBundleManifestRecord {
            artifact_kind: SCATTER_REPORT_BUNDLE_ARTIFACT_KIND,
            bundle_schema_version: EVIDENCE_REPORT_BUNDLE_SCHEMA_VERSION,
            evidence_artifact_kind: SCATTER_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
            evidence_schema_version: SCATTER_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
            active_dataset_profile: evidence
                .active_dataset_profile
                .map(DatasetProfileId::as_str),
            bundle_dir: &self.bundle_dir,
            evidence_json_path: &self.evidence_json_path,
            evidence_markdown_path: &self.evidence_markdown_path,
            visual_context_path: &self.visual_context_path,
            visual_context_kind: VISUAL_CONTEXT_KIND_SUMMARY_TEXT,
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

    pub(crate) fn write_timeline_v3(
        &self,
        evidence: &TimelineSelectionEvidenceV3,
    ) -> Result<(), Box<dyn Error>> {
        self.transactional(|staged| staged.write_timeline_v3_unstaged(evidence))
    }

    fn write_timeline_v3_unstaged(
        &self,
        evidence: &TimelineSelectionEvidenceV3,
    ) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(&self.bundle_dir)?;
        fs::write(
            &self.evidence_json_path,
            timeline_selection_evidence_v3_json(evidence)?,
        )?;
        fs::write(
            &self.evidence_markdown_path,
            timeline_selection_evidence_v3_markdown(evidence),
        )?;
        fs::write(
            &self.visual_context_path,
            timeline_visual_context_v3(evidence),
        )?;

        let manifest_record = TimelineEvidenceReportBundleManifestRecord {
            artifact_kind: TIMELINE_REPORT_BUNDLE_ARTIFACT_KIND,
            bundle_schema_version: EVIDENCE_REPORT_BUNDLE_SCHEMA_VERSION,
            evidence_artifact_kind: TIMELINE_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
            evidence_schema_version: TIMELINE_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
            active_dataset_profile: evidence
                .active_dataset_profile
                .map(DatasetProfileId::as_str),
            bundle_dir: &self.bundle_dir,
            evidence_json_path: &self.evidence_json_path,
            evidence_markdown_path: &self.evidence_markdown_path,
            visual_context_path: &self.visual_context_path,
            visual_context_kind: VISUAL_CONTEXT_KIND_SUMMARY_TEXT,
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

    fn transactional(
        &self,
        write: impl FnOnce(&EvidenceReportBundlePaths) -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(self.bundle_dir.parent().unwrap_or_else(|| Path::new(".")))?;
        let temp_dir = self
            .bundle_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!(
                ".{}.tmp-{}",
                self.bundle_dir.file_name().unwrap().to_string_lossy(),
                std::process::id()
            ));
        fs::create_dir(&temp_dir)?;
        let staged = Self {
            bundle_dir: temp_dir.clone(),
            evidence_json_path: temp_dir.join(EVIDENCE_JSON_FILE_NAME),
            evidence_markdown_path: temp_dir.join(EVIDENCE_MARKDOWN_FILE_NAME),
            visual_context_path: temp_dir.join(VISUAL_CONTEXT_FILE_NAME),
            manifest_path: temp_dir.join(BUNDLE_MANIFEST_FILE_NAME),
            export_timestamp_unix_ms: self.export_timestamp_unix_ms,
            export_counter: self.export_counter,
        };
        let result = write(&staged)
            .and_then(|_| fs::rename(&temp_dir, &self.bundle_dir).map_err(|source| source.into()));
        if result.is_err() {
            let _ = fs::remove_dir_all(&temp_dir);
        }
        result
    }
}

#[derive(Serialize)]
struct ScatterEvidenceReportBundleManifestRecord<'a> {
    artifact_kind: &'static str,
    bundle_schema_version: u32,
    evidence_artifact_kind: &'static str,
    evidence_schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_dataset_profile: Option<&'static str>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    active_dataset_profile: Option<&'static str>,
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

fn scatter_visual_context_v3(evidence: &ScatterSelectionEvidenceV3) -> String {
    let dataset_profile = dataset_profile_context_line(evidence.active_dataset_profile);
    format!(
        "RawScope visual context\n\
view_kind: scatter\n\
dataset_source: {}\n\
dataset_rows: {}\n\
{}\
grid_size: {}x{}\n\
view_x_range: {:.6}..{:.6}\n\
view_y_range: {:.6}..{:.6}\n\
density_transform: {}\n\
density_palette: {}\n\
density_normalization: {}\n\
density_presentation: {}\n\
selected_row_count: {}\n\
selected_percentage: {:.6}\n\
comparison_baseline: {}\n\
comparison_baseline_row_count: {}\n\
aggregate_context_bin_count: {}\n\
aggregate_context_bin_limit: {}\n\
capture_status: {}\n\
capture_reason: {}\n",
        dataset_source_label(&evidence.dataset_identity.source),
        evidence.dataset_identity.row_count,
        dataset_profile,
        evidence.view.grid_width,
        evidence.view.grid_height,
        evidence.view.x_range.min,
        evidence.view.x_range.max,
        evidence.view.y_range.min,
        evidence.view.y_range.max,
        evidence.view.density_encoding.transform.label(),
        evidence.view.density_encoding.palette.label(),
        evidence.view.density_encoding.normalization.label(),
        evidence.view.density_presentation.evidence_label(),
        evidence.selected_row_count,
        evidence.selected_percentage,
        SCATTER_COMPARISON_BASELINE_LABEL,
        evidence.comparison.baseline_row_count,
        evidence.aggregate_context.bins.len(),
        evidence.aggregate_context.bin_limit,
        VISUAL_CAPTURE_STATUS_DEFERRED,
        VISUAL_CAPTURE_DEFERRED_REASON,
    )
}

fn timeline_visual_context_v3(evidence: &TimelineSelectionEvidenceV3) -> String {
    let dataset_profile = dataset_profile_context_line(evidence.active_dataset_profile);
    format!(
        "RawScope visual context\n\
view_kind: timeline\n\
dataset_source: {}\n\
dataset_rows: {}\n\
{}\
grid_size: {}x{}\n\
view_time_range: {}..{}\n\
full_time_range: {}..{}\n\
lane_count: {}\n\
density_transform: {}\n\
density_palette: {}\n\
density_normalization: {}\n\
selected_time_range: {}..{}\n\
selected_lane_range: {}..{}\n\
selected_event_count: {}\n\
selected_percentage: {:.6}\n\
comparison_baseline: {}\n\
comparison_baseline_event_count: {}\n\
aggregate_context_bin_count: {}\n\
aggregate_context_bin_limit: {}\n\
capture_status: {}\n\
capture_reason: {}\n",
        dataset_source_label(&evidence.dataset_identity.source),
        evidence.dataset_identity.row_count,
        dataset_profile,
        evidence.view.grid_width,
        evidence.view.grid_height,
        evidence.view.time_range.min,
        evidence.view.time_range.max,
        evidence.view.full_time_range.min,
        evidence.view.full_time_range.max,
        evidence.view.lane_count,
        evidence.view.density_encoding.transform.label(),
        evidence.view.density_encoding.palette.label(),
        evidence.view.density_encoding.normalization.label(),
        evidence.selected_time_range.min,
        evidence.selected_time_range.max,
        evidence.selected_lane_range.start,
        evidence.selected_lane_range.end_exclusive,
        evidence.selected_event_count,
        evidence.selected_percentage,
        TIMELINE_COMPARISON_BASELINE_LABEL,
        evidence.comparison.baseline_event_count,
        evidence.aggregate_context.bins.len(),
        evidence.aggregate_context.bin_limit,
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
            portable_dataset_label(path),
            limit
                .map(|value| format!(":limit={value}"))
                .unwrap_or_default()
        ),
        DatasetSource::LocalParquet { path, limit } => format!(
            "local_parquet:{}{}",
            portable_dataset_label(path),
            limit
                .map(|value| format!(":limit={value}"))
                .unwrap_or_default()
        ),
    }
}

fn portable_dataset_label(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("dataset")
        .to_string()
}

fn dataset_profile_context_line(profile_id: Option<DatasetProfileId>) -> String {
    profile_id
        .map(|profile_id| format!("dataset_profile: {}\n", profile_id.as_str()))
        .unwrap_or_default()
}

pub(crate) fn current_unix_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
