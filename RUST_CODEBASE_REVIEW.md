### 1. Executive assessment

Review date: 2026-07-11. This assessment covers the current working tree, including the pre-existing uncommitted interaction/inspection changes on `codex/fix-hover-zoom-crosshair`. No production code was changed for this review.

Score scale: 10 means suitable for long-lived public use with only routine maintenance; 1 means fundamentally unfit for publication.

| Area | Score | Assessment |
| --- | ---: | --- |
| Overall quality | 5/10 | There is substantial real implementation and a meaningful test base, but several user-visible correctness defects, evidence-integrity failures, synchronous hot paths, and release blockers prevent calling the repository production-grade. |
| Architectural health | 4/10 | The crate DAG is clean, but `rawscope-render` has become a second domain layer and `WorkbenchApp` is a shared mutable god object coordinated through roughly fifty sibling modules. The apparent crate boundaries no longer match responsibility boundaries. |
| Idiomatic Rust quality | 6/10 | Ownership is generally straightforward, error enums are common, and there is no Rust `unsafe`. However, public structs routinely expose invariants, constructors panic on recoverable input, revisions are caller-controlled, and many APIs are merely implementation state made public. |
| Correctness risk | 4/10 | Confirmed defects include wrong synthetic row counts, CSV/Parquet semantic divergence, accepted-but-unloadable Float16 columns, vertically inverted timeline lanes, filtered evidence suppression, filter-blind linked selections, and partial GPU state commits. |
| Performance risk (10 = low risk) | 3/10 | Whole datasets are materialized into several redundant forms; timeline interaction rebuilds GPU resources and blocks on readback; exact scatter settling combines GPU synchronization with repeated full CPU scans; relief shading is exceptionally sample-heavy. Existing benchmarks do not cover the resident interactive path or memory/VRAM. |
| Maintainability | 4/10 | Seventeen production files are at or above roughly 400 lines, evidence versions duplicate large serializer trees, the workbench state surface is broad, and historical feature plans are larger than the maintained product documentation. |
| Test quality | 6/10 | The 371 passing Rust tests and 13 passing Python tests protect many deterministic contracts, and all 12 ignored GPU tests passed on the review machine. Important boundary cases and cross-component transactional guarantees remain untested. |
| Open-source readiness | 2/10 | The declared dual licence has no licence texts; Cargo packaging fails; Clippy fails under the requested warning policy; there is no CI, MSRV/toolchain policy, contribution/security/release process, or platform matrix; dependency policy is absent and the lockfile has four advisory findings. |

The largest structural weakness is that RawScope currently treats a single-threaded native UI coordinator as the integration boundary for ingestion, analytics, GPU scheduling, evidence construction, and filesystem export. That makes partial failure, UI stalls, repeated computation, and state divergence likely. The second weakness is evidence architecture: five scatter shapes and three timeline shapes are copied through serializer-specific DTO layers without a single validated canonical model or reader. The third is release engineering: this workspace cannot be legally or mechanically published in its current form.

Inspection covered all manifests and the lockfile, all 183 Rust files (40,516 lines), all 9 WGSL shaders (906 lines), all 12 Python source/test files (1,286 lines), public facades, tests, benches, schema/product/architecture documentation, historical feature plans, and the pre-existing working-tree diff. Important runtime paths were traced from CLI/session/dataframe input through ingestion, CPU analysis, GPU upload/dispatch/readback, interaction, selection, versioned evidence, and bundle IO.

The catalogue contains 143 findings: 11 Critical, 73 High, 50 Medium, 7 Low, and 2 Nit.

#### Verification record

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo check --workspace --all-targets --all-features` | Passed. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Failed at `crates/rawscope-data/src/local_dataset/parquet.rs:277` (`clippy::while-let-on-iterator`). |
| `cargo test --workspace --all-features` | Passed: 371 passed, 12 ignored. |
| Ignored scatter GPU test target | Passed: 9/9 on the local WGPU adapter. |
| Ignored timeline GPU test target | Passed: 3/3 on the local WGPU adapter. |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` | Passed. This does not enforce `missing_docs`. |
| `cargo check --release --workspace --all-features` | Passed. |
| `python -m unittest discover -s sdk/python/tests -v` | Passed: 13/13 with pandas, Polars, and PyArrow installed. |
| `cargo package --workspace --allow-dirty --no-verify` | Failed after packaging `rawscope-core`: `rawscope-data` depends on `rawscope-core` without a version requirement. Cargo also warned about missing package metadata. |
| `cargo deny check` | Failed: four advisory findings and no configured licence allow-list. Bans and sources checks passed using cargo-deny's defaults. |
| Python sdist/wheel build | Not validated. `python -m build` was unavailable and the fallback wheel command found no `setuptools` in the isolated environment; dependencies were not installed merely to make the audit pass. |
| `cargo audit` | Not run because `cargo-audit` is not installed. `cargo-deny` supplied the available advisory check. |
| Cross-platform compilation | Not run because only `x86_64-pc-windows-msvc` is installed and the repository declares no support matrix. |
| Benchmarks | Not run. The benchmark source and claim policy were inspected; no benchmark result is inferred from compilation or tests. |
| Native GUI/manual visual pass | Not run. This was a source/API/readiness audit, and no manual window exercise was requested. |

### 2. Architecture map

#### Workspace and dependency flow

The workspace has six packages:

| Package | Intended responsibility | Actual responsibility |
| --- | --- | --- |
| `rawscope-core` | Dependency-free identifiers, ranges, density primitives, shared selection contract | Small and dependency-free as intended, but exposes invalid states and panicking construction. |
| `rawscope-data` | Dataset identity, local CSV/Parquet ingestion, profiles, filters, projections, synthetic records | Also owns retained string tables, Parquet chunk metadata, evidence-key validation, and several UI-oriented catalogue contracts. Its Parquet loader buffers all Arrow batches before conversion, then discards the arrays rather than exposing a true columnar store. |
| `rawscope-gpu` | WGPU adapter/device/surface setup | Appropriately narrow, but has no fallback-adapter or device-loss strategy. |
| `rawscope-render` | CPU/GPU density and presentation | In practice owns rendering, analytics, brushing, axes, missingness, dataset diff, inspection, comparison, evidence domain models, five scatter artifact versions, three timeline versions, and formatting. It is a broad domain/application crate, not a render crate. |
| `rawscope-session` | Session manifest parsing and resolution | The layer is narrow, but it depends on all of `rawscope-data` merely to obtain profile identifiers and parsing. |
| `rawscope-workbench` | Native UI and orchestration | Owns nearly all runtime state plus loading, scheduling, analytics refresh, selection publication, evidence version promotion, report writing, and error recovery across roughly fifty modules. |

Actual dependency graph:

```text
rawscope-core
    ^
    |
rawscope-data <------ rawscope-session
    ^  ^                  ^
    |  |                  |
    |  +---- rawscope-render ---- rawscope-gpu
    |              ^                ^
    +--------------|----------------+
                   |
           rawscope-workbench
```

There are no circular Cargo dependencies. The important implicit cycles are state cycles inside the workbench: filters affect masks, renderers, summaries, inspection, point reveal, selection, evidence, title/UI state, and exports; each concern mutates fields on the same `WorkbenchApp` through sibling `impl` blocks.

#### Important runtime paths

1. **Local scatter load:** CLI/session resolution -> optional profile schema read -> a second full CSV/Parquet load -> temporary full `RecordBatch` vector for Parquet -> retained per-cell `String`s + visual `ScatterPointRecord`s + chunk metadata -> main scatter GPU state + two more point buffers for difference density -> CPU summaries/aggregate/inspection.
2. **Scatter interaction:** winit pointer event -> viewport mutation -> reproject last field -> scheduled preview/exact compute -> exact GPU readback -> marginal scan -> active inspection scan -> optional baseline inspection scan/difference distribution -> point-reveal selection -> redraw.
3. **Timeline interaction:** every wheel/pan event -> CPU marginal/overview work -> full timeline compute resource creation -> GPU dispatch -> indefinite mapped-buffer wait -> renderer bind-group replacement -> redraw.
4. **Evidence:** finalized brush -> independent CPU scans for summary, drilldown, linked selection, v1 evidence, comparison, and aggregate context -> v2/v3/v4/v5 copying -> JSON/Markdown formatting -> sequential filesystem writes.
5. **Python bridge:** dataframe adapter -> whole-column evidence-key validation -> Parquet file write -> atomic manifest write -> `subprocess.Popen` of the native workbench.

#### State, concurrency, and low-level boundaries

`WorkbenchApp` is the sole application state owner and runs on winit's event thread. There is no task executor, worker pool, cancellation token, job generation object, or asynchronous completion queue. WGPU initialization is blocked through `pollster`; readback uses `Device::poll(wait_indefinitely)` and an `mpsc` receive. The functions labelled `async` do not provide non-blocking execution. Python launches a separate process but does not introduce in-process concurrency.

No Rust `unsafe` block, unsafe function, unsafe trait implementation, FFI boundary, raw-pointer operation, `transmute`, or manual `Send`/`Sync` implementation exists in the reviewed Rust source. The low-level safety boundary is nevertheless real: numerous `#[repr(C)] + bytemuck::Pod/Zeroable` host structs are manually mirrored in WGSL. `Pod` proves Rust-side bit validity and padding constraints; it does not prove host/shader field offsets, address-space layout, binding minimum sizes, or semantic agreement.

#### Low-level invariant audit

| Boundary | Exact invariant | Documentation/caller assessment | Safer/harder-to-misuse direction |
| --- | --- | --- | --- |
| `#[repr(C)] + Pod/Zeroable` upload structs (`crates/rawscope-render/src/gpu_scatter_density_pack.rs:7-45`, `crates/rawscope-render/src/gpu_timeline_density_pack.rs:9-46`, render-param structs) | Every byte must be initialized and plain-data-valid; Rust offsets/size/alignment must exactly match the corresponding WGSL struct and binding address-space rules. | Derives establish Rust-side `Pod`; callers use `bytes_of`/`cast_slice` correctly for those types. WGSL agreement is manual and mostly undocumented; only the 144-byte relief/render struct has a total-size test. | Centralize ABI declarations, assert every offset/size, set binding minimum sizes, and add semantic field-mapping GPU tests. |
| `bytemuck::cast_slice::<u8, u32>` readback (`crates/rawscope-render/src/gpu_density_pipeline.rs:148-173`) | The mapped byte length must be a multiple of four, suitably aligned for u32, contain at least `grid_bin_count` elements, remain mapped and immutably borrowed during the read, and be unmapped only after the view drops. | Lexical scope drops the mapped view before `unmap`, and WGPU mapping success is checked. Buffer-size/grid-count agreement is convention-based and vulnerable to unchecked dimension arithmetic; there is no explicit assertion before slicing. | Carry validated byte/element counts in one buffer object, check mapped length, and decode with an explicit bounded view while completing mapping asynchronously. |
| Storage-buffer indexing/atomics in `crates/rawscope-render/src/shaders/scatter_density.wgsl` and `crates/rawscope-render/src/shaders/timeline_density.wgsl` | Uploaded record/mask arrays, dispatch ranges, params, and count-buffer dimensions must agree; every computed bin index must be below the allocated count length; counters must stay within u32. | Dispatch shaders guard record count and mask alignment has a Rust error path. Dimension/device-limit and CPU/WGSL numerical agreement are not fully validated; non-finite scatter values and timeline arithmetic violate local proof. | Accept only validated finite records/configs, use checked shared bin formulas and device limits, and keep parity tests at extrema. |
| WGPU resource lifetime | Buffers/bind groups/pipelines must outlive encoded/submitted uses; resources must not be remapped while GPU uses them; device/surface generation must match renderer generation. | Rust/WGPU ownership handles ordinary lifetime/aliasing safely. Logical generation/device-loss invariants are not modelled, so stale or partially replaced resources remain a reliability risk rather than Rust UB. | Own resources in immutable device/dataset generations and atomically swap complete renderer states after successful creation. |

The architecture document describes a narrow milestone slice, but its own `rawscope-render` responsibility paragraph spans GPU compute, presentation, analytics, evidence, missingness, formatting, and brushing (`docs/ARCHITECTURE.md:52`). The implementation has already outgrown that description.

### 3. Complete findings catalogue

Effort uses **S** (hours to roughly two days), **M** (several days), **L** (one or more weeks), and **XL** (multi-stage programme). Scope is local, cross-cutting, or architectural.

Classification convention: statements that code “does/is” describe observed implementation or a reproducible check result; statements that it “can/may” describe a reachable risk whose trigger was established by code tracing but not exercised during this no-change audit. Subjective boundary recommendations are identified as architecture/API debt rather than presented as runtime defects.

#### Release, legal, dependencies, and tooling

#### RS-001 — Critical — Legal / licensing

- **Location / symbol:** `Cargo.toml:12-15`, `[workspace.package]`; repository root, expected `LICENSE-MIT` and `LICENSE-APACHE` (absent).
- **Finding:** Every package declares `MIT OR Apache-2.0`, but neither licence text is present. A metadata string is not a grant accompanied by the required licence terms.
- **Evidence and consequence:** Consumers cannot reliably determine or satisfy redistribution obligations, and crates.io/GitHub publication would present a legally incomplete project.
- **Correction:** Add canonical MIT and Apache-2.0 texts, identify copyright ownership, and add a README licence section; make the Python package include the same files in sdists/wheels.
- **Effort / scope:** S; repository-wide release blocker.

#### RS-002 — Critical — Cargo packaging

- **Location / symbol:** `crates/rawscope-data/Cargo.toml:7-12` and every internal `path = ...` dependency in `crates/rawscope-render/Cargo.toml:9-11`, `crates/rawscope-session/Cargo.toml:8`, and `apps/rawscope-workbench/Cargo.toml:13-17`.
- **Finding:** Internal path dependencies have no `version`, so the workspace is not publishable.
- **Evidence and consequence:** `cargo package --workspace --allow-dirty --no-verify` packages `rawscope-core`, then fails on `rawscope-data`: dependency `rawscope-core` lacks a version requirement. No downstream crate can be packaged in dependency order.
- **Correction:** Decide which crates are public. Add matching `version = "0.1.0"` beside publishable path dependencies, set `publish = false` on private crates/apps, and validate each publishable package in dependency order.
- **Effort / scope:** S; cross-cutting release blocker.

#### RS-003 — Critical — Continuous integration

- **Location / symbol:** `Cargo.toml:1-18`; repository root `.github/workflows/` (absent).
- **Finding:** There is no CI at all.
- **Evidence and consequence:** Formatting, compilation, Clippy, tests, Rustdoc, Python compatibility, licence/advisory policy, packaging, and platform support can regress without any merge gate. The current Clippy and packaging failures demonstrate that this is not theoretical.
- **Correction:** Add a required CI workflow with pinned actions, a supported OS/Rust/Python matrix, cache discipline, `fmt`, `check`, `clippy -D warnings`, tests, Rustdoc, packaging, and configured `cargo-deny`; keep GPU hardware tests in a clearly separate opt-in job.
- **Effort / scope:** M; repository-wide.

#### RS-004 — High — Warning policy

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/parquet.rs:270-285`, `read_parquet_batches`.
- **Finding:** The repository fails its requested production Clippy policy because an iterator is manually consumed with `while let Some(batch) = reader.next()`.
- **Evidence and consequence:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` fails on `clippy::while-let-on-iterator`. A public repository that documents strict Rust quality cannot merge cleanly under its own review command.
- **Correction:** Express the traversal as a `for batch in reader` loop and make the exact Clippy invocation a required CI check.
- **Effort / scope:** S; local, but the missing gate is cross-cutting.

#### RS-005 — High — Supply-chain advisories

- **Location / symbol:** `Cargo.lock:2227-2232` (`paste 1.0.15`), `Cargo.lock:2426-2431` (`quick-xml 0.39.4`), `Cargo.lock:3200-3205` (`ttf-parser 0.25.1`); dependency roots `crates/rawscope-data/Cargo.toml:8-12` and `crates/rawscope-gpu/Cargo.toml:9-10`.
- **Finding:** The lockfile triggers four cargo-deny findings: RUSTSEC-2024-0436 (`paste`, unmaintained), RUSTSEC-2026-0194 and RUSTSEC-2026-0195 (`quick-xml`, quadratic attribute parsing and namespace-reader memory exhaustion), and RUSTSEC-2026-0192 (`ttf-parser`, unmaintained).
- **Evidence and consequence:** `paste` arrives through `parquet`; `quick-xml` is an all-target Wayland build/proc-macro dependency through `winit`; `ttf-parser` is a Linux decoration/font dependency through `winit`. The `quick-xml` path is not an obvious untrusted runtime parser in RawScope, but vulnerable/unmaintained locked components still undermine reproducible release review.
- **Correction:** Upgrade the direct roots to versions resolving the advisories where available; otherwise record narrow, expiry-dated exceptions with reachability justification in `deny.toml` and track upstream replacements.
- **Effort / scope:** M; dependency-wide.

#### RS-006 — High — Licence and dependency policy

- **Location / symbol:** `Cargo.toml:17-18`; repository root `deny.toml` (absent).
- **Finding:** `cargo-deny` is installed but has no repository policy.
- **Evidence and consequence:** `cargo deny check` uses defaults and fails the licence phase because no licences are explicitly allowed; sources and duplicate-version expectations are not project decisions. The command cannot function as a stable release gate.
- **Correction:** Add reviewed licence allow/exception lists, source restrictions, advisory policy, duplicate rules, and exception expiry/ownership to `deny.toml`.
- **Effort / scope:** S-M; repository-wide.

#### RS-007 — High — Package metadata and publication intent

- **Location / symbol:** `Cargo.toml:12-15`; package sections in all six `Cargo.toml` files (`:1-5`).
- **Finding:** Rust packages inherit only version, edition, and licence. They lack `description`, `repository`, `homepage`, package `readme`, `keywords`, `categories`, and `rust-version`; the GUI application is not marked `publish = false`.
- **Evidence and consequence:** Cargo emitted missing-metadata warnings during packaging. Crates are undiscoverable, their support contract is unspecified, and an application package can be published accidentally.
- **Correction:** Define shared metadata, explicitly mark private packages, include licence/readme files, and add crate-specific descriptions/categories.
- **Effort / scope:** S; cross-cutting.

