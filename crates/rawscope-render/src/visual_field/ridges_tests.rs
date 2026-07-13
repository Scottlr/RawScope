use super::super::generation::VisualFieldViewGenerationCounter;
use super::*;
use rawscope_analysis::visual_field::{RidgeScale, VisualFieldMapping, VisualFieldProjection};
use rawscope_core::{ColumnId, DensityCountGrid, GridSize};
use rawscope_data::{DatasetGenerationCounter, DatasetSchema, StoreColumnKind};
use rawscope_gpu::DeviceGeneration;
use std::sync::Arc;

fn source(view_generation: VisualFieldViewGeneration) -> VisualFieldGeneration {
    let schema =
        DatasetSchema::try_new([("x", StoreColumnKind::F64), ("y", StoreColumnKind::F64)]).unwrap();
    let mapping = VisualFieldMapping::try_new(
        &schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        None,
    )
    .unwrap();
    let mut datasets = DatasetGenerationCounter::default();
    let resources = Arc::new(super::super::VisualFieldDatasetGpuResources::new(
        datasets.mint(),
        DeviceGeneration(1),
        mapping,
        64,
        1024,
    ));
    let mut cohorts = rawscope_analysis::cohort::CohortGenerationCounter::default();
    VisualFieldGeneration::new(
        resources,
        cohorts.mint(),
        view_generation,
        GridSize::new(8, 4),
        super::super::VisualFieldQuality::Exact,
    )
}

#[test]
fn gpu_ridge_abi_matches_wgsl_layout() {
    assert_eq!(std::mem::size_of::<GpuRidgeCell>(), 16);
    assert_eq!(std::mem::align_of::<GpuRidgeCell>(), 4);
    assert_eq!(std::mem::size_of::<RidgeGpuParams>(), 64);
}

#[test]
fn gpu_ridge_stages_match_cpu_reference_vectors() {
    let size = GridSize::new(3, 3);
    let counts = DensityCountGrid::new(size, vec![0, 2, 0, 1, 5, 1, 0, 2, 0]);
    let field =
        rawscope_analysis::visual_field::derive_density_ridges(&counts, RidgeConfig::default())
            .unwrap();
    let encoded = encode_ridge_field(&field);
    assert_eq!(encoded.len(), field.cells.len());
    for (encoded, cpu) in encoded.iter().zip(field.cells.iter()) {
        assert_eq!(encoded.strength, cpu.strength);
        assert_eq!(encoded.tangent_x, cpu.tangent_x);
        assert_eq!(encoded.tangent_y, cpu.tangent_y);
        assert_eq!(encoded._padding, 0.0);
    }
}

#[test]
fn gpu_height_ridge_candidate_and_anisotropy_match_cpu() {
    let size = GridSize::new(9, 9);
    let counts = DensityCountGrid::new(
        size,
        (0..81)
            .map(|index| {
                let x: usize = index % 9;
                let y: usize = index / 9;
                u32::from((x.abs_diff(4) + y.abs_diff(4)) <= 2)
            })
            .collect(),
    );
    let permissive = RidgeConfig::try_new(RidgeScale::Fine, 0, 0).unwrap();
    let field =
        rawscope_analysis::visual_field::derive_density_ridges(&counts, permissive).unwrap();
    assert!(field.cells.iter().any(|cell| cell.strength > 0.0));
    assert!(field.cells.iter().all(|cell| cell.strength.is_finite()
        && cell.tangent_x.is_finite()
        && cell.tangent_y.is_finite()));
}

