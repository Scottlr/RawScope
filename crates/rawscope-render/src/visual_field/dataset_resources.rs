//! Generation-tagged metadata for one immutable visual-field dataset upload.

use rawscope_analysis::visual_field::VisualFieldMapping;
use rawscope_data::DatasetGeneration;
use rawscope_gpu::DeviceGeneration;

/// Identity and accounting for one validated visual-field dataset allocation.
///
/// The concrete WGPU buffers are owned by [`super::ResidentExactField`]. Keeping
/// the mapping beside the allocation identity prevents a field from being
/// presented with axes that came from a different projection.
#[derive(Debug, Default)]
struct VisualFieldDatasetGpuResourceHandles {
    point_buffer: Option<wgpu::Buffer>,
    row_mapping_buffer: Option<wgpu::Buffer>,
}

/// Immutable identity and private GPU handles for one visual-field dataset.
#[derive(Debug, Clone)]
pub struct VisualFieldDatasetGpuResources {
    handles: std::sync::Arc<VisualFieldDatasetGpuResourceHandles>,
    dataset_generation: DatasetGeneration,
    device_generation: DeviceGeneration,
    mapping: VisualFieldMapping,
    point_count: u64,
    allocated_bytes: u64,
}

impl PartialEq for VisualFieldDatasetGpuResources {
    fn eq(&self, other: &Self) -> bool {
        self.dataset_generation == other.dataset_generation
            && self.device_generation == other.device_generation
            && self.mapping == other.mapping
            && self.point_count == other.point_count
            && self.allocated_bytes == other.allocated_bytes
    }
}

impl Eq for VisualFieldDatasetGpuResources {}

impl std::hash::Hash for VisualFieldDatasetGpuResources {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.dataset_generation.hash(state);
        self.device_generation.hash(state);
        self.mapping.hash(state);
        self.point_count.hash(state);
        self.allocated_bytes.hash(state);
    }
}

impl VisualFieldDatasetGpuResources {
    pub fn new(
        dataset_generation: DatasetGeneration,
        device_generation: DeviceGeneration,
        mapping: VisualFieldMapping,
        point_count: u64,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            handles: std::sync::Arc::new(VisualFieldDatasetGpuResourceHandles::default()),
            dataset_generation,
            device_generation,
            mapping,
            point_count,
            allocated_bytes,
        }
    }

    pub const fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }

    pub const fn device_generation(&self) -> DeviceGeneration {
        self.device_generation
    }

    pub const fn mapping(&self) -> VisualFieldMapping {
        self.mapping
    }

    pub const fn point_count(&self) -> u64 {
        self.point_count
    }

    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    pub fn has_gpu_buffers(&self) -> bool {
        self.handles.point_buffer.is_some() && self.handles.row_mapping_buffer.is_some()
    }
}
