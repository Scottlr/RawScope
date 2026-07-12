# Platform and backend support

RawScope currently validates the following support tiers in GitHub Actions:

| Tier | Platforms | Evidence | Meaning |
| --- | --- | --- | --- |
| Required build | Ubuntu 24.04, Windows Server 2025, macOS 14 | `platform-build` | The workspace compiles for the runner target. This is not a claim of runtime parity. |
| Required quality | Ubuntu 24.04 | `rust-quality`, `msrv` | Formatting, checks, tests, documentation, and the declared MSRV contract run on the Linux runner. |
| Required Python | Ubuntu 24.04, Python 3.10–3.13 | `python-contracts` | The SDK tests and wheel build run for these interpreter versions. |
| Opt-in GPU | Runner/backend reported in the workflow summary | `GPU correctness (opt-in)` | Ignored adapter-backed tests are exercised manually or on schedule; they are not a generic hosted-runner merge gate. |

The platform-build tier is deliberately a build-only statement. Runtime support,
adapter quality, and production performance require an explicit validated runner
and are not inferred from compilation. Branch-protection administration is a
repository-owner decision; the workflow job names are stable so they can be
required after a successful observed run.

The current declared Rust MSRV is 1.88.0. It is validated against the locked
dependency graph; the contributor toolchain remains separately pinned in
`rust-toolchain.toml`.
