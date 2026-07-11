//! Settled scatter inspection cache and hover/pin coordination.

use std::collections::BTreeMap;

use rawscope_data::{
    available_profile_filter_hints, dataset_profile, DatasetProfileFilterKind, FilterMask,
    FilterRevision, LoadedSourceRow,
};
use rawscope_render::{
    build_scatter_inspection_grid, ScatterInspectionConfig, ScatterInspectionGrid,
    ScatterInspectionHit, ScatterInspectionSummary,
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
    pub(crate) pinned: Option<PinnedScatterInspection>,
    pub(crate) cache_viewport_revision: u64,
    pub(crate) cache_filter_revision: FilterRevision,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PinnedScatterInspection {
    pub(crate) hit: ScatterInspectionHit,
    pub(crate) category_summaries: Vec<PinnedCategorySummary>,
    pub(crate) source_rows: Vec<LoadedSourceRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinnedCategorySummary {
    pub(crate) column_name: String,
    pub(crate) value_counts: Vec<(String, usize)>,
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
                self.scatter_inspection.baseline_grid = build_scatter_inspection_grid(
                    &self.scatter.points,
                    &baseline_mask,
                    viewport.x_range(),
                    viewport.y_range(),
                    FilterRevision::default(),
                    config,
                )
                .ok();
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
        let category_summaries = self.pinned_category_summaries(&source_rows);
        self.scatter_inspection.pinned = Some(PinnedScatterInspection {
            hit,
            category_summaries,
            source_rows,
        });
        self.request_redraw();
    }

    pub(crate) fn clear_pinned_scatter_inspection(&mut self) {
        self.scatter_inspection.pinned = None;
        self.request_redraw();
    }

    fn pinned_category_summaries(&self, rows: &[LoadedSourceRow]) -> Vec<PinnedCategorySummary> {
        let (Some(profile_id), Some(catalog), Some(source)) = (
            self.active_dataset_profile,
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
                    value_counts,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use egui::{pos2, Rect};
    use rawscope_core::{F32Range, RowId};
    use rawscope_data::{
        build_visual_field_catalog, evaluate_filters, FilterSet, LoadedColumnKind,
        LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, ScatterPointKind,
        ScatterPointRecord, VisualFieldCatalogConfig,
    };
    use rawscope_render::{
        build_scatter_inspection_grid, PlotRectPx, ScatterInspectionConfig, ScatterViewport,
    };
    use winit::dpi::PhysicalPosition;

    use super::*;

    #[test]
    fn stale_revision_disables_hover_hit() {
        let mut app = inspection_app();
        app.refresh_scatter_inspection_hover();
        assert!(app.scatter_inspection.hovered.is_some());
        app.scatter_inspection.cache_filter_revision = FilterRevision(99);

        app.refresh_scatter_inspection_hover();

        assert!(app.scatter_inspection.hovered.is_none());
        assert!(app.scatter_inspection.hovered_summary.is_none());
    }

    #[test]
    fn settled_hover_consumes_cached_inspection_summary() {
        let mut app = inspection_app();

        app.refresh_scatter_inspection_hover();

        let summary = app
            .scatter_inspection
            .hovered_summary
            .as_ref()
            .expect("current hover should project its settled summary");
        assert_eq!(summary.hit.count, 1);
        assert_eq!(summary.neighborhood_count, 1);
    }

    fn inspection_app() -> WorkbenchApp {
        let source = LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "winner".into(),
                kind: LoadedColumnKind::String,
            }],
            rows: vec![LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["white".into()],
            }],
        };
        let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
        let evaluation = evaluate_filters(&source, &catalog, &FilterSet::default()).unwrap();
        let points = vec![ScatterPointRecord {
            row_id: RowId(0),
            x: 5.0,
            y: 5.0,
            kind: ScatterPointKind::Unclassified,
        }];
        let grid = build_scatter_inspection_grid(
            &points,
            &evaluation.mask,
            F32Range::new(0.0, 10.0),
            F32Range::new(0.0, 10.0),
            evaluation.revision,
            ScatterInspectionConfig {
                grid_width: 10,
                grid_height: 10,
                max_row_ids_per_bin: 16,
            },
        )
        .unwrap();
        WorkbenchApp {
            demo_mode: DemoMode::Scatter,
            interaction_mode: WorkbenchInteractionMode::Inspect,
            cursor_position: Some(PhysicalPosition::new(50.0, 50.0)),
            plot_surface: Some(crate::ui_plot_surface::PlotSurfaceLayout {
                logical_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)),
                physical_rect: PlotRectPx::try_new(0, 0, 100, 100, 100, 100).unwrap(),
                axis_layout: crate::ui_plot_surface::PlotAxisLayout {
                    outer_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)),
                    plot_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)),
                },
            }),
            scatter: crate::app::ScatterWorkbenchState {
                points,
                source_rows: Some(source),
                viewport: Some(ScatterViewport::new(
                    F32Range::new(0.0, 10.0),
                    F32Range::new(0.0, 10.0),
                )),
                ..Default::default()
            },
            scatter_filters: crate::app_scatter_filter::ScatterFilterState {
                catalog: Some(catalog),
                evaluation: Some(evaluation),
                ..Default::default()
            },
            scatter_inspection: ScatterInspectionState {
                grid: Some(grid),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
