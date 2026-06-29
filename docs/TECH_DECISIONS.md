# Technical Decisions

## ADR-0001: Rust First

- Decision: Use Rust as the implementation language for the core engine and native application scaffold.
- Status: Accepted
- Context: RawScope needs explicit control over data layout, strong crate boundaries, deterministic tests, and a path to native GPU integration.
- Consequences: Early code should stay simple and idiomatic, with boring APIs, explicit ownership, and small crates.

## ADR-0002: WGPU Later For GPU Compute And Rendering

- Decision: Plan around `wgpu` for future compute and render pipelines, but do not introduce it in the initial scaffold.
- Status: Accepted
- Context: The first pass is documentation and a compiling workspace. The first GPU work should happen after synthetic data and a CPU reference are stable.
- Consequences: The crate layout reserves a GPU layer while keeping the current repository dependency-light.

## ADR-0003: WGSL For Shaders

- Decision: Use WGSL for future shader code.
- Status: Accepted
- Context: WGSL is the shader language used by modern `wgpu` workflows and fits the cross-platform rendering direction.
- Consequences: Future shader loading, validation, and examples should assume WGSL assets and tooling.

## ADR-0004: egui/eframe First

- Decision: Use `egui` and `eframe` first when the GUI layer is introduced.
- Status: Accepted
- Context: RawScope needs a native desktop workbench before it needs a webview shell or browser deployment story.
- Consequences: UI integration should live in `rawscope-egui`, and application coordination should stay in `rawscope-workbench`.

## ADR-0005: Tauri Deferred

- Decision: Do not include Tauri in the initial implementation.
- Status: Deferred
- Context: Tauri introduces packaging, webview, and frontend concerns that are not needed for the first rendering milestones.
- Consequences: Native desktop work should proceed directly in Rust for now. Revisit Tauri only if product needs later justify it.

## ADR-0006: Arrow-Style Columnar Data Later

- Decision: Aim for Arrow-style columnar chunks as the future data representation, but do not implement that model yet.
- Status: Accepted
- Context: Columnar layouts align well with large scans, visual aggregation, row-id mappings, and future ecosystem integration.
- Consequences: Early data abstractions should leave space for chunked columnar ownership, schema summaries, and stable row identifiers.

## ADR-0007: DataFusion Deferred

- Decision: Keep DataFusion out of the initial scaffold and revisit it later for richer query planning.
- Status: Deferred
- Context: RawScope's first product milestone is visual exploration from synthetic data, not a general query engine.
- Consequences: The project can refine visual query and evidence needs before adopting a larger dependency and execution model.

## ADR-0008: Native Desktop First

- Decision: Target a native desktop workbench first.
- Status: Accepted
- Context: Local-first exploration, privacy, and direct GPU access matter more than browser reach in the first product phase.
- Consequences: Early workflows should center on a desktop analyst workbench.

## ADR-0009: WASM/Web Deferred

- Decision: Do not optimize for WASM or browser delivery in the initial scaffold.
- Status: Deferred
- Context: Web support adds platform constraints before the native rendering and interaction model is proven.
- Consequences: Crate APIs can remain portable where practical, but no web-specific implementation work is needed yet.

## ADR-0010: Benchmark Before Claims

- Decision: Avoid performance claims until they are backed by benchmarks.
- Status: Accepted
- Context: GPU-scale language is a product direction, not a measured result in the scaffold. Claims about speed, scale, or copy behavior need evidence.
- Consequences: Documentation and pull requests should use careful language, prefer "copy-minimising" where accurate, and add benchmarks before optimization claims.
