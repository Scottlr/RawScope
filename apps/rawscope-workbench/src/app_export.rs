//! Evidence report-bundle export routing for the workbench density demos.

use tracing::{error, info, warn};

use rawscope_render::{
    scatter_aggregate_evidence_context, timeline_aggregate_evidence_context, ScatterEvidenceView,
    ScatterSelectionEvidenceV2, ScatterSelectionEvidenceV3, TimelineEvidenceView,
    TimelineSelectionEvidenceV2, TimelineSelectionEvidenceV3,
};

use crate::{
    app::WorkbenchApp,
    app_comparison::WorkbenchComparison,
    app_report_bundle::{
        current_unix_timestamp_ms, EvidenceReportBundlePaths, EXPORT_DIR,
        SCATTER_REPORT_BUNDLE_DIR_PREFIX, TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
    },
    ui::ExportStatus,
};

impl WorkbenchApp {
    pub(crate) fn export_selection_evidence(&mut self) {
        if self.scatter_filters.is_active() {
            self.export_status = ExportStatus::Unavailable {
                reason: crate::ui_filters::FILTERED_EXPORT_UNAVAILABLE_REASON.to_string(),
            };
            return;
        }
        let Some(evidence_v2) = self.scatter_selection_evidence_v2() else {
            self.export_status = ExportStatus::NoSelection;
            warn!(
                reason = "no finalized scatter selection evidence",
                "scatter selection evidence export skipped"
            );
            return;
        };
        let evidence_v3 = self.scatter_selection_evidence_v3(&evidence_v2);

        let export_counter = self.next_evidence_export_counter();
        let export_timestamp_unix_ms = current_unix_timestamp_ms();
        let export_paths = EvidenceReportBundlePaths::next_available(
            EXPORT_DIR,
            SCATTER_REPORT_BUNDLE_DIR_PREFIX,
            export_timestamp_unix_ms,
            export_counter,
        );
        let write_result = match evidence_v3.as_ref() {
            Some(evidence) => export_paths.write_scatter_v3(evidence),
            None => export_paths.write_scatter(&evidence_v2),
        };
        if let Err(err) = write_result {
            self.export_status = ExportStatus::Failed {
                message: err.to_string(),
            };
            error!(
                error = %err,
                bundle_dir = %export_paths.bundle_dir.display(),
                evidence_json_path = %export_paths.evidence_json_path.display(),
                evidence_markdown_path = %export_paths.evidence_markdown_path.display(),
                visual_context_path = %export_paths.visual_context_path.display(),
                manifest_path = %export_paths.manifest_path.display(),
                "failed to export scatter selection evidence report bundle"
            );
            return;
        }
        let (selected_row_count, evidence_schema_version) = evidence_v3
            .as_ref()
            .map(|evidence| (evidence.selected_row_count, evidence.schema_version))
            .unwrap_or((evidence_v2.selected_row_count, 2));
        self.evidence_export_counter = export_paths.export_counter;
        self.export_status = ExportStatus::Exported {
            bundle_dir: export_paths.bundle_dir.display().to_string(),
        };

        info!(
            evidence_schema_version,
            export_counter = export_paths.export_counter,
            bundle_dir = %export_paths.bundle_dir.display(),
            evidence_json_path = %export_paths.evidence_json_path.display(),
            evidence_markdown_path = %export_paths.evidence_markdown_path.display(),
            visual_context_path = %export_paths.visual_context_path.display(),
            manifest_path = %export_paths.manifest_path.display(),
            selected_row_count,
            "RawScope scatter selection evidence report bundle exported"
        );
    }