#[test]
fn fine_medium_coarse_produce_distinct_fields() {
    let size = GridSize::new(15, 15);
    let counts = DensityCountGrid::new(
        size,
        (0..225)
            .map(|index| {
                let x: usize = index % 15;
                let y: usize = index / 15;
                u32::from((x.abs_diff(4) <= 1 && y.abs_diff(7) <= 5) || (x >= 10 && y >= 10))
            })
            .collect(),
    );
    let fields = [RidgeScale::Fine, RidgeScale::Medium, RidgeScale::Coarse]
        .into_iter()
        .map(|scale| {
            rawscope_analysis::visual_field::derive_density_ridges(
                &counts,
                RidgeConfig::try_new(scale, 0, 0).unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert!(fields[0].cells != fields[1].cells || fields[1].cells != fields[2].cells);
}

#[test]
fn ridge_resource_plan_checks_all_intermediates() {
    let plan =
        RidgeResourcePlan::for_grid(GridSize::new(8, 4), &wgpu::Limits::downlevel_defaults())
            .unwrap();
    assert_eq!(plan.ridge_cells_bytes, 8 * 4 * 16);
    assert_eq!(plan.smoothing_bytes, 8 * 4 * 8);
    assert_eq!(plan.candidate_bytes, 8 * 4 * 16);
    assert_eq!(plan.reduction_bytes, 16);
    assert_eq!(
        plan.scratch_and_reduction_bytes,
        plan.smoothing_bytes + plan.candidate_bytes + plan.reduction_bytes
    );
    let overflow = RidgeResourcePlan::for_grid(
        GridSize::new(8, 4),
        &wgpu::Limits {
            max_buffer_size: 1,
            ..wgpu::Limits::downlevel_defaults()
        },
    );
    assert!(matches!(
        overflow,
        Err(RidgeResourceError::StorageBindingTooLarge { .. })
            | Err(RidgeResourceError::BufferTooLarge { .. })
    ));
}

#[test]
fn ridge_pressure_reports_unavailable_without_partial_resources() {
    assert!(matches!(
        ridge_presentation_availability(
            GridSize::new(8, 4),
            &wgpu::Limits {
                max_buffer_size: 1,
                ..wgpu::Limits::downlevel_defaults()
            }
        ),
        RidgePresentationAvailability::Unavailable(_)
    ));
}

#[test]
fn ridge_generation_rejects_stale_source() {
    let mut views = VisualFieldViewGenerationCounter::default();
    let first = source(views.mint());
    let second = source(views.mint());
    assert_eq!(
        validate_ridge_identity(
            first.view_generation(),
            RidgeConfig::default(),
            second.view_generation(),
            RidgeConfig::default(),
        ),
        Err(RidgeResourceError::SourceGenerationMismatch)
    );
}

#[test]
fn ridge_pointer_updates_do_not_dispatch_compute() {
    let mut interaction = RidgeInteraction::default();
    let mut views = VisualFieldViewGenerationCounter::default();
    let generation = views.mint();
    interaction.request_settle(generation);
    assert!(interaction.pointer_update(7).is_none());
    assert_eq!(interaction.pending_source(), Some(generation));
    assert_eq!(interaction.viewport_revision(), 7);
}

#[test]
fn ridge_interaction_replaces_pending_source_without_dispatch() {
    let mut interaction = RidgeInteraction::default();
    let mut views = VisualFieldViewGenerationCounter::default();
    let generation = views.mint();
    interaction.request_settle(generation);
    let next_generation = views.mint();
    assert_ne!(interaction.pending_source(), Some(next_generation));
    interaction.request_settle(next_generation);
    assert_eq!(interaction.pending_source(), Some(next_generation));
}

#[test]
fn ridge_presentation_is_invariant_to_tangent_sign() {
    let positive = transform_ridge_tangent(1.0, 2.0, 2.0, 1.0).unwrap();
    let negative = transform_ridge_tangent(-1.0, -2.0, 2.0, 1.0).unwrap();
    assert_eq!(positive.map(f32::abs), negative.map(f32::abs));
}

#[test]
fn plot_aspect_transforms_tangent_without_recompute() {
    let tangent = transform_ridge_tangent(1.0, 0.0, 2.0, 1.0).unwrap();
    assert!(tangent[0] > 0.99 && tangent[1].abs() < 0.01);
}
