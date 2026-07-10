# Next-Generation GPU Visual Analytics Tasks

## Discovery Summary

This bundle plans a next-generation scatter-analysis experience that uses the
local 200,000-game Lichess profile dataset as the flagship acceptance dataset
while keeping every new contract useful for other local numeric datasets.
The feature is intentionally density-first: visual beauty must strengthen the
analyst's understanding of rows, ranges, filters, comparisons, and evidence.

Files and behavior inspected:

- `AGENTS.md`, `docs/AGENTS.md`, and the repository Rust skills for governance,
  ownership, testing, constants, observability, and file-size constraints.
- `research/rawscope-gpu-scale-visual-analytics.md` for the research position
  that RawScope should improve truthfulness, comparison, multiscale behavior,
  and row evidence before becoming a generic charting or dashboard product.
- `docs/ARCHITECTURE.md`, `docs/TECH_DECISIONS.md`, `docs/NON_GOALS.md`, and
  `docs/BENCHMARKS.md` for crate boundaries and performance-claim discipline.
- Root and crate manifests for the five-member Rust/WGPU workspace.
- `apps/rawscope-workbench/src/app.rs`, `app_events.rs`, `app_render.rs`,
  `ui.rs`, `ui_controls.rs`, `ui_view_context.rs`, `ui_visual_encoding.rs`, and
  `ui_theme.rs` for current application state, full-surface rendering, input
  translation, fixed axis guard margins, visual controls, and theme behavior.
- `crates/rawscope-render/src/scatter_density_renderer.rs`,
  `gpu_scatter_density.rs`, `scatter_density_presentation.rs`,
  `density_encoding.rs`, `scatter_viewport.rs`, `view_axes.rs`,
  `aggregate_cache.rs`, brush geometry, and scatter WGSL shaders.
- `crates/rawscope-data/src/local_dataset.rs`, `visual_record.rs`, and
  `dataset_profile.rs` for retained source rows, schema kinds, visual records,
  and the current `lichess-games` profile.
- Evidence v1/v2/v3 structs, serializers, workbench report routing, and focused
  package tests.

Existing patterns to preserve:

- `rawscope-core` is dependency-free and owns only foundational contracts.
- `rawscope-data` owns loaded schema/source rows, profile contracts, field
  catalogs, parsing, and reusable CPU filtering over retained local data.
- `rawscope-gpu` owns WGPU context and reusable GPU resource mechanics.
- `rawscope-render` owns viewport math, density compute/render contracts,
  shaders, overlays, visual summaries, inspection grids, and evidence DTOs.
- `rawscope-workbench` owns egui/winit layout, interaction mode, UI projection,
  frame scheduling, dataset coordination, and report writes.
- Evidence versions are additive. Existing v1, v2, and v3 APIs and artifact
  shapes remain callable and unchanged when v4 is added.
- The real 200,000-game file stays outside the repository under the user's
  `RawScope-data` directory. Small deterministic test fixtures may mirror its
  schema, but agents must not commit or download the large source dataset.

The highest-impact discovered problems are concrete:

- Density is drawn across the full swapchain while egui and axes independently
  reserve space; `ui_view_context.rs` compensates with fixed guard constants.
- Pan, zoom, brush projection, axis drawing, and WGPU rendering do not share one
  authoritative physical plot rectangle.
- Every scatter viewport update recreates point/output/readback resources,
  rebuilds pipelines/bind groups, uploads all points, synchronously reads the
  full count grid, and scans CPU marginals.
- The source table retains useful Lichess dimensions (`winner`, `opening`,
  `time_control`, and others), but there is no field catalog or filter mask.
- Dense bins have deterministic row samples, but hover inspection and point
  reveal are not exposed as first-class interactions.
- `app.rs`, `ui.rs`, `ui_view_context.rs`, and report owners already exceed the
  repository's approximate 400-line review threshold.

## Planning Invariants

