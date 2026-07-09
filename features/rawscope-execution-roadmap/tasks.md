# RawScope Execution Roadmap Tasks

## Discovery Summary

Assumed product goal: RawScope is a local-first GPU-scale visual analytics engine for large raw datasets. Its job is to show full-dataset shape before the user knows the query, notebook analysis, dashboard, or model they need, then preserve a credible path from visible pixels back to row-level evidence.

Assumed north-star workflow: open a local dataset, see scatter/timeline/data-quality shape quickly, brush an interesting region, inspect exact or sampled contributing rows, compare that selection across linked views, and export a reproducible evidence bundle with dataset identity, view configuration, summaries, rows, and visual context.

What RawScope should not become: a generic charting library, BI/dashboard builder, SQL/database engine, dataframe engine, notebook replacement, cloud analytics platform, plugin framework, or general UI framework. Those boundaries are explicit in `docs/AGENTS.md`, `docs/NON_GOALS.md`, and `README.md`.

Files inspected:
- `AGENTS.md` and `docs/AGENTS.md` for governance, product boundary, and sequencing constraints.
- `README.md`, `docs/VISION.md`, `docs/GOALS.md`, `docs/MVP.md`, `docs/ARCHITECTURE.md`, `docs/TECH_DECISIONS.md`, and `docs/NON_GOALS.md` for current product intent and milestone status.
- Root `Cargo.toml` plus crate manifests under `crates/rawscope-core`, `crates/rawscope-data`, `crates/rawscope-gpu`, `crates/rawscope-render`, and `apps/rawscope-workbench`.
- Thin facades: `crates/rawscope-core/src/lib.rs`, `crates/rawscope-data/src/lib.rs`, `crates/rawscope-gpu/src/lib.rs`, `crates/rawscope-render/src/lib.rs`, and `apps/rawscope-workbench/src/main.rs`.
- Data/model paths: `crates/rawscope-data/src/dataset.rs`, `crates/rawscope-data/src/local_dataset.rs`, `crates/rawscope-data/src/local_dataset/csv.rs`, `crates/rawscope-data/src/synthetic/point.rs`, and `crates/rawscope-data/src/synthetic/event.rs`.
- Selection/evidence/render paths: `crates/rawscope-render/src/scatter_brush.rs`, `crates/rawscope-render/src/timeline_brush.rs`, `crates/rawscope-render/src/scatter_selection_evidence.rs`, `crates/rawscope-render/src/timeline_selection_evidence.rs`, `crates/rawscope-render/src/scatter_selection_export.rs`, `crates/rawscope-render/src/timeline_selection_export.rs`, `crates/rawscope-render/src/scatter_viewport.rs`, `crates/rawscope-render/src/timeline_viewport.rs`, `crates/rawscope-render/src/gpu_scatter_density.rs`, `crates/rawscope-render/src/gpu_timeline_density.rs`, `crates/rawscope-render/src/scatter_density_renderer.rs`, and `crates/rawscope-render/src/timeline_density_renderer.rs`.
- Workbench paths: `apps/rawscope-workbench/src/app.rs`, `apps/rawscope-workbench/src/app_brush.rs`, `apps/rawscope-workbench/src/app_timeline_brush.rs`, `apps/rawscope-workbench/src/app_timeline.rs`, `apps/rawscope-workbench/src/app_events.rs`, `apps/rawscope-workbench/src/app_render.rs`, `apps/rawscope-workbench/src/app_export.rs`, `apps/rawscope-workbench/src/app_export_files.rs`, `apps/rawscope-workbench/src/cli.rs`, and `apps/rawscope-workbench/src/demo.rs`.
- Schema/test paths: `docs/schemas/scatter-selection-evidence-v1.md`, `docs/schemas/timeline-selection-evidence-v1.md`, `crates/rawscope-data/tests/local_dataset_import.rs`, `crates/rawscope-render/tests/density_reference.rs`, `crates/rawscope-render/tests/scatter_selection_export.rs`, and `crates/rawscope-render/tests/timeline_selection_export.rs`.

Current implementation baseline:
- The workspace has five members: `rawscope-core`, `rawscope-data`, `rawscope-gpu`, `rawscope-render`, and `rawscope-workbench`.
- `rawscope-core` owns foundational range, row-id, selection-id, and density-grid types with no dependencies.
- `rawscope-data` owns deterministic synthetic point/event datasets and a first CSV loader for scatter/timeline column bindings.
- `rawscope-gpu` owns WGPU context/bootstrap, headless compute context, adapter metadata, and surface resize behavior.
- `rawscope-render` owns CPU density references, WGPU scatter/timeline density compute, simple renderers, viewport math, data-anchored brushing, CPU-side summaries/evidence, and JSON/Markdown evidence formatting.
- `rawscope-workbench` owns the current `winit` app shell, startup CLI, active scatter/timeline state, input handling, title-bar summaries, brush/export routing, and file writes.
- Tests cover deterministic synthetic data, CSV loader behavior, CPU reference density, ignored local GPU correctness, viewport math, brush projection, selection evidence, and export formatting.

