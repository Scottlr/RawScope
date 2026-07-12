//! CPU and early GPU correctness density renderers for RawScope.

mod aggregate_cache;
mod dataset_diff;
mod density_encoding;
mod density_reference;
mod density_render_pipeline;
mod difference_density;
mod difference_inspection;
mod evidence_sample;
mod gpu_density_pipeline;
mod gpu_scatter_density;
mod gpu_scatter_density_pack;
mod gpu_timeline_density;
mod gpu_timeline_density_pack;
mod markdown_escape;
mod mask_alignment;
mod missingness_reference;
mod plot_geometry;
mod scatter_brush;
mod scatter_brush_overlay;
mod scatter_density_gpu_state;
mod scatter_density_presentation;
mod scatter_density_renderer;
mod scatter_density_reprojection;
mod scatter_difference_renderer;
mod scatter_inspection;
mod scatter_inspection_overlay;
mod scatter_point_renderer;
mod scatter_point_reveal;
mod scatter_relief;
mod scatter_selection_evidence;
mod scatter_selection_evidence_v3;
mod scatter_selection_evidence_v4;
mod scatter_selection_evidence_v5;
mod scatter_selection_export;
mod scatter_selection_export_v3;
mod scatter_selection_export_v4;
mod scatter_selection_export_v5;
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
mod visual_transition;

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
pub use difference_density::{
    difference_inspection, fixed_point_max_abs_delta, normalized_difference_density,
    DifferenceDensityConfig, DifferenceDensityError, DifferenceDensityGrid, DifferenceDensityStats,
    DifferenceInspection, DifferencePalette, ScatterDensityMode, DIFFERENCE_FIXED_POINT_SCALE,
};
pub use difference_inspection::{
    build_difference_inspection_distribution, DifferenceDirection,
    DifferenceInspectionDistribution, DifferenceInspectionSummary,
};
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
pub use scatter_difference_renderer::{ScatterDifferenceRenderStats, ScatterDifferenceRenderer};
pub use scatter_inspection::{
    build_scatter_inspection_grid, ScatterInspectionBin, ScatterInspectionConfig,
    ScatterInspectionDistribution, ScatterInspectionError, ScatterInspectionGrid,
    ScatterInspectionHit, ScatterInspectionSummary, INSPECTION_NEIGHBORHOOD_RADIUS_BINS,
};
pub use scatter_inspection_overlay::{
    project_scatter_inspection_overlay, InspectionFocusKind, ScatterInspectionOverlay,
    ScatterInspectionOverlayRenderer,
};
pub use scatter_point_renderer::ScatterPointRenderer;
pub use scatter_point_reveal::{
    project_point_to_plot_fraction, select_points_for_reveal, PointRevealConfig, PointRevealError,
    PointRevealMode, PointRevealSelection, PointRevealStats,
};
pub use scatter_relief::{
    relief_normal_from_samples, validate_relief_field_config, ReliefFieldConfig,
    ReliefFieldConfigError, MAX_RELIEF_ELEVATION_DEGREES, MAX_RELIEF_HEIGHT_STRENGTH,
    MAX_RELIEF_NORMAL_RADIUS_BINS, MIN_RELIEF_ELEVATION_DEGREES, MIN_RELIEF_HEIGHT_STRENGTH,
    MIN_RELIEF_NORMAL_RADIUS_BINS,
};
pub use scatter_selection_evidence::{
    ScatterEvidenceView, ScatterSelectionEvidence, ScatterSelectionEvidenceV2, SelectedPointSample,
    SelectedPointSampleV2, SelectedSourceRowSample, SelectionEvidenceConfig,
};
pub use scatter_selection_evidence_v3::{
    AggregateEvidenceBin, ScatterAggregateEvidenceContext, ScatterEvidenceViewV3,
    ScatterSelectionEvidenceV3, SCATTER_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
};
pub use scatter_selection_evidence_v4::{
    DifferenceDensityEvidenceConfig, PinnedScatterInspectionEvidence, PointRevealEvidence,
    ScatterCohortEvidence, ScatterSelectionEvidenceV4, ScatterSelectionEvidenceV4Error,
    ScatterVisualQueryV4, DIFFERENCE_BASELINE_ID, DIFFERENCE_FORMULA_ID,
    SCATTER_SELECTION_EVIDENCE_V4_SCHEMA_VERSION,
};
pub use scatter_selection_evidence_v5::{
    DifferenceDirectionEvidenceV5, PinnedDifferenceInspectionEvidenceV5,
    PinnedScatterInspectionEvidenceV5, ScatterSelectionEvidenceV5, ScatterSelectionEvidenceV5Error,
    SessionDataFormatEvidenceV5, SessionEvidenceContextV5,
    SCATTER_SELECTION_EVIDENCE_V5_SCHEMA_VERSION,
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
pub use scatter_selection_export_v4::{
    scatter_selection_evidence_v4_json, scatter_selection_evidence_v4_markdown,
    SCATTER_SELECTION_EVIDENCE_V4_ARTIFACT_KIND,
};
pub use scatter_selection_export_v5::{
    scatter_selection_evidence_v5_json, scatter_selection_evidence_v5_markdown,
    SCATTER_SELECTION_EVIDENCE_V5_ARTIFACT_KIND,
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
    format_axis_value, scatter_axes_context, scatter_axes_context_with_options,
    timeline_axes_context, AxisTick, AxisValueFormat, NumericAxisContext, ScatterAxesContext,
    ScatterAxesOptions, ScatterReferenceGuide, ScatterReferenceGuideKind,
    ScatterReferenceGuideSegment, TimelineAxesContext, TimelineLaneLabel,
};
pub use view_summaries::{
    scatter_marginal_summary, scatter_marginal_summary_masked, timeline_marginal_summary,
    timeline_overview_summary, ScatterMarginalSummary, SummaryBin, TimelineMarginalSummary,
    TimelineOverviewSummary, TimelineOverviewWindow,
};
pub use visual_transition::{
    ease_out_cubic, semantic_color_crossfade, transition_progress, validate_transition_config,
    TransitionKind, TransitionProgress, VisualTransitionConfig, VisualTransitionConfigError,
    MAX_VISUAL_TRANSITION_DURATION_MS,
};
