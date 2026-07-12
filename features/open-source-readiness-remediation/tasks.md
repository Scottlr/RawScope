# RawScope Open-Source Readiness Remediation Tasks

## Discovery Summary

This bundle turns `RUST_CODEBASE_REVIEW.md` into an incremental remediation
programme for the current RawScope workspace. The review contains 143 findings:
11 Critical, 73 High, 50 Medium, 7 Low, and 2 Nit. The intent is not a rewrite.
Each task establishes one owner or invariant, migrates callers in a bounded
slice, and leaves the repository in a buildable old-or-new state.

Files and behavior inspected:

- `AGENTS.md`, `docs/AGENTS.md`, every repository Rust skill, and the invoked
  `large-feature-planning` skill.
- `RUST_CODEBASE_REVIEW.md`, especially the architecture map, complete finding
  catalogue, target crate graph, and prioritised remediation plan.
- Root and package `Cargo.toml` files, `Cargo.lock`, all crate `src/lib.rs`
  facades, and `apps/rawscope-workbench/src/main.rs`.
- Current data owners under `crates/rawscope-data/src/`, analytical and evidence
  owners under `crates/rawscope-render/src/`, GPU bootstrap under
  `crates/rawscope-gpu/src/`, session manifests, workbench orchestration, Python
  bridge modules, integration tests, ignored GPU tests, and Criterion benches.
- Existing feature bundles under `features/` so this bundle keeps the repository
  index/detail format and does not overwrite completed historical work.

Existing foundations to preserve:

- `rawscope-core` is dependency-free and already owns foundational IDs, ranges,
  grids, and visual-selection contracts.
- CPU reference implementations and deterministic fixtures exist and should
  remain the semantic oracle for GPU behavior.
- Evidence v1-v5 and timeline v1-v3 are already consumed artifacts. Their
  compatibility is an explicit migration constraint, not disposable history.
- The scatter resident-resource work and the current uncommitted inspection
  changes are real user work. Implementing sessions must inspect and preserve
  the then-current diff before touching overlapping files.
- There are meaningful Rust integration tests, Python contract tests, ignored
  adapter-backed GPU tests, and real benchmark harnesses to extend. No new smoke
  application or diagnostic-only harness is needed.

Target ownership after the programme:

```text
rawscope-core
  checked dependency-free IDs, domains, dimensions, and generation tokens

rawscope-session-contracts        rawscope-data
  bounded serde wire contract      typed/chunked ingestion and DatasetStore
                \                  /
                 rawscope-analysis
                 cohorts, binning, indexes, summaries, selections
                         |
                 rawscope-evidence
                 canonical evidence, wire adapters, readers, bundles
                         |
                 rawscope-render <---- rawscope-gpu
                 presentation/GPU      adapter, limits, recovery
                         \              /
                         rawscope-workbench
                         commands, controllers, jobs, borrowed UI projection
```

## Planning Invariants

- Treat `RUST_CODEBASE_REVIEW.md` as the finding source of truth. A task may
  refine a proposed correction, but it must not silently drop the finding.
- Migrate incrementally. No task may require a flag day across all crates, and
  every intermediate commit must have one coherent dependency graph.
- Keep `rawscope-core` dependency-free. It must not gain Serde, Arrow, Parquet,
  WGPU, winit, evidence formatting, filesystem IO, or UI concepts.
- `rawscope-data` owns source adapters, typed rectangular storage, raw evidence
  cells, normalized analytical values, stable row/column IDs, chunks, indexes,
  and RAM accounting. It does not own brushes, shaders, Markdown, or widgets.
- `rawscope-analysis` owns normative binning, cohorts, projections, profiles,
  missingness, diff, summaries, inspection, and selection snapshots. Its pure
  APIs consume immutable inputs and return immutable generation-tagged results.
- `rawscope-evidence` owns the canonical validated evidence domain, versioned
  readers/writers, escaping, provenance policy, and transactional bundle schema.
  It must never depend on WGPU, winit, egui, or the workbench application.
