//! Simple visible scatter-density rendering for the native workbench proof.

use rawscope_core::F32Range;
use rawscope_data::{FilterMask, FilterRevision};
use rawscope_gpu::DeviceGeneration;
use std::sync::Arc;

use super::exact_field::{DensityReadbackPolicy, ResidentExactField, ResidentExactFieldUpdate};
use super::generation::VisualFieldQuality;
use super::gpu::{VisualFieldCountGrid, VisualFieldGpuError};
use super::point_pack::VisualFieldPoint;
use super::reprojection::VisualFieldViewport;
use crate::density_render_pipeline::create_density_render_pipeline;
use crate::{DensityEncoding, ReliefFieldConfig, ScatterDensityPresentation};

const RENDER_SHADER_SOURCE: &str = include_str!("../../shaders/scatter_density_render.wgsl");

#[path = "resources.rs"]
mod resources;
use resources::{density_render_bind_group, density_render_bind_group_layout, DensityRenderParams};

mod exact;
mod relief;
mod topographic;

pub use relief::relief_normal_from_samples;

fn presentation_shader_id(presentation: ScatterDensityPresentation) -> u32 {
    match presentation {
        ScatterDensityPresentation::ExactCells => exact::EXACT_PRESENTATION.shader_id(),
        ScatterDensityPresentation::TopographicField => {
            topographic::TOPOGRAPHIC_PRESENTATION.shader_id()
        }
        ScatterDensityPresentation::ReliefField => presentation.shader_id(),
    }
}

/// Render stats needed by the workbench title and density colour scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DensityPresentationRenderStats {
    pub point_count: usize,
    pub grid_width: u32,
    pub grid_height: u32,
    pub max_bin_count: u32,
    pub max_bin_count_is_current: bool,
}

/// Configuration for the first visible scatter-density proof.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DensityPresentationConfig {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
    pub encoding: DensityEncoding,
    pub presentation: ScatterDensityPresentation,
    pub relief: ReliefFieldConfig,
}

impl DensityPresentationConfig {
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
pub struct DensityPresentation {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) bind_groups: [wgpu::BindGroup; 2],
    pub(super) params_buffer: wgpu::Buffer,
    pub(super) gpu_state: ResidentExactField,
    pub(super) stats: DensityPresentationRenderStats,
    pub(super) completed_field: VisualFieldViewport,
    pub(super) display_x_range: F32Range,
    pub(super) display_y_range: F32Range,
    pub(super) config: DensityPresentationConfig,
    pub(super) previous_config: DensityPresentationConfig,
    pub(super) previous_stats: DensityPresentationRenderStats,
    pub(super) previous_field: VisualFieldViewport,
    pub(super) transition_progress: f32,
    pub(super) palette: Arc<crate::PaletteGpuResources>,
}

