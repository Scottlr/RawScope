//! Bounded GPU resources for category composition.

use std::{error::Error, fmt, num::NonZeroU8, sync::Arc};

use bytemuck::{Pod, Zeroable};
use rawscope_analysis::visual_field::CategoryLayerPlan;
use rawscope_core::{ColumnId, GridSize};
use rawscope_data::{CategoryCodeLayout, DatasetGeneration};
use rawscope_gpu::{
    CategoryCompositionResourcePlan, DeviceGeneration, GpuLimitError, GpuReadbackTicket,
    MAX_CATEGORY_COMPOSITION_LAYERS,
};

use super::super::generation::{VisualFieldGeneration, VisualFieldViewGeneration};
use super::layer_counts_match_total;

pub const NO_CATEGORY_LAYER: u32 = u32::MAX;

/// WGSL-compatible special-category mapping and layer count.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct SpecialLayerParams {
    pub untracked_layer: u32,
    pub missing_layer: u32,
    pub invalid_layer: u32,
    pub layer_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionResourceError {
    GpuLimit(GpuLimitError),
    RowCountOverflow,
    LayerPlanEmpty,
    LayerCountOverflow,
    LayerLookupOutOfRange {
        layer_id: u32,
        layer_count: u8,
    },
    DatasetGenerationMismatch,
    DeviceGenerationMismatch,
    CategoryColumnMismatch {
        expected: ColumnId,
        actual: ColumnId,
    },
    MappingHasNoCategory,
}

impl fmt::Display for CompositionResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GpuLimit(error) => write!(formatter, "category composition GPU limit: {error}"),
            Self::RowCountOverflow => formatter.write_str("category row count exceeds u64"),
            Self::LayerPlanEmpty => formatter.write_str("category layer plan contains no layers"),
            Self::LayerCountOverflow => {
                formatter.write_str("category layer plan exceeds the bounded layer count")
            }
            Self::LayerLookupOutOfRange {
                layer_id,
                layer_count,
            } => write!(
                formatter,
                "category layer lookup resolves to {layer_id}, outside {layer_count} layers"
            ),
            Self::DatasetGenerationMismatch => {
                formatter.write_str("category composition dataset generations do not match")
            }
            Self::DeviceGenerationMismatch => {
                formatter.write_str("category composition device generations do not match")
            }
            Self::CategoryColumnMismatch { expected, actual } => write!(
                formatter,
                "category composition column {actual:?} does not match mapping column {expected:?}"
            ),
            Self::MappingHasNoCategory => {
                formatter.write_str("category composition mapping has no category column")
            }
        }
    }
}

impl Error for CompositionResourceError {}

impl From<GpuLimitError> for CompositionResourceError {
    fn from(error: GpuLimitError) -> Self {
        Self::GpuLimit(error)
    }
}

/// Stable identity of the row-aligned category-code upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CategoryChannelIdentity {
    dataset_generation: DatasetGeneration,
    device_generation: DeviceGeneration,
    column_id: ColumnId,
}

impl CategoryChannelIdentity {
    pub const fn new(
        dataset_generation: DatasetGeneration,
        device_generation: DeviceGeneration,
        column_id: ColumnId,
    ) -> Self {
        Self {
            dataset_generation,
            device_generation,
            column_id,
        }
    }

    pub const fn dataset_generation(self) -> DatasetGeneration {
        self.dataset_generation
    }

    pub const fn device_generation(self) -> DeviceGeneration {
        self.device_generation
    }

    pub const fn column_id(self) -> ColumnId {
        self.column_id
    }
}

/// One reusable packed row-code channel for a validated category column.
pub struct CategoryChannelGpuResources {
    identity: CategoryChannelIdentity,
    category_codes_buffer: wgpu::Buffer,
    row_count: u64,
    code_layout: CategoryCodeLayout,
    allocated_bytes: u64,
}

