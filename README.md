# RawScope

See the shape before writing the query.

RawScope is a GPU-scale visual analytics engine for large raw datasets. It helps analysts, researchers, data scientists, and big data engineers visually inspect the shape of data before they know exactly what SQL query, notebook analysis, dashboard, or model they need.

Current status: early design/scaffold.

## Target Users

- Analysts investigating changes, anomalies, data quality, and top contributors
- Researchers exploring large experimental datasets, embeddings, clusters, and repeatable views
- Data scientists checking feature drift, label imbalance, outliers, and cohort differences
- Big data engineers debugging pipeline regressions, missing partitions, schema drift, null spikes, and freshness gaps

## First Milestone

The current scaffold establishes docs, boundaries, and a compiling Rust workspace. The next implementation milestone is deterministic synthetic point/event data plus a CPU-side density reference. The first GPU milestone is a synthetic density rendering demo.

## Current Workbench Demo

Run the native scatter-density demo with:

```powershell
cargo run -p rawscope-workbench
```

Controls:

- Mouse wheel: zoom the scatter-density viewport around the cursor.
- Left or middle mouse drag: pan the current data viewport.
- Right mouse drag or Shift + left mouse drag: create or replace a rectangular brush selection.
- `Escape`: clear the current brush selection.
- `R`: reset to the full synthetic data range.
- `1`: switch to 20,000 synthetic points.
- `2`: switch to 200,000 synthetic points.
- `3`: switch to 1,000,000 synthetic points.
- `4`: switch to 5,000,000 synthetic points.
- `P` or `F12`: screenshot capture is currently skipped in-app; use the OS screenshot tool for now.

The demo uses deterministic synthetic point data, recomputes GPU density counts for the current viewport, and shows compact diagnostics in the window title: point count, grid size, viewport ranges, max bin count, selected-region summary, redraw count, latest CPU-observed update/frame timings, and adapter/backend. The selected-region summary is computed on CPU from synthetic records and includes selected row count, percentage, brush x/y ranges, selected data extents, category counts, and top category. These diagnostics are smoke observations, not GPU benchmark results.

Brush overlay note: Milestone 3E keeps the brush visible through title-bar diagnostics and structured logs rather than drawing a rectangle overlay. A render-pass overlay can be added later without changing the CPU summary contract.

Screenshot capture note: in-app screenshot capture is intentionally deferred because native surface readback and image encoding would add a dedicated capture path or extra dependencies. For Milestone 3D, OS-level screenshots are the recommended path.

## Repo Layout

- `docs/`: project intent, architecture, goals, MVP milestones, and agent guidance
- `crates/rawscope-core`: shared foundational types and selection/view concepts
- `crates/rawscope-data`: future data abstractions, dataset metadata, row ids, and chunk storage
- `crates/rawscope-gpu`: future GPU device/session and resource management
- `crates/rawscope-render`: future density, heatmap, timeline, and selection rendering logic
- `crates/rawscope-egui`: future egui integration layer
- `apps/rawscope-workbench`: native desktop app shell

## Non-Goals

RawScope is not a generic charting library, BI dashboard builder, Tableau/Grafana/Plotly clone, SQL database, dataframe engine, notebook replacement, cloud analytics platform, or general-purpose UI framework. It should complement these tools by focusing on local-first visual exploration of large raw datasets, with a clear path from visible patterns back to row-level evidence.

## Development Status

This repository is intentionally small right now. Do not add file import, egui UI, Tauri packaging, or broad abstractions until the synthetic scatter-density path is stable.
