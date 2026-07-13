//! GPU normalized-difference density rendering over resident scatter fields.

use bytemuck::{Pod, Zeroable};
use rawscope_core::F32Range;
use rawscope_data::{FilterMask, FilterRevision};

use super::super::density_presentation::DensityPresentationConfig;
use super::super::exact_field::{
    DensityReadbackPolicy, ResidentExactField, ResidentExactFieldUpdate,
};
use super::super::generation::VisualFieldQuality;
use super::super::gpu::VisualFieldGpuError;
use super::super::point_pack::VisualFieldPoint;
use super::super::reprojection::VisualFieldViewport;
use crate::{PlotRectPx, DIFFERENCE_FIXED_POINT_SCALE};

const REDUCTION_SHADER: &str = include_str!("../../shaders/scatter_difference_reduce.wgsl");
const RENDER_SHADER: &str = include_str!("../../shaders/scatter_difference_render.wgsl");
const REDUCTION_WORKGROUP_SIZE: u32 = 64;

use super::resources::{
    compute_pipeline, difference_bind_group, difference_bind_group_layout, render_pipeline,
};

use super::presentation::ComparisonFieldRenderStats;

pub struct ComparisonFieldRenderer {
    pub(super) baseline: ResidentExactField,
    pub(super) active: ResidentExactField,
    pub(super) reduction_pipeline: wgpu::ComputePipeline,
    pub(super) reduction_layout: wgpu::BindGroupLayout,
    pub(super) render_pipeline: wgpu::RenderPipeline,
    pub(super) render_layout: wgpu::BindGroupLayout,
    pub(super) max_abs_buffer: wgpu::Buffer,
    pub(super) params_buffer: wgpu::Buffer,
    pub(super) source_field: VisualFieldViewport,
    pub(super) display_x_range: F32Range,
    pub(super) display_y_range: F32Range,
    pub(super) baseline_total: u64,
    pub(super) active_total: u64,
    pub(super) baseline_recompute_count: u64,
}

