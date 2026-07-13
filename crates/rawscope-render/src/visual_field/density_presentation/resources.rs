//! Scatter-specific render uniforms and bind resources.

use bytemuck::{Pod, Zeroable};
use rawscope_core::F32Range;
use std::num::NonZeroU64;

use super::DensityPresentationConfig;
use crate::VisualFieldViewport;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(super) struct DensityRenderParams {
    grid_width: u32,
    grid_height: u32,
    max_bin_count: u32,
    transform_id: u32,
    palette_id: u32,
    presentation_id: u32,
    padding: [u32; 2],
    source_x_min: f32,
    source_x_max: f32,
    source_y_min: f32,
    source_y_max: f32,
    display_x_min: f32,
    display_x_max: f32,
    display_y_min: f32,
    display_y_max: f32,
    relief_height_strength: f32,
    relief_normal_radius_bins: u32,
    relief_light_azimuth_radians: f32,
    relief_light_elevation_radians: f32,
    relief_ambient_strength: f32,
    relief_shadow_strength: f32,
    relief_contour_strength: f32,
    relief_padding: f32,
    previous_max_bin_count: u32,
    previous_transform_id: u32,
    previous_palette_id: u32,
    previous_presentation_id: u32,
    previous_source_x_min: f32,
    previous_source_x_max: f32,
    previous_source_y_min: f32,
    previous_source_y_max: f32,
    transition_progress: f32,
    transition_padding: [f32; 3],
}

const DENSITY_RENDER_PARAMS_SIZE_BYTES: u64 = std::mem::size_of::<DensityRenderParams>() as u64;

impl DensityRenderParams {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        config: DensityPresentationConfig,
        max_bin_count: u32,
        source: VisualFieldViewport,
        display_x: F32Range,
        display_y: F32Range,
        previous_config: DensityPresentationConfig,
        previous_max_bin_count: u32,
        previous_source: VisualFieldViewport,
        transition_progress: f32,
    ) -> Self {
        Self {
            grid_width: source.grid_width,
            grid_height: source.grid_height,
            max_bin_count,
            transform_id: config.encoding.transform.shader_id(),
            palette_id: config.encoding.palette.shader_id(),
            presentation_id: super::presentation_shader_id(config.presentation),
            padding: [0; 2],
            source_x_min: source.x_range.min,
            source_x_max: source.x_range.max,
            source_y_min: source.y_range.min,
            source_y_max: source.y_range.max,
            display_x_min: display_x.min,
            display_x_max: display_x.max,
            display_y_min: display_y.min,
            display_y_max: display_y.max,
            relief_height_strength: config.relief.height_strength,
            relief_normal_radius_bins: config.relief.normal_radius_bins,
            relief_light_azimuth_radians: config.relief.light_azimuth_degrees.to_radians(),
            relief_light_elevation_radians: config.relief.light_elevation_degrees.to_radians(),
            relief_ambient_strength: config.relief.ambient_strength,
            relief_shadow_strength: config.relief.shadow_strength,
            relief_contour_strength: config.relief.contour_strength,
            relief_padding: 0.0,
            previous_max_bin_count,
            previous_transform_id: previous_config.encoding.transform.shader_id(),
            previous_palette_id: previous_config.encoding.palette.shader_id(),
            previous_presentation_id: super::presentation_shader_id(previous_config.presentation),
            previous_source_x_min: previous_source.x_range.min,
            previous_source_x_max: previous_source.x_range.max,
            previous_source_y_min: previous_source.y_range.min,
            previous_source_y_max: previous_source.y_range.max,
            transition_progress: transition_progress.clamp(0.0, 1.0),
            transition_padding: [0.0; 3],
        }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::DensityRenderParams;

    #[test]
    fn relief_render_params_are_wgsl_aligned() {
        assert_eq!(std::mem::size_of::<DensityRenderParams>(), 144);
        assert_eq!(std::mem::align_of::<DensityRenderParams>(), 4);
        assert_eq!(std::mem::size_of::<DensityRenderParams>() % 16, 0);
    }
}

pub(super) fn density_render_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Scatter Render Bind Group Layout"),
        entries: &[
            storage_layout_entry(0),
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(DENSITY_RENDER_PARAMS_SIZE_BYTES),
                },
                count: None,
            },
            storage_layout_entry(2),
            storage_layout_entry(3),
        ],
    })
}

fn storage_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub(super) fn density_render_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    counts: &wgpu::Buffer,
    params: &wgpu::Buffer,
    max_count: &wgpu::Buffer,
    previous_counts: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Scatter Render Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: counts.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: max_count.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: previous_counts.as_entire_binding(),
            },
        ],
    })
}