#### RS-008 — High — Toolchain and MSRV contract

- **Location / symbol:** `Cargo.toml:12-15`; repository root `rust-toolchain.toml` (absent).
- **Finding:** There is neither a pinned development toolchain nor a declared minimum supported Rust version.
- **Evidence and consequence:** The review happened on Rust 1.95.0, but contributors and CI have no reproducible compiler choice and consumers cannot know what is supported. Dependency updates can silently raise the MSRV.
- **Correction:** Declare `rust-version`, document the MSRV policy, pin the contributor/CI toolchain (with required components/targets), and test both MSRV and stable when those differ.
- **Effort / scope:** S-M; repository-wide.

#### RS-009 — Medium — Platform support

- **Location / symbol:** `README.md:81-112`; `crates/rawscope-gpu/src/context.rs:38-59`; absent CI matrix.
- **Finding:** Build/run instructions are PowerShell-first, only Windows was validated, and no supported OS/GPU/backend matrix is declared despite `winit`/WGPU pulling substantial Linux/macOS platform code.
- **Evidence and consequence:** Linux-specific dependencies are where three advisory findings appear, yet those targets are not compiled in normal validation. Unknown consumers cannot distinguish supported platforms from incidental compilability.
- **Correction:** State supported OS/architecture/GPU backends and test them in CI; give shell-neutral commands plus platform-specific prerequisites and known limitations.
- **Effort / scope:** M; cross-cutting.

#### RS-010 — High — Release engineering

- **Location / symbol:** `README.md:76-78,203`; repository root `CHANGELOG.md` and release workflow (absent).
- **Finding:** There is no release process, changelog, tag/version policy, binary packaging, checksums, SBOM/provenance, signing, installer/archive strategy, or rollback guidance.
- **Evidence and consequence:** Even after source correctness work, maintainers cannot reproducibly turn a commit into consumable native artifacts or communicate compatibility changes to evidence schemas and APIs.
- **Correction:** Define versioning and schema compatibility, add a reviewed release checklist/workflow, build per-platform archives, publish checksums/SBOM/provenance, and maintain a changelog.
- **Effort / scope:** L; repository-wide.

#### RS-011 — High — Contributor and security governance

- **Location / symbol:** repository root; `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, issue templates, and pull-request template (all absent).
- **Finding:** External contributors have no supported workflow, disclosure channel, conduct expectations, review checklist, or issue taxonomy.
- **Evidence and consequence:** A competent outsider cannot know development prerequisites, test expectations, architectural constraints, security contact, or release ownership without private knowledge embedded in agent-only files and historical plans.
- **Correction:** Add concise public governance documents and templates derived from the actual repository commands and architecture rules; do not expose private automation instructions as a substitute.
- **Effort / scope:** M; repository-wide.

#### RS-012 — Medium — Feature and build-cost design

- **Location / symbol:** `crates/rawscope-data/Cargo.toml:7-12`, `crates/rawscope-gpu/Cargo.toml:7-10`, `crates/rawscope-render/Cargo.toml:7-14`, `apps/rawscope-workbench/Cargo.toml:7-22`.
- **Finding:** There are no Cargo features. Arrow/Parquet, WGPU/windowing, evidence serialization, and native UI are always compiled in every crate that depends on their owner.
- **Evidence and consequence:** `rawscope-session` transitively builds Arrow/Parquet; headless/data consumers cannot avoid GPU/UI cost; platform feature surfaces are uncontrolled. Build time and supply-chain exposure grow with unrelated capabilities.
- **Correction:** First fix crate boundaries; then introduce a small, deliberate feature matrix (for example Parquet ingestion and native presentation) with minimal defaults. Do not use features to conceal an incorrect dependency direction.
- **Effort / scope:** L; architectural.

#### RS-013 — Medium — Duplicate transitive dependencies

- **Location / symbol:** `Cargo.lock` package sets for `foldhash`, `getrandom`, `hashbrown`, `itertools`, `log`, `rustc-hash`, and `windows-sys`; direct roots in the workspace manifests.
- **Finding:** The current Windows-target graph contains seven duplicated package names; all-target analysis adds more platform-version duplication.
- **Evidence and consequence:** Duplicate generic collections/runtime crates increase compile time and binary size and can create type incompatibilities across public boundaries. Some duplication is ecosystem-driven, but it is not governed or measured.
- **Correction:** Record accepted duplicates in `deny.toml`, align direct dependency families where feasible, and measure before forcing risky transitive upgrades.
- **Effort / scope:** M; dependency-wide.

#### RS-014 — Low — Formatting and lint configuration

- **Location / symbol:** repository root `rustfmt.toml` and `clippy.toml` (absent); `Cargo.toml:1-18`.
- **Finding:** Formatting and lint behavior relies entirely on whichever toolchain a contributor happens to use.
- **Evidence and consequence:** With no pinned toolchain, formatting diffs and lint interpretation can change across machines. The current Clippy failure has no repository-level allow/deny rationale.
- **Correction:** Pin the toolchain first; add configuration only for deliberate non-default policy, and document every broad lint exception.
- **Effort / scope:** S; repository-wide.

#### RS-015 — Medium — Python distribution metadata

- **Location / symbol:** `sdk/python/pyproject.toml:1-22`, `[project]` and setuptools configuration.
- **Finding:** The Python package has no project readme reference, authors/maintainers, project URLs, classifiers, licence files, or explicit package-data policy.
- **Evidence and consequence:** Wheel/sdist consumers receive an under-described package with no source/support links and no demonstrated inclusion of the dual-licence texts. The repository has no Python build/publish job.
- **Correction:** Complete PEP 621 metadata, include public docs/licences, add build-and-inspect checks, and define whether this bridge is published independently or only shipped with native releases.
- **Effort / scope:** S-M; SDK/release.

#### Core and public API design

#### RS-016 — High — Representable invalid states

- **Location / symbol:** `crates/rawscope-core/src/density.rs:7-23` (`GridSize`), `crates/rawscope-core/src/range.rs:5-64` (`F32Range`, `U64Range`), `crates/rawscope-core/src/selection.rs:22-38` (`CoreLaneRange`).
- **Finding:** Invariant-bearing public types expose all fields, so callers can construct zero grids, reversed/non-finite ranges, and empty/reversed lane ranges without constructors.
- **Evidence and consequence:** Downstream code subtracts, divides, indexes, and dispatches under assumptions that these values are valid. The constructors' assertions do not protect public struct literals or deserialization-like manual construction.
- **Correction:** Make fields private; provide checked `TryFrom`/`new -> Result` constructors and read-only accessors. Use `NonZeroU32` where it genuinely simplifies grid/lane invariants.
- **Effort / scope:** M; cross-cutting API change.

#### RS-017 — High — Panicking public construction

- **Location / symbol:** `crates/rawscope-core/src/density.rs:12-18`, `GridSize::new`; `crates/rawscope-core/src/range.rs:10-15,45-50`, range constructors; `crates/rawscope-core/src/selection.rs:27-35`, `CoreLaneRange::new`; `crates/rawscope-render/src/timeline_density_renderer.rs:38-49`.
- **Finding:** Public constructors use `assert!` for ordinary invalid input.
- **Evidence and consequence:** Library consumers cannot recover from malformed user/configuration data without preduplicating invariants, and a GUI/import path can terminate the process. This is not an internal impossible-state assertion.
- **Correction:** Return domain-specific errors from checked constructors; reserve assertions for private invariants already proven by typed boundaries.
- **Effort / scope:** M; cross-cutting semver break.

#### RS-018 — High — Contradictory selection discriminator

- **Location / symbol:** `crates/rawscope-core/src/selection.rs:43-83`, `VisualSelection` and `VisualSelectionGeometry`.
- **Finding:** `VisualSelection` stores both a public `kind` and public geometry enum. Callers can pair `ScatterRect` with `TimelineRect` geometry.
- **Evidence and consequence:** Consumers must decide which field is authoritative; serializers and future linked views can branch differently over the same selection.
- **Correction:** Remove the redundant discriminator or derive it from a single private geometry enum; expose constructor/accessor methods that keep IDs and sorted row IDs coherent.
- **Effort / scope:** S-M; cross-cutting API change.

#### RS-019 — High — Unchecked count arithmetic

- **Location / symbol:** `crates/rawscope-core/src/density.rs:20-23`, `GridSize::bin_count`; `crates/rawscope-core/src/density.rs:33-38`, `DensityBin::push`.
- **Finding:** Grid dimensions multiply unchecked into `usize`, and each bin increments a `u32` unchecked.
- **Evidence and consequence:** Large public dimensions can wrap in optimized builds and under-allocate; more than `u32::MAX` rows in one bin wraps the count while retaining more row IDs. Subsequent indexing, GPU sizing, and evidence counts become incorrect.
- **Correction:** Validate dimensions with `checked_mul` before allocation/dispatch and use a count type consistent with supported dataset scale or checked/saturating accumulation with explicit overflow errors.
- **Effort / scope:** M; cross-cutting correctness.

#### RS-020 — Medium — Fabricated singleton extents

- **Location / symbol:** `crates/rawscope-core/src/range.rs:27-35,62-75`, `from_bounds_expanded`.
- **Finding:** Observing one value changes the reported domain to neighbouring values (`12` becomes `11..13`; an `f32` gets an arbitrary epsilon halo).
- **Evidence and consequence:** The same type represents both observed evidence extents and display-safe non-zero domains. Exports can therefore claim values that were not observed, while binning code still depends on non-zero spans.
- **Correction:** Separate `ObservedExtent` (which permits equality) from `NonEmptyDomain`/viewport ranges, and perform display expansion explicitly at the presentation boundary with disclosure.
- **Effort / scope:** M; cross-cutting domain-model change.

#### RS-021 — Medium — Evidence-key descriptor can lie

- **Location / symbol:** `crates/rawscope-data/src/evidence_key.rs:13-24`, `DatasetEvidenceKey`.
- **Finding:** A type documented as validated has public `column_name` and `column_index` fields and no private proof of association with a specific table.
- **Evidence and consequence:** Callers can construct a descriptor whose name and index disagree or reuse it against another table; `value` trusts the index and can return the wrong source field.
- **Correction:** Make construction private to validation, store a typed column index/name pair behind accessors, and document/table-bind the lifetime or revalidate when crossing datasets.
- **Effort / scope:** S-M; local API with downstream updates.

#### RS-022 — Medium — Row-index identity is exposed but implicit

- **Location / symbol:** `crates/rawscope-data/src/local_dataset.rs:88-107`, `LoadedSourceRow`, `LoadedSourceTable::row`.
- **Finding:** Public rows/columns allow arbitrary mutation, while lookup assumes `RowId(n)` is exactly at vector index `n` and then silently returns `None` if the public state violates that invariant.
- **Evidence and consequence:** The same table is used for evidence, filters, drilldown, and session validation. One malformed consumer-created table produces missing or mismatched evidence rather than an explicit construction error.
- **Correction:** Make table storage private, validate contiguous IDs once in a constructor, expose iterators/indexed access, and use a separate keyed representation if non-contiguous IDs are supported.
- **Effort / scope:** M; cross-cutting API change.

#### RS-023 — Medium — Public revision values are not capabilities

- **Location / symbol:** `crates/rawscope-data/src/dataset_filter.rs:47-88`, `FilterRevision`, `FilterSet`; `crates/rawscope-render/src/scatter_density_gpu_lifecycle.rs:13-59`.
- **Finding:** Revisions are public caller-controlled integers and are used as proof that GPU data is unchanged.
- **Evidence and consequence:** A caller can alter filters without bumping `revision`, or reuse a dataset revision with different points. The GPU lifecycle then returns early and renders stale data.
- **Correction:** Keep revisions private and derive them from owner-controlled mutations, or compare content identities/generation tokens minted only by the owning store.
- **Effort / scope:** M; cross-cutting state-contract change.

#### RS-024 — Medium — Boolean/string primitive modelling

- **Location / symbol:** `crates/rawscope-data/src/dataset_filter.rs:9-21`, `DatasetFilter`; `crates/rawscope-session/src/manifest.rs:129-153`, session dataset/view strings.
- **Finding:** Missing-value policy is repeated as a boolean and important field/profile/display values are plain strings with normalization rules scattered across layers.
- **Evidence and consequence:** Call sites obscure intent (`true` means include missing), and Python/Rust normalization already diverges. Invalid or whitespace-distinct identifiers propagate until a later lookup.
- **Correction:** Introduce a small `MissingValuePolicy` enum and validated newtypes for non-blank column/display identifiers at external boundaries; avoid speculative traits.
- **Effort / scope:** M; cross-cutting API cleanup.

#### RS-025 — Nit — Top-value tie policy is accidental

- **Location / symbol:** `crates/rawscope-render/src/timeline_brush.rs:141-180,327-333`, `top_event_type`, `top_event_kind`, `top_lane`.
- **Finding:** `Iterator::max_by_key` makes equal-count ties resolve to the later iterator entry/highest lane, but no contract or label discloses that policy.
- **Evidence and consequence:** Reordering enum presentation or lane iteration changes evidence summaries despite equal data, weakening deterministic interpretation.
- **Correction:** Define and test a stable tie-break rule (for example lowest lane and explicit enum rank), or return all tied leaders.
- **Effort / scope:** S; local.

#### Data generation, ingestion, filtering, and profiling

#### RS-026 — High — Synthetic event count is wrong for small inputs

- **Location / symbol:** `crates/rawscope-data/src/synthetic/event.rs:62-75,88-120`, `generate_synthetic_events`.
- **Finding:** Every pattern count is forced to at least one before subtracting from `row_count`. For `row_count` 0 or 1, the spike and stale-lane loops still emit two records.
- **Evidence and consequence:** The returned identity/metadata claims the requested row count while `events.len()` is larger; row IDs can exist outside the declared dataset. Tests and demos using small boundary inputs receive internally contradictory data.
- **Correction:** Partition exactly `row_count` records using bounded allocations whose sum is the requested count, and add regression cases for 0, 1, 2, and 3 rows.
- **Effort / scope:** S; local correctness fix.

#### RS-027 — High — Synthetic time-range configuration is not actually supported

- **Location / symbol:** `crates/rawscope-data/src/synthetic/event.rs:10-19,94-129,149-187`, `sample_time_outside_gap`, `sample_window_time`; `crates/rawscope-data/src/synthetic/rng.rs:36-44`.
- **Finding:** Pattern windows are hard-coded to the default `0..1000` domain, and `sample_window_time` calls a panicking non-empty RNG range even when a configured range does not intersect a pattern. Inclusive maxima are converted with unchecked `max + 1`.
- **Evidence and consequence:** A valid `SyntheticEventConfig` with a narrow/custom time range can panic; a range ending at `u64::MAX` can overflow in debug or wrap in release. The public configuration is therefore misleading.
- **Correction:** Validate/normalize pattern windows against the configured domain, use inclusive sampling or checked upper-bound conversion, and return a configuration error rather than panic.
- **Effort / scope:** S-M; local API/correctness.

#### RS-028 — High — CSV `limit = 0` loads one row

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/csv.rs:215-232`, `read_csv_table`.
- **Finding:** The loop pushes a record before testing whether `rows.len() >= max_rows`.
- **Evidence and consequence:** The public loaders accept `Some(0)` and load one row, violating the limit contract. CLI/session validation happens elsewhere and does not protect library consumers.
- **Correction:** Reject zero at the loader boundary or check the limit before reading/pushing; share a typed positive-row-limit across CLI, session, Python, and data APIs.
- **Effort / scope:** S; local with cross-boundary contract alignment.

#### RS-029 — High — CSV timestamp inference rejects valid `u64` data

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/csv.rs:248-287`, `infer_column_kind`; `crates/rawscope-data/src/local_dataset/csv.rs:312-322`, `parse_u64_cell`.
- **Finding:** Schema inference recognises integers only by parsing `i64`, while the timeline parser accepts `u64`.
- **Evidence and consequence:** A legitimate timestamp above `i64::MAX` is inferred as float or string and rejected by the integer binding check before the `u64` parser can accept it.
- **Correction:** Infer a signed/unsigned integer domain explicitly, or parse the target binding with its target type rather than relying on a lossy global summary enum.
- **Effort / scope:** M; local ingestion contract.

#### RS-030 — High — CSV evidence silently rewrites source data

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/csv.rs:163-176`, `source_table`.
- **Finding:** Every retained source cell is trimmed before storage.
- **Evidence and consequence:** Evidence/drilldown cannot reproduce the original CSV value; identifiers and categories that differ only by leading/trailing whitespace collapse visually. Parquet retains those spaces, so identical logical data has format-dependent evidence.
- **Correction:** Preserve the raw decoded cell for evidence and maintain a separate normalized value for parsing/filtering. Document CSV decoding and normalization explicitly.
- **Effort / scope:** M; cross-cutting evidence/ingestion change.

#### RS-031 — High — Local ingestion is eager and memory-amplifying

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/csv.rs:215-245`; `crates/rawscope-data/src/local_dataset/parquet.rs:238-290`; `crates/rawscope-data/src/local_dataset.rs:115-142`.
- **Finding:** CSV stores every `StringRecord` before conversion. Parquet collects every `RecordBatch` before walking it. Both then allocate visual records and a full per-cell `String` table; Parquet also constructs chunk/schema metadata.
- **Evidence and consequence:** Peak memory is the source decoder representation plus all visual records plus all evidence strings, followed in the app by projection copies and several GPU point buffers. Large-data positioning is not supported by this implementation and OOM occurs before incremental progress is possible.
- **Correction:** Introduce a chunk/stream ingestion pipeline with explicit retained-column policy, bounded profiling, and a stable row store. Upload/process chunks incrementally and retain raw evidence only according to a documented storage budget.
- **Effort / scope:** XL; architectural.

#### RS-032 — High — Parquet Float16 passes validation but cannot load

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/parquet.rs:304-355`, `loaded_column_kind`/`ensure_numeric_column`; `crates/rawscope-data/src/local_dataset/parquet.rs:421-489`, `numeric_cell_as_f32`.
- **Finding:** Float16 is classified and accepted as numeric, but the cell converter has no Float16 branch and returns `UnsupportedParquetColumnType`.
- **Evidence and consequence:** Profile/schema validation can succeed and the subsequent data load fail on the same column. The Python Arrow validator also accepts Float16, so bridge-created sessions fail only after launch.
- **Correction:** Either implement checked Float16 conversion and source formatting or reject Float16 consistently in schema validation and Python adapters. Add an end-to-end regression fixture.
- **Effort / scope:** S-M; cross-language boundary.

