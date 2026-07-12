//! Settled scatter inspection cache and hover/pin coordination.

use std::collections::BTreeMap;

use rawscope_data::{
    available_profile_filter_hints, dataset_profile, DatasetProfileFilterKind, FilterMask,
    FilterRevision, LoadedSourceRow,
};
use rawscope_render::{
    build_difference_inspection_distribution, build_scatter_inspection_grid,
    DifferenceInspectionDistribution, DifferenceInspectionSummary, ScatterDensityMode,
    ScatterInspectionConfig, ScatterInspectionGrid, ScatterInspectionHit, ScatterInspectionSummary,
};

use crate::{
    app::WorkbenchApp,
    app_interaction_mode::{ActivePointerGesture, WorkbenchInteractionMode},
    demo::DemoMode,
};

const MAX_PINNED_CATEGORY_FIELDS: usize = 3;
const MAX_PINNED_CATEGORY_VALUES: usize = 4;

#[derive(Debug, Clone, Default)]
pub(crate) struct ScatterInspectionState {
    pub(crate) grid: Option<ScatterInspectionGrid>,
    pub(crate) baseline_grid: Option<ScatterInspectionGrid>,
    pub(crate) hovered: Option<ScatterInspectionHit>,
    pub(crate) hovered_summary: Option<ScatterInspectionSummary>,
    pub(crate) difference_distribution: Option<DifferenceInspectionDistribution>,
    pub(crate) pinned: Option<PinnedScatterInspection>,
    pub(crate) cache_viewport_revision: u64,
    pub(crate) cache_filter_revision: FilterRevision,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PinnedScatterInspection {
    pub(crate) summary: ScatterInspectionSummary,
    pub(crate) difference: Option<DifferenceInspectionSummary>,
    pub(crate) category_summaries: Vec<PinnedCategorySummary>,
    pub(crate) source_rows: Vec<LoadedSourceRow>,
    pub(crate) evidence_keys: Vec<PinnedEvidenceKeyValue>,
    pub(crate) cache_viewport_revision: u64,
    pub(crate) cache_filter_revision: FilterRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinnedCategorySummary {
    pub(crate) column_name: String,
    pub(crate) sample_row_count: usize,
    pub(crate) bin_row_count: u32,
    pub(crate) value_counts: Vec<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinnedEvidenceKeyValue {
    pub(crate) column_name: String,
    pub(crate) value: String,
}

impl WorkbenchApp {
    pub(crate) fn rebuild_scatter_inspection_cache(&mut self) {
        let (Some(viewport), Some(stats), Some(evaluation)) = (
            self.scatter.viewport,
            self.scatter.render_stats,
            self.scatter_filters.evaluation.as_ref(),
        ) else {
            self.scatter_inspection = ScatterInspectionState::default();
            return;
        };
        let viewport_revision = self.render_schedule.settled_revision();
        let config = ScatterInspectionConfig {
            grid_width: stats.grid_width,
            grid_height: stats.grid_height,
            ..ScatterInspectionConfig::default()
        };
        match build_scatter_inspection_grid(
            &self.scatter.points,
            &evaluation.mask,
            viewport.x_range(),
            viewport.y_range(),
            evaluation.revision,
            config,
        ) {
            Ok(grid) => {
                let baseline_mask = FilterMask::all_included(self.scatter.points.len());
                let baseline_grid = build_scatter_inspection_grid(
                    &self.scatter.points,
                    &baseline_mask,
                    viewport.x_range(),
                    viewport.y_range(),
                    FilterRevision::default(),
                    config,
                )
                .ok();
                let difference_distribution = baseline_grid.as_ref().and_then(|baseline| {
                    let baseline_counts = baseline
                        .bins
                        .iter()
                        .map(|bin| bin.count)
                        .collect::<Vec<_>>();
                    let active_counts = grid.bins.iter().map(|bin| bin.count).collect::<Vec<_>>();
                    build_difference_inspection_distribution(
                        &baseline_counts,
                        &active_counts,
                        self.scatter.points.len() as u64,
                        evaluation.included_count as u64,
                    )
                    .ok()
                });
                self.scatter_inspection.baseline_grid = baseline_grid;
                self.scatter_inspection.difference_distribution = difference_distribution;
                self.scatter_inspection.grid = Some(grid);
                self.scatter_inspection.hovered = None;
                self.scatter_inspection.hovered_summary = None;
                self.scatter_inspection.cache_viewport_revision = viewport_revision;
                self.scatter_inspection.cache_filter_revision = evaluation.revision;
            }
            Err(_) => self.scatter_inspection = ScatterInspectionState::default(),
        }
    }

    pub(crate) fn invalidate_scatter_inspection(&mut self) {
        self.scatter_inspection = ScatterInspectionState::default();
        self.clear_inspection_presentation();
    }

    pub(crate) fn clear_scatter_inspection_hover(&mut self) {
        self.scatter_inspection.hovered = None;
        self.scatter_inspection.hovered_summary = None;
    }

    pub(crate) fn refresh_scatter_inspection_hover(&mut self) {
        if self.demo_mode != DemoMode::Scatter
            || self.interaction_mode != WorkbenchInteractionMode::Inspect
            || self.render_schedule.is_refining()
            || matches!(
                self.active_pointer_gesture,
                Some(
                    ActivePointerGesture::Pan
                        | ActivePointerGesture::ScatterBrush
                        | ActivePointerGesture::TimelineBrush
                )
            )
        {
            self.clear_scatter_inspection_hover();
            return;
        }
        let Some((x_fraction, y_fraction)) = self.cursor_fraction() else {
            self.clear_scatter_inspection_hover();
            return;
        };
        let Some(evaluation) = self.scatter_filters.evaluation.as_ref() else {
            self.clear_scatter_inspection_hover();
            return;
        };
        let cache_is_current = self.scatter_inspection.cache_viewport_revision
            == self.render_schedule.settled_revision()
            && self.scatter_inspection.cache_filter_revision == evaluation.revision;
        self.scatter_inspection.hovered = cache_is_current
            .then(|| {
                self.scatter_inspection
                    .grid
                    .as_ref()?
                    .inspect_fraction(x_fraction, y_fraction)
            })
            .flatten();
        self.scatter_inspection.hovered_summary =
            self.scatter_inspection.hovered.as_ref().and_then(|hit| {
                self.scatter_inspection
                    .grid
                    .as_ref()
                    .map(|grid| grid.summarize_hit(hit.clone()))
            });
    }

    pub(crate) fn pin_scatter_inspection(&mut self) {
        self.refresh_scatter_inspection_hover();
        let Some(hit) = self.scatter_inspection.hovered.clone() else {
            return;
        };
        if !crate::ui_scatter_inspection::inspection_hit_is_meaningful(self, &hit) {
            return;
        }
        let Some(summary) = self
            .scatter_inspection
            .hovered_summary
            .clone()
            .filter(|summary| summary.hit.bin_x == hit.bin_x && summary.hit.bin_y == hit.bin_y)
        else {
            return;
        };
        let source_rows = self
            .scatter
            .source_rows
            .as_ref()
            .map(|table| {
                hit.row_ids
                    .iter()
                    .filter_map(|row_id| table.row(*row_id).cloned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let category_summaries = self.pinned_category_summaries(&source_rows, hit.count);
        let evidence_keys = self.pinned_evidence_key_values(&source_rows);
        let difference = difference_summary_for_hit(self, &hit);
        self.scatter_inspection.pinned = Some(PinnedScatterInspection {
            summary,
            difference,
            category_summaries,
            source_rows,
            evidence_keys,
            cache_viewport_revision: self.scatter_inspection.cache_viewport_revision,
            cache_filter_revision: self.scatter_inspection.cache_filter_revision,
        });
        self.request_redraw();
    }

    pub(crate) fn clear_pinned_scatter_inspection(&mut self) {
        self.scatter_inspection.pinned = None;
        self.request_redraw();
    }

    fn pinned_category_summaries(
        &self,
        rows: &[LoadedSourceRow],
        bin_row_count: u32,
    ) -> Vec<PinnedCategorySummary> {
        let (Some(profile_id), Some(catalog), Some(source)) = (
            self.workbench_state.active_dataset_profile,
            self.scatter_filters.catalog.as_ref(),
            self.scatter.source_rows.as_ref(),
        ) else {
            return Vec::new();
        };
        available_profile_filter_hints(dataset_profile(profile_id), catalog)
            .into_iter()
            .filter(|hint| hint.kind == DatasetProfileFilterKind::Categorical)
            .take(MAX_PINNED_CATEGORY_FIELDS)
            .filter_map(|hint| {
                let column_index = source
                    .columns
                    .iter()
                    .position(|column| column.name == hint.column_name)?;
                let mut counts = BTreeMap::<String, usize>::new();
                for row in rows {
                    let value = row.values.get(column_index)?.trim();
                    let display_value = if value.is_empty() { "Missing" } else { value };
                    *counts.entry(display_value.to_string()).or_default() += 1;
                }
                let mut value_counts = counts.into_iter().collect::<Vec<_>>();
                value_counts
                    .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
                value_counts.truncate(MAX_PINNED_CATEGORY_VALUES);
                Some(PinnedCategorySummary {
                    column_name: hint.column_name.to_string(),
                    sample_row_count: rows.len(),
                    bin_row_count,
                    value_counts,
                })
            })
            .collect()
    }

    fn pinned_evidence_key_values(&self, rows: &[LoadedSourceRow]) -> Vec<PinnedEvidenceKeyValue> {
        let Some(key) = self
            .workbench_state
            .active_session
            .as_ref()
            .and_then(|session| session.evidence_key.as_ref())
        else {
            return Vec::new();
        };
        rows.iter()
            .filter_map(|row| {
                Some(PinnedEvidenceKeyValue {
                    column_name: key.column_name.clone(),
                    value: key.value(row)?.to_string(),
                })
            })
            .collect()
    }
}

pub(crate) fn difference_summary_for_hit(
    app: &WorkbenchApp,
    hit: &ScatterInspectionHit,
) -> Option<DifferenceInspectionSummary> {
    if app.scatter.density_mode != ScatterDensityMode::FilteredDifference {
        return None;
    }
    let baseline_count = app
        .scatter_inspection
        .baseline_grid
        .as_ref()?
        .inspect_bin(hit.bin_x, hit.bin_y)?
        .count;
    app.scatter_inspection
        .difference_distribution
        .as_ref()
        .map(|distribution| distribution.summarize_counts(baseline_count, hit.count))
}

#[cfg(test)]
#[path = "app_scatter_inspection_tests.rs"]
mod tests;
