//! Bounded category-composition layer planning over a store-owned index.

use std::{error::Error, fmt, sync::Arc};

use rawscope_core::{ColumnId, Generation, GenerationCounter, RowId};
use rawscope_data::{
    CategoryCodeKind, CategoryIndexAccuracy, CategoryMembershipIndex, CategoryValueId,
    DatasetGeneration,
};

use super::mapping::MAX_CATEGORY_COMPOSITION_LAYERS;
use crate::cohort::CohortSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CategoryLayerId(u8);

impl CategoryLayerId {
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryLayerKind {
    Value(CategoryValueId),
    Other,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryLayer {
    pub id: CategoryLayerId,
    pub kind: CategoryLayerKind,
    pub label: Arc<str>,
    pub row_count: u64,
    pub accuracy: CategoryIndexAccuracy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryLayerLookup {
    pub value_layers: Arc<[CategoryLayerId]>,
    pub untracked_layer: Option<CategoryLayerId>,
    pub missing_layer: Option<CategoryLayerId>,
    pub invalid_layer: Option<CategoryLayerId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CategoryLayerPlanGeneration(Generation<CategoryLayerPlanOwner>);

impl CategoryLayerPlanGeneration {
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug, Default)]
pub struct CategoryLayerPlanGenerationCounter(GenerationCounter<CategoryLayerPlanOwner>);

impl CategoryLayerPlanGenerationCounter {
    pub fn mint(&mut self) -> CategoryLayerPlanGeneration {
        CategoryLayerPlanGeneration(self.0.mint())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CategoryLayerPlanOwner;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryLayerPlan {
    dataset_generation: DatasetGeneration,
    column_id: ColumnId,
    generation: CategoryLayerPlanGeneration,
    selected_values: Arc<[CategoryValueId]>,
    layers: Arc<[CategoryLayer]>,
    lookup: CategoryLayerLookup,
    omitted_value_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryLayerPlanError {
    DatasetGenerationMismatch,
    RowCodeMissing { row_id: RowId },
    LayerCountOverflow,
}

impl fmt::Display for CategoryLayerPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatasetGenerationMismatch => formatter
                .write_str("category index and cohort belong to different dataset generations"),
            Self::RowCodeMissing { row_id } => {
                write!(formatter, "category index has no row code for {row_id:?}")
            }
            Self::LayerCountOverflow => {
                formatter.write_str("category layer count exceeds the bounded layer representation")
            }
        }
    }
}

impl Error for CategoryLayerPlanError {}

impl CategoryLayerPlan {
    pub fn build(
        index: &CategoryMembershipIndex,
        cohort: &CohortSnapshot,
        selected_values: &[CategoryValueId],
    ) -> Result<Self, CategoryLayerPlanError> {
        let mut generations = CategoryLayerPlanGenerationCounter::default();
        Self::build_with_generation(index, cohort, selected_values, generations.mint())
    }

    pub fn build_with_generation(
        index: &CategoryMembershipIndex,
        cohort: &CohortSnapshot,
        selected_values: &[CategoryValueId],
        generation: CategoryLayerPlanGeneration,
    ) -> Result<Self, CategoryLayerPlanError> {
        if index.dataset_generation() != cohort.dataset_generation() {
            return Err(CategoryLayerPlanError::DatasetGenerationMismatch);
        }
        let mut value_counts = vec![0_u64; index.values().len()];
        let mut other_count = 0_u64;
        let mut missing_count = 0_u64;
        let mut invalid_count = 0_u64;
        for row_id in cohort.included_row_ids().iter().copied() {
            match index
                .code(row_id)
                .ok_or(CategoryLayerPlanError::RowCodeMissing { row_id })?
            {
                CategoryCodeKind::Value(value_id) => {
                    if let Some(count) = value_counts.get_mut(value_id.get() as usize) {
                        *count = count.saturating_add(1);
                    } else {
                        return Err(CategoryLayerPlanError::RowCodeMissing { row_id });
                    }
                }
                CategoryCodeKind::Untracked => other_count = other_count.saturating_add(1),
                CategoryCodeKind::Missing => missing_count = missing_count.saturating_add(1),
                CategoryCodeKind::Invalid => invalid_count = invalid_count.saturating_add(1),
            }
        }
        let valid_values = value_counts.iter().filter(|count| **count > 0).count();
        let special_count = usize::from(missing_count > 0) + usize::from(invalid_count > 0);
        let value_capacity_without_other =
            usize::from(MAX_CATEGORY_COMPOSITION_LAYERS).saturating_sub(special_count);
        let other_needed = other_count > 0 || valid_values > value_capacity_without_other;
        let reserved = special_count + usize::from(other_needed);
        let value_capacity = usize::from(MAX_CATEGORY_COMPOSITION_LAYERS).saturating_sub(reserved);
        let mut chosen = Vec::<CategoryValueId>::new();
        for value_id in selected_values.iter().copied() {
            let index = value_id.get() as usize;
            if value_counts.get(index).is_some_and(|count| *count > 0)
                && !chosen.contains(&value_id)
                && chosen.len() < value_capacity
            {
                chosen.push(value_id);
            }
        }
        let mut remaining = value_counts
            .iter()
            .enumerate()
            .filter(|(_, count)| **count > 0)
            .map(|(index, count)| (CategoryValueId::new(index as u32), *count))
            .filter(|(value_id, _)| !chosen.contains(value_id))
            .collect::<Vec<_>>();
        remaining.sort_by(|(left_id, left_count), (right_id, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| left_id.cmp(right_id))
        });
        for (value_id, _) in remaining {
            if chosen.len() == value_capacity {
                break;
            }
            chosen.push(value_id);
        }
        let omitted_value_count = valid_values.saturating_sub(chosen.len()) as u64;
        let mut layers = Vec::new();
        let mut value_layers = vec![None; index.values().len()];
        for value_id in chosen.iter().copied() {
            let source = &index.values()[value_id.get() as usize];
            let id = next_layer_id(layers.len())?;
            value_layers[value_id.get() as usize] = Some(id);
            layers.push(CategoryLayer {
                id,
                kind: CategoryLayerKind::Value(value_id),
                label: Arc::clone(&source.label),
                row_count: value_counts[value_id.get() as usize],
                accuracy: source.accuracy,
            });
        }
        let untracked_layer = if other_needed {
            let id = next_layer_id(layers.len())?;
            for (index, count) in value_counts.iter().enumerate() {
                if *count > 0 && !chosen.contains(&CategoryValueId::new(index as u32)) {
                    value_layers[index] = Some(id);
                }
            }
            layers.push(CategoryLayer {
                id,
                kind: CategoryLayerKind::Other,
                label: Arc::<str>::from("Other"),
                row_count: other_count.saturating_add(
                    value_counts
                        .iter()
                        .enumerate()
                        .filter(|(index, count)| {
                            **count > 0 && !chosen.contains(&CategoryValueId::new(*index as u32))
                        })
                        .map(|(_, count)| *count)
                        .sum::<u64>(),
                ),
                accuracy: index.accuracy(),
            });
            Some(id)
        } else {
            None
        };
        let missing_layer = if missing_count > 0 {
            let id = next_layer_id(layers.len())?;
            layers.push(CategoryLayer {
                id,
                kind: CategoryLayerKind::Missing,
                label: Arc::<str>::from("Missing"),
                row_count: missing_count,
                accuracy: CategoryIndexAccuracy::Exact,
            });
            Some(id)
        } else {
            None
        };
        let invalid_layer = if invalid_count > 0 {
            let id = next_layer_id(layers.len())?;
            layers.push(CategoryLayer {
                id,
                kind: CategoryLayerKind::Invalid,
                label: Arc::<str>::from("Invalid"),
                row_count: invalid_count,
                accuracy: CategoryIndexAccuracy::Exact,
            });
            Some(id)
        } else {
            None
        };
        let fallback = layers
            .first()
            .map(|layer| layer.id)
            .unwrap_or(CategoryLayerId(0));
        let value_layers = value_layers
            .into_iter()
            .map(|value_layer| value_layer.unwrap_or(untracked_layer.unwrap_or(fallback)))
            .collect::<Vec<_>>();
        Ok(Self {
            dataset_generation: index.dataset_generation(),
            column_id: index.column_id(),
            generation,
            selected_values: selected_values.iter().copied().collect::<Vec<_>>().into(),
            layers: layers.into(),
            lookup: CategoryLayerLookup {
                value_layers: value_layers.into(),
                untracked_layer,
                missing_layer,
                invalid_layer,
            },
            omitted_value_count,
        })
    }

    pub fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }
    pub const fn column_id(&self) -> ColumnId {
        self.column_id
    }
    pub const fn generation(&self) -> CategoryLayerPlanGeneration {
        self.generation
    }
    pub fn layers(&self) -> &[CategoryLayer] {
        &self.layers
    }
    pub fn lookup(&self) -> &CategoryLayerLookup {
        &self.lookup
    }
    pub fn omitted_value_count(&self) -> u64 {
        self.omitted_value_count
    }
    pub fn is_compatible(
        &self,
        dataset: DatasetGeneration,
        column: ColumnId,
        selected: &[CategoryValueId],
    ) -> bool {
        self.dataset_generation == dataset
            && self.column_id == column
            && self.selected_values.as_ref() == selected
    }
}

fn next_layer_id(length: usize) -> Result<CategoryLayerId, CategoryLayerPlanError> {
    let id = u8::try_from(length).map_err(|_| CategoryLayerPlanError::LayerCountOverflow)?;
    if id >= MAX_CATEGORY_COMPOSITION_LAYERS {
        return Err(CategoryLayerPlanError::LayerCountOverflow);
    }
    Ok(CategoryLayerId(id))
}
