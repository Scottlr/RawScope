//! Deterministic synthetic datasets used for CPU reference rendering tests.

mod event;
mod point;
mod rng;

pub use event::{
    generate_synthetic_events, SyntheticEventConfig, SyntheticEventDataset, SyntheticEventType,
};
pub use point::{
    generate_synthetic_points, SyntheticPointCategory, SyntheticPointConfig, SyntheticPointDataset,
};
