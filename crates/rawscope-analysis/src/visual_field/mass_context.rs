//! Exact count-derived mass contours and axis marginals for visual fields.
//!
//! This module deliberately depends only on the settled count grid.  It does
//! not know about projected rows, smoothing, palettes, or a concrete renderer,
//! so one immutable context can be shared by inspection, evidence, and GPU
//! presentation owners without triggering another source scan or readback.

use std::{collections::BTreeMap, error::Error, fmt, num::NonZeroU32, sync::Arc};

use rawscope_core::DensityCountGrid;

/// Denominator used by [`MassFractionBasisPoints`].
pub const MASS_FRACTION_DENOMINATOR: u16 = 10_000;

/// Reviewed default mass levels: 50%, 80%, 95%, and 99% of rows.
pub const DEFAULT_MASS_FRACTIONS: [MassFractionBasisPoints; 4] = [
    MassFractionBasisPoints(5_000),
    MassFractionBasisPoints(8_000),
    MassFractionBasisPoints(9_500),
    MassFractionBasisPoints(9_900),
];

/// A requested mass fraction represented in basis points.
///
/// The value is intentionally bounded to one through 10,000 basis points;
/// using a newtype prevents callers from accidentally passing a percentage or
/// a floating-point intensity value to the exact mass algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MassFractionBasisPoints(u16);

impl MassFractionBasisPoints {
    /// Constructs a mass fraction in the inclusive range 1..=10,000.
    pub const fn try_new(value: u16) -> Result<Self, MassFractionError> {
        if value == 0 || value > MASS_FRACTION_DENOMINATOR {
            return Err(MassFractionError::OutOfRange { value });
        }
        Ok(Self(value))
    }

    /// Returns the integer basis-point value.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Returns the fraction as a display-oriented percentage value.
    pub fn percentage(self) -> f32 {
        f32::from(self.0) / 100.0
    }
}

/// Validation failure for a requested mass fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassFractionError {
    OutOfRange { value: u16 },
}

impl fmt::Display for MassFractionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange { value } => write!(
                formatter,
                "mass fraction basis points must be between 1 and {MASS_FRACTION_DENOMINATOR}, got {value}"
            ),
        }
    }
}

impl Error for MassFractionError {}

/// One exact tied threshold derived from a settled count grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MassContourLevel {
    /// Fraction requested by the caller.
    pub requested: MassFractionBasisPoints,
    /// Smallest nonzero bin count included by this contour, if any.
    ///
    /// Every bin tied at this count is included.  Consequently the enclosed
    /// row count can exceed the requested fraction and is never understated.
    pub minimum_bin_count: Option<NonZeroU32>,
    /// Actual rows enclosed by this tied threshold.
    pub enclosed_row_count: u64,
    /// Total rows represented by the settled grid.
    pub total_row_count: u64,
}

/// Exact mass thresholds for one settled count grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MassContourSet {
    pub levels: Arc<[MassContourLevel]>,
}

