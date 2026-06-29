---
name: rust-observability
description: Use when adding or changing tracing, logging, spans, error context, IO workflows, parsing, decode/encode, background tasks, retries, or state transitions.
---

# Rust Observability

## When to use this skill

Use this skill before adding logs, spans, instrumentation, error context, retry reporting, background-task reporting, or diagnostics.

## Rules

- Prefer `tracing` over ad-hoc logging.
- Use spans for workflows with meaningful duration or nested operations.
- Logs should include structured fields that help diagnose the issue.
- Errors should preserve source and context.
- Do not swallow errors.
- Do not use `println!` or `eprintln!` for production diagnostics.
- Do not log noisy success paths unless the signal is useful.
- Do not log secrets, tokens, full raw payloads, or large buffers.
- Avoid vague errors like `"failed"` or `"invalid data"`.
- Use log levels consistently.

## Good patterns

Instrument decode logic with useful fields:

```rust
#[tracing::instrument(skip(bytes), fields(route = ?route, len = bytes.len()))]
pub fn decode(route: ClientDecodeRoute, bytes: &[u8]) -> Result<ClientPacket, ProtoError> {
    // ...
}
```

Preserve source and context:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("payload too short for {route:?}: expected at least {expected} bytes, got {actual}")]
    PayloadTooShort {
        route: ClientDecodeRoute,
        expected: usize,
        actual: usize,
    },

    #[error("failed to read packet source")]
    ReadSource(#[source] std::io::Error),
}
```

Use levels intentionally:

```rust
tracing::debug!(chunk_id, row_count, "loaded chunk metadata");
tracing::info!(dataset_id, rows = row_count, "dataset opened");
tracing::warn!(retry = attempt, error = %err, "retrying transient read failure");
tracing::error!(error = %err, "background task failed");
```

## Bad patterns

No production `println!` debugging:

```rust
println!("got here");
```

No unstructured or context-free errors:

```rust
return Err("failed".into());
```

No swallowed errors:

```rust
if let Err(_) = write_report(report) {
    // ignored
}
```

## Checklist before editing

- Is this code doing IO, parsing, decode/encode, background work, expensive work, retries, or state transitions?
- Would a span help connect related logs?
- Which fields identify the dataset, route, chunk, selection, row range, retry, or operation?
- Are errors preserving source and enough context?
- Could any field expose secrets or large payloads?
- Is the chosen level `debug`, `info`, `warn`, or `error` appropriate?
- Is there existing tracing setup or error style to follow?

## Checklist before final response

- List observability changes.
- Explain what context the logs/spans/errors now preserve.
- Confirm no production `println!` or unstructured debug logging was added.
- Note any diagnostics still missing.
- List tests/checks run.
