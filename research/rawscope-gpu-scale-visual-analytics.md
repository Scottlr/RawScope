# RawScope GPU-Scale Visual Analytics Research Takeaways

Date: 2026-07-09
Status: research synthesis
Source: Codex attachment `00e957d6-f968-4ba3-9cf2-46487d7bb8b5/pasted-text.txt`

The source memo included non-portable citation markers from a prior research
session. Treat this document as a durable product and architecture synthesis,
not as a verified citation bundle. Re-check external dataset sizes, API status,
and library claims before using them in public docs, benchmarks, or marketing.

## Core Takeaway

The research reinforces RawScope's current thesis: RawScope is strongest as a
local-first, evidence-centered visual analytics engine for large raw datasets.
The next value is not more chart types. It is making dense views truthful,
comparable, multiscale, and explainable back to rows.

In short: strengthen the path from pattern -> explanation -> evidence before
expanding the surface area.

## What RawScope Already Has Right

- The product boundary is coherent: see full-dataset shape before writing a
  query, then connect visible patterns back to row evidence.
- The non-goals are important product assets. RawScope should not become a
  generic charting library, BI dashboard builder, SQL engine, notebook
  replacement, cloud analytics platform, or plugin framework.
- The current architecture direction matches the problem: raw or synthetic data
  becomes chunked columnar data, visual query inputs, aggregate bins or masks,
  rendered pixels, and evidence on demand.
- Density-first rendering is the right default for large scatter and event
  views. Rendering one marker per row is not the scalable product model.
- The repo's correctness-first posture is healthy. Synthetic data, CPU
  references, deterministic tests, evidence schemas, and benchmark discipline
  matter more than early performance claims.

## Highest-Leverage Product Direction

### 1. Improve analytical legibility before adding new surfaces

The current renderer can reveal structure, but the analyst needs stronger
context to trust what they are seeing.

Useful next improvements:

- visible legends for density, count, and transform semantics
- explicit density transform controls such as linear, log/log1p, and rank or
  equalized modes
- readable axis ticks, lane labels, and units
- marginal histograms for scatter and timeline views
- a persistent overview strip or mini-map for timeline navigation
- selection overlays that do not compete with the density color scale
- side-panel summaries that explain the current view even before a brush exists

The important principle is that visual polish is not decorative here. For large
data, misleading color maps, hidden transforms, overplotting, undersampling, and
dynamic-range issues can make the visualization lie by accident.

### 2. Make linked comparison a headline capability

RawScope's differentiator is not a single dense canvas. It is coordinated
evidence across views.

The research points toward linked panes such as:

- full overview plus zoomed detail
- selected cohort versus baseline cohort
- timeline plus missingness
- scatter plus marginal histograms
- before versus after dataset diff
- data-quality pane alongside a primary analytical view

This should stay narrower than a dashboard system. The goal is not arbitrary
crossfiltering; the goal is explaining a selected visual pattern across a small
set of evidence-rich surfaces.

### 3. Treat multiscale aggregates as core architecture

Viewport rebinning is useful for immediate interaction, but it should not be the
only abstraction. The research argues for a tile or level-of-detail model:

- cheap whole-dataset overview cache on load
- progressively refined data-space or tile-space aggregates while zooming
- fast viewport-level GPU aggregation for interactive feedback
- persistent aggregate metadata that can support reproducible evidence bundles

This is where RawScope starts to feel less like a redraw loop and more like a
visual query engine.

### 4. Strengthen evidence as a first-class object

Every finalized brush should produce more than a visual rectangle. It should
produce an evidence object that can be inspected, exported, and compared.

Useful evidence fields:

- selected count and percent of baseline
- aggregate counts by relevant dimension
- top contributors or dominant categories
- representative source-row samples
- exact row IDs when feasible
- view configuration, transform, zoom, filter, color scale, and bin or tile
  level
- dataset identity and fingerprint

The research recommends a layered evidence model:

- cheap aggregate bins for default interaction
- per-bin or per-tile sample reservoirs for fast explanation
- exact row recovery when the selection is narrow enough or the backing dataset
  can be selectively rescanned

That keeps the product promise credible without pretending that full GPU row-id
preservation is cheap everywhere.

### 5. Lean into data-quality and diff workflows

Missingness, null bands, source freshness, provider gaps, schema drift, and
dataset diffs are unusually strong fits for RawScope. They answer the questions
large-data users ask early:

- what changed?
- where did it change?
- since when?
- which rows explain it?

