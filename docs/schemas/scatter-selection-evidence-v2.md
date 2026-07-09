# Scatter Selection Evidence Schema v2

This document describes the source-aware JSON artifact written by the RawScope workbench for scatter brush selections.

## Identity

- Artifact kind: `scatter-selection-evidence`
- Schema version: `2`
- Current writer: `rawscope-workbench`
- Formatting owner: `rawscope-render`

## Top-Level Fields

- `artifact_kind`: string discriminator. For this schema it is always `scatter-selection-evidence`.
- `schema_version`: numeric schema version. For this schema it is always `2`.
- `dataset_identity`: source-aware dataset identity object.
- `view`: current scatter view configuration object.
- `brush_range`: finalized scatter brush in data coordinates.
- `selected_row_count`: number of selected visual rows.
- `selected_percentage`: selected rows as a percentage of dataset rows.
- `selected_extent`: selected x/y extents, or `null` when empty.
- `point_kind_counts`: selected-row counts by visual point kind.
- `top_point_kind`: point kind with the highest selected count, or `null` for empty selections.
- `row_id_sample`: deterministic sample of selected stable row ids.
- `selected_record_sample`: deterministic sample of selected scatter visual records.
- `source_columns`: retained local source column names when available.
- `selected_source_row_sample`: deterministic sample of retained local source rows when available.

## Dataset Identity

`dataset_identity` contains:

- `visual_kind`: `scatter`.
- `source`: tagged source object.
  - Synthetic source: `{"kind":"synthetic","seed":...,"generator":"..."}`
  - Local CSV source: `{"kind":"local_csv","path":"...","limit":...}`
  - Local Parquet source: `{"kind":"local_parquet","path":"...","limit":...}`
- `row_count`: dataset row count.
- `field_bindings`: current scatter field bindings such as `x` and `y`.
- `lane_labels`: empty for scatter v2.

This data comes from `rawscope-data::DatasetIdentity` and is serialized through render-owned DTOs. The data crate does not own JSON serialization.

## View

`view` contains the current scatter viewport and density-grid configuration:

- `x_range`
- `y_range`
- `grid_width`
- `grid_height`

This is the current visual context, distinct from the finalized `brush_range`.

## Point-Kind Counts

`point_kind_counts` contains:

- `cluster`
- `background`
- `outlier`
- `unclassified`

Synthetic datasets use the synthetic labels. Local CSV and Parquet datasets use `unclassified` unless a later task defines a richer local kind model.

## Sampled Visual Records

`selected_record_sample` contains deterministic sampled visual rows with:

- `row_id`
- `x`
- `y`
- `kind`

`kind` is one of `cluster`, `background`, `outlier`, or `unclassified`.

## Sampled Source Rows

When retained local source rows are available:

- `source_columns` lists column names in source order.
- `selected_source_row_sample` contains deterministic retained rows with:
  - `row_id`
  - `values`

`values` preserves source column order. Empty cells remain empty strings.

When source rows are unavailable, such as current synthetic datasets, both collections are empty.

## Determinism

- `row_id_sample` and `selected_record_sample` use the lowest selected row ids up to the configured sample size.
- `selected_source_row_sample` is derived from the same sampled row ids, preserving the same deterministic order.

## Deliberately Not Included Yet

- Screenshot or rendered image data.
- GPU-preserved row ids.
- Full selected row tables.
- HTML reports or styling.
- Benchmark or performance claims.

## Manifest

Current workbench exports place this artifact inside a collision-safe report bundle directory such as `target/rawscope-exports/report-scatter-<unix-ms>-<counter>/evidence.json`.

The same bundle also includes:

- `evidence.md`
- `manifest.json`
- `visual-context.txt`

The bundle manifest records the evidence artifact kind/schema version, bundle paths, visual-context path, selected row count, selected percentage, dataset row count, UNIX timestamp milliseconds, and export counter. The placeholder visual-context file records the exact view configuration while native image capture remains deferred.