Main audit findings:
- The product direction is coherent: density-first visual exploration with row evidence, local-first privacy, and careful performance claims.
- The repository has already passed the stale `docs/AGENTS.md` "Current Next Task" note. The real baseline is closer to post-Milestone 5C: synthetic and local CSV scatter/timeline density, brushing, evidence export, manifests, and a first CPU-backed missingness slice.
- The largest model mismatch is synthetic naming leaking into local data. CSV rows are mapped into `SyntheticPointRecord`, `SyntheticEventRecord`, and `SyntheticDatasetMetadata::new(0, row_count)`, so evidence still reads like synthetic proof even for local files.
- Evidence v1 is useful but synthetic-scoped. It omits external dataset source, dataset fingerprint, column bindings, lane labels, selected original row values, view configuration, and visual context.
- The workbench now has a small egui shell with visible controls, axis labels, export status, and selected-row drilldown, but it remains a correctness-first single-view tool without file dialogs, linked views, screenshot capture, or broader report polish.
- `apps/rawscope-workbench/src/app.rs` is at 405 lines, slightly over the repository's 400-line review threshold. The app has already been split into domain helper files, but central state can become a coordination bottleneck.
- GPU density currently returns count buffers, not row-id-preserving bins or selection masks. That is correct for the current slice but should remain explicit until a row-evidence design exists.
- The next execution arc should move from visual proof to evidence-centric local workflow before adding Parquet/Arrow, performance claims, DataFusion, Tauri, web, cloud, or plugins.

Existing owner and pattern checks:
- Data ownership should extend `crates/rawscope-data/src/dataset.rs`, `local_dataset.rs`, `local_dataset/csv.rs`, and synthetic modules before creating a new generic data bucket.
- Shared non-UI primitives should extend `rawscope-core` only when they are genuinely foundational and do not depend on data, render, GPU, or app concerns.
- Evidence summarization and artifact formatting should stay in `rawscope-render` until a dedicated report crate is justified.
- The workbench should coordinate app flows in `apps/rawscope-workbench`; future egui integration should adapt existing library APIs instead of pulling UI concerns into core/data/render.
- Tests should continue using focused crate-level integration tests under `tests/` and tiny inline tests for local math only.

## Planning Invariants

- RawScope's center of gravity is visual exploration of large raw datasets with row-level evidence. Do not redirect this roadmap toward dashboards, general charting, SQL, dataframe execution, cloud analytics, or plugin systems.
- "Pixels should be explainable back to rows" is the main product contract. Any feature that renders new visual aggregates must define how selected pixels, bins, or regions lead back to row ids and evidence.
- Keep the current crate separation: `rawscope-core` stays foundational and dependency-light; `rawscope-data` owns dataset loading/identity/rows; `rawscope-gpu` owns GPU context/resource concerns; `rawscope-render` owns render/evidence logic; `rawscope-workbench` owns app coordination and UI.
- Do not add a dumping-ground `utils.rs`, `helpers.rs`, `types.rs`, `models.rs`, `contracts.rs`, `dto.rs`, or global `constants.rs`.
- Do not add DataFusion, Tauri, WASM/web, cloud flows, or plugins in this roadmap wave.
- Do not add Parquet/Arrow before dataset identity, local row retention, evidence v2, and row drilldown are stable.
- Do not claim benchmarked performance, zero-copy behavior, or production scale until the benchmark task lands and produces repeatable evidence.
- Prefer copy-minimising wording. Do not call the current CSV path, GPU upload, or evidence export zero-copy.
- Treat local CSV evidence as real local-data evidence only after the artifact includes source identity, column bindings, and original selected row values or an explicit sampled-row policy.
- Evidence schemas are versioned. Do not silently mutate v1 shapes; add v2 artifacts or maintain compatibility with documented migration behavior.
- A selected region must be data-anchored after finalization. Zoom, pan, resize, and reset must not silently change the data-space selection.
- Selection summaries and exports must be deterministic for the same dataset, view configuration, and selection.
- GPU row-id preservation remains out of scope until a task explicitly designs GPU-side row evidence. CPU-side drilldown is acceptable first.
- WGPU readback during interaction is acceptable for correctness-first proofs but must not be described as the final performance architecture.
- The first egui workbench must expose existing functionality; it must not become a broad UI framework or landing page.
- New visual views should be vertical slices with deterministic fixtures and CPU references before GPU acceleration.
- File imports must remain explicit and narrow. Avoid broad connector systems.
- For local data, preserve privacy by default. Do not add telemetry, upload, remote publish, or cloud report flows.
- Tests must prove behavior that would fail if the feature were fake. Avoid startup-only smoke tests.
- Any file over roughly 400 lines requires either a split plan or a written justification in the implementing task.
- Future agents should update docs when architecture-changing code lands, especially `README.md`, `docs/MVP.md`, `docs/ARCHITECTURE.md`, and schema docs.

