//! CPU and early GPU correctness density renderers for RawScope.

mod aggregate_cache;
mod dataset_diff;
mod density_reference;
mod density_render_pipeline;
mod difference_density;
mod difference_inspection;
mod gpu_density_pipeline;
mod gpu_timeline_density;
mod gpu_timeline_density_pack;
mod mask_alignment;
mod missingness_reference;
mod plot_geometry;
mod scatter_brush;
mod scatter_brush_overlay;
mod scatter_inspection;
mod scatter_inspection_overlay;
mod scatter_point_renderer;
mod scatter_point_reveal;
mod scatter_selection_export_v3_helpers;
mod scatter_viewport;
mod selection_comparison;
mod selection_drilldown;
mod timeline_brush;
mod timeline_density_renderer;
mod timeline_resident;
mod timeline_selection_evidence;
mod timeline_selection_export_v3_helpers;
mod timeline_viewport;
mod view_axes;
mod view_summaries;
mod visual_field;
mod visual_transition;

/// Settled count context bound to the renderer's view-generation identity.
///
/// The analysis contract remains generic so it can be used without a renderer;
/// this facade alias is the concrete context that workbench consumers share
/// with GPU presentation and inspection.
pub type SettledDensityContext<G = visual_field::VisualFieldViewGeneration> =
    rawscope_analysis::visual_field::SettledDensityContext<G>;

