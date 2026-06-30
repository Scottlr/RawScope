# MVP Milestones

## Milestone 0: Docs + Compiling Scaffold

- Purpose: establish project boundaries, documentation, and workspace structure.
- Acceptance criteria: docs exist, the workspace compiles, crates are minimal, and the workbench prints a placeholder message.
- Non-goals: GPU rendering, file import, polished UI, and broad abstractions.

## Milestone 1: Synthetic Data + CPU Reference

- Purpose: create deterministic datasets and CPU-side reference outputs for future renderer tests.
- Acceptance criteria: synthetic point data, synthetic event data, deterministic generation, CPU density binning reference, and tests proving determinism.
- Non-goals: WGPU, UI polish, Parquet, CSV import, and external data connectors.
- Status note: the repository now includes deterministic synthetic point/event generators, stable row ids, and CPU reference density binning. GPU bootstrap and render-surface work remain for Milestone 2.

## Milestone 2: WGPU Bootstrap

- Purpose: create a GPU device/session and simple render surface.
- Acceptance criteria: the app opens a native window, WGPU initializes, a clear screen or basic render pass works, and GPU diagnostics are visible or logged.
- Non-goals: data rendering, advanced interaction, and production renderer architecture.
- Status note: the workbench now initializes WGPU through `rawscope-gpu`, opens a native `winit` window, logs adapter diagnostics, handles resize, and clears the surface to a solid colour. Density rendering remains deferred to Milestone 3.

## Milestone 3: GPU Scatter Density

- Purpose: prove that raw synthetic points can become density pixels.
- Acceptance criteria: synthetic points upload to GPU resources, a GPU or GPU-assisted density view renders, basic zoom/pan works, frame timing is shown, and CPU reference comparison exists where practical.
- Non-goals: generic scatter-plot feature breadth, external file support, and dashboard-style configuration.
- Status note: Milestone 3A adds a correctness-first WGPU compute path for scatter-density counts and compares it against the CPU reference in ignored local-GPU tests. It does not yet render density pixels, add zoom/pan, or claim performance.
- Status note: Milestone 3B renders deterministic synthetic scatter-density counts in the native workbench using the same WGPU device/queue for compute and presentation. The view is a simple visual proof with log-scaled colour, not a polished UI, benchmark, interaction model, or row-drilldown path.
- Status note: Milestone 3C adds basic viewport interaction for the workbench scatter-density view: mouse-wheel zoom, mouse-drag pan, `R` reset, viewport re-binning, and CPU-observed diagnostics. It still excludes brushing, row drilldown, timeline density, egui, file import, Tauri, and performance claims.
- Status note: Milestone 3D hardens the scatter-density demo with title-bar diagnostics, deterministic point-count presets from 20,000 to 5,000,000 points, throttled update/frame diagnostics, and documented controls. In-app screenshot capture remains deferred to avoid adding image encoding dependencies or a broader surface readback path.
- Status note: Milestone 3E adds a basic scatter brush via right-drag or Shift + left-drag, CPU-side selected-region summaries over synthetic points, Escape-to-clear behavior, and title-bar/log reporting. The slice intentionally does not add a row table, GPU row-id preservation, screenshot readback, or a drawn brush overlay.
- Status note: Milestone 3F adds a minimal screen-space brush rectangle overlay rendered after the scatter-density pass. The overlay uses a simple amber fill and border, remains visible after finalizing a brush, and clears on Escape, reset, or preset switch. Zoom and pan deliberately keep the existing screen-space brush until it is cleared or replaced.
- Status note: Milestone 3G makes finalized scatter brushes data-anchored. Dragging remains screen-space, but final selections are stored as data-space x/y ranges, summaries use those ranges, and the overlay is projected into the current viewport after zoom, pan, resize, or reset. Fully offscreen selections are hidden, partially visible selections are clamped to the viewport edge, Escape clears, and preset changes clear because the dataset changes.

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