## Task Dependency Table

| ID | Completed | Title | Description | Github Issue # | Blocked By | Task File |
|---|---|---|---|---|---|---|
| T001 | [x] | Reset Docs To Current Execution Baseline | Refresh stale status notes and add the roadmap baseline so future agents start from the real post-CSV/evidence-export state. |  | None | [`tasks/T001.md`](tasks/T001.md) |
| T002 | [x] | Add Dataset Identity And Field Bindings | Add a source-aware dataset identity model covering synthetic and local CSV datasets without pulling serde or UI concerns into data. |  | T001 | [`tasks/T002.md`](tasks/T002.md) |
| T003 | [x] | Add Generic Visual Row Records | Introduce scatter/timeline view record types that are not named synthetic, then migrate synthetic generators, CSV loading, density, brushing, and evidence inputs to those records. |  | T002 | [`tasks/T003.md`](tasks/T003.md) |
| T004 | [x] | Retain Local Source Rows For Drilldown | Preserve bounded original CSV row values by `RowId` so selected local rows can be inspected and exported instead of losing source columns during visual mapping. |  | T002 | [`tasks/T004.md`](tasks/T004.md) |
| T005 | [x] | Publish Evidence Schema V2 | Add scatter/timeline evidence v2 artifacts with dataset identity, field bindings, view config, lane labels, and selected source-row samples while keeping v1 behavior documented. |  | T003, T004 | [`tasks/T005.md`](tasks/T005.md) |
| T006 | [x] | Add CPU-Backed Row Drilldown | Add a selection drilldown model and workbench route that exposes exact or sampled selected rows from CPU state without claiming GPU row-id preservation. |  | T004, T005 | [`tasks/T006.md`](tasks/T006.md) |
| T007 | [x] | Add Egui Workbench Shell | Introduce a small egui/eframe-facing workbench shell with visible controls, axes labels, export status, and row-drilldown panel while preserving existing WGPU proof paths. |  | T006 | [`tasks/T007.md`](tasks/T007.md) |
| T008 | [x] | Add Linked Selection Contract | Add a shared visual selection/query state so scatter and timeline views can report and consume the same selection without becoming a dashboard system. |  | T006, T007 | [`tasks/T008.md`](tasks/T008.md) |
| T009 | [x] | Add Missingness Heatmap Slice | Add the first data-quality view for null/missingness shape with CPU reference, deterministic fixtures, brushing, and evidence hooks. |  | T002, T004, T007, T008 | [`tasks/T009.md`](tasks/T009.md) |
| T010 | [ ] | Add Evidence Report Bundles | Add report-bundle export with evidence JSON, Markdown, manifest metadata, and rendered visual context after screenshot/readback design is explicit. |  | T005, T007 | [`tasks/T010.md`](tasks/T010.md) |
| T011 | [ ] | Add Chunked Parquet Ingestion | Add Parquet/Arrow-backed chunked local dataset loading only after identity, row retention, evidence, and UI flows are stable. |  | T002, T003, T004, T005 | [`tasks/T011.md`](tasks/T011.md) |
| T012 | [ ] | Add Benchmarks And Performance Gates | Add repeatable CPU/GPU/workbench benchmarks and documentation rules that allow measured performance claims without polluting interactive UI paths. |  | T003, T011 | [`tasks/T012.md`](tasks/T012.md) |

Task details live in separate files under `tasks/`, named by task ID.

## Final Notes

- Recommended implementation order: T001, T002, T003, T004, T005, T006, T007, T008, T009, T010, T011, T012.
- Product priority: evidence credibility before UI polish, UI shell before linked multi-view workflows, row/source contracts before Parquet, benchmarks before performance claims.
- Unresolved questions: whether evidence v2 should replace v1 exports by default or live behind a temporary explicit export mode; how much local source-row data is acceptable to keep in memory for very large CSV files; when the app should move from `winit` directly to `eframe`; whether screenshot/readback should live in `rawscope-gpu`, `rawscope-render`, or a narrow report/export owner.
- Risks: synthetic-only naming can harden into public API; app state can centralize in `WorkbenchApp`; row drilldown can drift toward dataframe behavior; egui work can become a general UI rewrite; Parquet/Arrow can arrive before RawScope knows its evidence contract; benchmark numbers can be mistaken for product guarantees.
- Areas that need human review before implementation: evidence v2 schema shape, local row retention limits, UI shell direction, screenshot/readback dependency choices, Parquet dependency acceptance, and benchmark datasets/thresholds.
- Manual adversarial review notes folded into this bundle: do not rely on field-incompatible type aliases during the generic record migration; keep source rows in workbench state before row drilldown; make missingness depend on retained source rows; convert dataset identity into export DTOs rather than adding serde to `rawscope-data`; use checked row-id-to-index conversion for source-row lookup.
