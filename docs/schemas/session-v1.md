# RawScope Session v1

The local Python bridge writes `analysis.rawscope.json` with this stable
top-level contract:

- `artifact_kind`: `rawscope.session`
- `schema_version`: `1`
- `dataset`: local path/format plus optional display name, row limit, and
  validated evidence-key column
- `view`: scatter `x`/`y` bindings or timeline `time`/`lane` bindings, plus an
  optional dataset profile

Supported dataset formats are `csv` and `parquet`. File-backed sessions may
reference an existing local file; dataframe-backed sessions materialize a flat
`data.parquet` bundle. The bridge does not upload data, stream dataframe
objects, or persist arbitrary workbench styling in this schema.

The native workbench validates the manifest before loading the dataset and
validates the optional evidence key against the loaded source columns. Session
metadata is projected into scatter evidence v5 as terse local provenance; the
full manifest and source path are not copied into the evidence artifact.
