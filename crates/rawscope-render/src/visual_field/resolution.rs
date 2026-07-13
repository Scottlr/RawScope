//! Deterministic physical-plot resolution policy for continuous visual fields.

use std::{
    error::Error,
    fmt,
    num::{NonZeroU16, NonZeroU64, NonZeroU8},
};

use rawscope_analysis::{
    cohort::CohortGeneration,
    visual_field::{VisualFieldMapping, VisualFieldMode},
};
use rawscope_core::GridSize;
use rawscope_data::{DatasetGeneration, ResourceBudget};

use super::resource_estimate::VisualFieldResourceEstimate;
use super::{resource_estimate::VisualFieldResourceEstimateError, VisualFieldQuality};
use crate::plot_geometry::PlotRectPx;

pub const DEFAULT_TARGET_PHYSICAL_PIXELS_PER_BIN: NonZeroU8 = NonZeroU8::new(2).unwrap();
pub const DEFAULT_MAX_EXACT_BINS: NonZeroU64 = NonZeroU64::new(4 * 1024 * 1024).unwrap();

const DEFAULT_PREVIEW_TIERS: &[VisualResolutionTier] = &[VisualResolutionTier::const_new(
    VisualFieldQuality::Preview,
    128,
    8,
    65_536,
)];
const DEFAULT_EXACT_TIERS: &[VisualResolutionTier] = &[
    VisualResolutionTier::const_new(VisualFieldQuality::Exact, 256, 8, 262_144),
    VisualResolutionTier::const_new(VisualFieldQuality::Exact, 512, 8, 1_048_576),
    VisualResolutionTier::const_new(VisualFieldQuality::Exact, 1024, 8, 4_194_304),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualResolutionPolicy {
    preview_tiers: &'static [VisualResolutionTier],
    exact_tiers: &'static [VisualResolutionTier],
    target_physical_pixels_per_bin: NonZeroU8,
    max_exact_bins: NonZeroU64,
}

impl VisualResolutionPolicy {
    pub const fn new(
        preview_tiers: &'static [VisualResolutionTier],
        exact_tiers: &'static [VisualResolutionTier],
        target_physical_pixels_per_bin: NonZeroU8,
        max_exact_bins: NonZeroU64,
    ) -> Self {
        Self {
            preview_tiers,
            exact_tiers,
            target_physical_pixels_per_bin,
            max_exact_bins,
        }
    }

    pub fn try_new(
        preview_tiers: &'static [VisualResolutionTier],
        exact_tiers: &'static [VisualResolutionTier],
        target_physical_pixels_per_bin: Option<NonZeroU8>,
        max_exact_bins: Option<NonZeroU64>,
    ) -> Result<Self, VisualResolutionPolicyError> {
        let target_physical_pixels_per_bin =
            target_physical_pixels_per_bin.ok_or(VisualResolutionPolicyError::ZeroPixelsPerBin)?;
        let max_exact_bins = max_exact_bins.ok_or(VisualResolutionPolicyError::ZeroMaxExactBins)?;
        validate_tiers(preview_tiers, VisualFieldQuality::Preview)?;
        validate_tiers(exact_tiers, VisualFieldQuality::Exact)?;
        Ok(Self::new(
            preview_tiers,
            exact_tiers,
            target_physical_pixels_per_bin,
            max_exact_bins,
        ))
    }

    pub const fn preview_tiers(self) -> &'static [VisualResolutionTier] {
        self.preview_tiers
    }

    pub const fn exact_tiers(self) -> &'static [VisualResolutionTier] {
        self.exact_tiers
    }

    pub const fn target_physical_pixels_per_bin(self) -> NonZeroU8 {
        self.target_physical_pixels_per_bin
    }

    pub const fn max_exact_bins(self) -> NonZeroU64 {
        self.max_exact_bins
    }

    pub const fn tiers(self, quality: VisualFieldQuality) -> &'static [VisualResolutionTier] {
        match quality {
            VisualFieldQuality::Preview => self.preview_tiers,
            VisualFieldQuality::Exact => self.exact_tiers,
        }
    }
}

