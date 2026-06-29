# RawScope Agent Governance

This file is mandatory context for every agent working in this repository. Product-specific boundaries also live in `docs/AGENTS.md`; read both before changing code.

## Mandatory Preflight

Before editing, the agent must:

- Inspect the relevant files first, including `Cargo.toml`, crate manifests, nearby `src/lib.rs` or `src/main.rs`, and any module/test files being changed.
- List which repo skills apply from `.codex/skills`.
- Identify likely architectural risks before changing code.
- Say what it will not do for this task.
- Check whether an existing owner, module, app, example, test harness, macro, or abstraction already exists before creating a new one.

Always consider these skills:

- `.codex/skills/agent-preflight-and-handoff/SKILL.md` for preflight and final handoff.
- `.codex/skills/rust-code-organization/SKILL.md` when adding or changing Rust modules/files.
- `.codex/skills/rust-testing-policy/SKILL.md` when adding or changing tests.
- `.codex/skills/rust-observability/SKILL.md` when adding logging, tracing, errors, IO, background work, parsing, decode/encode, retries, or state transitions.
- `.codex/skills/rust-idioms-and-abstractions/SKILL.md` when designing Rust types, traits, builders, ownership, or error surfaces.
- `.codex/skills/rust-constants-and-config/SKILL.md` when introducing strings, numbers, limits, route names, IDs, timeouts, byte sizes, or configuration.
- `.codex/skills/rust-macro-policy/SKILL.md` before introducing or expanding macros.

## Rust Structure Rules

- Prefer vertical/domain ownership over generic buckets.
- Split by responsibility, not arbitrary line count.
- Files over roughly 400 lines require justification or a split plan.
- `lib.rs`, `main.rs`, and `mod.rs` should be routing/facade files, not implementation dumps.
- Public facades should be thin and use `pub use` intentionally.
- Concepts that change together should usually live together.
- Tests should not bloat production files.
- Do not create god files, god modules, `utils.rs`, `helpers.rs`, `common.rs`, `misc.rs`, or giant `types.rs`, `models.rs`, `contracts.rs`, `dto.rs`, or `packet.rs` dumping grounds.

## Testing Policy

- Inline unit tests are only for tiny, local behavior.
- Larger behavior tests belong under `tests/`.
- Shared test fixtures/helpers belong under `tests/common/`.
- Prefer table-driven tests and deterministic fixtures.
- No meaningless smoke tests.
- No smoke apps or fake demo apps unless the repo already has a real example/demo app pattern.
- Test names should describe behavior, not implementation.
- Tests must prove behavior that would fail if the feature were fake.

## Logging And Observability

- Prefer `tracing` over ad-hoc logging.
- Logs must include useful structured context.
- Use spans for workflows.
- Errors should preserve source and context.
- Do not swallow errors.
- Do not log noisy success paths unless the signal is useful.
- Do not log secrets or large payloads.
- No `println!` debugging in production code.
- Avoid vague errors like `"failed"` or `"invalid data"`.

## Constants And Config

- Replace repeated strings/numbers with named constants or typed config.
- Constants should live near the domain that owns them.
- Do not create giant `constants.rs` files.
- Explain units in names, such as `timeout_ms`, `max_rows`, or `chunk_size_bytes`.
- Prefer enums/newtypes for constrained values.
- Document protocol numbers, IDs, and limits with source/context comments.

## Idiomatic Rust

- Prefer ownership-aware APIs and avoid unnecessary allocation.
- Prefer iterators where clearer and loops where clearer.
- Use `Result` and domain errors for recoverable failures.
- Use traits for real polymorphic seams, not speculative abstraction.
- Use generics when they reduce duplication or encode constraints.
- Use typestate/newtypes where they prevent invalid states.
- Avoid overuse of `clone`, `unwrap`, `expect`, global state, and mutable shared state.
- Do not introduce premature `unsafe`.
- Avoid cleverness that makes future agent maintenance harder.

## Macro Policy

- Macros are allowed only when they improve readability or eliminate noisy repeated structure.
- Prefer functions, traits, builders, derives, and declarative data before macros.
- Macro-generated behavior must be obvious at call sites.
- Macro use must include tests for expanded behavior.
- No macro DSLs unless there is a strong repeated domain pattern.
- Do not hide business logic in macros.

## Final Handoff

Every final response must include:

- Files changed.
- Architectural decisions made.
- Tests and checks run.
- Tests not run, with reasons.
- Remaining risks or follow-up work.
