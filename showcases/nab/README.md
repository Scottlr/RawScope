# NAB showcase

The Numenta Anomaly Benchmark contains timestamped real-world and synthetic
metric series, labelled anomaly windows, and detector result files. This
minimal showcase uses `realTraffic/speed_7578.csv`, the smallest data series in
the pinned NAB tree: 25,928 bytes and 1,127 observations.

The workflow downloads that source from pinned NAB commit
`ea702d75cc2258d9d7dd35ca8e5e2539d71f3140`, verifies its SHA-256 digest,
adds a stable sample index while retaining timestamps as evidence, prepares a
RawScope session through the external `rawscope` API, and launches the native
workbench.

Planned workflow:

```text
cargo run -p rawscope-showcase-nab -- info
cargo run -p rawscope-showcase-nab -- fetch
cargo run -p rawscope-showcase-nab -- transform
cargo run -p rawscope-showcase-nab -- visualise
cargo run -p rawscope-showcase-nab -- run
```

For the complete one-command path on PowerShell, use the included script. It
builds the local native workbench, sets the launcher path for this process, and
runs fetch, transform, session preparation, and visualisation:

```powershell
.\showcases\nab\run.ps1
```

Alternatively, install or otherwise expose `rawscope-workbench` on `PATH`, then
run `cargo run --release -p rawscope-showcase-nab -- run` directly.

Downloaded and generated files remain beneath the ignored `.showcase-data/`
directory. Re-running `fetch` verifies the existing file rather than replacing
it. `analyse` remains reserved for the future labelled-window/SpanFold workflow;
this small example visualises source values and does not claim NAB scoring.

Source, licence, attribution, revision, checksum, and scope caveats are recorded
in `dataset.toml`.
