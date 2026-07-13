//! Generic, profile-independent visual-field mapping and projection contracts.

pub mod encoding;
mod mapping;
mod projection;

pub use encoding::{DensityEncoding, DensityNormalization, DensityTransform};
pub use mapping::{
    VisualAxisKind, VisualFieldMapping, VisualFieldMappingError, VisualFieldMode,
    VisualFieldModeSupport, VisualFieldProjection, MAX_CATEGORY_COMPOSITION_LAYERS,
};
pub use projection::{
    ProjectedVisualFieldGeneration, ProjectedVisualPoint, VisualAxisDomain,
    VisualFieldProjectionError, VisualFieldRowPolicy,
};
