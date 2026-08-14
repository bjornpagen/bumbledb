# exec-001: `FindSpec::Agg` / `SinkSpec::Agg` keep Count as `over_slot: None`

- **Severity:** high
- **Tree:** exec
- **Status:** OPEN
- **Source:** audit/plan-exec.md F1
- **Depends on:** none (sink vocabulary; `find_specs` in `api/prepared/build.rs` is the one writer)

## The bug

CONTRACT §C6 already split SDK aggregate finds: Count carries no `over`; folds require it. The engine's trusted `FindSpec` (and post-parse `SinkSpec`) still spell both as one product (`exec/sink.rs:77-86,102-114`):

```rust
Agg {
    op: FoldOp,              // includes Count
    over_slot: Option<usize>, // None = Count
    over_width: usize,
    signed: bool,
}
```

`api/prepared/build.rs:1156-1181` copies the hostile `FindTerm::Aggregate { over: Option }` into this hole (`None => (None, 1, U64) // Count`). The IR already did the right split for *measures* (`FindTerm::AggregateMeasure { over: VarId }` is required) then put ordinary aggregates back into Option. Every fold site then `over_slot.expect("validated: Sum has a variable")` (`fold_row.rs:75-96`, `fold_batch.rs:132-157`, `aggregate/sink.rs:143-157`). Illegal states representable: `Count`+`Some(slot)`, `Sum`+`None`, `Count`+`signed: true`.

## Why it's wrong

Insight 4: one product, several valid states, the rest guarded by `expect`. Insight 6: IR validation judged Count-has-no-over and the sink throws the proof away. C1: the hostile `FindTerm` Option stays; the *trusted* `FindSpec` is a sum. C6 already named the collapsing type for SDKs — the engine never got the same parse.

## The fix

Per `audit/CONTRACT.md` §C1 (trusted layer is a sum) and §C6 (Count carries no over; the analogous engine parse):

```rust
enum AggSpec {
    Count,
    Fold { op: FoldOp /* Sum|Min|Max */, slot: usize, width: usize, signed: bool },
}
enum FindSpec { Var {..}, Duration {..}, Agg(AggSpec), AggDuration {..}, Pack {..} }
enum SinkSpec { Var {..}, Agg(AggSpec), Pack {..} }
```

- `find_specs` matches `AggOp::Count` into `AggSpec::Count`; folds require `over`.
- Fold sites match `Count` vs `Fold` — no `expect`.
- `union_span` / scan `over_slot.and_then` become the Fold arm (Count contributes nothing, as today).
- Hostile `ir.rs::FindTerm::Aggregate { over: Option }` **does not change** (C1).

## Acceptance criteria

- [ ] Gone: `rg -n 'over_slot: Option' crates/bumbledb/src/exec/sink.rs` → no matches; `rg -n 'over_slot.expect' crates/bumbledb/src/exec` → no matches.
- [ ] Unchanged tests: `cargo test -p bumbledb` green with zero assertion edits; Count still emits the group key with no fold input; Sum/Min/Max still require a variable.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`. Corpus / C ABI / `FindTerm` shape untouched.

## Constraints

- C1: `ir.rs::FindTerm` stays. This issue is `FindSpec`/`SinkSpec` (trusted) plus the one `find_specs` writer in prepare. `FoldOp::Count` may remain as the accumulator tag seeded from `AggSpec::Count`, or Count becomes its own `Acc` construction without a FoldOp — either, so long as `AggSpec` cannot spell Count-with-a-slot.
