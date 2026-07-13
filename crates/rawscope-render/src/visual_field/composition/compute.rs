//! Compute pipeline and ABI for layer-major category aggregation.

use bytemuck::{Pod, Zeroable};

use super::resources::{CategoryChannelGpuResources, CategoryLayerPlanGpuResources};

const COMPOSITION_SHADER: &str = include_str!("../../shaders/visual_field_composition.wgsl");

/// Coordinate and dispatch parameters shared with exact scatter density.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct CompositionParams {
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub point_start: u32,
    pub dispatch_point_count: u32,
}

impl CompositionParams {
    pub const fn new(
        x_min: f32,
        x_max: f32,
        y_min: f32,
        y_max: f32,
        grid_width: u32,
        grid_height: u32,
        point_start: u32,
        dispatch_point_count: u32,
    ) -> Self {
        Self {
            x_min,
            x_max,
            y_min,
            y_max,
            grid_width,
            grid_height,
            point_start,
            dispatch_point_count,
        }
    }
}

/// ABI layout for the composition compute pass.
pub fn composition_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Category Composition Compute Layout"),
        entries: &[
            storage_entry(0, true),  // shared points
            uniform_entry(1),        // shared coordinate contract
            storage_entry(2, true),  // existing exact total field
            storage_entry(3, true),  // shared cohort mask
            storage_entry(4, true),  // stable row category codes
            storage_entry(5, true),  // bounded value-to-layer lookup
            storage_entry(6, true),  // special layer params
            storage_entry(7, false), // pending layer-major counts
        ],
    })
}

/// Builds the single composition compute pipeline. Layer count does not alter
/// the pipeline or bind-group layout; it is supplied in `SpecialLayerParams`.
pub fn composition_compute_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("RawScope Category Composition Shader"),
        source: wgpu::ShaderSource::Wgsl(COMPOSITION_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("RawScope Category Composition Pipeline Layout"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("RawScope Category Composition Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

/// Creates one bind group for a pending layer field. The exact count field and
/// point/mask buffers are supplied by the existing resident field owner; no
/// coordinates or total counts are copied into composition resources.
pub fn composition_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    points: &wgpu::Buffer,
    params: &wgpu::Buffer,
    total_counts: &wgpu::Buffer,
    filter_mask: &wgpu::Buffer,
    category_channel: &CategoryChannelGpuResources,
    layer_plan: &CategoryLayerPlanGpuResources,
    pending_layer_counts: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Category Composition Compute Bind Group"),
        layout,
        entries: &[
            buffer_entry(0, points),
            buffer_entry(1, params),
            buffer_entry(2, total_counts),
            buffer_entry(3, filter_mask),
            buffer_entry(4, category_channel.buffer()),
            buffer_entry(5, layer_plan.value_to_layer_buffer()),
            buffer_entry(6, layer_plan.special_layer_params_buffer()),
            buffer_entry(7, pending_layer_counts),
        ],
    })
}

/// Clears the pending layer-major field before a compute dispatch.
///
/// Clearing is explicit and occurs on the same command encoder as the atomic
/// binning pass so a pending publication can never retain counts from a prior
/// plan or generation.
pub fn clear_pending_layer_counts(
    encoder: &mut wgpu::CommandEncoder,
    pending_layer_counts: &wgpu::Buffer,
    allocated_bytes: u64,
) {
    encoder.clear_buffer(pending_layer_counts, 0, Some(allocated_bytes));
}

fn buffer_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: std::num::NonZeroU64::new(
                std::mem::size_of::<CompositionParams>() as u64
            ),
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_params_match_wgsl_uniform_alignment() {
        assert_eq!(std::mem::size_of::<CompositionParams>(), 32);
        assert_eq!(std::mem::align_of::<CompositionParams>(), 4);
        assert_eq!(std::mem::size_of::<CompositionParams>() % 16, 0);
    }
}
