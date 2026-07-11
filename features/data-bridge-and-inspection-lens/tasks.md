# Data Bridge And Inspection Lens Tasks

## Discovery Summary

This bundle turns two related product ideas into one implementable feature:

1. make local CSV, Parquet, and Python dataframe data straightforward to open
   in RawScope without teaching every caller the workbench CLI; and
2. make an inspected density region explain itself through truthful metrics,
   GPU-native focus treatment, stable motion, pinning, and exportable evidence.

The tracks meet at the same product promise: a user can bring data to RawScope,
see a dense pattern, understand what the highlighted region means, and retain
the result as evidence.

Files and behavior inspected:

- `AGENTS.md`, `docs/AGENTS.md`, the repository Rust skills, and the global
  `large-feature-planning` skill.
- `research/rawscope-gpu-scale-visual-analytics.md`, especially its direction
  from pattern to explanation to row evidence, its Parquet/columnar guidance,
  and its warning against becoming an ETL or dataframe engine.
- `docs/ARCHITECTURE.md`, `docs/TECH_DECISIONS.md`, `docs/NON_GOALS.md`, and
  workspace/crate manifests.
- `rawscope-data` local CSV/Parquet loaders, loaded schema/source-row models,
  dataset profiles, visual field catalogs, filters, and dense ordinal `RowId`
  lookup.
- `rawscope-workbench` CLI parsing, startup binding resolution, dataset/profile
  coordination, render scheduling, inspection state, theme, tooltip, and report
  routing.
- `rawscope-render` inspection grid, difference-density math, brush/probe
  overlay and WGSL, visual transitions, evidence v1-v4, and serializer owners.
- The locked `egui 0.35.0` API for `Ui::set_opacity`,
  `Context::animate_value_with_time`, and bounded repaint scheduling.
- Existing feature-plan format and completed next-generation GPU tasks,
  particularly T009 density inspection and T015 evidence v4.

Current capabilities to extend rather than duplicate:

- CSV and Parquet already load through explicit scatter or timeline bindings.
- Profiles can validate a schema and supply bindings for Lichess.
- Source rows use dense ordinal internal `RowId` values, permitting O(1)
  retained-row lookup.
- The CLI already supports local input, comparison input, limits, profiles, and
  explicit field bindings.
- A settled 256x256 inspection grid already stores exact counts and up to 16
  deterministic row IDs per bin.
- Inspect mode already performs O(1) hover lookup, disables stale/refining
  caches, and supports click-to-pin.
- Difference mode already reports active and full-baseline bin shares.
- WGPU already draws a clipped probe rectangle after the density pass.
- Evidence v4 already records a bounded pinned inspection while preserving all
  earlier schemas.

The missing product contracts are specific:

- There is no stable, versioned launch/session artifact that another language
  can generate.
- There is no Python package or dataframe adapter.
- A natural source key cannot be declared and validated as evidence metadata.
- The current tooltip follows the pointer, can run off-screen, reports little
  context, and has no deliberate enter/exit behavior.
- Inspection does not report cohort share, occupied-cell density percentile,
  or bounded neighborhood context.
- The current probe is only a flat rectangle and disappears after pinning.
- Pinned category counts are computed from a bounded sample but are not labeled
  prominently enough as sample-derived.
- Evidence v4 cannot preserve the richer inspection/session context.

## Planning Invariants

- Every task must ship used product behavior. No task may complete with only
  schema prose, screenshots, mock controls, or an unused API.
- The bridge is local-first. It writes or references local files and launches a
  local native process; it does not upload datasets or start a network service.
- The first interoperability boundary is a versioned JSON session manifest plus
  CSV/Parquet data. JSON carries metadata, never the row corpus.
- Session v1 is startup intent, not arbitrary workspace persistence. It binds a
  dataset to a scatter or timeline view and optional evidence key; it does not
  serialize panel layout, hover, animation, GPU resources, or every UI setting.
- Python dataframe inputs are materialized as Parquet. Existing CSV/Parquet path
  inputs are referenced in place unless the caller explicitly chooses a bundle
  destination that copies them.
- The Python package is an adapter and launcher, not a dataframe engine. It may
  accept pandas, Polars, and PyArrow objects through narrow adapters but must not
  implement joins, expressions, query planning, transforms, or remote fetches.
