---
name: rust-code-organization
description: Use before adding, moving, or restructuring Rust modules, crate files, facades, domain folders, or tests that affect repo navigation.
---

# Rust Code Organization

## When to use this skill

Use this skill before changing Rust file layout, adding modules, splitting files, creating facades, or deciding where behavior belongs.

## Rules

- Organize by domain ownership first, not by generic technical buckets.
- Split by responsibility and change pattern, not arbitrary line count.
- Files over roughly 400 lines require a written justification or a split plan.
- Do not create god files or god modules.
- Do not create dumping-ground files named `utils.rs`, `helpers.rs`, `common.rs`, `misc.rs`, `types.rs`, `models.rs`, `contracts.rs`, `dto.rs`, or `packet.rs`.
- `lib.rs`, `main.rs`, and `mod.rs` should be thin routing/facade files.
- Use `pub use` intentionally to expose a narrow public facade.
- Keep concepts that change together near each other.
- Keep large behavioral and integration tests out of production files.
- Name non-trivial loop ranges immediately before the loop that uses them when practical.
- Do not bury range arithmetic or domain boundaries inside `for`, `if`, `while`, or `match` headers.
- Before creating a new module, check whether an existing domain owner should be extended.

## Good patterns

Domain-first packet layout:

```text
src/
  packet/
    mod.rs
    msg_action.rs
    msg_item.rs
    msg_friend.rs
  decode/
    route.rs
    client.rs
  encode/
    writer.rs
  error.rs
```

Thin facade:

```rust
mod msg_action;
mod msg_friend;
mod msg_item;

pub use msg_action::{MsgAction, MsgActionError};
pub use msg_friend::MsgFriend;
pub use msg_item::MsgItem;
```

A domain file may own the pieces that make that concept understandable:

```rust
//! Action packet decoding, validation, and encoding.

pub struct MsgAction {
    pub row_id: RowId,
    pub action: ActionKind,
}

impl MsgAction {
    pub fn validate(&self) -> Result<(), PacketError> {
        // Domain validation belongs close to the domain type.
        Ok(())
    }
}
```

It is acceptable for a packet/domain file to own:

- Rustdoc explaining purpose.
- Decode logic.
- Encode/write logic.
- Validation.
- Tiny domain-owned unit tests.

Large behavioral or integration tests belong under `tests/`.

Name derived ranges before looping:

```rust
let stale_lane_start_row = spike_end_row;
let stale_lane_end_row = stale_lane_start_row + stale_lane_count;
let stale_lane_rows = stale_lane_start_row..stale_lane_end_row;
for row_index in stale_lane_rows {
    generate_stale_lane_event(row_index);
}
```

## Bad patterns

Do not do this:

```text
src/
  lib.rs          # 900 lines of implementation
  utils.rs        # random parsing, validation, IO, and formatting helpers
  types.rs        # every public type in the crate
  packet.rs       # every packet plus all decode and encode logic
```

Do not split a coherent 120-line domain file into five files just to look modular. Navigation should improve after a split.

Do not expose a broad public API for convenience:

```rust
pub mod decode;
pub mod encode;
pub mod internal_state;
pub mod scratch;
```

Prefer narrow re-exports:

```rust
mod decode;

pub use decode::{decode_client_packet, ClientDecodeRoute};
```

Do not hide domain boundaries in the loop header:

```rust
for row_index in (background_count + spike_count)..(background_count + spike_count + stale_count) {
    generate_stale_lane_event(row_index);
}
```

## Checklist before editing

- Which domain owns this behavior?
- What existing modules already cover this concept?
- Will a new file improve navigation or create a parallel concept?
- Are `lib.rs`, `main.rs`, or `mod.rs` staying thin?
- Are public exports intentional and minimal?
- Does any file over roughly 400 lines need a split plan?
- Are derived ranges and non-trivial branch predicates named near their use?
- Are tests located where they prove behavior without bloating production files?

## Checklist before final response

- List files created, moved, or reorganized.
- Explain the ownership decision.
- Note any file that still risks becoming too large.
- Confirm no dumping-ground module was introduced.
- Confirm production facades stayed thin.