- `rawscope-render` owns presentation pipelines, shaders, render-specific
  resource generations, overlays, and transitions. It consumes validated
  analysis snapshots; it does not scan source tables or define evidence wire
  schemas after migration.
- `rawscope-gpu` stays narrow. Adapter selection, device limits, error scopes,
  recovery signals, and reusable low-level job/readback mechanics belong there;
  analyst workflow and renderer business logic do not.
- `rawscope-workbench` becomes a composition root and event router. Domain
  controllers own private state and communicate through typed commands/results.
- Preserve byte-level JSON and known-safe legacy Markdown golden compatibility.
  When context escaping necessarily changes hostile Markdown bytes, preserve
  reader/semantic compatibility, document the security correction, and add exact
  before/after fixtures; never keep unsafe default output merely for byte identity.
  New canonical types may not be serialized by accidental derive layout; explicit
  wire adapters own field names, versions, and ordering.
- Preserve session schema v1 and keep Rust/Python validation vectors in parity.
  Persist fixed-width integers, never `usize`, and bound reads before allocation.
- Raw source values and normalized analytical values are distinct contracts.
  Evidence must not silently rewrite source spelling, precision, nulls, or paths.
- One normative binning contract drives CPU analysis, selection, evidence, and
  documented WGSL parity vectors. Renderer-specific copies are forbidden.
- One immutable `SelectionSnapshot` is authoritative for sorted row IDs,
  selected counts, samples, bins, drilldown, linked views, and evidence.
- Missing, invalid, and valid values are three states. Every adapter, filter,
  profile, UI label, and evidence artifact must use the same policy.
- All dimensions, byte counts, row limits, timeouts, sample caps, channel
  capacities, and budgets use checked typed values with units near their owner.
- Job IDs and generation tokens are owner-minted. Stale completions may be
  discarded, but errors must remain observable and resources must be cleaned up.
- No lock guard may cross an await. Blocking IO/CPU work runs on bounded workers;
  WGPU resource creation and presentation remain on the appropriate owner thread.
- Interactive pointer paths may update constant-size state and submit bounded,
  coalesced work. They must not block on readback, rescan full datasets, or
  rebuild pipelines/resources per event.
- State transitions are transactional: prepare a complete next generation, then
  atomically publish it. On failure, keep the previous valid generation.
- RAM, VRAM, retained raw values, caches, queues, and samples have explicit
  budgets and a documented reject/degrade policy. Do not claim zero-copy.
- No benchmark or scale claim is allowed without the repository protocol,
  command, dataset, hardware/backend, date, and repeatable result.
- Until T035 passes, all packages remain non-published by policy. Do not enable
  crates.io or artifact publication merely because packaging succeeds.
- Do not add DataFusion, Tauri, WASM, cloud services, plugins, a SQL/dataframe
  engine, a dashboard framework, remote ingestion, or a new charting product.
- Prefer explicit structs, enums, newtypes, builders, and standard derives.
  Do not introduce a macro DSL or hide validation/business logic in macros.
- Keep facades thin and split by domain ownership. Do not add `utils.rs`,
  `helpers.rs`, `common.rs`, `misc.rs`, or giant type/model dumping grounds.
- Tests must prove behavior, compatibility, atomicity, parity, cancellation, or
  boundedness. No startup-only tests, fake demo apps, screenshots as assertions,
  broad snapshot churn, sleeps as synchronization, or meaningless smoke tests.
- GPU hardware tests stay clearly opt-in/scheduled until a declared runner
  exists. Normal CI must still test CPU/WGSL formulas and host ABI contracts.
- Every implementation session must inspect the current worktree and merge with
  user changes. This planning bundle does not authorize resetting or rewriting
  the current uncommitted inspection work.
- GitHub issue creation is outside this bundle. The `Github Issue #` column
  remains empty until the user explicitly requests issue publication.

