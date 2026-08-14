# exec-007: `batch_sources` / `scan_sources` are `Vec<Option<usize>>` after `LeafSource` exists

- **Severity:** medium
- **Tree:** exec
- **Status:** FIXED(44db7ad6)
- **Source:** audit/plan-exec.md F10
- **Depends on:** exec-001 (`scan_sources` Count-as-None rides the AggSpec sum)

## The bug

`LeafSource { Key(usize) | Outer }` already exists (`exec/run.rs:57-63`). Projection then writes `Some(word) | None` into `batch_sources: Vec<Option<usize>>` (`sink.rs:244`; `projection/sink.rs:34-37`). Aggregate scan writes `Option<usize>` into `scan_sources` (`sink.rs:423`; `aggregate/sink.rs:45`: `over_slot.and_then(|slot| key_slots.position(...))`). The sum was parsed, then flattened so every row loop re-tests `if let Some`.

## Why it's wrong

Insight 5: null in every projected word. Insight 6: `LeafSource` was the parse; Option is the discarded proof. Count-as-None in `scan_sources` is exec-001's hole in a second array.

## The fix

Per `audit/CONTRACT.md` §C1:

- Projection: `batch_sources: Vec<LeafSource>` (or reuse `MeasuredSource`, which already exists for the measured path).
- Scan: `enum FoldSource { Outer, Column(usize) }` aligned with fold inputs — Count contributes *no slot* (exec-001), not a None in a parallel array.

## Acceptance criteria

- [ ] Gone: `rg -n 'batch_sources: Vec<Option' crates/bumbledb/src/exec` → no matches; `rg -n 'scan_sources: Vec<Option' crates/bumbledb/src/exec` → no matches.
- [ ] Unchanged tests: projection batch/scan and aggregate scan-fold suites green; answers identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Per-slot resolution stays at batch/scan entry, never per-row. Land after or with exec-001 so Count is not a None hole.