#### RS-033 — High — Unsupported Parquet values are fabricated as type labels

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/parquet.rs:293-318`, `loaded_column_kind`; `crates/rawscope-data/src/local_dataset/parquet.rs:679-758`, `display_array_value`.
- **Finding:** Every unrecognised Arrow type is reported as `LoadedColumnKind::String`, while retained evidence uses a placeholder such as `<Date32>` or `<List(...)>` for every non-null cell.
- **Evidence and consequence:** Dates, decimals, booleans, binaries, lists, structs, dictionaries, and timestamps lose their actual value but appear to be successfully retained. Exports can contain fabricated repeated type strings and be mistaken for source evidence.
- **Correction:** Model `Unsupported(DataType)` separately, implement explicit value formatting for supported types, and fail loading/evidence retention for unsupported values rather than invent content.
- **Effort / scope:** L; ingestion/evidence redesign.

#### RS-034 — High — Numeric ingestion silently narrows to `f32`

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/csv.rs:290-309`, `parse_f32_cell`; `crates/rawscope-data/src/local_dataset/parquet.rs:421-493`, `numeric_cell_as_f32`.
- **Finding:** CSV decimal strings and all Arrow integer/f64 values are converted to `f32` without a precision-loss check or disclosure.
- **Evidence and consequence:** Integers above 2^24 and close f64 values collapse to the same coordinate, changing bins, selections, comparisons, and evidence extents. Very large finite f64 values become non-finite and are rejected despite being valid source data.
- **Correction:** Keep analytical coordinates as f64 or a typed source numeric representation through CPU semantics; define and test an explicit GPU normalization/quantization boundary and disclose it in evidence.
- **Effort / scope:** XL; architectural numerical contract.

#### RS-035 — Medium — Timeline lane normalization depends on file format

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/csv.rs:324-348`, `parse_lane_cell`/`cell_value`; `crates/rawscope-data/src/local_dataset/parquet.rs:589-664`, `lane_cell_as_string`.
- **Finding:** CSV lane values are trimmed through `cell_value`; Parquet UTF-8 lane values are preserved verbatim and blank/whitespace-only strings are accepted as lanes.
- **Evidence and consequence:** `"lane"` and `" lane "` are one lane in CSV but distinct lanes in Parquet; a blank Parquet lane succeeds where a blank CSV cell fails. Cross-format comparisons and profiles are not semantically stable.
- **Correction:** Define one lane-label normalization policy, preserve the raw source separately, and apply the policy identically at both adapters.
- **Effort / scope:** S-M; cross-format contract.

#### RS-036 — Medium — Parquet errors claim CSV row semantics

- **Location / symbol:** `crates/rawscope-data/src/local_dataset.rs:173-178,228-236`, `DatasetLoadError::InvalidColumnValue`; Parquet call sites `crates/rawscope-data/src/local_dataset/parquet.rs:421-665,768-769`.
- **Finding:** A shared invalid-value error always formats its location as `CSV row`, including errors raised while reading Parquet.
- **Evidence and consequence:** Diagnostics are factually wrong, complicate support, and obscure whether a row number refers to a text record or Arrow row offset.
- **Correction:** Carry a source format/location enum or distinct CSV/Parquet error variants and include the path/chunk/row context that a user can act on.
- **Effort / scope:** S; local diagnostics.

#### RS-037 — Medium — Duplicate column names are not rejected

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/csv.rs:217-222,185-193`; `crates/rawscope-data/src/local_dataset/parquet.rs:293-332`; `crates/rawscope-render/src/dataset_diff.rs:117-141`.
- **Finding:** Binding resolves the first matching name, while later map-based analytics collapse duplicate names.
- **Evidence and consequence:** Two physical columns can be silently conflated: visual bindings read one, source tables contain both, and dataset diff/profile maps report only one. Evidence column names become ambiguous.
- **Correction:** Reject duplicate names at the ingestion boundary or introduce stable column IDs and make name lookup explicitly ambiguous.
- **Effort / scope:** M; cross-cutting schema contract.

#### RS-038 — Medium — Evidence-key validation duplicates the key column

- **Location / symbol:** `crates/rawscope-data/src/evidence_key.rs:76-125`, `validate_evidence_key`.
- **Finding:** Validation clones every key into `HashMap<String, RowId>` even though every key already exists in the retained source table.
- **Evidence and consequence:** A high-cardinality identifier column incurs another O(N) allocation and string copy at activation; the Python adapters independently materialize and hash the whole column as well.
- **Correction:** Validate while ingesting or store interned/borrowed key representations in a table-owned uniqueness index; make the memory cost explicit and reusable.
- **Effort / scope:** L; cross-language/data-store change.

#### RS-039 — High — Filter invariants can be bypassed

- **Location / symbol:** `crates/rawscope-data/src/dataset_filter.rs:33-88`, `DatasetFilter::normalize`, `FilterSet`.
- **Finding:** `normalize` is private to mutation helpers, but `FilterSet.filters` and `revision` are public. Callers can insert unsorted category lists, duplicate filters, duplicate values, or mutate without a revision increment.
- **Evidence and consequence:** Category matching uses binary search and can return false results for a manually constructed unsorted list; GPU mask upload can be skipped because the stale revision appears current.
- **Correction:** Make fields private, expose validated mutations/iteration, and mint revisions internally only after semantic changes.
- **Effort / scope:** M; cross-cutting API correction.

#### RS-040 — High — Invalid numeric values are treated as missing

- **Location / symbol:** `crates/rawscope-data/src/dataset_filter.rs:200-218`, `ResolvedFilter::matches`.
- **Finding:** Failed parses and non-finite numeric values use `map_or(include_missing, ...)`, conflating invalid data with an empty cell.
- **Evidence and consequence:** Enabling “include missing” also includes `NaN`, `inf`, and malformed strings even though the field catalogue reports invalid and missing counts separately. Cohort counts and exported filter semantics are false.
- **Correction:** Model valid/missing/invalid as three states and give filters an explicit policy for each; align UI wording and evidence serialization.
- **Effort / scope:** M; cross-cutting semantic change.

#### RS-041 — High — Categorical profiling is unbounded before truncation

- **Location / symbol:** `crates/rawscope-data/src/visual_field_catalog.rs:141-176`, `summarize_categorical`.
- **Finding:** The profiler allocates a `BTreeMap<String, usize>` entry for every distinct category, converts the entire map to a vector, sorts it, and only then truncates to `max_category_values`.
- **Evidence and consequence:** A UUID-like column with millions of unique values creates O(N) cloned strings and O(N log N) work merely to display a bounded top list. The configuration bounds output, not resource use.
- **Correction:** Use a bounded heavy-hitter strategy plus an exact/approximate distinct counter, or explicitly reject/profile high-cardinality fields incrementally with a budget.
- **Effort / scope:** L; profiling architecture.

#### RS-042 — Medium — Malformed row semantics differ by subsystem

- **Location / symbol:** `crates/rawscope-data/src/visual_field_catalog.rs:110-129,148-158`; `crates/rawscope-data/src/dataset_filter.rs:160-170`; `crates/rawscope-render/src/missingness_reference.rs:124-146`; `crates/rawscope-render/src/dataset_diff.rs:125-141`.
- **Finding:** Catalogue code treats an absent cell as empty/missing; filter evaluation rejects a width mismatch; missingness and diff treat `None` as not missing.
- **Evidence and consequence:** The same malformed `LoadedSourceTable` yields incompatible missing counts or a hard error depending on which screen runs. Public table fields make this state constructible.
- **Correction:** Validate rectangular rows at table construction and define one missingness predicate over a typed cell representation.
- **Effort / scope:** M; cross-cutting data invariant.

#### RS-043 — Medium — Raw projection performs a full copy

- **Location / symbol:** `crates/rawscope-data/src/scatter_projection.rs:82-124,144-160`, `project_scatter_points`; `apps/rawscope-workbench/src/app_scatter_projection.rs:61-70`.
- **Finding:** Even `ScatterProjection::RawXY` allocates and copies every point; the workbench separately retains `raw_points = self.scatter.points.clone()`.
- **Evidence and consequence:** Loading a projection-capable dataset immediately duplicates all CPU point records, and switching back to raw allocates another full vector. This compounds the already duplicated source strings/GPU buffers.
- **Correction:** Retain one canonical point store and represent projection as derived coordinates in a reusable buffer/cache; return borrowed/copy-on-write raw data when no transform is needed.
- **Effort / scope:** L; cross-cutting data ownership.

#### RS-044 — High — Profile validation reads the dataset twice

- **Location / symbol:** `apps/rawscope-workbench/src/app_dataset_profile.rs:43-52,89-98,135-160`; `apps/rawscope-workbench/src/app.rs:247-259`; `apps/rawscope-workbench/src/app_timeline.rs:41-53`.
- **Finding:** Selecting a profile calls `load_dataset_schema`, then the normal preparation path loads the dataset again. For CSV, schema inference reads all limited rows; Parquet opens a separate reader.
- **Evidence and consequence:** Profile-backed startup doubles IO/parsing work before GPU initialization, exactly where the UI/event thread is blocked. Large CSV startup cost and memory churn are unnecessarily repeated.
- **Correction:** Load once into a staged dataset/schema handle, validate the profile against that schema, then continue conversion/streaming from the same ingestion plan.
- **Effort / scope:** L; ingestion/workbench architecture.

#### RS-045 — High — Local data is labelled with synthetic metadata

- **Location / symbol:** `apps/rawscope-workbench/src/app.rs:297-303`; `apps/rawscope-workbench/src/app_timeline.rs:82-90`; field declaration comment `apps/rawscope-workbench/src/app.rs:80-81`.
- **Finding:** Local datasets are assigned `SyntheticDatasetMetadata::new(0, row_count)` to satisfy legacy evidence.
- **Evidence and consequence:** Legacy exports can claim a deterministic synthetic seed for real CSV/Parquet data. This is an evidence-integrity defect, not merely awkward naming.
- **Correction:** Stop building synthetic-only evidence for local data; use a source-aware canonical metadata enum and make legacy export unavailable when its schema cannot represent the truth.
- **Effort / scope:** M; evidence/domain redesign.

#### RS-046 — Low — Local Parquet is logged as CSV

- **Location / symbol:** `apps/rawscope-workbench/src/app.rs:286-294`; `apps/rawscope-workbench/src/app_timeline.rs:71-80`.
- **Finding:** Both preparation logs hard-code “local CSV” even when the loader dispatches Parquet.
- **Evidence and consequence:** Operational diagnostics misidentify the input path and make format-specific failures/performance reports less actionable.
- **Correction:** Log a structured source-format field derived from `DatasetSource` and use format-neutral message text.
- **Effort / scope:** S; local diagnostics.

#### RS-047 — Medium — Columnar API promises data it does not retain

- **Location / symbol:** `crates/rawscope-data/src/local_dataset.rs:28-47`, `LoadedColumnarChunk`/`LoadedColumnarDataset`; `crates/rawscope-data/src/local_dataset/parquet.rs:238-290`.
- **Finding:** The public type is documented as “chunked local dataset structure retained alongside row-oriented view records,” but it contains only identity, schema, chunk IDs, offsets, and counts—not Arrow arrays or a way to load a chunk.
- **Evidence and consequence:** Consumers may reasonably expect a columnar data access path; the actual batches are discarded after eager conversion. The abstraction adds public surface without enabling streaming or columnar analytics.
- **Correction:** Rename it to explicit chunk metadata or provide a real chunk store/reader interface with ownership and lifetime semantics.
- **Effort / scope:** M; public API design.

#### Low-level GPU correctness and performance

#### RS-048 — High — Host/WGSL ABI is manually mirrored and under-validated

- **Location / symbol:** representative structs `crates/rawscope-render/src/gpu_scatter_density_pack.rs:7-45`, `crates/rawscope-render/src/gpu_timeline_density_pack.rs:9-46`, `crates/rawscope-render/src/scatter_density_render_resources.rs:9-94`; bindings with `min_binding_size: None` in `crates/rawscope-render/src/gpu_density_pipeline.rs:60-94`, `crates/rawscope-render/src/density_render_pipeline.rs:9-31`, and `crates/rawscope-render/src/scatter_density_render_resources.rs:108-139`.
- **Finding:** There is no Rust `unsafe`, but low-level buffer correctness depends on separately maintained Rust and WGSL layouts. Most bindings declare no minimum size; only `ScatterDensityRenderParams` has a total-size/alignment test.
- **Evidence and consequence:** `Pod` prevents Rust padding/invalid-bit-pattern mistakes but cannot prove WGSL offsets or field semantics. A host/shader edit can silently read the wrong values or fail only under backend validation.
- **Correction:** Centralize each ABI contract, use `NonZeroU64` minimum binding sizes, add offset/size assertions for every host struct, and add meaningful GPU parity tests for field mapping—not just successful dispatch.
- **Effort / scope:** L; cross-cutting low-level boundary.

#### RS-049 — Critical — GPU readback blocks indefinitely

- **Location / symbol:** `crates/rawscope-render/src/gpu_density_pipeline.rs:148-173`, `readback_counts_from_buffer`.
- **Finding:** Readback calls `Device::poll(PollType::wait_indefinitely())` and then performs a blocking `mpsc::Receiver::recv()`.
- **Evidence and consequence:** When called from the native event thread, GPU/driver latency freezes input, painting, and window responsiveness with no timeout or cancellation. Device loss or a callback problem can produce an unbounded stall.
- **Correction:** Submit readback as an asynchronous job, retain the mapped buffer until callback completion, deliver results through an event-loop proxy/generation token, and implement timeout/device-loss cancellation. Avoid readback entirely for display normalization where a GPU reduction can remain resident.
- **Effort / scope:** L; architectural concurrency boundary.

#### RS-050 — Critical — Timeline updates rebuild the compute stack

- **Location / symbol:** `crates/rawscope-render/src/gpu_timeline_density.rs:210-294`, `dispatch_timeline_density`; `crates/rawscope-render/src/timeline_density_renderer.rs:151-187`, `update_density`.
- **Finding:** Every timeline update recreates output/readback buffers, shader module, bind-group layout, compute pipeline, dispatch parameter buffers, and bind groups, then reads the full grid back.
- **Evidence and consequence:** Pan/zoom uses this path on every pointer event. Pipeline creation and allocation dominate useful compute, produce allocator/driver churn, and combine with RS-049 to freeze interaction.
- **Correction:** Give timeline density a resident GPU state equivalent to scatter: persistent point/event buffers, pipeline/layouts, reusable count/max buffers, revisioned parameters, preview/exact scheduling, and non-blocking/no-readback interactive updates.
- **Effort / scope:** XL; architectural.

#### RS-051 — Medium — Clearing allocates and uploads a full zero vector

- **Location / symbol:** `crates/rawscope-render/src/gpu_density_pipeline.rs:47-54`, `clear_output_buffer`.
- **Finding:** Each clear allocates `Vec<u8>` equal to the output buffer and uploads it through the queue.
- **Evidence and consequence:** Repeated density updates add CPU allocation, memory zeroing, and bus/upload traffic proportional to grid size. WGPU already exposes encoder buffer clearing.
- **Correction:** Reuse an encoder and `clear_buffer`, or keep/reuse a bounded zero buffer only where backend constraints require it.
- **Effort / scope:** S; local performance fix.

#### RS-052 — Medium — `async` GPU APIs are synchronously blocking

- **Location / symbol:** `crates/rawscope-render/src/gpu_timeline_density.rs:131-176`, `gpu_timeline_density`/`gpu_timeline_density_on_device`; analogous scatter entry points in `crates/rawscope-render/src/gpu_scatter_density.rs:106-164`.
- **Finding:** Public functions are declared `async` but immediately call synchronous setup/dispatch and the indefinite blocking readback; the wrapper's only `.await` is another function with the same behaviour.
- **Evidence and consequence:** Callers may schedule these on an async runtime expecting cooperative progress, but the executor thread blocks. Cancellation cannot interrupt WGPU polling.
- **Correction:** Make synchronous semantics explicit now, or implement a real callback/future state machine that yields and supports cancellation. Do not retain fake async signatures.
- **Effort / scope:** M-L; cross-cutting API/concurrency.

#### RS-053 — Critical — Timeline lanes render upside down relative to interaction

- **Location / symbol:** `crates/rawscope-render/src/shaders/timeline_density.wgsl:36-58`, `bin_lane`; `crates/rawscope-render/src/shaders/timeline_density_render.wgsl:23-35,90-98`; `crates/rawscope-render/src/timeline_brush.rs:275-286`; `crates/rawscope-render/src/view_axes.rs:304-325`; `apps/rawscope-workbench/src/ui_plot_axes.rs:87-100`.
- **Finding:** Compute stores lane 0 in y-bin 0. The render shader maps top-of-screen fragments to high y bins, while brush hit-testing and axis labels map top-of-screen to lane 0.
- **Evidence and consequence:** The visible density for lane 0 appears at the bottom, but its label and selected brush region are at the top. Users can select or interpret a different lane from the one visibly carrying the events.
- **Correction:** Define one screen/data y convention and enforce it in compute, rendering, overlays, axes, and hit-testing. Add a GPU-backed regression with uniquely populated first/last lanes and screen-space selection assertions.
- **Effort / scope:** M; cross-cutting correctness.

