//! Deterministic, CPU-reference multiscale density ridge analysis.
//!
//! A ridge is local geometry of a smoothed scalar count field.  It is not a
//! path, cluster, trajectory, or membership assignment.  This module is
//! intentionally independent of renderers, serde, and dataset profiles so a
//! later GPU implementation can be checked against the same intermediate
//! formulas.

use std::{error::Error, fmt, sync::Arc};

use rawscope_core::DensityCountGrid;
/// Version of the closed CPU ridge formulas exposed by this module.
pub const RIDGE_FORMULA_VERSION: u16 = 1;
/// Basis-point denominator used by both ridge cutoffs.
pub const RIDGE_BASIS_POINTS_DENOMINATOR: u16 = 10_000;
/// Default normalized strength cutoff (2.5%).
pub const DEFAULT_RIDGE_MINIMUM_STRENGTH_BASIS_POINTS: u16 = 250;
/// Default eigengap anisotropy cutoff (15%).
pub const DEFAULT_RIDGE_MINIMUM_ANISOTROPY_BASIS_POINTS: u16 = 1_500;
/// Small denominator epsilon used for normalized eigengap calculations.
pub const RIDGE_NORMALIZATION_EPSILON: f64 = 1.0e-12;
/// Absolute eigengap floor preventing an arbitrary direction at an isotropic cell.
pub const RIDGE_EIGENGAP_EPSILON: f64 = 1.0e-12;

const FINE_KERNEL: &[f64] = &[0.25, 0.5, 0.25];
const MEDIUM_KERNEL: &[f64] = &[0.0625, 0.25, 0.375, 0.25, 0.0625];
const COARSE_KERNEL: &[f64] = &[
    0.00390625, 0.03125, 0.109375, 0.21875, 0.2734375, 0.21875, 0.109375, 0.03125, 0.00390625,
];

/// The three reviewed, bounded smoothing scales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RidgeScale {
    Fine,
    Medium,
    Coarse,
}

impl RidgeScale {
    /// Radius in cells of the fixed separable kernel.
    pub const fn radius_cells(self) -> usize {
        match self {
            Self::Fine => 1,
            Self::Medium => 2,
            Self::Coarse => 4,
        }
    }

    /// Exact normalized one-dimensional kernel coefficients.
    pub const fn kernel_coefficients(self) -> &'static [f64] {
        match self {
            Self::Fine => FINE_KERNEL,
            Self::Medium => MEDIUM_KERNEL,
            Self::Coarse => COARSE_KERNEL,
        }
    }

    /// Effective sigma of the corresponding binomial kernel in grid cells.
    /// A binomial row of order `2 * radius` has variance `radius / 2`.
    pub const fn effective_sigma_cells(self) -> f64 {
        match self {
            Self::Fine => 0.7071067811865476,
            Self::Medium => 1.0,
            Self::Coarse => 1.4142135623730951,
        }
    }
}

/// Validated strength and anisotropy filters for one ridge scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RidgeConfig {
    pub scale: RidgeScale,
    pub minimum_strength_basis_points: u16,
    pub minimum_anisotropy_basis_points: u16,
}

impl Default for RidgeConfig {
    fn default() -> Self {
        Self {
            scale: RidgeScale::Medium,
            minimum_strength_basis_points: DEFAULT_RIDGE_MINIMUM_STRENGTH_BASIS_POINTS,
            minimum_anisotropy_basis_points: DEFAULT_RIDGE_MINIMUM_ANISOTROPY_BASIS_POINTS,
        }
    }
}

