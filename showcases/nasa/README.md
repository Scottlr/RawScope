# NASA SMAP/MSL showcase

This showcase targets telemetry channels and labelled anomaly sequences from
NASA's Soil Moisture Active Passive (SMAP) satellite and Mars Science Laboratory
(MSL) rover dataset published with Telemanom.

RawScope will visualise channel telemetry, anomaly windows, channel families,
and supporting row evidence. Spanfold will calculate detector comparisons,
channel-level anomaly windows, spacecraft and channel-family roll-ups, and point
versus contextual anomaly summaries.

Planned workflow:

```text
cargo run -p rawscope-showcase-nasa -- info
cargo run -p rawscope-showcase-nasa -- fetch
cargo run -p rawscope-showcase-nasa -- transform
cargo run -p rawscope-showcase-nasa -- analyse
cargo run -p rawscope-showcase-nasa -- visualise
cargo run -p rawscope-showcase-nasa -- run
```

Cross-channel time alignment must be verified before this showcase claims
synchronized incident aggregation. Source and attribution are recorded in
`dataset.toml`. The current crate is a scaffold only: no downloads,
transformations, analysis, or rendering have been performed. Users must review
the upstream licence, dataset terms, and NASA attribution requirements.
