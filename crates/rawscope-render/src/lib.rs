//! CPU reference density renderers for RawScope.

mod density_reference;

pub use density_reference::{scatter_density, timeline_density};

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "CPU reference density renderers and future GPU-backed renderers"
}
