# Scatter Selection Evidence v4

Scatter evidence v4 is the default RawScope scatter report artifact. It is an
additive schema: v1, v2, and v3 APIs and artifact shapes remain unchanged.

## Identity

- `artifact_kind`: `rawscope.scatter-selection-evidence.v4`
- `schema_version`: `4`
- `dataset_identity`: source, row count, visual kind, and field bindings
- `active_dataset_profile`: omitted when no profile is active

## Visual Query

`visual_query` records the settled semantic query that produced the visual:

- exact x/y ranges and density grid dimensions;
- projection variant and the mean/difference sign convention;
- ordered active filter definitions, excluding filter revisions;
- absolute or filtered-difference density mode;
- transform, palette, normalization, and presentation;
- difference formula, full-dataset baseline, cohort totals, and symmetric
  maximum absolute share delta when difference mode is active;
- point-reveal mode, eligible/rendered counts, and sampling disclosure;
- relief configuration only when Relief is the active presentation.

The difference formula identifier is
`active_share_minus_full_baseline_share`. Difference artifacts never claim
Relief shading or rendered point-reveal points.

## Cohort And Evidence

`cohort` contains full, included, and excluded row counts. Included plus
excluded must equal full. Selection counts, bounded row-id/record/source-row
samples, comparison ratios, and aggregate context retain the v3 meanings.

`pinned_inspection`, when present, records the exact bin indices, data ranges,
row count, bounded row-id sample, and sample limit. Screen coordinates, hover
state, animation progress, GPU adapter details, and transient freshness state
are excluded.

## Compatibility

V4 does not add fields to earlier schemas. Explicit v1/v2/v3 formatters and
report writers continue to emit their prior bytes and labels. Serialization
DTOs live in `rawscope-render`; `rawscope-data` domain contracts do not derive
or depend on serde for this schema.
