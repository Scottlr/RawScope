# Research Driven Visual Analytics Tasks

## Discovery Summary

This bundle turns the research synthesis in
`research/rawscope-gpu-scale-visual-analytics.md` into code-shaped RawScope work.
The planning goal is to make current density views more truthful, comparable,
multiscale, and explainable back to rows without expanding RawScope into a
dashboard, charting library, dataframe engine, SQL layer, ETL platform, or cloud
workflow.

Files inspected:

- `AGENTS.md` and `docs/AGENTS.md` for mandatory governance, product
  boundaries, testing policy, and final handoff requirements.
- `README.md`, `docs/VISION.md`, `docs/GOALS.md`, `docs/MVP.md`,
  `docs/ARCHITECTURE.md`, `docs/TECH_DECISIONS.md`, `docs/BENCHMARKS.md`, and
  `docs/NON_GOALS.md` for current product intent, milestone status, technical
  decisions, benchmark discipline, and non-goals.
- `research/rawscope-gpu-scale-visual-analytics.md` for the research takeaways:
  visible encoding semantics, transform controls, axes/lane labels, marginals,
  overview strips, selected-vs-baseline comparison, multiscale aggregates,
  stronger evidence, data-quality/diff workflows, and a verified flagship
  dataset path.
- Root `Cargo.toml` and crate manifests under `crates/rawscope-core`,
  `crates/rawscope-data`, `crates/rawscope-gpu`, `crates/rawscope-render`, and
  `apps/rawscope-workbench`.
- Thin facades: `crates/rawscope-core/src/lib.rs`,
  `crates/rawscope-data/src/lib.rs`, `crates/rawscope-render/src/lib.rs`,
  `crates/rawscope-gpu/src/lib.rs`, and `apps/rawscope-workbench/src/main.rs`.
- Core primitives: `crates/rawscope-core/src/density.rs` and
  `crates/rawscope-core/src/selection.rs`.
- Data owners: `crates/rawscope-data/src/dataset.rs`,
  `crates/rawscope-data/src/visual_record.rs`,
  `crates/rawscope-data/src/local_dataset.rs`,
  `crates/rawscope-data/src/local_dataset/csv.rs`, and
  `crates/rawscope-data/src/local_dataset/parquet.rs`.
- Render owners: `crates/rawscope-render/src/density_reference.rs`,
  `crates/rawscope-render/src/missingness_reference.rs`,
  `crates/rawscope-render/src/scatter_density_renderer.rs`,
  `crates/rawscope-render/src/timeline_density_renderer.rs`,
  `crates/rawscope-render/src/scatter_brush.rs`,
  `crates/rawscope-render/src/timeline_brush.rs`,
  `crates/rawscope-render/src/scatter_selection_evidence.rs`,
  `crates/rawscope-render/src/timeline_selection_evidence.rs`,
  `crates/rawscope-render/src/scatter_selection_export.rs`, and
  `crates/rawscope-render/src/timeline_selection_export.rs`.
- Workbench owners: `apps/rawscope-workbench/src/app.rs`,
  `apps/rawscope-workbench/src/app_selection.rs`,
  `apps/rawscope-workbench/src/app_missingness.rs`,
  `apps/rawscope-workbench/src/app_export.rs`,
  `apps/rawscope-workbench/src/app_report_bundle.rs`,
  `apps/rawscope-workbench/src/ui.rs`,
  `apps/rawscope-workbench/src/ui_controls.rs`, and the current uncommitted
  `apps/rawscope-workbench/src/ui_visual_encoding.rs` foothold.
- Existing tests and benches:
  `crates/rawscope-data/tests/local_dataset_import.rs`,
  `crates/rawscope-render/tests/density_reference.rs`,
  `crates/rawscope-render/tests/missingness_reference.rs`,
  `crates/rawscope-render/tests/selection_drilldown.rs`,
  `crates/rawscope-render/tests/scatter_selection_export.rs`,
  `crates/rawscope-render/tests/timeline_selection_export.rs`, and
  `apps/rawscope-workbench/src/app_report_bundle_tests.rs`.

