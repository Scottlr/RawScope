//! CPU and early GPU correctness density renderers for RawScope.

mod density_reference;
mod gpu_scatter_density;

pub use density_reference::{scatter_density, timeline_density};
pub use gpu_scatter_density::{gpu_scatter_density, GpuScatterDensityError, GpuScatterDensityGrid};

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "CPU reference density renderers and correctness-first GPU scatter-density compute"
}
