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

Run a local CSV scatter-density view with explicit numeric columns:

```powershell
cargo run -p rawscope-workbench -- --demo scatter --input data.csv --x latency_ms --y payload_size
```

Run a local CSV timeline-density view with an integer timestamp and string or integer lane column:

```powershell
cargo run -p rawscope-workbench -- --demo timeline --input events.csv --time timestamp --lane provider
```

Add `--limit <rows>` to cap the first imported rows. Parquet input is intentionally deferred in this slice; `.parquet` paths return a clear error instead of adding the Arrow/Parquet stack early.

Controls:

- Mouse wheel: zoom the current viewport. Scatter zooms x/y around the cursor; timeline zooms the visible time range.
- Left or middle mouse drag: pan the current data viewport. Timeline mode pans time while lane mapping remains stable.
- Right mouse drag or Shift + left mouse drag: create or replace a visible rectangular brush selection.
- `Escape`: clear the current brush selection.
- `E`: export the latest finalized selection evidence for the active demo to JSON and Markdown under `target/rawscope-exports/`.
- `R`: reset to the full synthetic data range.
- `1`: switch to 20,000 synthetic points.
- `2`: switch to 200,000 synthetic points.
- `3`: switch to 1,000,000 synthetic points.
- `4`: switch to 5,000,000 synthetic points.
- `P` or `F12`: screenshot capture is currently skipped in-app; use the OS screenshot tool for now.

The scatter demo uses deterministic synthetic point data by default, or local CSV rows when `--input`, `--x`, and `--y` are provided. It recomputes GPU density counts for the current viewport and keeps the window title focused on the active view: point count, grid size, viewport ranges, max bin count, and selected-region summary. The selected-region summary is computed on CPU from the active point records and includes selected row count, percentage, brush x/y ranges, selected data extents, category counts, and top category. The workbench does not present these values as GPU benchmark results.

Timeline demo note: `--demo timeline` uses deterministic synthetic event data by default, or local CSV rows when `--input`, `--time`, and `--lane` are provided. It renders GPU timeline-density counts as a simple full-window view where x is time, y is lane/source, and intensity is event count. Mouse wheel zooms time, left or middle drag pans time, and `R` resets to the full time range; each viewport change re-bins the visible time range while lane mapping remains stable. Right-drag or Shift + left-drag creates a data-anchored timeline brush over a time/lane region, and the title reports a CPU-side selected-event summary with event count, selected percentage, lane counts, event-type counts, top lane/type, timestamp extent, and value extent. Once finalized, timeline brushes also cache deterministic CPU-side evidence with the lowest selected row ids and sampled event records, and `E` exports that cached evidence. The injected spike, gap, and stale-lane patterns should be visible in synthetic mode. Timeline rendering is a visual proof only: no axes, labels, row table UI, arbitrary timestamp normalization, or Parquet import are included yet. The current GPU timeline path deliberately keeps the `u32` time-span guard from Milestone 4A.

Brush overlay note: the current rectangle overlay is intentionally simple: a faint amber fill with a brighter border, rendered after the density pass. During drag, the rectangle follows screen-space mouse movement. Once finalized, the selection is anchored to data-space x/y ranges, and the overlay is projected back into the current viewport after zoom, pan, resize, or reset. Fully offscreen selections are hidden; partially visible selections are clamped to the viewport edge. Preset changes clear the brush because the synthetic dataset changes.

Selection evidence note: finalized brushes also build a small CPU-side evidence object from synthetic records. Evidence includes selected counts, category counts, min/max x/y, brush range, dataset seed/row count, and a deterministic sample of the lowest selected row ids plus their synthetic records. This is logged once when the brush finalizes and is not a row table UI or GPU row-id path.

Evidence export note: pressing `E` writes the active demo's cached selection evidence to `target/rawscope-exports/`, then appends a matching entry to `target/rawscope-exports/manifest.jsonl`. Scatter exports use `scatter-selection-<unix-ms>-<counter>.json` and `.md`; timeline exports use `timeline-selection-<unix-ms>-<counter>.json` and `.md`. If no finalized brush evidence exists in the active demo, the app logs a warning and does not write files. These artifacts are deterministic synthetic CPU-side evidence, not the final report system. The JSON schemas are documented in `docs/schemas/scatter-selection-evidence-v1.md` and `docs/schemas/timeline-selection-evidence-v1.md`.

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

This repository is intentionally small right now. The workbench supports a first local CSV import path for scatter and timeline density views; avoid broad import systems, egui UI, Tauri packaging, or dataframe/query abstractions until the product milestones call for them.
