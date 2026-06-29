# Goals

## Goal 1: Visualise Full-Dataset Shape At Large Scale

- Problem: users cannot directly inspect huge datasets and often have to sample, pre-aggregate, or guess the right query.
- Desired user outcome: users can see the real shape of the dataset before deciding what to query.
- Technical direction: use columnar chunks, visual summaries, GPU-side binning/density/tile rendering, and pixels as aggregates over many rows.
- Examples: spikes, gaps, null bands, stale sources, outliers, distribution drift, category explosions, dense clusters, missing partitions, duplicate bursts, source/provider lag.

## Goal 2: Keep Exploration Smooth As Data Grows

- Problem: traditional charting slows down when interactions move millions of rows through UI state or rebuild large chart structures.
- Desired user outcome: zooming, panning, brushing, filtering, and linked views remain interactive on large datasets.
- Technical direction: use Rust for data/control-plane work, WGPU for future GPU compute/rendering, chunked data layout, tile-based level of detail, GPU-side masks where useful, copy-minimising data movement, and limited GPU readback during interaction.
- Examples: smooth zoom over event density, brushing a dense cluster, comparing selected-vs-baseline populations, and panning a long timeline without rebuilding one object per row.

## Goal 3: Bridge Visual Patterns To Exact Row Evidence

- Problem: a heatmap or density view is not enough if users cannot identify which rows caused a spike, gap, drift, or anomaly.
- Desired user outcome: users can move from visual pattern to selected region to contributing records and exportable proof.
- Technical direction: maintain row-id mappings, preserve selection masks, support row drilldown, store view configuration, and produce evidence exports.
- Examples: brush a spike, view summary statistics, inspect top contributors, inspect exact or sampled rows, and export the finding with context.

## Goal 4: Support Local-First And Private Dataset Exploration

- Problem: many datasets cannot be casually uploaded to cloud BI tools or duplicated into remote warehouses for early exploration.
- Desired user outcome: users can inspect local datasets without needing a server or cloud service.
- Technical direction: build a native desktop app first, keep local file opening as the long-term default, avoid server requirements, preserve privacy by default, and defer web/WASM until the native path is proven.
- Examples: private operational logs, experimental research outputs, local Parquet extracts, customer-sensitive telemetry, and early pipeline debugging files.

## Goal 5: Produce Reproducible Evidence Reports

- Problem: screenshots alone lose analytical context and make findings hard to share, revisit, or validate.
- Desired user outcome: users can export what they found, why it matters, and which rows support it.
- Technical direction: design reports around dataset fingerprints, view configuration, selected regions, filters, summaries, top contributors, sample or exact rows, rendered visuals, and Markdown/HTML/JSON evidence bundles.
- Examples: a missing-partition report, a provider-lag investigation, a model-drift note, a null-rate regression summary, or a suspicious-cluster evidence bundle.
