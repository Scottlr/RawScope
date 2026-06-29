---
name: rust-testing-policy
description: Use before adding or changing Rust tests, fixtures, examples, smoke checks, snapshots, property tests, or test harnesses.
---

# Rust Testing Policy

## When to use this skill

Use this skill whenever a change adds tests, modifies tests, creates fixtures, introduces examples, or claims behavior is verified.

## Rules

- Meaningful tests only.
- No "it starts" tests unless startup is the real behavior under test.
- No smoke app sprawl.
- No fake apps created as validation.
- Tests must prove behavior that would fail if the implementation were fake.
- Test behavior and contracts, not private implementation details.
- Inline unit tests are only for tiny, local behavior.
- Larger behavior tests belong under `tests/`.
- Shared integration fixtures/helpers belong under `tests/common/`.
- Prefer table-driven tests.
- Prefer deterministic fixtures.
- Cover edge cases and failure paths, not just success paths.
- Production files should not become mostly tests.
- Test names should describe behavior, not implementation.

## Good patterns

Table-driven behavior test:

```rust
#[test]
fn density_bins_points_on_expected_edges() {
    let cases = [
        (Point { x: 0.0, y: 0.0 }, Bin { x: 0, y: 0 }),
        (Point { x: 0.99, y: 0.99 }, Bin { x: 0, y: 0 }),
        (Point { x: 1.0, y: 1.0 }, Bin { x: 1, y: 1 }),
    ];

    for (point, expected) in cases {
        assert_eq!(bin_point(point, 10, 10), expected);
    }
}
```

Failure-path test:

```rust
#[test]
fn decode_rejects_payload_shorter_than_header() {
    let err = decode_client_packet(&[0x01]).expect_err("short payload should fail");

    assert!(matches!(err, ProtoError::PayloadTooShort { .. }));
}
```

Integration fixture layout:

```text
tests/
  common/
    mod.rs
    fixtures.rs
  decode_packets.rs
  density_reference.rs
```

## Bad patterns

Meaningless smoke test:

```rust
#[test]
fn it_runs() {
    assert!(run().is_ok());
}
```

Implementation-detail test:

```rust
#[test]
fn parser_uses_three_internal_steps() {
    assert_eq!(parser.debug_step_count(), 3);
}
```

Fake validation app:

```text
apps/
  smoke-check/
    src/main.rs
```

Do not create a new app just to prove that a library compiles. Use `cargo check`, `cargo test`, and focused behavior tests.

## Checklist before editing

- What behavior should fail if the implementation is fake?
- Is this a unit, integration, snapshot, or property-style test?
- Can the test be deterministic?
- Does it cover failure paths and edge cases?
- Should shared setup live in `tests/common/`?
- Is the test name behavior-focused?
- Would this make a production file mostly tests?
- Is there an existing test owner or pattern to extend?

## Checklist before final response

- List tests added or changed.
- List commands run, usually `cargo fmt --check`, `cargo test`, `cargo clippy`, or `cargo check --workspace`.
- State any tests not run and why.
- Confirm no smoke app or meaningless smoke test was introduced.
- Note any remaining behavior that lacks coverage.
