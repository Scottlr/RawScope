# Evidence-Preserving GPU Visual Fields Tasks

## Discovery Summary

This bundle plans a generic GPU visual-field architecture and a deliberately
small set of new visual analytics modes. It does not make any dataset profile,
including the existing Lichess profile, part of RawScope's product model.
Profiles remain optional presets over the same typed field contracts available
to every CSV, Parquet, session, and Python-bridged dataset.

Repository discovery covered:

- `AGENTS.md`, `docs/AGENTS.md`, and the repository skills for preflight,
  Rust organization, testing, constants/configuration, observability, idioms,
  and macro policy.
- `docs/ARCHITECTURE.md`, `docs/TECH_DECISIONS.md`, `docs/BENCHMARKS.md`,
  `docs/VISION.md`, and `research/rawscope-gpu-scale-visual-analytics.md`.
- The completed `features/next-generation-gpu-visual-analytics` and
  `features/data-bridge-and-inspection-lens` bundles.
- The active `features/open-source-readiness-remediation` destination for
  checked stores, analysis ownership, immutable cohorts/selections, evidence,
  resident GPU resources, render coordination, controllers, resource budgets,
  and reproducible performance measurement.
- Root and crate manifests plus the current facades for `rawscope-core`,
  `rawscope-data`, `rawscope-analysis`, `rawscope-evidence`, `rawscope-gpu`,
  `rawscope-render`, session contracts, the Python SDK, and the workbench.
- Current visual owners including `density_encoding.rs`, scatter density/
  difference/point renderers, resident field metadata, WGSL shaders, inspection,
  axes/marginals, visual encoding UI, render scheduling, active generations,
  controller prototypes, session v1, and canonical evidence context.
- Current production file sizes. Several render/workbench owners exceed the
  repository's approximate 400-line review threshold, so this bundle plans
  owner-first splits before adding behavior.

Existing architecture to preserve and complete:

- `rawscope-core` remains dependency-free and owns only checked foundational
  values.
- `rawscope-data` owns typed/chunked storage, schemas, source adapters, stable
  row identity, and bounded source indexes.
- `rawscope-analysis` owns generic projection, cohort, binning, summaries,
  composition math, and other GPU-independent analytical truth.
- `rawscope-gpu` owns WGPU device/surface/limits/recovery mechanics.
- `rawscope-render` owns resident GPU visual fields, derived presentation
  resources, shaders, plot geometry, and rendering math; it does not own
  evidence serialization or dataset-specific semantics.
- `rawscope-evidence` owns canonical evidence and every wire serializer.
- `rawscope-workbench` owns controllers, interaction, mode coordination, UI
  projection, and atomic publication of complete generations.
- Session contracts and the Python SDK map generic source columns into native
  field roles; they do not reproduce render internals.

This bundle assumes the relevant structural work in
`features/open-source-readiness-remediation` has reached its intended final
owners before implementation. In particular, implementations must verify the
landed behavior corresponding to T009-T010, T013-T017, T022-T030, T034, T038,
T048, T050, and T057. Completion checkboxes may lag code; inspect actual owners
and never recreate a temporary compatibility implementation.

## Planning Invariants

- RawScope is an evidence-centered visual analytics instrument, not a charting
  library, dashboard builder, shader playground, or general render framework.
- No production branch may check `DatasetProfileId::LichessGames` (or any other
  profile identity) to decide whether a visual mode exists. Profiles may supply
  optional field, formatting, guide, and ordering hints only.
- The same generic modes must work with arbitrary compatible column names and
  without any profile.
- Row count remains the only density aggregation in this bundle. Weighted
  density, arbitrary measures, and user-defined aggregation expressions are
  deferred because they change pixel and tooltip semantics.
- One validated `VisualFieldMapping` is the source of truth for numeric-pair and
  time-value projections plus an optional categorical channel. Do not add a
  parallel mapping type per mode.
- Integer and timestamp axis domains remain exact typed values through analysis,
  selection, inspection, and evidence. Normalized f64/f32 visual coordinates are
  a disclosed quantization boundary, not reconstructed source truth.
- Existing scatter behavior migrates into the generic visual-field path. New
  modes must not wrap the current scatter renderer while leaving two long-term
  projection/resource/state owners.
- The exact integer count field is analytical truth. Smoothing, palette lookup,
  contours, relief, composition coloring, split comparison, semantic zoom, and
  ridges are derived presentations and never rewrite counts or row membership.
