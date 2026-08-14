# exec-010: `all_cancelled: bool` and `poison: Option<Poison>` are one stop, two fields

- **Severity:** medium
- **Tree:** exec
- **Status:** OPEN
- **Source:** audit/plan-exec.md F13
- **Depends on:** none (executor stop state; parallel-safe)

## The bug

`crates/bumbledb/src/exec/run.rs:646-657` comments that poison is "one sum, not parallel flags" and `all_cancelled` is "the ONE stop condition" — then there are two fields. `cancel.rs:11-14` `poison()` writes both (`get_or_insert` + `all_cancelled = true`). D2 root-skip writes only `all_cancelled` (`probe_pass.rs:670`). Representable: `poison: Some` with `all_cancelled == false`. Loops test the bool; `execute.rs:387-392` drains the Option.

## Why it's wrong

Insight 4: two independent fields admit unpaired poison. The pairing lives in `poison()`'s convention, which is exactly the guard a sum deletes. Dual encoding of "stop," with the reason as a sidecar that can go missing.

## The fix

Per `audit/CONTRACT.md` §C1:

```rust
enum DriveState { Running, SkipDone, Poisoned(Poison) }
```

Loops test `!= Running`. `execute` matches `SkipDone` vs `Poisoned`. Unpaired poison is unrepresentable. D2 skip and typed `OriginOverflow` stay distinct answers.

## Acceptance criteria

- [ ] Gone: `rg -nw 'all_cancelled' crates/bumbledb/src/exec` → no matches; `rg -n 'poison: Option<Poison>' crates/bumbledb/src/exec/run.rs` → no matches.
- [ ] Unchanged tests: D2 skip, origin-overflow, and MeasureOfRay (if it shares the poison drain) suites green; answers and error names identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- `OverflowKind::OriginCapacity` name and behavior locked. A skip is still an answer, not an error. Poison still set-once (first wins). Measure-of-ray may stay sink-side (`ray: Option`) — this issue is the executor's drive stop only.
