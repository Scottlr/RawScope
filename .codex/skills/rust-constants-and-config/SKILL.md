---
name: rust-constants-and-config
description: Use when introducing strings, numbers, limits, packet IDs, route names, byte sizes, retry counts, timeouts, thresholds, or configuration.
---

# Rust Constants And Config

## When to use this skill

Use this skill whenever code introduces or repeats strings, numbers, IDs, limits, byte sizes, retry counts, timeouts, thresholds, route names, file names, or environment/configuration values.

## Rules

- Replace repeated strings and numbers with named constants or typed config.
- Constants should live near the domain that owns them.
- Do not create a giant global `constants.rs`.
- Use names that explain units, such as `DEFAULT_TIMEOUT_MS`, `MAX_ROWS`, or `CHUNK_SIZE_BYTES`.
- Config struct fields should also carry units, such as `timeout_ms`, `max_rows`, or `chunk_size_bytes`.
- Prefer enums/newtypes for constrained values.
- Document protocol numbers, packet IDs, byte flags, and externally defined limits with source/context comments.
- Do not hide domain decisions in anonymous literals.
- Do not promote a one-off literal into a global constant unless it has domain meaning or is repeated.

## Good patterns

Domain-owned constants:

```rust
mod density {
    /// Chosen to fit one tile in the current CPU reference fixture size.
    pub const DEFAULT_TILE_SIZE_PX: u32 = 256;
    pub const MAX_ROWS_PER_TILE: usize = 65_536;
}
```

Typed config:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryConfig {
    pub max_attempts: u8,
    pub backoff_ms: u64,
}
```

Protocol numbers with context:

```rust
/// Client action packet route ID from protocol v3 route table.
pub const CLIENT_ACTION_ROUTE_ID: u16 = 0x0012;
```

Constrained values as enums:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceFormat {
    Markdown,
    Html,
    Json,
}
```

## Bad patterns

Magic numbers:

```rust
if payload.len() > 65536 {
    return Err(PacketError::TooLarge);
}
```

Magic strings:

```rust
if format == "markdown" {
    export_markdown(report)?;
}
```

Global dumping ground:

```text
src/
  constants.rs    # every unrelated route, timeout, limit, label, and file name
```

## Checklist before editing

- Is this literal repeated?
- Does this literal encode domain knowledge?
- Which module owns this value?
- Does the name include units where needed?
- Should this be a constant, config field, enum, or newtype?
- Does an existing config or constant already cover it?
- Does a protocol value need a source/context comment?

## Checklist before final response

- List constants/config added or changed.
- Explain where they live and why that domain owns them.
- Confirm no global dumping-ground `constants.rs` was introduced.
- Confirm units are clear in names.
- Note any remaining magic values and why they are acceptable.
