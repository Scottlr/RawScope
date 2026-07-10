//! CPU and early GPU correctness density renderers for RawScope.

mod aggregate_cache;
mod dataset_diff;
mod density_encoding;
mod density_reference;
mod density_render_pipeline;
mod evidence_sample;
mod gpu_density_pipeline;
mod gpu_scatter_density;
mod gpu_scatter_density_pack;
mod gpu_timeline_density;
mod gpu_timeline_density_pack;
mod mask_alignment;
mod missingness_reference;
mod plot_geometry;
mod scatter_brush;
mod scatter_brush_overlay;
mod scatter_density_gpu_state;
mod scatter_density_presentation;
mod scatter_density_renderer;
mod scatter_density_reprojection;
mod scatter_inspection;
mod scatter_selection_evidence;
mod scatter_selection_evidence_v3;
mod scatter_selection_export;
mod scatter_selection_export_v3;
mod scatter_viewport;
mod selection_comparison;
mod selection_drilldown;
mod timeline_brush;
mod timeline_density_renderer;
mod timeline_selection_evidence;
mod timeline_selection_evidence_v3;
mod timeline_selection_export;
mod timeline_selection_export_v3;
mod timeline_viewport;
mod view_axes;
mod view_summaries;

pub use aggregate_cache::{
    scatter_aggregate_overview, timeline_aggregate_overview, AggregateBinSample,
    AggregateCacheConfig, AggregateCacheError, ScatterAggregateOverview, TimelineAggregateOverview,
};
pub use dataset_diff::{
    dataset_diff_summary, DatasetDiffColumn, DatasetDiffColumnStatus, DatasetDiffMissingnessDelta,
    DatasetDiffSummary,
};
pub use density_encoding::{
    density_intensity, DensityEncoding, DensityNormalization, DensityPalette, DensityTransform,
};
pub use density_reference::{scatter_density, timeline_density};
pub use gpu_scatter_density::{
    gpu_scatter_density, gpu_scatter_density_masked, gpu_scatter_density_on_device,
    GpuScatterDensityError, GpuScatterDensityGrid,
};
pub use gpu_timeline_density::{
    gpu_timeline_density, gpu_timeline_density_on_device, GpuTimelineDensityError,
    GpuTimelineDensityGrid,
};
pub use mask_alignment::MaskAlignmentError;
pub use missingness_reference::{
    missingness_grid, missingness_selection_summary, MissingnessCell, MissingnessGrid,
    MissingnessSelection, MissingnessSelectionSummary,
};
pub use plot_geometry::{PlotGeometryError, PlotPointPx, PlotRectPx};
pub use scatter_brush::{
    selected_region_summary_masked, BrushScreenPoint, BrushScreenRect, BrushScreenSize,
    ScatterBrushDrag, ScatterBrushSelection, SelectedCategoryCounts, SelectedRegionSummary,
};
pub use scatter_brush_overlay::ScatterBrushOverlayRenderer;
pub use scatter_density_gpu_state::{
    DensityReadbackPolicy, ScatterDensityGpuState, ScatterDensityUpdate,
};
pub use scatter_density_presentation::ScatterDensityPresentation;
pub use scatter_density_renderer::{
    ScatterDensityRenderStats, ScatterDensityRenderer, ScatterDensityRendererConfig,
};
pub use scatter_density_reprojection::{
    DensityFieldViewport, DensityQualityTier, DensityReprojection,
};
pub use scatter_inspection::{
    build_scatter_inspection_grid, ScatterInspectionBin, ScatterInspectionConfig,
    ScatterInspectionError, ScatterInspectionGrid, ScatterInspectionHit,
};
pub use scatter_selection_evidence::{
    ScatterEvidenceView, ScatterSelectionEvidence, ScatterSelectionEvidenceV2, SelectedPointSample,
    SelectedPointSampleV2, SelectedSourceRowSample, SelectionEvidenceConfig,
};
pub use scatter_selection_evidence_v3::{
    AggregateEvidenceBin, ScatterAggregateEvidenceContext, ScatterEvidenceViewV3,
    ScatterSelectionEvidenceV3, SCATTER_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
};
pub use scatter_selection_export::{
    scatter_selection_evidence_json, scatter_selection_evidence_markdown,
    scatter_selection_evidence_v2_json, scatter_selection_evidence_v2_markdown,
    SCATTER_SELECTION_EVIDENCE_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_SCHEMA_VERSION,
    SCATTER_SELECTION_EVIDENCE_V2_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
};
pub use scatter_selection_export_v3::{
    scatter_aggregate_evidence_context, scatter_selection_evidence_v3_json,
    scatter_selection_evidence_v3_markdown, ScatterSelectionExportError,
    SCATTER_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
};
pub use scatter_viewport::ScatterViewport;
pub use selection_comparison::{
    missingness_selection_comparison, scatter_selection_comparison,
    scatter_selection_comparison_masked, timeline_selection_comparison, ComparisonRatio,
    MissingnessSelectionComparison, ScatterKindComparison, ScatterSelectionComparison,
    TimelineKindComparison, TimelineSelectionComparison,
};
pub use selection_drilldown::{
    scatter_selection_drilldown, scatter_selection_drilldown_masked, timeline_selection_drilldown,
    DrilldownColumn, DrilldownConfig, DrilldownRow, SelectionDrilldown,
};
pub use timeline_brush::{
    SelectedEventTypeCounts, TimelineBrushDrag, TimelineBrushSelection, TimelineLaneRange,
    TimelineSelectionSummary,
};
pub use timeline_density_renderer::{
    TimelineDensityRenderStats, TimelineDensityRenderer, TimelineDensityRendererConfig,
};
pub use timeline_selection_evidence::{
    SelectedTimelineEventSample, SelectedTimelineEventSampleV2, TimelineEvidenceConfig,
    TimelineEvidenceView, TimelineSelectionEvidence, TimelineSelectionEvidenceV2,
};
pub use timeline_selection_evidence_v3::{
    TimelineAggregateEvidenceContext, TimelineEvidenceViewV3, TimelineSelectionEvidenceV3,
    TIMELINE_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
};
pub use timeline_selection_export::{
    timeline_selection_evidence_json, timeline_selection_evidence_markdown,
    timeline_selection_evidence_v2_json, timeline_selection_evidence_v2_markdown,
    TIMELINE_SELECTION_EVIDENCE_ARTIFACT_KIND, TIMELINE_SELECTION_EVIDENCE_SCHEMA_VERSION,
    TIMELINE_SELECTION_EVIDENCE_V2_ARTIFACT_KIND, TIMELINE_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
};
pub use timeline_selection_export_v3::{
    timeline_aggregate_evidence_context, timeline_selection_evidence_v3_json,
    timeline_selection_evidence_v3_markdown, TimelineSelectionExportError,
    TIMELINE_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
};
pub use timeline_viewport::TimelineViewport;
pub use view_axes::{
    scatter_axes_context, scatter_axes_context_with_options, timeline_axes_context, AxisTick,
    AxisValueFormat, NumericAxisContext, ScatterAxesContext, ScatterAxesOptions,
    ScatterReferenceGuide, ScatterReferenceGuideKind, ScatterReferenceGuideSegment,
    TimelineAxesContext, TimelineLaneLabel,
};
pub use view_summaries::{
    scatter_marginal_summary, scatter_marginal_summary_masked, timeline_marginal_summary,
    timeline_overview_summary, ScatterMarginalSummary, SummaryBin, TimelineMarginalSummary,
    TimelineOverviewSummary, TimelineOverviewWindow,
};
