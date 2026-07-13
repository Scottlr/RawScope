//! Checked GPU allocation and dispatch plans shared by low-level owners.

use std::{error::Error, fmt};

/// Portable composition layer ceiling shared by the render-side plan.
pub const MAX_CATEGORY_COMPOSITION_LAYERS: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuResourcePlan {
    pub grid_width: u32,
    pub grid_height: u32,
    pub bin_count: u64,
    pub buffer_size_bytes: u64,
    pub dispatch_workgroups_x: u32,
}

/// Checked allocation plan for one resident category-composition field.
///
/// Composition keeps the row category channel and the bounded layer lookup
/// separate from the two layer-major count fields.  The latter are both live
/// during publication, so the estimate intentionally includes active and
/// pending storage rather than treating them as one reusable allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryCompositionResourcePlan {
    pub grid_width: u32,
    pub grid_height: u32,
    pub layer_count: u8,
    pub bin_count: u64,
    pub category_code_bytes: u64,
    pub lookup_bytes: u64,
    pub special_params_bytes: u64,
    pub layer_field_bytes: u64,
    pub readback_bytes: u64,
    pub allocated_bytes: u64,
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
    LayerCountZero,
    LayerCountTooLarge {
        requested: u8,
        max: u8,
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
            Self::LayerCountZero => f.write_str("category composition requires one layer"),
            Self::LayerCountTooLarge { requested, max } => write!(
                f,
                "category composition requests {requested} layers; maximum is {max}"
            ),
        }
    }
}

impl Error for GpuLimitError {}

impl GpuResourcePlan {
    pub fn for_grid(
        grid_width: u32,
        grid_height: u32,
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
        let max_allowed_buffer_bytes = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        if buffer_size_bytes > max_allowed_buffer_bytes {
            return Err(GpuLimitError::BufferTooLarge {
                requested_bytes: buffer_size_bytes,
                max_bytes: max_allowed_buffer_bytes,
            });
        }
        Ok(Self {
            grid_width,
            grid_height,
            bin_count,
            buffer_size_bytes,
            dispatch_workgroups_x: 0,
        })
    }

    pub fn for_density(
        grid_width: u32,
        grid_height: u32,
        records: u64,
        limits: &wgpu::Limits,
    ) -> Result<Self, GpuLimitError> {
        let mut plan = Self::for_grid(grid_width, grid_height, limits)?;
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
        plan.dispatch_workgroups_x = dispatch_workgroups_x;
        Ok(plan)
    }
}

