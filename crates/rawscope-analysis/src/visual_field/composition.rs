//! Exact category-composition summaries over one bounded visible layer set.

use std::{error::Error, fmt, sync::Arc};

use super::category_layers::CategoryLayerId;
use super::mapping::MAX_CATEGORY_COMPOSITION_LAYERS;

/// One exact layer share in a settled composition cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositionLayerShare {
    pub layer_id: CategoryLayerId,
    pub count: u32,
    pub share: f64,
}

/// Dominant category and normalized mixedness for one exact cell.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionCellSummary {
    pub total_count: u64,
    pub dominant_layer: Option<CategoryLayerId>,
    pub dominant_share: f64,
    pub normalized_entropy: f64,
    pub purity: f64,
    pub layers: Arc<[CompositionLayerShare]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionError {
    EmptyLayerSet,
    LayerCountOverflow,
    CountOverflow,
    NonFiniteResult,
}

impl fmt::Display for CompositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLayerSet => {
                formatter.write_str("composition requires at least one visible layer")
            }
            Self::LayerCountOverflow => formatter.write_str(
                "composition layer count exceeds the bounded visible layer representation",
            ),
            Self::CountOverflow => formatter.write_str("composition layer counts exceed u64"),
            Self::NonFiniteResult => {
                formatter.write_str("composition entropy produced a non-finite result")
            }
        }
    }
}

impl Error for CompositionError {}

/// Summarizes a fixed visible layer set using normalized Shannon entropy.
///
/// The layer slice is the plan's fixed visible layer count, not the number of
/// occupied categories in this particular cell. Ties retain the lowest stable
/// `CategoryLayerId` because the input order is the plan order.
pub fn summarize_composition_cell(
    layer_counts: &[u32],
) -> Result<CompositionCellSummary, CompositionError> {
    if layer_counts.is_empty() {
        return Err(CompositionError::EmptyLayerSet);
    }
    if layer_counts.len() > usize::from(MAX_CATEGORY_COMPOSITION_LAYERS) {
        return Err(CompositionError::LayerCountOverflow);
    }
    let total_count = layer_counts.iter().try_fold(0_u64, |total, count| {
        total
            .checked_add(u64::from(*count))
            .ok_or(CompositionError::CountOverflow)
    })?;
    if total_count == 0 {
        return Ok(CompositionCellSummary {
            total_count,
            dominant_layer: None,
            dominant_share: 0.0,
            normalized_entropy: 0.0,
            purity: 0.0,
            layers: layer_shares(layer_counts, total_count),
        });
    }
    let (dominant_index, dominant_count) = layer_counts
        .iter()
        .copied()
        .enumerate()
        .max_by_key(|(index, count)| (*count, std::cmp::Reverse(*index)))
        .ok_or(CompositionError::EmptyLayerSet)?;
    let mut entropy = 0.0_f64;
    for count in layer_counts.iter().copied().filter(|count| *count > 0) {
        let share = f64::from(count) / total_count as f64;
        entropy -= share * share.ln();
    }
    let normalized_entropy = if layer_counts.len() <= 1 {
        0.0
    } else {
        entropy / (layer_counts.len() as f64).ln()
    };
    let purity = (1.0 - normalized_entropy).clamp(0.0, 1.0);
    let dominant_share = f64::from(dominant_count) / total_count as f64;
    if !normalized_entropy.is_finite() || !purity.is_finite() || !dominant_share.is_finite() {
        return Err(CompositionError::NonFiniteResult);
    }
    Ok(CompositionCellSummary {
        total_count,
        dominant_layer: Some(CategoryLayerId::new(dominant_index as u8)),
        dominant_share,
        normalized_entropy,
        purity,
        layers: layer_shares(layer_counts, total_count),
    })
}

fn layer_shares(layer_counts: &[u32], total_count: u64) -> Arc<[CompositionLayerShare]> {
    layer_counts
        .iter()
        .copied()
        .enumerate()
        .map(|(index, count)| CompositionLayerShare {
            layer_id: CategoryLayerId::new(index as u8),
            count,
            share: if total_count == 0 {
                0.0
            } else {
                f64::from(count) / total_count as f64
            },
        })
        .collect::<Vec<_>>()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_summary_handles_empty_single_mixed_and_tied_cells() {
        assert!(matches!(
            summarize_composition_cell(&[]),
            Err(CompositionError::EmptyLayerSet)
        ));
        let empty = summarize_composition_cell(&[0, 0]).unwrap();
        assert_eq!(empty.dominant_layer, None);
        assert_eq!(empty.purity, 0.0);
        let single = summarize_composition_cell(&[4]).unwrap();
        assert_eq!(single.dominant_layer, Some(CategoryLayerId::new(0)));
        assert_eq!(single.purity, 1.0);
        let mixed = summarize_composition_cell(&[2, 2]).unwrap();
        assert_eq!(mixed.dominant_layer, Some(CategoryLayerId::new(0)));
        assert!((mixed.purity - 0.0).abs() < 1.0e-12);
        let tied = summarize_composition_cell(&[3, 3, 1]).unwrap();
        assert_eq!(tied.dominant_layer, Some(CategoryLayerId::new(0)));
        assert!(matches!(
            summarize_composition_cell(&[0; 9]),
            Err(CompositionError::LayerCountOverflow)
        ));
    }

    #[test]
    fn composition_purity_uses_fixed_layer_count_without_occupancy_discontinuity() {
        let two_layer = summarize_composition_cell(&[5, 0]).unwrap();
        let three_layer = summarize_composition_cell(&[5, 0, 0]).unwrap();
        assert_eq!(two_layer.purity, 1.0);
        assert_eq!(three_layer.purity, 1.0);
        let balanced = summarize_composition_cell(&[1, 1, 1]).unwrap();
        assert!(balanced.purity.abs() < 1.0e-12);
    }

    #[test]
    fn composition_summary_exposes_exact_layer_shares() {
        let summary = summarize_composition_cell(&[1, 2, 1]).unwrap();
        assert_eq!(summary.total_count, 4);
        assert_eq!(summary.layers[1].count, 2);
        assert_eq!(summary.layers[1].share, 0.5);
        assert!(
            (summary.layers.iter().map(|layer| layer.share).sum::<f64>() - 1.0).abs() < 1.0e-12
        );
    }
}
