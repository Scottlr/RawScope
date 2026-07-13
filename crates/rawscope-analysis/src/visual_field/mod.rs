//! Generic, profile-independent visual-field mapping and projection contracts.

pub mod encoding;
mod mapping;
pub mod mass_context;
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
pub use projection::{
    ProjectedVisualFieldGeneration, ProjectedVisualPoint, VisualAxisDomain,
    VisualFieldProjectionError, VisualFieldRowPolicy,
};