#### RS-054 — High — GPU timeline binning loses timestamp precision

- **Location / symbol:** `crates/rawscope-render/src/shaders/timeline_density.wgsl:26-33`, `bin_time`; CPU references `crates/rawscope-render/src/aggregate_cache.rs:197-208` and `crates/rawscope-render/src/view_summaries.rs:232-243`.
- **Finding:** The shader converts `u32` offsets/spans to `f32`; CPU semantics use `f64`.
- **Evidence and consequence:** Above 2^24, adjacent offsets are not distinguishable as f32. CPU summaries/evidence and GPU pixels can assign a timestamp to different bins even though the advertised supported span is the full `u32` range.
- **Correction:** Use overflow-safe integer binning (for example widened multiply/divide or quotient/remainder decomposition) with the same formula on CPU and WGSL; reduce the supported span only if that contract is explicit.
- **Effort / scope:** M; cross-cutting numerical correctness.

#### RS-055 — High — Timeline lane binning can overflow

- **Location / symbol:** `crates/rawscope-render/src/shaders/timeline_density.wgsl:36-38`, `bin_lane`.
- **Finding:** `(lane * height) / lane_count` multiplies two `u32` values before division.
- **Evidence and consequence:** Valid high lane indices and grid heights can wrap and write to the wrong y bin. CPU code uses wider arithmetic/conversions and will disagree.
- **Correction:** Use a widened/overflow-safe mapping shared with CPU code, and validate grid/lane limits before dispatch.
- **Effort / scope:** S-M; local shader plus parity tests.

#### RS-056 — Medium — Far-after-range events produce the wrong error

- **Location / symbol:** `crates/rawscope-render/src/gpu_timeline_density_pack.rs:64-83`, `pack_event`.
- **Finding:** An event known to be after the viewport still converts its full offset to `u32`; a sufficiently distant event returns `TimeRangeTooWide` instead of using an irrelevant sentinel offset.
- **Evidence and consequence:** A supported viewport can fail merely because an off-screen event is far away. The error names the event offset as though it were the viewport span.
- **Correction:** Branch first: encode before/after sentinels without converting the full offset; only convert offsets for events inside the validated range.
- **Effort / scope:** S; local correctness.

#### RS-057 — Medium — Empty timeline input bypasses configuration validation

- **Location / symbol:** `crates/rawscope-render/src/gpu_timeline_density.rs:156-184`, `gpu_timeline_density_on_device`.
- **Finding:** The zero-event fast path returns before validating the timeline span or lane count. Zero dimensions still reach panicking `GridSize::new`, while lane_count zero can be accepted if dimensions are non-zero.
- **Evidence and consequence:** API validity depends on whether the input slice happens to be empty; callers cannot rely on one configuration contract.
- **Correction:** Validate all configuration first through a checked config type, then take the empty-data fast path.
- **Effort / scope:** S; local API correctness.

#### RS-058 — High — GPU sizes are not checked against arithmetic or device limits

- **Location / symbol:** `crates/rawscope-render/src/scatter_density_gpu_state.rs:129-171,215-222,265-266`; `crates/rawscope-render/src/scatter_difference_renderer.rs:281-294`; timeline allocation `crates/rawscope-render/src/gpu_timeline_density.rs:210-235`.
- **Finding:** Public grid dimensions flow into buffer creation and one-dimensional dispatch without checking WGPU storage-buffer/buffer-size/workgroup limits; several paths multiply `u32 * u32` before conversion.
- **Evidence and consequence:** Oversized but type-valid dimensions can overflow `bin_count`, understate a readback length, exceed device limits, or trigger a WGPU validation panic/uncaptured error rather than a domain `Result`.
- **Correction:** Validate checked bin/byte counts against `device.limits()` before allocation, tile reductions across legal workgroup dimensions, and return a typed resource-limit error.
- **Effort / scope:** L; cross-cutting GPU boundary.

#### RS-059 — High — WGPU validation/device-loss errors are not contained

- **Location / symbol:** `crates/rawscope-gpu/src/context.rs:42-66,124-158`; `crates/rawscope-gpu/src/compute.rs:17-39`; pipeline/buffer creation throughout `crates/rawscope-render/src`.
- **Finding:** There are no scoped validation errors, uncaptured-error callback policy, device-lost recovery path, or renderer reinitialization state machine. `Suboptimal` surface acquisition is treated as a normal presented frame without reconfiguration.
- **Evidence and consequence:** Backend validation and device loss can escape the typed error surface, terminate work, or leave stale renderers. Laptop GPU switching/suspend and backend-specific limits are normal production events.
- **Correction:** Install structured uncaptured/device-lost handling, use error scopes around fallible resource rebuilds, reconfigure suboptimal surfaces, and model renderer recovery/reload as an explicit state transition.
- **Effort / scope:** L-XL; architectural reliability.

#### RS-060 — High — Non-finite public point data diverges between CPU and GPU

- **Location / symbol:** `crates/rawscope-render/src/gpu_scatter_density_pack.rs:49-56`, `pack_points`; shader `crates/rawscope-render/src/shaders/scatter_density.wgsl:31-63`; CPU reference `crates/rawscope-render/src/density_reference.rs:7-25`.
- **Finding:** Public `ScatterPointRecord` values are uploaded without finite validation. CPU comparisons make NaN fail range containment and skip the point; WGSL range comparisons with NaN are false, so the shader proceeds to `floor` and float-to-uint conversion.
- **Evidence and consequence:** Invalid public input can be binned unpredictably on GPU while CPU summaries omit it. WGPU conversion semantics are not an evidence contract.
- **Correction:** Validate/construct finite visual records once at the data boundary and still defensively reject non-finite values in the shader using `isNan`/finite-range checks where available.
- **Effort / scope:** M; cross-cutting domain/GPU correctness.

#### RS-061 — High — Density transitions cannot represent previous grid configuration

- **Location / symbol:** `crates/rawscope-render/src/scatter_density_render_resources.rs:9-94`; shader `crates/rawscope-render/src/shaders/scatter_density_render.wgsl:141-162,222-277,280-313`; update `crates/rawscope-render/src/scatter_density_renderer.rs:207-253`.
- **Finding:** Render params contain only current `grid_width/grid_height` and current relief parameters. Previous counts are indexed with current dimensions, and previous relief uses current relief radius/light/strength.
- **Evidence and consequence:** Crossfades across preview/exact grid sizes or relief changes sample the previous buffer with the wrong layout/config. Grid reallocation can also discard the very previous field being advertised.
- **Correction:** Carry complete previous field dimensions and presentation parameters, retain a valid prior buffer until transition completion, or disable crossfade for incompatible resource generations.
- **Effort / scope:** L; cross-cutting renderer architecture.

#### RS-062 — High — Relief shading has extreme fragment sample amplification

- **Location / symbol:** `crates/rawscope-render/src/shaders/scatter_density_render.wgsl:141-185,222-277`, `topographic_intensity`/`relief_colour`.
- **Finding:** One reconstructed sample performs four count loads; topographic intensity takes nine reconstructed samples. Relief evaluates it for the centre, eight gradient points, and eight horizon steps—well over one hundred storage-buffer reads per fragment before transition doubles work for previous/current fields.
- **Evidence and consequence:** Cost scales with output pixels, not occupied bins, and can dominate frame time/bandwidth at normal window sizes. No end-to-end frame benchmark substantiates acceptable latency.
- **Correction:** Precompute a filtered height/normal texture in compute passes, sample texture hardware efficiently, bound horizon work by quality tier, and benchmark the real workbench frame on named hardware before retaining the effect as a default.
- **Effort / scope:** L; GPU performance architecture.

#### RS-063 — High — Difference mode triples resident point state

- **Location / symbol:** `crates/rawscope-render/src/scatter_difference_renderer.rs:30-57,115-128`; main renderer `crates/rawscope-render/src/scatter_density_renderer.rs:78-94`; projection copy `apps/rawscope-workbench/src/app_scatter_projection.rs:61-70`.
- **Finding:** Difference rendering owns two complete `ScatterDensityGpuState`s (baseline and active) while the normal renderer owns a third, each with a point buffer and associated masks/count buffers.
- **Evidence and consequence:** The same point coordinates are uploaded three times; CPU raw/projected copies and source strings compound VRAM/RAM usage. Large datasets can fail despite the algorithm needing only shared points plus separate masks/count grids.
- **Correction:** Introduce shared immutable dataset GPU resources and independent cohort/grid states; make renderer views borrow/reference a generation-owned resource handle.
- **Effort / scope:** L; architectural GPU ownership.

#### RS-064 — Medium — Difference bind groups churn per operation

- **Location / symbol:** `crates/rawscope-render/src/scatter_difference_renderer.rs:197-220,264-294`, render/reduction setup.
- **Finding:** Difference bind groups are created inside render/reduction paths even though their buffers/layouts usually persist.
- **Evidence and consequence:** Per-frame/per-update driver object allocation adds CPU overhead and allocator churn, especially while interaction already schedules frequent rebins.
- **Correction:** Cache bind groups by buffer generation and rebuild only when a backing resource changes.
- **Effort / scope:** S-M; local performance fix.

#### RS-065 — High — Difference normalization loses totals above 2^24

- **Location / symbol:** `crates/rawscope-render/src/scatter_difference_renderer.rs:297-313`, `params`; shaders `crates/rawscope-render/src/shaders/scatter_difference_reduce.wgsl:31-38` and `crates/rawscope-render/src/shaders/scatter_difference_render.wgsl:77-78`.
- **Finding:** `u64` baseline/active totals are cast to `f32` before share calculation.
- **Evidence and consequence:** Exact row totals above 16,777,216 lose integer precision; small share deltas can be rounded differently from CPU f64 evidence and inspection. Zero totals are also not rejected at this direct renderer boundary.
- **Correction:** Use a numerically stable scaled representation (or f64-capable CPU-precomputed reciprocal with a documented error bound), validate non-zero totals, and assert GPU/CPU delta tolerance at large counts.
- **Effort / scope:** M; numerical/GPU contract.

#### RS-066 — Medium — Point reveal configuration accepts invalid radius

- **Location / symbol:** `crates/rawscope-render/src/scatter_point_reveal.rs:22-39,94-117`; `crates/rawscope-render/src/scatter_point_renderer.rs:95-129`.
- **Finding:** Thresholds are validated but `radius_px` is not. The renderer clamps it; NaN remains NaN through `f32::clamp` and reaches the shader.
- **Evidence and consequence:** Public configuration can produce invalid geometry with no error, while negative/infinite values are silently changed instead of rejected.
- **Correction:** Validate all config fields in one checked constructor, keep fields private, and make renderer methods accept only validated configuration.
- **Effort / scope:** S-M; local API.

#### RS-067 — High — Field metadata can disagree with computed density

- **Location / symbol:** `crates/rawscope-render/src/scatter_density_renderer.rs:207-253`, `update_density_for_field`.
- **Finding:** The method accepts an `update.config` used for compute and an independent `DensityFieldViewport` used for render/evidence metadata without checking that ranges and dimensions agree.
- **Evidence and consequence:** A caller can compute one grid and label/reproject it as another, making pixels, axes, hit-testing, and evidence inconsistent.
- **Correction:** Derive `DensityFieldViewport` from the validated compute config plus owner-minted revision/quality, rather than accepting both from the caller.
- **Effort / scope:** S-M; public API correction.

#### RS-068 — Medium — Point renderer reports unrendered indices as rendered

- **Location / symbol:** `crates/rawscope-render/src/scatter_point_renderer.rs:95-129`, `update_selection`.
- **Finding:** Invalid selection indices are discarded with `filter_map`, but `self.stats = selection.stats()` keeps the original `point_indices.len()` as `rendered_count`.
- **Evidence and consequence:** Publicly constructible `PointRevealSelection` can make UI/evidence claim more glyphs than were uploaded. Silent filtering hides the invariant violation.
- **Correction:** Validate indices and return an error, or calculate stats from the actual packed set; make selection fields private and constructor-generated.
- **Effort / scope:** S; local correctness.

#### RS-069 — Medium — Adapter selection excludes software fallback

- **Location / symbol:** `crates/rawscope-gpu/src/context.rs:41-48`; `crates/rawscope-gpu/src/compute.rs:19-26`.
- **Finding:** Both contexts request only `HighPerformance` with `force_fallback_adapter: false`, with no configurable fallback attempt.
- **Evidence and consequence:** Systems without a qualifying discrete/hardware adapter fail outright even when a software or low-power adapter could run correctness-first workflows. This is a poor default for an unknown-consumer native application unless the hardware requirement is explicit.
- **Correction:** Define adapter policy in typed configuration, log selection, and attempt an explicit fallback when supported unless the user opts out.
- **Effort / scope:** M; GPU initialization.

#### RS-070 — High — Binning semantics are duplicated and have already drifted

- **Location / symbol:** `crates/rawscope-render/src/density_reference.rs:7-62`; `crates/rawscope-render/src/aggregate_cache.rs:183-208`; `crates/rawscope-render/src/view_summaries.rs:218-243`; `crates/rawscope-render/src/scatter_inspection.rs:312-331`; shaders `crates/rawscope-render/src/shaders/scatter_density.wgsl:31-39` and `crates/rawscope-render/src/shaders/timeline_density.wgsl:26-38`.
- **Finding:** Scatter/timeline bin formulas are reimplemented independently across CPU summaries, caches, inspection, and WGSL.
- **Evidence and consequence:** Timeline f32/f64 and u32-overflow differences are confirmed drift. Every new analysis path risks selecting a different bin than the visible GPU field.
- **Correction:** Define normative integer/float binning contracts in `rawscope-core`, reuse CPU functions, mirror one documented WGSL formula, and maintain property/parity vectors at boundaries and extreme values.
- **Effort / scope:** L; cross-cutting correctness architecture.

#### RS-071 — High — Timeline summary counts out-of-range events inconsistently

- **Location / symbol:** `crates/rawscope-render/src/timeline_brush.rs:217-270`, `TimelineSelectionSummary::from_events`.
- **Finding:** An event inside the brush time/lane selection increments `selected_event_count` even when its lane is outside the supplied `lane_count`; `lane_counts.get_mut` then omits it from lane totals.
- **Evidence and consequence:** Selected totals, percentages, lane breakdowns, and top lane do not add up. Public event records and independent lane_count make this state representable.
- **Correction:** Validate the dataset once or explicitly reject/skip invalid lanes consistently and return a typed error from summary construction.
- **Effort / scope:** S-M; local plus data invariant.

#### RS-072 — Critical — Axis generation can loop forever

- **Location / symbol:** `crates/rawscope-render/src/view_axes.rs:166-203`, `nice_numeric_axis_ticks`.
- **Finding:** If `value += step` rounds back to the same large f32 value and the duplicate label prevents another push, `ticks.len()` never reaches the cap and the `while` condition remains true forever.
- **Evidence and consequence:** A legal narrow range at a large magnitude can freeze the UI thread while constructing axes.
- **Correction:** Detect non-progress (`next <= value`) and terminate; preferably generate tick indices in f64/integer space with a bounded `for` loop and deduplicate after generation. Add a regression using adjacent large f32 values.
- **Effort / scope:** S-M; local correctness.

#### RS-073 — High — Timeline tick count is unbounded

- **Location / symbol:** `crates/rawscope-render/src/view_axes.rs:139-164,206-240`, `timeline_axes_context`/`u64_axis_ticks`.
- **Finding:** `clamp_tick_count` imposes only a minimum; unlike scatter ticks, timeline ticks are not capped before collecting `0..tick_count`.
- **Evidence and consequence:** A public caller can request an enormous allocation/loop. This is avoidable resource exhaustion in a formatting helper.
- **Correction:** Apply a documented maximum or return an iterator/bounded error; use the same validated axis options type for scatter and timeline.
- **Effort / scope:** S; local.

#### RS-074 — Medium — U64 axis labels lose integer precision

- **Location / symbol:** `crates/rawscope-render/src/view_axes.rs:206-236`, `u64_axis_ticks`.
- **Finding:** The function converts `min`, `max`, and span to f64 to derive tick values.
- **Evidence and consequence:** Above 2^53, f64 cannot represent every u64. Ticks can duplicate, skip, or round outside the intended exact integer sequence before clamping.
- **Correction:** Compute tick values with integer quotient/remainder or u128 interpolation; use floating point only for the display fraction.
- **Effort / scope:** S-M; local numerical correctness.

#### RS-075 — High — Inspection can construct a zero-width range and panic

- **Location / symbol:** `crates/rawscope-render/src/scatter_inspection.rs:312-331`, `bin_range`.
- **Finding:** Bin boundaries are computed in f32 and passed to panicking `F32Range::new`. At large magnitudes/small spans, adjacent boundary calculations can round to the same value.
- **Evidence and consequence:** Hovering/inspecting a valid viewport can panic even if axis generation did not already stall.
- **Correction:** Represent bin extents using f64 or bin-index/domain metadata, return a checked extent, and avoid requiring a strictly positive f32 interval for evidence.
- **Effort / scope:** M; local with range-model dependency.

#### RS-076 — High — Inspection allocates per-bin sample capacity eagerly

- **Location / symbol:** `crates/rawscope-render/src/scatter_inspection.rs:121-160,283-299`; `crates/rawscope-render/src/aggregate_cache.rs:157-170`.
- **Finding:** Each bin creates a `Vec` with `sample_limit` capacity before knowing whether it is occupied; sample insertion is sorted `Vec::insert` plus truncate.
- **Evidence and consequence:** Memory is O(grid bins × sample limit) even for sparse data, and per-record sampling can shift O(K) elements. The workbench builds active and sometimes baseline grids, doubling the cost.
- **Correction:** Allocate samples lazily for occupied bins and use a bounded max-heap or exploit monotonically increasing row IDs when that invariant is explicit.
- **Effort / scope:** M; local performance redesign.

#### RS-077 — Medium — Public difference inspection divides by zero

