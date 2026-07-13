//! Stable comparison presentation statistics and contract.

use rawscope_analysis::visual_field::{
    summarize_difference_cell, ComparisonError, DensityMarginals, DifferenceCellContext,
    SettledDensityContext,
};

/// Statistics disclosed by the comparison visual field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComparisonFieldRenderStats {
    pub baseline_total: u64,
    pub active_total: u64,
    pub baseline_recompute_count: u64,
}

/// Closed comparison presentations supported by the generic visual-field owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonPresentation {
    SignedDifference,
    VerticalSplit,
}

/// Plot-space location of the vertical comparison split.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComparisonSplit {
    fraction: f32,
}

impl ComparisonSplit {
    pub fn new(fraction: f32) -> Result<Self, ComparisonPresentationError> {
        if !fraction.is_finite() {
            return Err(ComparisonPresentationError::NonFiniteFraction);
        }
        if !(0.0..=1.0).contains(&fraction) {
            return Err(ComparisonPresentationError::FractionOutOfRange { fraction });
        }
        Ok(Self { fraction })
    }

    pub const fn fraction(self) -> f32 {
        self.fraction
    }
}

impl Default for ComparisonSplit {
    fn default() -> Self {
        Self { fraction: 0.5 }
    }
}

/// Failure while validating a comparison presentation control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComparisonPresentationError {
    NonFiniteFraction,
    FractionOutOfRange { fraction: f32 },
}

impl std::fmt::Display for ComparisonPresentationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteFraction => {
                formatter.write_str("comparison split fraction must be finite")
            }
            Self::FractionOutOfRange { fraction } => write!(
                formatter,
                "comparison split fraction {fraction} must be between 0 and 1"
            ),
        }
    }
}

impl std::error::Error for ComparisonPresentationError {}

/// Support-aware presentation values for one exact comparison cell.
///
/// `normalized_delta` is the signed comparison value. `support_visibility` is
/// a presentation aid only; it does not alter the reported delta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportAwareDifference {
    pub normalized_delta: f64,
    pub support_visibility: f64,
}

impl SupportAwareDifference {
    pub fn from_context(
        context: DifferenceCellContext,
        max_abs_delta: f64,
        max_support_share: f64,
    ) -> Self {
        let normalized_delta = if max_abs_delta.is_finite() && max_abs_delta > 0.0 {
            (context.signed_delta / max_abs_delta).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let support_visibility = if max_support_share.is_finite() && max_support_share > 0.0 {
            (context.support_share / max_support_share).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self {
            normalized_delta,
            support_visibility,
        }
    }
}

/// Exact baseline/active marginals using one shared display scale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonMarginals {
    pub baseline: DensityMarginals,
    pub active: DensityMarginals,
    pub shared_max_x_count: u64,
    pub shared_max_y_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonMarginalError {
    GridMismatch,
}

impl std::fmt::Display for ComparisonMarginalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("comparison marginals require identically sized grids")
    }
}

impl std::error::Error for ComparisonMarginalError {}

pub fn derive_comparison_marginals<G1, G2>(
    baseline: &SettledDensityContext<G1>,
    active: &SettledDensityContext<G2>,
) -> Result<ComparisonMarginals, ComparisonMarginalError> {
    if baseline.counts.size() != active.counts.size() {
        return Err(ComparisonMarginalError::GridMismatch);
    }
    let shared_max_x_count = baseline
        .marginals
        .max_x_count
        .max(active.marginals.max_x_count);
    let shared_max_y_count = baseline
        .marginals
        .max_y_count
        .max(active.marginals.max_y_count);
    Ok(ComparisonMarginals {
        baseline: baseline.marginals.clone(),
        active: active.marginals.clone(),
        shared_max_x_count,
        shared_max_y_count,
    })
}

/// Exact cell inspection from the paired settled count context.
pub fn comparison_inspection(
    baseline_count: u32,
    active_count: u32,
    baseline_total: u64,
    active_total: u64,
) -> Result<DifferenceCellContext, ComparisonError> {
    summarize_difference_cell(baseline_count, baseline_total, active_count, active_total)
}

/// Returns the shared maximum used for both split density sides.
pub const fn shared_density_maximum(baseline_max_count: u32, active_max_count: u32) -> u32 {
    if baseline_max_count > active_max_count {
        baseline_max_count
    } else {
        active_max_count
    }
}