- Internal `RowId` remains a dense zero-based ordinal aligned with points,
  masks, source rows, GPU buffers, and evidence samples. A user-declared
  evidence key is metadata and never replaces `RowId`.
- Evidence keys, when declared, must name an existing source column and contain
  no missing or duplicate values in the loaded cohort. Validation occurs once
  after load, never during hover.
- Relative dataset paths resolve from the manifest's directory. Absolute local
  paths are allowed. URI schemes and network fetch behavior are rejected.
- Existing direct CLI flags remain supported and retain their current meaning.
  `--session` is additive and mutually exclusive with direct input/binding
  flags.
- `rawscope-core` remains dependency-free.
- `rawscope-data` owns loaded schema/source tables and reusable evidence-key
  validation without serde.
- A narrow `rawscope-session` crate owns session JSON parsing, version checks,
  path resolution, and startup DTOs. It may use serde and depend on
  `rawscope-data`; it must not depend on WGPU, egui, winit, or the workbench.
- `rawscope-render` owns reusable inspection math, WGPU focus rendering, and all
  evidence serialization. It must not depend on Python or app UI.
- `rawscope-workbench` owns CLI/session coordination, egui placement/motion,
  frame scheduling, and projection into render/data contracts.
- Hover inspection uses only a current settled cache. It must not trigger GPU
  readback, source-table rescans, density dispatch, point uploads, or allocation
  proportional to dataset size.
- Exact-cell count and range are the primary tooltip facts. Neighborhood facts
  are explicitly labeled and use a fixed one-bin radius (at most 3x3 cells).
- Density percentile means tie-inclusive rank among occupied cells in the same
  settled active grid. Empty cells have no density percentile.
- Difference strength percentile means tie-inclusive rank of absolute share
  delta among cells with a non-zero active or baseline count. It is not a count
  percentile.
- Difference language always reflects `active_share - full_baseline_share`.
  Positive means more common in the active cohort; negative means less common.
- Sample-derived category summaries and source rows are always labeled with the
  exact sample size and total bin count.
- Hover and pin remain inspection concepts. They do not silently become brush
  selections or filters.
- The focus lens may add an exact-cell border, bounded neighborhood halo,
  crosshair, and axis-range cues, but it must not recolor density or imply finer
  precision than the inspected bin.
- Motion is short and functional: 110 ms enter/change, 80 ms exit, at most 4 px
  translation, no perpetual pulse, and no redraw once settled.
- Reduced motion makes inspection presentation immediate and requests no
  animation frames.
- Tooltip placement is anchored to the inspected bin, not raw pointer motion;
  it is clamped to the viewport and chooses a side that does not cover the bin.
- A pinned region remains visibly highlighted when the pointer leaves and is
  visually distinct from a transient hover.
- Evidence v5 is additive. v1, v2, v3, and v4 APIs, artifact kinds, and JSON
  shapes remain callable and unchanged.
- Transient hover, tooltip position, animation progress, and presentation alpha
  never enter evidence.
- Split existing owners before adding substantial bulk. In particular,
  `cli.rs`, `app_render_schedule.rs`, and the brush overlay must not become
  larger mixed-responsibility files.
- No `utils.rs`, `helpers.rs`, `common.rs`, giant `types.rs`, or speculative
  trait framework.
- Tests must prove parsing, path/binding semantics, real adapter output,
  cache-derived metrics, difference signs, overlay packing, motion scheduling,
  evidence compatibility, or app action projection. No startup smoke tests,
  window-open tests, screenshot tests, fake demo apps, or diagnostic scripts.
- The external Lichess datasets remain manual acceptance inputs and are never
  copied into the repository or test suite.

## Task Dependency Table