- **Location / symbol:** `crates/rawscope-render/src/difference_density.rs:143-157`, `difference_inspection`; validated wrapper `:166-187`.
- **Finding:** The public scalar helper divides by totals without validation, although the grid wrapper rejects zero totals.
- **Evidence and consequence:** Direct callers receive NaN/infinity and can pass it into fixed-point encoding, percentile, evidence, or UI logic.
- **Correction:** Make the unchecked helper private after validated construction, or return `Result<DifferenceInspection, DifferenceDensityError>`.
- **Effort / scope:** S; local API.

#### RS-078 — High — Missingness invents a column for zero-column tables

- **Location / symbol:** `crates/rawscope-render/src/missingness_reference.rs:159-210,217-240`, `missingness_selection_summary`/`clamp_selection`.
- **Finding:** With `column_count == 0`, saturating/clamp logic produces selection range `0..1`; loops then count totals for a phantom column while no selected column name exists.
- **Evidence and consequence:** Public empty tables can yield internally contradictory missingness evidence instead of an empty result/error.
- **Correction:** Reject zero-column inputs or return an explicitly empty summary before clamping; validate table shape at construction.
- **Effort / scope:** S; local correctness.

#### RS-079 — High — Missingness recomputes cache-unfriendly O(rows × columns) scans

- **Location / symbol:** `crates/rawscope-render/src/missingness_reference.rs:110-153,158-210`.
- **Finding:** Grid construction loops buckets -> columns -> rows over row-oriented `Vec<String>` data, and selection summary scans the selected region again.
- **Evidence and consequence:** Wide/large datasets suffer repeated pointer chasing and string trimming on the UI thread; the same missing counts are separately recomputed for dataset diff/catalogue.
- **Correction:** Compute typed column missing bitsets/count indexes once during ingestion, aggregate buckets from them, and reuse the same index for filters/diff/selection.
- **Effort / scope:** L; data/analysis architecture.

#### RS-080 — Medium — Dataset diff collapses duplicate columns

- **Location / symbol:** `crates/rawscope-render/src/dataset_diff.rs:52-103,117-141`, `column_kinds_by_name`/`missingness_counts_by_name`.
- **Finding:** `BTreeMap<&str, ...>` keys schema and missingness exclusively by column name.
- **Evidence and consequence:** Duplicate input names overwrite each other and produce a plausible but false diff; this is a distinct downstream consequence of RS-037.
- **Correction:** Reject duplicates at ingestion or diff by stable column ID/ordinal plus name, surfacing ambiguity explicitly.
- **Effort / scope:** S after RS-037; otherwise M.

#### RS-081 — Medium — Public summary functions panic on configuration

- **Location / symbol:** `crates/rawscope-render/src/view_summaries.rs:65-76,101-113,135-145,170-178`.
- **Finding:** Public scatter/timeline summary builders assert non-zero bin/lane counts and allocate directly from caller values.
- **Evidence and consequence:** These are externally reachable analytical APIs, not private post-validation helpers. Invalid user configuration terminates the process and huge counts allocate unbounded memory.
- **Correction:** Accept validated non-zero bounded config types or return typed errors, sharing limits with GPU grid configuration.
- **Effort / scope:** M; public API cleanup.

#### RS-082 — Medium — Source drilldown sampling disclosure is ambiguous

- **Location / symbol:** `crates/rawscope-render/src/selection_drilldown.rs:94-170`, `build_selection_drilldown`.
- **Finding:** The unmasked path sets `rows_are_sampled` only when `selected_row_count > max_rows`, not when retained source rows are missing and `rows.len() < selected_row_count`.
- **Evidence and consequence:** A drilldown may display fewer rows than selected while claiming it is not sampled/truncated. The masked path uses the more truthful `selected_row_count > rows.len()` rule.
- **Correction:** Distinguish `sampled_due_to_limit` from `source_rows_unavailable`, and use one builder/contract for masked and unmasked paths.
- **Effort / scope:** S-M; local evidence/UI contract.

#### Evidence models, serialization, and report bundles

#### RS-083 — Critical — Filtered scatter evidence is still disabled

- **Location / symbol:** `apps/rawscope-workbench/src/app_brush.rs:139-160`, `build_selection_evidence`; claimed completion `features/next-generation-gpu-visual-analytics/tasks/T008.md:221-226` and `features/next-generation-gpu-visual-analytics/tasks/T016.md:193-198`.
- **Finding:** If any scatter filter is active, brush finalization sets `selection_evidence = None` and returns. V4/V5 can describe filters and cohorts, but the prerequisite evidence is never built.
- **Evidence and consequence:** The flagship filtered density workflow cannot export evidence at all. Historical task T008 says this was a temporary gate that T015 would remove, and T016 declares filtered selection/v4 evidence agreement; current code contradicts both and README's evidence path.
- **Correction:** Build the canonical selection from the active mask, derive all counts/samples from the same cohort snapshot, then emit v4/v5. Keep legacy v1-v3 unavailable for filtered state if they cannot represent filters truthfully.
- **Effort / scope:** L; cross-cutting product correctness.

#### RS-084 — High — Linked scatter selections ignore the active filter

- **Location / symbol:** `apps/rawscope-workbench/src/app_selection.rs:21-49`, `publish_scatter_active_selection`; masked summary/drilldown `apps/rawscope-workbench/src/app_brush.rs:162-204`.
- **Finding:** Linked selection scans all points inside the brush without consulting `FilterEvaluation.mask`, while visible density, summary, and drilldown are mask-aware.
- **Evidence and consequence:** A downstream linked view receives excluded row IDs and disagrees with the visible/selected cohort. This is a confirmed cross-view correctness defect.
- **Correction:** Publish a selection snapshot produced once from the active cohort, and make summary, drilldown, evidence, and linked consumers read that same immutable row-ID set.
- **Effort / scope:** M; cross-cutting selection architecture.

#### RS-085 — High — Aggregate evidence can omit selected bins

- **Location / symbol:** call site `apps/rawscope-workbench/src/app_export.rs:193-212`; builder `crates/rawscope-render/src/scatter_selection_export_v3.rs:175-236`, `scatter_aggregate_evidence_context`.
- **Finding:** Selected bins are detected by intersecting `evidence_v2.selected_row_id_sample` with each aggregate bin's own bounded `row_ids` sample.
- **Evidence and consequence:** A bin containing selected rows is missed whenever neither bounded sample happens to share the same row ID. It can then be replaced by an unrelated dense context bin, so exported aggregate context does not necessarily cover the selected region.
- **Correction:** Identify selected bin indices from geometry/complete selected IDs during the canonical selection pass; samples may illustrate a known selected bin but must not determine whether it is selected.
- **Effort / scope:** M; evidence/aggregate contract.

#### RS-086 — High — V4 validity is constructor-only and incomplete

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_evidence_v4.rs:19-85,134-169,171-222`, public v4 structs and `from_v3`.
- **Finding:** All fields are public, there is no public `validate(&self)`, and serializers accept arbitrary instances. Constructor validation does not check schema version, non-zero grid dimensions, finite/ordered ranges, filter validity/order, selected sample coherence, aggregate-bin bounds, selected percentage, comparison coherence, or pinned bin/range bounds.
- **Evidence and consequence:** Consumers can serialize internally contradictory “v4” evidence that passes no validation, and mutations after construction bypass the one checked path.
- **Correction:** Make contracts immutable/private, implement comprehensive validation reusable by constructor and serializer, and model validated subcontracts for query/cohort/samples rather than one broad public DTO.
- **Effort / scope:** L; cross-cutting evidence API.

#### RS-087 — High — V4 accepts impossible difference magnitude

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_evidence_v4.rs:190-198`, `validate_visual_query`.
- **Finding:** `max_abs_delta` is checked only for finite and non-negative values, although it is the absolute difference between two shares and therefore cannot exceed 1.
- **Evidence and consequence:** Evidence can claim a normalized share delta of 5.0 while passing the official constructor.
- **Correction:** Validate `0.0..=1.0` with the same numerical tolerance/definition used by CPU/GPU difference semantics.
- **Effort / scope:** S; local correctness.

#### RS-088 — High — V5 does not verify its V4 source contract

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_evidence_v5.rs:97-118,146-200`, `LegacySchemaVersion`, `from_v4`, `validate`.
- **Finding:** `LegacySchemaVersion` is defined but never used. `from_v4` does not check `evidence.schema_version == 4`, and `validate` does not re-run V4 query/cohort/sample validation.
- **Evidence and consequence:** A manually invalid or schema-mislabeled V4 object is copied into V5 and can pass V5 validation. V5 therefore does not prove the compatibility contract its error enum advertises.
- **Correction:** Expose one canonical V4 validation function, require a validated V4 value in `from_v4`, and invoke it from V5 validation/serialization.
- **Effort / scope:** M; evidence contract.

#### RS-089 — High — V5 mode/config presence validation is asymmetric

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_evidence_v5.rs:172-187`, `validate`.
- **Finding:** Filtered-difference requires `visual_query.difference`, but absolute density is not rejected merely for carrying a difference config; it is rejected only when a pinned inspection also has difference details.
- **Evidence and consequence:** A public V5 object can claim absolute density and include a global difference configuration while passing if there is no pinned difference. Consumers cannot determine which semantic mode is authoritative.
- **Correction:** Require exact bidirectional agreement between density mode and every difference-bearing field, independent of whether inspection is pinned.
- **Effort / scope:** S; local validation.

#### RS-090 — High — Serializers do not validate evidence

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_export_v4.rs:12-18`; `crates/rawscope-render/src/scatter_selection_export_v5.rs:15-45`; analogous v1-v3/timeline JSON entry points in `crates/rawscope-render/src/scatter_selection_export.rs:27-33`, `crates/rawscope-render/src/scatter_selection_export_v3.rs:27-32`, and `crates/rawscope-render/src/timeline_selection_export_v3.rs:27-32`.
- **Finding:** Public JSON/Markdown functions directly map public mutable DTOs without a validation call.
- **Evidence and consequence:** Schema versions, counts, ranges, samples, formulas, and artifact kinds can disagree in durable files. Passing construction once is insufficient because fields remain public.
- **Correction:** Serialize only an opaque validated evidence type, or make every export entry point validate and return a domain validation/serialization error.
- **Effort / scope:** M; cross-cutting API.

#### RS-091 — Medium — Missing source rows are silently dropped from evidence

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_evidence.rs:139-177`; `crates/rawscope-render/src/timeline_selection_evidence.rs:135-176`, source-sample `filter_map` conversions.
- **Finding:** Source lookup failures are removed with `filter_map` without an error or disclosure field.
- **Evidence and consequence:** `selected_row_id_sample` and `selected_source_row_sample` can have different lengths while the artifact merely appears bounded. A broken row-index invariant becomes silent evidence loss.
- **Correction:** Treat missing retained rows as an invariant error or record explicit unavailable row IDs/reasons in the evidence contract.
- **Effort / scope:** S-M; evidence/data invariant.

#### RS-092 — High — Finalization repeatedly rescans the same dataset

- **Location / symbol:** `apps/rawscope-workbench/src/app_brush.rs:121-204`; `apps/rawscope-workbench/src/app_selection.rs:21-49`; `apps/rawscope-workbench/src/app_export.rs:193-229`; builders in `crates/rawscope-render/src/scatter_selection_evidence.rs:139-242` and `crates/rawscope-render/src/selection_drilldown.rs:22-170`.
- **Finding:** Summary, drilldown, linked selection, v1 evidence, comparison, and aggregate context independently discover selected records/IDs.
- **Evidence and consequence:** Finalizing a brush performs multiple O(N) scans and can produce drift because some passes use masks and others do not. On large local data this blocks the UI thread.
- **Correction:** Create one immutable `SelectionSnapshot` per dataset/filter/viewport generation containing sorted IDs, counts, typed samples, and source references; derive all views/artifacts from it.
- **Effort / scope:** L; architectural.

#### RS-093 — High — Markdown export permits content injection

- **Location / symbol:** representative raw joins `crates/rawscope-render/src/scatter_selection_export.rs:206-220`, `crates/rawscope-render/src/scatter_selection_export_v3.rs:155-169`, `crates/rawscope-render/src/timeline_selection_export.rs:223-237`, `crates/rawscope-render/src/timeline_selection_export_v3.rs:186-200`; v4 filter formatting `crates/rawscope-render/src/scatter_selection_export_v4.rs:140-162`.
- **Finding:** Untrusted column names, categories, display names, and source cells are inserted into Markdown tables/lists without escaping pipes, newlines, backticks, links, or image syntax.
- **Evidence and consequence:** Data can alter report structure, spoof headings/fields, hide neighbouring cells, or trigger remote resource loads when opened by a Markdown renderer. The durable evidence view is not integrity-preserving.
- **Correction:** Use a single context-aware Markdown escaping layer (table cell, inline code, list text) and regression fixtures containing pipes/newlines/link/image syntax.
- **Effort / scope:** M; cross-cutting artifact security.

#### RS-094 — High — Evidence leaks absolute local paths

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_export.rs:375-395,596-608`; `crates/rawscope-render/src/timeline_selection_export.rs:392-412,674-686`; v3 equivalents `crates/rawscope-render/src/scatter_selection_export_v3.rs:321-341,615-628` and `crates/rawscope-render/src/timeline_selection_export_v3.rs:356-376,689-701`; bundle context `apps/rawscope-workbench/src/app_report_bundle.rs:493-512`.
- **Finding:** JSON, Markdown, and visual context serialize `path.display()` for local datasets.
- **Evidence and consequence:** Reports reveal usernames, directory layout, mounted shares, and project/customer names; paths are machine-specific and make artifacts non-portable. Sharing a report can disclose information unrelated to analysis.
- **Correction:** Store a portable source label/content fingerprint and optional relative/display path; require explicit opt-in for absolute provenance and mark it as sensitive.
- **Effort / scope:** M; cross-cutting privacy/schema change.

#### RS-095 — Medium — Evidence artifacts are write-only

- **Location / symbol:** serializer DTOs throughout `crates/rawscope-render/src/scatter_selection_export*.rs` and `crates/rawscope-render/src/timeline_selection_export*.rs` derive `Serialize` only; no reader/validator module exists in `crates/rawscope-render/src/lib.rs:31-47`.
- **Finding:** RawScope cannot parse, validate, migrate, or inspect the evidence it emits.
- **Evidence and consequence:** Compatibility claims are tested only at string-generation time; consumers must reverse-engineer private DTOs, and corrupt artifacts cannot be distinguished from valid ones by the project.
- **Correction:** Move schema models to a dedicated evidence crate/module with `Serialize + Deserialize`, strict version dispatch, validation, migration policy, and round-trip/golden contract tests.
- **Effort / scope:** XL; architectural.

#### RS-096 — Medium — Markdown and JSON disagree on numeric precision

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_export.rs:49-60,90-95,136-158,194-218,549-552`; `crates/rawscope-render/src/timeline_selection_export.rs:621-624`; versioned exporters use the same fixed formatting.
- **Finding:** Markdown rounds coordinates/ranges to six decimals (percentages to four) while JSON retains f32 values.
- **Evidence and consequence:** Small but meaningful extents can collapse to identical text, so the human-readable evidence can contradict the machine artifact.
- **Correction:** Define schema-level display precision/round-trip requirements, use shortest round-trippable formatting where evidence exactness matters, and explicitly label any presentation rounding.
- **Effort / scope:** S-M; cross-cutting artifact contract.

#### RS-097 — Medium — V4/V5 serialization round-trips through text/`Value`

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_export_v4_artifact.rs:43-100`; `crates/rawscope-render/src/scatter_selection_export_v5.rs:15-88`.
- **Finding:** V4 constructs a legacy v3 object, serializes to a JSON string, reparses to `Value`, indexes/clones fields, and uses `expect`; V5 repeats the pattern through V4 and mutates the object.
- **Evidence and consequence:** Export performs redundant allocations/parsing and encodes version inheritance as runtime JSON surgery. Internal shape changes can panic or silently drop/rename fields rather than fail at compile time.
- **Correction:** Define typed compositional artifact structs/common flattened components and convert directly; eliminate `expect` and string/`Value` schema inheritance.
- **Effort / scope:** L; evidence serialization architecture.

#### RS-098 — Low — V5 Markdown versioning is a string replacement

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_export_v5.rs:71-89`, `scatter_selection_evidence_v5_markdown`.
- **Finding:** V5 builds V4 Markdown then replaces the first literal `Evidence v4` with `Evidence v5`.
- **Evidence and consequence:** Heading wording becomes an implicit API; a harmless V4 copy edit can leave V5 mislabeled. This is brittle version coupling.
- **Correction:** Render shared sections through typed helpers and supply the version/heading explicitly.
- **Effort / scope:** S; local, best done with RS-097.

