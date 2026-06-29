# Agent Instructions

## Project Summary

RawScope is a GPU-scale visual analytics engine for large raw datasets. It helps users see full-dataset shape before they know what query, notebook analysis, dashboard, or model they need.

Core tagline: See the shape before writing the query.

Core promise: pixels should be explainable back to rows.

## Hard Boundaries

- Preserve RawScope's product boundary.
- Do not turn RawScope into a generic charting library.
- Do not build a BI dashboard system.
- Do not make RawScope a SQL database, dataframe engine, notebook replacement, cloud analytics platform, or general UI framework.
- Do not add file import before synthetic data and CPU reference outputs are stable.
- Do not add WGPU before the workspace/docs and synthetic CPU reference path are stable.
- Do not add Tauri in the initial implementation.
- Do not build an early plugin system.
- Do not publish crates until the core API has stabilized.

## Coding Rules

- Keep code simple and idiomatic Rust.
- Keep APIs boring and explicit.
- Prefer small vertical slices over broad abstractions.
- Give derived ranges, thresholds, and predicates meaningful names near the loops or branches that use them.
- Prefer small crates and clean ownership boundaries.
- Keep data, GPU, render, and UI layers separate.
- Use egui/eframe first when the native app UI begins.
- Start with synthetic reproducible datasets before external files.
- Prefer deterministic tests.
- Use clear module-level rustdoc comments explaining each crate's purpose.
- Avoid premature abstraction and premature optimization.

## Dependency Rules

- Do not introduce large dependencies without documenting why.
- Add dependencies at the narrowest useful scope.
- Keep the scaffold dependency-light until the next milestone requires otherwise.
- Defer DataFusion, Arrow integration, Tauri, and WASM/web support until their milestones justify them.

## Performance-Claim Rules

- Do not claim zero-copy unless technically exact.
- Use copy-minimising language where appropriate.
- Do not claim performance without benchmark evidence.
- Add benchmarks before making optimization claims.
- Avoid claims like infinite scale, renders every row, or replaces BI.

## Documentation Rules

- Keep docs clear, direct, technical, and product-aware.
- Avoid hype, vague AI-powered language, and exaggerated performance claims.
- Update docs when architecture-changing code lands.
- Use precise phrases such as copy-minimising, GPU-side visual aggregation, density views, row-level evidence, local-first, visual query, linked brushing, and large raw datasets.

## Current Next Task

Build deterministic synthetic point/event datasets and a CPU-side density reference implementation for testing future GPU output.

## Suggested Future Prompt

Implement Milestone 1 for RawScope: add deterministic synthetic point/event datasets and a CPU-side density binning reference, with focused tests proving deterministic generation and stable density output. Keep dependencies minimal and do not add WGPU, egui, file import, or UI features yet.
