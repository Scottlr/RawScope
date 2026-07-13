# RawScope dataset showcases

This workspace contains scaffolded, reproducible integrations for real-world
temporal datasets. Each showcase will eventually acquire upstream data,
transform it into local RawScope sessions, use Spanfold for temporal comparison,
and launch the native workbench.

The Lichess showcase is an executable external-API integration over a prepared
local CSV. NAB also has a complete minimal path that downloads one pinned
series, verifies and transforms it, then launches RawScope. NASA and SMD remain
structure only.

| Showcase | Dataset | Current state |
| --- | --- | --- |
| `rawscope-showcase-lichess` | Prepared Lichess games CSV | Prepares and launches a profiled session through `rawscope-adapters` |
| `rawscope-showcase-nab` | Numenta Anomaly Benchmark (NAB) | Fetches and visualises the smallest pinned NAB series |
| `rawscope-showcase-nasa` | NASA SMAP/MSL telemetry | Metadata and CLI scaffold |
| `rawscope-showcase-smd` | Server Machine Dataset (SMD) | Metadata and CLI scaffold |

The Lichess binary exposes `prepare` and `run`; see its README for the required
schema. Each temporal showcase exposes `info`, `fetch`, `transform`, `analyse`,
`visualise`, and `run`. NAB implements the acquisition-to-visualisation path;
only `info` is enabled for the NASA and SMD scaffolds.

Each `dataset.toml` is reviewable acquisition metadata rather than runtime
configuration. The implemented NAB fetcher keeps its pinned source and checksum
beside the code that enforces them.

Runtime data will stay beneath `.showcase-data/`, which is ignored by Git. No
upstream dataset may be committed to this repository.
