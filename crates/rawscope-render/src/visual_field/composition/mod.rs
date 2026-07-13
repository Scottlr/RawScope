//! Resident category-composition aggregation over shared visual-field inputs.
//!
//! The CPU reference in this module owns the coordinate/bin contract used by
//! the compute shader. GPU resources keep the row-code channel stable while a
//! bounded layer-plan lookup and double-buffered layer-major fields can be
//! replaced independently.

mod compute;
mod inspection;
mod presentation;
mod resources;

use std::num::NonZeroU32;

use rawscope_analysis::density::{bin_f32, BinIndex, BinPlacement};
use rawscope_core::{F32Range, GridSize};

pub use compute::{
    clear_pending_layer_counts, composition_bind_group, composition_bind_group_layout,
    composition_compute_pipeline, CompositionParams,
};
pub use inspection::{
    CompositionInspectionCache, CompositionInspectionDetail, CompositionInspectionError,
    CompositionInspectionLayer, CompositionInspectionLayerMetadata,
};
pub use presentation::{
    composition_render_bind_group, composition_render_bind_group_layout,
    composition_render_pipeline, present_composition_cell, CompositionCellPresentation,
    CompositionPaletteGpuResources, CompositionPresentationConfig, CompositionPresentationError,
    CompositionRenderParams, CompositionRenderPipeline,
};
pub use resources::{
    CategoryChannelGpuResources, CategoryChannelIdentity, CategoryCompositionFieldGeneration,
    CategoryLayerPlanGpuResources, CompositionPublicationError, CompositionPublicationIdentity,
    CompositionResourceError, SpecialLayerParams, NO_CATEGORY_LAYER,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryCompositionReference {
    grid: GridSize,
    layer_count: u8,
    layer_major_counts: Vec<u32>,
}

impl CategoryCompositionReference {
    pub fn grid(&self) -> GridSize {
        self.grid
    }

    pub const fn layer_count(&self) -> u8 {
        self.layer_count
    }

    pub fn counts(&self) -> &[u32] {
        &self.layer_major_counts
    }

    pub fn count(&self, layer_id: u8, x_bin: u32, y_bin: u32) -> Option<u32> {
        if layer_id >= self.layer_count || x_bin >= self.grid.width() || y_bin >= self.grid.height()
        {
            return None;
        }
        let bin_index = (y_bin as usize) * (self.grid.width() as usize) + x_bin as usize;
        let index = (layer_id as usize) * self.grid.bin_count() + bin_index;
        self.layer_major_counts.get(index).copied()
    }

    pub fn total_counts(&self) -> Vec<u32> {
        let mut totals = vec![0_u32; self.grid.bin_count()];
        for layer_counts in self.layer_major_counts.chunks_exact(self.grid.bin_count()) {
            for (total, layer_count) in totals.iter_mut().zip(layer_counts) {
                *total = total.saturating_add(*layer_count);
            }
        }
        totals
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionReferenceError {
    PointCodeLengthMismatch { points: usize, codes: usize },
    FilterMaskLengthMismatch { points: usize, mask: usize },
    LayerCountZero,
    LayerLookupLengthMismatch { values: usize, tracked: u32 },
    LayerLookupOutOfRange { layer_id: u32, layer_count: u8 },
    UnassignedCategoryLayer { code: u32 },
    CountOverflow,
    InvalidGrid(rawscope_core::GridSizeError),
}

/// Bins row-aligned category codes into bounded layer-major fields.
///
/// Coordinates, mask inclusion, and edge handling intentionally mirror the
/// exact density field. `value_to_layer` is bounded by the T007 index and
/// `special_layers` carries the separate Other/Missing/Invalid assignments.
pub fn composition_reference<T: crate::VisualFieldPoint>(
    points: &[T],
    category_codes: &[u32],
    code_layout: rawscope_data::CategoryCodeLayout,
    value_to_layer: &[u32],
    special_layers: SpecialLayerParams,
    filter_mask: Option<&[u32]>,
    x_range: F32Range,
    y_range: F32Range,
    width: u32,
    height: u32,
) -> Result<CategoryCompositionReference, CompositionReferenceError> {
    if points.len() != category_codes.len() {
        return Err(CompositionReferenceError::PointCodeLengthMismatch {
            points: points.len(),
            codes: category_codes.len(),
        });
    }
    if let Some(mask) = filter_mask {
        if mask.len() != points.len() {
            return Err(CompositionReferenceError::FilterMaskLengthMismatch {
                points: points.len(),
                mask: mask.len(),
            });
        }
    }
    let layer_count = u8::try_from(special_layers.layer_count).map_err(|_| {
        CompositionReferenceError::LayerLookupOutOfRange {
            layer_id: special_layers.layer_count,
            layer_count: u8::MAX,
        }
    })?;
    if layer_count == 0 {
        return Err(CompositionReferenceError::LayerCountZero);
    }
    if value_to_layer.len() != code_layout.tracked_value_count as usize {
        return Err(CompositionReferenceError::LayerLookupLengthMismatch {
            values: value_to_layer.len(),
            tracked: code_layout.tracked_value_count,
        });
    }
    validate_layer_lookup(value_to_layer, special_layers, layer_count)?;
    let grid = GridSize::try_new(width, height).map_err(CompositionReferenceError::InvalidGrid)?;
    let mut layer_major_counts = vec![0_u32; grid.bin_count() * usize::from(layer_count)];
    let x_bins = NonZeroU32::new(width).expect("validated composition width");
    let y_bins = NonZeroU32::new(height).expect("validated composition height");
    for (index, point) in points.iter().enumerate() {
        if filter_mask.is_some_and(|mask| mask[index] == 0) {
            continue;
        }
        let Some(x_bin) = in_domain_bin(bin_f32(point.x(), x_range, x_bins)) else {
            continue;
        };
        let Some(y_bin) = in_domain_bin(bin_f32(point.y(), y_range, y_bins)) else {
            continue;
        };
        let layer_id = resolve_layer(
            category_codes[index],
            code_layout,
            value_to_layer,
            special_layers,
        )?
        .ok_or(CompositionReferenceError::UnassignedCategoryLayer {
            code: category_codes[index],
        })?;
        let bin_index = (y_bin as usize) * width as usize + x_bin as usize;
        let index = usize::from(layer_id) * grid.bin_count() + bin_index;
        layer_major_counts[index] = layer_major_counts[index]
            .checked_add(1)
            .ok_or(CompositionReferenceError::CountOverflow)?;
    }
    Ok(CategoryCompositionReference {
        grid,
        layer_count,
        layer_major_counts,
    })
}

/// Alias emphasizing that the returned vector is layer-major.
pub fn composition_reference_layers<T: crate::VisualFieldPoint>(
    points: &[T],
    category_codes: &[u32],
    code_layout: rawscope_data::CategoryCodeLayout,
    value_to_layer: &[u32],
    special_layers: SpecialLayerParams,
    filter_mask: Option<&[u32]>,
    x_range: F32Range,
    y_range: F32Range,
    width: u32,
    height: u32,
) -> Result<CategoryCompositionReference, CompositionReferenceError> {
    composition_reference(
        points,
        category_codes,
        code_layout,
        value_to_layer,
        special_layers,
        filter_mask,
        x_range,
        y_range,
        width,
        height,
    )
}

pub fn publication_generation_matches(
    expected: CompositionPublicationIdentity,
    actual: CompositionPublicationIdentity,
) -> bool {
    expected == actual
}

/// Verifies the publication invariant for one settled layer readback.
pub fn layer_counts_match_total(
    layer_counts: &[u32],
    exact_counts: &[u32],
    layer_count: u8,
) -> bool {
    let Some(expected_len) = exact_counts.len().checked_mul(usize::from(layer_count)) else {
        return false;
    };
    if layer_count == 0 || exact_counts.is_empty() || layer_counts.len() != expected_len {
        return false;
    }
    let mut totals = vec![0_u32; exact_counts.len()];
    for layer in layer_counts.chunks_exact(exact_counts.len()) {
        for (total, count) in totals.iter_mut().zip(layer) {
            let Some(next) = total.checked_add(*count) else {
                return false;
            };
            *total = next;
        }
    }
    totals == exact_counts
}

fn resolve_layer(
    code: u32,
    code_layout: rawscope_data::CategoryCodeLayout,
    value_to_layer: &[u32],
    special_layers: SpecialLayerParams,
) -> Result<Option<u8>, CompositionReferenceError> {
    let layer = if code < code_layout.tracked_value_count {
        value_to_layer.get(code as usize).copied()
    } else if code == code_layout.untracked_code {
        Some(special_layers.untracked_layer)
    } else if code == code_layout.missing_code {
        Some(special_layers.missing_layer)
    } else {
        Some(special_layers.invalid_layer)
    };
    let Some(layer) = layer else {
        return Ok(None);
    };
    if layer == NO_CATEGORY_LAYER {
        return Ok(None);
    }
    let layer =
        u8::try_from(layer).map_err(|_| CompositionReferenceError::LayerLookupOutOfRange {
            layer_id: layer,
            layer_count: special_layers.layer_count as u8,
        })?;
    if u32::from(layer) >= special_layers.layer_count {
        return Err(CompositionReferenceError::LayerLookupOutOfRange {
            layer_id: u32::from(layer),
            layer_count: special_layers.layer_count as u8,
        });
    }
    Ok(Some(layer))
}

fn validate_layer_lookup(
    value_to_layer: &[u32],
    special_layers: SpecialLayerParams,
    layer_count: u8,
) -> Result<(), CompositionReferenceError> {
    let special_layer_ids = [
        special_layers.untracked_layer,
        special_layers.missing_layer,
        special_layers.invalid_layer,
    ];
    for &layer_id in value_to_layer.iter().chain(special_layer_ids.iter()) {
        if layer_id != NO_CATEGORY_LAYER && layer_id >= u32::from(layer_count) {
            return Err(CompositionReferenceError::LayerLookupOutOfRange {
                layer_id,
                layer_count,
            });
        }
    }
    Ok(())
}

fn in_domain_bin(
    result: Result<BinPlacement, rawscope_analysis::density::BinningError>,
) -> Option<u32> {
    match result.ok()? {
        BinPlacement::InDomain(BinIndex(index)) => Some(index),
        BinPlacement::BeforeDomain | BinPlacement::AfterDomain => None,
    }
}

#[cfg(test)]
mod tests {
    use rawscope_core::F32Range;

    use super::*;

    #[derive(Clone, Copy)]
    struct Point {
        x: f32,
        y: f32,
    }

    impl crate::VisualFieldPoint for Point {
        fn x(&self) -> f32 {
            self.x
        }

        fn y(&self) -> f32 {
            self.y
        }
    }

    fn layout() -> rawscope_data::CategoryCodeLayout {
        rawscope_data::CategoryCodeLayout {
            tracked_value_count: 2,
            untracked_code: 2,
            missing_code: 3,
            invalid_code: 4,
        }
    }

    fn special() -> SpecialLayerParams {
        SpecialLayerParams {
            untracked_layer: 1,
            missing_layer: 2,
            invalid_layer: NO_CATEGORY_LAYER,
            layer_count: 3,
        }
    }

    #[test]
    fn composition_reference_layers_sum_to_total() {
        let points = [
            Point { x: 0.1, y: 0.1 },
            Point { x: 0.9, y: 0.1 },
            Point { x: 0.1, y: 0.9 },
            Point { x: 0.9, y: 0.9 },
        ];
        let reference = composition_reference(
            &points,
            &[0, 1, 2, 3],
            layout(),
            &[0, 0],
            special(),
            Some(&[1, 1, 1, 1]),
            F32Range::new(0.0, 1.0),
            F32Range::new(0.0, 1.0),
            2,
            2,
        )
        .unwrap();
        assert_eq!(reference.total_counts(), vec![1, 1, 1, 1]);
        assert_eq!(reference.count(0, 0, 0), Some(1));
        assert_eq!(reference.count(0, 1, 0), Some(1));
        assert_eq!(reference.count(1, 0, 1), Some(1));
        assert_eq!(reference.count(2, 1, 1), Some(1));
    }

    #[test]
    fn composition_rejects_out_of_range_layer_lookup() {
        let points = [Point { x: 0.5, y: 0.5 }];
        assert_eq!(
            composition_reference(
                &points,
                &[0],
                layout(),
                &[7, 0],
                special(),
                None,
                F32Range::new(0.0, 1.0),
                F32Range::new(0.0, 1.0),
                1,
                1,
            ),
            Err(CompositionReferenceError::LayerLookupOutOfRange {
                layer_id: 7,
                layer_count: 3,
            })
        );
    }

    #[test]
    fn category_channel_reuses_upload_for_layer_plan_and_viewport_changes() {
        let identity = CategoryChannelIdentity::new(
            rawscope_data::DatasetGenerationCounter::default().mint(),
            rawscope_gpu::DeviceGeneration(4),
            rawscope_core::ColumnId::new(2),
        );
        assert!(identity == identity);
        assert!(
            identity
                == CategoryChannelIdentity::new(
                    identity.dataset_generation(),
                    identity.device_generation(),
                    identity.column_id(),
                )
        );
    }

    #[test]
    fn composition_publication_rejects_generation_mismatch() {
        let dataset = rawscope_data::DatasetGenerationCounter::default().mint();
        let mut views = rawscope_render_view_counter();
        let identity = CompositionPublicationIdentity {
            dataset_generation: dataset,
            device_generation: rawscope_gpu::DeviceGeneration(1),
            cohort_generation: rawscope_analysis::cohort::CohortGenerationCounter::default().mint(),
            view_generation: views.mint(),
            plan_generation:
                rawscope_analysis::visual_field::CategoryLayerPlanGenerationCounter::default()
                    .mint(),
            grid: GridSize::new(2, 2),
        };
        let mut stale = identity;
        stale.device_generation = rawscope_gpu::DeviceGeneration(2);
        assert!(!publication_generation_matches(identity, stale));
    }

    #[test]
    fn composition_publication_rejects_layer_total_mismatch() {
        assert!(layer_counts_match_total(&[1, 0, 0, 1], &[1, 1], 2));
        assert!(!layer_counts_match_total(&[1, 0, 0, 0], &[1, 1], 2));
        assert!(!layer_counts_match_total(&[1, u32::MAX, 0, 0], &[1, 1], 2));
    }

    fn rawscope_render_view_counter() -> crate::VisualFieldViewGenerationCounter {
        crate::VisualFieldViewGenerationCounter::default()
    }
}