impl DensityPresentation {
    /// Computes scatter-density counts and creates a renderer using the provided device/queue.
    ///
    /// The count buffer is produced on the same device used later for rendering. A one-time
    /// readback is used for log-scaled colour normalization.
    pub fn new<T: VisualFieldPoint>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        points: &[T],
        config: DensityPresentationConfig,
    ) -> Result<Self, VisualFieldGpuError> {
        Self::new_with_palette(
            device,
            queue,
            surface_format,
            points,
            config,
            Arc::new(crate::PaletteGpuResources::new(
                device,
                queue,
                DeviceGeneration(0),
            )),
        )
    }

    pub fn new_with_palette<T: VisualFieldPoint>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        points: &[T],
        config: DensityPresentationConfig,
        palette: Arc<crate::PaletteGpuResources>,
    ) -> Result<Self, VisualFieldGpuError> {
        let mut gpu_state = ResidentExactField::new(device, queue, points, config, 0)?;
        let output = gpu_state.update_with_output(
            device,
            queue,
            ResidentExactFieldUpdate {
                config,
                readback: DensityReadbackPolicy::MaxOnly,
            },
        )?;
        let max_bin_count = output.stats.max_bin_count;

        let completed_field = VisualFieldViewport {
            x_range: config.x_range,
            y_range: config.y_range,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            viewport_revision: 0,
            quality: VisualFieldQuality::Exact,
        };
        let render_params = DensityRenderParams::new(
            config,
            max_bin_count,
            completed_field,
            config.x_range,
            config.y_range,
            config,
            max_bin_count,
            completed_field,
            1.0,
        );
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Scatter Density Render Params Buffer"),
            size: std::mem::size_of::<DensityRenderParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&render_params));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RawScope Scatter Density Render Shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_SHADER_SOURCE.into()),
        });
        let bind_group_layout = density_render_bind_group_layout(device);
        let bind_groups = std::array::from_fn(|index| {
            density_render_bind_group(
                device,
                &bind_group_layout,
                gpu_state.count_buffer(index),
                &params_buffer,
                gpu_state.max_count_buffer(),
                gpu_state.count_buffer(1 - index),
                &palette,
            )
        });
        let pipeline = create_density_render_pipeline(
            device,
            "RawScope Scatter Density Render Pipeline Layout",
            "RawScope Scatter Density Render Pipeline",
            &bind_group_layout,
            &shader,
            surface_format,
            constant_crossfade_blend(),
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
            previous_config: config,
            previous_stats: output.stats,
            previous_field: completed_field,
            transition_progress: 1.0,
            palette,
        })
    }

    /// Recomputes density counts for the provided viewport/config on the existing device.
    pub fn update_density(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        update: ResidentExactFieldUpdate,
    ) -> Result<DensityPresentationRenderStats, VisualFieldGpuError> {
        let field = VisualFieldViewport {
            x_range: update.config.x_range,
            y_range: update.config.y_range,
            grid_width: update.config.grid_width,
            grid_height: update.config.grid_height,
            viewport_revision: self.completed_field.viewport_revision + 1,
            quality: VisualFieldQuality::Exact,
        };
        self.update_density_for_field(device, queue, update, field)
    }

    pub fn update_density_for_field(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        update: ResidentExactFieldUpdate,
        field: VisualFieldViewport,
    ) -> Result<DensityPresentationRenderStats, VisualFieldGpuError> {
        let generation_before = self.gpu_state.grid_generation();
        let previous_config = self.config;
        let previous_stats = self.stats;
        let previous_field = self.completed_field;
        let output = self.gpu_state.update_with_output(device, queue, update)?;
        if self.gpu_state.grid_generation() != generation_before {
            self.bind_groups = std::array::from_fn(|index| {
                density_render_bind_group(
                    device,
                    &self.bind_group_layout,
                    self.gpu_state.count_buffer(index),
                    &self.params_buffer,
                    self.gpu_state.max_count_buffer(),
                    self.gpu_state.count_buffer(1 - index),
                    &self.palette,
                )
            });
        }
        let config = update.config;

        let render_params = DensityRenderParams::new(
            config,
            output.stats.max_bin_count,
            field,
            field.x_range,
            field.y_range,
            previous_config,
            previous_stats.max_bin_count,
            previous_field,
            0.0,
        );
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&render_params));
        self.stats = output.stats;
        self.completed_field = field;
        self.display_x_range = field.x_range;
        self.display_y_range = field.y_range;
        self.config = config;
        self.previous_config = previous_config;
        self.previous_stats = previous_stats;
        self.previous_field = previous_field;
        self.transition_progress = 0.0;

        Ok(self.stats)
    }

    /// Starts a nonblocking full-count readback for the resident active field.
    pub fn begin_full_readback(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), VisualFieldGpuError> {
        self.gpu_state.begin_full_readback(device, queue)
    }

    /// Advances a previously started full-count readback without waiting.
    pub fn poll_full_readback(
        &mut self,
        device: &wgpu::Device,
    ) -> Result<Option<VisualFieldCountGrid>, VisualFieldGpuError> {
        self.gpu_state.poll_full_readback(device).map(|counts| {
            counts.map(|counts| {
                VisualFieldCountGrid::new(self.config.grid_width, self.config.grid_height, counts)
            })
        })
    }

    pub fn cancel_full_readback(&mut self) {
        self.gpu_state.cancel_full_readback();
    }

    pub fn has_pending_full_readback(&self) -> bool {
        self.gpu_state.has_pending_full_readback()
    }

    pub fn set_display_viewport(
        &mut self,
        queue: &wgpu::Queue,
        x_range: F32Range,
        y_range: F32Range,
    ) {
        self.display_x_range = x_range;
        self.display_y_range = y_range;
        let params = DensityRenderParams::new(
            self.config,
            self.stats.max_bin_count,
            self.completed_field,
            x_range,
            y_range,
            self.previous_config,
            self.previous_stats.max_bin_count,
            self.previous_field,
            self.transition_progress,
        );
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }

    pub fn set_transition_progress(&mut self, queue: &wgpu::Queue, progress: f32) {
        self.transition_progress = progress.clamp(0.0, 1.0);
        let params = DensityRenderParams::new(
            self.config,
            self.stats.max_bin_count,
            self.completed_field,
            self.display_x_range,
            self.display_y_range,
            self.previous_config,
            self.previous_stats.max_bin_count,
            self.previous_field,
            self.transition_progress,
        );
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }

    pub fn completed_field(&self) -> VisualFieldViewport {
        self.completed_field
    }

    pub fn replace_dataset<T: VisualFieldPoint>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[T],
        dataset_revision: u64,
    ) -> Result<(), VisualFieldGpuError> {
        self.gpu_state
            .replace_dataset(device, queue, points, dataset_revision)
    }

    pub fn validate_dataset<T: VisualFieldPoint>(
        &self,
        points: &[T],
    ) -> Result<(), VisualFieldGpuError> {
        self.gpu_state.validate_dataset(points)
    }

    pub fn update_filter_mask(
        &mut self,
        queue: &wgpu::Queue,
        mask: &FilterMask,
        revision: FilterRevision,
    ) -> Result<bool, VisualFieldGpuError> {
        self.gpu_state
            .update_filter_mask(queue, mask.as_gpu_u32_slice(), revision)
    }

    pub fn validate_filter_mask(&self, mask: &FilterMask) -> Result<(), VisualFieldGpuError> {
        self.gpu_state.validate_filter_mask(mask.as_gpu_u32_slice())
    }

    /// Returns the current render stats.
    pub fn stats(&self) -> DensityPresentationRenderStats {
        self.stats
    }
}

fn constant_crossfade_blend() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Constant,
            dst_factor: wgpu::BlendFactor::OneMinusConstant,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent::OVER,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
