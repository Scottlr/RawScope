//! Instanced screen-space glyph rendering for settled point-reveal selections.

use bytemuck::{Pod, Zeroable};
use rawscope_analysis::visual_field::PointRevealPlan;
use rawscope_core::{F32Range, RowId};
use rawscope_data::ScatterPointRecord;
use std::num::NonZeroU64;

use crate::{
    PlotRectPx, PointRevealConfig, PointRevealPresentationFrame, PointRevealSelection,
    PointRevealStats,
};

const SHADER_SOURCE: &str = include_str!("shaders/scatter_point_reveal.wgsl");
const MIN_RADIUS_PX: f32 = 1.0;
const MAX_RADIUS_PX: f32 = 6.0;

pub struct ScatterPointRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    point_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
    point_capacity: usize,
    rendered_count: u32,
    stats: PointRevealStats,
    x_range: F32Range,
    y_range: F32Range,
    radius_px: f32,
    emphasized_row_id: Option<RowId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointRevealPlanRenderError {
    RowIdNotResident { row_id: RowId },
    CountOverflow,
}

impl std::fmt::Display for PointRevealPlanRenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RowIdNotResident { row_id } => {
                write!(
                    formatter,
                    "point reveal row {row_id:?} is not resident in the projected buffer"
                )
            }
            Self::CountOverflow => {
                formatter.write_str("point reveal disclosure count overflowed usize")
            }
        }
    }
}

impl std::error::Error for PointRevealPlanRenderError {}