- Use a closed mode enum and explicit compatibility table. Do not add a plugin
  API, dynamic shader registry, ECS, generic render graph, trait-object pipeline,
  macro DSL, or stringly typed mode map.
- Numeric-pair and time-value views share the same validated 2D field pipeline.
  The existing time-by-lane timeline remains a separate discrete-lane view.
- Time-value density is static and brushable in this bundle. Do not add animated
  XY playback, inferred trajectories, or motion that suggests rows moved.
- Category composition is bounded. Every displayed layer, `Other`, missing, and
  invalid policy is explicit. High-cardinality values are never silently
  discarded or blended into an unnamed color.
- Category identity remains typed for text, boolean, and integer-coded groups.
  Store-owned packed row codes upload once per dataset/column/device; changing
  visible layers uploads a bounded lookup rather than rewriting every row.
- Category hue identifies the dominant displayed layer; lightness communicates
  total row density; saturation communicates normalized composition purity.
  Synthetic blended hues must not be presented as real categories.
- Cohort difference remains normalized active share minus baseline share.
  Support-aware visibility may suppress visual noise but must not be called
  statistical significance.
- A comparison split lens renders baseline and active fields with identical
  domain, grid, transform, normalization, and palette. Moving the split is a
  constant-size uniform update.
- Mass contours are derived from one settled exact count snapshot and record the
  actual enclosed row count after count ties. Do not label transformed-intensity
  contours as 50/80/95/99 percent mass.
- Marginals, contours, inspection, and evidence share one settled count snapshot.
  Do not trigger duplicate full-grid readbacks for each consumer.
- Density ridges are undirected, scale-dependent structure in a smoothed scalar
  field. Do not call them flow, paths, trajectories, clusters, or causal motion.
- Semantic zoom crossfades one hierarchy: density to representative/exact
  points. Do not leave opaque cells and equally dominant point glyphs competing.
- Sampled point reveal and row evidence always disclose sampled versus complete
  status and the eligible/rendered counts.
- Pointer handlers perform only plot-coordinate math, generation-tagged intent
  submission, and constant-size uniform/state updates. They do not scan rows,
  rebuild indexes, upload columns, dispatch compute directly, map buffers, or
  wait for GPU work.
- Interactive gestures reproject the last compatible settled field immediately.
  Preview/exact work coalesces through the final render coordinator and stale
  completions cannot publish.
- No GPU readback occurs per pointer event or animation frame. Required exact
  readback is asynchronous, generation-tagged, cancellable, and shared by
  settled consumers.
- Adaptive resolution chooses from named validated tiers using physical plot
  size, WGPU limits, and the existing resource budget. It never allocates from
  raw viewport dimensions without checked arithmetic.
- Every new GPU buffer, texture, pipeline cache, transition field, staging
  buffer, and retained previous generation contributes to the existing
  active/pending/retiring resource plan.
- Resource pressure may reduce an explicitly recorded presentation quality tier
  or disable a recomputable derived presentation. It may not drop source rows,
  alter cohort/selection truth, hide category layers, or weaken evidence.
- CPU references own formulas for mass thresholds, composition purity,
  difference support, time projection, and ridge derivation. GPU tests compare
  against those references on deterministic boundary fixtures.
- GPU ABI structs remain `#[repr(C)]` with explicit host/WGSL size/alignment
  tests and device-limit validation before allocation or dispatch.
- Palette definitions have one render-owned LUT source used by shaders, legends,
  and exported visual context. Do not duplicate RGB stops in WGSL and egui.
- Color interpolation is linear-light and palettes are curated/closed. Do not
  add arbitrary user color editors or a broad color-science dependency.
- Presentation animation is bounded and reduced-motion aware. Animation changes
  neither evidence nor mode truth.
- The workbench shell uses a restrained professional analytical visual language:
  compact command chrome, a focused central viewport, collapsible tool/property
  regions, a concise status strip, and consistently rounded panels with subtle
  borders and elevation. Blender and modern Visual Studio are inspiration for
  information density and hierarchy only; do not copy their branding or exact
  layouts.
- Shell layout remains one curated RawScope workspace, not a generic docking or
  window-management framework. Secondary regions collapse before the central
  plot falls below its named usable dimensions.
- Egui layout, pointer routing, axes, WGPU viewport/scissor, and inspection share
  one authoritative physical plot rectangle. Rounded chrome must not introduce
  decorative clipping or overlays that disagree with that rectangle.
