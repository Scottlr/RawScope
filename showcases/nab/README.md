# NAB showcase

The Numenta Anomaly Benchmark contains timestamped real-world and synthetic
metric series, labelled anomaly windows, and detector result files.

RawScope will visualise metric shape, labelled windows, detector windows, and
row-level evidence. Spanfold will calculate ground-truth/detector overlap,
residual and missing ranges, lead/lag, detector consensus, and corpus-level
roll-ups.

Planned workflow:

```text
cargo run -p rawscope-showcase-nab -- info
cargo run -p rawscope-showcase-nab -- fetch
cargo run -p rawscope-showcase-nab -- transform
cargo run -p rawscope-showcase-nab -- analyse
cargo run -p rawscope-showcase-nab -- visualise
cargo run -p rawscope-showcase-nab -- run
```

Source and attribution are recorded in `dataset.toml`. The current crate is a
scaffold only: no downloads, transformations, analysis, or rendering have been
performed. Users must review the upstream repository and dataset licence terms
at the pinned revision before acquiring or redistributing any data.