## Task Dependency Table

| ID | Completed | Title | Description | Github Issue # | Blocked By | Task File |
|---|---|---|---|---|---|---|
| T001 | [ ] | Complete Legal And Package Metadata | Add licence texts, explicit private/public intent, publishable path versions, Cargo metadata, and Python distribution metadata. |  | None | [`tasks/T001.md`](tasks/T001.md) |
| T002 | [ ] | Pin Toolchain Lints And Supply-Chain Policy | Establish MSRV/toolchain, make Clippy clean, configure cargo-deny, and resolve or time-bound audited dependency exceptions. |  | None | [`tasks/T002.md`](tasks/T002.md) |
| T003 | [ ] | Add Required CI And Platform Gates | Add pinned merge gates for Rust, Python, packaging, docs, policy, supported platforms, and opt-in GPU correctness. |  | T001, T002 | [`tasks/T003.md`](tasks/T003.md) |
| T004 | [ ] | Publish Contributor Security And Release Governance | Add public contribution, disclosure, conduct, support, versioning, and release-policy documents/templates. |  | T002, T003 | [`tasks/T004.md`](tasks/T004.md) |
| T005 | [x] | Introduce Checked Core Domain Contracts | Make foundational ranges, dimensions, identities, selections, and owner-minted generations difficult to misuse without adding dependencies. |  | None | [`tasks/T005.md`](tasks/T005.md) |
| T006 | [x] | Add A Typed Rectangular DatasetStore | Replace publicly mutable row/table shapes with a generation-owned store that separates raw, normalized, missing, and invalid values. |  | T005 | [`tasks/T006.md`](tasks/T006.md) |
| T007 | [ ] | Correct Synthetic And CSV Boundary Semantics | Fix small synthetic counts/ranges and make CSV limits and unsigned timestamps obey checked contracts immediately. |  | T005 | [`tasks/T007.md`](tasks/T007.md) |
| T008 | [ ] | Correct Parquet Type Dispatch And Errors | Reject or support types consistently, stop fabricated values, and report actionable source-format errors immediately. |  | T005 | [`tasks/T008.md`](tasks/T008.md) |
| T009 | [ ] | Implement Chunked Ingestion And Bounded Source Indexes | Stage one-pass profile validation, stream chunks into the store, bound category/key work, and account retained memory. |  | T006, T007, T008, T044, T045 | [`tasks/T009.md`](tasks/T009.md) |
| T010 | [ ] | Define Precision-Safe Projection And GPU Quantization | Keep CPU/source numeric truth precise, avoid raw projection copies, validate finite visual records, and disclose GPU quantization. |  | T005, T006, T013 | [`tasks/T010.md`](tasks/T010.md) |
| T011 | [ ] | Split Bounded Session Contracts | Create a lightweight session-contract crate with normalized identifiers, bounded IO, fixed-width wire limits, and role-aware errors. |  | T005 | [`tasks/T011.md`](tasks/T011.md) |
| T012 | [ ] | Align And Harden The Python Dataframe Bridge | Match Rust validation, reject booleans/type mismatches, prepare bundles transactionally, bound key checks, and clean launch failures. |  | T007, T008, T011, T045 | [`tasks/T012.md`](tasks/T012.md) |
| T013 | [ ] | Create Normative Analysis Binning | Establish `rawscope-analysis`, one bounded binning contract, and checked summary configuration after the tactical axis fix. |  | T005, T036 | [`tasks/T013.md`](tasks/T013.md) |
| T014 | [ ] | Add Immutable Cohort Generations | Move validated filters into analysis, distinguish missing/invalid policy, mint revisions internally, and publish immutable cohort snapshots. |  | T006, T013 | [`tasks/T014.md`](tasks/T014.md) |
| T015 | [ ] | Add Reusable Profile Missingness And Diff Indexes | Move bounded profiles, missingness, and stable-column diff into reusable analysis indexes with deterministic tie policy. |  | T006, T009, T013, T014 | [`tasks/T015.md`](tasks/T015.md) |
| T016 | [ ] | Move The Canonical SelectionSnapshot Into Analysis | Move the already-correct filtered snapshot contract into analysis and migrate pure consumers without changing truth. |  | T013, T014, T038 | [`tasks/T016.md`](tasks/T016.md) |
| T017 | [ ] | Migrate Inspection And Drilldown To Checked Snapshots | Remove panicking/zero-width inspection paths, bound occupied-bin sampling, and disclose unavailable versus limited source rows. |  | T013, T016 | [`tasks/T017.md`](tasks/T017.md) |
| T018 | [ ] | Make Current Evidence Contracts Serializer-Validated | Privatize v4/v5 state, enforce complete validation at every serializer, reject missing source rows, and make schema versions authoritative. |  | T016, T017 | [`tasks/T018.md`](tasks/T018.md) |
| T019 | [ ] | Create The Canonical Evidence Domain | Create a private canonical evidence model/validation crate without moving legacy adapters or callers in the same PR. |  | T010, T011, T013, T018 | [`tasks/T019.md`](tasks/T019.md) |
| T020 | [ ] | Add Safe V6 V4 Evidence And Schema Docs | Add scatter v6/timeline v4 wire contracts for portable provenance/quantization, safe formatting, precise numbers, and complete docs. |  | T010, T019, T039, T046, T047 | [`tasks/T020.md`](tasks/T020.md) |
| T021 | [ ] | Add Bundle Manifest V2 And Validated Readers | Move transactional behavior into evidence, add relative-path bundle schema v2, and read every legacy v1 path form strictly. |  | T019, T020, T040, T046, T047 | [`tasks/T021.md`](tasks/T021.md) |
| T022 | [ ] | Validate GPU ABI And Resource Limits | Prove host/WGSL layouts, binding sizes, buffer lengths, checked allocation bytes, and legal dispatch plans. |  | T005 | [`tasks/T022.md`](tasks/T022.md) |
| T023 | [ ] | Add Cancellable Nonblocking GPU Readback | Replace indefinite polling and fake async APIs with callback-driven jobs, cancellation/timeouts, bounded clearing, and surfaced failures. |  | T022, T058 | [`tasks/T023.md`](tasks/T023.md) |
| T024 | [ ] | Build A Resident Timeline Engine | Reuse corrected timeline GPU resources and expose nonblocking preview/exact generations without re-fixing interaction semantics. |  | T013, T022, T023, T037 | [`tasks/T024.md`](tasks/T024.md) |
| T025 | [ ] | Share Scatter GPU Resources Across Validated Generations | Share dataset buffers, cache bind groups, stabilize large-count difference math, validate point configs/indices, and derive field metadata. |  | T010, T013, T022, T023 | [`tasks/T025.md`](tasks/T025.md) |
| T026 | [ ] | Bound Relief And Transition Resource Costs | Precompute relief inputs, cap quality work, and disable or rebuild transitions when prior/current resource generations are incompatible. |  | T025 | [`tasks/T026.md`](tasks/T026.md) |
| T027 | [ ] | Add A Native Job Coordinator And Typed Operation Errors | Introduce bounded jobs, owner-minted IDs, cancellation, event-loop completions, and source-preserving recovery classifications. |  | T005, T011 | [`tasks/T027.md`](tasks/T027.md) |
| T028 | [ ] | Admit Dataset Generations Under Resource Budgets | Add generation-owned RAM/VRAM accounting and atomic active/pending admission after tactical startup is nonblocking. |  | T006, T009, T022, T025, T043 | [`tasks/T028.md`](tasks/T028.md) |
| T029 | [ ] | Migrate Settling Into The Final Render Coordinator | Move already-contained interaction work into final analysis/GPU generations with bounded ticket retirement and exact publication. |  | T016, T017, T023, T024, T025, T028, T042 | [`tasks/T029.md`](tasks/T029.md) |
| T030 | [ ] | Publish One Active Workbench Generation And Cached UI | Swap one immutable aggregate generation, cache borrowed UI projections, and preserve typed degraded state through a thin app facade. |  | T028, T029, T041, T049, T050 | [`tasks/T030.md`](tasks/T030.md) |
| T031 | [ ] | Add A Deliberate Cargo Feature Matrix | After ownership is correct, make Parquet/native presentation optional at narrow owners and measure dependency/build-cost changes. |  | T001, T003, T011, T019, T022, T030, T058 | [`tasks/T031.md`](tasks/T031.md) |
| T032 | [ ] | Add Cross-Boundary Atomicity Regressions | Prove filtered row-set truth, old-or-new controller/resource/bundle commits, and required CPU/WGSL parity without fuzz scope. |  | T003, T012, T017, T021, T024, T025, T030, T049, T050 | [`tasks/T032.md`](tasks/T032.md) |
| T033 | [ ] | Add Reproducible CPU Product Benchmarks | Benchmark ingest, indexes, selection, evidence, and bundle paths with versioned metadata and RAII fixtures. |  | T009, T012, T015, T021, T032 | [`tasks/T033.md`](tasks/T033.md) |
| T034 | [ ] | Finalize Crate And Module Boundaries | Remove residual dual/oversized owners and temporary re-exports, leaving thin facades and the final allowed dependency DAG. |  | T011, T015, T024, T025, T030, T031, T048, T050 | [`tasks/T034.md`](tasks/T034.md) |
| T035 | [ ] | Assemble And Inspect Release Artifacts | Build approved source/native/Python packages, inspect contents, and emit checksums/release manifest without publication. |  | T001, T002, T003, T004, T031, T032, T034, T051, T052, T053, T054 | [`tasks/T035.md`](tasks/T035.md) |
| T036 | [ ] | Repair Current Axis Termination And Integer Ticks | Fix non-progressing axes, bound tick counts, and preserve large-u64 labels in current owners before extraction. |  | None | [`tasks/T036.md`](tasks/T036.md) |
| T037 | [ ] | Repair Current Timeline Lane And Range Semantics | Fix lane orientation, integer binning/overflow, empty-config validation, and before/after errors in current owners. |  | None | [`tasks/T037.md`](tasks/T037.md) |
| T038 | [ ] | Repair Current Filtered Selection Truth | Build one current-owner filtered snapshot so linked views, bins, samples, aggregates, and evidence agree immediately. |  | T005 | [`tasks/T038.md`](tasks/T038.md) |
| T039 | [ ] | Secure Current Evidence Formatting And Provenance | Add contextual Markdown escaping and default absolute-path redaction to current writers before evidence extraction. |  | None | [`tasks/T039.md`](tasks/T039.md) |
| T040 | [ ] | Make Current Report Bundle Writes Transactional | Replace sequential/TOCTOU bundle writes with no-clobber staged commit in the current owner before migration. |  | None | [`tasks/T040.md`](tasks/T040.md) |
| T041 | [ ] | Make Current Demo Filter And Projection Updates Atomic | Stage current multi-consumer mutations and commit complete old-or-new state before controller restructuring. |  | T005, T038 | [`tasks/T041.md`](tasks/T041.md) |
| T042 | [ ] | Contain Pointer-Time Scatter And Timeline Work | Remove compute/readback/full scans from pointer handlers, coalesce gestures, and perform one latest settle before final coordinator migration. |  | T023, T024, T025, T027, T038 | [`tasks/T042.md`](tasks/T042.md) |
| T043 | [ ] | Move Current Startup And GPU Initialization Off The Event Thread | Stage manifest/data loading and adapter/device futures without event-thread `pollster::block_on`, preserving prior ready state. |  | T005, T011, T022, T027, T058 | [`tasks/T043.md`](tasks/T043.md) |
| T044 | [ ] | Preserve CSV Raw Values In DatasetStore | Add raw-versus-normalized CSV retention to the checked store after immediate CSV correctness lands. |  | T006, T007 | [`tasks/T044.md`](tasks/T044.md) |
| T045 | [ ] | Unify Source Adapter Normalization And Metadata | Make CSV/Parquet lane normalization, source identity, and structured logging identical over the checked store. |  | T006, T007, T008, T044 | [`tasks/T045.md`](tasks/T045.md) |
| T046 | [ ] | Migrate Scatter Evidence Adapters And Readers | Move scatter v1-v5 typed adapters/readers/goldens into evidence without caller migration or new schema behavior. |  | T019 | [`tasks/T046.md`](tasks/T046.md) |
| T047 | [ ] | Migrate Timeline Evidence Adapters And Readers | Move timeline v1-v3 typed adapters/readers/goldens into evidence independently of scatter/callers. |  | T019 | [`tasks/T047.md`](tasks/T047.md) |
| T048 | [ ] | Migrate Evidence Callers And Remove Render Ownership | Switch workbench/tests to evidence, remove render semantic modules/re-exports, and leave render presentation-only. |  | T020, T021, T046, T047 | [`tasks/T048.md`](tasks/T048.md) |
| T049 | [ ] | Add The Async Export Controller | Capture validated evidence, submit transactional bundle jobs, and commit matching completion/error state off the event thread. |  | T021, T027, T048 | [`tasks/T049.md`](tasks/T049.md) |
| T050 | [ ] | Split Scatter Timeline And Selection Controllers | Move already-correct current behavior into three vertical private controllers with typed commands/results. |  | T028, T029, T041, T042, T043 | [`tasks/T050.md`](tasks/T050.md) |
| T051 | [ ] | Add Property And Fuzz Coverage | Add bounded range/bin/store/session/CSV/Parquet properties and three semantic fuzz targets separate from atomicity tests. |  | T010, T011, T013, T019, T032 | [`tasks/T051.md`](tasks/T051.md) |
| T052 | [ ] | Reconcile Architecture README And Public API Docs | Document enforceable final ownership/runtime sequences and correct claims/rustdoc against code and approved benchmarks. |  | T033, T034, T057 | [`tasks/T052.md`](tasks/T052.md) |
| T053 | [ ] | Archive Historical Feature Plans | Move completed plans outside the active contributor path without rewriting records and repair all indexes/links. |  | T052 | [`tasks/T053.md`](tasks/T053.md) |
| T054 | [ ] | Add Maintained Rust Examples | Add compile-checked real ingestion, selection, and evidence-reader examples over stable final APIs. |  | T010, T011, T019, T031, T034 | [`tasks/T054.md`](tasks/T054.md) |
| T055 | [ ] | Add Release SBOM Provenance And Signing | Generate hash-bound SBOM/provenance and optional approved signatures for T035 artifacts without registry mutation. |  | T002, T003, T035 | [`tasks/T055.md`](tasks/T055.md) |
| T056 | [ ] | Add Protected Publishing And Rollback | Enable only the approved publication matrix behind protected human approval and document partial-failure/yank/rollback response. |  | T004, T035, T055 | [`tasks/T056.md`](tasks/T056.md) |
| T057 | [ ] | Benchmark Interactive GPU And Resource Paths | Measure real gesture/settle, resident GPU, relief, RAM/VRAM/upload, and generation metrics on named hardware. |  | T024, T025, T026, T028, T029, T030, T033 | [`tasks/T057.md`](tasks/T057.md) |
| T058 | [ ] | Add GPU Adapter Policy And Recovery | Add typed fallback selection, error scopes, uncaptured/device-loss handling, and explicit surface/device recovery generations. |  | T005, T022 | [`tasks/T058.md`](tasks/T058.md) |

