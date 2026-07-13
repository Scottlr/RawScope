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

Synthetic data arrived first so the visual/evidence contract could stabilize. The current local-data path now supports explicit CSV bindings plus narrow Parquet-backed chunked ingestion for scatter and timeline datasets.

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

The generic visual-field path is now the shared owner for continuous two-
dimensional analysis. `rawscope-analysis::visual_field` validates one
profile-independent numeric-pair or time-value mapping, preserves typed domains,
and owns exact count-derived marginals, tied mass contours, bounded category
composition, cohort-share differences, semantic density-to-point planning, and
multiscale undirected ridges. `rawscope-render::visual_field` owns the resident
field/resource contract, reprojection, palette LUT, resolution policy, and
derived presentation resources. `rawscope-workbench` owns the typed controller,
session-role adaptation, command/generation intent, and canonical report
adapter; `rawscope-evidence` owns the versioned visual-field artifact. The old
scatter and discrete-lane timeline codecs remain narrow compatibility owners for
their documented wire versions rather than parallel generic runtime state.

Neutral deterministic workbench integration tests and the benchmark targets in
`docs/BENCHMARKS.md` exercise these owners without requiring an external profile
or dataset. The native launcher still exposes the established scatter and
discrete-lane timeline demos, so the generic contracts are not described here as
a universal chart switch or a claim of measured performance on every machine.

The native launcher keeps this narrower: `rawscope-render` owns GPU scatter-density compute, GPU timeline-density compute, simple scatter and timeline presentation renderers, scatter-specific viewport math, timeline time-axis viewport math, testable brush geometry, CPU-side selected-region summaries/evidence for synthetic points, CPU-side selected-event summaries/evidence for synthetic timeline events, a CPU-backed missingness reference grid and selection summary, deterministic JSON/Markdown scatter and timeline evidence artifact formatting, a CPU-backed selection drilldown model, and a minimal brush overlay pass. `rawscope-data` now owns explicit local CSV readers plus narrow Parquet-backed chunk metadata, schema summaries, binding validation, and retained source rows for the current scatter/timeline workflows. Compute uploads synthetic point/event coordinates, bins counts with WGSL atomics, and can read counts back for correctness and density colour scaling. Brush interaction separates in-progress screen-space drag rectangles from finalized data-space selections; summaries, evidence, drilldown, overlay projection, and exported artifacts use finalized data-space selections as their source of truth. Timeline brushes finalize into data-space time ranges plus half-open lane ranges, then re-project after zoom, pan, resize, or reset. Timeline evidence is still CPU-side over synthetic records: it samples the lowest selected row ids deterministically and is cached by the workbench until the brush or dataset changes. The missingness slice operates on retained local source rows, buckets rows deterministically, and summarizes selected cells back to missing counts, column names, and sorted row ids without requiring a GPU path. The native workbench renders scatter and timeline density counts with the same WGPU device/queue used by the window surface, then composites the projected brush rectangle and egui shell after the density pass, avoiding a separate compute device in the visual path. `rawscope-workbench` translates winit mouse/keyboard events into viewport updates, brush updates, deterministic point-count presets, visible egui workbench state, missingness cell selection for local CSV or Parquet data, one-shot evidence logging, and collision-safe local report-bundle writes that include evidence JSON, Markdown, manifest metadata, and a deterministic visual-context placeholder, then asks the active renderer to re-bin the visible range. Timeline density uses a startup `--demo timeline` path that generates deterministic synthetic events, computes GPU timeline-density counts, renders a simple full-window density view, and translates wheel/drag/reset input into time-range viewport changes while keeping lane mapping stable. Native image capture remains deferred behind the documented readback/PNG design, and `rawscope-gpu` continues to own both the reusable headless compute context for ignored correctness tests and the window surface context. GPU row-id preservation and broader file-dialog/report UI remain deferred until a later evidence-focused design.

## Visual Query Concept

A visual query describes the data fields, ranges, grouping, aggregation, filters, view transform, and selection state needed to render a view. It is not a general SQL replacement. It is the contract between data, render, UI, and evidence layers for answering a visual question.

Examples of visual query inputs include:

- x and y columns for scatter density
- time and lane columns for timeline density
- column or partition dimensions for missingness heatmaps
- baseline and selected cohorts for distribution drift
- key columns for dataset diff views

## Crate Responsibilities

- `rawscope-core`: shared types, ranges, dimensions, view specs, selections, errors, and visual query definitions
- `rawscope-data`: dataset metadata, local CSV and Parquet readers, retained source rows, schema summaries, and narrow chunked columnar metadata for current local datasets
- `rawscope-gpu`: WGPU device/session abstraction, surface bootstrap, headless compute bootstrap, checked limits, recovery, and generation-safe readback mechanics
- `rawscope-render`: CPU references plus resident visual-field resources, GPU density/composition/ridge presentation owners, palettes, reprojection, resolution/resource policy, selection overlays, axes, and timeline renderers
- `rawscope-workbench`: desktop app shell, typed visual-field controller/session adaptation, layout, interaction coordination, and canonical/legacy report-bundle adapters

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
