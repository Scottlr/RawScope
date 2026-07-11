# Scatter Selection Evidence v5

Scatter evidence v5 is the default new RawScope scatter report artifact. It
copies the v4 visual-query, cohort, selection, comparison, and aggregate
semantics without changing the v1-v4 APIs or serialized artifacts.

## Identity And Session Context

- `artifact_kind`: `rawscope.scatter-selection-evidence.v5`
- `schema_version`: `5`
- `dataset_identity`: the existing dataset identity contract
- `session_context`: optional local bridge metadata, omitted for direct or
  synthetic startup

When present, `session_context` identifies the `rawscope.session` artifact and
schema, display name, `csv` or `parquet` format, and optional validated
evidence-key column. It does not include the manifest path, source file, hash,
or full data corpus.

## Pinned Inspection Context

`pinned_inspection`, when present, is the immutable analytical payload shown by
the pinned workbench rail:

- exact bin indices and data ranges;
- exact row count and active-cohort share;
- occupied-cell density percentile when available;
- fixed one-bin neighborhood row count and active-cohort share;
- bounded row-id sample and its limit;
- bounded natural-key values when a session evidence key is configured;
- optional active-minus-full-baseline difference counts, shares, direction,
  formula/baseline identifiers, and absolute-delta strength percentile.

The schema validates finite unit-interval shares and percentiles, the fixed
neighborhood radius, sample bounds, session-key requirements, and coherent
difference totals/deltas. Hover placement, alpha, animation progress, GPU
state, and transient pointer state are excluded.

## Compatibility

V5 is additive. Explicit v1/v2/v3/v4 formatters and report writers remain
available and retain their existing artifact kinds and shapes. Evidence DTOs
and serialization remain owned by `rawscope-render`; workbench session state
is projected into the v5 contract rather than serialized directly.