- Session v1 and all legacy evidence artifacts remain readable and byte-stable
  for their supported safe inputs. New semantics use additive session/evidence
  contracts rather than mutating old bytes.
- Serde remains in session/evidence owners. Do not add serialization derives to
  `rawscope-analysis`, `rawscope-data`, or final presentation-only render types.
- No new runtime dependency is expected. A task must stop for human review
  before adding one.
- Files over roughly 400 lines must be split by vertical responsibility before
  receiving substantial behavior. Do not create `utils.rs`, `helpers.rs`,
  `common.rs`, `types.rs`, `models.rs`, or another broad dumping ground.
- Tests prove formulas, membership, generation consistency, mode compatibility,
  CPU/GPU parity, bounded resource behavior, or real UI action projection. No
  startup smoke tests, fake apps, screenshot tests, diagnostic scripts, or broad
  test churn.
- Deterministic generic fixtures use neutral field names and at least two schema
  shapes. The external Lichess files are optional manual acceptance inputs only
  and are never required by CI or copied into the repository.
- Performance claims require the repository benchmark protocol: command,
  scenario, commit, date, OS, CPU/RAM, GPU adapter/backend/driver, dataset, and
  configuration. Unit tests contain no wall-clock assertions.

## Task Dependency Table

| ID | Completed | Title | Description | Github Issue # | Blocked By | Task File |
|---|---|---|---|---|---|---|
| T001 | [x] | Add Generic Visual Field Mapping And Projection Contracts | Replace scatter-only projection identity with one validated numeric-pair/time-value mapping and immutable projected generation, while leaving discrete-lane timelines separate. |  | None | [`tasks/T001.md`](tasks/T001.md) |
| T002 | [x] | Reorganize Scatter Rendering Into Visual Field Owners | Move existing exact/topographic/relief/reprojection behavior into focused visual-field modules and one resident field generation without retaining parallel scatter owners. |  | T001 | [`tasks/T002.md`](tasks/T002.md) |
| T003 | [x] | Add Adaptive Resolution And Derived-Field Resource Policy | Select checked grid tiers from physical plot size/device limits, account every derived resource, and route preview/exact work through final generation budgets. |  | T002 | [`tasks/T003.md`](tasks/T003.md) |
| T004 | [x] | Add Perceptual Palette LUTs And Stable Color Semantics | Replace duplicated four-stop shader/UI palettes with one linear-light LUT source and explicit semantic transform/normalization ownership. |  | T002, T003 | [`tasks/T004.md`](tasks/T004.md) |
| T005 | [x] | Add Exact Mass Contours And Axis-Integrated Marginals | Derive tied mass thresholds and X/Y marginals from one settled count snapshot, then render them in the plot/axis composition. |  | T001, T002, T003, T004 | [`tasks/T005.md`](tasks/T005.md) |
| T006 | [x] | Complete Continuous Density-To-Point Semantic Zoom | Rework point reveal into a generation-cached, off-pointer-time plan with one deliberate density/point crossfade and truthful sample disclosure. |  | T002, T003, T004 | [`tasks/T006.md`](tasks/T006.md) |
| T007 | [x] | Add Bounded Generic Category Channel Indexes | Extend the final store/index architecture with stable row-aligned category IDs and an analysis-owned bounded layer plan including explicit Other/missing/invalid semantics. |  | T001 | [`tasks/T007.md`](tasks/T007.md) |
| T008 | [x] | Add Resident GPU Category Composition Aggregation | Upload one reusable category channel and compute bounded per-layer count fields tied to dataset/cohort/mapping generations. | 80c9d7d | T002, T003, T007 | [`tasks/T008.md`](tasks/T008.md) |
| T009 | [ ] | Add Category Composition Presentation And Inspection | Render density, dominant category, and purity without invented hues; extend O(1) inspection and semantic point reveal with exact layer shares. |  | T004, T006, T008 | [`tasks/T009.md`](tasks/T009.md) |
| T010 | [x] | Add Cohort Comparison Split Lens And Contextual Difference Shading | Rework the existing difference renderer around shared field resources, add support-aware baseline context, and add a uniform-only split comparison lens. | 5ffe2e2 | T002, T003, T004, T005 | [`tasks/T010.md`](tasks/T010.md) |
| T011 | [x] | Add Generic Time-Value Density Projection | Project any validated timestamp and numeric value column through the common 2D field pipeline with precise time axes, brushing, inspection, and optional category composition. | 11e6e0e | T001, T002, T003, T004 | [`tasks/T011.md`](tasks/T011.md) |
| T012 | [x] | Add Multiscale Density Ridge Analysis | Define bounded fine/medium/coarse scalar-field smoothing, Hessian ridge strength, and undirected orientation in analysis with deterministic CPU references. |  | T001, T003 | [`tasks/T012.md`](tasks/T012.md) |
| T013 | [ ] | Add Resident GPU Density Ridge Presentation | Compute and render generation-safe ridge strength/orientation fields only on settled compatible density generations, with CPU/GPU parity and no motion implication. |  | T002, T003, T004, T012 | [`tasks/T013.md`](tasks/T013.md) |
| T014 | [ ] | Add Closed Mode Compatibility And Generic Workbench Controls | Replace profile-identity branches and oversized visual UI owners with mapping-driven availability, typed controller commands, and production-grade controls for the shipped modes. |  | T005, T006, T009, T010, T011, T013, T018 | [`tasks/T014.md`](tasks/T014.md) |
| T015 | [ ] | Add Session V2 And Python Generic Field Bindings | Add additive numeric-pair/time-value/category bindings to Rust session contracts and the dependency-free Python bridge while preserving v1 output for legacy views. |  | T001, T014 | [`tasks/T015.md`](tasks/T015.md) |
| T016 | [ ] | Add Visual Field Evidence V1 And Report Integration | Add a generic evidence artifact for projection and mode semantics, preserve every legacy artifact, and route report bundles through canonical evidence ownership. |  | T005, T006, T009, T010, T011, T013, T014, T015 | [`tasks/T016.md`](tasks/T016.md) |
| T017 | [ ] | Benchmark And Curate The Generic Visual Field Workbench | Exercise the complete generic workflow on neutral deterministic schemas, extend real product-path benchmarks/resource reports, and remove remaining duplicate or oversized owners. |  | T003, T005, T006, T009, T010, T011, T013, T014, T015, T016 | [`tasks/T017.md`](tasks/T017.md) |
| T018 | [x] | Add A Rounded Professional Analytical Workbench Shell | Rework the existing egui shell into a simpler Blender/modern-Visual-Studio-inspired workspace with responsive tool/property regions while preserving one authoritative GPU plot rectangle. |  | T003 | [`tasks/T018.md`](tasks/T018.md) |

