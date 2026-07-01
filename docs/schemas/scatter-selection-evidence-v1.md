# Scatter Selection Evidence Schema v1

This document describes the JSON artifact written by the RawScope workbench for synthetic scatter brush selections.

## Identity

- Artifact kind: `scatter-selection-evidence`
- Schema version: `1`
- Current writer: `rawscope-workbench`
- Formatting owner: `rawscope-render`

## Top-Level Fields

- `artifact_kind`: string discriminator. For this schema it is always `scatter-selection-evidence`.
- `schema_version`: numeric schema version. For this schema it is always `1`.
- `dataset_metadata`: synthetic dataset metadata object.
- `point_preset_row_count`: row count for the active deterministic synthetic point preset.
- `brush_range`: data-space brush range object.
- `selected_row_count`: number of synthetic point records inside the brush range.
- `selected_percentage`: selected rows as a percentage of the dataset row count.
- `selected_extent`: min/max x and y extents among selected rows, or `null` ranges when no rows are selected.
- `category_counts`: selected-row counts by synthetic point category.
- `top_category`: category with the highest selected count, or `null` for empty selections.
- `row_id_sample`: deterministic sample of selected row ids.
- `selected_record_sample`: deterministic sample of selected synthetic point records.

## Dataset Metadata

- `seed`: deterministic synthetic generator seed.
- `row_count`: number of generated synthetic point rows in the dataset.

This metadata is synthetic-only for now. It is not external file provenance, a dataset fingerprint, or import metadata.

## Brush Range

`brush_range` stores the finalized brush in data coordinates:

- `x.min`: minimum selected x value.
- `x.max`: maximum selected x value.
- `y.min`: minimum selected y value.
- `y.max`: maximum selected y value.

The brush is data-anchored after finalization. Zooming, panning, and resetting the viewport do not change this range.

## Selected Summary

- `selected_row_count`: count of selected synthetic point records.
- `selected_percentage`: selected count divided by total dataset row count, expressed as a percentage.
- `selected_extent.x`: selected-record x min/max, or `null` when the selection is empty.
- `selected_extent.y`: selected-record y min/max, or `null` when the selection is empty.

## Category Counts

`category_counts` contains selected-row counts for the synthetic point categories:

- `cluster`
- `background`
- `outlier`

`top_category` is one of `Cluster`, `Background`, or `Outlier` when at least one row is selected.

## Row ID Sample

`row_id_sample` is a deterministic sample of selected stable row ids. The current strategy samples the lowest selected row ids up to the configured evidence sample size.

## Selected Record Sample

`selected_record_sample` contains deterministic synthetic point record samples with:

- `row_id`: stable synthetic row id.
- `x`: synthetic x coordinate.
- `y`: synthetic y coordinate.
- `category`: synthetic point category.

## Deliberately Not Included Yet

- Screenshot or rendered image data.
- GPU-preserved row ids.
- Full selected row table.
- External dataset source.
- File import metadata.
- Timeline-density evidence.
- Performance measurements or benchmark claims.

## Manifest

Each workbench export appends one line to `target/rawscope-exports/manifest.jsonl`. Each JSONL line describes one JSON/Markdown export pair and includes the artifact kind, schema version, JSON path, Markdown path, selected count, selected percentage, point preset row count, UNIX timestamp milliseconds, and export counter.
