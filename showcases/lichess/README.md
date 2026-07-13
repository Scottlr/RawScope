# Lichess showcase

This is the executable version of RawScope's existing Lichess example. It shows
how a downstream Rust crate can prepare and launch a session using only the
consumer-facing `rawscope-adapters` API, imported as `rawscope` in
`Cargo.toml`. It does not depend on RawScope's data, session, render, or
workbench implementation crates.

The input is an existing local CSV with the profile columns:

- `created_at` (integer)
- `white_rating` (integer)
- `black_rating` (integer)
- `winner` (string)
- `game_id` (unique, non-empty evidence identifier)

Prepare a persistent session manifest without starting the workbench:

```powershell
cargo run -p rawscope-showcase-lichess -- prepare C:\data\games_profile.csv
```

Or prepare and launch it in one command:

```powershell
cargo install --path apps/rawscope-workbench
cargo run --release -p rawscope-showcase-lichess -- run C:\data\games_profile.csv
```

The optional final argument selects the output directory. It defaults to
`.showcase-data/lichess/session`. RawScope will not overwrite an existing
session directory, and the generated manifest references the source CSV in
place rather than copying it.

The showcase intentionally does not download or redistribute Lichess data.
Supply data you are entitled to use and retain its source, licence, and
attribution alongside your local copy.
