# exec-011: `row_fold_only: bool` restates `pack` and `measures`

- **Severity:** low
- **Tree:** exec
- **Status:** FIXED(44db7ad6)
- **Source:** audit/plan-exec.md F14
- **Depends on:** exec-001 (optional; AggSpec may absorb Pack/measures into a body sum)

## The bug

`crates/bumbledb/src/exec/sink.rs:377-381`; `sink/aggregate/new.rs:227`: `row_fold_only = pack.is_some() || !measures.is_empty()`. Stored, then tested on the scan path (`aggregate/sink.rs:28,198`). `aim` updates `pack`/`measures` and does not recompute the flag — sticky-correct only because Pack-with-measures is validation-refused.

## Why it's wrong

A flag that is a function of two other fields (Insight 4). Drift is representable the moment `aim` or a new head shape updates one field without the other.

## The fix

Per `audit/CONTRACT.md` §C1: don't store it. Test `self.pack.is_some() || !self.measures.is_empty()` at the two scan sites, or make Pack/measures a sum that *is* the row-fold arm.

## Acceptance criteria

- [ ] Gone: `rg -nw 'row_fold_only' crates/bumbledb/src/exec` → no matches.
- [ ] Unchanged tests: aggregate scan-fold still declines under Pack and under measures; batch path unchanged.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Decline predicates identical. Pure deletion of a cached bool, or a body-sum if exec-001 is landing the same files.
