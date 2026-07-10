//! Simple visible scatter-density rendering for the native workbench proof.

use bytemuck::{Pod, Zeroable};

use rawscope_core::F32Range;
use rawscope_data::ScatterPointRecord;

use crate::density_render_pipeline::{
    create_density_render_bind_group, create_density_render_bind_group_layout,
    create_density_render_pipeline,
};
use crate::gpu_scatter_density::{
    dispatch_scatter_density, GpuScatterDensityError, ScatterDensityComputeConfig,
};
use crate::{DensityEncoding, PlotRectPx, ScatterDensityPresentation};

const RENDER_SHADER_SOURCE: &str = include_str!("shaders/scatter_density_render.wgsl");

/// Render stats needed by the workbench title and density colour scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterDensityRenderStats {
    pub point_count: usize,
    pub grid_width: u32,
    pub grid_height: u32,
    pub max_bin_count: u32,
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
}

/// Renders a precomputed GPU scatter-density count buffer to a surface view.
pub struct ScatterDensityRenderer {
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    params_buffer: wgpu::Buffer,
    stats: ScatterDensityRenderStats,
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
        let compute_config = ScatterDensityComputeConfig {
            x_range: config.x_range,
            y_range: config.y_range,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
        };
        let compute_output = dispatch_scatter_density(device, queue, points, compute_config, true)?;
        let counts = compute_output
            .counts
            .ok_or(GpuScatterDensityError::MissingReadbackCounts)?;
        let max_bin_count = counts.iter().copied().max().unwrap_or(0);

        let render_params = ScatterDensityRenderParams {
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            max_bin_count,
            transform_id: config.encoding.transform.shader_id(),
            palette_id: config.encoding.palette.shader_id(),
            presentation_id: config.presentation.shader_id(),
            _padding: [0; 2],
        };
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
        let bind_group_layout = create_density_render_bind_group_layout(
            device,
            "RawScope Scatter Density Render Bind Group Layout",
        );
        let bind_group = create_density_render_bind_group(
            device,
            "RawScope Scatter Density Render Bind Group",
            &bind_group_layout,
            &compute_output.count_buffer,
            &params_buffer,
        );
        let pipeline = create_density_render_pipeline(
            device,
            "RawScope Scatter Density Render Pipeline Layout",
            "RawScope Scatter Density Render Pipeline",
            &bind_group_layout,
            &shader,
            surface_format,
        );

        Ok(Self {
            bind_group_layout,
            pipeline,
            bind_group,
            params_buffer,
            stats: ScatterDensityRenderStats {
                point_count: points.len(),
                grid_width: config.grid_width,
                grid_height: config.grid_height,
                max_bin_count,
            },
        })
    }

    /// Recomputes density counts for the provided viewport/config on the existing device.
    pub fn update_density(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[ScatterPointRecord],
        config: ScatterDensityRendererConfig,
    ) -> Result<ScatterDensityRenderStats, GpuScatterDensityError> {
        let compute_config = ScatterDensityComputeConfig {
            x_range: config.x_range,
            y_range: config.y_range,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
        };
        let compute_output = dispatch_scatter_density(device, queue, points, compute_config, true)?;
        let counts = compute_output
            .counts
            .ok_or(GpuScatterDensityError::MissingReadbackCounts)?;
        let max_bin_count = counts.iter().copied().max().unwrap_or(0);

        let render_params = ScatterDensityRenderParams {
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            max_bin_count,
            transform_id: config.encoding.transform.shader_id(),
            palette_id: config.encoding.palette.shader_id(),
            presentation_id: config.presentation.shader_id(),
            _padding: [0; 2],
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&render_params));
        self.bind_group = create_density_render_bind_group(
            device,
            "RawScope Scatter Density Render Bind Group",
            &self.bind_group_layout,
            &compute_output.count_buffer,
            &self.params_buffer,
        );
        self.stats = ScatterDensityRenderStats {
            point_count: points.len(),
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            max_bin_count,
        };

        Ok(self.stats)
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
        render_pass.set_bind_group(0, &self.bind_group, &[]);
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

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ScatterDensityRenderParams {
    grid_width: u32,
    grid_height: u32,
    max_bin_count: u32,
    transform_id: u32,
    palette_id: u32,
    presentation_id: u32,
    _padding: [u32; 2],
}

#[cfg(test)]
mod tests {
    use crate::DensityTransform;

    #[test]
    fn density_intensity_maps_empty_bins_to_zero() {
        assert_eq!(
            crate::density_intensity(0, 12, DensityTransform::Log1p),
            0.0
        );
        assert_eq!(crate::density_intensity(4, 0, DensityTransform::Log1p), 0.0);
    }

    #[test]
    fn log1p_density_intensity_maps_max_count_to_one() {
        assert_eq!(
            crate::density_intensity(12, 12, DensityTransform::Log1p),
            1.0
        );
    }

    #[test]
    fn log1p_density_intensity_keeps_mid_counts_visible() {
        let linear_midpoint = 0.25;
        let log_scaled = crate::density_intensity(1, 4, DensityTransform::Log1p);

        assert!(log_scaled > linear_midpoint);
        assert!(log_scaled < 1.0);
    }
}
