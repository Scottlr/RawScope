# RawScope

See the shape before writing the query.

RawScope is a GPU-scale visual analytics engine for large raw datasets. It helps analysts, researchers, data scientists, and big data engineers visually inspect the shape of data before they know exactly what SQL query, notebook analysis, dashboard, or model they need.

Current status: early native workbench with deterministic synthetic data, CPU reference density outputs, WGPU scatter/timeline density rendering, a small egui control shell, a CPU-backed local missingness slice, local CSV and Parquet loading, and local evidence report-bundle export.

## Target Users

- Analysts investigating changes, anomalies, data quality, and top contributors
- Researchers exploring large experimental datasets, embeddings, clusters, and repeatable views
- Data scientists checking feature drift, label imbalance, outliers, and cohort differences
- Big data engineers debugging pipeline regressions, missing partitions, schema drift, null spikes, and freshness gaps

## Goals

RawScope is built around five product and technical goals:

- Show full-dataset visual shape before users commit to a query, dashboard, notebook analysis, or model.
- Keep zooming, panning, brushing, and future linked views smooth as datasets grow.
- Bridge visual patterns back to row-level evidence through stable row ids, selection summaries, and exportable artifacts.
- Stay local-first for private datasets and early forensic exploration.
- Produce reproducible evidence reports that preserve view context, selected regions, summaries, and sampled or exact supporting rows.

See `docs/GOALS.md` for the fuller goal model.

## Supported Today

RawScope currently supports:

- Deterministic synthetic scatter point datasets with stable `RowId`s, category labels, dense clusters, background points, and sparse outliers.
- Deterministic synthetic timeline event datasets with stable `RowId`s, lanes, injected spike/gap/stale-lane/high-value patterns, and event types.
- CPU reference scatter and timeline density grids for deterministic correctness checks.
- WGPU bootstrap for native window rendering, surface resize handling, adapter metadata logging, and headless compute tests.
- GPU scatter-density count binning and simple fullscreen log-scaled scatter density rendering.
- GPU timeline-density count binning and simple fullscreen log-scaled timeline density rendering.
- Mouse-wheel zoom, drag pan, reset, and viewport re-binning for scatter and timeline modes.
- Deterministic scatter point-count presets from 20,000 to 5,000,000 synthetic points.
- Data-anchored rectangular brush selections for scatter and timeline views.
- CPU-side selected-region summaries for scatter brushes, including selected count, percentage, data extents, category counts, and top category.
- CPU-side selected-event summaries for timeline brushes, including selected count, percentage, lane counts, event-type counts, top lane/type, timestamp extent, and value extent.
- Deterministic CPU-side evidence objects for finalized scatter and timeline selections, including lowest-row-id samples.
- Local report-bundle export for scatter and timeline selections, including `evidence.json`, `evidence.md`, `manifest.json`, and a deterministic `visual-context.txt` placeholder.
- Local CSV and Parquet loading for scatter density with explicit numeric `--x`/`--y` columns.
- Local CSV and Parquet loading for timeline density with explicit integer `--time` and string or integer `--lane` columns.
- Optional `--limit <rows>` for local CSV and Parquet loading.
- A small egui workbench shell with visible view controls, selection status, axis labels, export status, and selected-row drilldown.
- A CPU-backed missingness heatmap slice for local datasets with retained source rows, including cell selection and row-id summaries for missing values.

These features are still correctness-first and visual-proof oriented. RawScope does not currently claim benchmarked performance, full GPU row-id preservation, screenshot/report capture, or production report workflows.

Benchmark commands, benchmark input sizes, GPU opt-in rules, and claim guidance live in `docs/BENCHMARKS.md`.

## Current Workbench

Run the native scatter-density demo with:

```powershell
cargo run -p rawscope-workbench
```

Run the native timeline-density demo with:

```powershell
cargo run -p rawscope-workbench -- --demo timeline
```

Run a local scatter-density view with explicit numeric columns from `.csv` or `.parquet` input:

