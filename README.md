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

Run the native timeline-density demo with:

```powershell
cargo run -p rawscope-workbench -- --demo timeline
```

Controls:

- Mouse wheel: zoom the scatter-density viewport around the cursor.
- Left or middle mouse drag: pan the current data viewport.
- Right mouse drag or Shift + left mouse drag: create or replace a visible rectangular brush selection.
- `Escape`: clear the current brush selection.
- `E`: export the latest finalized selection evidence to JSON and Markdown under `target/rawscope-exports/`.
- `R`: reset to the full synthetic data range.
- `1`: switch to 20,000 synthetic points.
- `2`: switch to 200,000 synthetic points.
- `3`: switch to 1,000,000 synthetic points.
- `4`: switch to 5,000,000 synthetic points.
- `P` or `F12`: screenshot capture is currently skipped in-app; use the OS screenshot tool for now.

The demo uses deterministic synthetic point data, recomputes GPU density counts for the current viewport, and shows compact diagnostics in the window title: point count, grid size, viewport ranges, max bin count, selected-region summary, redraw count, latest CPU-observed update/frame timings, and adapter/backend. The selected-region summary is computed on CPU from synthetic records and includes selected row count, percentage, brush x/y ranges, selected data extents, category counts, and top category. These diagnostics are smoke observations, not GPU benchmark results.

Timeline demo note: `--demo timeline` uses deterministic synthetic event data and renders GPU timeline-density counts as a simple full-window view where x is time, y is lane/source, and intensity is event count. The injected spike, gap, and stale-lane patterns should be visible. Timeline rendering is a visual proof only: no axes, labels, brushing, row evidence, file import, or arbitrary timestamp normalization are included yet. The current GPU timeline path deliberately keeps the `u32` time-span guard from Milestone 4A.

Brush overlay note: the current rectangle overlay is intentionally simple: a faint amber fill with a brighter border, rendered after the density pass. During drag, the rectangle follows screen-space mouse movement. Once finalized, the selection is anchored to data-space x/y ranges, and the overlay is projected back into the current viewport after zoom, pan, resize, or reset. Fully offscreen selections are hidden; partially visible selections are clamped to the viewport edge. Preset changes clear the brush because the synthetic dataset changes.

Selection evidence note: finalized brushes also build a small CPU-side evidence object from synthetic records. Evidence includes selected counts, category counts, min/max x/y, brush range, dataset seed/row count, and a deterministic sample of the lowest selected row ids plus their synthetic records. This is logged once when the brush finalizes and is not a row table UI or GPU row-id path.

Evidence export note: pressing `E` writes the cached selection evidence to `target/rawscope-exports/scatter-selection-<unix-ms>-<counter>.json` and `.md`, then appends a matching entry to `target/rawscope-exports/manifest.jsonl`. If no finalized brush evidence exists, the app logs a warning and does not write files. These artifacts are deterministic synthetic CPU-side evidence, not the final report system. The JSON schema is documented in `docs/schemas/scatter-selection-evidence-v1.md`.

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
