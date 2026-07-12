# Publishing policy

All current RawScope packages are private development artifacts. This policy
prevents accidental publication while the open-source readiness programme is
in progress.

| Package | Current state | Intended consumer | Publication prerequisite |
| --- | --- | --- | --- |
| `rawscope-core` | private | internal Rust domain contracts | T035 and explicit maintainer approval |
| `rawscope-data` | private | internal Rust data ingestion | T035 and explicit maintainer approval |
| `rawscope-analysis` | private | internal Rust analysis | T035 and explicit maintainer approval |
| `rawscope-evidence` | private | internal Rust evidence contracts | T035 and explicit maintainer approval |
| `rawscope-gpu` | private | internal WGPU support | T035 and explicit maintainer approval |
| `rawscope-render` | private | internal presentation layer | T035 and explicit maintainer approval |
| `rawscope-session` | private | internal startup/session loading | T035 and explicit maintainer approval |
| `rawscope-session-contracts` | private | Rust/Python session wire parity | T035 and explicit maintainer approval |
| `rawscope-workbench` | private | native application | T035 and explicit maintainer approval |
| `rawscope-sdk` | private | local Python bridge | T035 and explicit maintainer approval |

T035 is the only task allowed to propose enabling publication for an approved
package set. No task may publish crates, wheels, binaries, tags, or releases as
an implementation side effect. The dual-licence texts and exact copyright
attribution remain intentionally absent until the maintainer approves them.
