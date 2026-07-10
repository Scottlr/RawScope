//! GPU normalized-difference density rendering over resident scatter fields.

use bytemuck::{Pod, Zeroable};
use rawscope_core::F32Range;
use rawscope_data::{FilterMask, FilterRevision, ScatterPointRecord};

use crate::{
    DensityFieldViewport, DensityQualityTier, DensityReadbackPolicy, GpuScatterDensityError,
    PlotRectPx, ScatterDensityGpuState, ScatterDensityRendererConfig, ScatterDensityUpdate,
    DIFFERENCE_FIXED_POINT_SCALE,
};

const REDUCTION_SHADER: &str = include_str!("shaders/scatter_difference_reduce.wgsl");
const RENDER_SHADER: &str = include_str!("shaders/scatter_difference_render.wgsl");
const REDUCTION_WORKGROUP_SIZE: u32 = 64;

#[path = "scatter_difference_resources.rs"]
mod resources;
use resources::{
    compute_pipeline, difference_bind_group, difference_bind_group_layout, render_pipeline,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterDifferenceRenderStats {
    pub baseline_total: u64,
    pub active_total: u64,
    pub baseline_recompute_count: u64,
}

pub struct ScatterDifferenceRenderer {
    baseline: ScatterDensityGpuState,
    active: ScatterDensityGpuState,
    reduction_pipeline: wgpu::ComputePipeline,
    reduction_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    render_layout: wgpu::BindGroupLayout,
    max_abs_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
    source_field: DensityFieldViewport,
    display_x_range: F32Range,
    display_y_range: F32Range,
    baseline_total: u64,
    active_total: u64,
    baseline_recompute_count: u64,
}

impl ScatterDifferenceRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        points: &[ScatterPointRecord],
        config: ScatterDensityRendererConfig,
    ) -> Result<Self, GpuScatterDensityError> {
        let baseline = ScatterDensityGpuState::new(device, queue, points, config, 0)?;
        let active = ScatterDensityGpuState::new(device, queue, points, config, 0)?;
        let reduction_layout = difference_bind_group_layout(device, wgpu::ShaderStages::COMPUTE);
        let render_layout = difference_bind_group_layout(device, wgpu::ShaderStages::FRAGMENT);
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
        let source_field = DensityFieldViewport {
            x_range: config.x_range,
            y_range: config.y_range,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            viewport_revision: 0,
            quality: DensityQualityTier::Exact,
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

    pub fn replace_dataset(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[ScatterPointRecord],
        dataset_revision: u64,
    ) -> Result<(), GpuScatterDensityError> {
        self.baseline
            .replace_dataset(device, queue, points, dataset_revision)?;
        self.active
            .replace_dataset(device, queue, points, dataset_revision)?;
        self.baseline_total = points.len() as u64;
        self.active_total = points.len() as u64;
        Ok(())
    }

    pub fn update_filter_mask(
        &mut self,
        queue: &wgpu::Queue,
        mask: &FilterMask,
        revision: FilterRevision,
    ) -> Result<bool, GpuScatterDensityError> {
        self.active
            .update_filter_mask(queue, mask.as_gpu_u32_slice(), revision)
    }

    pub fn update_fields(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: ScatterDensityRendererConfig,
        baseline_dirty: bool,
        active_total: u64,
        viewport_revision: u64,
    ) -> Result<ScatterDifferenceRenderStats, GpuScatterDensityError> {
        let update = ScatterDensityUpdate {
            config,
            readback: DensityReadbackPolicy::None,
        };
        if baseline_dirty {
            self.baseline.update(device, queue, update)?;
            self.baseline_recompute_count += 1;
        }
        self.active.update(device, queue, update)?;
        self.active_total = active_total;
        self.source_field = DensityFieldViewport {
            x_range: config.x_range,
            y_range: config.y_range,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            viewport_revision,
            quality: DensityQualityTier::Exact,
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

    pub fn stats(&self) -> ScatterDifferenceRenderStats {
        ScatterDifferenceRenderStats {
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
        self.render_blended(device, queue, encoder, target_view, plot_rect, true, 1.0);
    }

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
        let params = self.params();
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
        let bind_group = difference_bind_group(
            device,
            &self.render_layout,
            self.baseline
                .count_buffer(self.baseline.active_count_buffer_index()),
            self.active
                .count_buffer(self.active.active_count_buffer_index()),
            &self.max_abs_buffer,
            &self.params_buffer,
            "RawScope Difference Render Bind Group",
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RawScope Difference Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: if clear {
                        wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.024,
                            b: 0.03,
                            a: 1.0,
                        })
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.render_pipeline);
        let opacity = f64::from(opacity.clamp(0.0, 1.0));
        pass.set_blend_constant(wgpu::Color {
            r: opacity,
            g: opacity,
            b: opacity,
            a: opacity,
        });
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_viewport(
            plot_rect.x as f32,
            plot_rect.y as f32,
            plot_rect.width as f32,
            plot_rect.height as f32,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(plot_rect.x, plot_rect.y, plot_rect.width, plot_rect.height);
        pass.draw(0..3, 0..1);
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

    fn params(&self) -> DifferenceGpuParams {
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
struct DifferenceGpuParams {
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
