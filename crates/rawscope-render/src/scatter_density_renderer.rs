//! Simple visible scatter-density rendering for the native workbench proof.

use rawscope_core::F32Range;
use rawscope_data::{FilterMask, FilterRevision, ScatterPointRecord};

use crate::density_render_pipeline::create_density_render_pipeline;
use crate::gpu_scatter_density::GpuScatterDensityError;
use crate::{
    DensityEncoding, DensityFieldViewport, DensityQualityTier, DensityReadbackPolicy, PlotRectPx,
    ReliefFieldConfig, ScatterDensityGpuState, ScatterDensityPresentation, ScatterDensityUpdate,
};

const RENDER_SHADER_SOURCE: &str = include_str!("shaders/scatter_density_render.wgsl");

#[path = "scatter_density_render_resources.rs"]
mod resources;
use resources::{
    scatter_render_bind_group, scatter_render_bind_group_layout, ScatterDensityRenderParams,
};

/// Render stats needed by the workbench title and density colour scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterDensityRenderStats {
    pub point_count: usize,
    pub grid_width: u32,
    pub grid_height: u32,
    pub max_bin_count: u32,
    pub max_bin_count_is_current: bool,
}

/// Configuration for the first visible scatter-density proof.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterDensityRendererConfig {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
    pub encoding: DensityEncoding,
    pub presentation: ScatterDensityPresentation,
    pub relief: ReliefFieldConfig,
}

impl ScatterDensityRendererConfig {
    /// Creates a scatter-density render config for a non-empty bin grid.
    pub fn new(x_range: F32Range, y_range: F32Range, grid_width: u32, grid_height: u32) -> Self {
        assert!(grid_width > 0, "grid width must be positive");
        assert!(grid_height > 0, "grid height must be positive");

        Self {
            x_range,
            y_range,
            grid_width,
            grid_height,
            encoding: DensityEncoding::scatter_default(),
            presentation: ScatterDensityPresentation::ExactCells,
            relief: ReliefFieldConfig::default(),
        }
    }

    /// Returns this config with an explicit density color encoding.
    pub fn with_encoding(mut self, encoding: DensityEncoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// Returns this config with an explicit fragment-stage presentation mode.
    pub fn with_presentation(mut self, presentation: ScatterDensityPresentation) -> Self {
        self.presentation = presentation;
        self
    }

    pub fn with_relief(mut self, relief: ReliefFieldConfig) -> Self {
        self.relief = relief;
        self
    }
}

/// Renders a precomputed GPU scatter-density count buffer to a surface view.
pub struct ScatterDensityRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_groups: [wgpu::BindGroup; 2],
    params_buffer: wgpu::Buffer,
    gpu_state: ScatterDensityGpuState,
    stats: ScatterDensityRenderStats,
    completed_field: DensityFieldViewport,
    display_x_range: F32Range,
    display_y_range: F32Range,
    config: ScatterDensityRendererConfig,
}

