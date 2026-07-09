# Timeline Selection Evidence Schema v2

This document describes the source-aware JSON artifact written by the RawScope workbench for timeline brush selections.

## Identity

- Artifact kind: `timeline-selection-evidence`
- Schema version: `2`
- Current writer: `rawscope-workbench`
- Formatting owner: `rawscope-render`

## Top-Level Fields

- `artifact_kind`: string discriminator. For this schema it is always `timeline-selection-evidence`.
- `schema_version`: numeric schema version. For this schema it is always `2`.
- `dataset_identity`: source-aware dataset identity object.
- `view`: current timeline view configuration object.
- `brush_range`: finalized timeline brush in data coordinates.
- `selected_event_count`: number of selected visual events.
- `selected_percentage`: selected events as a percentage of dataset rows.
- `selected_timestamp_range`: selected timestamp extent, or `null` when empty.
- `selected_value_range`: selected value extent, or `null` when empty.
- `lane_counts`: selected-event counts by lane index.
- `event_kind_counts`: selected-event counts by visual event kind.
- `top_lane`: lane with the highest selected count, or `null` for empty selections.
- `top_event_kind`: event kind with the highest selected count, or `null` for empty selections.
- `row_id_sample`: deterministic sample of selected stable row ids.
- `selected_event_sample`: deterministic sample of selected timeline visual records.
- `source_columns`: retained local source column names when available.
- `selected_source_row_sample`: deterministic sample of retained local source rows when available.

## Dataset Identity

`dataset_identity` contains:

- `visual_kind`: `timeline`.
- `source`: tagged source object.
  - Synthetic source: `{"kind":"synthetic","seed":...,"generator":"..."}`
  - Local CSV source: `{"kind":"local_csv","path":"...","limit":...}`
  - Local Parquet source: `{"kind":"local_parquet","path":"...","limit":...}`
- `row_count`: dataset row count.
- `field_bindings`: current timeline field bindings such as `time` and `lane`.
- `lane_labels`: timeline lane labels when available.

This data comes from `rawscope-data::DatasetIdentity` and is serialized through render-owned DTOs. The data crate does not own JSON serialization.

## View

`view` contains the current timeline visual context:

- `time_range`
- `full_time_range`
- `lane_count`
- `grid_width`
- `grid_height`

This is the current visible and full timeline context, distinct from the finalized `brush_range`.

## Event-Kind Counts

`event_kind_counts` contains:

- `background`
- `spike`
- `stale_lane`
- `high_value_band`
- `unclassified`

Synthetic datasets use the synthetic labels. Local CSV and Parquet datasets use `unclassified` unless a later task defines a richer local kind model.

## Sampled Visual Events

`selected_event_sample` contains deterministic sampled visual events with:

- `row_id`
- `timestamp`
- `lane`
- `value`
- `kind`

`kind` is one of `background`, `spike`, `stale_lane`, `high_value_band`, or `unclassified`.

## Sampled Source Rows

When retained local source rows are available:

- `source_columns` lists column names in source order.
- `selected_source_row_sample` contains deterministic retained rows with:
  - `row_id`
  - `values`

`values` preserves source column order. Empty cells remain empty strings.

When source rows are unavailable, such as current synthetic datasets, both collections are empty.

## Determinism

- `row_id_sample` and `selected_event_sample` use the lowest selected row ids up to the configured sample size.
- `selected_source_row_sample` is derived from the same sampled row ids, preserving the same deterministic order.

## Deliberately Not Included Yet

- Screenshot or rendered image data.
- GPU-preserved row ids.
- Full selected row tables.
- HTML reports or styling.
- Benchmark or performance claims.

## Manifest

Current workbench exports place this artifact inside a collision-safe report bundle directory such as `target/rawscope-exports/report-timeline-<unix-ms>-<counter>/evidence.json`.

The same bundle also includes:

- `evidence.md`
- `manifest.json`
- `visual-context.txt`

The bundle manifest records the evidence artifact kind/schema version, bundle paths, visual-context path, selected event count, selected percentage, dataset row count, UNIX timestamp milliseconds, and export counter. The placeholder visual-context file records the exact view configuration while native image capture remains deferred.