```powershell
cargo run -p rawscope-workbench -- --demo scatter --input data.parquet --x latency_ms --y payload_size
```

Run a local timeline-density view with an integer timestamp and string or integer lane column from `.csv` or `.parquet` input:

```powershell
cargo run -p rawscope-workbench -- --demo timeline --input events.parquet --time timestamp --lane provider
```

Add `--limit <rows>` to cap the first imported rows. For local Parquet input, the current slice supports numeric scatter bindings plus integer timestamp and string or integer lane timeline bindings, retains source-row evidence, and records chunk metadata for the Arrow-backed read path.

Controls:

- Visible toolbar and panels: switch scatter or timeline mode, open the local-data missingness slice when source rows are available, inspect dataset/view status, clear selections, reset, export, and inspect selected rows.
- Mouse wheel: zoom the current viewport. Scatter zooms x/y around the cursor; timeline zooms the visible time range.
- Left or middle mouse drag: pan the current data viewport. Timeline mode pans time while lane mapping remains stable.
- Right mouse drag or Shift + left mouse drag: create or replace a visible rectangular brush selection.
- `Escape`: clear the current brush selection.
- `E`: export the latest finalized selection evidence for the active demo as a local report bundle under `target/rawscope-exports/`.
- `R`: reset to the full synthetic data range.
- `1`: switch to 20,000 synthetic points.
- `2`: switch to 200,000 synthetic points.
- `3`: switch to 1,000,000 synthetic points.
- `4`: switch to 5,000,000 synthetic points.
- `P` or `F12`: screenshot capture is currently skipped in-app; use the OS screenshot tool for now.

The scatter view uses deterministic synthetic point data by default, or local rows from CSV or Parquet input when `--input`, `--x`, and `--y` are provided. It recomputes GPU density counts for the current viewport and presents current state through the egui shell: a top toolbar for active view and selection actions, a right drilldown panel for selected rows, and a bottom status bar for axis ranges and export status. The window title remains a thin projection of that same UI state. Selected-region summaries are computed on CPU from the active point records and include selected row count, percentage, brush x/y ranges, selected data extents, category counts, and top category. The workbench does not present these values as GPU benchmark results.

Timeline note: `--demo timeline` uses deterministic synthetic event data by default, or local rows from CSV or Parquet input when `--input`, `--time`, and `--lane` are provided. It renders GPU timeline-density counts as a simple full-window view where x is time, y is lane/source, and intensity is event count. Mouse wheel zooms time, left or middle drag pans time, and `R` resets to the full time range; each viewport change re-bins the visible time range while lane mapping remains stable. Right-drag or Shift + left-drag creates a data-anchored timeline brush over a time/lane region, and the egui shell reports a CPU-side selected-event summary with event count, selected percentage, lane counts, event-type counts, top lane/type, timestamp extent, and value extent. Once finalized, timeline brushes also cache deterministic CPU-side evidence with the lowest selected row ids and sampled event records, and `E` exports that cached evidence. The injected spike, gap, and stale-lane patterns should be visible in synthetic mode. Timeline rendering is still intentionally narrow: no arbitrary timestamp normalization, linked multi-view coordination, or screenshot capture are included yet. The current GPU timeline path deliberately keeps the `u32` time-span guard from Milestone 4A.

Missingness note: when the current dataset comes from a local source with retained source rows, the egui shell also exposes a `Missingness` surface. The first slice is CPU-backed and uses retained source rows rather than a GPU pass. Cells are bucketed by row range and column, missingness is currently defined as trimmed-empty string values, and clicking a cell summarizes missing counts, selected columns, and sorted row ids that explain that region. This slice is intended as a data-quality foothold, not a full profiling system or benchmark path.

Brush overlay note: the current rectangle overlay is intentionally simple: a faint amber fill with a brighter border, rendered after the density pass. During drag, the rectangle follows screen-space mouse movement. Once finalized, the selection is anchored to data-space x/y ranges, and the overlay is projected back into the current viewport after zoom, pan, resize, or reset. Fully offscreen selections are hidden; partially visible selections are clamped to the viewport edge. Preset changes clear the brush because the synthetic dataset changes.

