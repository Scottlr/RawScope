# Next-Generation GPU Visual Analytics Implementation Plan

## Purpose

This file applies the `large-feature-implementation-planner` workflow to the
task bundle in [`tasks.md`](tasks.md). It provides dependency waves, write-set
ownership, preferred GPT-5.6 models, explicit reasoning effort, generally
available fallbacks, escalation rules, validation, and handoff prompts.

The schedule is planning only. Implementing agents must read the complete task
file they own before editing and must leave the workspace compiling at every
handoff.

## Model Guidance Verified On 2026-07-10

OpenAI's current public model catalog recommends GPT-5.5 for complex reasoning
and coding, GPT-5.4 for a more affordable high-capability option, and GPT-5.4
mini for faster lower-cost work. The GPT-5.6 family is currently a limited
preview in API and Codex for selected organizations:

- `gpt-5.6-sol`: flagship tier; preferred for ambiguous cross-crate GPU
  architecture, difficult WGSL, compatibility design, and final adversarial
  review. Use `max` reasoning for the hardest implementation slices. Use
  `ultra` only for a multi-agent architecture/review pass when the current
  Codex surface exposes it; do not assume `ultra` is a normal single-agent
  reasoning setting.
- `gpt-5.6-terra`: balanced tier; preferred for moderate Rust data contracts,
  UI/data integration, deterministic algorithms, and bounded cross-module
  tasks. Use the highest normally available reasoning effort when the task
  changes contracts.
- `gpt-5.6-luna`: fastest tier; preferred for bounded mechanical wiring,
  focused parser/UI updates after contracts exist, formatting, and focused
  tests. Do not assign Luna independent shader/API architecture decisions.

Official sources:

