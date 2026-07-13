//! Generic, profile-independent visual-field mapping and projection contracts.

pub mod encoding;
mod mapping;
pub mod mass_context;
pub mod point_reveal;
mod projection;

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
