//! Concrete owners for continuous two-dimensional visual fields.

mod dataset_resources;
mod exact_field;
mod generation;
mod gpu;
mod point_pack;
mod reprojection;

pub mod comparison;
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
pub use reprojection::{
    VisualFieldReprojection, VisualFieldReprojectionError, VisualFieldViewport,
};

pub use comparison::{ComparisonFieldRenderStats, ComparisonFieldRenderer};
pub use density_presentation::relief_normal_from_samples;
pub use density_presentation::{
    DensityPresentation, DensityPresentationConfig, DensityPresentationRenderStats,
};
pub use point_pack::{
    pack_visual_points, GpuQuantization, GpuQuantizationDisclosure, PackedVisualPoint,
    VisualFieldPoint, VisualPackingError,
};