Task details live in separate files under `tasks/`, named by task ID.

## Final Notes

- Recommended implementation order: external remediation gate, T001 -> T002 ->
  T003; then T004, T007, T012, and T018; then T005/T006, T008, T010, T011,
  and T013; then T009 -> T014 -> T015/T016 -> T017.
- Safe parallel work after T003: T005/T006 own settled context/zoom, T007/T008/T009
  own composition, T010 owns comparison, T011 owns time-value projection,
  T012/T013 own ridges, and T018 owns shell layout/theme only. Do not overlap
  sessions editing shared renderer modules until T002 has landed and write sets
  are explicit.
- Deliberately excluded: animated scatter playback, weighted density, arbitrary
  aggregations, perspective/3D terrain, particle streams, LIC flow texture,
  missingness animation, generic small multiples, chart grammar, plugins, and
  user-authored shaders.
- Unresolved human decisions: default exact/preview grid tiers, maximum visible
  category layers, final sequential/diverging/categorical palettes, default
  mass contour levels, point crossfade thresholds, comparison presentation
  default, ridge scale/strength defaults, mode ordering in the workbench, shell
  density/radius tokens, secondary-panel default visibility, and the narrow-
  window collapse threshold.
- Human review must inspect color accessibility, category ambiguity, contour
  labels, sparse glyph hierarchy, reduced motion, difference visibility, and
  ridge interpretation on at least two unrelated schemas, plus shell hierarchy
  and responsive behavior at normal, narrow, and HiDPI window sizes.
- Main risks: retaining temporary and final projection owners simultaneously;
  category buffers multiplying VRAM; count readback fan-out; mode combinations
  creating an implicit chart grammar; ridge shading implying unsupported flow;
  session/evidence contracts exposing unstable render implementation details;
  and decorative shell geometry diverging from the authoritative plot rectangle.
- The adversarial planning review must reject any implementation that adds a
  mode label without a real compute/presentation path, creates a Lichess-only
  code branch, performs row work in pointer handlers, or leaves old scatter
  state as a permanent mirror.