impl CategoryCompositionResourcePlan {
    /// Builds a checked composition allocation plan for the supplied device.
    ///
    /// `tracked_value_count` is the length of the bounded value-to-layer
    /// lookup, while `row_count` controls the shared row-code channel and
    /// compute dispatch. Every individual storage/readback buffer is checked
    /// against the same backend limits used by the ordinary density plan.
    pub fn for_grid(
        grid_width: u32,
        grid_height: u32,
        layer_count: u8,
        tracked_value_count: u64,
        row_count: u64,
        limits: &wgpu::Limits,
    ) -> Result<Self, GpuLimitError> {
        if grid_width == 0 || grid_height == 0 {
            return Err(GpuLimitError::ZeroDimension);
        }
        if layer_count == 0 {
            return Err(GpuLimitError::LayerCountZero);
        }
        if layer_count > MAX_CATEGORY_COMPOSITION_LAYERS {
            return Err(GpuLimitError::LayerCountTooLarge {
                requested: layer_count,
                max: MAX_CATEGORY_COMPOSITION_LAYERS,
            });
        }
        let bin_count = u64::from(grid_width)
            .checked_mul(u64::from(grid_height))
            .ok_or(GpuLimitError::ArithmeticOverflow)?;
        let category_code_bytes = row_count
            .checked_mul(4)
            .ok_or(GpuLimitError::ArithmeticOverflow)?
            .max(4);
        // Special categories are carried by the fixed parameter block; this
        // lookup contains only the bounded tracked-value assignments.
        let lookup_bytes = tracked_value_count
            .checked_mul(4)
            .ok_or(GpuLimitError::ArithmeticOverflow)?
            .max(4);
        let special_params_bytes = 16;
        let layer_field_bytes = bin_count
            .checked_mul(u64::from(layer_count))
            .and_then(|bytes| bytes.checked_mul(4))
            .ok_or(GpuLimitError::ArithmeticOverflow)?;
        let readback_bytes = layer_field_bytes;
        let max_storage_buffer_bytes = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        for requested_bytes in [
            category_code_bytes,
            lookup_bytes,
            special_params_bytes,
            layer_field_bytes,
        ] {
            if requested_bytes > max_storage_buffer_bytes {
                return Err(GpuLimitError::BufferTooLarge {
                    requested_bytes,
                    max_bytes: max_storage_buffer_bytes,
                });
            }
        }
        if readback_bytes > limits.max_buffer_size {
            return Err(GpuLimitError::BufferTooLarge {
                requested_bytes: readback_bytes,
                max_bytes: limits.max_buffer_size,
            });
        }
        let dispatch_workgroups_x =
            row_count
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
        let allocated_bytes = category_code_bytes
            .checked_add(lookup_bytes)
            .and_then(|bytes| bytes.checked_add(special_params_bytes))
            .and_then(|bytes| bytes.checked_add(layer_field_bytes.checked_mul(2)?))
            .and_then(|bytes| bytes.checked_add(readback_bytes))
            .ok_or(GpuLimitError::ArithmeticOverflow)?;
        Ok(Self {
            grid_width,
            grid_height,
            layer_count,
            bin_count,
            category_code_bytes,
            lookup_bytes,
            special_params_bytes,
            layer_field_bytes,
            readback_bytes,
            allocated_bytes,
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

    #[test]
    fn resource_plan_honors_total_buffer_limit() {
        let mut limits = wgpu::Limits::downlevel_defaults();
        limits.max_storage_buffer_binding_size = u64::MAX;
        limits.max_buffer_size = 64;
        assert_eq!(
            GpuResourcePlan::for_density(5, 5, 1, &limits),
            Err(GpuLimitError::BufferTooLarge {
                requested_bytes: 100,
                max_bytes: 64,
            })
        );
    }

    #[test]
    fn grid_plan_reports_checked_output_size_without_dispatch_work() {
        let plan = GpuResourcePlan::for_grid(5, 5, &wgpu::Limits::downlevel_defaults()).unwrap();
        assert_eq!(plan.bin_count, 25);
        assert_eq!(plan.buffer_size_bytes, 100);
        assert_eq!(plan.dispatch_workgroups_x, 0);
    }

    #[test]
    fn composition_plan_accounts_for_both_layer_fields_and_readback() {
        let plan = CategoryCompositionResourcePlan::for_grid(
            4,
            2,
            3,
            5,
            64,
            &wgpu::Limits::downlevel_defaults(),
        )
        .unwrap();
        assert_eq!(plan.layer_field_bytes, 4 * 2 * 3 * 4);
        assert_eq!(plan.readback_bytes, plan.layer_field_bytes);
        assert_eq!(
            plan.allocated_bytes,
            64 * 4 + (5 + 3) * 4 + 16 + 3 * 4 * 2 * 4 + 3 * 4 * 2 * 4 + 3 * 4 * 2 * 4
        );
    }

    #[test]
    fn composition_plan_rejects_zero_layers_and_device_limit() {
        let limits = wgpu::Limits::downlevel_defaults();
        assert_eq!(
            CategoryCompositionResourcePlan::for_grid(4, 4, 0, 1, 1, &limits),
            Err(GpuLimitError::LayerCountZero)
        );
        let mut tiny = limits;
        tiny.max_storage_buffer_binding_size = 32;
        tiny.max_buffer_size = 32;
        assert!(matches!(
            CategoryCompositionResourcePlan::for_grid(4, 4, 2, 1, 1, &tiny),
            Err(GpuLimitError::BufferTooLarge { .. })
        ));
    }
}
