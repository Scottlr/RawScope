//! Error types for WGPU bootstrap and clear-frame rendering.

use std::{error::Error, fmt};

use crate::DeviceLossReason;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuRuntimeSignal {
    UncapturedError {
        message: String,
    },
    DeviceLost {
        reason: DeviceLossReason,
        message: String,
    },
}

/// Errors returned by the RawScope GPU bootstrap layer.
#[derive(Debug)]
pub enum GpuError {
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapter(wgpu::RequestAdapterError),
    RequestDevice(wgpu::RequestDeviceError),
    ValidationScope {
        operation: &'static str,
        source: wgpu::Error,
    },
    MissingSurfaceConfig,
    SurfaceValidation,
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateSurface(err) => write!(f, "failed to create WGPU surface: {err}"),
            Self::RequestAdapter(err) => write!(f, "failed to select WGPU adapter: {err}"),
            Self::RequestDevice(err) => write!(f, "failed to create WGPU device: {err}"),
            Self::ValidationScope { operation, source } => {
                write!(f, "WGPU validation failed during {operation}: {source}")
            }
            Self::MissingSurfaceConfig => {
                write!(f, "selected WGPU adapter has no supported surface config")
            }
            Self::SurfaceValidation => write!(f, "WGPU surface validation failed"),
        }
    }
}

impl Error for GpuError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateSurface(err) => Some(err),
            Self::RequestAdapter(err) => Some(err),
            Self::RequestDevice(err) => Some(err),
            Self::ValidationScope { source, .. } => Some(source),
            Self::MissingSurfaceConfig | Self::SurfaceValidation => None,
        }
    }
}
