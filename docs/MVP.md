# MVP Milestones

## Milestone 0: Docs + Compiling Scaffold

- Purpose: establish project boundaries, documentation, and workspace structure.
- Acceptance criteria: docs exist, the workspace compiles, crates are minimal, and the workbench prints a placeholder message.
- Non-goals: GPU rendering, file import, polished UI, and broad abstractions.

## Milestone 1: Synthetic Data + CPU Reference

- Purpose: create deterministic datasets and CPU-side reference outputs for future renderer tests.
- Acceptance criteria: synthetic point data, synthetic event data, deterministic generation, CPU density binning reference, and tests proving determinism.
- Non-goals: WGPU, UI polish, Parquet, CSV import, and external data connectors.

## Milestone 2: WGPU Bootstrap

- Purpose: create a GPU device/session and simple render surface.
- Acceptance criteria: the app opens a native window, WGPU initializes, a clear screen or basic render pass works, and GPU diagnostics are visible or logged.
- Non-goals: data rendering, advanced interaction, and production renderer architecture.

## Milestone 3: GPU Scatter Density

- Purpose: prove that raw synthetic points can become density pixels.
- Acceptance criteria: synthetic points upload to GPU resources, a GPU or GPU-assisted density view renders, basic zoom/pan works, frame timing is shown, and CPU reference comparison exists where practical.
- Non-goals: generic scatter-plot feature breadth, external file support, and dashboard-style configuration.

## Milestone 4: GPU Timeline Density

- Purpose: prove that event data can be explored as time/source density.
- Acceptance criteria: synthetic event data renders as time x lane density, spike/gap patterns are visible, and basic brush selection works.
- Non-goals: full time-series analysis tooling, annotation systems, and production evidence export.

## Milestone 5: Linked Selection

- Purpose: bridge visual patterns to data evidence.
- Acceptance criteria: a user can brush a region, see a selection summary, inspect top contributors, and drill down to sample or exact synthetic row ids.
- Non-goals: full crossfilter dashboards, arbitrary multi-view coordination, and data editing.

## Milestone 6: First Evidence Export

- Purpose: make findings shareable and repeatable.
- Acceptance criteria: a selected region can export Markdown, HTML, or JSON containing view config, generation seed or dataset fingerprint, summary data, sample rows, and visual context.
- Non-goals: polished reporting templates, collaboration features, cloud publishing, and broad report customization.