- Implement product behavior, not docs-only placeholders. No task is complete
  if it only adds labels, screenshots, mock controls, or unused contracts.
- Keep RawScope a local-first visual analytics instrument. Do not build a
  generic charting library, dashboard designer, SQL engine, dataframe engine,
  notebook replacement, cloud workflow, plugin framework, or ETL system.
- The plot rectangle is the single source of truth for WGPU viewport/scissor,
  axis/grid layout, pan normalization, zoom anchors, brush projection, hover
  hit-testing, point reveal, and overlays.
- GPU visuals must remain explainable in data coordinates. Do not introduce a
  perspective or oblique camera that breaks exact pointer-to-data mapping.
- Cells remain the exact reference presentation. Topographic and relief modes
  may interpolate or shade density, but must not change bin counts or imply
  unsupported precision.
- Large scatter views remain density-first. Individual markers appear only
  through bounded automatic point reveal or selected/pinned evidence.
- Filtering uses explicit AND semantics across predicates and deterministic
  category/range behavior. It must never silently reinterpret an empty
  selection, missing value, parse failure, or truncated category catalog.
- Filter masks align one-to-one with loaded visual row ids. GPU code must not
  infer row identity from a compacted or reordered buffer unless the mapping is
  explicit and tested.
- The full dataset remains the default comparison baseline. Difference density
  uses normalized cohort shares and a zero-centered diverging palette, not raw
  count subtraction between unequal cohort sizes.
- Advanced-mode compatibility is explicit: AbsoluteDensity supports Cells,
  Topographic, Relief, and automatic point reveal; FilteredDifference uses its
  dedicated signed field, disables Relief and point reveal, and restores the
  prior absolute presentation when the analyst returns to AbsoluteDensity.
- Hover inspection must not trigger GPU readback, source-table rescans, or
  allocations proportional to dataset size per pointer event.
- Interaction-time density updates must reuse resident GPU resources and avoid
  synchronous count-grid readback. Exact final summaries may be refreshed when
  the gesture settles.
- Pointer-move handlers must perform only constant-size viewport, scheduler,
  and uniform updates. They must not dispatch density compute, upload points,
  map GPU buffers, scan source rows, or rebuild CPU marginals.
- Interactive reprojection is an explicitly temporary analytical preview. It
  must map through data coordinates, leave uncovered regions unsmeared, expose
  a subtle refining state, and settle to one exact full-resolution field on
  gesture release.
- Preview density work must coalesce to the latest viewport revision, allow at
  most one dispatch per presented frame and one in-flight preview, and never
  overwrite a newer settled result.
- No benchmark or scale claim is allowed without the repository's named
  benchmark protocol, hardware, dataset, command, and date.
- Preserve the user's established horizontal/vertical pan semantics. The plot
  geometry task may correct normalization but must not casually reverse either
  axis.
- Hide full local paths from persistent chrome and the window title. A full
  path may remain available in a tooltip/details surface and reproducible
  evidence where current compatibility requires it.
- Use familiar icons for compact commands. The workbench may add `iconflow`
  with only the Lucide pack enabled if its Rust/egui integration is compatible;
  do not enable all icon packs or add a broad widget framework.
- Controls must expose meaningful analytical choices only: interaction mode,
  density presentation/transform, field projection, filters, comparison mode,
  and bounded relief parameters. Do not expose shader implementation knobs.
- Reduced-motion behavior is mandatory for visual transitions. Transient
  animation progress must not affect evidence or selection truth.
- `rawscope-core` remains dependency-free.
- Serde and evidence formatting remain in `rawscope-render`, not
  `rawscope-data`.
- Reusable data/filter logic belongs in `rawscope-data`; reusable render and
  visual-evidence logic belongs in `rawscope-render`; egui/winit coordination
  stays in `rawscope-workbench`.
