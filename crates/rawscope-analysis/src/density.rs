//! Checked density bin placement shared by CPU and GPU parity tests.

use std::{error::Error, fmt, num::NonZeroU32};

use rawscope_core::{F32Range, U64Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinIndex(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinPlacement {
    BeforeDomain,
    InDomain(BinIndex),
    AfterDomain,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinningError {
    NonFiniteValue,
    InvalidFloatDomain,
    InvalidBinCount,
}

impl fmt::Display for BinningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonFiniteValue => "bin value must be finite",
            Self::InvalidFloatDomain => "float bin domain must be finite and ordered",
            Self::InvalidBinCount => "bin count must be positive",
        })
    }
}

impl Error for BinningError {}

pub fn bin_f64(
    value: f64,
    min: f64,
    max: f64,
    bins: NonZeroU32,
) -> Result<BinPlacement, BinningError> {
    if !value.is_finite() {
        return Err(BinningError::NonFiniteValue);
    }
    if !min.is_finite() || !max.is_finite() || max <= min {
        return Err(BinningError::InvalidFloatDomain);
    }
    if value < min {
        return Ok(BinPlacement::BeforeDomain);
    }
    if value >= max {
        return Ok(if value == max {
            BinPlacement::InDomain(BinIndex(bins.get() - 1))
        } else {
            BinPlacement::AfterDomain
        });
    }
    let index = (((value - min) / (max - min)) * f64::from(bins.get())).floor() as u32;
    Ok(BinPlacement::InDomain(BinIndex(index.min(bins.get() - 1))))
}

pub fn bin_u64(
    value: u64,
    domain: U64Range,
    bins: NonZeroU32,
) -> Result<BinPlacement, BinningError> {
    if value < domain.min() {
        return Ok(BinPlacement::BeforeDomain);
    }
    if value > domain.max() {
        return Ok(BinPlacement::AfterDomain);
    }
    let offset = u128::from(value - domain.min());
    let span = u128::from(domain.span());
    let index = (offset * u128::from(bins.get()) / span) as u32;
    Ok(BinPlacement::InDomain(BinIndex(index.min(bins.get() - 1))))
}

pub fn bin_f32(
    value: f32,
    domain: F32Range,
    bins: NonZeroU32,
) -> Result<BinPlacement, BinningError> {
    bin_f64(
        f64::from(value),
        f64::from(domain.min),
        f64::from(domain.max),
        bins,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_boundaries_are_checked_and_classified() {
        let bins = NonZeroU32::new(4).unwrap();
        assert_eq!(
            bin_f64(-1.0, 0.0, 1.0, bins).unwrap(),
            BinPlacement::BeforeDomain
        );
        assert_eq!(
            bin_f64(1.0, 0.0, 1.0, bins).unwrap(),
            BinPlacement::InDomain(BinIndex(3))
        );
        assert_eq!(
            bin_f64(f64::NAN, 0.0, 1.0, bins),
            Err(BinningError::NonFiniteValue)
        );
    }

    #[test]
    fn u64_max_scale_uses_integer_arithmetic() {
        let bins = NonZeroU32::new(4).unwrap();
        let domain = U64Range::new(u64::MAX - 4, u64::MAX - 1);
        assert_eq!(
            bin_u64(u64::MAX, domain, bins).unwrap(),
            BinPlacement::AfterDomain
        );
        assert_eq!(
            bin_u64(domain.max(), domain, bins).unwrap(),
            BinPlacement::InDomain(BinIndex(3))
        );
    }
}