impl CategoryChannelGpuResources {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        identity: CategoryChannelIdentity,
        row_codes: &[u32],
        code_layout: CategoryCodeLayout,
    ) -> Result<Self, CompositionResourceError> {
        let row_count = u64::try_from(row_codes.len())
            .map_err(|_| CompositionResourceError::RowCountOverflow)?;
        let allocated_bytes = bytes_for_entries(row_count)?;
        validate_storage_bytes(device, allocated_bytes)?;
        let category_codes_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Category Composition Row Codes"),
            size: allocated_bytes.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !row_codes.is_empty() {
            queue.write_buffer(&category_codes_buffer, 0, bytemuck::cast_slice(row_codes));
        }
        Ok(Self {
            identity,
            category_codes_buffer,
            row_count,
            code_layout,
            allocated_bytes,
        })
    }

    pub const fn identity(&self) -> CategoryChannelIdentity {
        self.identity
    }

    pub const fn dataset_generation(&self) -> DatasetGeneration {
        self.identity.dataset_generation
    }

    pub const fn device_generation(&self) -> DeviceGeneration {
        self.identity.device_generation
    }

    pub const fn column_id(&self) -> ColumnId {
        self.identity.column_id
    }

    pub const fn row_count(&self) -> u64 {
        self.row_count
    }

    pub const fn code_layout(&self) -> CategoryCodeLayout {
        self.code_layout
    }

    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.category_codes_buffer
    }

    /// Returns whether the existing upload can be reused without rewriting row codes.
    pub fn is_reusable_for(&self, self_identity: CategoryChannelIdentity) -> bool {
        self.identity == self_identity
    }
}

/// GPU-resident bounded value/special lookup for one category layer plan.
pub struct CategoryLayerPlanGpuResources {
    plan_generation: rawscope_analysis::visual_field::CategoryLayerPlanGeneration,
    dataset_generation: DatasetGeneration,
    device_generation: DeviceGeneration,
    column_id: ColumnId,
    value_to_layer_buffer: wgpu::Buffer,
    special_layer_params_buffer: wgpu::Buffer,
    value_to_layer: Arc<[u32]>,
    special_layer_params: SpecialLayerParams,
    layer_count: NonZeroU8,
    allocated_bytes: u64,
}

impl CategoryLayerPlanGpuResources {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        plan: &CategoryLayerPlan,
        device_generation: DeviceGeneration,
    ) -> Result<Self, CompositionResourceError> {
        let layer_count = NonZeroU8::new(
            u8::try_from(plan.layers().len())
                .map_err(|_| CompositionResourceError::LayerCountOverflow)?,
        )
        .ok_or(CompositionResourceError::LayerPlanEmpty)?;
        if layer_count.get() > MAX_CATEGORY_COMPOSITION_LAYERS {
            return Err(CompositionResourceError::LayerCountOverflow);
        }
        let value_to_layer: Arc<[u32]> = plan
            .lookup()
            .value_layers
            .iter()
            .copied()
            .map(|layer| validate_layer_id(layer.get(), layer_count.get()))
            .collect::<Result<Vec<_>, _>>()?
            .into();
        // Keep three fixed slots after tracked values so the storage array has
        // a meaningful length even when the source has no tracked values.
        // The shader can therefore resolve untracked, missing, and invalid
        // codes without reading an uninitialised padding word.
        let lookup_upload: Arc<[u32]> = value_to_layer
            .iter()
            .copied()
            .chain([
                special_layer_id(plan.lookup().untracked_layer, layer_count.get())?,
                special_layer_id(plan.lookup().missing_layer, layer_count.get())?,
                special_layer_id(plan.lookup().invalid_layer, layer_count.get())?,
            ])
            .collect::<Vec<_>>()
            .into();
        let special_layer_params = SpecialLayerParams {
            untracked_layer: special_layer_id(plan.lookup().untracked_layer, layer_count.get())?,
            missing_layer: special_layer_id(plan.lookup().missing_layer, layer_count.get())?,
            invalid_layer: special_layer_id(plan.lookup().invalid_layer, layer_count.get())?,
            layer_count: u32::from(layer_count.get()),
        };
        let lookup_bytes = bytes_for_entries(lookup_upload.len() as u64)?;
        let special_bytes = std::mem::size_of::<SpecialLayerParams>() as u64;
        validate_storage_bytes(device, lookup_bytes)?;
        validate_storage_bytes(device, special_bytes)?;
        let value_to_layer_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Category Composition Layer Lookup"),
            size: lookup_bytes.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !value_to_layer.is_empty() {
            queue.write_buffer(
                &value_to_layer_buffer,
                0,
                bytemuck::cast_slice(lookup_upload.as_ref()),
            );
        }
        let special_layer_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Category Composition Special Layers"),
            size: special_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &special_layer_params_buffer,
            0,
            bytemuck::bytes_of(&special_layer_params),
        );
        Ok(Self {
            plan_generation: plan.generation(),
            dataset_generation: plan.dataset_generation(),
            device_generation,
            column_id: plan.column_id(),
            value_to_layer_buffer,
            special_layer_params_buffer,
            value_to_layer,
            special_layer_params,
            layer_count,
            allocated_bytes: lookup_bytes.checked_add(special_bytes).ok_or(
                CompositionResourceError::GpuLimit(GpuLimitError::ArithmeticOverflow),
            )?,
        })
    }

    pub const fn plan_generation(
        &self,
    ) -> rawscope_analysis::visual_field::CategoryLayerPlanGeneration {
        self.plan_generation
    }

    pub const fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }

    pub const fn device_generation(&self) -> DeviceGeneration {
        self.device_generation
    }

    pub const fn column_id(&self) -> ColumnId {
        self.column_id
    }

    pub const fn layer_count(&self) -> NonZeroU8 {
        self.layer_count
    }

    pub fn value_to_layer(&self) -> &[u32] {
        &self.value_to_layer
    }

    pub const fn special_layer_params(&self) -> SpecialLayerParams {
        self.special_layer_params
    }

    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    pub fn value_to_layer_buffer(&self) -> &wgpu::Buffer {
        &self.value_to_layer_buffer
    }

    pub fn special_layer_params_buffer(&self) -> &wgpu::Buffer {
        &self.special_layer_params_buffer
    }
}

