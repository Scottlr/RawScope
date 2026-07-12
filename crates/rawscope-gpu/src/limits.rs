//! Checked GPU allocation and dispatch plans shared by low-level owners.

use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuResourcePlan {
    pub grid_width: u32,
    pub grid_height: u32,
    pub bin_count: u64,
    pub buffer_size_bytes: u64,
    pub dispatch_workgroups_x: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuLimitError {
    ZeroDimension,
    ArithmeticOverflow,
    BufferTooLarge {
        requested_bytes: u64,
        max_bytes: u64,
    },
    DispatchTooLarge {
        requested: u32,
        max: u32,
    },
}

impl fmt::Display for GpuLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => f.write_str("GPU resource dimensions must be positive"),
            Self::ArithmeticOverflow => f.write_str("GPU resource size arithmetic overflowed"),
            Self::BufferTooLarge {
                requested_bytes,
                max_bytes,
            } => write!(
                f,
                "GPU buffer requires {requested_bytes} bytes, device allows {max_bytes}"
            ),
            Self::DispatchTooLarge { requested, max } => write!(
                f,
                "GPU dispatch requires {requested} workgroups, device allows {max}"
            ),
        }
    }
}

impl Error for GpuLimitError {}

impl GpuResourcePlan {
    pub fn for_density(
        grid_width: u32,
        grid_height: u32,
        records: u64,
        limits: &wgpu::Limits,
    ) -> Result<Self, GpuLimitError> {
        if grid_width == 0 || grid_height == 0 {
            return Err(GpuLimitError::ZeroDimension);
        }
        let bin_count = u64::from(grid_width)
            .checked_mul(u64::from(grid_height))
            .ok_or(GpuLimitError::ArithmeticOverflow)?;
        let buffer_size_bytes = bin_count
            .checked_mul(4)
            .ok_or(GpuLimitError::ArithmeticOverflow)?;
        if buffer_size_bytes > limits.max_storage_buffer_binding_size {
            return Err(GpuLimitError::BufferTooLarge {
                requested_bytes: buffer_size_bytes,
                max_bytes: limits.max_storage_buffer_binding_size,
            });
        }
        let dispatch_workgroups_x =
            records
                .div_ceil(64)
                .try_into()
                .map_err(|_| GpuLimitError::DispatchTooLarge {
                    requested: u32::MAX,
                    max: limits.max_compute_workgroups_per_dimension,
                })?;
        if dispatch_workgroups_x > limits.max_compute_workgroups_per_dimension {
            return Err(GpuLimitError::DispatchTooLarge {
                requested: dispatch_workgroups_x,
                max: limits.max_compute_workgroups_per_dimension,
            });
        }
        Ok(Self {
            grid_width,
            grid_height,
            bin_count,
            buffer_size_bytes,
            dispatch_workgroups_x,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_plan_rejects_zero_and_over_limit_requests() {
        let limits = wgpu::Limits::downlevel_defaults();
        assert_eq!(
            GpuResourcePlan::for_density(0, 1, 1, &limits),
            Err(GpuLimitError::ZeroDimension)
        );
        assert!(matches!(
            GpuResourcePlan::for_density(u32::MAX, u32::MAX, 1, &limits),
            Err(GpuLimitError::BufferTooLarge { .. } | GpuLimitError::ArithmeticOverflow)
        ));
    }
}