impl RidgeConfig {
    /// Validates a configuration without changing caller-selected cutoffs.
    pub const fn try_new(
        scale: RidgeScale,
        minimum_strength_basis_points: u16,
        minimum_anisotropy_basis_points: u16,
    ) -> Result<Self, RidgeError> {
        if minimum_strength_basis_points > RIDGE_BASIS_POINTS_DENOMINATOR {
            return Err(RidgeError::StrengthBasisPointsOutOfRange {
                value: minimum_strength_basis_points,
            });
        }
        if minimum_anisotropy_basis_points > RIDGE_BASIS_POINTS_DENOMINATOR {
            return Err(RidgeError::AnisotropyBasisPointsOutOfRange {
                value: minimum_anisotropy_basis_points,
            });
        }
        Ok(Self {
            scale,
            minimum_strength_basis_points,
            minimum_anisotropy_basis_points,
        })
    }
}

/// One cell's normalized ridge strength and undirected tangent orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RidgeCell {
    pub strength: f32,
    pub tangent_x: f32,
    pub tangent_y: f32,
}

impl RidgeCell {
    const ZERO: Self = Self {
        strength: 0.0,
        tangent_x: 0.0,
        tangent_y: 0.0,
    };
}

/// Immutable CPU ridge field for one exact count-grid generation and config.
#[derive(Debug, Clone, PartialEq)]
pub struct DensityRidgeField {
    pub width: u32,
    pub height: u32,
    pub config: RidgeConfig,
    pub cells: Arc<[RidgeCell]>,
}

/// Failure while validating or deriving a ridge field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RidgeError {
    StrengthBasisPointsOutOfRange { value: u16 },
    AnisotropyBasisPointsOutOfRange { value: u16 },
    NonFiniteComputation,
}

impl fmt::Display for RidgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StrengthBasisPointsOutOfRange { value } => write!(
                formatter,
                "ridge strength basis points must be at most {RIDGE_BASIS_POINTS_DENOMINATOR}, got {value}"
            ),
            Self::AnisotropyBasisPointsOutOfRange { value } => write!(
                formatter,
                "ridge anisotropy basis points must be at most {RIDGE_BASIS_POINTS_DENOMINATOR}, got {value}"
            ),
            Self::NonFiniteComputation => formatter.write_str("ridge derivation produced a non-finite value"),
        }
    }
}

impl Error for RidgeError {}