pub use aggregate_cache::{
    scatter_aggregate_overview, timeline_aggregate_overview, AggregateBinSample,
    AggregateCacheConfig, AggregateCacheError, ScatterAggregateOverview, TimelineAggregateOverview,
};
pub use dataset_diff::{
    dataset_diff_summary, DatasetDiffColumn, DatasetDiffColumnStatus, DatasetDiffMissingnessDelta,
    DatasetDiffSummary,
};
pub use density_reference::{scatter_density, timeline_density};
pub use difference_density::{
    difference_inspection, fixed_point_max_abs_delta, normalized_difference_density,
    DifferenceDensityConfig, DifferenceDensityError, DifferenceDensityGrid, DifferenceDensityStats,
    DifferenceInspection, DifferencePalette, StableShareParts, DIFFERENCE_FIXED_POINT_SCALE,
};
pub use difference_inspection::{
    build_difference_inspection_distribution, DifferenceDirection,
    DifferenceInspectionDistribution, DifferenceInspectionSummary,
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
    density_intensity, DensityEncoding, DensityNormalization, DensityPalette, DensityTransform,
    ScatterDensityPresentation,
};
pub use rawscope_evidence::{
    scatter_selection_evidence_v3_json, scatter_selection_evidence_v3_markdown,
    ScatterSelectionExportError, SCATTER_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
};
pub use rawscope_evidence::{
    scatter_selection_evidence_v4_json, scatter_selection_evidence_v4_markdown,
    scatter_selection_evidence_v5_json, scatter_selection_evidence_v5_markdown,
    validate_relief_field_config, DifferenceDensityEvidenceConfig, DifferenceDirectionEvidenceV5,
    PinnedDifferenceInspectionEvidenceV5, PinnedScatterInspectionEvidence,
    PinnedScatterInspectionEvidenceV5, PointRevealEvidence, PointRevealMode, ReliefFieldConfig,
    ReliefFieldConfigError, ScatterCohortEvidence, ScatterDensityMode, ScatterSelectionEvidenceV4,
    ScatterSelectionEvidenceV4Error, ScatterSelectionEvidenceV5, ScatterSelectionEvidenceV5Error,
    ScatterVisualQueryV4, SessionDataFormatEvidenceV5, SessionEvidenceContextV5,
    DIFFERENCE_BASELINE_ID, DIFFERENCE_FORMULA_ID, INSPECTION_NEIGHBORHOOD_RADIUS_BINS,
    MAX_RELIEF_ELEVATION_DEGREES, MAX_RELIEF_HEIGHT_STRENGTH, MAX_RELIEF_NORMAL_RADIUS_BINS,
    MIN_RELIEF_ELEVATION_DEGREES, MIN_RELIEF_HEIGHT_STRENGTH, MIN_RELIEF_NORMAL_RADIUS_BINS,
    SCATTER_SELECTION_EVIDENCE_V4_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_V4_SCHEMA_VERSION,
    SCATTER_SELECTION_EVIDENCE_V5_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_V5_SCHEMA_VERSION,
};
pub use rawscope_evidence::{
    timeline_selection_evidence_v3_json, timeline_selection_evidence_v3_markdown,
    TimelineSelectionExportError, TIMELINE_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
};
pub use rawscope_evidence::{
    AggregateEvidenceBin, ScatterAggregateEvidenceContext, TimelineAggregateEvidenceContext,
};
pub use rawscope_evidence::{
    ComparisonRatio, ScatterKindComparison, ScatterSelectionComparison, TimelineKindComparison,
    TimelineSelectionComparison,
};
pub use rawscope_evidence::{
    ScatterEvidenceViewV3, ScatterSelectionEvidenceV3, SCATTER_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
};
pub use rawscope_evidence::{
    SelectedEventTypeCounts, SelectedTimelineEventSample, SelectedTimelineEventSampleV2,
    TimelineEvidenceConfig, TimelineEvidenceView, TimelineLaneRange, TimelineSelectionEvidence,
    TimelineSelectionEvidenceV2,
};
pub use rawscope_evidence::{
    TimelineEvidenceViewV3, TimelineSelectionEvidenceV3,
    TIMELINE_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
};
pub use scatter_brush::{
    selected_region_summary_masked, selected_region_summary_snapshot, BrushScreenPoint,
    BrushScreenRect, BrushScreenSize, ScatterBrushDrag, ScatterBrushSelection,
    SelectedCategoryCounts, SelectedRegionSummary,
};
pub use scatter_brush_overlay::ScatterBrushOverlayRenderer;
pub use scatter_inspection::{
    build_scatter_inspection_grid, ScatterInspectionBin, ScatterInspectionConfig,
    ScatterInspectionDistribution, ScatterInspectionError, ScatterInspectionGrid,
    ScatterInspectionHit, ScatterInspectionSummary,
};
pub use scatter_inspection_overlay::{
    project_scatter_inspection_overlay, InspectionFocusKind, ScatterInspectionOverlay,
    ScatterInspectionOverlayRenderer,
};
pub use scatter_point_renderer::{PointRevealPlanRenderError, ScatterPointRenderer};
pub use scatter_point_reveal::{
    project_point_to_plot_fraction, select_points_for_reveal, PointRevealConfig, PointRevealError,
    PointRevealSelection, PointRevealStats,
};
pub use scatter_selection_export_v3_helpers::{
    scatter_aggregate_evidence_context, scatter_aggregate_evidence_context_for_bins,
};
pub use scatter_viewport::ScatterViewport;
pub use selection_comparison::{
    missingness_selection_comparison, scatter_selection_comparison,
    scatter_selection_comparison_masked, timeline_selection_comparison,
    MissingnessSelectionComparison,
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
pub use timeline_selection_export_v3_helpers::timeline_aggregate_evidence_context;
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
pub use visual_field::composition::{
    clear_pending_layer_counts, composition_bind_group, composition_bind_group_layout,
    composition_compute_pipeline, composition_reference, composition_reference_layers,
    publication_generation_matches, CategoryChannelGpuResources, CategoryChannelIdentity,
    CategoryCompositionFieldGeneration, CategoryCompositionReference,
    CategoryLayerPlanGpuResources, CompositionParams, CompositionPublicationError,
    CompositionPublicationIdentity, CompositionReferenceError, CompositionResourceError,
    SpecialLayerParams, NO_CATEGORY_LAYER,
};
pub use visual_field::PointRevealPresentationFrame;
pub use visual_field::{
    aggregate_visual_field_resources, choose_visual_resolution,
    choose_visual_resolution_for_quality, estimate_visual_field_resources,
    ResolutionDecisionReason, VisualFieldBudget, VisualFieldIntent, VisualFieldResourceEstimate,
    VisualFieldResourceEstimateError, VisualFieldResourceOptions, VisualPlotSizePx,
    VisualResolutionDecision, VisualResolutionError, VisualResolutionPolicy,
    VisualResolutionPolicyError, VisualResolutionTier, DEFAULT_BIND_PIPELINE_BYTES,
    DEFAULT_MAX_EXACT_BINS, DEFAULT_TARGET_PHYSICAL_PIXELS_PER_BIN,
};
pub use visual_field::{
    pack_visual_points, relief_normal_from_samples, GpuQuantization, GpuQuantizationDisclosure,
    PackedVisualPoint, VisualFieldPoint, VisualPackingError,
};
pub use visual_field::{
    transition_decision, ComparisonFieldRenderStats, ComparisonFieldRenderer, DensityPresentation,
    DensityPresentationConfig, DensityPresentationRenderStats, DensityReadbackPolicy,
    MassContourUniforms, ResidentExactField, ResidentExactFieldUpdate, TransitionDecision,
    VisualFieldDatasetGpuResources, VisualFieldGeneration, VisualFieldQuality,
    VisualFieldReprojection, VisualFieldReprojectionError, VisualFieldTransitionField,
    VisualFieldViewGeneration, VisualFieldViewGenerationCounter, VisualFieldViewport,
};
pub use visual_field::{
    visual_field_density, visual_field_density_masked, visual_field_density_on_device,
    VisualFieldCountGrid, VisualFieldGpuError,
};
pub use visual_field::{
    CategoricalPalette, CategoryPaletteEntries, ContinuousPalette, ContinuousPaletteLut,
    PaletteGpuResources, CONTINUOUS_PALETTE_ROW_COUNT, MAX_MASS_CONTOUR_LEVELS, PALETTE_LUT_BYTES,
    PALETTE_LUT_SIZE,
};
pub use visual_transition::{
    ease_out_cubic, semantic_color_crossfade, transition_progress, validate_transition_config,
    TransitionKind, TransitionProgress, VisualTransitionConfig, VisualTransitionConfigError,
    MAX_VISUAL_TRANSITION_DURATION_MS,
};
