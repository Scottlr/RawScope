# NAB showcase

The Numenta Anomaly Benchmark contains timestamped real-world and synthetic
metric series, labelled anomaly windows, and detector result files. This
minimal showcase uses `realTraffic/speed_7578.csv`, the smallest data series in
the pinned NAB tree: 25,928 bytes and 1,127 observations.

The workflow downloads four artifacts from pinned NAB commit
`ea702d75cc2258d9d7dd35ca8e5e2539d71f3140`: the source series, combined label
windows, Numenta detector output, and published detector thresholds. Every file
is SHA-256 verified. The showcase converts the official standard-threshold hits
into contiguous windows, compares them with the four ground-truth windows using
SpanFold 0.1.1, then prepares three views through the external `rawscope` API:

- raw traffic speed by sample index;
- SpanFold interval start offset by duration;
- all 1,127 source samples grouped into missed anomaly, detected anomaly,
  false-positive, and outside-anomaly lanes derived from SpanFold interval rows.

The state timeline does not manufacture detector events. It projects each real
source sample through the mutually exclusive SpanFold overlap, residual, and
missing ranges, retaining the original timestamp/value and contributing
SpanFold row identifiers as evidence. The separate interval dataset retains
exact end, duration, coverage, finality, and source record identifiers. The
analysis also writes `spanfold-aggregations.csv` with interval counts, total and
mean durations by family, plus overall coverage totals.

Planned workflow:

```text
cargo run -p rawscope-showcase-nab -- info
cargo run -p rawscope-showcase-nab -- fetch
cargo run -p rawscope-showcase-nab -- transform
cargo run -p rawscope-showcase-nab -- analyse
cargo run -p rawscope-showcase-nab -- visualise
cargo run -p rawscope-showcase-nab -- run
```

For the complete one-command path on PowerShell, use the included script. It
builds the local native workbench, sets the launcher path for this process, and
runs fetch, transform, SpanFold analysis, session preparation, and all three
visualisations:

```powershell
.\showcases\nab\run.ps1
```

Alternatively, install or otherwise expose `rawscope-workbench` on `PATH`, then
run `cargo run --release -p rawscope-showcase-nab -- run` directly.

Downloaded and generated files remain beneath the ignored `.showcase-data/`
directory. Re-running `fetch` verifies existing files rather than replacing
them. The comparison uses NAB's published standard Numenta threshold; it does
not reimplement or claim the NAB benchmark score.

Source, licence, attribution, revision, checksum, and scope caveats are recorded
in `dataset.toml`.