impl Default for VisualResolutionPolicy {
    fn default() -> Self {
        Self::new(
            DEFAULT_PREVIEW_TIERS,
            DEFAULT_EXACT_TIERS,
            DEFAULT_TARGET_PHYSICAL_PIXELS_PER_BIN,
            DEFAULT_MAX_EXACT_BINS,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualResolutionTier {
    pub quality: VisualFieldQuality,
    pub short_axis_bins: NonZeroU16,
    pub dimension_alignment_bins: NonZeroU8,
    pub max_total_bins: NonZeroU64,
}

impl VisualResolutionTier {
    pub const fn const_new(
        quality: VisualFieldQuality,
        short_axis_bins: u16,
        dimension_alignment_bins: u8,
        max_total_bins: u64,
    ) -> Self {
        Self {
            quality,
            short_axis_bins: NonZeroU16::new(short_axis_bins).unwrap(),
            dimension_alignment_bins: NonZeroU8::new(dimension_alignment_bins).unwrap(),
            max_total_bins: NonZeroU64::new(max_total_bins).unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualResolutionDecision {
    pub grid: GridSize,
    pub quality: VisualFieldQuality,
    pub reason: ResolutionDecisionReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolutionDecisionReason {
    PlotMatched,
    DeviceLimited,
    ResourceBudgetLimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualResolutionPolicyError {
    EmptyTiers(VisualFieldQuality),
    TierQualityMismatch(VisualFieldQuality),
    TiersNotAscending(VisualFieldQuality),
    ZeroPixelsPerBin,
    ZeroMaxExactBins,
}

impl fmt::Display for VisualResolutionPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTiers(quality) => {
                write!(formatter, "{quality:?} resolution tiers are empty")
            }
            Self::TierQualityMismatch(quality) => {
                write!(
                    formatter,
                    "resolution tier quality does not match {quality:?}"
                )
            }
            Self::TiersNotAscending(quality) => {
                write!(formatter, "{quality:?} resolution tiers must be ascending")
            }
            Self::ZeroPixelsPerBin => formatter.write_str("pixels per bin must be positive"),
            Self::ZeroMaxExactBins => {
                formatter.write_str("maximum exact bin count must be positive")
            }
        }
    }
}

impl Error for VisualResolutionPolicyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualResolutionError {
    ZeroPlotDimension,
    NoPlotTierFits { quality: VisualFieldQuality },
    NoDeviceTierFits { quality: VisualFieldQuality },
    NoResourceBudgetTierFits { quality: VisualFieldQuality },
    ArithmeticOverflow,
    GridConstruction,
    ResourceEstimate(VisualFieldResourceEstimateError),
}

impl fmt::Display for VisualResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroPlotDimension => {
                formatter.write_str("physical plot dimensions must be positive")
            }
            Self::NoPlotTierFits { quality } => write!(
                formatter,
                "no minimum {quality:?} tier fits the physical plot"
            ),
            Self::NoDeviceTierFits { quality } => write!(
                formatter,
                "no {quality:?} resolution tier fits device limits"
            ),
            Self::NoResourceBudgetTierFits { quality } => write!(
                formatter,
                "no {quality:?} resolution tier fits the remaining resource budget"
            ),
            Self::ArithmeticOverflow => {
                formatter.write_str("visual resolution arithmetic overflowed")
            }
            Self::GridConstruction => {
                formatter.write_str("visual resolution could not construct a checked grid")
            }
            Self::ResourceEstimate(error) => error.fmt(formatter),
        }
    }
}

impl Error for VisualResolutionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualPlotSizePx {
    pub width: u32,
    pub height: u32,
}

impl From<(u32, u32)> for VisualPlotSizePx {
    fn from((width, height): (u32, u32)) -> Self {
        Self { width, height }
    }
}

impl From<PlotRectPx> for VisualPlotSizePx {
    fn from(plot: PlotRectPx) -> Self {
        Self {
            width: plot.width,
            height: plot.height,
        }
    }
}

pub trait VisualFieldBudget {
    fn remaining_vram_bytes(&self) -> u64;
}

impl VisualFieldBudget for u64 {
    fn remaining_vram_bytes(&self) -> u64 {
        *self
    }
}

impl VisualFieldBudget for ResourceBudget {
    fn remaining_vram_bytes(&self) -> u64 {
        self.max_vram_bytes
    }
}

impl<T: VisualFieldBudget + ?Sized> VisualFieldBudget for &T {
    fn remaining_vram_bytes(&self) -> u64 {
        (**self).remaining_vram_bytes()
    }
}

pub fn choose_visual_resolution<P, B>(
    plot_size_px: P,
    policy: &VisualResolutionPolicy,
    device_limits: &wgpu::Limits,
    remaining_budget: B,
) -> Result<VisualResolutionDecision, VisualResolutionError>
where
    P: Into<VisualPlotSizePx>,
    B: VisualFieldBudget,
{
    choose_visual_resolution_for_quality(
        plot_size_px,
        policy,
        VisualFieldQuality::Exact,
        device_limits,
        remaining_budget,
    )
}

pub fn choose_visual_resolution_for_quality<P, B>(
    plot_size_px: P,
    policy: &VisualResolutionPolicy,
    quality: VisualFieldQuality,
    device_limits: &wgpu::Limits,
    remaining_budget: B,
) -> Result<VisualResolutionDecision, VisualResolutionError>
where
    P: Into<VisualPlotSizePx>,
    B: VisualFieldBudget,
{
    let plot_size_px = plot_size_px.into();
    let short_axis_px = plot_size_px.width.min(plot_size_px.height);
    let long_axis_px = plot_size_px.width.max(plot_size_px.height);
    if short_axis_px == 0 || long_axis_px == 0 {
        return Err(VisualResolutionError::ZeroPlotDimension);
    }

    let tiers = policy.tiers(quality);
    let target_pixels = u64::from(policy.target_physical_pixels_per_bin().get());
    let mut plot_tier_seen = false;
    let mut device_limited = false;
    let mut budget_limited = false;
    for tier in tiers.iter().rev() {
        let minimum_pixels = u64::from(tier.short_axis_bins.get())
            .checked_mul(target_pixels)
            .ok_or(VisualResolutionError::ArithmeticOverflow)?;
        if u64::from(short_axis_px) < minimum_pixels {
            continue;
        }
        plot_tier_seen = true;
        let grid = match derive_grid(plot_size_px, *tier) {
            Ok(grid) => grid,
            Err(_) => continue,
        };
        if quality == VisualFieldQuality::Exact
            && grid.bin_count() as u64 > policy.max_exact_bins().get()
        {
            budget_limited = true;
            continue;
        }
        let estimate = VisualFieldResourceEstimate::for_grid(grid)
            .map_err(VisualResolutionError::ResourceEstimate)?;
        let total_bytes = estimate
            .total_bytes()
            .map_err(VisualResolutionError::ResourceEstimate)?;
        if !device_allows(grid, estimate, device_limits) {
            device_limited = true;
            continue;
        }
        if total_bytes > remaining_budget.remaining_vram_bytes() {
            budget_limited = true;
            continue;
        }
        let reason = if device_limited {
            ResolutionDecisionReason::DeviceLimited
        } else if budget_limited {
            ResolutionDecisionReason::ResourceBudgetLimited
        } else {
            ResolutionDecisionReason::PlotMatched
        };
        return Ok(VisualResolutionDecision {
            grid,
            quality,
            reason,
        });
    }

    if !plot_tier_seen {
        return Err(VisualResolutionError::NoPlotTierFits { quality });
    }
    if budget_limited {
        return Err(VisualResolutionError::NoResourceBudgetTierFits { quality });
    }
    if device_limited {
        return Err(VisualResolutionError::NoDeviceTierFits { quality });
    }
    Err(VisualResolutionError::GridConstruction)
}

fn derive_grid(
    plot_size_px: VisualPlotSizePx,
    tier: VisualResolutionTier,
) -> Result<GridSize, VisualResolutionError> {
    let short_bins = u64::from(tier.short_axis_bins.get());
    let numerator = short_bins
        .checked_mul(u64::from(plot_size_px.width.max(plot_size_px.height)))
        .ok_or(VisualResolutionError::ArithmeticOverflow)?;
    let denominator = u64::from(plot_size_px.width.min(plot_size_px.height));
    let unaligned_long = numerator
        .checked_add(denominator - 1)
        .ok_or(VisualResolutionError::ArithmeticOverflow)?
        / denominator;
    let alignment = u64::from(tier.dimension_alignment_bins.get());
    let aligned_long = unaligned_long
        .checked_add(alignment - 1)
        .ok_or(VisualResolutionError::ArithmeticOverflow)?
        / alignment
        * alignment;
    let max_long = tier.max_total_bins.get() / short_bins;
    let long_bins = aligned_long.min(max_long);
    let long_bins = long_bins - (long_bins % alignment);
    if long_bins < alignment {
        return Err(VisualResolutionError::GridConstruction);
    }
    let (width, height) = if plot_size_px.width >= plot_size_px.height {
        (long_bins, short_bins)
    } else {
        (short_bins, long_bins)
    };
    let width = u32::try_from(width).map_err(|_| VisualResolutionError::ArithmeticOverflow)?;
    let height = u32::try_from(height).map_err(|_| VisualResolutionError::ArithmeticOverflow)?;
    GridSize::try_new(width, height).map_err(|_| VisualResolutionError::GridConstruction)
}

fn device_allows(
    grid: GridSize,
    estimate: VisualFieldResourceEstimate,
    limits: &wgpu::Limits,
) -> bool {
    let dimension = limits.max_texture_dimension_2d;
    if grid.width() > dimension || grid.height() > dimension {
        return false;
    }
    if estimate.largest_binding_bytes() > limits.max_storage_buffer_binding_size {
        return false;
    }
    let total_bytes = match estimate.total_bytes() {
        Ok(total_bytes) => total_bytes,
        Err(_) => return false,
    };
    if total_bytes > limits.max_buffer_size {
        return false;
    }
    let workgroup_x = grid.width().div_ceil(8);
    let workgroup_y = grid.height().div_ceil(8);
    workgroup_x <= limits.max_compute_workgroups_per_dimension
        && workgroup_y <= limits.max_compute_workgroups_per_dimension
}

fn validate_tiers(
    tiers: &[VisualResolutionTier],
    quality: VisualFieldQuality,
) -> Result<(), VisualResolutionPolicyError> {
    if tiers.is_empty() {
        return Err(VisualResolutionPolicyError::EmptyTiers(quality));
    }
    let mut previous = 0_u16;
    for tier in tiers {
        if tier.quality != quality {
            return Err(VisualResolutionPolicyError::TierQualityMismatch(quality));
        }
        if tier.short_axis_bins.get() <= previous {
            return Err(VisualResolutionPolicyError::TiersNotAscending(quality));
        }
        previous = tier.short_axis_bins.get();
    }
    Ok(())
}

/// Full identity required to publish a visual-field result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualFieldIntent {
    pub dataset_generation: DatasetGeneration,
    pub cohort_generation: CohortGeneration,
    pub view_generation: super::VisualFieldViewGeneration,
    pub mapping: VisualFieldMapping,
    pub mode: VisualFieldMode,
    pub resolution: VisualResolutionDecision,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> VisualResolutionPolicy {
        const PREVIEW: &[VisualResolutionTier] = &[
            VisualResolutionTier::const_new(VisualFieldQuality::Preview, 64, 8, 32_768),
            VisualResolutionTier::const_new(VisualFieldQuality::Preview, 128, 8, 131_072),
        ];
        const EXACT: &[VisualResolutionTier] = &[
            VisualResolutionTier::const_new(VisualFieldQuality::Exact, 128, 8, 131_072),
            VisualResolutionTier::const_new(VisualFieldQuality::Exact, 256, 8, 524_288),
        ];
        VisualResolutionPolicy::new(
            PREVIEW,
            EXACT,
            NonZeroU8::new(2).unwrap(),
            NonZeroU64::new(524_288).unwrap(),
        )
    }

    #[test]
    fn resolution_chooses_largest_plot_matched_tier() {
        let decision = choose_visual_resolution_for_quality(
            (1_600, 900),
            &policy(),
            VisualFieldQuality::Preview,
            &wgpu::Limits::default(),
            u64::MAX,
        )
        .unwrap();
        assert_eq!(decision.grid.width(), 232);
        assert_eq!(decision.grid.height(), 128);
        assert_eq!(decision.quality, VisualFieldQuality::Preview);
        assert_eq!(decision.reason, ResolutionDecisionReason::PlotMatched);
    }

    #[test]
    fn resolution_preserves_wide_and_tall_plot_aspect_with_alignment() {
        let wide = choose_visual_resolution_for_quality(
            (1_600, 800),
            &policy(),
            VisualFieldQuality::Exact,
            &wgpu::Limits::default(),
            u64::MAX,
        )
        .unwrap();
        let tall = choose_visual_resolution_for_quality(
            (800, 1_600),
            &policy(),
            VisualFieldQuality::Exact,
            &wgpu::Limits::default(),
            u64::MAX,
        )
        .unwrap();
        assert_eq!(wide.grid.width() % 8, 0);
        assert_eq!(wide.grid.height() % 8, 0);
        assert_eq!(tall.grid.width() % 8, 0);
        assert_eq!(tall.grid.height() % 8, 0);
        assert_eq!(wide.grid.width(), tall.grid.height());
        assert_eq!(wide.grid.height(), tall.grid.width());
    }

    #[test]
    fn resolution_respects_device_and_budget_limits() {
        let mut limits = wgpu::Limits::default();
        // The default visual-field estimate includes the bounded category,
        // ridge, transition, and readback resources. Keep the limit above the
        // smallest exact tier's largest binding while below the next tier so
        // this remains a real device-limited fallback assertion.
        limits.max_storage_buffer_binding_size = 2_000_000;
        let decision = choose_visual_resolution_for_quality(
            (1_600, 900),
            &policy(),
            VisualFieldQuality::Exact,
            &limits,
            u64::MAX,
        )
        .unwrap();
        assert_eq!(decision.grid.height(), 128);
        assert_eq!(decision.reason, ResolutionDecisionReason::DeviceLimited);

        let decision = choose_visual_resolution_for_quality(
            (1_600, 900),
            &policy(),
            VisualFieldQuality::Exact,
            &wgpu::Limits::default(),
            1_000,
        );
        assert!(matches!(
            decision,
            Err(VisualResolutionError::NoResourceBudgetTierFits { .. })
        ));
    }
}
