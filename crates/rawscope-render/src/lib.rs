//! CPU and early GPU correctness density renderers for RawScope.

mod aggregate_cache;
mod dataset_diff;
mod density_encoding;
mod density_reference;
mod density_render_pipeline;
mod difference_density;
mod difference_inspection;
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
mod scatter_difference_renderer;
mod scatter_inspection;
mod scatter_inspection_overlay;
mod scatter_point_renderer;
mod scatter_point_reveal;
mod scatter_relief;
mod scatter_resident;
mod scatter_selection_evidence_v3;
mod scatter_selection_evidence_v4;
mod scatter_selection_evidence_v5;
mod scatter_selection_export_v3;
mod scatter_selection_export_v4;
mod scatter_selection_export_v5;
mod scatter_transition;
mod scatter_viewport;
mod selection_comparison;
mod selection_drilldown;
mod timeline_brush;
mod timeline_density_renderer;
mod timeline_resident;
mod timeline_selection_evidence;
mod timeline_selection_evidence_v3;
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
    DifferenceInspection, DifferencePalette, ScatterDensityMode, StableShareParts,
    DIFFERENCE_FIXED_POINT_SCALE,
};
pub use difference_inspection::{
    build_difference_inspection_distribution, DifferenceDirection,
    DifferenceInspectionDistribution, DifferenceInspectionSummary,
};
pub use gpu_scatter_density::{
    gpu_scatter_density, gpu_scatter_density_masked, gpu_scatter_density_on_device,
    GpuScatterDensityError, GpuScatterDensityGrid,
};
pub use gpu_scatter_density_pack::{
    pack_scatter_points, GpuQuantization, GpuQuantizationDisclosure, PackedScatterPoint,
    VisualPackingError,
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
pub use rawscope_evidence::{
    SelectedEventTypeCounts, SelectedTimelineEventSample, SelectedTimelineEventSampleV2,
    TimelineEvidenceConfig, TimelineEvidenceView, TimelineLaneRange, TimelineSelectionEvidence,
    TimelineSelectionEvidenceV2,
};
pub use scatter_brush::{
    selected_region_summary_masked, selected_region_summary_snapshot, BrushScreenPoint,
    BrushScreenRect, BrushScreenSize, ScatterBrushDrag, ScatterBrushSelection,
    SelectedCategoryCounts, SelectedRegionSummary,
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
pub use scatter_resident::{
    ScatterDatasetGpuResources, ScatterFieldGeneration, ScatterViewGeneration,
    ScatterViewGenerationCounter,
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
pub use scatter_selection_export_v3::{
    scatter_aggregate_evidence_context, scatter_aggregate_evidence_context_for_bins,
    scatter_selection_evidence_v3_json, scatter_selection_evidence_v3_markdown,
    ScatterSelectionExportError, SCATTER_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
};
pub use scatter_selection_export_v4::{
    scatter_selection_evidence_v4_json, scatter_selection_evidence_v4_markdown,
    SCATTER_SELECTION_EVIDENCE_V4_ARTIFACT_KIND,
};
pub use scatter_selection_export_v5::{
    scatter_selection_evidence_v5_json, scatter_selection_evidence_v5_markdown,
    SCATTER_SELECTION_EVIDENCE_V5_ARTIFACT_KIND,
};
pub use scatter_transition::{transition_decision, ScatterTransitionField, TransitionDecision};
pub use scatter_viewport::ScatterViewport;
pub use selection_comparison::{
    missingness_selection_comparison, scatter_selection_comparison,
    scatter_selection_comparison_masked, timeline_selection_comparison, ComparisonRatio,
    MissingnessSelectionComparison, ScatterKindComparison, ScatterSelectionComparison,
    TimelineKindComparison, TimelineSelectionComparison,
};
pub use selection_drilldown::{
    scatter_selection_drilldown, scatter_selection_drilldown_masked,
    scatter_selection_drilldown_snapshot, timeline_selection_drilldown,
    timeline_selection_drilldown_snapshot, DrilldownColumn, DrilldownConfig, DrilldownRow,
    SelectionDrilldown,
};
pub use timeline_brush::{TimelineBrushDrag, TimelineBrushSelection, TimelineSelectionSummary};
pub use timeline_density_renderer::{
    TimelineDensityRenderStats, TimelineDensityRenderer, TimelineDensityRendererConfig,
};
pub use timeline_resident::{
    PendingTimelineField, TimelineFieldGeneration, TimelineFieldState, TimelineResidentError,
    TimelineResidentState, TimelineUpdateQuality, TimelineUpdateRequest,
};
pub use timeline_selection_evidence::timeline_selection_evidence_from_events;
pub use timeline_selection_evidence_v3::{
    TimelineAggregateEvidenceContext, TimelineEvidenceViewV3, TimelineSelectionEvidenceV3,
    TIMELINE_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
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
