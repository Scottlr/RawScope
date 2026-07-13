//! Generic, profile-independent visual-field mapping and projection contracts.

mod mapping;
mod projection;

pub use mapping::{
    VisualAxisKind, VisualFieldMapping, VisualFieldMappingError, VisualFieldMode,
    VisualFieldModeSupport, VisualFieldProjection, MAX_CATEGORY_COMPOSITION_LAYERS,
};
pub use projection::{
    ProjectedVisualFieldGeneration, ProjectedVisualPoint, VisualAxisDomain,
    VisualFieldProjectionError, VisualFieldRowPolicy,
};
