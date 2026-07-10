//! Simple visible timeline-density rendering for the native workbench proof.

use bytemuck::{Pod, Zeroable};
use rawscope_core::U64Range;
use rawscope_data::TimelineEventRecord;

use crate::density_render_pipeline::{
    create_density_render_bind_group, create_density_render_bind_group_layout,
    create_density_render_pipeline,
};
use crate::gpu_timeline_density::{
    dispatch_timeline_density, GpuTimelineDensityError, TimelineDensityComputeConfig,
};
use crate::{DensityEncoding, PlotRectPx};

const RENDER_SHADER_SOURCE: &str = include_str!("shaders/timeline_density_render.wgsl");

/// Render stats needed by the workbench title and density colour scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineDensityRenderStats {
    pub event_count: usize,
    pub lane_count: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub max_bin_count: u32,
}

/// Configuration for the first visible timeline-density proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineDensityRendererConfig {
    pub time_range: U64Range,
    pub lane_count: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub encoding: DensityEncoding,
}

impl TimelineDensityRendererConfig {
    /// Creates a timeline-density render config for a non-empty bin grid.
    pub fn new(time_range: U64Range, lane_count: u32, grid_width: u32, grid_height: u32) -> Self {
        assert!(lane_count > 0, "lane_count must be positive");
        assert!(grid_width > 0, "grid width must be positive");
        assert!(grid_height > 0, "grid height must be positive");

        Self {
            time_range,
            lane_count,
            grid_width,
            grid_height,
            encoding: DensityEncoding::timeline_default(),
        }
    }

    /// Returns this config with an explicit density color encoding.
    pub fn with_encoding(mut self, encoding: DensityEncoding) -> Self {
        self.encoding = encoding;
        self
    }
}

/// Renders a precomputed GPU timeline-density count buffer to a surface view.
pub struct TimelineDensityRenderer {
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    params_buffer: wgpu::Buffer,
    stats: TimelineDensityRenderStats,
}

impl TimelineDensityRenderer {
    /// Computes timeline-density counts and creates a renderer using the provided device/queue.
    ///
    /// The count buffer is produced on the same device used later for rendering. A one-time
    /// readback is used for log-scaled colour normalization.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        events: &[TimelineEventRecord],
        config: TimelineDensityRendererConfig,
    ) -> Result<Self, GpuTimelineDensityError> {
        let compute_config = TimelineDensityComputeConfig {
            time_range: config.time_range,
            lane_count: config.lane_count,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
        };
        let compute_output =
            dispatch_timeline_density(device, queue, events, compute_config, true)?;
        let counts = compute_output
            .counts
            .ok_or(GpuTimelineDensityError::MissingReadbackCounts)?;
        let max_bin_count = counts.iter().copied().max().unwrap_or(0);

        let render_params = TimelineDensityRenderParams {
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            max_bin_count,
            transform_id: config.encoding.transform.shader_id(),
            palette_id: config.encoding.palette.shader_id(),
            _padding: [0; 3],
        };
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Timeline Density Render Params Buffer"),
            size: std::mem::size_of::<TimelineDensityRenderParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&render_params));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RawScope Timeline Density Render Shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_SHADER_SOURCE.into()),
        });
        let bind_group_layout = create_density_render_bind_group_layout(
            device,
            "RawScope Timeline Density Render Bind Group Layout",
        );
        let bind_group = create_density_render_bind_group(
            device,
            "RawScope Timeline Density Render Bind Group",
            &bind_group_layout,
            &compute_output.count_buffer,
            &params_buffer,
        );
        let pipeline = create_density_render_pipeline(
            device,
            "RawScope Timeline Density Render Pipeline Layout",
            "RawScope Timeline Density Render Pipeline",
            &bind_group_layout,
            &shader,
            surface_format,
        );

        Ok(Self {
            bind_group_layout,
            pipeline,
            bind_group,
            params_buffer,
            stats: TimelineDensityRenderStats {
                event_count: events.len(),
                lane_count: config.lane_count,
                grid_width: config.grid_width,
                grid_height: config.grid_height,
                max_bin_count,
            },
        })
    }

    /// Recomputes timeline-density counts for a new visible time range.
    pub fn update_density(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        events: &[TimelineEventRecord],
        config: TimelineDensityRendererConfig,
    ) -> Result<TimelineDensityRenderStats, GpuTimelineDensityError> {
        let compute_config = TimelineDensityComputeConfig {
            time_range: config.time_range,
            lane_count: config.lane_count,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
        };
        let compute_output =
            dispatch_timeline_density(device, queue, events, compute_config, true)?;
        let counts = compute_output
            .counts
            .ok_or(GpuTimelineDensityError::MissingReadbackCounts)?;
        let max_bin_count = counts.iter().copied().max().unwrap_or(0);

        let render_params = TimelineDensityRenderParams {
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            max_bin_count,
            transform_id: config.encoding.transform.shader_id(),
            palette_id: config.encoding.palette.shader_id(),
            _padding: [0; 3],
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&render_params));
        self.bind_group = create_density_render_bind_group(
            device,
            "RawScope Timeline Density Render Bind Group",
            &self.bind_group_layout,
            &compute_output.count_buffer,
            &self.params_buffer,
        );
        self.stats = TimelineDensityRenderStats {
            event_count: events.len(),
            lane_count: config.lane_count,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            max_bin_count,
        };

        Ok(self.stats)
    }

    /// Returns the current render stats.
    pub fn stats(&self) -> TimelineDensityRenderStats {
        self.stats
    }

    /// Encodes one timeline-density render pass clipped to the physical plot.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RawScope Timeline Density Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.012,
                        g: 0.015,
                        b: 0.030,
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
struct TimelineDensityRenderParams {
    grid_width: u32,
    grid_height: u32,
    max_bin_count: u32,
    transform_id: u32,
    palette_id: u32,
    _padding: [u32; 3],
}