Existing patterns:

- `rawscope-core` owns dependency-free foundational types such as row ids,
  ranges, density grids, and linked selection primitives.
- `rawscope-data` owns dataset identity, explicit local CSV/Parquet loading,
  chunk metadata, schema summaries, retained source rows, and visual records.
- `rawscope-render` owns CPU references, GPU density compute/render wrappers,
  viewport math, brush geometry, selected-region summaries, drilldown,
  evidence structs, JSON/Markdown formatting, and render-owned DTOs.
- `rawscope-workbench` owns app coordination, startup args, visible egui state,
  surface switching, selection publishing, missingness state, export routing,
  and local report-bundle writes.
- Evidence schemas are versioned; v1/v2 compatibility is preserved by adding
  new structs and formatters rather than mutating old artifact shapes in place.
- Normal tests are focused behavior tests. Local GPU correctness tests are
  ignored by default. Benchmarks exist but must not become product claims.

Important current risks:

- `apps/rawscope-workbench/src/ui.rs`,
  `crates/rawscope-render/src/scatter_selection_export.rs`, and
  `crates/rawscope-render/src/timeline_selection_export.rs` are already over
  the repo's 400-line review threshold. New work should split focused owners
  instead of adding more bulk to these files.
- GPU density paths are count-only. They do not preserve row ids or selection
  masks, so evidence work must remain CPU-backed or aggregate-cache-backed until
  a future GPU row-id design is explicitly approved.
- The current UI has a small visual-encoding foothold, but transform/palette
  semantics are not yet a render/evidence contract.
- Parquet chunk metadata exists, but RawScope does not yet own a multiscale
  aggregate cache or tile model.

## Planning Invariants

- Convert research into product behavior and code contracts, not a doc-only
  backlog.
- Do not implement production code as part of this planning bundle.
- Do not create smoke apps, fake demo apps, dry-run commands, or diagnostic-only
  tasks. Verification must be behavior-focused and tied to real code paths.
- Do not concentrate effort on documentation or broad unit-test churn. Docs are
  allowed only when a schema, public behavior, or evidence artifact changes.
  Tests should prove behavior that would fail if the feature were fake.
- Keep RawScope density-first for large scatter and event views. Raw markers are
  only for small selected-row drilldown, not the default large-data encoding.
- Make visual encoding explicit wherever density is shown: transform, palette,
  normalization scope, bin grid, and count range must be available to UI and
  evidence export.
- Linked comparison means a small set of coordinated evidence surfaces:
  overview/detail, selected-vs-baseline, and data-quality context. It must not
  become arbitrary dashboard/crossfilter layout.
- Multiscale work starts with CPU/reference aggregate metadata and cached
  summaries. It must not claim final GPU tile performance or full GPU row-id
  preservation.
- Selection evidence remains deterministic. Sampled rows must use stable row-id
  ordering or an explicitly documented deterministic reservoir policy.
- Dataset diff work must stay bounded to local, already-loaded, explicit
  datasets and schema summaries. Do not add general ETL, remote connectors,
  DataFusion, SQL, cloud upload, or dataframe execution.
- Parquet/Arrow changes must stay inside `rawscope-data` unless a later task
  explicitly designs a broader data layer. Do not leak Arrow types into
  `rawscope-core`.
- `rawscope-core` must remain dependency-free.
- `rawscope-data` must not gain serde just to support evidence JSON. Render-owned
  DTOs remain the serialization boundary.
- `rawscope-workbench` coordinates UI and app state; reusable summary,
  comparison, aggregate, and evidence logic belongs in `rawscope-render` or
  `rawscope-core` according to ownership.
- Do not make benchmark or scale claims. If a task touches performance-related
  code, acceptance criteria must verify correctness and cache reuse, not claim
  throughput.
- Existing uncommitted work in `research/` and `apps/rawscope-workbench/src/ui_visual_encoding.rs`
  is user/worktree context and must not be reverted by implementing agents.
- Files over roughly 400 lines require a split plan before adding substantial
  behavior.