/// Identity checked before pending layer fields can become active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositionPublicationIdentity {
    pub dataset_generation: DatasetGeneration,
    pub device_generation: DeviceGeneration,
    pub cohort_generation: rawscope_analysis::cohort::CohortGeneration,
    pub view_generation: VisualFieldViewGeneration,
    pub plan_generation: rawscope_analysis::visual_field::CategoryLayerPlanGeneration,
    pub grid: GridSize,
}

/// Double-buffered layer-major count fields tied to one exact visual field.
pub struct CategoryCompositionFieldGeneration {
    exact_field: Arc<VisualFieldGeneration>,
    category_channel: Arc<CategoryChannelGpuResources>,
    layer_plan: Arc<CategoryLayerPlanGpuResources>,
    active_layer_counts: wgpu::Buffer,
    pending_layer_counts: wgpu::Buffer,
    readback: Option<GpuReadbackTicket<Vec<u32>>>,
    allocated_bytes: u64,
}

impl CategoryCompositionFieldGeneration {
    pub fn new(
        device: &wgpu::Device,
        exact_field: Arc<VisualFieldGeneration>,
        category_channel: Arc<CategoryChannelGpuResources>,
        layer_plan: Arc<CategoryLayerPlanGpuResources>,
    ) -> Result<Self, CompositionResourceError> {
        let category_column = exact_field
            .resources()
            .mapping()
            .category()
            .ok_or(CompositionResourceError::MappingHasNoCategory)?;
        if category_column != category_channel.column_id() {
            return Err(CompositionResourceError::CategoryColumnMismatch {
                expected: category_column,
                actual: category_channel.column_id(),
            });
        }
        if category_channel.dataset_generation() != exact_field.resources().dataset_generation()
            || layer_plan.dataset_generation() != exact_field.resources().dataset_generation()
        {
            return Err(CompositionResourceError::DatasetGenerationMismatch);
        }
        if category_channel.device_generation() != exact_field.resources().device_generation()
            || layer_plan.device_generation() != exact_field.resources().device_generation()
        {
            return Err(CompositionResourceError::DeviceGenerationMismatch);
        }
        if layer_plan.column_id() != category_channel.column_id() {
            return Err(CompositionResourceError::CategoryColumnMismatch {
                expected: category_channel.column_id(),
                actual: layer_plan.column_id(),
            });
        }
        let allocation = CategoryCompositionResourcePlan::for_grid(
            exact_field.grid_width(),
            exact_field.grid_height(),
            layer_plan.layer_count().get(),
            u64::from(category_channel.code_layout().tracked_value_count),
            category_channel.row_count(),
            &device.limits(),
        )?;
        let active_layer_counts = layer_count_buffer(device, allocation.layer_field_bytes);
        let pending_layer_counts = layer_count_buffer(device, allocation.layer_field_bytes);
        Ok(Self {
            exact_field,
            category_channel,
            layer_plan,
            active_layer_counts,
            pending_layer_counts,
            readback: None,
            allocated_bytes: allocation.allocated_bytes,
        })
    }

    pub fn identity(&self) -> CompositionPublicationIdentity {
        CompositionPublicationIdentity {
            dataset_generation: self.exact_field.resources().dataset_generation(),
            device_generation: self.exact_field.resources().device_generation(),
            cohort_generation: self.exact_field.cohort_generation(),
            view_generation: self.exact_field.view_generation(),
            plan_generation: self.layer_plan.plan_generation(),
            grid: self.exact_field.grid(),
        }
    }

    pub fn publish_if_current(
        &mut self,
        expected: CompositionPublicationIdentity,
    ) -> Result<(), CompositionPublicationError> {
        if self.identity() != expected {
            return Err(CompositionPublicationError::GenerationMismatch {
                expected,
                actual: self.identity(),
            });
        }
        std::mem::swap(
            &mut self.active_layer_counts,
            &mut self.pending_layer_counts,
        );
        Ok(())
    }

