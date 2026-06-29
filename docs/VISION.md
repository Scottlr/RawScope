# Vision

## Mission

RawScope helps analysts, researchers, data scientists, and big data engineers visually inspect the shape of huge raw datasets before they know exactly what SQL query, notebook analysis, dashboard, or model they need.

Tagline: See the shape before writing the query.

RawScope is for the messy early stage of analysis:

- I have a huge dataset.
- I do not fully know what is wrong yet.
- Sampling may hide the problem.
- Pre-aggregation forces the question too early.
- BI tools make me model the data before I can visually inspect it.
- Notebooks are powerful, but slow and iterative when exploring very large raw files.
- I need to spot spikes, gaps, drift, null bands, outliers, stale sources, and suspicious clusters quickly.
- When I spot something visually, I need to drill back to the exact contributing rows.

RawScope should turn large raw datasets into GPU-rendered visual summaries where every visible pattern can be traced back to row-level evidence. Pixels should be explainable back to rows.

## Target Users

### Analysts

Analysts want to answer what changed, when it changed, which segment caused it, whether the data can be trusted, and what they should investigate next.

They care about fast visual profiling, data quality, distribution shifts, timeline anomalies, top contributors, simple reports, and reproducible findings.

### Data Scientists

Data scientists want to inspect feature drift, label imbalance, prediction errors, model version differences, training-vs-production differences, outliers, embedding shape, and cohort differences.

They care about scatter density, distribution drift, missingness, cohort brushing, selected-vs-baseline comparison, and exact row drilldown.

### Researchers

Researchers want to inspect large experimental datasets, embeddings, point clouds, clusters, matrix or heatmap structures, annotated subsets, and repeatable views.

They care about large visual density, cluster selection, metadata overlays, reproducibility, and exportable visual evidence.

### Big Data Engineers

Big data engineers want to debug pipeline regressions, bad backfills, missing partitions, late-arriving data, duplicate bursts, schema drift, null spikes, key churn, source freshness, provider gaps, and partition skew.

They care about dataset diffs, schema diffs, missing or new keys, timeline density, null-rate maps, partition heatmaps, and row-level proof.

## Why Now

Modern teams produce more raw data than they can inspect directly. Existing systems are strong once the question is known, but the first step is often unclear: the user needs to see the data's shape before deciding what query, model, dashboard, or notebook path is worth pursuing.

GPU-side visual aggregation, local-first desktop workflows, and columnar data layouts make it practical to build a tool focused on this pre-query exploration stage.

## User Pain

- Huge datasets are hard to inspect directly without losing important detail.
- Existing tools often force sampling, pre-aggregation, waiting, or cloud upload before exploration can begin.
- Visual summaries often make it hard to recover the exact row evidence behind an interesting pattern.
- Screenshots alone lose analytical context and are difficult to reproduce.

## North-Star Workflow

Open a large dataset -> see full-dataset visual shape -> spot a spike, gap, drift, null band, outlier, stale source, or suspicious cluster -> brush an interesting region -> inspect exact or sampled contributing rows -> export reproducible evidence.

## Product Principles

- Full-dataset shape comes before polished dashboards.
- Visual patterns must connect back to row-level evidence.
- Local-first exploration should be the default.
- Synthetic data comes before external file import.
- Copy-minimising language is preferred over zero-copy claims unless the claim is technically exact.
- Performance claims require benchmarks.
- RawScope complements BI tools, notebooks, and query engines. It does not replace them.

## What Success Looks Like

- Users can spot major data shape problems before writing a query.
- Brushing a visual region can explain which rows contributed to it.
- Linked views can show how a selection behaves across time, category, distribution, and missingness views.
- Evidence exports preserve dataset fingerprint, view configuration, selected region, summaries, contributors, rows, and visual context.
- Future developers can extend the system without collapsing data, GPU, render, and UI concerns together.