#### RS-099 — High — Schema version is mutable data instead of serializer truth

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_export_v3.rs:239-262`; `crates/rawscope-render/src/scatter_selection_export_v4_artifact.rs:25-48`; `crates/rawscope-render/src/scatter_selection_export_v5.rs:24-32`; timeline v3 `crates/rawscope-render/src/timeline_selection_export_v3.rs:270-295`.
- **Finding:** Version-specific serializers write `evidence.schema_version`, a public mutable field, rather than the serializer's fixed version constant.
- **Evidence and consequence:** A v3/v4/v5 shape can claim another version while the artifact kind/file path says otherwise, breaking dispatch and compatibility.
- **Correction:** Remove mutable schema-version fields from in-memory validated types or make serializers emit their compile-time constant and reject mismatches.
- **Effort / scope:** M; cross-cutting schema API.

#### RS-100 — High — Evidence-version duplication is an architectural dead end

- **Location / symbol:** `crates/rawscope-render/src/scatter_selection_export.rs:1-619`, `crates/rawscope-render/src/scatter_selection_export_v3.rs:1-648`, `crates/rawscope-render/src/timeline_selection_export.rs:1-700`, `crates/rawscope-render/src/timeline_selection_export_v3.rs:1-725`, plus v4/v5 modules; facade `crates/rawscope-render/src/lib.rs:124-171`.
- **Finding:** Each version duplicates dataset/source DTOs, formatting, samples, comparisons, and conversion code inside the render crate.
- **Evidence and consequence:** Fixes such as escaping, path privacy, numeric formatting, and source support must be repeated across many paths and are already inconsistent. Adding a version scales by copy rather than extension.
- **Correction:** Establish a dedicated canonical evidence domain and versioned wire adapters composed from shared typed components; preserve old bytes with golden tests, not copy-pasted business logic.
- **Effort / scope:** XL; architectural.

#### RS-101 — High — Report bundles are non-atomic and leave partial evidence

- **Location / symbol:** `apps/rawscope-workbench/src/app_report_bundle.rs:103-180,185-276`; `apps/rawscope-workbench/src/app_report_bundle_v4.rs:20-56`; `apps/rawscope-workbench/src/app_report_bundle_v5.rs:20-56`.
- **Finding:** Writers create the final directory and sequentially write JSON, Markdown, visual context, then manifest. Failure does not remove or mark the partial bundle.
- **Evidence and consequence:** Disk-full, permission, serialization, or interruption leaves a directory that looks like a report but is incomplete; retries may select another directory and strand corrupt artifacts.
- **Correction:** Write into an exclusively created temporary sibling directory, fsync as appropriate, write manifest last, atomically rename to final, and clean temporary state on error.
- **Effort / scope:** M; report IO boundary.

#### RS-102 — High — “Collision-safe” bundle naming has a TOCTOU race

- **Location / symbol:** `apps/rawscope-workbench/src/app_report_bundle.rs:55-72`, `EvidenceReportBundlePaths::next_available`; later `create_dir_all` at `:107`, `:148`, `:189`, `:232` and v4/v5 `:22`.
- **Finding:** Availability is checked with `exists()` and the directory is created later with non-exclusive `create_dir_all`.
- **Evidence and consequence:** Two processes/threads selecting the same timestamp/counter can both proceed and overwrite files. README's `collision-safe` claim (`README.md:139`) is false under concurrency.
- **Correction:** Reserve the name with an atomic exclusive directory/file creation operation and retry on `AlreadyExists`; combine with the transactional temporary-directory design.
- **Effort / scope:** S-M; local IO correctness.

#### RS-103 — Medium — Bundle manifest path semantics change by version

- **Location / symbol:** legacy manifest records `apps/rawscope-workbench/src/app_report_bundle.rs:121-139,162-180,203-223,246-266`; v4 filenames `apps/rawscope-workbench/src/app_report_bundle_v4.rs:35-55`; v5 filenames `apps/rawscope-workbench/src/app_report_bundle_v5.rs:35-55`.
- **Finding:** V1-v3 manifests serialize full `Path` values for child files, while V4/V5 serialize only file names.
- **Evidence and consequence:** Moving a legacy bundle can invalidate its manifest and leak the export root; consumers need undocumented version-specific path resolution.
- **Correction:** Use bundle-relative normalized paths in every new schema, document legacy resolution, and add a reader that normalizes both forms.
- **Effort / scope:** M; schema compatibility.

#### RS-104 — Medium — Evidence export runs synchronously on the event thread

- **Location / symbol:** UI action `apps/rawscope-workbench/src/ui.rs:384-388`; report writes in `apps/rawscope-workbench/src/app_report_bundle.rs:103-276`, `apps/rawscope-workbench/src/app_report_bundle_v4.rs:20-56`, and `apps/rawscope-workbench/src/app_report_bundle_v5.rs:20-56`.
- **Finding:** Serialization, Markdown construction, directory probing, and all filesystem writes run inline during UI action handling.
- **Evidence and consequence:** Large source samples or slow/remote disks freeze the window; cancellation and progress reporting are impossible.
- **Correction:** Capture a validated immutable evidence snapshot, perform transactional export on a bounded worker, and report completion/failure back through the event loop with a generation/job ID.
- **Effort / scope:** L; concurrency/IO boundary.

#### RS-105 — Medium — Schema documentation is incomplete

- **Location / symbol:** `docs/schemas/` contains scatter v1, v2, v4, v5 and timeline v1, v2; code exposes scatter v3 at `crates/rawscope-render/src/lib.rs:128-131` and timeline v3 at `:183-190`.
- **Finding:** Both v3 wire formats are public and actively used but have no schema document.
- **Evidence and consequence:** External consumers cannot understand the additive comparison/aggregate fields or distinguish implemented compatibility from internal history.
- **Correction:** Document v3 exactly, link every version from an index, and generate/check examples from the typed reader/writer contract to prevent drift.
- **Effort / scope:** S-M; documentation/API.

#### Workbench state, responsiveness, and failure handling

#### RS-106 — High — `WorkbenchApp` is a god state object

- **Location / symbol:** `apps/rawscope-workbench/src/app.rs:64-110`, `WorkbenchApp`; nested view states `:112-156`; sibling module list `apps/rawscope-workbench/src/main.rs:1-55`.
- **Finding:** One struct owns window/GPU/egui resources, datasets, sessions, selection, comparison, missingness, filters, inspection, projection, gestures, scheduling, transitions, export status, and both views. Roughly fifty sibling modules mutate its fields through distributed `impl WorkbenchApp` blocks.
- **Evidence and consequence:** Invariants are implicit across files; any operation can partially mutate unrelated concerns; borrow boundaries are worked around through broad field visibility and take/replace patterns. Growth increases combinatorial state coupling rather than adding isolated capabilities.
- **Correction:** Split state into lifecycle-owned domain controllers (`DatasetSession`, `ScatterViewModel`, `TimelineViewModel`, `SelectionSnapshotStore`, `RenderCoordinator`, `ExportJobs`) with narrow command/result APIs; keep the winit app as an event router.
- **Effort / scope:** XL; architectural.

#### RS-107 — Critical — Startup and dataset preparation block the event thread

- **Location / symbol:** `apps/rawscope-workbench/src/app_events.rs:18-22`, `resumed`; `apps/rawscope-workbench/src/app_window.rs:13-47`, `create_window_and_gpu`; scatter load `apps/rawscope-workbench/src/app.rs:226-329`; timeline load `apps/rawscope-workbench/src/app_timeline.rs:28-115`.
- **Finding:** WGPU initialization uses `pollster::block_on`, then profile validation, full dataset loading, evidence-key validation, multiple GPU buffer/pipeline builds, summaries, aggregate, inspection, and comparison setup execute synchronously before returning to winit.
- **Evidence and consequence:** Large/slow input makes the native window appear hung or unresponsive and offers no progress/cancellation. Failures after partial setup complicate recovery.
- **Correction:** Stage startup through explicit loading states and background IO/CPU jobs; create GPU resources incrementally on the owning thread/queue; deliver generation-tagged results to the event loop and permit cancellation/retry.
- **Effort / scope:** XL; architectural concurrency.

#### RS-108 — Critical — Exact scatter settling combines readback with full CPU rescans

- **Location / symbol:** `apps/rawscope-workbench/src/app_render_schedule.rs:208-303`, `prepare_scheduled_density`; `apps/rawscope-workbench/src/app_scatter_inspection.rs:58-115`; readback `crates/rawscope-render/src/gpu_density_pipeline.rs:148-173`.
- **Finding:** Exact work requests `FullCounts`, blocks for readback, then immediately refreshes marginals and rebuilds active inspection; difference mode also builds a baseline grid/distribution, and point reveal is invalidated for another scan.
- **Evidence and consequence:** Releasing a pan/zoom gesture can synchronously perform GPU wait plus several O(N) CPU passes on the UI thread. The “progressive” preview only delays the stall until settle.
- **Correction:** Keep exact aggregation resident, schedule readback/CPU indexes off-thread, reuse one selection/aggregate index, and atomically publish completed generation snapshots without blocking rendering.
- **Effort / scope:** XL; architectural performance.

#### RS-109 — High — Render scheduler models synchronous work as in-flight

- **Location / symbol:** `apps/rawscope-workbench/src/app_render_schedule.rs:45-169,208-303`, `RenderSchedule`/`prepare_scheduled_density`.
- **Finding:** Scheduler state prevents multiple “in-flight” jobs, but dispatch, readback, difference update, and completion all occur in the same call before `work_completed`.
- **Evidence and consequence:** The abstraction gives the appearance of asynchronous coalescing without a completion primitive, cancellation boundary, or stale-result rejection. Its tests prove phase bookkeeping, not non-blocking behaviour.
- **Correction:** Either simplify it to an honest synchronous debounce or introduce actual job handles/generation IDs and completion events; do not call work in-flight until control returns before completion.
- **Effort / scope:** L; architectural scheduling.

#### RS-110 — Critical — Timeline recomputes synchronously on every pointer event

- **Location / symbol:** event routing `apps/rawscope-workbench/src/app_events.rs:65-76`; interaction `apps/rawscope-workbench/src/app_timeline.rs:166-226`; recompute `apps/rawscope-workbench/src/app_timeline.rs:245-269`.
- **Finding:** Wheel zoom and every pan cursor move call `recompute_timeline_density`, which refreshes CPU summaries and runs the resource-rebuilding/readback path in RS-050.
- **Evidence and consequence:** Pointer-frequency input directly drives full O(events + grid + GPU setup/wait) work, making freezes inevitable as event count grows.
- **Correction:** Reproject during gestures, coalesce bounded preview work, perform one exact non-blocking settle after release, and share the scatter/timeline render coordinator.
- **Effort / scope:** XL; architectural.

#### RS-111 — High — Demo switching silently retains partial failure

- **Location / symbol:** `apps/rawscope-workbench/src/ui.rs:436-473`, `switch_demo_mode`.
- **Finding:** The method changes `demo_mode`, export/surface state, and lets preparation mutate the app; if preparation fails, it ignores the error and does not restore the previous mode/state or show an error.
- **Evidence and consequence:** A failed switch can leave a new mode selected with old/partial renderers and dataset fields. Users receive no actionable diagnosis.
- **Correction:** Prepare a complete next-mode state off to the side, commit it only on success, and expose a persistent failure status; otherwise retain the prior state.
- **Effort / scope:** L; cross-cutting transactional state.

#### RS-112 — Medium — UI projection clones substantial state every frame

- **Location / symbol:** `apps/rawscope-workbench/src/ui.rs:170-296`, `ui_state`; `apps/rawscope-workbench/src/app_window.rs:57-63`, `update_window_title`.
- **Finding:** Each `ui_state()` call clones comparison, drilldown, dataset diff, inspection, export strings, and multiple vectors/labels. Updating the title builds the whole UI projection merely to read its title.
- **Evidence and consequence:** Frame and state-change paths allocate/copy evidence-facing structures unnecessarily; cost grows with drilldown/diff complexity.
- **Correction:** Borrow immutable view state for rendering, derive a small title projection directly, and use revisioned cached presentation models for expensive owned data.
- **Effort / scope:** M-L; workbench architecture/performance.

#### RS-113 — High — Filter updates are not transactional across GPU consumers

- **Location / symbol:** `apps/rawscope-workbench/src/app_scatter_filter.rs:114-158`, `apply_scatter_filter_action`.
- **Finding:** The main renderer mask is uploaded first. If the difference renderer upload fails, CPU filter/evaluation state is not committed, but the main GPU mask has already changed.
- **Evidence and consequence:** Visible main density can use the new mask while UI, summaries, selection, and difference mode still claim the old cohort. The operation returns without rollback.
- **Correction:** Validate/prepare all resources first, commit a single cohort generation to every consumer, and only publish CPU/UI state when all uploads are accepted; on failure retain the prior generation everywhere.
- **Effort / scope:** L; cross-cutting transactional state.

#### RS-114 — High — Projection updates are not transactional

- **Location / symbol:** `apps/rawscope-workbench/src/app_scatter_projection.rs:73-132`, `set_scatter_projection`.
- **Finding:** The main renderer dataset/revision and mask are replaced before the difference renderer. Any later error returns before CPU points/viewport/projection labels are committed or before recompute succeeds.
- **Evidence and consequence:** GPU and CPU generations can diverge, and the monotonic dataset revision is consumed by a failed operation. Subsequent retries may be suppressed by revision equality.
- **Correction:** Build generation-owned projected CPU/GPU resources, validate every consumer, and swap the complete projection state atomically.
- **Effort / scope:** L; cross-cutting transactional state.

#### RS-115 — High — Runtime memory has no explicit budget or ownership model

- **Location / symbol:** CPU state `apps/rawscope-workbench/src/app.rs:112-156`; raw projection clone `apps/rawscope-workbench/src/app_scatter_projection.rs:61-70`; GPU duplication `crates/rawscope-render/src/scatter_difference_renderer.rs:30-57`; source materialization RS-031.
- **Finding:** The workbench retains source strings, visual records, raw projection records, comparison source tables, active/baseline inspection samples, and three GPU point states without a memory estimate, eviction, or maximum dataset policy.
- **Evidence and consequence:** Memory/VRAM scales through several independent owners. Failure occurs at arbitrary allocation/backend points instead of a controlled admission check, undermining “large raw datasets” positioning.
- **Correction:** Introduce a dataset generation/resource owner with measured RAM/VRAM accounting, shared buffers, bounded caches, and explicit admission/degradation policy.
- **Effort / scope:** XL; architectural.

#### RS-116 — Medium — Inspection failures are swallowed

- **Location / symbol:** `apps/rawscope-workbench/src/app_scatter_inspection.rs:58-115`, `rebuild_scatter_inspection_cache`.
- **Finding:** Any inspection build error falls through `Err(_) => default()` with no log, status, or preserved context.
- **Evidence and consequence:** Mask mismatch, invalid config, or numerical failure silently disables inspection, and users/maintainers cannot distinguish empty data from a defect.
- **Correction:** Preserve the prior valid cache where safe, log structured dataset/filter/viewport generations with the source error, and expose a concise degraded-state message.
- **Effort / scope:** S; local diagnostics.

#### RS-117 — Medium — Error type erasure reaches orchestration boundaries

- **Location / symbol:** `apps/rawscope-workbench/src/app.rs:226-399`, `prepare_scatter_demo`; `apps/rawscope-workbench/src/app_timeline.rs:28-269`; `apps/rawscope-workbench/src/app_render_schedule.rs:208-303`, all returning `Box<dyn Error>`.
- **Finding:** Broad orchestration methods erase concrete error categories even though recovery differs for dataset, profile, GPU limits, readback, session, and filesystem failures.
- **Evidence and consequence:** Callers cannot implement targeted retry/degrade/reconfigure behaviour and UI messages fall back to strings.
- **Correction:** Define a workbench operation error enum with source-preserving variants and recovery classification; keep low-level domain errors intact.
- **Effort / scope:** M-L; cross-cutting error boundary.

#### Session and Python bridge

#### RS-118 — Medium — Session crate depends on the full data stack for one profile type

- **Location / symbol:** `crates/rawscope-session/Cargo.toml:7-10`; imports `crates/rawscope-session/src/manifest.rs:295-303` and `crates/rawscope-session/src/resolved_session.rs:8,129-137`.
- **Finding:** `rawscope-session` depends on `rawscope-data`, thereby transitively pulling Arrow/Parquet, merely to parse/store `DatasetProfileId`.
- **Evidence and consequence:** A small manifest schema crate has heavy compile/supply-chain cost and cannot be reused independently from ingestion.
- **Correction:** Move profile identity/parsing to a dependency-light domain/contract crate (or the dependency-free core if the concept belongs there); keep concrete profile implementation in data.
- **Effort / scope:** M; crate-boundary change.

#### RS-119 — Medium — Session strings are checked but not normalized

- **Location / symbol:** `crates/rawscope-session/src/manifest.rs:222-303`, validation; `crates/rawscope-session/src/resolved_session.rs:55-137`, resolution.
- **Finding:** Bindings/display/evidence-key values are rejected only if `trim().is_empty()` but the original whitespace is stored; equality compares untrimmed strings.
- **Evidence and consequence:** `x = "rating"`, `y = " rating "` passes the “different columns” check and later fails lookup; profile/display identifiers have similar inconsistent semantics.
- **Correction:** Decide whether whitespace is significant. For identifiers, return validated normalized newtypes; for display text, preserve it deliberately but compare/validate through a documented normalization.
- **Effort / scope:** M; cross-language contract.

#### RS-120 — Medium — Session manifest reads are unbounded

- **Location / symbol:** `crates/rawscope-session/src/resolved_session.rs:45-62`, `load_session_manifest`.
- **Finding:** `fs::read_to_string` loads an arbitrary-size local file before parsing.
- **Evidence and consequence:** An unknown consumer opening a huge/mistaken file can allocate excessive memory on the UI startup path. Session v1 is tiny and should have a hard size budget.
- **Correction:** Check metadata and reject manifests above a documented small limit before bounded reading/parsing.
- **Effort / scope:** S; local robustness.

#### RS-121 — Medium — Persisted `usize` is platform-dependent

- **Location / symbol:** `crates/rawscope-session/src/manifest.rs:129-153`, `SessionDatasetV1::limit`; `crates/rawscope-session/src/resolved_session.rs:20-31`; Python emitter `sdk/python/src/rawscope/manifest.py:148-158`.
- **Finding:** The wire schema serializes a Rust `usize` while Python integers are unbounded.
- **Evidence and consequence:** A manifest accepted on 64-bit can fail on 32-bit; the schema has no architecture-independent maximum. This is unsuitable for a versioned portable artifact.
- **Correction:** Use `u64` in the wire schema, validate against an application maximum, and convert checked to `usize` at allocation boundaries.
- **Effort / scope:** S-M; schema change.

#### RS-122 — Low — URI/path and file-role diagnostics are ad hoc

- **Location / symbol:** `crates/rawscope-session/src/resolved_session.rs:139-162`, `canonical_file_path`/`reject_uri_path`; Python analogue `sdk/python/src/rawscope/models.py:78-92`.
- **Finding:** URI rejection is a substring search for `://`; the same canonical-file helper raises `DatasetNotRegularFile` even when validating the manifest itself.
- **Evidence and consequence:** Some valid local names can be rejected, real URI forms without that substring are not semantically parsed, and errors mislabel the failing file role.
- **Correction:** Treat manifest dataset paths as local-path syntax by schema definition, use role-specific error context, and avoid pretending a substring test is URI parsing.
- **Effort / scope:** S; cross-language diagnostics.