/// Derives a bounded, deterministic multiscale ridge field from exact counts.
///
/// Counts are smoothed in flattened row-major order with the fixed separable
/// binomial kernel for `config.scale`. Coordinates are normalized UVs, with
/// `1 / (width - 1)` and `1 / (height - 1)` derivative spacing (unit spacing
/// for a one-cell axis). Sampling and derivatives use clamped edge values;
/// output cells retain the input row-major order and no input memory is changed.
pub fn derive_density_ridges(
    counts: &DensityCountGrid,
    config: RidgeConfig,
) -> Result<DensityRidgeField, RidgeError> {
    let config = RidgeConfig::try_new(
        config.scale,
        config.minimum_strength_basis_points,
        config.minimum_anisotropy_basis_points,
    )?;
    let width = counts.width();
    let height = counts.height();
    let smoothed = smooth_counts(counts, config.scale);
    let x_step = if width > 1 {
        1.0 / (f64::from(width) - 1.0)
    } else {
        1.0
    };
    let y_step = if height > 1 {
        1.0 / (f64::from(height) - 1.0)
    } else {
        1.0
    };
    let mut candidates = Vec::with_capacity(smoothed.len());
    let mut maximum_raw_strength = 0.0_f64;

    for y in 0..height as usize {
        for x in 0..width as usize {
            let center = sample_index(
                &smoothed,
                width as usize,
                height as usize,
                x as f64,
                y as f64,
            );
            let (dxx, dxy, dyy) = hessian_at(
                &smoothed,
                width as usize,
                height as usize,
                x,
                y,
                x_step,
                y_step,
            );
            let (lambda_min, lambda_max, normal_x, normal_y) =
                eigen_for_minor_direction(dxx, dxy, dyy);
            let eigengap = (lambda_max - lambda_min).abs();
            let anisotropy =
                eigengap / (lambda_max.abs() + lambda_min.abs() + RIDGE_NORMALIZATION_EPSILON);
            let anisotropy_cutoff = f64::from(config.minimum_anisotropy_basis_points)
                / f64::from(RIDGE_BASIS_POINTS_DENOMINATOR);
            let local_maximum = if normal_x == 0.0 && normal_y == 0.0 {
                false
            } else {
                let plus = sample_index(
                    &smoothed,
                    width as usize,
                    height as usize,
                    x as f64 + normal_x,
                    y as f64 + normal_y,
                );
                let minus = sample_index(
                    &smoothed,
                    width as usize,
                    height as usize,
                    x as f64 - normal_x,
                    y as f64 - normal_y,
                );
                center >= plus && center >= minus && (center > plus || center > minus)
            };
            let is_candidate = lambda_min < 0.0
                && anisotropy.is_finite()
                && anisotropy >= anisotropy_cutoff
                && eigengap > RIDGE_EIGENGAP_EPSILON
                && local_maximum;
            let raw_strength = if is_candidate {
                config.scale.effective_sigma_cells().powi(2) * (-lambda_min).max(0.0)
            } else {
                0.0
            };
            if !raw_strength.is_finite() {
                return Err(RidgeError::NonFiniteComputation);
            }
            maximum_raw_strength = maximum_raw_strength.max(raw_strength);
            candidates.push((raw_strength, normal_x, normal_y));
        }
    }

    let strength_cutoff =
        f64::from(config.minimum_strength_basis_points) / f64::from(RIDGE_BASIS_POINTS_DENOMINATOR);
    let mut cells = Vec::with_capacity(candidates.len());
    for (raw_strength, normal_x, normal_y) in candidates {
        if maximum_raw_strength <= 0.0 {
            cells.push(RidgeCell::ZERO);
            continue;
        }
        let normalized_strength = (raw_strength / maximum_raw_strength).clamp(0.0, 1.0);
        if !normalized_strength.is_finite() {
            return Err(RidgeError::NonFiniteComputation);
        }
        if normalized_strength < strength_cutoff || normalized_strength <= 0.0 {
            cells.push(RidgeCell::ZERO);
            continue;
        }
        // Tangent is perpendicular to the selected normal.  Sign canonicalization
        // is only for deterministic bytes; the product meaning remains undirected.
        let mut tangent_x = -normal_y;
        let mut tangent_y = normal_x;
        if tangent_x < 0.0 || (tangent_x == 0.0 && tangent_y < 0.0) {
            tangent_x = -tangent_x;
            tangent_y = -tangent_y;
        }
        let length = (tangent_x * tangent_x + tangent_y * tangent_y).sqrt();
        if !length.is_finite() || length <= RIDGE_EIGENGAP_EPSILON {
            cells.push(RidgeCell::ZERO);
            continue;
        }
        let cell = RidgeCell {
            strength: normalized_strength as f32,
            tangent_x: (tangent_x / length) as f32,
            tangent_y: (tangent_y / length) as f32,
        };
        if !cell.strength.is_finite() || !cell.tangent_x.is_finite() || !cell.tangent_y.is_finite()
        {
            return Err(RidgeError::NonFiniteComputation);
        }
        cells.push(cell);
    }

    Ok(DensityRidgeField {
        width,
        height,
        config,
        cells: cells.into(),
    })
}

fn smooth_counts(counts: &DensityCountGrid, scale: RidgeScale) -> Vec<f64> {
    let width = counts.width() as usize;
    let height = counts.height() as usize;
    let kernel = scale.kernel_coefficients();
    let radius = scale.radius_cells() as isize;
    let mut horizontal = vec![0.0; width * height];
    for y in 0..height {
        for x in 0..width {
            let mut value = 0.0;
            for (kernel_index, coefficient) in kernel.iter().enumerate() {
                let offset = kernel_index as isize - radius;
                let sample_x = (x as isize + offset).clamp(0, width as isize - 1) as usize;
                value += *coefficient * f64::from(counts.count(sample_x as u32, y as u32));
            }
            horizontal[y * width + x] = value;
        }
    }
    let mut smoothed = vec![0.0; width * height];
    for y in 0..height {
        for x in 0..width {
            let mut value = 0.0;
            for (kernel_index, coefficient) in kernel.iter().enumerate() {
                let offset = kernel_index as isize - radius;
                let sample_y = (y as isize + offset).clamp(0, height as isize - 1) as usize;
                value += *coefficient * horizontal[sample_y * width + x];
            }
            smoothed[y * width + x] = value;
        }
    }
    smoothed
}

