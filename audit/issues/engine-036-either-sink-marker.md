# engine-036: `_either_sink_marker` — a dead function hushing an import kept for layout symmetry

- **Severity:** low
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F36
- **Depends on:** engine-026 (the projection-only derived story makes the import obviously wrong)

## The bug

`crates/bumbledb/src/api/prepared/reach.rs:651-653`:

```rust
/// Suppress unused-import warning if EitherSink is only needed by execute.
#[allow(dead_code)]
fn _either_sink_marker(_: &EitherSink) {}
```

## Why it's wrong

A null object for a missing use (Insight 15): the import exists to keep reach.rs's use-list symmetric with execute.rs, and a dead function exists to keep the import. Derived tables are projection-only (engine-026); `EitherSink` genuinely does not belong here.

## The fix

Delete the marker fn and the `EitherSink` import from reach.rs's `use super::{...}` list (line 13).

## Acceptance criteria

- [ ] Gone: `rg -n '_either_sink_marker|EitherSink' crates/bumbledb/src/api/prepared/reach.rs` → no matches.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh` (clippy clean — no unused-import warning replaces the marker).

## Constraints

- Two-line deletion; zero behavior change.
