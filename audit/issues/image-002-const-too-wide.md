# image-002: `Const` is a universal value; `ResolvedWordSource::Var` is inhabited then `unreachable!`

- **Severity:** medium
- **Tree:** image
- **Status:** OPEN
- **Source:** audit/storage-schema.md F13
- **Depends on:** none
- **Conflicts with:** none

## The bug

`FilterPredicate` (`image/view.rs:112-209`) is a good kind-sum. Its payloads then carry `Const`, which admits Word/Byte/Words/Interval/Param/ParamSet/WordSet/PendingIntern at every site. `FieldAllen.other`, `DurationCompare.value`, `AnyPointIn.set` each legally hold the wrong arm. `image/view/apply.rs` is a forest of `unreachable!("validated: …")`.

`ResolvedWordSource::Var` "never reaches the view evaluator" (`view.rs:91-95`) — plan routes it to membership probes — but the type still has the arm, and `point_word` (`apply.rs:70-72`) panics. Proof discarded at plan, re-asserted here.

## Why it's wrong

Insight 6 — plan/validate learned the constant's shape and stored a universal value. Insight 7 — tag-plus-all-payloads of `Const`. Insight 2 — `Var` is a leftover coordinate of a different phase.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree. Per-kind payloads:

```rust
// DurationCompare.value: WordOrParam, not Const
// AnyPointIn.set: SetConst (ParamSet | WordSet)
// FieldAllen.other: IntervalConst (Interval | Param)
```

Drop `Var` from the view-level source (it lives on `PlanNode::point_probes` only). The unreachable arms delete.

## Acceptance criteria

- [ ] Gone: `rg -n 'unreachable!\("validated:' crates/bumbledb/src/image/view/apply.rs` → no Const-shape unreachables (interval-span asserts may remain as layout invariants).
- [ ] Gone: `rg -n 'ResolvedWordSource::Var' crates/bumbledb/src/image`.
- [ ] Unchanged tests: `image/view/tests.rs` and filter/interval/measure view tests green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Filter-order law and Ray-vs-Fails verdict algebra unchanged. Bind-time Param vs resolved Word stays; only the *type* of each site narrows. Plan-node `Var` membership probes are out of this issue's tree except as the destination of the dropped arm.
