# RawScope Evidence Schema Index

This index records the version families without changing any existing wire
contract. Legacy readers and writers remain byte-compatible for valid v1-v5
scatter and v1-v3 timeline artifacts.

| Family | Versions | Status |
| --- | --- | --- |
| Scatter selection | v1, v2, v3, v4, v5 | Legacy-compatible readers/writers |
| Scatter selection | v6 | New portable contract; canonical source is `rawscope-evidence` |
| Timeline selection | v1, v2, v3 | Legacy-compatible readers/writers |
| Timeline selection | v4 | New portable contract; canonical source is `rawscope-evidence` |
| Session | v1 | Bounded session contract |
| Session | v2 | Generic numeric-pair, time-value, and lane bindings |

New artifacts must use the explicit constants exported by `rawscope-evidence`:

- `SCATTER_SELECTION_EVIDENCE_V6_SCHEMA_VERSION = 6`
- `TIMELINE_SELECTION_EVIDENCE_V4_SCHEMA_VERSION = 4`

The v6/v4 contracts are additive. They carry portable provenance by default,
precise numeric/source semantics, and explicit GPU quantization disclosure.
Sensitive absolute paths require an explicit provenance policy; they are never
implied by a source label.

Session v2 is additive. Legacy v1 manifests and writers remain byte-stable;
see [`session-v2.md`](session-v2.md) for the generic binding contract.