impl MassContourSet {
    /// Derives tied mass thresholds from one immutable count grid.
    pub fn from_grid(
        grid: &DensityCountGrid,
        requested: &[MassFractionBasisPoints],
    ) -> Result<Self, MassContextError> {
        validate_requested_fractions(requested)?;

        let total_row_count = checked_total_count(grid)?;
        if total_row_count == 0 {
            let levels = requested
                .iter()
                .copied()
                .map(|requested| MassContourLevel {
                    requested,
                    minimum_bin_count: None,
                    enclosed_row_count: 0,
                    total_row_count: 0,
                })
                .collect::<Vec<_>>();
            return Ok(Self {
                levels: levels.into(),
            });
        }

        let targets = requested
            .iter()
            .copied()
            .map(|fraction| ceil_mass_target(total_row_count, fraction))
            .collect::<Result<Vec<_>, _>>()?;
        let histogram = nonzero_count_histogram(grid)?;
        let mut levels = Vec::with_capacity(requested.len());
        let mut target_index = 0;
        let mut enclosed_row_count = 0_u64;

        for (count, bin_frequency) in histogram.into_iter().rev() {
            let threshold_mass = u64::from(count)
                .checked_mul(bin_frequency)
                .ok_or(MassContextError::ArithmeticOverflow)?;
            enclosed_row_count = enclosed_row_count
                .checked_add(threshold_mass)
                .ok_or(MassContextError::ArithmeticOverflow)?;

            while target_index < targets.len() && enclosed_row_count >= targets[target_index] {
                let Some(minimum_bin_count) = NonZeroU32::new(count) else {
                    return Err(MassContextError::ArithmeticOverflow);
                };
                levels.push(MassContourLevel {
                    requested: requested[target_index],
                    minimum_bin_count: Some(minimum_bin_count),
                    enclosed_row_count,
                    total_row_count,
                });
                target_index += 1;
            }

            if target_index == targets.len() {
                break;
            }
        }

        // A nonzero grid always reaches every target (the largest target is
        // at most the total), but retain a checked failure if that invariant
        // is ever broken by a future histogram implementation.
        if target_index != requested.len() {
            return Err(MassContextError::ArithmeticOverflow);
        }

        Ok(Self {
            levels: levels.into(),
        })
    }

    /// Derives the reviewed default 50/80/95/99% levels.
    pub fn from_default_grid(grid: &DensityCountGrid) -> Result<Self, MassContextError> {
        Self::from_grid(grid, &DEFAULT_MASS_FRACTIONS)
    }
}

/// X/Y row-count marginals derived from the same settled count grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DensityMarginals {
    pub x_counts: Arc<[u64]>,
    pub y_counts: Arc<[u64]>,
    pub max_x_count: u64,
    pub max_y_count: u64,
}

impl DensityMarginals {
    /// Sums each grid cell exactly once into its X and Y axis marginal.
    pub fn from_grid(grid: &DensityCountGrid) -> Result<Self, MassContextError> {
        let mut x_counts = vec![0_u64; grid.width() as usize];
        let mut y_counts = vec![0_u64; grid.height() as usize];
        for (index, count) in grid.counts().iter().copied().enumerate() {
            let x = index % grid.width() as usize;
            let y = index / grid.width() as usize;
            let count = u64::from(count);
            x_counts[x] = x_counts[x]
                .checked_add(count)
                .ok_or(MassContextError::ArithmeticOverflow)?;
            y_counts[y] = y_counts[y]
                .checked_add(count)
                .ok_or(MassContextError::ArithmeticOverflow)?;
        }
        let max_x_count = x_counts.iter().copied().max().unwrap_or(0);
        let max_y_count = y_counts.iter().copied().max().unwrap_or(0);
        Ok(Self {
            x_counts: x_counts.into(),
            y_counts: y_counts.into(),
            max_x_count,
            max_y_count,
        })
    }
}

/// One immutable settled count snapshot and all count-derived summaries.
///
/// The generation identity is generic by design: `rawscope-analysis` cannot
/// depend on the renderer's owner-minted view token without creating a crate
/// cycle.  The render/workbench layer can instantiate this as
/// `SettledDensityContext<VisualFieldViewGeneration>`, while analysis-only
/// callers use the default `u64` identity or another owner-minted token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettledDensityContext<G = u64> {
    pub field_generation: G,
    pub counts: Arc<DensityCountGrid>,
    pub contours: MassContourSet,
    pub marginals: DensityMarginals,
}

