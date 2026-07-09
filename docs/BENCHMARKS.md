# Benchmarks

RawScope benchmarks exist to support measured engineering claims. They do not replace correctness tests, and they do not by themselves justify product-wide performance language.

## Claim Rule

Do not cite benchmark results without naming the exact command, dataset, hardware, and date.

## What Is Measured

- CPU density reference binning in `rawscope-render`
- Local CSV and Parquet ingestion for the current scatter and timeline bindings in `rawscope-data`
- Local evidence report-bundle writes in `rawscope-workbench`
- GPU density binning only as an explicit opt-in local benchmark

## What Is Not Measured

- End-to-end product latency
- Production GPU scalability claims
- Screenshot capture or report-image generation
- Cloud or remote-storage workflows
- Broad dataframe or SQL execution

## Deterministic Benchmark Inputs

- CPU density benchmarks use deterministic synthetic point and event generators with seed `42`
- Local ingestion benchmarks use generated CSV and Parquet fixtures with fixed column bindings
- Evidence export benchmarks use fixed v2 evidence fixtures with retained source rows

## Commands

Run the CPU reference density benchmarks:

```powershell
cargo bench -p rawscope-render --bench density_reference
```

Run the local ingestion benchmarks:

```powershell
cargo bench -p rawscope-data --bench local_ingest
```

Run the workbench evidence export benchmarks:

```powershell
cargo bench -p rawscope-workbench --bench evidence_export
```

Run the GPU density benchmarks only when you explicitly opt in and have a known compatible local WGPU adapter:

```powershell
$env:RAWSCOPE_ENABLE_GPU_BENCH='1'
cargo bench -p rawscope-render --bench gpu_density
```

Without `RAWSCOPE_ENABLE_GPU_BENCH=1`, the GPU benchmark target exits early with a clear skip message.

## Benchmark Sizes

- CPU density benchmarks: `200_000` and `1_000_000` rows/events on a `512x512` grid
- Local ingestion benchmarks: `20_000` and `100_000` rows for CSV and Parquet fixtures
- Evidence export benchmarks: `20_000`-row source-aware evidence fixtures with deterministic sample size `5`

These sizes are chosen for repeatability and local iteration speed, not as proof of final production scale.

## Hardware And Environment Notes

- GPU benchmark results are hardware-specific
- CPU, memory, storage, power mode, and thermal state can materially affect results
- Benchmark runs should prefer a quiet machine and a stable power profile
- Normal `cargo test --workspace` must remain independent of benchmark hardware

## Reporting Guidance

- Keep benchmark output out of source-controlled truth unless it is intentionally reviewed
- Treat benchmark findings as input for follow-up optimization tasks, not as automatic code-change justification
- Avoid quoting ad hoc workbench timings, compile timings, or one-off local runs as benchmark evidence
