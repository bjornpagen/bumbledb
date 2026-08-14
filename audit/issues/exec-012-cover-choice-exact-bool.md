# exec-012: `cover_choice(..., exact: bool)` throws away `KeyCount`

- **Severity:** low
- **Tree:** exec
- **Status:** OPEN
- **Source:** audit/plan-exec.md F15
- **Depends on:** none (counters seam; parallel-safe)

## The bug

`KeyCount { Exact(u64) | Estimate(u64) }` already exists (`exec/colt.rs:51-67`) and is the cover-choice comparison. The counters seam (`exec/run.rs:198-199`) then accepts `exact: bool` — the tag without the magnitude. Introspection histograms Exact vs Estimate from that bit.

## Why it's wrong

The sum exists; the observability seam flattened it to a bool (Insight 6). Label-first preference on this tag was the documented cover-choice bug; passing only the tag is how that class of mistake re-enters.

## The fix

Per `audit/CONTRACT.md` §C1: `fn cover_choice(&mut self, node: usize, subatom: usize, count: KeyCount)`. Histogram matches the enum. No bool.

## Acceptance criteria

- [ ] Gone: `rg -n 'fn cover_choice\([^)]*exact: bool' crates/bumbledb/src/exec` → no matches.
- [ ] Unchanged tests: cover-choice histograms still split Exact vs Estimate; magnitudes unused by stats (as today) unless a new field is explicitly added — do **not** add one.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`. `INTROSPECTION_VERSION` unchanged (same counted surface).

## Constraints

- Observability-only. Do not change cover-choice *policy* (magnitude first, label breaks ties). Coordinate with engine-012/029 only if a stats-shape change is already bumping the version — this issue alone must not bump it.