    pub(crate) fn export_timeline_selection_evidence(&mut self) {
        let Some(evidence_v2) = self.timeline_selection_evidence_v2() else {
            self.export_status = ExportStatus::NoSelection;
            warn!(
                reason = "no finalized timeline selection evidence",
                "timeline selection evidence export skipped"
            );
            return;
        };
        let evidence_v3 = self.timeline_selection_evidence_v3(&evidence_v2);

        let export_counter = self.next_evidence_export_counter();
        let export_timestamp_unix_ms = current_unix_timestamp_ms();
        let export_paths = EvidenceReportBundlePaths::next_available(
            EXPORT_DIR,
            TIMELINE_REPORT_BUNDLE_DIR_PREFIX,
            export_timestamp_unix_ms,
            export_counter,
        );
        let write_result = match evidence_v3.as_ref() {
            Some(evidence) => export_paths.write_timeline_v3(evidence),
            None => export_paths.write_timeline(&evidence_v2),
        };
        if let Err(err) = write_result {
            self.export_status = ExportStatus::Failed {
                message: err.to_string(),
            };
            error!(
                error = %err,
                bundle_dir = %export_paths.bundle_dir.display(),
                evidence_json_path = %export_paths.evidence_json_path.display(),
                evidence_markdown_path = %export_paths.evidence_markdown_path.display(),
                visual_context_path = %export_paths.visual_context_path.display(),
                manifest_path = %export_paths.manifest_path.display(),
                "failed to export timeline selection evidence report bundle"
            );
            return;
        }
        let (selected_event_count, evidence_schema_version) = evidence_v3
            .as_ref()
            .map(|evidence| (evidence.selected_event_count, evidence.schema_version))
            .unwrap_or((evidence_v2.selected_event_count, 2));
        self.evidence_export_counter = export_paths.export_counter;
        self.export_status = ExportStatus::Exported {
            bundle_dir: export_paths.bundle_dir.display().to_string(),
        };

        info!(
            evidence_schema_version,
            export_counter = export_paths.export_counter,
            bundle_dir = %export_paths.bundle_dir.display(),
            evidence_json_path = %export_paths.evidence_json_path.display(),
            evidence_markdown_path = %export_paths.evidence_markdown_path.display(),
            visual_context_path = %export_paths.visual_context_path.display(),
            manifest_path = %export_paths.manifest_path.display(),
            selected_event_count,
            "RawScope timeline selection evidence report bundle exported"
        );
    }

    fn next_evidence_export_counter(&mut self) -> u64 {
        self.evidence_export_counter += 1;
        self.evidence_export_counter
    }

    fn scatter_selection_evidence_v2(&self) -> Option<ScatterSelectionEvidenceV2> {
        let evidence = self.scatter.selection_evidence.as_ref()?;
        let dataset_identity = self.dataset_identity.clone()?;
        let viewport = self.scatter.viewport?;
        let render_stats = self.scatter.render_stats?;
        let view = ScatterEvidenceView {
            x_range: viewport.x_range(),
            y_range: viewport.y_range(),
            grid_width: render_stats.grid_width,
            grid_height: render_stats.grid_height,
        };

        Some(ScatterSelectionEvidenceV2::from_v1(
            evidence,
            dataset_identity,
            view,
            self.scatter.source_rows.as_ref(),
        ))
    }

    fn scatter_selection_evidence_v3(
        &self,
        evidence_v2: &ScatterSelectionEvidenceV2,
    ) -> Option<ScatterSelectionEvidenceV3> {
        let comparison = match self.active_comparison.as_ref()? {
            WorkbenchComparison::Scatter(comparison) => comparison.clone(),
            _ => return None,
        };
        let aggregate_overview = self.scatter.scatter_aggregate_overview.as_ref()?;
        Some(ScatterSelectionEvidenceV3::from_v2_with_presentation(
            evidence_v2,
            self.scatter.density_encoding,
            self.scatter.density_presentation,
            comparison,
            scatter_aggregate_evidence_context(
                aggregate_overview,
                &evidence_v2.selected_row_id_sample,
            ),
            self.active_dataset_profile,
        ))
    }

    fn timeline_selection_evidence_v2(&self) -> Option<TimelineSelectionEvidenceV2> {
        let evidence = self.timeline.selection_evidence.as_ref()?;
        let dataset_identity = self.dataset_identity.clone()?;
        let viewport = self.timeline.viewport?;
        let render_stats = self.timeline.render_stats?;
        let view = TimelineEvidenceView {
            time_range: viewport.time_range(),
            full_time_range: viewport.full_time_range(),
            lane_count: viewport.lane_count(),
            grid_width: render_stats.grid_width,
            grid_height: render_stats.grid_height,
        };

        Some(TimelineSelectionEvidenceV2::from_v1(
            evidence,
            dataset_identity,
            view,
            self.timeline.source_rows.as_ref(),
        ))
    }

    fn timeline_selection_evidence_v3(
        &self,
        evidence_v2: &TimelineSelectionEvidenceV2,
    ) -> Option<TimelineSelectionEvidenceV3> {
        let comparison = match self.active_comparison.as_ref()? {
            WorkbenchComparison::Timeline(comparison) => comparison.clone(),
            _ => return None,
        };
        let aggregate_overview = self.timeline.timeline_aggregate_overview.as_ref()?;
        Some(TimelineSelectionEvidenceV3::from_v2(
            evidence_v2,
            self.timeline.density_encoding,
            comparison,
            timeline_aggregate_evidence_context(
                aggregate_overview,
                &evidence_v2.selected_row_id_sample,
            ),
            self.active_dataset_profile,
        ))
    }
}
