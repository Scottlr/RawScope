//! Workbench coordination for density encoding and scatter presentation controls.

use rawscope_render::{
    validate_relief_field_config, DensityTransform, ReliefFieldConfig, ScatterDensityMode,
    ScatterDensityPresentation,
};
use tracing::error;

use crate::{app::WorkbenchApp, demo::DemoMode, ui::WorkbenchSurface};

impl WorkbenchApp {
    pub(crate) fn set_relief_config(&mut self, config: ReliefFieldConfig) {
        let Ok(config) = validate_relief_field_config(config) else {
            return;
        };
        if self.scatter.relief_config == config {
            return;
        }
        self.scatter.relief_config = config;
        if self.scatter.density_presentation == ScatterDensityPresentation::ReliefField
            && self.scatter.density_mode == ScatterDensityMode::AbsoluteDensity
        {
            if let Err(err) = self.recompute_density() {
                error!(error = %err, "failed to recompute density after relief change");
            }
        } else {
            self.request_redraw();
        }
    }

    pub(crate) fn set_scatter_density_mode(&mut self, mode: ScatterDensityMode) {
        if mode == self.scatter.density_mode || !self.demo_mode.is_scatter() {
            return;
        }
        let difference_is_available = self.scatter_filters.is_active()
            && self
                .scatter_filters
                .evaluation
                .as_ref()
                .is_some_and(|evaluation| evaluation.included_count > 0)
            && !self.scatter.points.is_empty();
        if mode == ScatterDensityMode::FilteredDifference && !difference_is_available {
            return;
        }
        self.scatter.density_mode = mode;
        if mode == ScatterDensityMode::FilteredDifference {
            self.invalidate_scatter_point_reveal();
        }
        self.request_redraw();
    }

    pub(crate) fn set_density_transform(&mut self, transform: DensityTransform) {
        if self.visible_surface != WorkbenchSurface::Primary {
            return;
        }

        let recompute_result = match self.demo_mode {
            DemoMode::Scatter => {
                let next_encoding = self.scatter.density_encoding.with_transform(transform);
                if self.scatter.density_encoding == next_encoding {
                    return;
                }
                self.scatter.density_encoding = next_encoding;
                self.recompute_density()
            }
            DemoMode::Timeline => {
                let next_encoding = self.timeline.density_encoding.with_transform(transform);
                if self.timeline.density_encoding == next_encoding {
                    return;
                }
                self.timeline.density_encoding = next_encoding;
                self.recompute_timeline_density()
            }
        };

        if let Err(err) = recompute_result {
            error!(
                error = %err,
                transform = transform.label(),
                "failed to recompute density after transform change"
            );
        }
    }

    pub(crate) fn set_scatter_density_presentation(
        &mut self,
        presentation: ScatterDensityPresentation,
    ) {
        if self.visible_surface != WorkbenchSurface::Primary || !self.demo_mode.is_scatter() {
            return;
        }
        if self.scatter.density_presentation == presentation {
            return;
        }

        self.scatter.density_presentation = presentation;
        if let Err(err) = self.recompute_density() {
            error!(
                error = %err,
                presentation = presentation.evidence_label(),
                "failed to recompute scatter density after presentation change"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use rawscope_core::RowId;
    use rawscope_data::{
        DatasetFilter, FilterEvaluation, FilterMask, FilterRevision, FilterSet, ScatterPointKind,
        ScatterPointRecord,
    };
    use rawscope_render::{PointRevealMode, ScatterDensityMode, ScatterDensityPresentation};

    use crate::app::WorkbenchApp;

    #[test]
    fn difference_mode_requires_filter_and_restores_absolute_settings() {
        let mut app = WorkbenchApp::default();
        app.scatter.points = vec![ScatterPointRecord {
            row_id: RowId(0),
            x: 1.0,
            y: 1.0,
            kind: ScatterPointKind::Unclassified,
        }];
        app.set_scatter_density_mode(ScatterDensityMode::FilteredDifference);
        assert_eq!(
            app.scatter.density_mode,
            ScatterDensityMode::AbsoluteDensity
        );

        app.scatter_filters.filters = FilterSet {
            filters: vec![DatasetFilter::Categories {
                column_name: "cohort".into(),
                included_values: vec!["keep".into()],
                include_missing: false,
            }],
            revision: FilterRevision(1),
        };
        app.scatter_filters.evaluation = Some(FilterEvaluation {
            mask: FilterMask::all_included(1),
            revision: FilterRevision(1),
            included_count: 1,
            excluded_count: 0,
        });
        app.scatter.density_presentation = ScatterDensityPresentation::TopographicField;
        app.point_reveal.config.mode = PointRevealMode::Auto;

        app.set_scatter_density_mode(ScatterDensityMode::FilteredDifference);
        assert_eq!(
            app.scatter.density_mode,
            ScatterDensityMode::FilteredDifference
        );
        app.set_scatter_density_mode(ScatterDensityMode::AbsoluteDensity);
        assert_eq!(
            app.scatter.density_presentation,
            ScatterDensityPresentation::TopographicField
        );
        assert_eq!(app.point_reveal.config.mode, PointRevealMode::Auto);
    }
}