Task details live in separate files under `tasks/`, named by task ID.

## Final Notes

- Primary audit coverage is traceable as follows: T001 owns RS-001, RS-002,
  RS-007, RS-015; T002 owns RS-004-RS-006, RS-008, RS-013, RS-014; T003 owns
  RS-003, RS-009, RS-132; T004 owns RS-011; T005 owns RS-016-RS-020, RS-023;
  T006 owns RS-021, RS-022, RS-037, RS-042, RS-047; T007 owns RS-026-RS-029;
  T008 owns RS-032, RS-033, RS-036; T009 owns RS-031, RS-038, RS-041, RS-044;
  T010 owns RS-034, RS-043, RS-060; T011 owns RS-118-RS-122; T012 owns
  RS-123-RS-130; T013 owns RS-070, RS-081; T014 owns RS-024, RS-039, RS-040,
  RS-071; T015 owns RS-025, RS-078-RS-080; T017 owns RS-075-RS-077, RS-082;
  T018 owns RS-086-RS-091, RS-099; T019 owns RS-100; T020 owns RS-096, RS-098,
  RS-105; T021 owns RS-103; T022 owns RS-048, RS-058; T023 owns RS-049,
  RS-051, RS-052; T024 owns RS-050; T025 owns RS-061, RS-063-RS-068; T026
  owns RS-062; T027 owns RS-109, RS-117; T028 owns RS-115; T030 owns RS-112,
  RS-116; T031 owns RS-012;
  T032 owns RS-133; T033 owns RS-143; T034 owns RS-135; T036 owns
  RS-072-RS-074; T037 owns RS-053-RS-057; T038 owns RS-083-RS-085, RS-092;
  T039 owns RS-093, RS-094; T040 owns RS-101, RS-102; T041 owns RS-111,
  RS-113, RS-114; T042 owns RS-108, RS-110; T043 owns RS-107; T044 owns
  RS-030; T045 owns RS-035, RS-045, RS-046; T046 owns RS-095, RS-097; T048
  owns RS-136; T049 owns RS-104; T050 owns RS-106, RS-137; T051 owns RS-131;
  T052 owns RS-138, RS-140, RS-141; T053 owns RS-139; T054 owns RS-142; T056
  owns RS-010; T057 owns RS-134; and T058 owns RS-059, RS-069.