- [OpenAI model catalog](https://developers.openai.com/api/docs/models)
- [GPT-5.6 Sol, Terra, and Luna preview](https://openai.com/index/previewing-gpt-5-6-sol/)
- [GPT-5.6 preview IDs and availability](https://help.openai.com/en/articles/20001325-a-preview-of-gpt-5-6-sol-terra-and-luna)

GPT-5.6 is the primary route for this bundle whenever it is selectable in the
current Codex workspace. Every task below names one GPT-5.6 model and one exact
reasoning level first, then a generally available fallback and its reasoning
level. Use the fallback only when the named GPT-5.6 model is unavailable; do
not choose it merely to reduce cost or because the expected diff is small.

Reasoning labels in this plan are assignments, not suggestions:

- `medium`: bounded implementation with established contracts and one clear
  owner.
- `high`: moderate design judgment, deterministic algorithms, UI/data
  integration, or compatibility-sensitive state transitions.
- `max`: WGSL, GPU resource lifetime, cross-crate architecture, evidence
  compatibility, or final adversarial integration.

Do not silently lower reasoning. If the current Codex surface does not expose
the exact label, choose its nearest stronger setting and record that mapping in
the task handoff.

## General Model Rubric

| Work class | Primary GPT-5.6 model | Primary reasoning | Fallback model | Fallback reasoning |
|---|---|---|---|---|
| Mechanical module wiring, icon/button plumbing, focused tests | `gpt-5.6-luna` | `medium` | `gpt-5.3-codex-spark` | `medium` |
| Pure Rust catalogs, filter algorithms, deterministic projections | `gpt-5.6-terra` | `high` | `gpt-5.4-mini` | `high` |
| Cross-module workbench/render integration, interaction state, UI geometry | `gpt-5.6-terra` | `high` | `gpt-5.4` | `xhigh` |
| WGSL, resident GPU resources, multi-buffer modes, evidence compatibility | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` |
| Architecture recovery after repeated failures or final adversarial review | `gpt-5.6-sol` | `max` | `gpt-5.5` plus independent review | `xhigh` |

Do not choose a weaker model because a diff is short. One-line changes to
buffer layouts, shader bindings, evidence schemas, or coordinate systems retain
their stronger GPT-5.6 primary assignment and fallback floor.

## Dependency Waves

### Wave 1 - Independent Foundations

- T001 Authoritative Plot Geometry And GPU Scissoring
- T005 Visual Field Catalog And Lichess Profile Hints

These may run in parallel because T001 owns render/workbench plot files while
T005 owns `rawscope-data` profile/catalog files. Both sessions may need facade
edits, but in different crates.

### Wave 2 - Legibility, Chrome, Filters, And GPU Residency

- T002 after T001
- T003 after T001
- T006 after T005
- T007 after T001

T002 and T003 should be sequential if both need the same workbench shell file.
T006 is data-only and may run in parallel. T007 exclusively owns scatter GPU
resource files and must not overlap another shader session.

### Wave 3 - Interaction Performance And Filtered Density

- T004 after T001 and T003
- T017 after T004 and T007
- T008 after T004, T006, T007, and T017

Do not parallelize T004, T017, and T008. T017 consumes T004 gesture ownership
and T007 resident resources; T008 then extends the completed scheduler and GPU
bindings without restoring pointer-time recomputation.

### Wave 4 - Inspection

- T009 after T002, T004, and T008

This task establishes the truthful row-evidence bridge needed by point reveal.

### Wave 5 - Independent Advanced Modes

- T010 after T008 and T009
- T011 after T002, T005, and T008
- T012 after T008
- T013 after T002 and T007

These may run in separate sessions only when each owns a new focused module and
shader. Coordinate facade and `ScatterDensityPresentation` edits sequentially
or nominate one integration owner to avoid conflicting enum/binding changes.

### Wave 6 - Motion And Evidence

- T014 after T010, T012, T013, and T017
- T015 after T009, T010, T011, T012, and T013

T014 and T015 can run in parallel if T014 avoids evidence/report files and T015
does not edit transition shaders. Both must be complete before final curation.

### Wave 7 - Flagship Integration

- T016 after all listed blockers

T016 is not permission for hidden architecture. It may tune curated defaults,
option order, labels, and integration defects only. Any new contract discovered
here must be split into a new task or sent back to its owning task.

## Task Model Allocation

| Task | Primary model | Primary reasoning | Fallback model | Fallback reasoning | Why |
|---|---|---|---|---|---|
| T001 | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` | Shared coordinate contract spans WGPU viewport/scissor, egui layout, input, brushes, and oversized-file splits. |
| T002 | `gpt-5.6-terra` | `high` | `gpt-5.4` | `high` | Nice-tick math is bounded, but app/render geometry and visual truthfulness cross modules. |
| T003 | `gpt-5.6-terra` | `high` | `gpt-5.4` | `high` | UI hierarchy, icon dependency, file splits, and identity privacy need design judgment. |
| T004 | `gpt-5.6-terra` | `high` | `gpt-5.4` | `xhigh` | Gesture ownership and mode overrides can regress brush/pan behavior despite small code. |
| T005 | `gpt-5.6-terra` | `medium` | `gpt-5.4-mini` | `high` | Pure Rust catalog/profile work with deterministic cardinality rules. |
| T006 | `gpt-5.6-terra` | `high` | `gpt-5.4-mini` | `high` | Pure CPU filter contracts and row-mask correctness in one crate. |
| T007 | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` | Resident WGPU resources, double-buffered count fields, GPU max reduction, and readback policy are high regression risk. |
| T017 | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` | Data-space shader reprojection, frame scheduling, revision safety, exact settle, and cross-crate gesture wiring directly affect truthfulness and responsiveness. |
| T008 | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` | Filter masks cross data, workbench, GPU buffers, WGSL, summaries, and selection state. |
| T009 | `gpt-5.6-terra` | `high` | `gpt-5.4` | `xhigh` | Inspection must remain truthful and allocation-free at pointer time across UI/render/data. |
| T010 | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` | New instanced point pipeline, LOD sampling, hit testing, and disclosure require shader and evidence judgment. |
| T011 | `gpt-5.6-terra` | `high` | `gpt-5.4` | `xhigh` | Projection math is moderate but must preserve row identity, axes, filters, and evidence. |
| T012 | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` | Dual GPU count fields, normalized signed math, and diverging shader semantics are easy to implement misleadingly. |
| T013 | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` | Multiscale normal/shadow WGSL must look dimensional without breaking data alignment or count truth. |
| T014 | `gpt-5.6-terra` | `high` | `gpt-5.4` | `xhigh` | Transition state is bounded but crosses render resources, timing, and reduced-motion behavior. |
| T015 | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` | Additive v4 evidence touches serialization compatibility and several new visual contracts. |
| T016 | `gpt-5.6-sol` | `max` | `gpt-5.5` | `xhigh` | Final visual integration requires repo-wide context and real native GPU inspection; use Luna only for isolated profile/parser fixes. |

## Routing And Escalation Rules

- Start every task on its listed GPT-5.6 primary model when that model is
  selectable. The fallback is an availability path, not the preferred route.
- Escalate a Terra task to GPT-5.6 Sol `max` after two non-trivial compile/test
  failures, a discovered shader-binding mismatch, uncertainty about row-id
  alignment, or unresolved cross-crate ownership.
- Escalate any fallback task one tier after two non-trivial compile/test failures, a
  discovered shader-binding mismatch, uncertainty about row-id alignment, or a
  conflict with uncommitted work in the same owner.
- Use GPT-5.6 Sol `ultra` only for a read-only architecture/adversarial review
  of T001/T007/T012/T013/T015/T017 or of the complete bundle. It must not
  create overlapping writes with implementation sessions.
- Split mechanical CLI/profile wiring from T016 and give it to GPT-5.6 Luna
  `medium` only after the integration owner has specified exact fields and
  behavior. Use Spark `medium` only when Luna is unavailable.
- Never use model escalation as a substitute for splitting a task whose write
  set or acceptance criteria are too broad.

## Shared Implementation Prompt

Use this prefix for every implementation session, followed by the complete
task-specific file:

```text
You are implementing one task from:
features/next-generation-gpu-visual-analytics/tasks.md

Read AGENTS.md, docs/AGENTS.md, and
.codex/skills/agent-preflight-and-handoff/SKILL.md first. Read the assigned
T### file fully before editing. Inspect root and touched crate manifests,
facades, nearby owners, current tests, and git diff. Do not revert existing
uncommitted work.

Implement real product behavior. Do not add smoke tests, startup/window tests,
screenshot tests, dry runs, diagnostic utilities, fake apps, or docs-only
substitutes. Preserve evidence v1/v2/v3 compatibility, keep rawscope-core
dependency-free, keep serde/evidence in rawscope-render, and keep reusable logic
out of the workbench.

Leave the task commit-worthy and the workspace compiling. Run cargo fmt,
task-focused behavior tests, and cargo check --workspace. End with files
changed, architecture decisions, checks run, checks skipped with reasons, and
remaining risks.
```

## Per-Task Work Package Summary

Each implementing session must use the exact `Related Code`, `Implementation
Touchpoints`, and `Testing Guidance` from its task file. The minimal ownership
summary is:

- T001: `plot_geometry`, plot-surface UI, app plot interaction, renderer
  viewport/scissor, brush projection.
- T002: `view_axes`, plot-axis UI, grid/reference guides.
- T003: shell/theme/dataset display identity and optional Lucide-only icon
  dependency.
- T004: interaction mode state and winit/egui routing.
- T005: `rawscope-data` field catalog and dataset profile hints.
- T006: `rawscope-data` filter predicates/masks.
- T007: scatter GPU resource lifetime, double buffering, shader max reduction.
- T017: density reprojection math/shader, frame scheduler, exact-settle app path.
- T008: filter-mask WGSL/buffers and workbench filter shelf.
- T009: inspection grid, probe overlay, pinned inspector.
- T010: point reveal renderer/shader/LOD state.
- T011: scatter projection math and projection controls.
- T012: signed difference-density compute/presentation and controls.
- T013: relief-field shader/config and controls.
- T014: transition state/buffers and reduced-motion option.
- T015: v4 evidence/export/report modules and compatibility tests.
- T016: profile defaults, option curation, real Lichess acceptance fixes.

## Executable Work Packages

### T001 Work Package

```text
Task: T001 Add Authoritative Plot Geometry And GPU Scissoring
Primary model: gpt-5.6-sol
Primary reasoning: max
Fallback model: gpt-5.5
Fallback reasoning: xhigh
Dependencies satisfied: yes; no blockers
Owned files/modules: rawscope-render plot geometry and renderer viewport/scissor;
  workbench plot surface, plot interaction, render/event/brush integration
Read first: T001.md plus app.rs, app_events.rs, app_render.rs,
  ui_controls.rs, ui_view_context.rs, brush modules, renderer modules
Implement:
- add checked PlotRectPx and logical/physical plot output
- use one rectangle for WGPU, input, brush, and overlay conversion
- split plot interaction out of app.rs without reversing pan semantics
Do not:
- redesign axes or UI theme
- touch filters, evidence, or GPU resource residency
Validation:
- cargo test -p rawscope-render plot_geometry
- cargo test -p rawscope-render scatter_brush timeline_brush
- cargo test -p rawscope-workbench ui_plot_surface
- cargo fmt --all --check
- cargo check --workspace
```

### T002 Work Package

```text
Task: T002 Add Refined Axes Grid And Dataset Guides
Primary model: gpt-5.6-terra
Primary reasoning: high
Fallback model: gpt-5.4
Fallback reasoning: high
Dependencies satisfied: only after T001
Owned files/modules: rawscope-render view_axes; workbench ui_plot_axes and plot gutters
Read first: T002.md, T001 handoff, view_axes.rs, ui_view_context.rs,
  ui_plot_surface.rs
Implement:
- add deterministic nice ticks and formatting options with old API wrapper
- reserve gutters and draw clipped grid/reference guides
- remove fixed axis guard constants
Do not:
- add a charting library or calendar/date system
- touch shaders, filters, or evidence
Validation:
- cargo test -p rawscope-render view_axes
- cargo test -p rawscope-workbench ui_plot_axes
- cargo fmt --all --check
- cargo check --workspace
```

### T003 Work Package

```text
Task: T003 Refine Workbench Chrome And Dataset Identity
Primary model: gpt-5.6-terra
Primary reasoning: high
Fallback model: gpt-5.4
Fallback reasoning: high
Dependencies satisfied: only after T001
Owned files/modules: workbench ui_shell, ui_dataset_identity, ui_theme,
  app window title, optional Lucide-only icon dependency
Read first: T003.md, T001 handoff, ui.rs, ui_controls.rs, ui_theme.rs,
  app_window.rs, workbench Cargo.toml
Implement:
- split shell/identity from oversized owners
- hide persistent full paths while keeping details/copy/evidence access
- add compact commands and collapsible inspector
Do not:
- add docking, cards everywhere, a UI framework, filters, or interaction modes
Validation:
- cargo test -p rawscope-workbench ui_dataset_identity
- cargo test -p rawscope-workbench ui_shell ui
- cargo fmt --all --check
- cargo check --workspace
```

### T004 Work Package

```text
Task: T004 Add Explicit Pan Brush And Inspect Modes
Primary model: gpt-5.6-terra
Primary reasoning: high
Fallback model: gpt-5.4
Fallback reasoning: xhigh
Dependencies satisfied: only after T001 and T003
Owned files/modules: workbench app_interaction_mode, app_events, brush routing,
  shell mode controls and cursor/status projection
Read first: T004.md plus T001/T003 handoffs, app_events.rs, brush modules,
  plot interaction and shell actions
Implement:
- add one persistent mode and one active gesture source of truth
- preserve modifier overrides and egui-consumed active releases
- show exact data coordinates in coordinate-only Inspect mode
Do not:
- add density inspection rows/counts or filters
- modify shaders/evidence
Validation:
- cargo test -p rawscope-workbench app_interaction_mode
- cargo test -p rawscope-workbench app_brush app_timeline_brush ui_shell
- cargo fmt --all --check
- cargo check --workspace
```

### T005 Work Package

```text
Task: T005 Add Visual Field Catalog And Lichess Profile Hints
Primary model: gpt-5.6-terra
Primary reasoning: medium
Fallback model: gpt-5.4-mini
Fallback reasoning: high
Dependencies satisfied: yes; no blockers
Owned files/modules: rawscope-data visual_field_catalog, dataset_profile, lib facade,
  focused data tests
Read first: T005.md, local_dataset.rs, dataset_profile.rs, visual_record.rs,
  dataset_profile tests
Implement:
- build deterministic numeric/categorical catalog with explicit truncation
- add optional profile filter/guide/projection hints
- keep existing required-column validation unchanged
Do not:
- add filtering, serde, UI, downloads, or semantic name inference
Validation:
- cargo test -p rawscope-data visual_field_catalog
- cargo test -p rawscope-data --test dataset_profile
- cargo fmt --all --check
- cargo check --workspace
```

### T006 Work Package

```text
Task: T006 Add Deterministic Dataset Filter Contracts
Primary model: gpt-5.6-terra
Primary reasoning: high
Fallback model: gpt-5.4-mini
Fallback reasoning: high
Dependencies satisfied: only after T005
Owned files/modules: rawscope-data dataset_filter and lib facade/tests
Read first: T006.md, T005 handoff, source-table/catalog contracts
Implement:
- add validated numeric/category predicates and AND-only FilterSet
- evaluate a contiguous u32 row-id-aligned mask
- implement stable revisions and explicit missing/invalid behavior
Do not:
- add a parser, OR groups, UI, GPU, serde, or query language
Validation:
- cargo test -p rawscope-data dataset_filter
- cargo test -p rawscope-data
- cargo fmt --all --check
- cargo check --workspace
```

### T007 Work Package

```text
Task: T007 Add Resident GPU Density Resources
Primary model: gpt-5.6-sol
Primary reasoning: max
Fallback model: gpt-5.5
Fallback reasoning: xhigh
Dependencies satisfied: only after T001
Owned files/modules: scatter GPU state/pipeline/helpers/WGSL and scatter renderer
Read first: T007.md, T001 handoff, gpu_scatter_density.rs,
  scatter_density_renderer.rs, GPU helpers/shaders/tests
Implement:
- keep point/double-count/max/pipeline resources dataset-resident
- compute max count on GPU and expose explicit no/max/full readback policy
- preserve a completed renderable field while its replacement computes
Do not:
- add gesture scheduling, reprojection, filters, point reveal, difference,
  relief, a thread, or a benchmark claim
Validation:
- cargo test -p rawscope-render scatter_density_renderer
- cargo test -p rawscope-render --test density_reference
- cargo test -p rawscope-render --test gpu_scatter_density -- --ignored when available
- cargo fmt --all --check
- cargo check --workspace
```

### T017 Work Package

```text
Task: T017 Add Progressive GPU Pan Reprojection And Refinement
Primary model: gpt-5.6-sol
Primary reasoning: max
Fallback model: gpt-5.5
Fallback reasoning: xhigh
Dependencies satisfied: only after T004 and T007
Owned files/modules: rawscope-render density reprojection contract/render params/WGSL;
  workbench app_render_schedule plus pan/zoom frame and settled-summary wiring
Read first: T017.md, T001/T004/T007 handoffs, app pan/event/render paths,
  scatter_viewport, scatter renderer/shader, WGPU completion APIs
Implement:
- make cursor movement update viewport, revision, uniforms, and redraw only
- reproject the last completed field in data space during the gesture
- coalesce bounded 128x128 previews and settle once to exact 256x256 on release
- refresh CPU marginals/inspection/stats only for the settled exact revision
Do not:
- reverse pan signs, smear uncovered bins, reduce final resolution, add an FPS
  panel/benchmark claim, or use wall-clock sleeps in tests
Validation:
- cargo test -p rawscope-render scatter_density_reprojection
- cargo test -p rawscope-render scatter_density_renderer
- cargo test -p rawscope-workbench app_render_schedule
- cargo test -p rawscope-workbench app_interaction_mode
- cargo test -p rawscope-render --test density_reference
- cargo test -p rawscope-render --test gpu_scatter_density -- --ignored when available
- cargo fmt --all --check
- cargo check --workspace
- manually pan/zoom the external 200,000-row Lichess dataset and record adapter/result
```

### T008 Work Package

```text
Task: T008 Add Filter-Aware GPU Density And Filter Shelf
Primary model: gpt-5.6-sol
Primary reasoning: max
Fallback model: gpt-5.5
Fallback reasoning: xhigh
Dependencies satisfied: only after T004, T006, T007, and T017
Owned files/modules: scatter filter app/UI, row-mask GPU binding/WGSL,
  masked summaries/selection/comparison integration
Read first: T008.md and all blocker handoffs, filter contracts, T017 scheduler,
  shader bindings, brush/comparison/summary owners
Implement:
- upload mask only on revision changes and filter before binning
- keep density/marginals/selection on one cohort
- add bounded profile-ordered filter shelf and temporary pre-v4 export gate
Do not:
- compact/reorder points, add query language, difference mode, or v4 serializer
Validation:
- cargo test -p rawscope-data dataset_filter
- cargo test -p rawscope-render masked
- cargo test -p rawscope-workbench app_scatter_filter ui_filters
- cargo test -p rawscope-render --test gpu_scatter_density -- --ignored when available
- cargo fmt --all --check
- cargo check --workspace
```

### T009 Work Package

```text
Task: T009 Add Density Inspection Lens And Pinned Evidence
Primary model: gpt-5.6-terra
Primary reasoning: high
Fallback model: gpt-5.4
Fallback reasoning: xhigh
Dependencies satisfied: only after T002, T004, and T008
Owned files/modules: rawscope-render scatter_inspection/probe overlay;
  workbench inspection app/UI state
Read first: T009.md, blocker handoffs, aggregate cache, brush overlay,
  source-row lookup and right-rail owners
Implement:
- build a density-grid-aligned CPU inspection cache at settled revisions
- provide O(1) hover hit and exact count/bounded row sample disclosure
- pin bounded sampled evidence without publishing a brush selection
Do not:
- read GPU/source table per pointer, draw points, or serialize evidence
Validation:
- cargo test -p rawscope-render scatter_inspection
- cargo test -p rawscope-workbench app_scatter_inspection ui_scatter_inspection
- cargo fmt --all --check
- cargo check --workspace
```

### T010 Work Package

```text
Task: T010 Add Automatic Point Reveal Level Of Detail
Primary model: gpt-5.6-sol
Primary reasoning: max
Fallback model: gpt-5.5
Fallback reasoning: xhigh
Dependencies satisfied: only after T008 and T009
Owned files/modules: scatter_point_reveal, point renderer/WGSL,
  point LOD app state and encoding controls
Read first: T010.md, blocker handoffs, resident point packing, plot transform,
  inspection hit tests and visual controls
Implement:
- select masked/in-view points by rows-per-pixel policy
- use exact SplitMix64 row priority when sampling over budget
- draw one instanced GPU disc pipeline and disclose eligible/rendered/sample state
Do not:
- render all dense rows, create egui point widgets, or add category mapping
Validation:
- cargo test -p rawscope-render scatter_point_reveal
- cargo test -p rawscope-render scatter_point_renderer
- cargo test -p rawscope-workbench ui_visual_encoding
- cargo fmt --all --check
- cargo check --workspace
```

### T011 Work Package

```text
Task: T011 Add Mean Difference Scatter Projection
Primary model: gpt-5.6-terra
Primary reasoning: high
Fallback model: gpt-5.4
Fallback reasoning: xhigh
Dependencies satisfied: only after T002, T005, and T008
Owned files/modules: rawscope-data scatter_projection; workbench projection state/UI;
  axis guide integration
Read first: T011.md, blocker handoffs, loaded scatter/point/range code,
  selection invalidation and axes
Implement:
- add RawXY and exact (x+y)/2, x-y projection
- preserve row ids/order/filter alignment
- reset data-space selection/pin and use equality/zero guides correctly
Do not:
- add regression/PCA/formulas, mutate source bindings, or serialize v4
Validation:
- cargo test -p rawscope-data scatter_projection
- cargo test -p rawscope-workbench app_scatter_projection ui_visual_encoding
- cargo test -p rawscope-render view_axes
- cargo fmt --all --check
- cargo check --workspace
```

### T012 Work Package

```text
Task: T012 Add Normalized Difference Density Mode
Primary model: gpt-5.6-sol
Primary reasoning: max
Fallback model: gpt-5.5
Fallback reasoning: xhigh
Dependencies satisfied: only after T008
Owned files/modules: difference_density CPU reference, dual GPU buffers/reduction/WGSL,
  difference mode UI and inspection comparison
Read first: T012.md, T007/T008 handoffs, density encoding/shaders,
  selection comparison and inspection contracts
Implement:
- compute active_share - full_baseline_share per bin
- reduce max absolute through fixed-point u32 GPU atomic max
- render symmetric diverging field and disable relief/points while active
Do not:
- subtract raw counts, add arbitrary cohorts/significance, or alter filters
Validation:
- cargo test -p rawscope-render difference_density
- cargo test -p rawscope-workbench ui_visual_encoding app_scatter_filter
- cargo test -p rawscope-render --test gpu_scatter_density -- --ignored when available
- cargo fmt --all --check
- cargo check --workspace
```

### T013 Work Package

```text
Task: T013 Add GPU Relief Field Presentation
Primary model: gpt-5.6-sol
Primary reasoning: max
Fallback model: gpt-5.5
Fallback reasoning: xhigh
Dependencies satisfied: only after T002 and T007
Owned files/modules: scatter_relief config/tests, scatter density presentation,
  relief render WGSL/params and UI controls
Read first: T013.md, blocker handoffs, current topographic shader,
  render params/alignment and encoding controls
Implement:
- add validated ReliefField config and enum variant
- shade the same scalar density with multiscale normals and bounded horizon samples
- preserve top-down UV/hit mapping and disable Relief in difference mode
Do not:
- add perspective/free camera/PBR, change counts, or expose shader internals
Validation:
- cargo test -p rawscope-render scatter_relief
- cargo test -p rawscope-render scatter_density_presentation scatter_density_renderer
- cargo test -p rawscope-workbench ui_visual_encoding
- cargo fmt --all --check
- cargo check --workspace
```

### T014 Work Package

```text
Task: T014 Add Refined Visual Transitions And Reduced Motion
Primary model: gpt-5.6-terra
Primary reasoning: high
Fallback model: gpt-5.4
Fallback reasoning: xhigh
Dependencies satisfied: only after T010, T012, T013, and T017
Owned files/modules: visual_transition math; workbench transition lifecycle;
  previous/current render resource blending
Read first: T014.md and blocker handoffs, T017 render scheduler, app_render.rs,
  visual mode actions and resource ownership
Implement:
- add <=500 ms validated progress/easing and 180 ms default
- crossfade settled fields/colors while keeping interaction immediate
- add reduced-motion toggle and stop redraw after completion
Do not:
- animate pointer moves, interpolate incompatible numeric units, or run idle motion
Validation:
- cargo test -p rawscope-render visual_transition
- cargo test -p rawscope-workbench app_visual_transition app_render_schedule
- cargo fmt --all --check
- cargo check --workspace
```

### T015 Work Package

```text
Task: T015 Add Evidence V4 Visual Query Context
Primary model: gpt-5.6-sol
Primary reasoning: max
Fallback model: gpt-5.5
Fallback reasoning: xhigh
Dependencies satisfied: only after T009, T010, T011, T012, and T013
Owned files/modules: scatter evidence/export v4, report bundle v4, schema v4,
  default scatter export routing and compatibility tests
Read first: T015.md, all legacy evidence/export/report modules/tests,
  blocker contracts and schema docs
Implement:
- add explicit additive v4 query/cohort/sample/inspection fields
- map filter/projection/mode configs into render-owned serde DTOs
- make v4 default while preserving legacy outputs and APIs unchanged
Do not:
- mutate v1/v2/v3, add serde to data, or serialize transient animation/hover state
Validation:
- cargo test -p rawscope-render --test scatter_selection_export
- cargo test -p rawscope-render --test selection_evidence_v3
- cargo test -p rawscope-render selection_evidence_v4
- cargo test -p rawscope-workbench app_report_bundle
- cargo fmt --all --check
- cargo check --workspace
```

### T016 Work Package

```text
Task: T016 Curate The Flagship Lichess Visual Workbench
Primary model: gpt-5.6-sol
Primary reasoning: max
Fallback model: gpt-5.5
Fallback reasoning: xhigh
Dependencies satisfied: only after every listed blocker in T016.md
Owned files/modules: Lichess profile defaults, final UI option order/default mapping,
  narrow integration fixes and real-data acceptance handoff
Read first: T016.md, all blocker handoffs, final profile/UI/render/evidence state,
  T017 performance acceptance, external dataset header, and current diff
Implement:
- curate conservative defaults and normal/advanced option hierarchy
- run every real-data acceptance workflow on 200,000 rows
- confirm repeated pan/zoom remains immediate and settles to exact current data
- fix only narrow integration defects; split any new architecture into a task
Do not:
- commit/download data, add a mode/contract, make performance claims, or hide unfinished work
Validation:
- cargo test -p rawscope-data --test dataset_profile
- cargo test -p rawscope-render
- cargo test -p rawscope-workbench
- cargo fmt --all --check
- cargo check --workspace
- run the exact external Lichess command and record adapter/dataset/result
```

Every package ends with the shared final handoff: files changed, architectural
decisions, tests/checks run, tests not run with reasons, and remaining risks.

## Validation Contract

Every task runs:

```powershell
cargo fmt --all --check
cargo check --workspace
```

Every task also runs the focused package tests named in its task file. Shader
tasks add CPU-reference or pure contract tests and extend existing ignored GPU
correctness tests where hardware is required. They do not add window or
screenshot tests. T017 and T016 perform manual native runs against the external
200,000-row profile file when available and record adapter/dataset/command in
the handoff without making a public performance claim.

## Final Review

After T016, run one read-only adversarial review using GPT-5.6 Sol `max` or
`ultra` when provisioned, otherwise GPT-5.5 xhigh. The reviewer must check:

- plot geometry is authoritative everywhere;
- pointer movement performs no density dispatch, GPU readback, or CPU row scan;
- interactive reprojection is data-aligned, leaves uncovered regions unsmeared,
  and settles once to an exact full-resolution field;
- no persistent UI path leakage remains;
- every mode changes real GPU output and exposes its encoding;
- filters, inspection, point reveal, comparison, and evidence use the same row
  cohort;
- point sampling and difference normalization are disclosed truthfully;
- relief preserves one-to-one data-space hit testing;
- reduced motion bypasses transitions;
- v1/v2/v3 artifacts remain unchanged;
- oversized owners were split instead of expanded;
- no new generic dashboard/query framework entered the repository.