- Feature tasks may update `docs/schemas/*` only when the serialized artifact
  shape changes; they must not create broad narrative docs as substitute work.

## Task Dependency Table

| ID | Completed | Title | Description | Github Issue # | Blocked By | Task File |
|---|---|---|---|---|---|---|
| T001 | [ ] | Add Density Encoding Contract And Controls | Promote density transform, palette, normalization, and count-range metadata into render-owned code, then expose explicit workbench controls for scatter/timeline density without rewriting the GPU compute path. |  | None | [`tasks/T001.md`](tasks/T001.md) |
| T002 | [ ] | Add Axis And Lane Context Overlays | Add render/workbench-owned axis tick, unit, and timeline lane-label overlay data so analysts can read ranges and lanes directly on the density surface. |  | T001 | [`tasks/T002.md`](tasks/T002.md) |
| T003 | [ ] | Add Marginal And Overview Context Summaries | Add CPU/reference marginal summaries for scatter/timeline and a timeline overview strip that shows full range versus current viewport. |  | T001, T002 | [`tasks/T003.md`](tasks/T003.md) |
| T004 | [ ] | Add Selection Baseline Comparison Summaries | Add selected-vs-baseline summary types for scatter, timeline, and missingness so brushes explain how a selected cohort differs from the current dataset baseline. |  | T001 | [`tasks/T004.md`](tasks/T004.md) |
| T005 | [ ] | Add Linked Comparison Workbench Panel | Use the existing linked-selection contract plus the comparison summaries to show a bounded comparison panel across the active primary view and data-quality context. |  | T003, T004 | [`tasks/T005.md`](tasks/T005.md) |
| T006 | [ ] | Add Aggregate Overview Cache With Row Samples | Add a CPU/reference aggregate cache that stores overview density counts plus deterministic per-bin row-id samples for scatter and timeline views. |  | T003 | [`tasks/T006.md`](tasks/T006.md) |
| T007 | [ ] | Add Evidence V3 Visual Context | Add v3 evidence artifacts that include visual encoding, baseline comparison, aggregate-cache context, and selected source-row evidence while preserving v1/v2 behavior. |  | T001, T004, T006 | [`tasks/T007.md`](tasks/T007.md) |
| T008 | [ ] | Add Bounded Dataset Diff Surface | Add a local-only dataset diff/data-quality surface for two compatible source tables, focused on row counts, schema presence, missingness deltas, and evidence row ids. |  | T004, T006 | [`tasks/T008.md`](tasks/T008.md) |
| T009 | [ ] | Add Flagship Dataset Profile Contract | Add a human-reviewed local dataset profile contract for one flagship dataset path, likely Lichess first, without adding downloaders, ETL, remote connectors, or public scale claims. |  | T007, T008 | [`tasks/T009.md`](tasks/T009.md) |

Task details live in separate files under `tasks/`, named by task ID.

## Final Notes

- Recommended implementation order: T001, T002, T003, T004, T005, T006, T007,
  T008, T009.
- Unresolved questions: whether density transform selection should be persisted
  globally or per active view; whether rank/equalized density should be CPU
  precomputed first or approximated in shader; how much row-id reservoir data is
  acceptable for large local files; which dataset profile should be approved
  first after license/current-size verification.
- Risks: transform controls can be faked as labels without changing renderer
  behavior; linked comparison can drift into a dashboard; aggregate caches can
  drift into a query engine; dataset diff can drift into ETL; evidence v3 can
  bloat export code that is already over the 400-line threshold.
- Areas that need human review before implementation: final palette choices,
  transform set for the first PR, reservoir sample size limits, evidence v3
  schema shape, dataset diff compatibility rules, and the first flagship dataset
  profile after external source/license verification.
- Manual adversarial review notes folded into this bundle: every task names
  source-of-truth modules; transform/palette UI must be connected to render
  config; comparison work must stay selected-vs-baseline; aggregate caches must
  preserve deterministic row-id samples without claiming GPU row preservation;
  dataset profiles must be local binding presets, not downloaders.