#### RS-123 — High — Python accepts boolean row limits

- **Location / symbol:** `sdk/python/src/rawscope/manifest.py:45-67`, `prepare_session`; same predicate in `sdk/python/src/rawscope/bundle.py:70-90` via validation.
- **Finding:** `isinstance(True, int)` is true, so `limit=True` passes the positive-integer check and serializes as JSON `true`.
- **Evidence and consequence:** Rust expects `usize` and rejects the manifest after Python claimed it was valid. `False` is rejected only because it compares `<= 0`, producing inconsistent boolean handling.
- **Correction:** Reject `bool` explicitly (`type(limit) is int` or integral-not-bool policy), bound it to the Rust schema maximum, and add a cross-language regression.
- **Effort / scope:** S; local correctness.

#### RS-124 — Medium — Python text validation preserves the same whitespace bugs

- **Location / symbol:** `sdk/python/src/rawscope/models.py:30-65`, `require_text`, `ScatterView`, `TimelineView`.
- **Finding:** `require_text` returns the original string; x/y and time/lane equality compare untrimmed values.
- **Evidence and consequence:** Python can generate sessions that pass its validation but fail Rust column resolution, duplicating RS-119 across the language boundary.
- **Correction:** Share documented normalization vectors and have frozen models store normalized identifiers at construction.
- **Effort / scope:** S-M; cross-language contract.

#### RS-125 — High — Dataframe bridge does not validate binding types

- **Location / symbol:** `sdk/python/src/rawscope/bundle.py:70-90`, `_validate_dataframe`; Arrow type helper `sdk/python/src/rawscope/adapters/common.py:18-38`.
- **Finding:** The bridge checks only that required column names exist and that the whole schema uses broad supported primitive families. It does not require numeric scatter x/y, integer timeline time, or string/integer lane.
- **Evidence and consequence:** A string scatter column or floating timeline timestamp is written successfully and fails only after the native application launches/loads the file.
- **Correction:** Add view-specific schema validation using the exact Rust binding matrix and maintain cross-language fixtures for every accepted/rejected type.
- **Effort / scope:** M; cross-language boundary.

#### RS-126 — High — Persistent dataframe bundles update data non-atomically

- **Location / symbol:** `sdk/python/src/rawscope/bundle.py:27-67`, `prepare_dataframe`.
- **Finding:** For a caller-provided destination, `data.parquet` is overwritten directly before the manifest's atomic replacement. On error, only temporary bundles are cleaned.
- **Evidence and consequence:** A failed write/manifest step can corrupt or replace a previously valid persistent bundle, leaving old manifest/new partial data or new data/old manifest.
- **Correction:** Write Parquet and manifest into a temporary sibling generation, fsync as appropriate, then atomically swap a directory or versioned files; never overwrite a valid bundle before commit.
- **Effort / scope:** M-L; SDK IO transaction.

#### RS-127 — Medium — Popen failure leaks temporary bundles

- **Location / symbol:** `sdk/python/src/rawscope/launcher.py:49-72`, `launch`; cleanup only inside `RawScopeProcess:82-115`.
- **Finding:** If `subprocess.Popen` raises, no `RawScopeProcess` exists to clean a temporary `PreparedSession`.
- **Evidence and consequence:** Failed executable resolution/permission/process creation strands potentially sensitive Parquet data in the system temp directory.
- **Correction:** Wrap process creation in `try/except` and invoke the same guarded temporary cleanup before re-raising; consider a context-managed prepared session.
- **Effort / scope:** S; local reliability/privacy.

#### RS-128 — Low — Explicit executable resolution ignores execute permission

- **Location / symbol:** `sdk/python/src/rawscope/launcher.py:132-137`, `_find_executable`.
- **Finding:** Any regular file is returned for an explicit/environment path; Unix execute permission is not checked.
- **Evidence and consequence:** Resolution reports success and later raises a lower-level `PermissionError`, bypassing the intended `RawScopeExecutableNotFound` diagnostic.
- **Correction:** On relevant platforms require `os.access(path, os.X_OK)` and report a distinct not-executable error.
- **Effort / scope:** S; local diagnostics.

#### RS-129 — High — Python evidence-key checks materialize whole columns

- **Location / symbol:** `sdk/python/src/rawscope/adapters/pyarrow.py:37-55`, `to_pylist`; `sdk/python/src/rawscope/adapters/polars.py:56-74`, `to_list`; `sdk/python/src/rawscope/adapters/pandas.py:35-53`.
- **Finding:** Every adapter builds Python objects for the complete key column and a `dict[str, int]` to test uniqueness.
- **Evidence and consequence:** High-cardinality keys roughly duplicate the column in Python object form before Parquet writing, creating severe memory/time cost and repeating Rust validation after launch.
- **Correction:** Use engine-native null/uniqueness aggregates where exact and bounded, validate during one streaming write/index pass, and persist a reusable validated key index/fingerprint.
- **Effort / scope:** L; cross-language performance.

#### RS-130 — Low — Dataframe adapter selection is stringly typed

- **Location / symbol:** `sdk/python/src/rawscope/adapters/__init__.py:25-45`, `select_adapter`.
- **Finding:** Adapter choice branches on `type(source).__module__` prefixes rather than supported protocols/types after lazy import.
- **Evidence and consequence:** Proxy/subclass/module relocation objects can be misclassified or rejected; unrelated classes in matching module namespaces can reach the wrong adapter.
- **Correction:** Lazily import an ecosystem only when its module is present, then use `isinstance` against the supported concrete/protocol types with clear fallback errors.
- **Effort / scope:** S-M; local API robustness.

#### Testing, documentation, and maintainability

#### RS-131 — High — Boundary-heavy parsers lack property/fuzz coverage

- **Location / symbol:** parser/conversion surfaces `crates/rawscope-data/src/local_dataset/csv.rs:215-348`, `crates/rawscope-data/src/local_dataset/parquet.rs:293-758`, session `crates/rawscope-session/src/manifest.rs:215-303`; no `fuzz/`, proptest dependency, or arbitrary-input harness exists in manifests.
- **Finding:** Deterministic examples cover ordinary cases, but arithmetic/type/shape boundaries found in RS-026 through RS-042 are not systematically generated.
- **Evidence and consequence:** Small changes in CSV inference, Arrow type dispatch, ranges, timestamps, row widths, or manifests can reintroduce panics/inconsistent acceptance without detection.
- **Correction:** Add focused property tests for range/bin invariants and table shapes, plus fuzz targets for session JSON and bounded CSV/Parquet schema/value dispatch. Each target must assert no panic and specific semantic invariants—not mere smoke execution.
- **Effort / scope:** L; cross-cutting tests.

#### RS-132 — High — GPU correctness is opt-out in normal testing

- **Location / symbol:** `crates/rawscope-render/tests/gpu_scatter_density.rs` and `crates/rawscope-render/tests/gpu_timeline_density.rs` ignored tests; `docs/BENCHMARKS.md:50-72`; absent CI hardware/backend job.
- **Finding:** Twelve GPU correctness tests are ignored by the workspace suite and there is no automated backend matrix.
- **Evidence and consequence:** They passed on this review's single Windows adapter, but shader/ABI/orientation/precision defects can merge unnoticed and backend-specific failures remain invisible.
- **Correction:** Keep normal tests hardware-independent, but add deterministic CPU/WGSL formula vectors and a scheduled/opt-in CI GPU job on at least one declared backend; record adapter/backend in results.
- **Effort / scope:** L; test infrastructure.

#### RS-133 — High — Critical cross-component regressions have no tests

- **Location / symbol:** filtered gate `apps/rawscope-workbench/src/app_brush.rs:139-160`; linked selection `apps/rawscope-workbench/src/app_selection.rs:21-49`; transactional updates `apps/rawscope-workbench/src/app_scatter_filter.rs:114-158` and `apps/rawscope-workbench/src/app_scatter_projection.rs:73-132`; bundle IO `apps/rawscope-workbench/src/app_report_bundle.rs:55-276`.
- **Finding:** Tests cover local phase/actions and happy-path artifacts, but do not prove filtered visible/summary/linked/export agreement, rollback on second-consumer GPU failure, or concurrent/partial bundle behaviour.
- **Evidence and consequence:** The repository's central evidence and state invariants are currently broken despite 371 passing tests.
- **Correction:** Add behavior tests around an injectable render/resource boundary and filesystem abstraction that deliberately fail the second stage and assert atomic old-or-new state; add a filtered selection golden contract proving one row set drives every consumer.
- **Effort / scope:** L; cross-cutting meaningful regression tests.

#### RS-134 — High — Benchmarks miss the paths behind performance risk

- **Location / symbol:** `docs/BENCHMARKS.md:12-78`; `crates/rawscope-render/benches/gpu_density.rs:15-95`; `crates/rawscope-data/benches/local_ingest.rs:15-254`; `apps/rawscope-workbench/benches/evidence_export.rs:30-235`.
- **Finding:** Benchmarks measure isolated CPU density, eager ingest up to 100k rows, helper-level GPU density with readback, and v2 export. They do not measure frame/gesture latency, resident scatter settle, timeline pipeline recreation, relief fragment cost, v5 export, peak RAM/VRAM, or multi-million-row admission.
- **Evidence and consequence:** The benchmark policy correctly disclaims broad claims, but the implementation has no evidence for its most performance-sensitive product paths.
- **Correction:** Add benchmark harnesses around real coordinator operations with named dataset sizes, p50/p95 gesture/settle latency, allocation/peak memory, upload/VRAM, and backend/hardware metadata. Keep results out of claims until reproducible.
- **Effort / scope:** L; performance engineering.

#### RS-135 — High — Production modules exceed repository structure policy

- **Location / symbol:** `crates/rawscope-data/src/local_dataset/parquet.rs:1-770`, `apps/rawscope-workbench/src/ui.rs:1-769`, `crates/rawscope-render/src/timeline_selection_export_v3.rs:1-725`, `crates/rawscope-render/src/timeline_selection_export.rs:1-700`, `crates/rawscope-render/src/scatter_selection_export_v3.rs:1-648`, `crates/rawscope-render/src/scatter_selection_export.rs:1-619`, `apps/rawscope-workbench/src/app_report_bundle.rs:1-526`, `crates/rawscope-render/src/scatter_inspection.rs:1-508`, `crates/rawscope-render/src/view_summaries.rs:1-434`, `apps/rawscope-workbench/src/ui_visual_encoding.rs:1-426`, `apps/rawscope-workbench/src/app_render_schedule.rs:1-419`, `crates/rawscope-render/src/scatter_point_reveal.rs:1-418`, `apps/rawscope-workbench/src/app_brush.rs:1-416`, `crates/rawscope-render/src/scatter_density_renderer.rs:1-409`, `crates/rawscope-render/src/gpu_timeline_density.rs:1-403`, `crates/rawscope-data/src/local_dataset/csv.rs:1-402`, `apps/rawscope-workbench/src/app.rs:1-401`.
- **Finding:** Seventeen production files are roughly 400 lines or larger and several combine contracts, algorithms, IO, formatting, and tests.
- **Evidence and consequence:** This violates the repository's own split-by-responsibility rule and makes targeted review/ownership difficult. The export files demonstrate copy growth rather than cohesive modules.
- **Correction:** Split along domain seams identified in section 4—schema model vs formatter, reader vs conversion, scheduler vs executor, state vs view projection—not arbitrary line counts.
- **Effort / scope:** L-XL; architectural organization.

#### RS-136 — High — `rawscope-render` is not a coherent crate boundary

- **Location / symbol:** module list/re-exports `crates/rawscope-render/src/lib.rs:3-216`; dependency manifest `crates/rawscope-render/Cargo.toml:7-14`.
- **Finding:** The crate publicly exposes GPU pipelines, viewports, brushes, axes, analytics summaries, missingness, dataset diff, comparison, drilldown, evidence models, serializers, and visual transitions.
- **Evidence and consequence:** Domain/evidence consumers must depend on WGPU/render code; the facade has a very broad semver surface and responsibility changes cannot be isolated.
- **Correction:** Move evidence contracts/serialization to `rawscope-evidence`, CPU analytical indexes/selections to `rawscope-analysis`, and leave `rawscope-render` with GPU/presentation resources consuming validated analysis snapshots.
- **Effort / scope:** XL; crate architecture.

#### RS-137 — Medium — Workbench modules are fragmented by action, not ownership

- **Location / symbol:** `apps/rawscope-workbench/src/main.rs:1-55`; state in `apps/rawscope-workbench/src/app.rs:64-156`.
- **Finding:** Dozens of `app_*` and `ui_*` siblings group individual operations while all share `WorkbenchApp`; module count creates navigation overhead without encapsulation.
- **Evidence and consequence:** A feature such as filtering spans state, action, scheduling, difference renderer, inspection, evidence, UI, and export modules with no owning vertical boundary.
- **Correction:** Organize vertical feature controllers with private state and a thin app facade; keep cross-feature messages typed and explicit.
- **Effort / scope:** XL; application architecture.

#### RS-138 — Medium — Architecture documentation records history, not enforceable boundaries

- **Location / symbol:** `docs/ARCHITECTURE.md:1-52`, especially the single milestone-responsibility paragraph at `:52`; actual module surface `crates/rawscope-render/src/lib.rs:3-216`.
- **Finding:** The document repeatedly describes planned/initial/milestone slices and compresses current ownership into one enormous paragraph. It does not state forbidden dependency directions, data ownership, concurrency boundaries, or evidence invariants in an actionable form.
- **Evidence and consequence:** External contributors cannot decide where new behavior belongs, and the render/workbench responsibility expansion appears sanctioned rather than flagged.
- **Correction:** Replace milestone prose with a current context map, allowed dependency table, state/data ownership, runtime sequences, invariants, and decision records for deliberate exceptions.
- **Effort / scope:** M; documentation tied to refactoring.

#### RS-139 — Medium — Historical feature plans contradict current behavior

- **Location / symbol:** `features/next-generation-gpu-visual-analytics/tasks.md:154-168`; `features/next-generation-gpu-visual-analytics/tasks/T008.md:221-226,253-260`; `features/next-generation-gpu-visual-analytics/tasks/T016.md:191-200`; current gate `apps/rawscope-workbench/src/app_brush.rs:139-143`.
- **Finding:** Completed task records state that T015 removes the filtered-export gate and that filtered evidence agrees, but the gate remains. The `features/` tree contains 55 Markdown files and roughly 13,400 lines of implementation history.
- **Evidence and consequence:** Stale completion narratives outweigh maintained product docs and are actively misleading during review/maintenance.
- **Correction:** Archive completed plans outside the primary contributor path, extract lasting decisions/invariants into maintained docs/tests, and correct completion status where behavior regressed or never landed.
- **Effort / scope:** M; documentation governance.

#### RS-140 — Medium — README overstates implemented guarantees

- **Location / symbol:** `README.md:27,42-44,68-76,129-139`; contradictory implementation at `apps/rawscope-workbench/src/app_brush.rs:139-143`, `apps/rawscope-workbench/src/app_report_bundle.rs:55-72,107-276`, and synchronous runtime findings.
- **Finding:** Phrases such as “filters ... and exported evidence preserve the path,” “production-oriented,” and “collision-safe” are stronger than the implementation supports.
- **Evidence and consequence:** The README correctly disclaims benchmarked performance, but still presents broken filtered export, non-atomic/racy bundles, and blocking interaction as established capabilities. Public trust suffers when claims are code-inconsistent.
- **Correction:** Narrow claims to currently proven behavior, link known limitations, and add doc/behavior checks for high-value product guarantees.
- **Effort / scope:** S now; ongoing documentation discipline.

#### RS-141 — Medium — Public API documentation is shallow despite broad exposure

- **Location / symbol:** crate roots `crates/rawscope-core/src/lib.rs:1-14`, `crates/rawscope-data/src/lib.rs:1-50`, `crates/rawscope-render/src/lib.rs:1-216`; manifests do not enable/document a `missing_docs` policy.
- **Finding:** Rustdoc builds, but broad re-exported APIs mostly have one-line item comments and lack crate-level usage, invariants, panic/error contracts, numerical precision, thread/device ownership, and semver intent.
- **Evidence and consequence:** Passing Rustdoc proves syntactic links, not that unknown consumers can use APIs safely. Many misuse cases catalogued above are undocumented.
- **Correction:** Reduce the public surface first, then add crate-level guides/examples and enforce `#![warn(missing_docs)]` or a reviewed equivalent for intentionally public library crates.
- **Effort / scope:** L; cross-cutting API/docs.

#### RS-142 — Low — There are no maintained Rust examples

- **Location / symbol:** workspace package manifests and crate roots; no `examples/` directories or `[[example]]` targets exist.
- **Finding:** The only runnable guidance is the full native application and README command lines.
- **Evidence and consequence:** External users cannot learn ingestion, CPU analysis, evidence validation, or headless GPU APIs through compile-checked minimal examples; the broad public library surface is effectively undocumented.
- **Correction:** After API stabilization, add a small number of real, compile-checked examples that exercise meaningful contracts (not fake demo apps or smoke binaries).
- **Effort / scope:** M; documentation/API.

#### RS-143 — Nit — Benchmark fixtures leave dead cleanup code

- **Location / symbol:** `crates/rawscope-data/benches/local_ingest.rs:236-251`, fixture-path creation and `remove_fixture`.
- **Finding:** The ingest benchmark creates temporary fixtures under its own path strategy but retains cleanup machinery that is not exercised consistently.
- **Evidence and consequence:** Repeated benchmark runs can leave files and dead code obscures the intended lifecycle. This is small but avoidable maintenance debt.
- **Correction:** Use an RAII temporary directory/fixture owner whose drop performs bounded cleanup, and remove unused helper paths.
- **Effort / scope:** S; local nit-level maintenance.