- Split oversized owners before adding substantial behavior. Do not append this
  feature to the bottom of `app.rs`, `ui.rs`, `ui_view_context.rs`, or
  `app_report_bundle.rs`.
- Do not add `utils.rs`, `helpers.rs`, `common.rs`, `types.rs`, or another
  dumping-ground module.
- Preserve all existing uncommitted work. Every implementing session must read
  the current diff in files it owns and merge with that intent.
- Tests must prove real math, filter membership, GPU/CPU agreement, state
  transitions, evidence compatibility, or UI action projection. No startup
  tests, window-open tests, screenshot tests, fake demo apps, or broad unit-test
  churn.
- The external Lichess dataset is a manual product-acceptance input, not a CI
  fixture. The plan must work when it is absent.

## Task Dependency Table

| ID | Completed | Title | Description | Github Issue # | Blocked By | Task File |
|---|---|---|---|---|---|---|
| T001 | [x] | Add Authoritative Plot Geometry And GPU Scissoring | Introduce one physical plot rectangle shared by WGPU rendering, input math, brush projection, and egui overlays; split plot coordination out of oversized workbench owners. |  | None | [`tasks/T001.md`](tasks/T001.md) |
| T002 | [x] | Add Refined Axes Grid And Dataset Guides | Replace fixed guard-margin axes with nice numeric ticks, clipped grid lines, stable gutters, and truthful equality/zero guides derived from the active plot geometry. |  | T001 | [`tasks/T002.md`](tasks/T002.md) |
| T003 | [x] | Refine Workbench Chrome And Dataset Identity | Build a compact icon-led toolbar, collapsible inspector, short dataset identity, restrained status bar, and concise window title without hiding reproducibility details. |  | T001 | [`tasks/T003.md`](tasks/T003.md) |
| T004 | [x] | Add Explicit Pan Brush And Inspect Modes | Make interaction state explicit with Pan, Brush, and Inspect modes, cursor feedback, temporary modifier overrides, and plot-bounded event routing. |  | T001, T003 | [`tasks/T004.md`](tasks/T004.md) |
| T005 | [x] | Add Visual Field Catalog And Lichess Profile Hints | Build deterministic numeric/categorical field metadata from retained source rows and enrich the Lichess profile with optional filter and guide recommendations. |  | None | [`tasks/T005.md`](tasks/T005.md) |
| T006 | [x] | Add Deterministic Dataset Filter Contracts | Add CPU-owned numeric/category predicates, explicit missing-value behavior, AND composition, and row-aligned filter masks without adding a query language. |  | T005 | [`tasks/T006.md`](tasks/T006.md) |
| T007 | [ ] | Add Resident GPU Density Resources | Reuse uploaded points, pipelines, double-buffered count fields, and GPU-side maximum reduction while preserving explicit exact readback paths. |  | T001 | [`tasks/T007.md`](tasks/T007.md) |
| T017 | [ ] | Add Progressive GPU Pan Reprojection And Refinement | Reproject the last completed field immediately during viewport gestures, coalesce bounded preview rebins, and perform one exact full-resolution settle without pointer-time scans or readback. |  | T004, T007 | [`tasks/T017.md`](tasks/T017.md) |
| T008 | [ ] | Add Filter-Aware GPU Density And Filter Shelf | Upload revisioned row masks, skip excluded rows in WGSL, expose bounded filter controls, and keep visible counts/marginals/evidence aligned with the active cohort. |  | T004, T006, T007, T017 | [`tasks/T008.md`](tasks/T008.md) |
| T009 | [ ] | Add Density Inspection Lens And Pinned Evidence | Build a deterministic viewport inspection grid, hover highlight, concise density tooltip, and click-to-pin row/category evidence without pointer-time rescans or readback. |  | T002, T004, T008 | [`tasks/T009.md`](tasks/T009.md) |
| T010 | [ ] | Add Automatic Point Reveal Level Of Detail | Fade from density to bounded GPU point glyphs only when the visible cohort is sparse enough, preserving deterministic sampling and truthful rendered-count disclosure. |  | T008, T009 | [`tasks/T010.md`](tasks/T010.md) |
| T011 | [ ] | Add Mean Difference Scatter Projection | Add a general raw-XY versus mean/difference projection contract, correct axes/guides, retained row identity, and a Lichess rating-difference mode. |  | T002, T005, T008 | [`tasks/T011.md`](tasks/T011.md) |
| T012 | [ ] | Add Normalized Difference Density Mode | Render active-filter versus full-baseline share deltas through a zero-centered diverging GPU field with explicit normalization and bounded comparison controls. |  | T008 | [`tasks/T012.md`](tasks/T012.md) |
| T013 | [ ] | Add GPU Relief Field Presentation | Add a top-down, data-aligned relief mode using multiscale normals, bounded horizon shading, and contours while preserving exact plot hit-testing. |  | T002, T007 | [`tasks/T013.md`](tasks/T013.md) |
| T014 | [ ] | Add Refined Visual Transitions And Reduced Motion | Add bounded GPU interpolation for settled density/mode changes, immediate interaction feedback, and a reduced-motion path with no effect on evidence state. |  | T010, T012, T013, T017 | [`tasks/T014.md`](tasks/T014.md) |
| T015 | [ ] | Add Evidence V4 Visual Query Context | Add additive v4 scatter evidence for filters, projection, comparison, point reveal, inspection, and relief configuration while preserving v1/v2/v3 byte-level shapes. |  | T009, T010, T011, T012, T013 | [`tasks/T015.md`](tasks/T015.md) |
| T016 | [ ] | Curate The Flagship Lichess Visual Workbench | Wire profile-specific defaults and option ordering, verify the complete workflow on the external 200,000-game dataset, and remove remaining visual/interaction inconsistencies without adding hidden architecture. |  | T003, T004, T008, T009, T010, T011, T012, T013, T014, T015, T017 | [`tasks/T016.md`](tasks/T016.md) |

