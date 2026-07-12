//! Evidence report-bundle export routing for the workbench density demos.

use tracing::{error, info, warn};

use rawscope_render::{
    normalized_difference_density, scatter_aggregate_evidence_context,
    timeline_aggregate_evidence_context, DifferenceDensityEvidenceConfig,
    PinnedScatterInspectionEvidence, PointRevealEvidence, PointRevealStats, ScatterCohortEvidence,
    ScatterDensityMode, ScatterDensityPresentation, ScatterEvidenceView, ScatterInspectionConfig,
    ScatterSelectionEvidenceV2, ScatterSelectionEvidenceV3, ScatterSelectionEvidenceV4,
    ScatterVisualQueryV4, TimelineEvidenceView, TimelineSelectionEvidenceV2,
    TimelineSelectionEvidenceV3, DIFFERENCE_BASELINE_ID, DIFFERENCE_FORMULA_ID,
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
        let Some(evidence_v2) = self.scatter_selection_evidence_v2() else {
            self.workbench_state.export_status = ExportStatus::NoSelection;
            warn!(
                reason = "no finalized scatter selection evidence",
                "scatter selection evidence export skipped"
            );
            return;
        };
        let evidence_v3 = self.scatter_selection_evidence_v3(&evidence_v2);
        let evidence_v4 = evidence_v3
            .as_ref()
            .and_then(|evidence| self.scatter_selection_evidence_v4(evidence));
        let evidence_v5 = evidence_v4
            .as_ref()
            .and_then(|evidence| self.scatter_selection_evidence_v5(evidence));

        let export_counter = self.next_evidence_export_counter();
        let export_timestamp_unix_ms = current_unix_timestamp_ms();
        let export_paths = EvidenceReportBundlePaths::next_available(
            EXPORT_DIR,
            SCATTER_REPORT_BUNDLE_DIR_PREFIX,
            export_timestamp_unix_ms,
            export_counter,
        );
        let write_result = match (
            evidence_v5.as_ref(),
            evidence_v4.as_ref(),
            evidence_v3.as_ref(),
        ) {
            (Some(evidence), _, _) => export_paths.write_scatter_v5(evidence),
            (None, Some(evidence), _) => export_paths.write_scatter_v4(evidence),
            (None, None, Some(evidence)) => export_paths.write_scatter_v3(evidence),
            (None, None, None) => export_paths.write_scatter(&evidence_v2),
        };
        if let Err(err) = write_result {
            self.workbench_state.export_status = ExportStatus::Failed {
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
        let (selected_row_count, evidence_schema_version) = evidence_v5
            .as_ref()
            .map(|evidence| (evidence.selected_row_count, evidence.schema_version))
            .or_else(|| {
                evidence_v4
                    .as_ref()
                    .map(|evidence| (evidence.selected_row_count, evidence.schema_version))
            })
            .or_else(|| {
                evidence_v3
                    .as_ref()
                    .map(|evidence| (evidence.selected_row_count, evidence.schema_version))
            })
            .unwrap_or((evidence_v2.selected_row_count, 2));
        self.workbench_state.evidence_export_counter = export_paths.export_counter;
        self.workbench_state.export_status = ExportStatus::Exported {
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
            self.workbench_state.export_status = ExportStatus::NoSelection;
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
            self.workbench_state.export_status = ExportStatus::Failed {
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
        self.workbench_state.evidence_export_counter = export_paths.export_counter;
        self.workbench_state.export_status = ExportStatus::Exported {
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
        self.workbench_state.evidence_export_counter = self
            .workbench_state
            .evidence_export_counter
            .saturating_add(1);
        self.workbench_state.evidence_export_counter
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

    fn scatter_selection_evidence_v4(
        &self,
        evidence_v3: &ScatterSelectionEvidenceV3,
    ) -> Option<ScatterSelectionEvidenceV4> {
        let full_row_count = self.scatter.points.len();
        let (included_row_count, excluded_row_count) = self
            .scatter_filters
            .evaluation
            .as_ref()
            .map(|evaluation| (evaluation.included_count, evaluation.excluded_count))
            .unwrap_or((full_row_count, 0));
        let point_stats = self.point_reveal.stats.unwrap_or(PointRevealStats {
            eligible_count: 0,
            rendered_count: 0,
            sampled: false,
            blend: 0.0,
        });
        let density_mode = self.scatter.density_mode;
        let difference = (density_mode == ScatterDensityMode::FilteredDifference)
            .then(|| self.difference_evidence_config())
            .flatten();
        if density_mode == ScatterDensityMode::FilteredDifference && difference.is_none() {
            return None;
        }
        let density_presentation = if density_mode == ScatterDensityMode::FilteredDifference {
            ScatterDensityPresentation::ExactCells
        } else {
            self.scatter.density_presentation
        };
        let rendered_count = if density_mode == ScatterDensityMode::FilteredDifference {
            0
        } else {
            point_stats.rendered_count
        };
        let visual_query = ScatterVisualQueryV4 {
            x_range: evidence_v3.view.x_range,
            y_range: evidence_v3.view.y_range,
            grid_width: evidence_v3.view.grid_width,
            grid_height: evidence_v3.view.grid_height,
            projection: self.scatter_projection.active,
            filters: self.scatter_filters.filters.filters.clone(),
            density_mode,
            density_encoding: self.scatter.density_encoding,
            density_presentation,
            difference,
            point_reveal: PointRevealEvidence {
                mode: self.point_reveal.config.mode,
                eligible_count: point_stats.eligible_count,
                rendered_count,
                sampled: point_stats.sampled,
            },
            relief: (density_mode == ScatterDensityMode::AbsoluteDensity
                && density_presentation == ScatterDensityPresentation::ReliefField)
                .then_some(self.scatter.relief_config),
        };
        let pinned_inspection =
            self.scatter_inspection
                .pinned
                .as_ref()
                .map(|pinned| PinnedScatterInspectionEvidence {
                    bin_x: pinned.summary.hit.bin_x,
                    bin_y: pinned.summary.hit.bin_y,
                    x_range: pinned.summary.hit.x_range,
                    y_range: pinned.summary.hit.y_range,
                    row_count: pinned.summary.hit.count,
                    row_id_sample: pinned.summary.hit.row_ids.iter().copied().collect(),
                    sample_limit: ScatterInspectionConfig::default().max_row_ids_per_bin,
                });
        ScatterSelectionEvidenceV4::from_v3(
            evidence_v3,
            visual_query,
            ScatterCohortEvidence {
                full_row_count,
                included_row_count,
                excluded_row_count,
            },
            pinned_inspection,
        )
        .ok()
    }

    fn difference_evidence_config(&self) -> Option<DifferenceDensityEvidenceConfig> {
        let baseline_grid = self.scatter_inspection.baseline_grid.as_ref()?;
        let active_grid = self.scatter_inspection.grid.as_ref()?;
        let difference_stats = self.scatter.difference_stats?;
        let baseline_counts = baseline_grid
            .bins
            .iter()
            .map(|bin| bin.count)
            .collect::<Vec<_>>();
        let active_counts = active_grid
            .bins
            .iter()
            .map(|bin| bin.count)
            .collect::<Vec<_>>();
        let grid = normalized_difference_density(
            &baseline_counts,
            &active_counts,
            difference_stats.baseline_total,
            difference_stats.active_total,
        )
        .ok()?;
        Some(DifferenceDensityEvidenceConfig {
            formula: DIFFERENCE_FORMULA_ID,
            baseline: DIFFERENCE_BASELINE_ID,
            baseline_total: difference_stats.baseline_total,
            active_total: difference_stats.active_total,
            max_abs_delta: grid.stats.max_abs_delta as f32,
        })
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
