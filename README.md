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

This repository is intentionally small right now. Do not add file import, WGPU rendering, egui UI, Tauri packaging, or broad abstractions until the synthetic data and CPU reference path is stable.
