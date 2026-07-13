//! Bounded exact composition details for settled hover and pin interactions.

use std::{error::Error, fmt, sync::Arc};

use rawscope_analysis::visual_field::{
    summarize_composition_cell, CategoryLayerId, CategoryLayerPlan, CompositionError,
};
use rawscope_core::GridSize;
use rawscope_data::CategoryIndexAccuracy;

use super::CategoryCompositionReference;

#[derive(Debug, Clone, PartialEq)]
pub struct CompositionInspectionLayer {
    pub layer_id: CategoryLayerId,
    pub label: Arc<str>,
    pub count: u32,
    pub share: f64,
    pub accuracy: CategoryIndexAccuracy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionInspectionLayerMetadata {
    pub layer_id: CategoryLayerId,
    pub label: Arc<str>,
    pub accuracy: CategoryIndexAccuracy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositionInspectionDetail {
    pub total_count: u64,
    pub dominant_layer: Option<CategoryLayerId>,
    pub dominant_share: f64,
    pub purity: f64,
    pub accuracy: CategoryIndexAccuracy,
    pub layers: Arc<[CompositionInspectionLayer]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionInspectionError {
    EmptyLayerPlan,
    LayerCountMismatch { counts: usize, plan: usize },
    CellCountMismatch { counts: usize, cells: usize },
    InvalidComposition(CompositionError),
}

impl fmt::Display for CompositionInspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLayerPlan => {
                formatter.write_str("composition inspection requires a layer plan")
            }
            Self::LayerCountMismatch { counts, plan } => write!(
                formatter,
                "composition inspection has {counts} layers but the plan has {plan}"
            ),
            Self::CellCountMismatch { counts, cells } => write!(
                formatter,
                "composition inspection has {counts} count entries but the grid has {cells} cells"
            ),
            Self::InvalidComposition(error) => error.fmt(formatter),
        }
    }
}

impl Error for CompositionInspectionError {}

impl From<CompositionError> for CompositionInspectionError {
    fn from(error: CompositionError) -> Self {
        Self::InvalidComposition(error)
    }
}

/// Settled, row-independent composition details indexed directly by grid cell.
///
/// The cache owns the bounded layer-major readback and plan metadata. Hover and
/// pin consumers only perform a grid-index lookup; they do not scan source rows
/// or the sampled row IDs carried by the scatter inspection grid.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionInspectionCache {
    grid: GridSize,
    details: Arc<[CompositionInspectionDetail]>,
}

impl CompositionInspectionCache {
    pub fn from_reference(
        reference: &CategoryCompositionReference,
        plan: Arc<CategoryLayerPlan>,
    ) -> Result<Self, CompositionInspectionError> {
        Self::from_layer_counts(reference.grid(), reference.counts(), plan)
    }

    pub fn from_layer_counts(
        grid: GridSize,
        layer_counts: &[u32],
        plan: Arc<CategoryLayerPlan>,
    ) -> Result<Self, CompositionInspectionError> {
        let metadata = plan
            .layers()
            .iter()
            .map(|layer| CompositionInspectionLayerMetadata {
                layer_id: layer.id,
                label: Arc::clone(&layer.label),
                accuracy: layer.accuracy,
            })
            .collect::<Vec<_>>();
        Self::from_layer_counts_with_metadata(grid, layer_counts, &metadata)
    }

    pub fn from_layer_counts_with_metadata(
        grid: GridSize,
        layer_counts: &[u32],
        metadata: &[CompositionInspectionLayerMetadata],
    ) -> Result<Self, CompositionInspectionError> {
        let layer_count = metadata.len();
        if layer_count == 0 {
            return Err(CompositionInspectionError::EmptyLayerPlan);
        }
        let expected_count_entries = grid.bin_count().checked_mul(layer_count).ok_or(
            CompositionInspectionError::CellCountMismatch {
                counts: layer_counts.len(),
                cells: grid.bin_count(),
            },
        )?;
        if layer_counts.len() != expected_count_entries {
            return Err(CompositionInspectionError::CellCountMismatch {
                counts: layer_counts.len(),
                cells: expected_count_entries,
            });
        }
        let mut details = Vec::with_capacity(grid.bin_count());
        for cell_index in 0..grid.bin_count() {
            let counts = layer_counts
                .chunks_exact(grid.bin_count())
                .map(|layer| layer[cell_index])
                .collect::<Vec<_>>();
            details.push(detail_from_counts(&counts, metadata)?);
        }
        Ok(Self {
            grid,
            details: details.into(),
        })
    }