These workflows are more distinctive than adding generic chart types.

## Architecture Implications

### Native Rust and WGPU

Rust and `wgpu` are a good fit, but the advantage should be framed carefully.
The win is not "web visualization cannot scale." The win is tighter local file
I/O control, memory layout control, GPU resource residency, privacy-preserving
local workflows, and a cleaner evidence path for private raw data.

### Columnar and Parquet direction

Arrow-style columnar memory and Parquet metadata are good long-term fits for
RawScope's visual-query model. The practical opportunity is late materialization:
use metadata, row groups, page indexes, and pushed-down predicates to recover
evidence for selected regions without decoding everything eagerly.

Guardrail: do not let this drift into a dataframe engine or SQL planner unless a
future roadmap task explicitly calls for that boundary change.

### Four useful work levels

1. Full-dataset overview cache for immediate first paint.
2. Viewport-level fast aggregates for pan, zoom, and brush feedback.
3. Selection-aware evidence recovery for summaries, samples, and exact rows.
4. Benchmark discipline for any performance or scale claim.

## Visual Truthfulness Rules

- Density and counts should use perceptually reasonable sequential color maps.
- Delta and diff surfaces should use diverging color maps centered on zero or a
  clear baseline.
- Selections should be overlays, not a second density palette.
- The active transform must be visible wherever density is shown.
- Raw point markers should be limited to small selections or drilldown, not used
  as the default large-scale encoding.
- Screenshots or exported images should preserve enough visual context to be
  interpreted later.

## Candidate Demo Datasets

These are candidate directions from the research memo, not verified commitments.
Validate availability, license, current size, and ingestion complexity before
planning work around them.

### Lichess

Best candidate for a flagship large gaming dataset. It can support timeline
density, scatter density, missingness, partition checks, and before/after
comparison across months, ratings, openings, or outcomes.

Why it fits:

- large public corpus
- natural time, rating, opening, outcome, and game-length dimensions
- good stress test for Parquet partitions and multiscale summaries

### OpenDota

Good candidate for event timelines, patch-era comparisons, hero/item/lane diffs,
and recognizably gaming-native analytics.

Why it fits:

- structured competitive event data
- patch and metagame comparisons are intuitive
- richer temporal/event interpretation than simple catalog metadata

### speedrun.com

Good candidate for investigative workflows around leaderboards, verification
latency, categories, platforms, timing systems, and ruleset complexity.

Why it fits:

- strong metadata and status fields
- naturally supports timeline and diff views
- smaller than Lichess, but easier to explain as an evidence workflow

### Steam

Useful as a catalog and metadata-diff source, but probably not the flagship
stress dataset unless paired with richer behavioral data.

Why it fits:

- catalog scale and schema quirks
- release cadence and app-type/category exploration
- good missingness and metadata-diff test case

### NYC TLC Trip Records

Useful non-gaming fallback for demos and benchmarks because the domain is easy
to understand and supports scatter, timeline, geospatial, fare, distance, and
missingness views.

## Recommended Product Sequence

1. Make current timeline, scatter, and missingness surfaces more legible:
   legends, transforms, axes, labels, marginals, overview strips, and persistent
   side-panel explanations.
2. Extend linked selection into linked comparison UI: overview/detail,
   selected-vs-baseline, and data-quality context without becoming a dashboard.
3. Design multiscale cached aggregates: overview cache, tile or level-of-detail
   summaries, and evidence-aware aggregate metadata.
4. Harden evidence recovery for local data: exact rows where feasible, sampled
   rows where necessary, and explicit visual context in exports.
5. Choose one flagship dataset path, likely Lichess first, after verifying
   availability, license, data shape, and ingestion cost.

## Guardrails For Future Work

- Do not broaden RawScope into generic charting.
- Do not add dashboard or BI workflows under the name of linked views.
- Do not add DataFusion, Tauri, web, cloud, plugins, or broad file connectors
  as part of this research direction unless a specific roadmap task requires
  them.
- Do not claim benchmarked scale or zero-copy behavior without exact evidence.
- Do not treat GPU row-id preservation as solved until there is an explicit,
  tested design.
- Do not let demo-dataset ingestion pull the repo into a general ETL platform.

## Bottom Line

The strongest takeaway is simple: keep RawScope narrow and serious.

Build better truthfulness, better comparison, better evidence, and better
multiscale navigation before adding more chart types. That direction matches
the existing docs, preserves the product boundary, and gives RawScope a clearer
identity than generic visual analytics tools.
