# Architecture Direction

## High-Level Pipeline

```text
Files / synthetic data
  -> chunked columnar data model
  -> visual query planner
  -> GPU resource layer
  -> WGPU compute passes
  -> WGPU render passes
  -> workbench UI
  -> row inspector / evidence export
```

The initial scaffold should keep this pipeline visible without pretending the later layers already exist.

## Planned Data Flow

Raw rows should eventually become typed, chunked, columnar buffers with stable row ids and dataset metadata. Visual views should request summaries through explicit view specs rather than by pulling arbitrary app state into the renderer.

The intended path is:

```text
raw files or synthetic generators
  -> decoded records or generated rows
  -> chunked columnar buffers
  -> visual query inputs
  -> aggregate bins, masks, tiles, or row-id mappings
  -> drilldown and evidence export
```

CSV and Parquet support are later concerns. The first implementation path should use deterministic synthetic point and event data.

## Planned Rendering Flow

RawScope should not render one object per row. A screen has limited pixels, and rendering one marker for every row does not produce a useful visual summary at very large scale.

The intended rendering path is:

```text
large raw rows
  -> chunked columnar buffers
  -> visual query
  -> GPU binning / density / masks / tiles
  -> rendered pixels
  -> row evidence on demand
```

Many rows become bins, bins become density textures, textures become visual fields, and visual fields support drilldown back to rows.

The current Milestone 6 slice keeps this narrower: `rawscope-render` owns GPU scatter-density compute, GPU timeline-density compute, simple scatter and timeline presentation renderers, scatter-specific viewport math, timeline time-axis viewport math, testable brush geometry, CPU-side selected-region summaries/evidence for synthetic points, CPU-side selected-event summaries/evidence for synthetic timeline events, a CPU-backed missingness reference grid and selection summary, deterministic JSON/Markdown scatter and timeline evidence artifact formatting, a CPU-backed selection drilldown model, and a minimal brush overlay pass. Compute uploads synthetic point/event coordinates, bins counts with WGSL atomics, and can read counts back for correctness and density colour scaling. Brush interaction separates in-progress screen-space drag rectangles from finalized data-space selections; summaries, evidence, drilldown, overlay projection, and exported artifacts use finalized data-space selections as their source of truth. Timeline brushes finalize into data-space time ranges plus half-open lane ranges, then re-project after zoom, pan, resize, or reset. Timeline evidence is still CPU-side over synthetic records: it samples the lowest selected row ids deterministically and is cached by the workbench until the brush or dataset changes. The missingness slice operates on retained local source rows, buckets rows deterministically, and summarizes selected cells back to missing counts, column names, and sorted row ids without requiring a GPU path. The native workbench renders scatter and timeline density counts with the same WGPU device/queue used by the window surface, then composites the projected brush rectangle and egui shell after the density pass, avoiding a separate compute device in the visual path. `rawscope-workbench` translates winit mouse/keyboard events into viewport updates, brush updates, deterministic point-count presets, visible egui workbench state, missingness cell selection for local CSV data, one-shot evidence logging, and collision-safe local report-bundle writes that include evidence JSON, Markdown, manifest metadata, and a deterministic visual-context placeholder, then asks the active renderer to re-bin the visible range. Timeline density uses a startup `--demo timeline` path that generates deterministic synthetic events, computes GPU timeline-density counts, renders a simple full-window density view, and translates wheel/drag/reset input into time-range viewport changes while keeping lane mapping stable. Native image capture remains deferred behind the documented readback/PNG design, and `rawscope-gpu` continues to own both the reusable headless compute context for ignored correctness tests and the window surface context. GPU row-id preservation and broader file-dialog/report UI remain deferred until a later evidence-focused design.

## Visual Query Concept

A visual query describes the data fields, ranges, grouping, aggregation, filters, view transform, and selection state needed to render a view. It is not a general SQL replacement. It is the contract between data, render, UI, and evidence layers for answering a visual question.

Examples of future visual query inputs include:

- x and y columns for scatter density
- time and lane columns for timeline density
- column or partition dimensions for missingness heatmaps
- baseline and selected cohorts for distribution drift
- key columns for dataset diff views

## Initial Crate Responsibilities

- `rawscope-core`: shared types, ranges, dimensions, view specs, selections, errors, and visual query definitions
- `rawscope-data`: future columnar abstractions, dataset metadata, row ids, schema summaries, chunk store, and file readers later
- `rawscope-gpu`: WGPU device/session abstraction, surface bootstrap, headless compute bootstrap, and future buffer allocation, texture allocation, compute pipeline cache, shader loading, and GPU timing hooks
- `rawscope-render`: CPU density references, correctness-first GPU scatter-density compute, and future density renderers, heatmap renderers, timeline renderers, selection overlays, axes, grids, and crosshair helpers
- `rawscope-workbench`: desktop app shell that eventually opens projects/datasets, hosts views, manages layout, coordinates interactions, and exports reports

## Separation Of Concerns

- `rawscope-core` should not depend on GPU or UI crates.
- `rawscope-data` should not know about rendering or app-specific UI.
- `rawscope-gpu` should own GPU resource concepts without knowing analyst workflows.
- `rawscope-render` should depend on core/GPU concepts, not app-specific UI.
- Future egui integration should adapt library functionality into UI widgets and panels without pulling UI concerns into core data/render crates.
- `rawscope-workbench` should coordinate the application, not absorb all domain logic.

## Important Visual Primitives

- Timeline density: event data where x is time, y is source/category/lane, and pixels encode count, error rate, latency, or another aggregate.
- Scatter density: numeric relationships where x and y are numeric columns, pixels encode row density, and overlays show category, segment, or selected cohort.
- Missingness heatmap: data quality views where pixels encode null rate, missing count, provider gaps, source freshness, or schema degradation.
- Distribution drift: comparisons across populations such as before/after deploy, old/new model, training/production, provider A/provider B, or yesterday/today.
- Dataset diff: before/after dataset comparison for new keys, missing keys, changed values, schema diff, row count changes, distribution drift, category explosions, and null-rate changes.
- Interval/state timeline: spans and windows such as suspended/open intervals, stale provider windows, incidents, job execution spans, and trace spans.

## Copy-Minimising, Not Magical Zero-Copy

- CSV and Parquet inputs require decoding before they become useful in memory.
- CPU memory and GPU memory are separate on many systems.
- GPU upload requires staging or copying even in efficient pipelines.
- WGPU resources live in GPU buffers and textures.
- Full end-to-end zero-copy is not a safe general claim.

Optimisation should focus on avoiding unnecessary CPU copies, using columnar buffers, chunking data, uploading reusable GPU buffers, keeping GPU resources resident, avoiding GPU readback during interaction, and generating summaries close to where they are consumed.

## First Vertical Slice

The next vertical slice should build deterministic synthetic point/event datasets and a CPU-side density reference implementation. That gives future GPU work a known-correct comparison target before introducing WGPU complexity.
