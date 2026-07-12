//! Checked configuration for bounded analytical summaries.

use std::{error::Error, fmt, num::NonZeroU32};

const MAX_SUMMARY_BINS: u64 = 16_777_216;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryConfig {
    x_bins: NonZeroU32,
    y_bins: NonZeroU32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryConfigError {
    ZeroBinCount { axis: SummaryAxis },
    TooManyBins { requested: u64, maximum: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryAxis {
    X,
    Y,
}

impl SummaryConfig {
    pub fn new(x_bins: u32, y_bins: u32) -> Result<Self, SummaryConfigError> {
        let x_bins = NonZeroU32::new(x_bins).ok_or(SummaryConfigError::ZeroBinCount {
            axis: SummaryAxis::X,
        })?;
        let y_bins = NonZeroU32::new(y_bins).ok_or(SummaryConfigError::ZeroBinCount {
            axis: SummaryAxis::Y,
        })?;
        let requested = u64::from(x_bins.get()) * u64::from(y_bins.get());
        if requested > MAX_SUMMARY_BINS {
            return Err(SummaryConfigError::TooManyBins {
                requested,
                maximum: MAX_SUMMARY_BINS,
            });
        }
        Ok(Self { x_bins, y_bins })
    }

    pub const fn x_bins(self) -> NonZeroU32 {
        self.x_bins
    }

    pub const fn y_bins(self) -> NonZeroU32 {
        self.y_bins
    }

    pub const fn total_bins(self) -> u64 {
        self.x_bins.get() as u64 * self.y_bins.get() as u64
    }
}

impl fmt::Display for SummaryConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroBinCount { axis } => {
                write!(formatter, "{axis:?} summary bin count must be positive")
            }
            Self::TooManyBins { requested, maximum } => write!(
                formatter,
                "summary requests {requested} bins; maximum is {maximum}"
            ),
        }
    }
}

impl Error for SummaryConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_configuration_rejects_zero_and_unbounded_grids() {
        assert_eq!(
            SummaryConfig::new(0, 4),
            Err(SummaryConfigError::ZeroBinCount {
                axis: SummaryAxis::X
            })
        );
        assert!(matches!(
            SummaryConfig::new(u32::MAX, u32::MAX),
            Err(SummaryConfigError::TooManyBins { .. })
        ));
    }

    #[test]
    fn summary_configuration_reports_checked_bin_product() {
        let config = SummaryConfig::new(256, 128).unwrap();
        assert_eq!(config.total_bins(), 32_768);
        assert_eq!(config.x_bins().get(), 256);
        assert_eq!(config.y_bins().get(), 128);
    }
}
