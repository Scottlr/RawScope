//! Curated linear-light palette definitions and device-generation LUTs.

use std::sync::Arc;

use rawscope_analysis::visual_field::MAX_CATEGORY_COMPOSITION_LAYERS;
use rawscope_evidence::DensityPalette;
use rawscope_gpu::DeviceGeneration;

pub const PALETTE_LUT_SIZE: usize = 256;
pub const CONTINUOUS_PALETTE_ROW_COUNT: u32 = 3;
pub const PALETTE_LUT_BYTES: u64 =
    (PALETTE_LUT_SIZE as u64) * (CONTINUOUS_PALETTE_ROW_COUNT as u64) * 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContinuousPalette {
    DensitySequential,
    TimelineSequential,
    CohortDifference,
}

impl ContinuousPalette {
    pub const fn row(self) -> u32 {
        match self {
            Self::DensitySequential => 0,
            Self::TimelineSequential => 1,
            Self::CohortDifference => 2,
        }
    }

    pub const fn from_density_palette(palette: DensityPalette) -> Self {
        match palette {
            DensityPalette::ScatterSequential => Self::DensitySequential,
            DensityPalette::TimelineSequential => Self::TimelineSequential,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CategoricalPalette {
    CategoryComposition,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuousPaletteLut {
    palette: ContinuousPalette,
    linear_rgba: [[f32; 4]; PALETTE_LUT_SIZE],
}

impl ContinuousPaletteLut {
    pub fn new(palette: ContinuousPalette) -> Self {
        let control_points = palette.control_points_srgb();
        let mut linear_rgba = [[0.0; 4]; PALETTE_LUT_SIZE];
        for (index, entry) in linear_rgba.iter_mut().enumerate() {
            let position = index as f32 / (PALETTE_LUT_SIZE - 1) as f32;
            let scaled = position * (control_points.len() - 1) as f32;
            let segment = (scaled.floor() as usize).min(control_points.len() - 2);
            let local = scaled - segment as f32;
            let start = control_points[segment];
            let end = control_points[segment + 1];
            *entry = [
                lerp(srgb_to_linear(start[0]), srgb_to_linear(end[0]), local),
                lerp(srgb_to_linear(start[1]), srgb_to_linear(end[1]), local),
                lerp(srgb_to_linear(start[2]), srgb_to_linear(end[2]), local),
                1.0,
            ];
        }
        Self {
            palette,
            linear_rgba,
        }
    }

    pub const fn palette(self) -> ContinuousPalette {
        self.palette
    }

    pub fn linear_rgba(&self) -> &[[f32; 4]; PALETTE_LUT_SIZE] {
        &self.linear_rgba
    }

    pub fn sample_linear(&self, intensity: f32) -> [f32; 4] {
        let position = intensity.clamp(0.0, 1.0) * (PALETTE_LUT_SIZE - 1) as f32;
        let lower = position.floor() as usize;
        let upper = (lower + 1).min(PALETTE_LUT_SIZE - 1);
        let fraction = position - lower as f32;
        let start = self.linear_rgba[lower];
        let end = self.linear_rgba[upper];
        [
            lerp(start[0], end[0], fraction),
            lerp(start[1], end[1], fraction),
            lerp(start[2], end[2], fraction),
            lerp(start[3], end[3], fraction),
        ]
    }

    /// Samples the exact LUT at four legend positions and converts at the UI boundary.
    pub fn legend_rgb(&self) -> [[u8; 3]; 4] {
        [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0].map(|intensity| {
            let [red, green, blue, _] = self.sample_linear(intensity);
            [
                linear_to_srgb_u8(red),
                linear_to_srgb_u8(green),
                linear_to_srgb_u8(blue),
            ]
        })
    }

    pub fn sample_srgb8(&self, intensity: f32) -> [u8; 3] {
        let [red, green, blue, _] = self.sample_linear(intensity);
        [
            linear_to_srgb_u8(red),
            linear_to_srgb_u8(green),
            linear_to_srgb_u8(blue),
        ]
    }
}

impl ContinuousPalette {
    const fn control_points_srgb(self) -> &'static [[u8; 3]] {
        match self {
            Self::DensitySequential => &[[4, 6, 9], [5, 48, 71], [20, 140, 148], [255, 189, 77]],
            Self::TimelineSequential => &[[3, 4, 8], [13, 41, 77], [41, 117, 199], [255, 148, 51]],
            Self::CohortDifference => &[[24, 144, 153], [36, 42, 48], [232, 112, 96]],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CategoryPaletteEntries {
    pub palette: CategoricalPalette,
    pub linear_rgba: Arc<[[f32; 4]]>,
    pub mixed_neutral_linear_rgba: [f32; 4],
}

impl CategoryPaletteEntries {
    pub fn category_composition() -> Self {
        let srgb_entries = [
            [46, 204, 113],
            [52, 152, 219],
            [155, 89, 182],
            [241, 196, 15],
            [230, 126, 34],
            [231, 76, 60],
            [26, 188, 156],
            [149, 165, 166],
        ];
        let linear_rgba = srgb_entries.map(|[red, green, blue]| {
            [
                srgb_to_linear(red),
                srgb_to_linear(green),
                srgb_to_linear(blue),
                1.0,
            ]
        });
        debug_assert_eq!(linear_rgba.len(), MAX_CATEGORY_COMPOSITION_LAYERS as usize);
        Self {
            palette: CategoricalPalette::CategoryComposition,
            linear_rgba: Arc::from(linear_rgba.to_vec()),
            mixed_neutral_linear_rgba: [0.18, 0.18, 0.18, 1.0],
        }
    }
}

pub struct PaletteGpuResources {
    device_generation: DeviceGeneration,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    allocated_bytes: u64,
}

impl PaletteGpuResources {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        device_generation: DeviceGeneration,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RawScope Visual Field Palette LUT"),
            size: wgpu::Extent3d {
                width: PALETTE_LUT_SIZE as u32,
                height: CONTINUOUS_PALETTE_ROW_COUNT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut bytes = Vec::with_capacity(PALETTE_LUT_BYTES as usize);
        for palette in [
            ContinuousPalette::DensitySequential,
            ContinuousPalette::TimelineSequential,
            ContinuousPalette::CohortDifference,
        ] {
            for rgba in ContinuousPaletteLut::new(palette).linear_rgba() {
                for channel in rgba {
                    bytes.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
                }
            }
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some((PALETTE_LUT_SIZE * 4) as u32),
                rows_per_image: Some(CONTINUOUS_PALETTE_ROW_COUNT),
            },
            wgpu::Extent3d {
                width: PALETTE_LUT_SIZE as u32,
                height: CONTINUOUS_PALETTE_ROW_COUNT,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RawScope Visual Field Palette LUT Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        Self {
            device_generation,
            texture,
            view,
            sampler,
            allocated_bytes: PALETTE_LUT_BYTES,
        }
    }

    pub const fn device_generation(&self) -> DeviceGeneration {
        self.device_generation
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    pub const fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}

fn lerp(start: f32, end: f32, fraction: f32) -> f32 {
    start + (end - start) * fraction
}

fn srgb_to_linear(value: u8) -> f32 {
    let value = value as f32 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb_u8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let srgb = if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_lut_endpoints_match_reviewed_control_points() {
        let lut = ContinuousPaletteLut::new(ContinuousPalette::DensitySequential);
        assert_eq!(
            lut.legend_rgb(),
            [[4, 6, 9], [5, 48, 71], [20, 140, 148], [255, 189, 77]]
        );
    }

    #[test]
    fn palette_lut_interpolates_in_linear_light() {
        let lut = ContinuousPaletteLut::new(ContinuousPalette::DensitySequential);
        let midpoint = lut.sample_linear(0.5);
        let gamma_midpoint = srgb_to_linear(5) + (srgb_to_linear(20) - srgb_to_linear(5)) * 0.5;
        assert!((midpoint[0] - gamma_midpoint).abs() < 0.02);
        assert!(midpoint[1] > srgb_to_linear(48));
    }

    #[test]
    fn categorical_entries_are_discrete_and_cover_layer_capacity() {
        let entries = CategoryPaletteEntries::category_composition();
        assert_eq!(
            entries.linear_rgba.len(),
            MAX_CATEGORY_COMPOSITION_LAYERS as usize
        );
        assert_ne!(entries.linear_rgba[0], entries.linear_rgba[1]);
        assert_eq!(entries.mixed_neutral_linear_rgba[3], 1.0);
    }

    #[test]
    fn legend_samples_the_shader_lut() {
        let lut = ContinuousPaletteLut::new(ContinuousPalette::from_density_palette(
            DensityPalette::TimelineSequential,
        ));
        assert_eq!(
            lut.legend_rgb(),
            [[3, 4, 8], [13, 41, 77], [41, 117, 199], [255, 148, 51]]
        );
    }
}
