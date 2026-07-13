//! Concrete owners for continuous two-dimensional visual fields.

mod dataset_resources;
mod exact_field;
mod generation;
mod gpu;
mod mass_contour;
mod palette;
mod point_pack;
mod point_presentation;
mod reprojection;
mod resolution;
mod resource_estimate;
mod ridges;

pub mod comparison;
pub mod composition;
pub mod density_presentation;

pub use dataset_resources::VisualFieldDatasetGpuResources;
pub use exact_field::{DensityReadbackPolicy, ResidentExactField, ResidentExactFieldUpdate};
pub use generation::{
    transition_decision, TransitionDecision, VisualFieldGeneration, VisualFieldQuality,
    VisualFieldTransitionField, VisualFieldViewGeneration, VisualFieldViewGenerationCounter,
};
pub use gpu::{
    visual_field_density, visual_field_density_masked, visual_field_density_on_device,
    VisualFieldCountGrid, VisualFieldGpuError,
};
pub use mass_contour::{MassContourUniforms, MAX_MASS_CONTOUR_LEVELS};
pub use reprojection::{
    VisualFieldReprojection, VisualFieldReprojectionError, VisualFieldViewport,
};
pub use ridges::{
    encode_ridge_field, ridge_compute_bind_group, ridge_compute_bind_group_layout,
    ridge_compute_pipeline, ridge_dispatch, ridge_presentation_availability,
    ridge_render_bind_group, ridge_render_bind_group_layout, ridge_render_pipeline,
    transform_ridge_tangent, validate_ridge_identity, GpuRidgeCell, RidgeComputePipeline,
    RidgeFieldGeneration, RidgeFieldGpuResources, RidgeGpuParams, RidgeInteraction,
    RidgePresentationAvailability, RidgeRenderParams, RidgeRenderPipeline, RidgeReprojection,
    RidgeResourceError, RidgeResourcePlan,
};

pub use comparison::{
    comparison_inspection, derive_comparison_marginals, sample_split_fields,
    shared_density_maximum, ComparisonCompatibility, ComparisonCompatibilityError,
    ComparisonFieldGeneration, ComparisonFieldRenderStats, ComparisonFieldRenderer,
    ComparisonFieldSemantics, ComparisonMarginalError, ComparisonMarginals, ComparisonPresentation,
    ComparisonPresentationError, ComparisonSplit, SupportAwareDifference,
};
pub use density_presentation::relief_normal_from_samples;
pub use density_presentation::{
    DensityPresentation, DensityPresentationConfig, DensityPresentationRenderStats,
};
pub use palette::{
    CategoricalPalette, CategoryPaletteEntries, ContinuousPalette, ContinuousPaletteLut,
    PaletteGpuResources, CONTINUOUS_PALETTE_ROW_COUNT, PALETTE_LUT_BYTES, PALETTE_LUT_SIZE,
};
pub use point_pack::{
    pack_visual_points, GpuQuantization, GpuQuantizationDisclosure, PackedVisualPoint,
    VisualFieldPoint, VisualPackingError,
};
pub use point_presentation::PointRevealPresentationFrame;
pub use resolution::{
    choose_visual_resolution, choose_visual_resolution_for_quality, ResolutionDecisionReason,
    VisualFieldBudget, VisualFieldIntent, VisualPlotSizePx, VisualResolutionDecision,
    VisualResolutionError, VisualResolutionPolicy, VisualResolutionPolicyError,
    VisualResolutionTier, DEFAULT_MAX_EXACT_BINS, DEFAULT_TARGET_PHYSICAL_PIXELS_PER_BIN,
};
pub use resource_estimate::{
    aggregate_visual_field_resources, estimate_visual_field_resources, VisualFieldResourceEstimate,
    VisualFieldResourceEstimateError, VisualFieldResourceOptions, DEFAULT_BIND_PIPELINE_BYTES,
};
