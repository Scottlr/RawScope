//! Future density and timeline rendering algorithms for RawScope.

/// Placeholder marker for future renderer ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DensityRendererMarker;

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "future density, heatmap, and timeline renderers"
}
