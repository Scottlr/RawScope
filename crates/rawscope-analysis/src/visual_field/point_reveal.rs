//! Settled, deterministic point-reveal planning for semantic zoom.
//!
//! The plan owns only row identities and disclosure counts.  It is therefore
//! safe to build on a worker and cheap to retain while the pointer moves.  A
//! renderer resolves those identities through its already resident projected
//! point buffer; this module never owns selection membership or GPU state.

use std::{cmp::Ordering, collections::BinaryHeap, error::Error, fmt, num::NonZeroU32};

use rawscope_core::{Generation, GenerationCounter, RowId};
use rawscope_data::DatasetGeneration;

use crate::{
    cohort::{CohortGeneration, CohortSnapshot},
    inspection::F64Domain,
};

use super::projection::{ProjectedVisualFieldGeneration, ProjectedVisualPoint};

const DEFAULT_DENSITY_ONLY_ABOVE_ROWS_PER_PIXEL: f32 = 0.12;
const DEFAULT_POINTS_ONLY_BELOW_ROWS_PER_PIXEL: f32 = 0.02;
const DEFAULT_MAX_RENDERED_POINTS: u32 = 20_000;
const DEFAULT_POINT_RADIUS_PX: f32 = 1.5;

/// Workbench-owned view identity used by analysis-only callers.  Renderers may
/// use their own owner token through the generic plan type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VisualFieldViewGeneration(Generation<VisualFieldViewOwner>);

impl VisualFieldViewGeneration {
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct VisualFieldViewOwner;

#[derive(Debug, Default)]
pub struct VisualFieldViewGenerationCounter(GenerationCounter<VisualFieldViewOwner>);

impl VisualFieldViewGenerationCounter {
    pub fn mint(&mut self) -> VisualFieldViewGeneration {
        VisualFieldViewGeneration(self.0.mint())
    }
}

/// Thresholds and resource limits for the density-to-point semantic zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticZoomPolicy {
    pub density_only_above_rows_per_pixel: f32,
    pub points_only_below_rows_per_pixel: f32,
    pub max_rendered_points: NonZeroU32,
    pub point_radius_px: f32,
}

impl Default for SemanticZoomPolicy {
    fn default() -> Self {
        Self {
            density_only_above_rows_per_pixel: DEFAULT_DENSITY_ONLY_ABOVE_ROWS_PER_PIXEL,
            points_only_below_rows_per_pixel: DEFAULT_POINTS_ONLY_BELOW_ROWS_PER_PIXEL,
            max_rendered_points: NonZeroU32::new(DEFAULT_MAX_RENDERED_POINTS)
                .expect("default point budget is non-zero"),
            point_radius_px: DEFAULT_POINT_RADIUS_PX,
        }
    }
}

impl SemanticZoomPolicy {
    pub fn try_new(
        points_only_below_rows_per_pixel: f32,
        density_only_above_rows_per_pixel: f32,
        max_rendered_points: u32,
        point_radius_px: f32,
    ) -> Result<Self, SemanticZoomPolicyError> {
        let thresholds_are_valid = points_only_below_rows_per_pixel.is_finite()
            && density_only_above_rows_per_pixel.is_finite()
            && points_only_below_rows_per_pixel >= 0.0
            && density_only_above_rows_per_pixel > points_only_below_rows_per_pixel;
        if !thresholds_are_valid {
            return Err(SemanticZoomPolicyError::InvalidThresholds);
        }
        let max_rendered_points =
            NonZeroU32::new(max_rendered_points).ok_or(SemanticZoomPolicyError::ZeroPointBudget)?;
        if !point_radius_px.is_finite() || point_radius_px <= 0.0 {
            return Err(SemanticZoomPolicyError::InvalidRadius);
        }
        Ok(Self {
            density_only_above_rows_per_pixel,
            points_only_below_rows_per_pixel,
            max_rendered_points,
            point_radius_px,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticZoomPolicyError {
    InvalidThresholds,
    ZeroPointBudget,
    InvalidRadius,
}

impl fmt::Display for SemanticZoomPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidThresholds => formatter
                .write_str("semantic zoom thresholds must be finite, non-negative, and increasing"),
            Self::ZeroPointBudget => {
                formatter.write_str("semantic zoom point budget must be positive")
            }
            Self::InvalidRadius => {
                formatter.write_str("semantic zoom point radius must be finite and positive")
            }
        }
    }
}

impl Error for SemanticZoomPolicyError {}

/// One bounded, continuous presentation decision.  Density remains as a
/// deliberately subdued contextual field at the point-only endpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticZoomFrame {
    pub density_alpha: f32,
    pub point_alpha: f32,
}

