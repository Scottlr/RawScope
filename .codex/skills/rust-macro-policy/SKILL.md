---
name: rust-macro-policy
description: Use before adding, expanding, or depending on Rust macros, macro_rules, proc macros, derives, or macro-based DSLs.
---

# Rust Macro Policy

## When to use this skill

Use this skill before introducing a macro, expanding a macro's scope, depending on a new macro crate, or turning repeated code into a macro-based pattern.

## Rules

- Macros are justified only when they improve readability or eliminate noisy repeated structure.
- Prefer functions, traits, builders, derives, and declarative data before macros.
- Derive macros from stable crates are acceptable when they express standard behavior clearly.
- Declarative macros for repeated protocol boilerplate may be valid when the repeated pattern is stable.
- Macro call sites must remain readable.
- Macro-generated behavior must be obvious at call sites.
- Macro behavior must be tested.
- Do not hide business logic in macros.
- Do not create a macro DSL unless a repeated domain grammar already exists.
- Do not use macros to avoid understanding ownership, generics, or module design.

## Good patterns

Optional packet route registration macro when route boilerplate is repetitive and stable:

```rust
macro_rules! packet_routes {
    ($($id:expr => $ty:ty),+ $(,)?) => {
        pub fn decode_route(id: u16, bytes: &[u8]) -> Result<ClientPacket, ProtoError> {
            match id {
                $(
                    $id => <$ty>::decode(bytes).map(ClientPacket::from),
                )+
                other => Err(ProtoError::UnknownRoute { route_id: other }),
            }
        }
    };
}

packet_routes! {
    CLIENT_ACTION_ROUTE_ID => MsgAction,
    CLIENT_ITEM_ROUTE_ID => MsgItem,
}
```

This is acceptable only if each packet type owns its real decode logic and tests cover generated routing behavior.

Prefer derive macros for standard behavior:

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PacketError {
    #[error("unknown route {route_id}")]
    UnknownRoute { route_id: u16 },
}
```

## Bad patterns

Macro hiding business logic:

```rust
make_packet!(MsgAction, validate_everything, decode_everything, export_everything);
```

Macro DSL before the domain is stable:

```rust
rawscope_dsl! {
    view scatter density magic optimize all
}
```

Macro used instead of a normal function:

```rust
macro_rules! add_one {
    ($value:expr) => {
        $value + 1
    };
}
```

## Checklist before editing

- What repeated structure would the macro remove?
- Can a function, trait, builder, derive, or data table solve this more clearly?
- Is the generated behavior obvious at the call site?
- Will each domain type still own its real logic?
- What tests prove the expanded behavior?
- Is this a stable repeated pattern or premature cleverness?
- Does the macro make compile errors harder for future agents?

## Checklist before final response

- State why a macro was or was not introduced.
- List macro call sites changed.
- List tests that cover generated behavior.
- Confirm business logic remains outside the macro.
- Note any remaining repetition intentionally left as normal Rust.
