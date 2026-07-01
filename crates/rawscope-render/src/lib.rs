//! CPU and early GPU correctness density renderers for RawScope.

mod density_reference;
mod gpu_density_pipeline;
mod gpu_scatter_density;
mod gpu_scatter_density_pack;
mod gpu_timeline_density;
mod gpu_timeline_density_pack;
mod scatter_brush;
mod scatter_brush_overlay;
mod scatter_density_renderer;
mod scatter_selection_evidence;
mod scatter_selection_export;
mod scatter_viewport;
mod timeline_brush;
mod timeline_density_renderer;
mod timeline_selection_evidence;
mod timeline_selection_export;
mod timeline_viewport;

pub use density_reference::{scatter_density, timeline_density};
pub use gpu_scatter_density::{
    gpu_scatter_density, gpu_scatter_density_on_device, GpuScatterDensityError,
    GpuScatterDensityGrid,
};
pub use gpu_timeline_density::{
    gpu_timeline_density, gpu_timeline_density_on_device, GpuTimelineDensityError,
    GpuTimelineDensityGrid,
};
pub use scatter_brush::{
    BrushScreenPoint, BrushScreenRect, BrushScreenSize, ScatterBrushDrag, ScatterBrushSelection,
    SelectedCategoryCounts, SelectedRegionSummary,
};
pub use scatter_brush_overlay::ScatterBrushOverlayRenderer;
pub use scatter_density_renderer::{
    log_density_intensity, ScatterDensityRenderStats, ScatterDensityRenderer,
    ScatterDensityRendererConfig,
};
pub use scatter_selection_evidence::{
    ScatterSelectionEvidence, SelectedPointSample, SelectionEvidenceConfig,
};
pub use scatter_selection_export::{
    scatter_selection_evidence_json, scatter_selection_evidence_markdown,
    SCATTER_SELECTION_EVIDENCE_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_SCHEMA_VERSION,
};
pub use scatter_viewport::ScatterViewport;
pub use timeline_brush::{
    SelectedEventTypeCounts, TimelineBrushDrag, TimelineBrushSelection, TimelineLaneRange,
    TimelineSelectionSummary,
};
pub use timeline_density_renderer::{
    TimelineDensityRenderStats, TimelineDensityRenderer, TimelineDensityRendererConfig,
};
pub use timeline_selection_evidence::{
    SelectedTimelineEventSample, TimelineEvidenceConfig, TimelineSelectionEvidence,
};
pub use timeline_selection_export::{
    timeline_selection_evidence_json, timeline_selection_evidence_markdown,
    TIMELINE_SELECTION_EVIDENCE_ARTIFACT_KIND, TIMELINE_SELECTION_EVIDENCE_SCHEMA_VERSION,
};
pub use timeline_viewport::TimelineViewport;

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "CPU reference density renderers, correctness-first GPU density compute, and simple density presentation"
}