impl<G> SettledDensityContext<G> {
    /// Combines a previously derived contour set with its shared count grid.
    ///
    /// This constructor is useful when a settled generation publishes the
    /// contour result before downstream consumers attach it.  The contour
    /// totals are checked against the same grid so a stale summary cannot be
    /// paired with a new readback.
    pub fn new(
        field_generation: G,
        counts: Arc<DensityCountGrid>,
        contours: MassContourSet,
    ) -> Result<Self, MassContextError> {
        let total_row_count = checked_total_count(&counts)?;
        if contours
            .levels
            .iter()
            .any(|level| level.total_row_count != total_row_count)
        {
            return Err(MassContextError::ContourTotalMismatch {
                expected: total_row_count,
            });
        }
        let marginals = DensityMarginals::from_grid(&counts)?;
        Ok(Self {
            field_generation,
            counts,
            contours,
            marginals,
        })
    }

    /// Builds contours and marginals from the one shared count-grid snapshot.
    pub fn try_new(
        field_generation: G,
        counts: Arc<DensityCountGrid>,
        requested: &[MassFractionBasisPoints],
    ) -> Result<Self, MassContextError> {
        let contours = MassContourSet::from_grid(&counts, requested)?;
        let marginals = DensityMarginals::from_grid(&counts)?;
        Ok(Self {
            field_generation,
            counts,
            contours,
            marginals,
        })
    }
}

impl<G> SettledDensityContext<G> {
    /// Builds the context with the reviewed default mass levels.
    pub fn try_new_with_default_fractions(
        field_generation: G,
        counts: Arc<DensityCountGrid>,
    ) -> Result<Self, MassContextError> {
        Self::try_new(field_generation, counts, &DEFAULT_MASS_FRACTIONS)
    }
}

/// Failure while deriving count-based mass or marginal summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassContextError {
    FractionsNotStrictlyIncreasing {
        previous: MassFractionBasisPoints,
        next: MassFractionBasisPoints,
    },
    ContourTotalMismatch {
        expected: u64,
    },
    ArithmeticOverflow,
}

impl fmt::Display for MassContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FractionsNotStrictlyIncreasing { previous, next } => write!(
                formatter,
                "mass fractions must be strictly increasing: {} then {}",
                previous.get(),
                next.get()
            ),
            Self::ContourTotalMismatch { expected } => write!(
                formatter,
                "settled contour totals do not match the count grid total of {expected}"
            ),
            Self::ArithmeticOverflow => formatter.write_str("mass context arithmetic overflowed"),
        }
    }
}

impl Error for MassContextError {}

fn validate_requested_fractions(
    requested: &[MassFractionBasisPoints],
) -> Result<(), MassContextError> {
    for pair in requested.windows(2) {
        if pair[0] >= pair[1] {
            return Err(MassContextError::FractionsNotStrictlyIncreasing {
                previous: pair[0],
                next: pair[1],
            });
        }
    }
    Ok(())
}

fn checked_total_count(grid: &DensityCountGrid) -> Result<u64, MassContextError> {
    grid.counts().iter().try_fold(0_u64, |total, count| {
        total
            .checked_add(u64::from(*count))
            .ok_or(MassContextError::ArithmeticOverflow)
    })
}

fn ceil_mass_target(
    total_row_count: u64,
    fraction: MassFractionBasisPoints,
) -> Result<u64, MassContextError> {
    let numerator = u128::from(total_row_count)
        .checked_mul(u128::from(fraction.get()))
        .ok_or(MassContextError::ArithmeticOverflow)?;
    let denominator = u128::from(MASS_FRACTION_DENOMINATOR);
    let target = numerator
        .checked_add(denominator - 1)
        .ok_or(MassContextError::ArithmeticOverflow)?
        / denominator;
    u64::try_from(target).map_err(|_| MassContextError::ArithmeticOverflow)
}

fn nonzero_count_histogram(
    grid: &DensityCountGrid,
) -> Result<BTreeMap<u32, u64>, MassContextError> {
    let mut histogram = BTreeMap::new();
    for count in grid.counts().iter().copied().filter(|count| *count != 0) {
        let frequency = histogram.entry(count).or_insert(0_u64);
        *frequency = frequency
            .checked_add(1)
            .ok_or(MassContextError::ArithmeticOverflow)?;
    }
    Ok(histogram)
}