impl ScatterDensityRenderer {
    /// Computes scatter-density counts and creates a renderer using the provided device/queue.
    ///
    /// The count buffer is produced on the same device used later for rendering. A one-time
    /// readback is used for log-scaled colour normalization.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        points: &[ScatterPointRecord],
        config: ScatterDensityRendererConfig,
    ) -> Result<Self, GpuScatterDensityError> {
        let mut gpu_state = ScatterDensityGpuState::new(device, queue, points, config, 0)?;
        let output = gpu_state.update_with_output(
            device,
            queue,
            ScatterDensityUpdate {
                config,
                readback: DensityReadbackPolicy::MaxOnly,
            },
        )?;
        let max_bin_count = output.stats.max_bin_count;

        let completed_field = DensityFieldViewport {
            x_range: config.x_range,
            y_range: config.y_range,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            viewport_revision: 0,
            quality: DensityQualityTier::Exact,
        };
        let render_params = ScatterDensityRenderParams::new(
            config,
            max_bin_count,
            completed_field,
            config.x_range,
            config.y_range,
        );
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Scatter Density Render Params Buffer"),
            size: std::mem::size_of::<ScatterDensityRenderParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&render_params));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RawScope Scatter Density Render Shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_SHADER_SOURCE.into()),
        });
        let bind_group_layout = scatter_render_bind_group_layout(device);
        let bind_groups = std::array::from_fn(|index| {
            scatter_render_bind_group(
                device,
                &bind_group_layout,
                gpu_state.count_buffer(index),
                &params_buffer,
                gpu_state.max_count_buffer(),
            )
        });
        let pipeline = create_density_render_pipeline(
            device,
            "RawScope Scatter Density Render Pipeline Layout",
            "RawScope Scatter Density Render Pipeline",
            &bind_group_layout,
            &shader,
            surface_format,
        );

        Ok(Self {
            pipeline,
            bind_group_layout,
            bind_groups,
            params_buffer,
            gpu_state,
            stats: output.stats,
            completed_field,
            display_x_range: config.x_range,
            display_y_range: config.y_range,
            config,
        })
    }

    /// Recomputes density counts for the provided viewport/config on the existing device.
    pub fn update_density(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        update: ScatterDensityUpdate,
    ) -> Result<ScatterDensityRenderStats, GpuScatterDensityError> {
        let field = DensityFieldViewport {
            x_range: update.config.x_range,
            y_range: update.config.y_range,
            grid_width: update.config.grid_width,
            grid_height: update.config.grid_height,
            viewport_revision: self.completed_field.viewport_revision + 1,
            quality: DensityQualityTier::Exact,
        };
        self.update_density_for_field(device, queue, update, field)
    }

    pub fn update_density_for_field(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        update: ScatterDensityUpdate,
        field: DensityFieldViewport,
    ) -> Result<ScatterDensityRenderStats, GpuScatterDensityError> {
        let generation_before = self.gpu_state.grid_generation();
        let output = self.gpu_state.update_with_output(device, queue, update)?;
        if self.gpu_state.grid_generation() != generation_before {
            self.bind_groups = std::array::from_fn(|index| {
                scatter_render_bind_group(
                    device,
                    &self.bind_group_layout,
                    self.gpu_state.count_buffer(index),
                    &self.params_buffer,
                    self.gpu_state.max_count_buffer(),
                )
            });
        }
        let config = update.config;

        let render_params = ScatterDensityRenderParams::new(
            config,
            output.stats.max_bin_count,
            field,
            field.x_range,
            field.y_range,
        );
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&render_params));
        self.stats = output.stats;
        self.completed_field = field;
        self.display_x_range = field.x_range;
        self.display_y_range = field.y_range;
        self.config = config;

        Ok(self.stats)
    }

    pub fn set_display_viewport(
        &mut self,
        queue: &wgpu::Queue,
        x_range: F32Range,
        y_range: F32Range,
    ) {
        self.display_x_range = x_range;
        self.display_y_range = y_range;
        let params = ScatterDensityRenderParams::new(
            self.config,
            self.stats.max_bin_count,
            self.completed_field,
            x_range,
            y_range,
        );
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }

    pub fn completed_field(&self) -> DensityFieldViewport {
        self.completed_field
    }

    pub fn replace_dataset(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[ScatterPointRecord],
        dataset_revision: u64,
    ) -> Result<(), GpuScatterDensityError> {
        self.gpu_state
            .replace_dataset(device, queue, points, dataset_revision)
    }

    pub fn update_filter_mask(
        &mut self,
        queue: &wgpu::Queue,
        mask: &FilterMask,
        revision: FilterRevision,
    ) -> Result<bool, GpuScatterDensityError> {
        self.gpu_state
            .update_filter_mask(queue, mask.as_gpu_u32_slice(), revision)
    }

    /// Returns the current render stats.
    pub fn stats(&self) -> ScatterDensityRenderStats {
        self.stats
    }

    /// Encodes one scatter-density render pass clipped to the physical plot.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RawScope Scatter Density Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.015,
                        g: 0.025,
                        b: 0.035,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(
            0,
            &self.bind_groups[self.gpu_state.active_count_buffer_index()],
            &[],
        );
        render_pass.set_viewport(
            plot_rect.x as f32,
            plot_rect.y as f32,
            plot_rect.width as f32,
            plot_rect.height as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(plot_rect.x, plot_rect.y, plot_rect.width, plot_rect.height);
        render_pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
#[path = "scatter_density_renderer_tests.rs"]
mod tests;
