# RawScope semantic fuzz targets

This private package is excluded from the workspace and is not publishable. Each
target caps input at 64 KiB and caps loader rows at 1,024 before invoking a
checked RawScope boundary. Temporary files are process-owned and removed after
each input; fuzzing uses one process per target by default.

Build or run manually with cargo-fuzz:

```powershell
cargo fuzz run session_manifest_v1 -- -max_len=65536 -runs=1000
cargo fuzz run csv_ingest_bounded -- -max_len=65536 -runs=1000
cargo fuzz run parquet_dispatch_bounded -- -max_len=65536 -runs=1000
```

Campaigns are manual/opt-in. Seeds are intentionally tiny and contain no user
data. A crash should be reduced to a seed and fixed in the owning boundary task.
