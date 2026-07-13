# SMD showcase

The Server Machine Dataset contains per-machine multivariate training and test
sequences, point anomaly labels, and interpretation labels identifying the
dimensions contributing to anomalies.

RawScope will visualise machine and dimension-level temporal shape, anomaly
windows, affected-dimension concurrency, and source evidence. Spanfold will
calculate dimension-level windows, machine incidents, contributing-dimension
coverage, and machine/group roll-ups.

Planned workflow:

```text
cargo run -p rawscope-showcase-smd -- info
cargo run -p rawscope-showcase-smd -- fetch
cargo run -p rawscope-showcase-smd -- transform
cargo run -p rawscope-showcase-smd -- analyse
cargo run -p rawscope-showcase-smd -- visualise
cargo run -p rawscope-showcase-smd -- run
```

Fleet-wide wall-clock alignment must be verified before this showcase claims
synchronized machine aggregation. Source and attribution are recorded in
`dataset.toml`. The current crate is a scaffold only: no downloads,
transformations, analysis, or rendering have been performed. Users must review
the upstream repository and dataset licence terms before acquisition or
redistribution.