impl ComparisonFieldRenderer {
    pub fn new<T: VisualFieldPoint>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        points: &[T],
        config: DensityPresentationConfig,
    ) -> Result<Self, VisualFieldGpuError> {
        let baseline = ResidentExactField::new(device, queue, points, config, 0)?;
        let active = ResidentExactField::new(device, queue, points, config, 0)?;
        let params_min_binding_size = std::mem::size_of::<DifferenceGpuParams>() as u64;
        let reduction_layout = difference_bind_group_layout(
            device,
            wgpu::ShaderStages::COMPUTE,
            params_min_binding_size,
        );
        let render_layout = difference_bind_group_layout(
            device,
            wgpu::ShaderStages::FRAGMENT,
            params_min_binding_size,
        );
        let reduction_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RawScope Difference Reduction Shader"),
            source: wgpu::ShaderSource::Wgsl(REDUCTION_SHADER.into()),
        });
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RawScope Difference Render Shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
        });
        let reduction_pipeline = compute_pipeline(
            device,
            &reduction_layout,
            &reduction_shader,
            "RawScope Difference Reduction Pipeline",
        );
        let render_pipeline =
            render_pipeline(device, &render_layout, &render_shader, surface_format);
        let max_abs_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Difference Maximum Absolute Delta"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Difference Params"),
            size: std::mem::size_of::<DifferenceGpuParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let source_field = VisualFieldViewport {
            x_range: config.x_range,
            y_range: config.y_range,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            viewport_revision: 0,
            quality: VisualFieldQuality::Exact,
        };
        let mut renderer = Self {
            baseline,
            active,
            reduction_pipeline,
            reduction_layout,
            render_pipeline,
            render_layout,
            max_abs_buffer,
            params_buffer,
            source_field,
            display_x_range: config.x_range,
            display_y_range: config.y_range,
            baseline_total: points.len() as u64,
            active_total: points.len() as u64,
            baseline_recompute_count: 0,
        };
        renderer.update_fields(device, queue, config, true, points.len() as u64, 0)?;
        Ok(renderer)
    }

    pub fn replace_dataset<T: VisualFieldPoint>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[T],
        dataset_revision: u64,
    ) -> Result<(), VisualFieldGpuError> {
        self.baseline
            .replace_dataset(device, queue, points, dataset_revision)?;
        self.active
            .replace_dataset(device, queue, points, dataset_revision)?;
        self.baseline_total = points.len() as u64;
        self.active_total = points.len() as u64;
        Ok(())
    }

    pub fn validate_dataset<T: VisualFieldPoint>(
        &self,
        points: &[T],
    ) -> Result<(), VisualFieldGpuError> {
        self.active.validate_dataset(points)
    }

    pub fn update_filter_mask(
        &mut self,
        queue: &wgpu::Queue,
        mask: &FilterMask,
        revision: FilterRevision,
    ) -> Result<bool, VisualFieldGpuError> {
        self.active
            .update_filter_mask(queue, mask.as_gpu_u32_slice(), revision)
    }

    pub fn validate_filter_mask(&self, mask: &FilterMask) -> Result<(), VisualFieldGpuError> {
        self.active.validate_filter_mask(mask.as_gpu_u32_slice())
    }

    pub fn update_fields(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: DensityPresentationConfig,
        baseline_dirty: bool,
        active_total: u64,
        viewport_revision: u64,
    ) -> Result<ComparisonFieldRenderStats, VisualFieldGpuError> {
        let update = ResidentExactFieldUpdate {
            config,
            readback: DensityReadbackPolicy::None,
        };
        if baseline_dirty {
            self.baseline.update(device, queue, update)?;
            self.baseline_recompute_count += 1;
        }
        self.active.update(device, queue, update)?;
        self.active_total = active_total;
        self.source_field = VisualFieldViewport {
            x_range: config.x_range,
            y_range: config.y_range,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            viewport_revision,
            quality: VisualFieldQuality::Exact,
        };
        self.display_x_range = config.x_range;
        self.display_y_range = config.y_range;
        self.dispatch_reduction(device, queue);
        Ok(self.stats())
    }

    pub fn set_display_viewport(&mut self, x_range: F32Range, y_range: F32Range) {
        self.display_x_range = x_range;
        self.display_y_range = y_range;
    }

    pub fn stats(&self) -> ComparisonFieldRenderStats {
        ComparisonFieldRenderStats {
            baseline_total: self.baseline_total,
            active_total: self.active_total,
            baseline_recompute_count: self.baseline_recompute_count,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
    ) {
        super::presentation::render(
            self,
            device,
            queue,
            encoder,
            target_view,
            plot_rect,
            true,
            1.0,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_blended(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
        clear: bool,
        opacity: f32,
    ) {
        super::presentation::render(
            self,
            device,
            queue,
            encoder,
            target_view,
            plot_rect,
            clear,
            opacity,
        );
    }

    fn dispatch_reduction(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let params = self.params();
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
        let bind_group = difference_bind_group(
            device,
            &self.reduction_layout,
            self.baseline
                .count_buffer(self.baseline.active_count_buffer_index()),
            self.active
                .count_buffer(self.active.active_count_buffer_index()),
            &self.max_abs_buffer,
            &self.params_buffer,
            "RawScope Difference Reduction Bind Group",
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RawScope Difference Reduction Encoder"),
        });
        encoder.clear_buffer(&self.max_abs_buffer, 0, None);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("RawScope Difference Reduction Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.reduction_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let bin_count = self.source_field.grid_width * self.source_field.grid_height;
        pass.dispatch_workgroups(bin_count.div_ceil(REDUCTION_WORKGROUP_SIZE), 1, 1);
        drop(pass);
        queue.submit(Some(encoder.finish()));
    }

    pub(super) fn params(&self) -> DifferenceGpuParams {
        DifferenceGpuParams {
            grid_width: self.source_field.grid_width,
            grid_height: self.source_field.grid_height,
            baseline_total: self.baseline_total as f32,
            active_total: self.active_total as f32,
            fixed_point_scale: DIFFERENCE_FIXED_POINT_SCALE,
            _padding: [0; 3],
            source_x_min: self.source_field.x_range.min,
            source_x_max: self.source_field.x_range.max,
            source_y_min: self.source_field.y_range.min,
            source_y_max: self.source_field.y_range.max,
            display_x_min: self.display_x_range.min,
            display_x_max: self.display_x_range.max,
            display_y_min: self.display_y_range.min,
            display_y_max: self.display_y_range.max,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct DifferenceGpuParams {
    grid_width: u32,
    grid_height: u32,
    baseline_total: f32,
    active_total: f32,
    fixed_point_scale: u32,
    _padding: [u32; 3],
    source_x_min: f32,
    source_x_max: f32,
    source_y_min: f32,
    source_y_max: f32,
    display_x_min: f32,
    display_x_max: f32,
    display_y_min: f32,
    display_y_max: f32,
}

#[cfg(test)]
mod abi_tests {
    use super::DifferenceGpuParams;

    #[test]
    fn difference_params_match_wgsl_uniform_alignment() {
        assert_eq!(std::mem::size_of::<DifferenceGpuParams>(), 64);
        assert_eq!(std::mem::align_of::<DifferenceGpuParams>(), 4);
        assert_eq!(std::mem::size_of::<DifferenceGpuParams>() % 16, 0);
    }
}