/// Samples two identically encoded fields using one plot-space split fraction.
/// Pointer motion can therefore update a uniform without rebuilding either
/// published field.
pub const fn sample_split_fields<T: Copy>(
    baseline: T,
    active: T,
    plot_x_fraction: f32,
    split: ComparisonSplit,
) -> T {
    if plot_x_fraction < split.fraction() {
        baseline
    } else {
        active
    }
}

use crate::PlotRectPx;

use super::compute::ComparisonFieldRenderer;
use super::resources::difference_bind_group;

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    renderer: &ComparisonFieldRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    target_view: &wgpu::TextureView,
    plot_rect: PlotRectPx,
    clear: bool,
    opacity: f32,
) {
    let params = renderer.params();
    queue.write_buffer(&renderer.params_buffer, 0, bytemuck::bytes_of(&params));
    let bind_group = difference_bind_group(
        device,
        &renderer.render_layout,
        renderer
            .baseline
            .count_buffer(renderer.baseline.active_count_buffer_index()),
        renderer
            .active
            .count_buffer(renderer.active.active_count_buffer_index()),
        &renderer.max_abs_buffer,
        &renderer.params_buffer,
        Some(&renderer.palette),
        "RawScope Difference Render Bind Group",
    );
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("RawScope Difference Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target_view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: if clear {
                    wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.02,
                        g: 0.024,
                        b: 0.03,
                        a: 1.0,
                    })
                } else {
                    wgpu::LoadOp::Load
                },
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(&renderer.render_pipeline);
    let opacity = f64::from(opacity.clamp(0.0, 1.0));
    pass.set_blend_constant(wgpu::Color {
        r: opacity,
        g: opacity,
        b: opacity,
        a: opacity,
    });
    pass.set_bind_group(0, &bind_group, &[]);
    pass.set_viewport(
        plot_rect.x as f32,
        plot_rect.y as f32,
        plot_rect.width as f32,
        plot_rect.height as f32,
        0.0,
        1.0,
    );
    pass.set_scissor_rect(plot_rect.x, plot_rect.y, plot_rect.width, plot_rect.height);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_samples_identically_encoded_fields() {
        let split = ComparisonSplit::new(0.5).unwrap();
        assert_eq!(sample_split_fields(10_u32, 20_u32, 0.25, split), 10);
        assert_eq!(sample_split_fields(10_u32, 20_u32, 0.5, split), 20);
        assert_eq!(sample_split_fields(10_u32, 20_u32, 0.75, split), 20);
    }

    #[test]
    fn split_density_uses_one_shared_maximum() {
        assert_eq!(shared_density_maximum(7, 11), 11);
        assert_eq!(shared_density_maximum(13, 5), 13);
    }

    #[test]
    fn split_drag_updates_uniform_without_scheduling_generation() {
        let before = ComparisonSplit::new(0.2).unwrap();
        let after = ComparisonSplit::new(0.8).unwrap();
        assert_ne!(before, after);
        assert_eq!(after.fraction(), 0.8);
    }

    #[test]
    fn support_visibility_does_not_change_reported_delta() {
        let context = comparison_inspection(20, 30, 100, 100).unwrap();
        let sparse = SupportAwareDifference::from_context(context, 1.0, 1.0);
        let denser_context = comparison_inspection(40, 50, 100, 100).unwrap();
        let denser = SupportAwareDifference::from_context(denser_context, 1.0, 1.0);
        assert_eq!(sparse.normalized_delta, denser.normalized_delta);
        assert!(sparse.support_visibility < denser.support_visibility);
    }

    #[test]
    fn comparison_inspection_reconstructs_delta_from_exact_counts() {
        let inspection = comparison_inspection(25, 20, 100, 40).unwrap();
        assert_eq!(inspection.baseline_count, 25);
        assert_eq!(inspection.active_count, 20);
        assert!((inspection.baseline_share - 0.25).abs() < 1.0e-12);
        assert!((inspection.active_share - 0.5).abs() < 1.0e-12);
        assert!((inspection.signed_delta - 0.25).abs() < 1.0e-12);
        assert!((inspection.support_share - 0.375).abs() < 1.0e-12);
    }

    #[test]
    fn split_rejects_non_finite_and_out_of_range_fraction() {
        assert_eq!(
            ComparisonSplit::new(f32::NAN),
            Err(ComparisonPresentationError::NonFiniteFraction)
        );
        assert!(matches!(
            ComparisonSplit::new(-0.1),
            Err(ComparisonPresentationError::FractionOutOfRange { .. })
        ));
        assert!(matches!(
            ComparisonSplit::new(1.1),
            Err(ComparisonPresentationError::FractionOutOfRange { .. })
        ));
    }
}
