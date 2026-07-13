//! Static composition presentation over a settled layer-major field.
//!
//! Composition does not invent a colour for a mixture of categories.  Each
//! cell chooses the colour of its dominant real layer, uses the exact total
//! to control lightness, and moves that colour toward the reviewed neutral
//! when the distribution is impure.  The same bounded formula is mirrored in
//! `visual_field_composition_render.wgsl` for resident GPU presentation.

use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};

use super::super::palette::CategoryPaletteEntries;

const COMPOSITION_RENDER_SHADER: &str =
    include_str!("../../shaders/visual_field_composition_render.wgsl");

const DEFAULT_DENSITY_FLOOR: f32 = 0.12;
const DEFAULT_OPACITY: f32 = 0.92;

/// Presentation controls that do not alter the settled composition field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositionPresentationConfig {
    /// Counts at or above this value receive full density lightness.
    pub max_count: f32,
    /// Minimum lightness for a non-empty cell, keeping sparse cells visible.
    pub density_floor: f32,
    /// Overall alpha applied to non-empty cells.
    pub opacity: f32,
}

impl Default for CompositionPresentationConfig {
    fn default() -> Self {
        Self {
            max_count: 1.0,
            density_floor: DEFAULT_DENSITY_FLOOR,
            opacity: DEFAULT_OPACITY,
        }
    }
}

impl CompositionPresentationConfig {
    fn sanitized(self) -> Self {
        Self {
            max_count: if self.max_count.is_finite() && self.max_count > 0.0 {
                self.max_count
            } else {
                1.0
            },
            density_floor: if self.density_floor.is_finite() {
                self.density_floor.clamp(0.0, 1.0)
            } else {
                DEFAULT_DENSITY_FLOOR
            },
            opacity: if self.opacity.is_finite() {
                self.opacity.clamp(0.0, 1.0)
            } else {
                DEFAULT_OPACITY
            },
        }
    }
}

/// Presentation output for one exact composition bin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositionCellPresentation {
    pub total_count: u64,
    pub dominant_layer: Option<u8>,
    pub dominant_share: f32,
    pub normalized_entropy: f32,
    pub purity: f32,
    pub rgba: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionPresentationError {
    EmptyPalette,
    LayerCountExceedsPalette {
        layers: usize,
        palette_entries: usize,
    },
    CountOverflow,
}

impl std::fmt::Display for CompositionPresentationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPalette => formatter.write_str("composition palette is empty"),
            Self::LayerCountExceedsPalette {
                layers,
                palette_entries,
            } => write!(
                formatter,
                "composition has {layers} layers but palette only contains {palette_entries} entries"
            ),
            Self::CountOverflow => formatter.write_str("composition count exceeds u64"),
        }
    }
}

impl std::error::Error for CompositionPresentationError {}

/// Computes the CPU presentation vector used by the composition renderer.
pub fn present_composition_cell(
    layer_counts: &[u32],
    palette: &CategoryPaletteEntries,
    config: CompositionPresentationConfig,
) -> Result<CompositionCellPresentation, CompositionPresentationError> {
    let palette_entries = palette.linear_rgba.as_ref();
    if palette_entries.is_empty() {
        return Err(CompositionPresentationError::EmptyPalette);
    }
    if layer_counts.len() > palette_entries.len() {
        return Err(CompositionPresentationError::LayerCountExceedsPalette {
            layers: layer_counts.len(),
            palette_entries: palette_entries.len(),
        });
    }

    let total_count = layer_counts.iter().try_fold(0_u64, |total, count| {
        total
            .checked_add(u64::from(*count))
            .ok_or(CompositionPresentationError::CountOverflow)
    })?;
    if total_count == 0 || layer_counts.is_empty() {
        return Ok(CompositionCellPresentation {
            total_count,
            dominant_layer: None,
            dominant_share: 0.0,
            normalized_entropy: 0.0,
            purity: 0.0,
            rgba: [
                palette.mixed_neutral_linear_rgba[0],
                palette.mixed_neutral_linear_rgba[1],
                palette.mixed_neutral_linear_rgba[2],
                0.0,
            ],
        });
    }

    let dominant_index = layer_counts
        .iter()
        .enumerate()
        .max_by_key(|(index, count)| (**count, std::cmp::Reverse(*index)))
        .map(|(index, _)| index)
        .expect("non-empty layer counts have a dominant layer");
    let total = total_count as f32;
    let dominant_share = layer_counts[dominant_index] as f32 / total;
    let normalized_entropy = normalized_entropy(layer_counts, total);
    let purity = (1.0 - normalized_entropy).clamp(0.0, 1.0);
    let config = config.sanitized();
    let density_ratio = (total / config.max_count).clamp(0.0, 1.0);
    let lightness = config.density_floor + (1.0 - config.density_floor) * density_ratio.sqrt();
    let dominant = palette_entries[dominant_index];
    let neutral = palette.mixed_neutral_linear_rgba;
    let rgb = [
        (neutral[0] + (dominant[0] - neutral[0]) * purity) * lightness,
        (neutral[1] + (dominant[1] - neutral[1]) * purity) * lightness,
        (neutral[2] + (dominant[2] - neutral[2]) * purity) * lightness,
    ];
    Ok(CompositionCellPresentation {
        total_count,
        dominant_layer: Some(u8::try_from(dominant_index).unwrap_or(u8::MAX)),
        dominant_share,
        normalized_entropy,
        purity,
        rgba: [rgb[0], rgb[1], rgb[2], config.opacity * density_ratio],
    })
}