Selection evidence note: finalized brushes also build a small CPU-side evidence object from active records. Scatter evidence includes selected counts, category counts, min/max x/y, brush range, dataset seed/row count, and a deterministic sample of the lowest selected row ids plus point records. Timeline evidence includes selected counts, lane and event-type counts, selected timestamp/value ranges, brush time/lane ranges, dataset seed/row count, and a deterministic sample of the lowest selected row ids plus event records. This is logged once when the brush finalizes and remains a CPU evidence path rather than GPU row-id preservation.

Evidence export note: pressing `E` writes the active demo's cached selection evidence into a collision-safe bundle directory under `target/rawscope-exports/`. Scatter exports use `report-scatter-<unix-ms>-<counter>/`; timeline exports use `report-timeline-<unix-ms>-<counter>/`. Each bundle currently contains `evidence.json`, `evidence.md`, `manifest.json`, and `visual-context.txt`. The visual-context file is a deterministic placeholder that records the exact view configuration while native image capture remains deferred. If no finalized brush evidence exists in the active demo, the app logs a warning and does not write files. These bundles are local-first CPU-side evidence artifacts, not cloud publishing or a final screenshot pipeline.

Screenshot capture note: in-app screenshot capture is intentionally deferred because native surface readback and image encoding would add a dedicated capture path or extra dependencies. For Milestone 3D, OS-level screenshots are the recommended path.

## Current Crate Responsibilities

- `docs/`: project intent, architecture, goals, MVP milestones, and agent guidance
- `crates/rawscope-core`: shared foundational types, row ids, ranges, grid sizes, row-count density grids, and count-only density grids
- `crates/rawscope-data`: deterministic synthetic point/event data, local CSV and Parquet loading, dataset metadata, schema summaries, retained source rows, and chunk metadata for current views
- `crates/rawscope-gpu`: WGPU surface context, headless compute context, adapter metadata, resize handling, and clear-frame status
- `crates/rawscope-render`: CPU density references, GPU scatter/timeline density compute, simple density renderers, viewport math, brush geometry, selection summaries/evidence, and evidence artifact formatting
- `apps/rawscope-workbench`: native `winit` workbench that coordinates startup args, WGPU context, active view state, input handling, brushing, title-bar summaries, and evidence export

## Deferred Work

The next larger areas remain intentionally deferred:

- Broader UI work such as file dialogs, richer layout management, and screenshot/report capture.
- GPU row-id preservation and exact row drilldown from rendered density bins.
- Broader Parquet logical-type coverage and richer columnar ownership beyond the current scatter/timeline bindings and chunk metadata.
- Linked multi-view selection, selected-vs-baseline comparison, and visual query persistence across scatter, timeline, and missingness.
- Native screenshot/readback capture and image-backed visual context inside report bundles.
- Arbitrary timestamp normalization for timeline data beyond the current `u32` GPU time-span guard.
- Broader performance analysis, optimization follow-up, and claim-backed benchmark result tracking.
- Tauri, web/WASM, cloud workflows, plugin systems, SQL/DataFusion, and dataframe-style execution.

## Non-Goals

RawScope is not a generic charting library, BI dashboard builder, Tableau/Grafana/Plotly clone, SQL database, dataframe engine, notebook replacement, cloud analytics platform, or general-purpose UI framework. It should complement these tools by focusing on local-first visual exploration of large raw datasets, with a clear path from visible patterns back to row-level evidence.

## Development Status

This repository is still intentionally small and milestone-driven. The current workbench proves synthetic and local CSV/Parquet scatter/timeline density workflows, GPU-side count aggregation, a restrained egui control shell, data-anchored brushing, CPU-side evidence, and local export artifacts. Keep future work focused on RawScope's visual exploration and row-evidence path; avoid broad import systems, generic charting features, BI/dashboard behavior, Tauri packaging, cloud services, plugin systems, or dataframe/query abstractions until the product milestones call for them.
