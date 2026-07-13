# RawScope dataset showcases

This workspace contains scaffolded, reproducible integrations for real-world
temporal datasets. Each showcase will eventually acquire upstream data,
transform it into local RawScope sessions, use Spanfold for temporal comparison,
and launch the native workbench.

The current crates are structure only. They do not download data, parse dataset
files, run analysis, or render results.

| Showcase | Dataset | Current state |
| --- | --- | --- |
| `rawscope-showcase-nab` | Numenta Anomaly Benchmark (NAB) | Metadata and CLI scaffold |
| `rawscope-showcase-nasa` | NASA SMAP/MSL telemetry | Metadata and CLI scaffold |
| `rawscope-showcase-smd` | Server Machine Dataset (SMD) | Metadata and CLI scaffold |

Every binary exposes `info`, `fetch`, `transform`, `analyse`, `visualise`, and
`run`. Only `info` is enabled. The other commands fail explicitly until dataset
acquisition is deliberately implemented.

Each `dataset.toml` is reviewable acquisition metadata, not an active fetch
configuration. The scaffold deliberately has no TOML parser or network client.

Runtime data will stay beneath `.showcase-data/`, which is ignored by Git. No
upstream dataset may be committed to this repository.
