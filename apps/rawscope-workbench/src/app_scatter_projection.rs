//! Scatter projection state and cross-cache coordination.

use std::error::Error;

use rawscope_analysis::visual_field::{VisualFieldMapping, VisualFieldProjection};
use rawscope_data::{
    dataset_profile, project_scatter_points, DatasetSchema, LoadedColumnKind, ProjectedScatterData,
    ScatterPointRecord, ScatterProjection, ScatterProjectionLabels, ScatterProjectionSpec,
    StoreColumnKind, VisualFieldCatalog,
};
use rawscope_render::ScatterViewport;

use crate::{app::WorkbenchApp, controllers::visual_field::VisualFieldController};

#[derive(Debug, Clone)]
pub(crate) struct ScatterProjectionState {
    pub(crate) active: ScatterProjection,
    pub(crate) spec: Option<ScatterProjectionSpec>,
    pub(crate) labels: ScatterProjectionLabels,
    pub(crate) raw_points: Vec<ScatterPointRecord>,
    pub(crate) available: bool,
    pub(crate) show_equality_guide: bool,
}

#[derive(Debug, Clone)]
struct PreparedProjectionState {
    projected: ProjectedScatterData,
    filter_state: Option<(rawscope_data::FilterMask, rawscope_data::FilterRevision)>,
}

impl Default for ScatterProjectionState {
    fn default() -> Self {
        Self {
            active: ScatterProjection::RawXY,
            spec: None,
            labels: ScatterProjectionLabels {
                x_label: "x".to_string(),
                y_label: "y".to_string(),
            },
            raw_points: Vec::new(),
            available: false,
            show_equality_guide: false,
        }
    }
}

impl WorkbenchApp {
    pub(crate) fn initialize_scatter_projection(&mut self, x_column: &str, y_column: &str) {
        let available = self
            .scatter_filters
            .catalog
            .as_ref()
            .is_some_and(|catalog| {
                [x_column, y_column].into_iter().all(|column_name| {
                    catalog.fields.iter().any(|field| {
                        field.column_name == column_name
                            && matches!(
                                field.source_kind,
                                LoadedColumnKind::Integer | LoadedColumnKind::Float
                            )
                    })
                })
            });
        let show_equality_guide = self
            .workbench_state
            .active_dataset_profile
            .map(dataset_profile)
            .is_some_and(|profile| profile.scatter_defaults.show_equality_guide);
        self.scatter_projection = ScatterProjectionState {
            active: ScatterProjection::RawXY,
            spec: Some(ScatterProjectionSpec::new(x_column, y_column)),
            labels: ScatterProjectionLabels {
                x_label: x_column.to_string(),
                y_label: y_column.to_string(),
            },
            raw_points: self.scatter.points.clone(),
            available,
            show_equality_guide,
        };
        self.visual_field_controller = self
            .scatter_filters
            .catalog
            .as_ref()
            .and_then(|catalog| visual_mapping_from_catalog(catalog, x_column, y_column))
            .map(VisualFieldController::new);
    }

    pub(crate) fn set_scatter_projection(
        &mut self,
        projection: ScatterProjection,
    ) -> Result<(), Box<dyn Error>> {
        if !self.scatter_projection.available || self.scatter_projection.active == projection {
            return Ok(());
        }
        let prepared = self.prepare_scatter_projection(projection)?;
        let projected = prepared.projected;
        let filter_state = prepared.filter_state;

        self.apply_scatter_projection(projected, filter_state)
    }

    fn prepare_scatter_projection(
        &self,
        projection: ScatterProjection,
    ) -> Result<PreparedProjectionState, Box<dyn Error>> {
        let Some(spec) = self.scatter_projection.spec.as_ref() else {
            return Err("scatter projection specification is unavailable".into());
        };
        let projected = project_scatter_points(
            &self.scatter_projection.raw_points,
            &spec.x_column,
            &spec.y_column,
            projection,
        )?;
        if let Some(renderer) = self.scatter.density_renderer.as_ref() {
            renderer.validate_dataset(&projected.points)?;
        }
        if let Some(renderer) = self.scatter.difference_renderer.as_ref() {
            renderer.validate_dataset(&projected.points)?;
        }
        let filter_state = self
            .scatter_filters
            .cohort_snapshot
            .as_ref()
            .map(|snapshot| (snapshot.filter_mask(), snapshot.filter_revision()))
            .or_else(|| {
                self.scatter_filters
                    .evaluation
                    .as_ref()
                    .map(|evaluation| (evaluation.mask.clone(), evaluation.revision))
            });

        Ok(PreparedProjectionState {
            projected,
            filter_state,
        })
    }

    fn apply_scatter_projection(
        &mut self,
        projected: ProjectedScatterData,
        filter_state: Option<(rawscope_data::FilterMask, rawscope_data::FilterRevision)>,
    ) -> Result<(), Box<dyn Error>> {
        self.scatter.density_dataset_revision += 1;
        if let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.density_renderer.as_mut())
        {
            renderer.replace_dataset(
                gpu.device(),
                gpu.queue(),
                &projected.points,
                self.scatter.density_dataset_revision,
            )?;
            if let Some((mask, revision)) = filter_state.as_ref() {
                renderer.update_filter_mask(gpu.queue(), mask, *revision)?;
                self.scatter_filters.last_uploaded_revision = *revision;
            }
        }
        if let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.difference_renderer.as_mut())
        {
            renderer.replace_dataset(
                gpu.device(),
                gpu.queue(),
                &projected.points,
                self.scatter.density_dataset_revision,
            )?;
            if let Some((mask, revision)) = filter_state.as_ref() {
                renderer.update_filter_mask(gpu.queue(), mask, *revision)?;
            }
        }
        self.scatter.difference_baseline_dirty = true;

        self.scatter.points = projected.points;
        self.scatter.viewport = Some(ScatterViewport::new(projected.x_range, projected.y_range));
        self.scatter_projection.active = projected.projection;
        self.scatter_projection.labels = projected.labels;
        self.clear_brush();
        self.invalidate_scatter_inspection();
        self.reset_scatter_point_reveal_mask();
        self.invalidate_scatter_point_reveal();
        self.rebuild_scatter_aggregate_overview();
        self.recompute_density()?;
        self.begin_visual_transition(rawscope_render::TransitionKind::ProjectionChange);
        Ok(())
    }
}