    /// Publishes a settled pending field only after its layer readback proves
    /// equality with the exact total field for every bin.
    pub fn publish_if_current_with_counts(
        &mut self,
        expected: CompositionPublicationIdentity,
        layer_counts: &[u32],
        exact_counts: &[u32],
    ) -> Result<(), CompositionPublicationError> {
        let actual = self.identity();
        if actual != expected {
            return Err(CompositionPublicationError::GenerationMismatch { expected, actual });
        }
        if !layer_counts_match_total(
            layer_counts,
            exact_counts,
            self.layer_plan.layer_count().get(),
        ) {
            return Err(CompositionPublicationError::LayerTotalsMismatch);
        }
        std::mem::swap(
            &mut self.active_layer_counts,
            &mut self.pending_layer_counts,
        );
        Ok(())
    }

    pub fn exact_field(&self) -> Arc<VisualFieldGeneration> {
        Arc::clone(&self.exact_field)
    }

    pub fn category_channel(&self) -> Arc<CategoryChannelGpuResources> {
        Arc::clone(&self.category_channel)
    }

    pub fn layer_plan(&self) -> Arc<CategoryLayerPlanGpuResources> {
        Arc::clone(&self.layer_plan)
    }

    pub fn active_layer_counts(&self) -> &wgpu::Buffer {
        &self.active_layer_counts
    }

    pub fn pending_layer_counts(&self) -> &wgpu::Buffer {
        &self.pending_layer_counts
    }

    pub const fn readback(&self) -> Option<&GpuReadbackTicket<Vec<u32>>> {
        self.readback.as_ref()
    }

    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionPublicationError {
    GenerationMismatch {
        expected: CompositionPublicationIdentity,
        actual: CompositionPublicationIdentity,
    },
    LayerTotalsMismatch,
}

impl fmt::Display for CompositionPublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GenerationMismatch { expected, actual } => write!(
                formatter,
                "category composition publication generation mismatch: expected {expected:?}, actual {actual:?}"
            ),
            Self::LayerTotalsMismatch => formatter.write_str(
                "category composition layer totals do not match the exact total field",
            ),
        }
    }
}

impl Error for CompositionPublicationError {}

fn special_layer_id(
    layer: Option<rawscope_analysis::visual_field::CategoryLayerId>,
    layer_count: u8,
) -> Result<u32, CompositionResourceError> {
    layer
        .map(|layer| validate_layer_id(layer.get(), layer_count))
        .transpose()
        .map(|value| value.unwrap_or(NO_CATEGORY_LAYER))
}

fn validate_layer_id(layer_id: u8, layer_count: u8) -> Result<u32, CompositionResourceError> {
    if layer_id >= layer_count {
        return Err(CompositionResourceError::LayerLookupOutOfRange {
            layer_id: u32::from(layer_id),
            layer_count,
        });
    }
    Ok(u32::from(layer_id))
}

fn bytes_for_entries(entries: u64) -> Result<u64, CompositionResourceError> {
    entries
        .checked_mul(4)
        .map(|bytes| bytes.max(4))
        .ok_or(CompositionResourceError::GpuLimit(
            GpuLimitError::ArithmeticOverflow,
        ))
}

fn validate_storage_bytes(
    device: &wgpu::Device,
    requested_bytes: u64,
) -> Result<(), CompositionResourceError> {
    let max_bytes = device
        .limits()
        .max_storage_buffer_binding_size
        .min(device.limits().max_buffer_size);
    if requested_bytes > max_bytes {
        return Err(CompositionResourceError::GpuLimit(
            GpuLimitError::BufferTooLarge {
                requested_bytes,
                max_bytes,
            },
        ));
    }
    Ok(())
}

fn layer_count_buffer(device: &wgpu::Device, bytes: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("RawScope Category Composition Layer Counts"),
        size: bytes.max(4),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_layer_params_match_wgsl_uniform_alignment() {
        assert_eq!(std::mem::size_of::<SpecialLayerParams>(), 16);
        assert_eq!(std::mem::align_of::<SpecialLayerParams>(), 4);
        assert_eq!(std::mem::size_of::<SpecialLayerParams>() % 16, 0);
    }

    #[test]
    fn lookup_sentinel_is_explicit_for_absent_special_layers() {
        assert_eq!(special_layer_id(None, 2).unwrap(), NO_CATEGORY_LAYER);
        assert_eq!(validate_layer_id(1, 2).unwrap(), 1);
        assert!(matches!(
            validate_layer_id(2, 2),
            Err(CompositionResourceError::LayerLookupOutOfRange { .. })
        ));
    }
}
