# Technical Decisions

## ADR-0001: Rust First

- Decision: Use Rust as the implementation language for the core engine and native application scaffold.
- Status: Accepted
- Context: RawScope needs explicit control over data layout, strong crate boundaries, deterministic tests, and a path to native GPU integration.
- Consequences: Early code should stay simple and idiomatic, with boring APIs, explicit ownership, and small crates.

## ADR-0002: WGPU Later For GPU Compute And Rendering

- Decision: Use `wgpu` for the first GPU bootstrap and future compute/render pipelines.
- Status: Accepted
- Context: Milestone 1 established deterministic synthetic data and CPU reference density outputs. Milestone 2 needs a native GPU device/session and surface clear path before implementing density rendering.
- Consequences: `rawscope-gpu` owns `wgpu`, adapter/device selection, surface configuration, resize handling, and clear-frame presentation. No visual analytics performance claims follow from this bootstrap.

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

## ADR-0004A: Winit For Minimal Native Window Bootstrap

- Decision: Use `winit` directly for the Milestone 2 native workbench window and event loop.
- Status: Accepted
- Context: The project needs to prove WGPU surface creation, resizing, and clear-frame presentation before adding egui or broader UI concerns.
- Consequences: `rawscope-workbench` owns the `winit` event loop for now. egui remains deferred until the raw WGPU path is proven.

## ADR-0004B: Pollster For Blocking WGPU Initialization

- Decision: Use `pollster` in the workbench binary to block on async WGPU initialization.
- Status: Accepted
- Context: `wgpu` adapter/device requests are async, while Milestone 2 does not need an async runtime.
- Consequences: The app avoids a runtime dependency and keeps initialization straightforward.

## ADR-0004C: Tracing For Startup Adapter Metadata

- Decision: Use `tracing` for structured GPU adapter metadata and `tracing-subscriber` in the workbench binary to emit logs.
- Status: Accepted
- Context: Milestone 2 requires adapter/backend/surface metadata without ad-hoc production `println!` debugging.
- Consequences: `rawscope-gpu` can emit structured adapter metadata, and the workbench owns subscriber setup.

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

## ADR-0011: Bytemuck For GPU Buffer Packing

- Decision: Use `bytemuck` in `rawscope-render` for explicit POD structs passed to WGPU buffers.
- Status: Accepted
- Context: Milestone 3A needs deterministic scatter-density compute tests that upload point coordinates and uniform parameters with predictable layouts.
- Consequences: GPU-facing structs must stay `#[repr(C)]` and derive `Pod`/`Zeroable`. This dependency is scoped to the render crate for now and does not imply a broader serialization or columnar memory model.

## ADR-0012: Ignored Local GPU Correctness Tests

- Decision: Keep GPU scatter-density correctness tests ignored by default and run them manually on machines with a reliable WGPU adapter.
- Status: Accepted
- Context: The CPU reference tests should remain stable in normal workspace test runs, while GPU adapter availability varies across CI and developer machines.
- Consequences: `cargo test --workspace` validates non-GPU behavior and compiles ignored GPU tests. Run `cargo test -p rawscope-render --test gpu_scatter_density -- --ignored --nocapture` to compare GPU counts against CPU counts locally.

## ADR-0013: Single Device For Workbench Scatter Density

- Decision: Run the workbench scatter-density compute and render passes on the same WGPU device/queue owned by the window `GpuContext`.
- Status: Accepted
- Context: Milestone 3B needs a visible density view, but separate compute and render devices would add synchronization and ownership complexity before the rendering path is proven.
- Consequences: `rawscope-gpu` exposes a small `render_frame` hook and read-only device/queue accessors. `rawscope-render` can build render resources against the window context while the headless `ComputeContext` remains available for ignored correctness tests.

## ADR-0014: Log-Scaled Proof Colour Mapping

- Decision: Use a simple log-scaled density colour ramp for the first visible scatter-density proof.
- Status: Accepted
- Context: Synthetic data has dense clusters plus sparse outliers, so linear scaling can wash out sparse regions or saturate clusters.
- Consequences: The workbench view makes dense bins brighter while keeping sparse outliers faintly visible. This is a visualization choice, not a performance or perceptual-quality claim.

## ADR-0015: Benchmark Timing Deferred

- Decision: Keep benchmark-style timing out of the workbench title and routine logs.
- Status: Accepted
- Context: Proper GPU timestamp queries are not part of this slice, and CPU-observed timings can be mistaken for benchmark evidence.
- Consequences: GPU execution timing and benchmark evidence remain deferred until a dedicated benchmark path exists.

## ADR-0016: Serde For Evidence Artifact Serialization

- Decision: Use `serde` and `serde_json` in `rawscope-render` for scatter selection evidence JSON artifacts.
- Status: Accepted
- Context: Milestone 3I needs versioned JSON export of cached synthetic selection evidence without hand-rolled serialization logic or a broader report system.
- Consequences: Serialization dependencies are scoped to the render crate. Core/data types are adapted into small artifact DTOs instead of deriving serialization broadly, and Markdown remains a simple deterministic formatter.

## ADR-0017: Startup Demo Selection Before Live Mode Switching

- Decision: Select the workbench visual proof with `--demo scatter` or `--demo timeline`, defaulting to scatter.
- Status: Accepted
- Context: Milestone 4B needs to show the timeline-density visual path without adding a UI framework, live mode switching, or timeline interaction state.
- Consequences: Scatter controls remain unchanged in the default demo, timeline mode stays focused on rendering deterministic synthetic event density, and richer workbench mode management remains deferred.
