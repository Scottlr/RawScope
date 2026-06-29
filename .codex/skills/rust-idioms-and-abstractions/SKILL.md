---
name: rust-idioms-and-abstractions
description: Use when designing Rust types, ownership, traits, builders, newtypes, typestate, generics, error types, borrowing, or copy-minimising APIs.
---

# Rust Idioms And Abstractions

## When to use this skill

Use this skill when shaping Rust APIs, domain types, ownership boundaries, abstraction seams, validation, error handling, or performance-sensitive data movement.

## Rules

- Advanced Rust is welcome when it makes illegal states impossible, improves performance, or clarifies ownership.
- Advanced Rust is not welcome when it is just showing off.
- Prefer explicit structs and enums over stringly typed maps.
- Use newtypes for IDs, constrained values, and units that should not mix.
- Use enums for closed sets of states or routes.
- Use `Result<T, E>` and domain errors for recoverable failures.
- Use `thiserror` or an equivalent domain-error pattern when errors need source/context.
- Use traits for real polymorphic seams, not speculative abstraction.
- Use builders when construction has many optional fields or validation steps.
- Use typestate when it prevents a real invalid workflow.
- Use `Cow`, borrowing, and `Arc` where they clarify ownership or reduce copying.
- Prefer iterators where clearer and loops where clearer.
- Avoid unnecessary allocation, `clone`, `unwrap`, `expect`, global state, and mutable shared state.
- Copy-minimising design is good; do not claim zero-copy unless it is technically exact.

## Good patterns

Newtypes prevent unit mixups:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkSizeBytes(pub usize);
```

Enums make routes explicit:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientDecodeRoute {
    Action,
    Item,
    Friend,
}
```

Traits should describe a real seam:

```rust
pub trait DensitySink {
    fn add_bin(&mut self, x: u32, y: u32, count: u32);
}
```

Builders are useful when validation matters:

```rust
pub struct ViewSpecBuilder {
    width: Option<u32>,
    height: Option<u32>,
}

impl ViewSpecBuilder {
    pub fn build(self) -> Result<ViewSpec, ViewSpecError> {
        let width = self.width.ok_or(ViewSpecError::MissingWidth)?;
        let height = self.height.ok_or(ViewSpecError::MissingHeight)?;
        Ok(ViewSpec { width, height })
    }
}
```

## Bad patterns

Speculative abstraction:

```rust
pub trait UniversalProcessor<TInput, TOutput> {
    fn process(&self, input: TInput) -> TOutput;
}
```

Stringly typed state:

```rust
let route = map.get("route").unwrap().to_string();
```

Unnecessary clone:

```rust
let rows = dataset.rows.clone();
render(rows);
```

Use borrowing or shared ownership only when the ownership story requires it.

## Checklist before editing

- What invalid state can this API prevent?
- Would a newtype or enum make the contract clearer?
- Is a trait needed by multiple real implementations now?
- Is a builder justified by optional fields or validation?
- Can borrowing avoid allocation without making the API awkward?
- Are errors domain-specific and contextual?
- Is any `clone`, `unwrap`, or shared mutable state avoidable?
- Is advanced Rust clarifying the code rather than decorating it?

## Checklist before final response

- List new abstractions and why they exist.
- Explain ownership and error-handling decisions.
- Confirm no speculative trait, global state, or unnecessary clone-heavy API was introduced.
- Note any intentionally deferred abstraction.
- List tests/checks run.
