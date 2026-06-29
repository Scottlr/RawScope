---
name: agent-preflight-and-handoff
description: Use at the start and end of every repo task to inspect context, list skills, name risks, set non-goals, and provide a complete final handoff.
---

# Agent Preflight And Handoff

## When to use this skill

Use this skill for every non-trivial repository task before editing and again before the final response.

## Rules

- Inspect relevant files before editing.
- List skills loaded or applicable.
- Summarize existing patterns found.
- Identify likely architectural risks.
- Provide a short plan.
- Say what will not be done.
- Before creating a new file, app, example, test harness, module, macro, or abstraction, check whether an existing owner or pattern already exists.
- Prefer extending the existing owner over creating a parallel concept.
- Final handoff must list files changed, architectural decisions, tests run, tests not run and why, and remaining risks.

## Good patterns

Preflight response template:

```text
Skills: rust-code-organization, rust-testing-policy, rust-idioms-and-abstractions.
Files inspected: Cargo.toml, crates/rawscope-data/src/lib.rs, tests/.
Existing pattern: thin crate facades with domain-owned library modules.
Risks: creating a generic data bucket, adding file import before synthetic data, weak tests.
Plan: add synthetic point/event data under the data owner, add CPU density reference, add deterministic tests.
Non-goals: no WGPU, no egui, no file import, no benchmark claims.
```

Final response template:

```text
Changed files: ...
Architectural decisions: ...
Tests run: ...
Tests not run: ...
Remaining risks: ...
```

## Bad patterns

Starting with edits before inspection:

```text
I added a new utils module because it seemed useful.
```

Creating parallel concepts:

```text
src/data.rs
src/dataset/
src/datasets.rs
src/model.rs
```

Incomplete handoff:

```text
Done.
```

## Checklist before editing

- Which files did I inspect?
- Which skills apply?
- What existing owner or pattern should this extend?
- What architectural risks could this task introduce?
- What is the smallest useful plan?
- What will I explicitly not do?
- Am I about to create a new file/module/test/app/macro/abstraction, and have I checked for an existing owner?

## Checklist before final response

- Did I re-read important changed files?
- Did I remove avoidable repetition?
- Did I run formatting, tests, linting, or explain why not?
- Did I list changed files?
- Did I explain architectural decisions?
- Did I list tests run?
- Did I list tests not run and why?
- Did I mention remaining risks or follow-up work?