### 4. Architectural recommendations

The repository does not need a wholesale rewrite. It needs explicit ownership and validation seams, followed by incremental migration. The recommended target is:

```text
rawscope-core
  immutable IDs, checked ranges/domains, row/column IDs, generation tokens

rawscope-session-contracts        rawscope-data
  lightweight serde schema         source adapters, typed/chunked dataset store
             \                       /
              \                     /
               rawscope-analysis
               filters, projections, indexes, summaries, selections, diff
                    |          \
                    |           rawscope-evidence
                    |           canonical evidence + versioned readers/writers
                    v
               rawscope-render <---- rawscope-gpu
               presentation/GPU      adapter/device/recovery/resource limits
                    \                 /
                     rawscope-workbench
                     event routing, controllers, jobs, UI projection
```

Names are secondary; responsibilities and dependency rules are the point.

#### Recommended crate boundaries

1. **Keep `rawscope-core` dependency-free.** It should own only cross-domain value types: opaque IDs, checked observed extents versus non-empty display domains, row/column identifiers, and owner-minted generation tokens. It must not absorb Serde, Arrow, WGPU, evidence formatting, or UI concepts.
2. **Make `rawscope-data` an ingestion/store crate.** Source adapters should produce a typed, rectangular, generation-owned `DatasetStore` incrementally. Preserve raw evidence values separately from normalized analytical values. The store should expose stable column IDs, chunk iteration, typed columns/missing bitmaps, and explicit memory/storage budgets. It should not know about WGPU, brushes, Markdown, or UI filter widgets.
3. **Create `rawscope-analysis`.** Move filters, visual projection, missingness, dataset diff, aggregate indexes, inspection, summaries, and selection construction here. This crate should consume immutable dataset/cohort/viewport snapshots and return immutable results. One normative binning contract must feed summaries, inspection, selection, evidence, and WGSL parity vectors.
4. **Create `rawscope-evidence`.** Own a canonical validated evidence model, v1-v5/timeline wire adapters, readers, migrations, JSON/Markdown escaping, provenance redaction, and bundle-manifest schemas. It may depend on core/analysis/data contracts and Serde, but never WGPU/winit. Preserve old bytes through golden fixtures while eliminating duplicated business logic.
5. **Narrow `rawscope-render`.** It should own presentation pipelines, GPU resource generations, render-specific uniforms/shaders, overlays, and viewport projection required for drawing. It consumes validated analysis snapshots; it should not own dataset diff, source-row formatting, evidence schemas, or report IO.
6. **Keep `rawscope-gpu` narrow but production-capable.** Add configurable adapter policy, validated limits, error scopes, uncaptured/device-lost handling, and resource-recovery signals. Do not move renderer business logic into it.
7. **Split lightweight session contracts from profile implementation.** `rawscope-session-contracts` (or a narrow `rawscope-session`) should parse/validate a bounded schema without Arrow/Parquet. Profile IDs that appear on the wire belong in a dependency-light contract layer; concrete profile hints stay in data/analysis.
8. **Make `rawscope-workbench` composition-only.** The winit application should route events/commands among controllers, render a borrowed/cached UI projection, and commit completed jobs. It should not scan datasets, define evidence versions, or perform report filesystem transactions inline.

#### Recommended module boundaries

- `rawscope-data::ingest::{csv, parquet}`: adapter-specific decoding only; both emit the same typed cell/column contract and normalization metadata.
- `rawscope-data::store`: dataset generation, typed columns, raw evidence store, chunk ownership, memory accounting, and stable `ColumnId`/`RowId` invariants.
- `rawscope-analysis::cohort`: private `FilterSet`, validated predicates, missing/invalid policy, owner-minted `CohortGeneration`, reusable mask/index.
- `rawscope-analysis::selection`: one `SelectionSnapshot` containing the authoritative sorted row IDs, counts, extents, typed samples, and generation tuple.
- `rawscope-analysis::{density, inspection, missingness, diff}`: each consumes immutable store/cohort/view inputs; common binning lives in `density::binning` rather than being copied.
- `rawscope-evidence::{model, validate, wire::v1..v5, markdown, bundle}`: canonical model and isolated wire compatibility. Shared source/query/cohort components are typed, not extracted through `serde_json::Value`.
- `rawscope-render::{scatter, timeline, overlay, transition}`: vertical renderer owners, each with resident compute/presentation resources and explicit resource generations.
- `rawscope-workbench::{dataset_controller, scatter_controller, timeline_controller, selection_controller, export_controller}`: private state per feature and typed commands/results. Avoid a new generic `utils`, `helpers`, or `common` bucket.

#### Public API and domain types

- Replace public invariant fields with checked constructors and accessors. Use `Result` for recoverable external input. Distinguish `ObservedExtent<T>` (may be a single value) from `DisplayDomain<T>` (strict non-zero span).
- Replace duplicate `VisualSelectionKind + VisualSelectionGeometry` with one geometry enum. A `SelectionSnapshot` should carry `DatasetGeneration`, `CohortGeneration`, and `ViewportGeneration`; stale snapshots then fail by type/state check instead of convention.
- Replace public numeric revisions with opaque generations minted by the owner. Resource/cache methods accept the generation and payload as one value so “same revision, different data” cannot be expressed.
- Replace `include_missing: bool` with an enum that also distinguishes invalid/non-finite cells. Replace string column references after ingestion with `ColumnId`; retain names for display/provenance.
- Model GPU configuration as validated types checked against `DeviceLimits`. Renderer update APIs should not accept independent compute config and field metadata.
- Keep traits only at real polymorphic seams: source reader/chunk decoder, analysis job executor, evidence sink, and possibly GPU device abstraction for fault injection. Do not introduce traits for every data struct or controller.

#### State ownership and data flow

The desired state progression is monotonic and transactional:

```text
source
  -> DatasetGeneration { typed store, raw evidence store, identity, budget }
  -> CohortGeneration { validated filters, mask/index, counts }
  -> ViewGeneration { viewport, projection, grid/quality }
  -> RenderGeneration { resident GPU buffers, no CPU authority }
  -> SelectionSnapshot { authoritative IDs/summaries/samples }
  -> ValidatedEvidence
  -> transactional bundle commit
```

Each downstream object records the upstream generation IDs. A result built for an old generation is discarded, not partially applied. Main/difference renderers share a dataset GPU resource; cohort-specific masks/count fields remain separate. CPU source data remains authoritative for evidence; GPU state is a derived cache.

#### Concurrency boundaries

- The winit thread owns windows, UI integration, and submission/presentation coordination only.
- Bounded workers own file IO, decoding, profiling, analysis scans, evidence construction, and serialization. Jobs carry cancellation and generation IDs; queues are bounded and replace/coalesce obsolete interactive jobs.
- WGPU readback is callback/future-driven. No `wait_indefinitely` occurs on the event thread. Interactive rendering uses resident max/reduction data and no full readback; exact evidence readback, if truly required, completes asynchronously.
- Timeline and scatter use one scheduling vocabulary: immediate reprojection, bounded preview, one exact settle. A “job in flight” has an actual completion event.
- Filesystem export writes a temporary bundle generation and atomically renames it; the UI receives progress/completion/failure messages.

#### Error boundaries and observability

- Adapter errors include source format, path, chunk/record location, column ID/name, and preserved source chain without embedding large/sensitive values.
- Analysis errors describe invalid typed invariants (row shape, mask alignment, range/bin configuration), not WGPU implementation details.
- GPU errors classify unsupported limits, validation, device loss, surface recovery, timeout/cancellation, and permanent failure. Recovery is a state transition.
- Workbench errors retain typed causes and a recovery class (`retry`, `choose another source`, `reduce quality`, `restart GPU`, `fatal`). UI uses this classification rather than string matching.
- Tracing spans should cover `load_dataset`, `build_cohort`, `settle_density`, `build_selection`, and `export_bundle` with dataset/generation/row/grid context. Do not log full source paths or values by default.

#### Patterns to introduce

- Opaque checked newtypes and immutable snapshot/generation objects.
- Staged builders only where construction genuinely has multiple validated inputs (evidence query and GPU resource plan); simple values should use checked constructors.
- Chunked typed storage, reusable missing/key indexes, and shared immutable GPU dataset resources.
- Versioned wire adapters over one canonical evidence model, with readers and validation.
- Transactional prepare/commit for dataset, filter, projection, renderer, and bundle changes.
- Bounded job execution with cancellation/coalescing and explicit completion events.
- Fault injection at render/export boundaries for atomicity tests.

#### Patterns to remove

- Public fields as validation contracts; panicking public constructors; caller-supplied revision integers.
- `WorkbenchApp` as shared mutable storage for every feature.
- Fake-async functions that block internally and synchronous “in-flight” scheduler state.
- JSON-string/`Value` surgery between evidence versions and duplicated serializer business logic.
- Independent scans that rediscover the same selection/cohort; eager all-row/all-column string duplication.
- Per-interaction pipeline/buffer/bind-group creation and full GPU readback.
- Hard-coded format names in logs, silent `Err(_)` resets, and broad `Box<dyn Error>` at recovery boundaries.

### 5. Prioritised remediation plan

The order matters. Do not start with cosmetic module moves; first stop producing incorrect evidence and unresponsive interactions, then establish the ownership seams needed to prevent recurrence.

#### 1. Immediate correctness and safety fixes

1. **Block public release now** on RS-001, RS-002, RS-003, and RS-005. Add licence texts, decide publish/private packages, and establish the minimal required CI/advisory gate before accepting unrelated feature work.
2. **Fix the UI-freeze defects:** add non-progress termination to axis ticks (RS-072), stop timeline per-pointer pipeline recreation/readback (RS-049/050/110), and place a temporary bounded/coalesced interaction policy around exact settling (RS-108).
3. **Fix visible/data disagreement:** unify timeline y orientation (RS-053), integer timeline binning (RS-054/055), and reject invalid GPU resource/config dimensions (RS-057/058/060/067).
4. **Create one filtered `SelectionSnapshot`** and route summary, drilldown, linked selection, evidence, and aggregate context through it. This fixes RS-083, RS-084, RS-085, and RS-092 together without weakening legacy schema truth.
5. **Fix ingestion corruption/acceptance defects:** synthetic small counts/custom ranges (RS-026/027), zero CSV limit and u64 inference (RS-028/029), raw CSV preservation (RS-030), Float16 and unsupported Parquet types (RS-032/033), and explicit numeric quantization (RS-034).
6. **Make workbench mutations atomic:** filter, projection, and demo-switch prepare/commit/rollback (RS-111/113/114). Until the larger controller refactor, use local staged state rather than mutating the live app progressively.
7. **Harden durable output:** validate before serialization, enforce schema constants, escape Markdown, redact/opt in absolute paths, and make bundle naming/writes transactional (RS-090, RS-093, RS-094, RS-099, RS-101/102).
8. **Fix the immediate quality gates:** Clippy RS-004 and a configured `deny.toml` RS-006. Do not suppress advisories/lints without narrow written rationale.

Dependencies: the canonical selection snapshot precedes filtered v4/v5 export; a checked binning contract precedes GPU/CPU parity fixes; validated evidence precedes transactional export; staged state precedes fault-injection tests.

#### 2. Structural refactoring

1. Introduce dependency-light session/profile contracts (RS-118) without changing wire bytes.
2. Build a private `DatasetStore` façade around current loaded data, validate rectangular rows/unique columns once, and migrate consumers behind stable `ColumnId`s. This enables fixes for RS-022, RS-037, RS-042, RS-079, and RS-080.
3. Extract `rawscope-analysis` responsibilities from render: binning, filters, missingness, diff, summaries, inspection, and selections. Move behavior with its meaningful tests; do not perform mechanical file shuffling.
4. Extract `rawscope-evidence` with canonical validated models and version adapters. Preserve v1-v5 bytes using existing golden fixtures before deleting duplicated paths.
5. Replace `WorkbenchApp` shared state with vertical controllers one feature at a time. Start with dataset/session generation, then cohort/selection, then render scheduling/export jobs.
6. Establish shared immutable GPU dataset resources so main/difference renderers stop triplicating points (RS-063/115).

Dependencies: the dataset façade can land before chunked storage; analysis extraction should precede evidence extraction only far enough to give evidence a stable selection/query model; controller extraction should consume these stable domain APIs.

#### 3. API redesign

1. Make ranges, grid/lane sizes, source tables, filter sets, evidence keys, selection/evidence structs, and render configs private/checked (RS-016-024, RS-039, RS-066-068, RS-081).
2. Separate observed singleton extents from non-empty display domains (RS-020), then remove epsilon-fabricated evidence.
3. Replace revisions with opaque generation tokens and make update payload/generation inseparable (RS-023).
4. Introduce architecture-independent wire types (`u64` limit) and normalized identifier newtypes shared by Rust/Python test vectors (RS-119-124).
5. Publish only intentionally supported crates/items. Add panic/error/thread/device/precision contracts and compile-checked examples after the surface is reduced (RS-007, RS-141/142).

This phase contains semver-breaking changes. It should precede any claim that 0.1 public APIs are stable.

#### 4. Performance work

1. Implement resident timeline compute/presentation resources and real asynchronous/coalesced scheduling (RS-049/050/052/109/110).
2. Stream/chunk ingestion and introduce an explicit evidence-retention/memory budget (RS-031/041/044/115). Measure peak memory before choosing cache sizes.
3. Share CPU selection/cohort/missing/key indexes; eliminate repeated scans and per-bin eager samples (RS-038/076/079/092/129).
4. Share GPU point resources, cache bind groups, use encoder clears, and validate device limits (RS-051/058/063/064).
5. Replace relief's fragment read amplification with precomputed field/normal textures and quality tiers (RS-062).
6. Move export/analysis IO to bounded jobs and render UI from borrowed/revisioned presentation state (RS-104/107/112).
7. Only after these changes, run the expanded benchmark matrix in step 5 and publish hardware/date/dataset-specific results.

#### 5. Test improvements

1. Add exact regressions for every confirmed correctness bug in steps 1-3: small synthetic counts, custom/max ranges, CSV zero/u64, Float16, timeline top/bottom lane, >2^24 timestamp bins, large f32 tick progress, filtered selection agreement, and Python boolean limits.
2. Add CPU/WGSL parity vectors for boundary binning and host/WGSL layout/semantic checks (RS-048, RS-054/055/070). Keep hardware tests separate but scheduled.
3. Add fault-injection behavior tests proving filter/projection/demo and bundle operations are old-or-new, never partial (RS-101/102/111/113/114/126).
4. Add canonical evidence reader/writer round trips, strict invalid fixtures, legacy golden bytes, Markdown adversarial cells, and path-redaction tests.
5. Add cross-language session/schema fixtures consumed by both Rust and Python for identifiers, limits, Arrow binding types, Float16, and evidence keys.
6. Add targeted property/fuzz tests for parser/range/bin/table invariants (RS-131). Avoid smoke binaries and broad test churn unrelated to these contracts.
7. Expand benchmarks to real coordinator latency and RAM/VRAM as specified by RS-134; record no product claim without named evidence.

#### 6. Documentation and OSS readiness

1. Add licences, contribution/security/conduct policies, issue/PR templates, support channels, changelog, and release/version policy (RS-001, RS-010/011).
2. Complete Cargo/Python metadata, publication intent, package licences/readmes, and reproducible packaging checks (RS-002, RS-007, RS-015).
3. Declare MSRV/toolchain and supported OS/architecture/GPU/backend/Python matrix; implement that matrix in CI (RS-003, RS-008/009).
4. Replace milestone architecture prose with current boundaries/invariants and archive historical plans after extracting durable decisions (RS-138/139).
5. Correct README capability claims immediately and add missing v3 schema docs plus a version index (RS-105/140).
6. Document numerical precision, evidence provenance/privacy, sampling/truncation, thread/device ownership, errors, and recovery for the reduced public API.

#### 7. Long-term improvements

1. Establish semver and evidence-schema compatibility governance with deprecation windows and migration tooling.
2. Add scheduled dependency updates, advisory review ownership, SBOM/provenance, signed/checksummed native releases, and reproducible build records.
3. Maintain a GPU/backend qualification suite and device-loss/suspend/resume exercises on declared platforms.
4. Introduce storage strategies for data larger than RAM only after the typed/chunked store contract and evidence-retention policy are stable.
5. Revisit approximate profiling/heavy-hitter algorithms only with explicit accuracy bounds and evidence disclosure.
6. Track end-to-end latency and memory budgets as release criteria, not aspirational README language.

### 6. Positive foundations

- The Cargo dependency graph is acyclic and `rawscope-core` is genuinely dependency-free. That is a sound base for stricter domain contracts.
- There is no Rust `unsafe`, FFI, raw-pointer ownership, or manual `Send`/`Sync`. Low-level risk is concentrated in inspectable WGPU/WGSL ABI boundaries.
- Row IDs, deterministic lowest-ID sampling, mask-alignment errors, and explicit CPU reference implementations provide useful raw material for a canonical analysis/evidence path.
- The test suite is substantive rather than decorative: 371 Rust tests and 13 Python tests passed, and all 12 opt-in GPU correctness tests passed on the review adapter. The problem is missing cross-component invariants, not an absence of testing effort.
- Structured `tracing` and source-preserving domain errors already exist in many IO/GPU paths; they can support the recommended workflow spans and recovery classification.
- Scatter's resident double-buffered GPU state, explicit readback policy, preview/exact quality concepts, and viewport reprojection are worth preserving after the scheduler becomes genuinely asynchronous and transitions become generation-correct.
- The benchmark document explicitly refuses unsupported broad performance claims and names commands, sizes, hardware caveats, and GPU opt-in rules. Expand that discipline to the actual interactive path rather than discarding it.
- Python process launch correctly uses argument vectors with `shell=False`, and manifest replacement is atomic. Those decisions should be extended to temporary cleanup and whole-bundle transactions.
- Version compatibility has been treated as a real requirement. Preserve v1-v5 bytes and public semantics with typed adapters/golden fixtures while removing the duplicated implementation.
