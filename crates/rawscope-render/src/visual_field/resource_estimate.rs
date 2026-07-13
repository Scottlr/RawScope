//! Checked derived-resource accounting for visual-field generations.

use std::{error::Error, fmt};

use rawscope_core::GridSize;
use rawscope_data::ResourceReservation;

/// Fixed bind/pipeline bookkeeping included in every visual-field estimate.
///
/// This is deliberately a small policy overhead, not a claim about a specific
/// backend allocation. Backend owners can replace it with a measured value in
/// a later, human-reviewed policy revision.
pub const DEFAULT_BIND_PIPELINE_BYTES: u64 = 4 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualFieldResourceOptions {
    /// Number of category-derived layers retained for this field.
    pub category_layers: u8,
    /// Number of derived ridge fields retained for this field.
    pub ridge_fields: u8,
    /// Number of previous-transition fields retained alongside the current field.
    pub transition_fields: u8,
    /// Whether a full count readback staging buffer is retained.
    pub readback: bool,
    /// Fixed bind-group/pipeline bookkeeping overhead.
    pub bind_pipeline_bytes: u64,
}

impl Default for VisualFieldResourceOptions {
    fn default() -> Self {
        Self {
            category_layers: 1,
            ridge_fields: 1,
            transition_fields: 1,
            readback: true,
            bind_pipeline_bytes: DEFAULT_BIND_PIPELINE_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualFieldResourceEstimateError {
    Grid(GridSizeError),
    ArithmeticOverflow,
}

impl fmt::Display for VisualFieldResourceEstimateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grid(error) => write!(formatter, "invalid visual-field grid: {error}"),
            Self::ArithmeticOverflow => {
                formatter.write_str("visual-field resource estimate arithmetic overflowed")
            }
        }
    }
}

impl Error for VisualFieldResourceEstimateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSizeError {
    ZeroDimension,
    BinCountOverflow,
}

impl fmt::Display for GridSizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => {
                formatter.write_str("visual-field grid dimensions must be positive")
            }
            Self::BinCountOverflow => formatter.write_str("visual-field grid bin count overflowed"),
        }
    }
}

/// Checked bytes for all fields that can be live for one visual-field view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisualFieldResourceEstimate {
    pub coordinate_bytes: u64,
    pub channel_bytes: u64,
    pub count_field_bytes: u64,
    pub derived_field_bytes: u64,
    pub transition_bytes: u64,
    pub staging_bytes: u64,
}

impl VisualFieldResourceEstimate {
    /// Estimates the default continuous 2D field resource set.
    pub fn for_grid(grid: GridSize) -> Result<Self, VisualFieldResourceEstimateError> {
        Self::for_grid_with_options(grid, VisualFieldResourceOptions::default())
    }

    /// Estimates a field with explicit optional derived resources.
    pub fn for_grid_with_options(
        grid: GridSize,
        options: VisualFieldResourceOptions,
    ) -> Result<Self, VisualFieldResourceEstimateError> {
        let bins = grid.bin_count() as u64;
        let coordinate_bytes = checked_mul(bins, 8)?;
        let channel_bytes = checked_mul(bins, 4)?;
        let count_field_bytes = checked_mul(checked_mul(bins, 4)?, 2)?;
        let derived_fields = u64::from(options.category_layers)
            .checked_add(u64::from(options.ridge_fields))
            .ok_or(VisualFieldResourceEstimateError::ArithmeticOverflow)?;
        let derived_field_bytes = checked_mul(checked_mul(bins, 4)?, derived_fields)?;
        let transition_bytes =
            checked_mul(checked_mul(bins, 4)?, u64::from(options.transition_fields))?;
        let readback_bytes = if options.readback {
            checked_mul(bins, 4)?
        } else {
            0
        };
        let staging_bytes = readback_bytes
            .checked_add(options.bind_pipeline_bytes)
            .ok_or(VisualFieldResourceEstimateError::ArithmeticOverflow)?;

        Ok(Self {
            coordinate_bytes,
            channel_bytes,
            count_field_bytes,
            derived_field_bytes,
            transition_bytes,
            staging_bytes,
        })
    }

    pub fn for_dimensions(
        width: u32,
        height: u32,
    ) -> Result<Self, VisualFieldResourceEstimateError> {
        let grid = GridSize::try_new(width, height).map_err(|error| match error {
            rawscope_core::GridSizeError::ZeroWidth | rawscope_core::GridSizeError::ZeroHeight => {
                VisualFieldResourceEstimateError::Grid(GridSizeError::ZeroDimension)
            }
            rawscope_core::GridSizeError::BinCountOverflow { .. } => {
                VisualFieldResourceEstimateError::Grid(GridSizeError::BinCountOverflow)
            }
            _ => VisualFieldResourceEstimateError::Grid(GridSizeError::BinCountOverflow),
        })?;
        Self::for_grid(grid)
    }