| ID | Completed | Title | Description | Github Issue # | Blocked By | Task File |
|---|---|---|---|---|---|---|
| T001 | [x] | Add Versioned Local Session Manifest Contract | Add a narrow serde-owning crate for session v1 parsing, strict validation, and manifest-relative local path resolution. |  | None | [`tasks/T001.md`](tasks/T001.md) |
| T002 | [x] | Add Dataset Evidence Key Validation | Validate an optional natural source key without changing dense internal row identity or adding serialization to `rawscope-data`. |  | None | [`tasks/T002.md`](tasks/T002.md) |
| T003 | [x] | Load Session Manifests In The Workbench | Add `--session`, map resolved session views into existing startup inputs, preserve direct CLI compatibility, and retain session context. |  | T001, T002 | [`tasks/T003.md`](tasks/T003.md) |
| T004 | [x] | Add Python File Bridge And Native Launcher | Create the local Python package, typed view/session API, exact Rust-compatible manifest writer, and executable resolution/launch behavior. |  | T001, T003 | [`tasks/T004.md`](tasks/T004.md) |
| T005 | [x] | Add Python Dataframe To Parquet Adapters | Accept pandas, Polars, and PyArrow tables, materialize typed Parquet bundles, validate columns/keys, and preserve bundle lifetime while RawScope runs. |  | T004 | [`tasks/T005.md`](tasks/T005.md) |
| T006 | [x] | Add Truthful Inspection Context Summaries | Enrich settled inspection grids with active share, occupied-cell percentile, and fixed-radius neighborhood metrics while preserving O(1) pointer lookup. |  | None | [`tasks/T006.md`](tasks/T006.md) |
| T007 | [x] | Add Difference Inspection Explanations | Derive signed active-versus-baseline meaning and difference-strength percentile from current settled grids without changing difference rendering math. |  | T006 | [`tasks/T007.md`](tasks/T007.md) |
| T008 | [x] | Add GPU Inspection Focus Lens | Split probe rendering from the brush overlay and add a clipped WGPU exact-cell focus, neighborhood halo, and crosshair driven only by constant-size uniforms. |  | T006 | [`tasks/T008.md`](tasks/T008.md) |
| T009 | [x] | Add Anchored Animated Inspection Tooltip | Replace pointer-following popup behavior with stable bin anchoring, edge-aware placement, bounded transitions, and reduced-motion-aware frame scheduling. |  | T006, T007, T008 | [`tasks/T009.md`](tasks/T009.md) |
| T010 | [x] | Refine Pinned Inspection And Persistent Focus | Keep pinned regions highlighted, clearly label sampled evidence, expose natural keys when present, and give the right rail a production-grade hierarchy. |  | T002, T003, T006, T007, T008, T009 | [`tasks/T010.md`](tasks/T010.md) |
| T011 | [x] | Add Evidence V5 Session And Inspection Context | Export additive v5 session provenance and rich pinned-inspection semantics while preserving v1-v4 byte-level artifact shapes. |  | T003, T006, T007, T010 | [`tasks/T011.md`](tasks/T011.md) |
| T012 | [ ] | Harden The Bridged Inspection Workflow | Integrate the Python-to-native and inspect-to-evidence paths, close real UI/performance inconsistencies, and validate both external Lichess scales without adding hidden architecture. |  | T004, T005, T009, T010, T011 | [`tasks/T012.md`](tasks/T012.md) |

Task details live in separate files under `tasks/`, named by task ID.

## Final Notes

- Recommended bridge path: T001 + T002 -> T003 -> T004 -> T005.
- Recommended inspection path: T006 -> T007 and T008 -> T009 -> T010.
- T001/T002 may run beside T006 because their write sets are disjoint. T004 may
  run beside T007/T008 after its blockers. Do not overlap T008, T009, or T010
  sessions because all touch focus-state/render coordination.
- T011 deliberately lands after both tracks so evidence can identify a session
  bundle and explain a pinned region in one additive schema.
- T012 is implementation hardening, not a docs-only finale. It may adjust real
  defaults, labels, spacing, launch handling, or frame scheduling found during
  native acceptance, then update user documentation to match shipped behavior.
- The v1 session manifest intentionally excludes initial filters and arbitrary
  visual styling. Those can be versioned later after the launch contract proves
  useful; adding them now would couple the SDK to volatile workbench controls.
- Live IPC, REST, sockets, notebook widgets, cloud URLs, package publication,
  automatic installers, and a graphical file-mapping wizard are deferred.
- Human review is required for tooltip density, tooltip side selection at plot
  edges, focus contrast in Cells/Topographic/Relief/Difference modes, reduced
  motion, pinned hierarchy, SDK ergonomics, and the 200,000-row and 343 MB
  Lichess workflows.
- Primary residual risks are Python/Rust schema drift, large dataframe
  materialization cost, stale session temp directories after abnormal process
  termination, percentile wording, and WGPU focus treatment competing with the
  density palette.
- Manual adversarial review must ensure that the SDK opens the exact prepared
  rows, the same bin drives tooltip/highlight/pin/evidence, every sampled claim
  is labeled, and no animation introduces permanent redraw or pointer-time
  analytical work.
