//! Render-side representation of exact mass contour thresholds.
//!
//! The mass-contour algorithm belongs to `rawscope-analysis`.  This module only
//! adapts its settled result into the compact, fixed-layout uniform consumed by
//! the scatter shader.  Keeping this adapter render-owned prevents the shader
//! from deriving labels or thresholds from display intensity.

use bytemuck::{Pod, Zeroable};
use rawscope_analysis::visual_field::{MassContourSet, SettledDensityContext};

/// Number of exact mass contour levels currently carried by the shader.
pub const MAX_MASS_CONTOUR_LEVELS: usize = 4;

/// Fixed-layout count thresholds for one settled field.
///
/// Thresholds are raw bin counts, not normalized intensities.  A zero count is
/// reserved for an unused slot, so empty fields produce no drawable contours.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct MassContourUniforms {
    thresholds: [u32; MAX_MASS_CONTOUR_LEVELS],
    threshold_count: u32,
    padding: [u32; 3],
}

impl MassContourUniforms {
    /// Returns an empty context that cannot draw a contour over zero-count bins.
    pub const fn empty() -> Self {
        Self {
            thresholds: [0; MAX_MASS_CONTOUR_LEVELS],
            threshold_count: 0,
            padding: [0; 3],
        }
    }

    /// Adapts one analysis-owned settled mass-contour set.
    pub fn from_mass_contours(contours: &MassContourSet) -> Self {
        let mut uniforms = Self::empty();
        for level in contours.levels.iter() {
            let Some(threshold) = level.minimum_bin_count else {
                continue;
            };
            if uniforms.threshold_count as usize >= MAX_MASS_CONTOUR_LEVELS {
                break;
            }
            uniforms.thresholds[uniforms.threshold_count as usize] = threshold.get();
            uniforms.threshold_count += 1;
        }
        uniforms
    }

    /// Adapts all settled context metadata without copying its count grid.
    pub fn from_settled_context<G>(context: &SettledDensityContext<G>) -> Self {
        Self::from_mass_contours(&context.contours)
    }

    /// Creates a uniform from already validated raw count thresholds.
    ///
    /// This is useful for render tests and for callers that have a settled
    /// `MassContourSet` from another process boundary.  The analysis adapter is
    /// preferred for normal application paths.
    pub fn from_thresholds(thresholds: &[u32]) -> Self {
        let mut uniforms = Self::empty();
        for &threshold in thresholds.iter().take(MAX_MASS_CONTOUR_LEVELS) {
            if threshold == 0 {
                continue;
            }
            uniforms.thresholds[uniforms.threshold_count as usize] = threshold;
            uniforms.threshold_count += 1;
        }
        uniforms
    }

    pub const fn thresholds(self) -> [u32; MAX_MASS_CONTOUR_LEVELS] {
        self.thresholds
    }

    pub const fn threshold_count(self) -> u32 {
        self.threshold_count
    }
}

impl Default for MassContourUniforms {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rawscope_analysis::visual_field::{MassContourSet, MassFractionBasisPoints};
    use rawscope_core::{DensityCountGrid, GridSize};

    use super::*;

    #[test]
    fn empty_mass_contours_do_not_draw_zero_count_cells() {
        let uniforms = MassContourUniforms::empty();

        assert_eq!(uniforms.threshold_count(), 0);
        assert_eq!(uniforms.thresholds(), [0; MAX_MASS_CONTOUR_LEVELS]);
    }

    #[test]
    fn settled_context_adapter_carries_only_nonzero_tied_thresholds() {
        let grid = DensityCountGrid::new(GridSize::new(2, 2), vec![4, 4, 2, 0]);
        let contours = MassContourSet::from_grid(
            &grid,
            &[
                MassFractionBasisPoints::try_new(5_000).unwrap(),
                MassFractionBasisPoints::try_new(9_500).unwrap(),
            ],
        )
        .unwrap();
        let context = SettledDensityContext::new(7_u64, Arc::new(grid), contours).unwrap();
        let uniforms = MassContourUniforms::from_settled_context(&context);

        assert_eq!(uniforms.threshold_count(), 2);
        assert_eq!(uniforms.thresholds()[0], 4);
        assert_eq!(uniforms.thresholds()[1], 2);
    }
}