impl ScatterPointRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let point_buffer = create_point_buffer(device, 1);
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Scatter Point Reveal Params"),
            size: std::mem::size_of::<PointRevealParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = create_bind_group_layout(device);
        let bind_group =
            create_bind_group(device, &bind_group_layout, &point_buffer, &params_buffer);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RawScope Scatter Point Reveal Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RawScope Scatter Point Reveal Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RawScope Scatter Point Reveal Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            point_buffer,
            params_buffer,
            point_capacity: 1,
            rendered_count: 0,
            stats: PointRevealStats {
                eligible_count: 0,
                rendered_count: 0,
                sampled: false,
                blend: 0.0,
            },
            x_range: F32Range::new(0.0, 1.0),
            y_range: F32Range::new(0.0, 1.0),
            radius_px: PointRevealConfig::default().radius_px,
            emphasized_row_id: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_selection(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[ScatterPointRecord],
        selection: &PointRevealSelection,
        x_range: F32Range,
        y_range: F32Range,
        config: PointRevealConfig,
    ) {
        let packed = selection
            .point_indices
            .iter()
            .filter_map(|index| points.get(*index as usize))
            .map(GpuRevealPoint::from)
            .collect::<Vec<_>>();
        if packed.len() > self.point_capacity {
            self.point_capacity = packed.len().next_power_of_two();
            self.point_buffer = create_point_buffer(device, self.point_capacity);
            self.bind_group = create_bind_group(
                device,
                &self.bind_group_layout,
                &self.point_buffer,
                &self.params_buffer,
            );
        }
        if !packed.is_empty() {
            queue.write_buffer(&self.point_buffer, 0, bytemuck::cast_slice(&packed));
        }
        self.rendered_count = packed.len() as u32;
        self.stats = selection.stats();
        self.x_range = x_range;
        self.y_range = y_range;
        self.radius_px = config.radius_px.clamp(MIN_RADIUS_PX, MAX_RADIUS_PX);
    }

    /// Upload one already-settled row plan.  Resolution is by the existing
    /// row-id keyed point buffer; stale or invalid identities are rejected
    /// instead of being silently dropped.
    pub fn update_plan<G: Copy>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[ScatterPointRecord],
        plan: &PointRevealPlan<G>,
        x_range: F32Range,
        y_range: F32Range,
        config: PointRevealConfig,
    ) -> Result<(), PointRevealPlanRenderError> {
        let mut packed = Vec::with_capacity(plan.row_ids().len());
        for row_id in plan.row_ids().iter().copied() {
            let point = points
                .binary_search_by_key(&row_id, |point| point.row_id)
                .ok()
                .and_then(|index| points.get(index))
                .ok_or(PointRevealPlanRenderError::RowIdNotResident { row_id })?;
            packed.push(GpuRevealPoint::from(point));
        }
        if packed.len() > self.point_capacity {
            self.point_capacity = packed.len().next_power_of_two();
            self.point_buffer = create_point_buffer(device, self.point_capacity);
            self.bind_group = create_bind_group(
                device,
                &self.bind_group_layout,
                &self.point_buffer,
                &self.params_buffer,
            );
        }
        if !packed.is_empty() {
            queue.write_buffer(&self.point_buffer, 0, bytemuck::cast_slice(&packed));
        }
        self.rendered_count = packed.len() as u32;
        self.stats = PointRevealStats {
            eligible_count: usize::try_from(plan.eligible_count())
                .map_err(|_| PointRevealPlanRenderError::CountOverflow)?,
            rendered_count: packed.len(),
            sampled: plan.sampled(),
            blend: 1.0,
        };
        self.x_range = x_range;
        self.y_range = y_range;
        self.radius_px = config.radius_px.clamp(MIN_RADIUS_PX, MAX_RADIUS_PX);
        Ok(())
    }

    pub fn set_emphasized_row(&mut self, row_id: Option<RowId>) {
        self.emphasized_row_id = row_id;
    }

    /// Reproject the resident point plan into a new viewport without touching
    /// row membership or rebuilding the plan.
    pub fn set_viewport(&mut self, x_range: F32Range, y_range: F32Range) {
        self.x_range = x_range;
        self.y_range = y_range;
    }

    pub fn hide(&mut self) {
        self.rendered_count = 0;
        self.stats.rendered_count = 0;
        self.stats.blend = 0.0;
    }

    pub fn stats(&self) -> PointRevealStats {
        self.stats
    }

    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
    ) {
        self.render_with_frame(
            queue,
            encoder,
            target_view,
            plot_rect,
            PointRevealPresentationFrame::point_only(1.0),
        );
    }

    pub fn render_with_transition_alpha(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
        transition_alpha: f32,
    ) {
        self.render_with_frame(
            queue,
            encoder,
            target_view,
            plot_rect,
            PointRevealPresentationFrame::point_only(transition_alpha),
        );
    }

    /// Render the resident point plan with an already-resolved semantic zoom
    /// frame.  This path only writes a small uniform and submits the resident
    /// point buffer; it never scans rows or rebuilds the selection.
    pub fn render_with_frame(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
        frame: PointRevealPresentationFrame,
    ) {
        let point_alpha = frame.apply_points(self.stats.blend);
        if self.rendered_count == 0 || point_alpha <= 0.0 {
            return;
        }
        let params = PointRevealParams::new(
            self.x_range,
            self.y_range,
            plot_rect,
            self.radius_px,
            point_alpha,
            self.emphasized_row_id,
        );
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RawScope Scatter Point Reveal Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_viewport(
            plot_rect.x as f32,
            plot_rect.y as f32,
            plot_rect.width as f32,
            plot_rect.height as f32,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(plot_rect.x, plot_rect.y, plot_rect.width, plot_rect.height);
        pass.draw(0..6, 0..self.rendered_count);
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuRevealPoint {
    x: f32,
    y: f32,
    row_id_low: u32,
    row_id_high: u32,
}

const GPU_REVEAL_POINT_SIZE_BYTES: u64 = std::mem::size_of::<GpuRevealPoint>() as u64;

impl From<&ScatterPointRecord> for GpuRevealPoint {
    fn from(point: &ScatterPointRecord) -> Self {
        Self {
            x: point.x,
            y: point.y,
            row_id_low: point.row_id.0 as u32,
            row_id_high: (point.row_id.0 >> 32) as u32,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct PointRevealParams {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    plot_width_px: f32,
    plot_height_px: f32,
    radius_px: f32,
    point_alpha: f32,
    emphasized_low: u32,
    emphasized_high: u32,
    has_emphasis: u32,
    _padding: u32,
}

const POINT_REVEAL_PARAMS_SIZE_BYTES: u64 = std::mem::size_of::<PointRevealParams>() as u64;

impl PointRevealParams {
    fn new(
        x_range: F32Range,
        y_range: F32Range,
        plot_rect: PlotRectPx,
        radius_px: f32,
        blend: f32,
        emphasized: Option<RowId>,
    ) -> Self {
        let emphasized_value = emphasized.map_or(0, |row_id| row_id.0);
        Self {
            x_min: x_range.min,
            x_max: x_range.max,
            y_min: y_range.min,
            y_max: y_range.max,
            plot_width_px: plot_rect.width as f32,
            plot_height_px: plot_rect.height as f32,
            radius_px,
            point_alpha: blend,
            emphasized_low: emphasized_value as u32,
            emphasized_high: (emphasized_value >> 32) as u32,
            has_emphasis: u32::from(emphasized.is_some()),
            _padding: 0,
        }
    }
}

fn create_point_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("RawScope Scatter Point Reveal Vertices"),
        size: (capacity * std::mem::size_of::<GpuRevealPoint>()).max(16) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Scatter Point Reveal Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(GPU_REVEAL_POINT_SIZE_BYTES),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(POINT_REVEAL_PARAMS_SIZE_BYTES),
                },
                count: None,
            },
        ],
    })
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    point_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Scatter Point Reveal Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: point_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_reveal_abis_match_wgsl_scalar_layout() {
        assert_eq!(std::mem::size_of::<GpuRevealPoint>(), 16);
        assert_eq!(std::mem::align_of::<GpuRevealPoint>(), 4);
        assert_eq!(std::mem::size_of::<PointRevealParams>(), 48);
        assert_eq!(std::mem::align_of::<PointRevealParams>(), 4);
        assert_eq!(std::mem::size_of::<PointRevealParams>() % 16, 0);
    }

    #[test]
    fn point_reveal_params_preserve_plot_and_high_row_id() {
        let params = PointRevealParams::new(
            F32Range::new(0.0, 10.0),
            F32Range::new(20.0, 30.0),
            PlotRectPx::try_new(0, 0, 800, 600, 800, 600).unwrap(),
            2.0,
            0.5,
            Some(RowId(u64::from(u32::MAX) + 7)),
        );
        assert_eq!(
            (params.plot_width_px, params.plot_height_px),
            (800.0, 600.0)
        );
        assert_eq!((params.emphasized_low, params.emphasized_high), (6, 1));
    }
}
