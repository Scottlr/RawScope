//! Row-alignment validation shared by masked analytical helpers.

use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskAlignmentError {
    pub record_count: usize,
    pub mask_len: usize,
}

impl MaskAlignmentError {
    pub(crate) fn require(record_count: usize, mask_len: usize) -> Result<(), Self> {
        if record_count == mask_len {
            Ok(())
        } else {
            Err(Self {
                record_count,
                mask_len,
            })
        }
    }
}

impl fmt::Display for MaskAlignmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "filter mask length {} does not match record count {}",
            self.mask_len, self.record_count
        )
    }
}

impl Error for MaskAlignmentError {}
