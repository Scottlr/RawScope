# RawScope Dependency Policy

RawScope keeps dependency policy in `deny.toml` and validates it with the
repository toolchain. The current baseline is intentionally private packages,
crates.io-only registry input, and warnings for ecosystem-driven duplicate
versions rather than unsafe forced unification.

## Review protocol

- Owner: RawScope maintainers.
- Review cadence: review advisory ignores and duplicate-version warnings before
  every release-artifact assembly and at least quarterly.
- Current review date: 2026-07-12.
- Next review date: 2026-10-12.
- Required command: `cargo deny check` from a clean checkout using the pinned
  `rust-toolchain.toml`.

An advisory exception is acceptable only when the affected crate is transitive,
the vulnerable path is not reachable through RawScope's supported runtime input,
and no compatible direct-root upgrade exists. Exceptions must be removed when
those conditions change.

## Current advisory exceptions

| Advisory | Reachability decision | Review action |
|---|---|---|
| `RUSTSEC-2024-0436` (`paste`) | Transitive build-time GUI dependency; no supported direct replacement in the locked stack. | Recheck on GUI-stack upgrades. |
| `RUSTSEC-2026-0194`, `RUSTSEC-2026-0195` (`quick-xml`) | Transitive dependency; RawScope does not parse attacker-controlled XML. | Recheck before any XML/parser feature. |
| `RUSTSEC-2026-0192` (`ttf-parser`) | Transitive platform-font dependency; no compatible upgrade is available in the locked renderer stack. | Recheck on font/rendering upgrades. |

These entries are narrow `deny.toml` ignores, not a blanket advisory waiver.

## Duplicate versions

`deny.toml` sets `multiple-versions = "warn"`. The current duplicate families
come from platform and GUI transitive branches (for example `bitflags`,
`calloop`, `rustix`, `thiserror`, and `windows-sys`). They are warnings because
forcing one version across those branches would require an unrelated upstream
or platform migration. Before changing that policy, inspect `cargo tree -d`
and identify the direct root that can be upgraded safely.
