//! Explicit source-to-GPU numeric quantization disclosure.

use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceNumericType {
    I64,
    U64,
    F32,
    F64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericDomainV1 {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuNumericEncoding {
    F32,
    U32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuantizationDisclosureV1 {
    pub source_numeric_type: SourceNumericType,
    pub source_domain: NumericDomainV1,
    pub gpu_domain: NumericDomainV1,
    pub encoding: GpuNumericEncoding,
    pub maximum_absolute_error: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuantizationError {
    InvalidDomain,
    InvalidErrorBound,
}

impl fmt::Display for QuantizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidDomain => "quantization domains must be finite and increasing",
            Self::InvalidErrorBound => "quantization error bound must be finite and non-negative",
        })
    }
}

impl Error for QuantizationError {}

impl QuantizationDisclosureV1 {
    pub fn validate(self) -> Result<(), QuantizationError> {
        for domain in [self.source_domain, self.gpu_domain] {
            if !domain.min.is_finite() || !domain.max.is_finite() || domain.max <= domain.min {
                return Err(QuantizationError::InvalidDomain);
            }
        }
        if self
            .maximum_absolute_error
            .is_some_and(|error| !error.is_finite() || error < 0.0)
        {
            return Err(QuantizationError::InvalidErrorBound);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disclosure() -> QuantizationDisclosureV1 {
        QuantizationDisclosureV1 {
            source_numeric_type: SourceNumericType::F64,
            source_domain: NumericDomainV1 { min: 0.0, max: 1.0 },
            gpu_domain: NumericDomainV1 { min: 0.0, max: 1.0 },
            encoding: GpuNumericEncoding::F32,
            maximum_absolute_error: Some(0.000001),
        }
    }

    #[test]
    fn quantization_requires_finite_ordered_domains() {
        assert!(disclosure().validate().is_ok());
        let invalid = QuantizationDisclosureV1 {
            source_domain: NumericDomainV1 { min: 1.0, max: 1.0 },
            ..disclosure()
        };
        assert_eq!(invalid.validate(), Err(QuantizationError::InvalidDomain));
    }

    #[test]
    fn quantization_rejects_negative_error_bounds() {
        let invalid = QuantizationDisclosureV1 {
            maximum_absolute_error: Some(-1.0),
            ..disclosure()
        };
        assert_eq!(
            invalid.validate(),
            Err(QuantizationError::InvalidErrorBound)
        );
    }
}
