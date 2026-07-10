//! Commands emitted by the egui workbench shell.

use rawscope_data::ScatterProjection;
use rawscope_render::{
    DensityTransform, PointRevealMode, ScatterDensityMode, ScatterDensityPresentation,
};

use crate::{
    app_interaction_mode::WorkbenchInteractionMode,
    ui::{ActiveView, WorkbenchSurface},
    ui_dataset_diff::DatasetDiffAction,
    ui_filters::FilterAction,
    ui_missingness::MissingnessAction,
    ui_scatter_inspection::ScatterInspectionAction,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct UiActions {
    pub(crate) activate_view: Option<ActiveView>,
    pub(crate) activate_surface: Option<WorkbenchSurface>,
    pub(crate) reset_requested: bool,
    pub(crate) export_requested: bool,
    pub(crate) clear_selection_requested: bool,
    pub(crate) copy_dataset_path: bool,
    pub(crate) toggle_inspector: bool,
    pub(crate) set_interaction_mode: Option<WorkbenchInteractionMode>,
    pub(crate) set_density_transform: Option<DensityTransform>,
    pub(crate) set_scatter_density_presentation: Option<ScatterDensityPresentation>,
    pub(crate) set_point_reveal_mode: Option<PointRevealMode>,
    pub(crate) set_scatter_projection: Option<ScatterProjection>,
    pub(crate) set_scatter_density_mode: Option<ScatterDensityMode>,
    pub(crate) missingness_action: Option<MissingnessAction>,
    pub(crate) dataset_diff_action: Option<DatasetDiffAction>,
    pub(crate) filter_action: Option<FilterAction>,
    pub(crate) scatter_inspection_action: Option<ScatterInspectionAction>,
}
