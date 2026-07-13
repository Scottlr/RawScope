//! Generic, profile-independent visual-field mapping and projection contracts.

pub mod category_layers;
pub mod comparison;
pub mod encoding;
mod mapping;
pub mod mass_context;
pub mod point_reveal;
mod projection;
pub mod ridges;

pub use category_layers::{
    CategoryLayer, CategoryLayerId, CategoryLayerKind, CategoryLayerLookup, CategoryLayerPlan,
    CategoryLayerPlanError, CategoryLayerPlanGeneration, CategoryLayerPlanGenerationCounter,
};
pub use comparison::{summarize_difference_cell, ComparisonError, DifferenceCellContext};
pub use encoding::{DensityEncoding, DensityNormalization, DensityTransform};
pub use mapping::{
    VisualAxisKind, VisualFieldMapping, VisualFieldMappingError, VisualFieldMode,
    VisualFieldModeSupport, VisualFieldProjection, MAX_CATEGORY_COMPOSITION_LAYERS,
};
pub use mass_context::{
    DensityMarginals, MassContextError, MassContourLevel, MassContourSet, MassFractionBasisPoints,
    MassFractionError, SettledDensityContext, DEFAULT_MASS_FRACTIONS, MASS_FRACTION_DENOMINATOR,
};
pub use point_reveal::{
    build_point_reveal_plan, build_point_reveal_plan_from_points, semantic_zoom_frame,
    PointRevealPlan, PointRevealPlanError, PointRevealViewport, SemanticZoomFrame,
    SemanticZoomPolicy, SemanticZoomPolicyError, VisualFieldViewGeneration,
    VisualFieldViewGenerationCounter,
};
pub use projection::{
    ProjectedVisualFieldGeneration, ProjectedVisualPoint, VisualAxisDomain,
    VisualFieldProjectionError, VisualFieldRowPolicy,
};
pub use ridges::{
    derive_density_ridges, DensityRidgeField, RidgeCell, RidgeConfig, RidgeError, RidgeScale,
    DEFAULT_RIDGE_MINIMUM_ANISOTROPY_BASIS_POINTS, DEFAULT_RIDGE_MINIMUM_STRENGTH_BASIS_POINTS,
    RIDGE_BASIS_POINTS_DENOMINATOR, RIDGE_EIGENGAP_EPSILON, RIDGE_FORMULA_VERSION,
    RIDGE_NORMALIZATION_EPSILON,
};
