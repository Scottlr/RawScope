# RawScope Session v2

Session v2 describes stable source and field-role intent. It does not persist
GPU resources, visual styling, transient interaction state, or profile-specific
capabilities.

The top-level shape is:

```json
{
  "artifact_kind": "rawscope.session",
  "schema_version": 2,
  "source": {
    "path": "data.parquet",
    "format": "parquet"
  },
  "view": {
    "kind": "numeric_pair",
    "x": "latency_ms",
    "y": "payload_size",
    "category": "service",
    "profile": "optional-hint"
  }
}
```

Supported views are `numeric_pair` (`x`, `y`, optional `category`),
`time_value` (`time`, `value`, optional `category`), and `timeline_lane`
(`time`, `lane`). Bindings are opaque source-column names. The workbench
resolves them once against the loaded typed schema before creating visual
resources; a profile is only a formatting/defaults hint. The current native
contract accepts the existing validated profile identifiers, but never uses
their identity to decide whether a mapping or mode is available.

The Python bridge keeps exact v1 output for `ScatterView` and `TimelineView`
unless a caller requests `schema_version=2`. A scatter category or
`TimeValueView` selects v2 automatically. Existing v1 manifests remain
readable and are adapted to the same version-neutral resolved binding intent.
