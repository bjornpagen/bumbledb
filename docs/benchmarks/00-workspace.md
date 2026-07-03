# PRD 00 — Bench crate skeleton and the dependency-quarantine ruling

Authority: `docs/architecture/00-product.md` (deps doctrine, success criteria),
`50-validation.md` (the suite this crate implements), `docs/benchmarks/README.md`
rules.

## Purpose

Create `crates/bumbledb-bench` — the one home for the oracle, the benchmark, and
trace tooling — and record the dependency-quarantine ruling in the architecture docs.

## Technical direction

- New workspace member `crates/bumbledb-bench`: `src/lib.rs` plus `src/main.rs`
  (binary `bumbledb-bench`). Library-first: every capability is a `pub` library
  function; `main.rs` is argument parsing plus dispatch only.
- `Cargo.toml`: `bumbledb = { path = "../bumbledb", features = ["trace",
  "alloc-counter"] }` — wait: features do not exist yet; depend with **no features**
  now and add them in the PRDs that create them (02, 13). `rusqlite = { version =
  "0.32", features = ["bundled"] }` — bundled so the canonical machine's system
  SQLite version is irrelevant and pinned. **No other dependencies.** The workspace
  lint table applies (clippy pedantic, `unsafe_code = "deny"` — this crate has zero
  sanctioned unsafe).
- Module skeleton (empty `//!`-documented modules compile from day one; each later
  PRD fills exactly one or two): `schema`, `gen`, `corpus`, `sqlmap`, `compare`,
  `querygen`, `verify`, `harness`, `families`, `sqlite_run`, `trace_out`, `report`,
  `cli`. A module may not be `pub` until a PRD gives it content (no mechanism
  without a reader — empty modules are `pub(crate)` placeholders or simply absent
  until needed; prefer absent).
- `main.rs`: hand-rolled argument parsing scaffold — a `Cmd` enum with a `parse(args:
  &[String]) -> Result<Cmd, String>` function and a `help()` string. Subcommands
  land in PRD 19; today `help` and version (from `env!("CARGO_PKG_VERSION")`) exist.
- Amend `docs/architecture/00-product.md`'s dependency doctrine in the same change:
  engine crates stay `heed + blake3` exactly; `bumbledb-bench` is the quarantined
  member and may hold `rusqlite (bundled)` only; the quarantine is one-directional
  (nothing in the engine may ever depend on the bench crate).
- `scripts/check.sh` already runs workspace-wide; confirm it covers the new member
  (it does via `--workspace`; no change expected — verify, don't assume).

## Non-goals

Any real functionality. clap/serde/criterion (forbidden permanently, README rule 4).

## Passing criteria

- Workspace builds; `cargo run -p bumbledb-bench -- help` prints the help text and
  exits 0; unknown arguments exit nonzero with the help text on stderr.
- Unit tests: `Cmd::parse` accepts `help`, rejects garbage with a message naming the
  offending token.
- `00-product.md` carries the quarantine amendment. `scripts/check.sh` green.