fn hessian_at(
    values: &[f64],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    x_step: f64,
    y_step: f64,
) -> (f64, f64, f64) {
    let center = sample_index(values, width, height, x as f64, y as f64);
    let left = sample_index(values, width, height, x as f64 - 1.0, y as f64);
    let right = sample_index(values, width, height, x as f64 + 1.0, y as f64);
    let up = sample_index(values, width, height, x as f64, y as f64 - 1.0);
    let down = sample_index(values, width, height, x as f64, y as f64 + 1.0);
    let upper_left = sample_index(values, width, height, x as f64 - 1.0, y as f64 - 1.0);
    let upper_right = sample_index(values, width, height, x as f64 + 1.0, y as f64 - 1.0);
    let lower_left = sample_index(values, width, height, x as f64 - 1.0, y as f64 + 1.0);
    let lower_right = sample_index(values, width, height, x as f64 + 1.0, y as f64 + 1.0);
    let dxx = (right - 2.0 * center + left) / (x_step * x_step);
    let dyy = (down - 2.0 * center + up) / (y_step * y_step);
    let dxy = (lower_right - lower_left - upper_right + upper_left) / (4.0 * x_step * y_step);
    (dxx, dxy, dyy)
}

fn eigen_for_minor_direction(dxx: f64, dxy: f64, dyy: f64) -> (f64, f64, f64, f64) {
    let half_trace = (dxx + dyy) * 0.5;
    let half_difference = (dxx - dyy) * 0.5;
    let discriminant = (half_difference * half_difference + dxy * dxy).sqrt();
    let lambda_min = half_trace - discriminant;
    let lambda_max = half_trace + discriminant;
    if !discriminant.is_finite() || discriminant <= RIDGE_EIGENGAP_EPSILON {
        return (lambda_min, lambda_max, 0.0, 0.0);
    }
    let candidate_a = (dxy, lambda_min - dxx);
    let candidate_b = (lambda_min - dyy, dxy);
    let candidate = if candidate_a.0 * candidate_a.0 + candidate_a.1 * candidate_a.1
        >= candidate_b.0 * candidate_b.0 + candidate_b.1 * candidate_b.1
    {
        candidate_a
    } else {
        candidate_b
    };
    let norm = (candidate.0 * candidate.0 + candidate.1 * candidate.1).sqrt();
    if !norm.is_finite() || norm <= RIDGE_EIGENGAP_EPSILON {
        (lambda_min, lambda_max, 0.0, 0.0)
    } else {
        (
            lambda_min,
            lambda_max,
            candidate.0 / norm,
            candidate.1 / norm,
        )
    }
}

/// Samples the smoothed grid with clamped coordinates (the documented edge rule).
fn sample_index(values: &[f64], width: usize, height: usize, x: f64, y: f64) -> f64 {
    let x = x.clamp(0.0, width.saturating_sub(1) as f64);
    let y = y.clamp(0.0, height.saturating_sub(1) as f64);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x - x0 as f64;
    let ty = y - y0 as f64;
    let top_left = values[y0 * width + x0];
    let top_right = values[y0 * width + x1];
    let bottom_left = values[y1 * width + x0];
    let bottom_right = values[y1 * width + x1];
    let top = top_left + (top_right - top_left) * tx;
    let bottom = bottom_left + (bottom_right - bottom_left) * tx;
    top + (bottom - top) * ty
}