- Tactical correctness comes first: T007/T008, T036-T043, T022/T058/T023, and
  T027 fix confirmed behavior in current owners. Structural tasks T013-T021,
  T024-T031, and T044-T050 then move already-correct contracts into final owners.
  Do not combine a tactical fix with its later crate/controller move merely to
  reduce task count.
- Key correctness paths are T036 -> T013; T037 -> T024; T005 -> T038 -> T041;
  T022 -> T058 -> T023 -> T024/T025 -> T042; and T005/T011/T022/T027/T058 ->
  T043. Filtered evidence then moves through T014 -> T016 -> T017 -> T018 ->
  T019 -> T046/T047 -> T020/T021 -> T048/T049. Final controllers and active state
  follow T028/T029/T041/T049/T050 -> T030.
- The release sequence is deliberately separate: T032/T051 regressions, T033/T057
  benchmark evidence, T034/T052-T054 public maintenance, T035 artifact assembly,
  T055 supply-chain metadata, then T056 protected human-approved publication.
- Human approval is required for the legal copyright holder, eventual public
  crate matrix, MSRV/support matrix, security contact, release signing keys,
  default provenance redaction policy, and RAM/VRAM budget defaults.
- Historical completed feature bundles remain historical evidence. T053 may
  archive them from the primary contributor path, but must not rewrite their
  original task records or pretend regressed behavior was never completed.
- The adversarial review was folded into this bundle: it forced tactical-first
  ordering, PR-sized evidence/controller/test/docs/release splits, explicit
  scatter v6/timeline v4 and bundle-manifest v2 contracts, durable GPU completion/
  retirement semantics, one active generation swap, and protected publication.
