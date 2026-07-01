//! CPU and early GPU correctness density renderers for RawScope.

mod density_reference;
mod gpu_scatter_density;
mod scatter_brush;
mod scatter_brush_overlay;
mod scatter_density_renderer;
mod scatter_selection_evidence;
mod scatter_viewport;

pub use density_reference::{scatter_density, timeline_density};
pub use gpu_scatter_density::{
    gpu_scatter_density, gpu_scatter_density_on_device, GpuScatterDensityError,
    GpuScatterDensityGrid,
};
pub use scatter_brush::{
    BrushScreenPoint, BrushScreenRect, BrushScreenSize, ScatterBrushDrag, ScatterBrushSelection,
    SelectedCategoryCounts, SelectedRegionSummary,
};
pub use scatter_brush_overlay::ScatterBrushOverlayRenderer;
pub use scatter_density_renderer::{
    log_density_intensity, ScatterDensityRenderDiagnostics, ScatterDensityRenderer,
    ScatterDensityRendererConfig,
};
pub use scatter_selection_evidence::{
    ScatterSelectionEvidence, SelectedPointSample, SelectionEvidenceConfig,
};
pub use scatter_viewport::ScatterViewport;

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "CPU reference density renderers, correctness-first GPU scatter-density compute, and simple density presentation"
}