    pub fn total_bytes(self) -> Result<u64, VisualFieldResourceEstimateError> {
        [
            self.coordinate_bytes,
            self.channel_bytes,
            self.count_field_bytes,
            self.derived_field_bytes,
            self.transition_bytes,
            self.staging_bytes,
        ]
        .into_iter()
        .try_fold(0_u64, |total, value| {
            total
                .checked_add(value)
                .ok_or(VisualFieldResourceEstimateError::ArithmeticOverflow)
        })
    }

    pub fn largest_binding_bytes(self) -> u64 {
        self.coordinate_bytes
            .max(self.channel_bytes)
            .max(self.count_field_bytes)
            .max(self.derived_field_bytes)
            .max(self.transition_bytes)
            .max(self.staging_bytes)
    }

    /// Adds one simultaneously live visual-field resource set with checked arithmetic.
    pub fn checked_add(self, other: Self) -> Result<Self, VisualFieldResourceEstimateError> {
        Ok(Self {
            coordinate_bytes: checked_add(self.coordinate_bytes, other.coordinate_bytes)?,
            channel_bytes: checked_add(self.channel_bytes, other.channel_bytes)?,
            count_field_bytes: checked_add(self.count_field_bytes, other.count_field_bytes)?,
            derived_field_bytes: checked_add(self.derived_field_bytes, other.derived_field_bytes)?,
            transition_bytes: checked_add(self.transition_bytes, other.transition_bytes)?,
            staging_bytes: checked_add(self.staging_bytes, other.staging_bytes)?,
        })
    }

    /// Converts the aggregate to the existing dataset admission reservation seam.
    ///
    /// CPU-side ownership is intentionally left to the caller; this owner accounts
    /// only the checked GPU bytes it can prove.
    pub fn to_resource_reservation(
        self,
    ) -> Result<ResourceReservation, VisualFieldResourceEstimateError> {
        Ok(ResourceReservation {
            ram_bytes: 0,
            vram_bytes: self.total_bytes()?,
        })
    }
}

pub fn estimate_visual_field_resources(
    grid: GridSize,
    options: VisualFieldResourceOptions,
) -> Result<VisualFieldResourceEstimate, VisualFieldResourceEstimateError> {
    VisualFieldResourceEstimate::for_grid_with_options(grid, options)
}

pub fn aggregate_visual_field_resources(
    estimates: impl IntoIterator<Item = VisualFieldResourceEstimate>,
) -> Result<VisualFieldResourceEstimate, VisualFieldResourceEstimateError> {
    estimates
        .into_iter()
        .try_fold(VisualFieldResourceEstimate::default(), |total, estimate| {
            total.checked_add(estimate)
        })
}

fn checked_mul(left: u64, right: u64) -> Result<u64, VisualFieldResourceEstimateError> {
    left.checked_mul(right)
        .ok_or(VisualFieldResourceEstimateError::ArithmeticOverflow)
}

fn checked_add(left: u64, right: u64) -> Result<u64, VisualFieldResourceEstimateError> {
    left.checked_add(right)
        .ok_or(VisualFieldResourceEstimateError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_resource_estimate_uses_checked_arithmetic() {
        let grid = GridSize::new(8, 4);
        let estimate = VisualFieldResourceEstimate::for_grid_with_options(
            grid,
            VisualFieldResourceOptions {
                category_layers: 2,
                ridge_fields: 3,
                transition_fields: 2,
                readback: true,
                bind_pipeline_bytes: 128,
            },
        )
        .unwrap();
        assert_eq!(estimate.coordinate_bytes, 256);
        assert_eq!(estimate.channel_bytes, 128);
        assert_eq!(estimate.count_field_bytes, 256);
        assert_eq!(estimate.derived_field_bytes, 640);
        assert_eq!(estimate.transition_bytes, 256);
        assert_eq!(estimate.staging_bytes, 256);

        let overflow = VisualFieldResourceEstimate::for_grid_with_options(
            grid,
            VisualFieldResourceOptions {
                bind_pipeline_bytes: u64::MAX,
                ..VisualFieldResourceOptions::default()
            },
        );
        assert_eq!(
            overflow,
            Err(VisualFieldResourceEstimateError::ArithmeticOverflow)
        );

        let aggregate = aggregate_visual_field_resources([estimate, estimate]).unwrap();
        assert_eq!(aggregate.count_field_bytes, 512);
        assert_eq!(aggregate.to_resource_reservation().unwrap().ram_bytes, 0);
    }
}
