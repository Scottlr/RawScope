//! Versioned session wire owners. Each wire version validates and serializes
//! independently; the parent facade intentionally exposes only their stable
//! public contracts.

mod v1;
mod v2;

pub use v1::*;
pub use v2::*;