pub fn semantic_zoom_frame(
    rows_per_physical_pixel: f32,
    policy: SemanticZoomPolicy,
) -> SemanticZoomFrame {
    if !rows_per_physical_pixel.is_finite()
        || !policy.points_only_below_rows_per_pixel.is_finite()
        || !policy.density_only_above_rows_per_pixel.is_finite()
        || policy.points_only_below_rows_per_pixel < 0.0
        || policy.density_only_above_rows_per_pixel <= policy.points_only_below_rows_per_pixel
    {
        return SemanticZoomFrame {
            density_alpha: 1.0,
            point_alpha: 0.0,
        };
    }
    let point_alpha = if rows_per_physical_pixel <= policy.points_only_below_rows_per_pixel {
        1.0
    } else if rows_per_physical_pixel >= policy.density_only_above_rows_per_pixel {
        0.0
    } else {
        (policy.density_only_above_rows_per_pixel - rows_per_physical_pixel)
            / (policy.density_only_above_rows_per_pixel - policy.points_only_below_rows_per_pixel)
    }
    .clamp(0.0, 1.0);
    let density_alpha = (1.0 - point_alpha * 0.88).clamp(0.12, 1.0);
    SemanticZoomFrame {
        density_alpha,
        point_alpha,
    }
}

/// A viewport over the immutable projected field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointRevealViewport {
    pub x: F64Domain,
    pub y: F64Domain,
}

impl PointRevealViewport {
    pub const fn new(x: F64Domain, y: F64Domain) -> Self {
        Self { x, y }
    }

    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x.min() && x <= self.x.max() && y >= self.y.min() && y <= self.y.max()
    }
}

/// Immutable row plan produced for one settled view.
#[derive(Debug, Clone, PartialEq)]
pub struct PointRevealPlan<G = VisualFieldViewGeneration> {
    dataset_generation: DatasetGeneration,
    cohort_generation: CohortGeneration,
    view_generation: G,
    row_ids: std::sync::Arc<[RowId]>,
    eligible_count: u64,
    sampled: bool,
}

impl<G: Copy> PointRevealPlan<G> {
    pub fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }
    pub fn cohort_generation(&self) -> CohortGeneration {
        self.cohort_generation
    }
    pub fn view_generation(&self) -> G {
        self.view_generation
    }
    pub fn row_ids(&self) -> std::sync::Arc<[RowId]> {
        std::sync::Arc::clone(&self.row_ids)
    }
    pub fn eligible_count(&self) -> u64 {
        self.eligible_count
    }
    pub fn rendered_count(&self) -> u64 {
        self.row_ids.len() as u64
    }
    pub fn sampled(&self) -> bool {
        self.sampled
    }
    pub fn is_compatible(
        &self,
        dataset: DatasetGeneration,
        cohort: CohortGeneration,
        view: G,
    ) -> bool
    where
        G: PartialEq,
    {
        self.dataset_generation == dataset
            && self.cohort_generation == cohort
            && self.view_generation == view
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointRevealPlanError {
    InvalidViewport,
    ProjectionGenerationMismatch,
    CohortGenerationMismatch,
    DatasetGenerationMismatch,
    PointBudgetOverflow,
}

impl fmt::Display for PointRevealPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidViewport => "point reveal viewport must have finite increasing domains",
            Self::ProjectionGenerationMismatch => {
                "point reveal projection generation is inconsistent"
            }
            Self::CohortGenerationMismatch => "point reveal cohort generation is inconsistent",
            Self::DatasetGenerationMismatch => "point reveal projection and cohort datasets differ",
            Self::PointBudgetOverflow => "point reveal point budget overflowed usize",
        })
    }
}

impl Error for PointRevealPlanError {}