fn visual_mapping_from_catalog(
    catalog: &VisualFieldCatalog,
    x_column: &str,
    y_column: &str,
) -> Option<VisualFieldMapping> {
    let schema = DatasetSchema::try_new(catalog.fields.iter().filter_map(|field| {
        loaded_kind_to_store_kind(field.source_kind).map(|kind| (field.column_name.clone(), kind))
    }))
    .ok()?;
    let projection = VisualFieldProjection::NumericPair {
        x: schema.column_id(x_column)?,
        y: schema.column_id(y_column)?,
    };
    let category = catalog
        .fields
        .iter()
        .filter(|field| field.column_name != x_column && field.column_name != y_column)
        .filter_map(|field| {
            loaded_kind_to_store_kind(field.source_kind).and_then(|kind| {
                matches!(
                    kind,
                    StoreColumnKind::Utf8
                        | StoreColumnKind::Bool
                        | StoreColumnKind::I64
                        | StoreColumnKind::U64
                )
                .then(|| schema.column_id(&field.column_name))
                .flatten()
            })
        })
        .next();
    VisualFieldMapping::try_new(&schema, projection, category).ok()
}

fn loaded_kind_to_store_kind(kind: LoadedColumnKind) -> Option<StoreColumnKind> {
    match kind {
        LoadedColumnKind::Integer => Some(StoreColumnKind::I64),
        LoadedColumnKind::Float => Some(StoreColumnKind::F64),
        LoadedColumnKind::String => Some(StoreColumnKind::Utf8),
        LoadedColumnKind::Empty | LoadedColumnKind::Unsupported => None,
    }
}

#[cfg(test)]
mod tests {
    use rawscope_core::{F32Range, RowId};
    use rawscope_data::{
        FilterEvaluation, FilterMask, FilterRevision, ScatterPointKind, VisualFieldCatalog,
        VisualFieldDescriptor, VisualFieldSummary,
    };
    use rawscope_render::ScatterBrushSelection;

    use super::*;

    #[test]
    fn projection_switch_keeps_filter_mask_and_clears_selection() {
        let raw_points = vec![point(0, 1_800.0, 1_600.0), point(1, 1_500.0, 1_700.0)];
        let mask = FilterMask::all_included(2);
        let mut app = WorkbenchApp::default();
        app.scatter.points = raw_points.clone();
        app.scatter.viewport = Some(ScatterViewport::new(
            F32Range::new(1_500.0, 1_800.0),
            F32Range::new(1_600.0, 1_700.0),
        ));
        app.scatter.active_brush_selection = Some(ScatterBrushSelection {
            x_range: F32Range::new(1_500.0, 1_800.0),
            y_range: F32Range::new(1_600.0, 1_700.0),
        });
        app.scatter_filters.catalog = Some(numeric_catalog());
        app.scatter_filters.evaluation = Some(FilterEvaluation {
            mask: mask.clone(),
            revision: FilterRevision::default(),
            included_count: 2,
            excluded_count: 0,
        });
        app.initialize_scatter_projection("white_rating", "black_rating");

        app.set_scatter_projection(ScatterProjection::MeanDifference)
            .unwrap();

        assert_eq!(app.scatter.points[0].row_id, raw_points[0].row_id);
        assert_eq!(app.scatter.points[0].y, 200.0);
        assert_eq!(app.scatter_filters.evaluation.as_ref().unwrap().mask, mask);
        assert!(app.scatter.active_brush_selection.is_none());
        assert!(app.scatter_inspection.pinned.is_none());
        assert_eq!(
            app.scatter.viewport.unwrap().full_y_range(),
            F32Range::from_bounds_expanded(-200.0, 200.0)
        );
    }

    #[test]
    fn failed_projection_preparation_keeps_active_projection_and_points() {
        let mut app = WorkbenchApp::default();
        app.scatter_projection.available = true;
        app.scatter_projection.spec = Some(ScatterProjectionSpec::new("x", "y"));
        app.scatter_projection.active = ScatterProjection::RawXY;

        let result = app.set_scatter_projection(ScatterProjection::MeanDifference);

        assert!(result.is_err());
        assert_eq!(app.scatter_projection.active, ScatterProjection::RawXY);
        assert!(app.scatter.points.is_empty());
    }

    fn point(row_id: u64, x: f32, y: f32) -> ScatterPointRecord {
        ScatterPointRecord {
            row_id: RowId(row_id),
            x,
            y,
            kind: ScatterPointKind::Unclassified,
        }
    }

    fn numeric_catalog() -> VisualFieldCatalog {
        VisualFieldCatalog {
            row_count: 2,
            fields: ["white_rating", "black_rating"]
                .into_iter()
                .enumerate()
                .map(|(column_index, column_name)| VisualFieldDescriptor {
                    column_name: column_name.to_string(),
                    column_index,
                    source_kind: LoadedColumnKind::Integer,
                    summary: VisualFieldSummary::Numeric(rawscope_data::NumericFieldSummary {
                        min: 1_500.0,
                        max: 1_800.0,
                        valid_count: 2,
                        missing_count: 0,
                        invalid_count: 0,
                    }),
                })
                .collect(),
        }
    }
}
