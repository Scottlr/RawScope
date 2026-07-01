//! Adapter metadata selected during WGPU startup.

/// GPU adapter and surface metadata captured during WGPU initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuAdapterInfo {
    pub adapter_name: String,
    pub backend: String,
    pub device_type: String,
    pub surface_format: String,
    pub present_mode: String,
    pub alpha_mode: String,
}

impl GpuAdapterInfo {
    pub(crate) fn from_parts(
        adapter_info: wgpu::AdapterInfo,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        Self {
            adapter_name: adapter_info.name,
            backend: format!("{:?}", adapter_info.backend),
            device_type: format!("{:?}", adapter_info.device_type),
            surface_format: format!("{:?}", surface_config.format),
            present_mode: format!("{:?}", surface_config.present_mode),
            alpha_mode: format!("{:?}", surface_config.alpha_mode),
        }
    }
}

/// Adapter metadata for headless compute contexts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeAdapterInfo {
    pub adapter_name: String,
    pub backend: String,
    pub device_type: String,
}

impl ComputeAdapterInfo {
    pub(crate) fn from_parts(adapter_info: wgpu::AdapterInfo) -> Self {
        Self {
            adapter_name: adapter_info.name,
            backend: format!("{:?}", adapter_info.backend),
            device_type: format!("{:?}", adapter_info.device_type),
        }
    }
}