fn normalized_entropy(layer_counts: &[u32], total: f32) -> f32 {
    if layer_counts.len() <= 1 {
        return 0.0;
    }
    let entropy = layer_counts.iter().fold(0.0, |entropy, count| {
        let share = *count as f32 / total;
        if share > 0.0 {
            entropy - share * share.ln()
        } else {
            entropy
        }
    });
    (entropy / (layer_counts.len() as f32).ln()).clamp(0.0, 1.0)
}

/// Uniform values shared by the CPU-facing render coordinator and WGSL.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct CompositionRenderParams {
    pub grid_width: u32,
    pub grid_height: u32,
    pub layer_count: u32,
    pub _padding: u32,
    pub max_count: f32,
    pub density_floor: f32,
    pub opacity: f32,
    pub _padding_params: u32,
    pub neutral_rgba: [f32; 4],
    pub _padding_tail: [u32; 4],
}

impl CompositionRenderParams {
    pub fn new(
        grid_width: u32,
        grid_height: u32,
        layer_count: u32,
        config: CompositionPresentationConfig,
        neutral_rgba: [f32; 4],
    ) -> Self {
        let config = config.sanitized();
        Self {
            grid_width,
            grid_height,
            layer_count,
            _padding: 0,
            max_count: config.max_count,
            density_floor: config.density_floor,
            opacity: config.opacity,
            _padding_params: 0,
            neutral_rgba,
            _padding_tail: [0; 4],
        }
    }
}

/// Small resident storage buffer containing the discrete categorical colours.
pub struct CompositionPaletteGpuResources {
    palette_buffer: wgpu::Buffer,
    layer_count: u32,
}

impl CompositionPaletteGpuResources {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        palette: &CategoryPaletteEntries,
    ) -> Result<Self, CompositionPresentationError> {
        let entries = palette.linear_rgba.as_ref();
        if entries.is_empty() {
            return Err(CompositionPresentationError::EmptyPalette);
        }
        let bytes = bytemuck::cast_slice(entries);
        let palette_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Category Composition Palette"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&palette_buffer, 0, bytes);
        Ok(Self {
            palette_buffer,
            layer_count: entries.len() as u32,
        })
    }

    pub const fn layer_count(&self) -> u32 {
        self.layer_count
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.palette_buffer
    }
}

pub struct CompositionRenderPipeline {
    pub layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
}

pub fn composition_render_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Category Composition Render Layout"),
        entries: &[
            storage_entry(0),
            storage_entry(1),
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        std::mem::size_of::<CompositionRenderParams>() as u64,
                    ),
                },
                count: None,
            },
        ],
    })
}

fn storage_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub fn composition_render_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    layer_counts: &wgpu::Buffer,
    palette: &CompositionPaletteGpuResources,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Category Composition Render Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: layer_counts.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: palette.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