    pub const fn grid(&self) -> GridSize {
        self.grid
    }

    pub fn inspect_bin(&self, bin_x: u32, bin_y: u32) -> Option<&CompositionInspectionDetail> {
        if bin_x >= self.grid.width() || bin_y >= self.grid.height() {
            return None;
        }
        let index = bin_y as usize * self.grid.width() as usize + bin_x as usize;
        self.details.get(index)
    }

    pub fn details(&self) -> &[CompositionInspectionDetail] {
        &self.details
    }
}

fn detail_from_counts(
    counts: &[u32],
    metadata: &[CompositionInspectionLayerMetadata],
) -> Result<CompositionInspectionDetail, CompositionInspectionError> {
    if counts.len() != metadata.len() {
        return Err(CompositionInspectionError::LayerCountMismatch {
            counts: counts.len(),
            plan: metadata.len(),
        });
    }
    let summary = summarize_composition_cell(counts)?;
    let layers = metadata
        .iter()
        .zip(summary.layers.iter())
        .map(|(metadata, share)| CompositionInspectionLayer {
            layer_id: metadata.layer_id,
            label: Arc::clone(&metadata.label),
            count: share.count,
            share: share.share,
            accuracy: metadata.accuracy,
        })
        .collect::<Vec<_>>()
        .into();
    let accuracy =
        metadata
            .iter()
            .fold(CategoryIndexAccuracy::Exact, |current, layer| {
                match (current, layer.accuracy) {
                    (CategoryIndexAccuracy::Truncated, _)
                    | (_, CategoryIndexAccuracy::Truncated) => CategoryIndexAccuracy::Truncated,
                    (CategoryIndexAccuracy::Estimated, _)
                    | (_, CategoryIndexAccuracy::Estimated) => CategoryIndexAccuracy::Estimated,
                    _ => CategoryIndexAccuracy::Exact,
                }
            });
    Ok(CompositionInspectionDetail {
        total_count: summary.total_count,
        dominant_layer: summary.dominant_layer,
        dominant_share: summary.dominant_share,
        purity: summary.purity,
        accuracy,
        layers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_core::GridSize;
    use rawscope_data::CategoryIndexAccuracy;

    fn metadata() -> [CompositionInspectionLayerMetadata; 2] {
        [
            CompositionInspectionLayerMetadata {
                layer_id: CategoryLayerId::new(0),
                label: Arc::from("Alpha"),
                accuracy: CategoryIndexAccuracy::Exact,
            },
            CompositionInspectionLayerMetadata {
                layer_id: CategoryLayerId::new(1),
                label: Arc::from("Other"),
                accuracy: CategoryIndexAccuracy::Estimated,
            },
        ]
    }

    #[test]
    fn inspection_layers_sum_to_exact_count() {
        let grid = GridSize::try_new(1, 1).unwrap();
        let cache =
            CompositionInspectionCache::from_layer_counts_with_metadata(grid, &[2, 1], &metadata())
                .unwrap();
        let detail = cache.inspect_bin(0, 0).unwrap();
        assert_eq!(detail.total_count, 3);
        assert_eq!(detail.accuracy, CategoryIndexAccuracy::Estimated);
        assert_eq!(
            detail.layers.iter().map(|layer| layer.count).sum::<u32>(),
            3
        );
        assert!((detail.layers.iter().map(|layer| layer.share).sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn hover_uses_settled_cache_without_row_access() {
        let grid = GridSize::try_new(2, 1).unwrap();
        let cache = CompositionInspectionCache::from_layer_counts_with_metadata(
            grid,
            &[2, 0, 0, 3],
            &metadata(),
        )
        .unwrap();
        assert_eq!(cache.inspect_bin(1, 0).unwrap().total_count, 3);
        assert!(cache.inspect_bin(2, 0).is_none());
    }
}
