# Timeline Selection Evidence Schema v1

This document describes the JSON artifact written by the RawScope workbench for synthetic timeline brush selections.

## Identity

- Artifact kind: `timeline-selection-evidence`
- Schema version: `1`
- Current writer: `rawscope-workbench`
- Formatting owner: `rawscope-render`

## Top-Level Fields

- `artifact_kind`: string discriminator. For this schema it is always `timeline-selection-evidence`.
- `schema_version`: numeric schema version. For this schema it is always `1`.
- `dataset_metadata`: synthetic dataset metadata object.
- `event_count`: number of generated synthetic event rows in the active demo dataset.
- `selected_time_range`: finalized data-space brush time range.
- `selected_lane_range`: finalized half-open lane range.
- `selected_event_count`: number of synthetic events inside the selected time/lane range.
- `selected_percentage`: selected events as a percentage of the synthetic event count.
- `selected_timestamp_range`: min/max timestamp among selected events, or `null` when no events are selected.
- `selected_value_range`: min/max value among selected events, or `null` when no events are selected.
- `lane_counts`: selected-event counts by lane index.
- `event_type_counts`: selected-event counts by synthetic event type.
- `top_lane`: lane with the highest selected count, or `null` for empty selections.
- `top_event_type`: event type with the highest selected count, or `null` for empty selections.
- `row_id_sample`: deterministic sample of selected stable row ids.
- `selected_event_sample`: deterministic sample of selected synthetic event records.

## Dataset Metadata

- `seed`: deterministic synthetic generator seed.
- `row_count`: number of generated synthetic event rows in the dataset.

This metadata is synthetic-only for now. It is not external file provenance, a dataset fingerprint, or import metadata.

## Selected Time Range

`selected_time_range` stores the finalized brush in data coordinates:

- `min`: minimum selected timestamp.
- `max`: maximum selected timestamp.

The current synthetic timeline demo deliberately keeps the existing `u32` time-span guard from the GPU timeline-density path. Arbitrary timestamp normalization is deferred.

## Selected Lane Range

`selected_lane_range` stores a half-open lane range:

- `start`: first selected lane.
- `end_exclusive`: first lane after the selection.

The brush is data-anchored after finalization. Zooming, panning, resetting, and resizing the viewport do not change this range.

## Selected Summary

- `selected_event_count`: count of selected synthetic event records.
- `selected_percentage`: selected count divided by total event count, expressed as a percentage.
- `selected_timestamp_range`: selected-event timestamp min/max, or `null` when the selection is empty.
- `selected_value_range`: selected-event value min/max, or `null` when the selection is empty.

## Lane Counts

`lane_counts` is an array indexed by lane number. Each value is the number of selected synthetic events in that lane.

`top_lane` is the lane index with the highest selected count when at least one event is selected.

## Event-Type Counts

`event_type_counts` contains selected-event counts for the synthetic event types:

- `background`
- `spike`
- `stale_lane`
- `high_value_band`

`top_event_type` is one of `Background`, `Spike`, `StaleLane`, or `HighValueBand` when at least one event is selected.

## Row ID Sample

`row_id_sample` is a deterministic sample of selected stable row ids. The current strategy samples the lowest selected row ids up to the configured evidence sample size.

## Selected Event Sample

`selected_event_sample` contains deterministic synthetic event samples with:

- `row_id`: stable synthetic row id.
- `timestamp`: synthetic timestamp.
- `lane`: synthetic lane/source index.
- `value`: synthetic event value.
- `event_type`: synthetic event type.

## Deliberately Not Included Yet

- Screenshot or rendered image data.
- GPU-preserved row ids.
- Full selected row table.
- External dataset source.
- File import metadata.
- Scatter-density evidence fields.
- Performance measurements or benchmark claims.

## Manifest

Each workbench export appends one line to `target/rawscope-exports/manifest.jsonl`. Each JSONL line describes one JSON/Markdown export pair and includes the artifact kind, schema version, JSON path, Markdown path, selected event count, selected percentage, event count, UNIX timestamp milliseconds, and export counter.