Task details live in separate files under `tasks/`, named by task ID.
The dependency waves, primary GPT-5.6 assignments, explicit reasoning levels,
fallback model floors, write-set ownership, and handoff prompts live in
[`implementation-plan.md`](implementation-plan.md).

## Final Notes

- Recommended critical path: T001 -> T003 -> T004 -> T007 -> T017 -> T008 ->
  T009 -> T010 -> T014 -> T015 -> T016.
- Safe early parallel path: T005 -> T006 can run beside T001 -> T003 when the
  sessions own disjoint files. T002 can run after T001 beside T005/T006. T007
  and T017 should be sequential and must not overlap another session editing
  scatter renderer/GPU or workbench frame-scheduling files.
- T011, T012, and T013 are independent mode branches after their blockers and
  can be implemented in separate sessions with disjoint modules, but all three
  must land before T014/T015.
- Human review is required for final axis density, icon dependency choice,
  filter option ordering, diverging palette, point-reveal threshold, relief
  defaults, and transition duration.
- The default flagship presentation should remain `TopographicField` until
  human review confirms that relief is equally truthful and readable.
- The full local path should disappear from persistent UI chrome, but existing
  evidence source identity must not be weakened accidentally.
- The 200,000-game file currently used for acceptance is external and
  unlicensed for redistribution in this repository. Do not commit it or add an
  automatic downloader.
- Risks: GPU resource reuse can regress correctness if buffers outlive dataset
  revisions; filter masks can desynchronize from row ids; point reveal can
  imply full-row rendering when sampled; difference density can lie if cohorts
  are not normalized; relief can overstate magnitude; v4 integration can bloat
  existing report owners unless split first.
- Manual adversarial review must verify that every visual mode changes real
  rendered output, every UI control reaches its owner, every filtered pixel can
  still reach row evidence, and no task substitutes a pretty label for data
  behavior.