/// Build a deterministic bounded plan from one immutable projection/cohort.
/// The projected generation is traversed once; the heap never exceeds the
/// approved point budget.
pub fn build_point_reveal_plan<G: Copy>(
    projection: &ProjectedVisualFieldGeneration,
    cohort: &CohortSnapshot,
    viewport: PointRevealViewport,
    view_generation: G,
    policy: SemanticZoomPolicy,
) -> Result<PointRevealPlan<G>, PointRevealPlanError> {
    build_point_reveal_plan_from_points(
        projection.points().as_ref(),
        projection.dataset_generation(),
        cohort,
        viewport,
        view_generation,
        policy,
    )
}

/// Equivalent worker contract for callers that already retain projected
/// points in a renderer-owned resident dataset buffer.
pub fn build_point_reveal_plan_from_points<G: Copy>(
    points: &[ProjectedVisualPoint],
    dataset_generation: DatasetGeneration,
    cohort: &CohortSnapshot,
    viewport: PointRevealViewport,
    view_generation: G,
    policy: SemanticZoomPolicy,
) -> Result<PointRevealPlan<G>, PointRevealPlanError> {
    if dataset_generation != cohort.dataset_generation() {
        return Err(PointRevealPlanError::DatasetGenerationMismatch);
    }
    if viewport.x.min() >= viewport.x.max()
        || viewport.y.min() >= viewport.y.max()
        || !viewport.x.min().is_finite()
        || !viewport.x.max().is_finite()
        || !viewport.y.min().is_finite()
        || !viewport.y.max().is_finite()
    {
        return Err(PointRevealPlanError::InvalidViewport);
    }
    let budget = usize::try_from(policy.max_rendered_points.get())
        .map_err(|_| PointRevealPlanError::PointBudgetOverflow)?;
    let mut selected = BinaryHeap::<Candidate>::with_capacity(budget.min(points.len()));
    let mut eligible_count = 0_u64;
    for point in points.iter().copied() {
        if !viewport.contains(point.x, point.y)
            || cohort
                .included_row_ids()
                .binary_search(&point.row_id)
                .is_err()
        {
            continue;
        }
        eligible_count = eligible_count.saturating_add(1);
        let candidate = Candidate {
            priority: stable_row_priority(point.row_id),
            row_id: point.row_id,
        };
        if selected.len() < budget {
            selected.push(candidate);
        } else if selected.peek().is_some_and(|worst| candidate < *worst) {
            let _ = selected.pop();
            selected.push(candidate);
        }
    }
    let mut row_ids = selected
        .into_iter()
        .map(|candidate| candidate.row_id)
        .collect::<Vec<_>>();
    row_ids.sort_unstable();
    row_ids.dedup();
    Ok(PointRevealPlan {
        dataset_generation,
        cohort_generation: cohort.cohort_generation(),
        view_generation,
        row_ids: row_ids.into(),
        eligible_count,
        sampled: eligible_count > u64::from(policy.max_rendered_points.get()),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Candidate {
    priority: u64,
    row_id: RowId,
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.priority, self.row_id).cmp(&(other.priority, other.row_id))
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn stable_row_priority(row_id: RowId) -> u64 {
    let mut value = row_id.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use rawscope_core::ColumnId;
    use rawscope_data::{
        build_visual_field_catalog, ColumnChunk, DatasetChunk, DatasetGenerationCounter,
        DatasetIdentity, DatasetMemoryBudget, DatasetSchema, DatasetStoreBuilder, LoadedColumnKind,
        LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, NormalizedValue, SourceValue,
        StoreColumnKind, StoredCell, VisualFieldCatalogConfig,
    };

    use super::*;
    use crate::visual_field::{VisualFieldMapping, VisualFieldProjection};

    fn projection_and_cohort(count: u32) -> (ProjectedVisualFieldGeneration, CohortSnapshot) {
        let schema =
            DatasetSchema::try_new([("x", StoreColumnKind::F64), ("y", StoreColumnKind::F64)])
                .unwrap();
        let mut generations = DatasetGenerationCounter::default();
        let rows = (0..count)
            .map(|index| {
                vec![
                    StoredCell::value(
                        SourceValue::F64(index as f64),
                        NormalizedValue::F64(index as f64),
                    ),
                    StoredCell::value(
                        SourceValue::F64(index as f64),
                        NormalizedValue::F64(index as f64),
                    ),
                ]
            })
            .collect::<Vec<_>>();
        let columns = (0..2)
            .map(|column_index| {
                ColumnChunk::new(
                    ColumnId::new(column_index),
                    rows.iter()
                        .map(|row| row[column_index as usize].clone())
                        .collect(),
                )
            })
            .collect();
        let mut builder = DatasetStoreBuilder::new(
            DatasetIdentity::synthetic_scatter(11, count as usize),
            schema.clone(),
            DatasetMemoryBudget::new(1_000_000),
            &mut generations,
        );
        builder
            .append_chunk(DatasetChunk::new(RowId(0), columns))
            .unwrap();
        let store = builder.finish();
        let mapping = VisualFieldMapping::try_new(
            &schema,
            VisualFieldProjection::NumericPair {
                x: ColumnId::new(0),
                y: ColumnId::new(1),
            },
            None,
        )
        .unwrap();
        let projection = ProjectedVisualFieldGeneration::from_store(&store, mapping).unwrap();
        let mut cohort_generations = crate::cohort::CohortGenerationCounter::default();
        let mut filters = crate::cohort::CohortBuilder::new(crate::cohort::CohortPolicy::default());
        let source = LoadedSourceTable {
            columns: vec![
                LoadedColumnSchema {
                    name: "x".into(),
                    kind: LoadedColumnKind::Float,
                },
                LoadedColumnSchema {
                    name: "y".into(),
                    kind: LoadedColumnKind::Float,
                },
            ],
            rows: (0..count)
                .map(|index| LoadedSourceRow {
                    row_id: RowId(index as u64),
                    values: vec![index.to_string(), index.to_string()],
                })
                .collect(),
        };
        let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
        let cohort = filters
            .evaluate(
                &source,
                &catalog,
                store.generation(),
                &mut cohort_generations,
            )
            .unwrap();
        (projection, cohort)
    }

    #[test]
    fn semantic_zoom_alpha_is_monotonic_and_continuous() {
        let policy = SemanticZoomPolicy::default();
        let low = semantic_zoom_frame(0.0, policy);
        let middle = semantic_zoom_frame(0.07, policy);
        let high = semantic_zoom_frame(1.0, policy);
        assert_eq!(low.point_alpha, 1.0);
        assert_eq!(high.point_alpha, 0.0);
        assert!(middle.point_alpha > 0.0 && middle.point_alpha < 1.0);
        assert!(low.density_alpha <= middle.density_alpha);
        assert!(middle.density_alpha <= high.density_alpha);
    }

    #[test]
    fn settled_point_plan_is_deterministic_and_bounded() {
        let (projection, cohort) = projection_and_cohort(100);
        let policy = SemanticZoomPolicy::try_new(0.02, 0.12, 7, 1.5).unwrap();
        let viewport = PointRevealViewport::new(
            F64Domain::try_new(0.0, 100.0).unwrap(),
            F64Domain::try_new(0.0, 100.0).unwrap(),
        );
        let first = build_point_reveal_plan(&projection, &cohort, viewport, 1_u64, policy).unwrap();
        let second =
            build_point_reveal_plan(&projection, &cohort, viewport, 1_u64, policy).unwrap();
        assert_eq!(first.row_ids(), second.row_ids());
        assert_eq!(first.rendered_count(), 7);
        assert!(first.row_ids().windows(2).all(|rows| rows[0] < rows[1]));
    }

    #[test]
    fn sample_disclosure_matches_eligible_and_rendered_counts() {
        let (projection, cohort) = projection_and_cohort(4);
        let policy = SemanticZoomPolicy::try_new(0.02, 0.12, 10, 1.5).unwrap();
        let viewport = PointRevealViewport::new(
            F64Domain::try_new(0.0, 10.0).unwrap(),
            F64Domain::try_new(0.0, 10.0).unwrap(),
        );
        let plan = build_point_reveal_plan(&projection, &cohort, viewport, 1_u64, policy).unwrap();
        assert_eq!(plan.eligible_count(), plan.rendered_count());
        assert!(!plan.sampled());
    }
}