pub fn composition_render_pipeline(
    device: &wgpu::Device,
    layout: wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> CompositionRenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("RawScope Category Composition Render Shader"),
        source: wgpu::ShaderSource::Wgsl(COMPOSITION_RENDER_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("RawScope Category Composition Render Pipeline Layout"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RawScope Category Composition Render Pipeline"),
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
                format,
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
    CompositionRenderPipeline { layout, pipeline }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> CategoryPaletteEntries {
        CategoryPaletteEntries::category_composition()
    }

    #[test]
    fn composition_presentation_uses_density_lightness_and_purity_neutral_mix() {
        let palette = palette();
        let sparse = present_composition_cell(
            &[1, 0],
            &palette,
            CompositionPresentationConfig {
                max_count: 4.0,
                ..Default::default()
            },
        )
        .unwrap();
        let dense = present_composition_cell(
            &[4, 0],
            &palette,
            CompositionPresentationConfig {
                max_count: 4.0,
                ..Default::default()
            },
        )
        .unwrap();
        let mixed = present_composition_cell(
            &[2, 2],
            &palette,
            CompositionPresentationConfig {
                max_count: 4.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(sparse.dominant_layer, Some(0));
        assert!(dense.rgba[1] > sparse.rgba[1]);
        assert!(mixed.purity < dense.purity);
        assert!(mixed.rgba[0] > 0.0);
        assert!((mixed.rgba[0] - mixed.rgba[1]).abs() < (dense.rgba[0] - dense.rgba[1]).abs());
    }

    #[test]
    fn dominant_hue_always_maps_to_a_real_layer() {
        let palette = palette();
        for counts in [[0, 0, 2, 0], [1, 1, 0, 0], [0, 0, 0, 4]] {
            let output = present_composition_cell(&counts, &palette, Default::default()).unwrap();
            let dominant = output
                .dominant_layer
                .expect("non-empty cell has a dominant layer");
            assert!(usize::from(dominant) < counts.len());
            assert_eq!(output.rgba[3], output.rgba[3].clamp(0.0, 1.0));
        }
    }

    #[test]
    fn composition_purity_cpu_gpu_vectors_match() {
        let palette = palette();
        let config = CompositionPresentationConfig {
            max_count: 8.0,
            density_floor: 0.2,
            opacity: 0.8,
        };
        for counts in [
            &[0, 0, 0][..],
            &[4, 2, 2][..],
            &[0, 8, 0][..],
            &[1, 1, 1, 1][..],
        ] {
            let cpu = present_composition_cell(&counts, &palette, config).unwrap();
            let gpu = gpu_reference_vector(&counts, &palette, config);
            for (cpu_channel, gpu_channel) in cpu.rgba.into_iter().zip(gpu) {
                assert!(
                    (cpu_channel - gpu_channel).abs() < 1.0e-5,
                    "cpu={cpu_channel} gpu={gpu_channel} counts={counts:?}"
                );
            }
            assert!((cpu.purity - gpu_reference_purity(&counts)).abs() < 1.0e-5);
        }
    }

    #[test]
    fn composition_render_params_have_wgsl_uniform_alignment() {
        assert_eq!(std::mem::size_of::<CompositionRenderParams>(), 64);
        assert_eq!(std::mem::align_of::<CompositionRenderParams>(), 4);
    }

    fn gpu_reference_vector(
        layer_counts: &[u32],
        palette: &CategoryPaletteEntries,
        config: CompositionPresentationConfig,
    ) -> [f32; 4] {
        let total = layer_counts.iter().map(|count| *count as f32).sum::<f32>();
        if total == 0.0 {
            return [
                palette.mixed_neutral_linear_rgba[0],
                palette.mixed_neutral_linear_rgba[1],
                palette.mixed_neutral_linear_rgba[2],
                0.0,
            ];
        }
        let dominant = layer_counts
            .iter()
            .enumerate()
            .max_by_key(|(index, count)| (**count, std::cmp::Reverse(*index)))
            .map(|(index, _)| index)
            .unwrap();
        let purity = gpu_reference_purity(layer_counts);
        let config = config.sanitized();
        let density = (total / config.max_count).clamp(0.0, 1.0);
        let lightness = config.density_floor + (1.0 - config.density_floor) * density.sqrt();
        let color = palette.linear_rgba[dominant];
        let neutral = palette.mixed_neutral_linear_rgba;
        [
            (neutral[0] + (color[0] - neutral[0]) * purity) * lightness,
            (neutral[1] + (color[1] - neutral[1]) * purity) * lightness,
            (neutral[2] + (color[2] - neutral[2]) * purity) * lightness,
            config.opacity * density,
        ]
    }

    fn gpu_reference_purity(layer_counts: &[u32]) -> f32 {
        let total = layer_counts.iter().map(|count| *count as f32).sum::<f32>();
        if total == 0.0 {
            return 0.0;
        }
        if layer_counts.len() <= 1 {
            return 1.0;
        }
        let entropy = layer_counts.iter().fold(0.0, |value, count| {
            let share = *count as f32 / total;
            if share > 0.0 {
                value - share * share.ln()
            } else {
                value
            }
        });
        (1.0 - (entropy / (layer_counts.len() as f32).ln()).clamp(0.0, 1.0)).clamp(0.0, 1.0)
    }
}
