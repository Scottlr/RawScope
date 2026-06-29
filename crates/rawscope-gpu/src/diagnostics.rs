//! Adapter and device diagnostics for startup logging.

/// Basic GPU diagnostics captured during WGPU initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDiagnostics {
    pub adapter_name: String,
    pub backend: String,
    pub device_type: String,
    pub adapter_features: String,
    pub adapter_limits: String,
    pub surface_format: String,
    pub present_mode: String,
    pub alpha_mode: String,
}

impl GpuDiagnostics {
    pub(crate) fn from_parts(
        adapter_info: wgpu::AdapterInfo,
        adapter_features: wgpu::Features,
        adapter_limits: wgpu::Limits,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        Self {
            adapter_name: adapter_info.name,
            backend: format!("{:?}", adapter_info.backend),
            device_type: format!("{:?}", adapter_info.device_type),
            adapter_features: format!("{adapter_features:?}"),
            adapter_limits: format!("{adapter_limits:?}"),
            surface_format: format!("{:?}", surface_config.format),
            present_mode: format!("{:?}", surface_config.present_mode),
            alpha_mode: format!("{:?}", surface_config.alpha_mode),
        }
    }
}

/// Adapter and device diagnostics for headless compute contexts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeDiagnostics {
    pub adapter_name: String,
    pub backend: String,
    pub device_type: String,
    pub adapter_features: String,
    pub adapter_limits: String,
}

impl ComputeDiagnostics {
    pub(crate) fn from_parts(
        adapter_info: wgpu::AdapterInfo,
        adapter_features: wgpu::Features,
        adapter_limits: wgpu::Limits,
    ) -> Self {
        Self {
            adapter_name: adapter_info.name,
            backend: format!("{:?}", adapter_info.backend),
            device_type: format!("{:?}", adapter_info.device_type),
            adapter_features: format!("{adapter_features:?}"),
            adapter_limits: format!("{adapter_limits:?}"),
        }
    }
}
