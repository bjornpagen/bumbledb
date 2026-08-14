# image-002: `Const` is a universal value; `ResolvedWordSource::Var` is inhabited then `unreachable!`

- **Severity:** medium
- **Tree:** image
- **Status:** OPEN
- **Source:** audit/storage-schema.md F13
- **Depends on:** none
- **Conflicts with:** none

## The bug

`FilterPredicate` (`image/view.rs:112-209`) is a good kind-sum. Its payloads then carry `Const`, which admits Word/Byte/Words/Interval/Param/ParamSet/WordSet/PendingIntern at every site. `FieldAllen.other`, `DurationCompare.value`, `AnyPointIn.set` each legally hold the wrong arm. `image/view/apply.rs` is a forest of `unreachable!("validated: …")` (lines 54, 68, 80, 122, 128, 177, 183, 214, 225, 233, 358, 373, 427, 578, 613).

`ResolvedWordSource::Var` "never reaches the view evaluator" (`view.rs:84-94`) — plan routes it to membership probes — but the type still has the arm, and `point_word` (`apply.rs:70-72`) panics. Proof discarded at plan, re-asserted here.

`ResolvedWordSource` is **shared** with plan/exec: `plan/fj/validate.rs`, `plan/ground.rs`, `ir/normalize/normalize.rs`, `exec/dispatch/{classify,key_probe_fact}.rs`, `api/prepared/bind.rs`, plus tests. Deleting `Var` from the shared enum in place would break those trees.

## Why it's wrong

Insight 6 — plan/validate learned the constant's shape and stored a universal value. Insight 7 — tag-plus-all-payloads of `Const`. Insight 2 — `Var` is a leftover coordinate of a different phase, living on a type the view evaluator also matches.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree. Per-kind payloads on `FilterPredicate`:

```rust
// DurationCompare.value: WordOrParam, not Const
// AnyPointIn.set: SetConst (ParamSet | WordSet)
// FieldAllen.other / FieldWithin.outer: IntervalConst (Interval | Param)
```

**Split the word source.** Do **not** delete `Var` from the shared enum:

```rust
// view evaluator only
enum ViewWordSource { Word(u64), Param(ParamId) }

// plan / exec membership probes keep Var
enum ResolvedWordSource { Word(u64), Param(ParamId), Var(VarId) }
```

`PointIn.point` on a view-level filter is `ViewWordSource`. Plan-node `point_probes` keep `ResolvedWordSource::Var`. `Const` stays the param-slice universal value (bind-time Param vs resolved Word is essential); only each *filter site* narrows.

## Acceptance criteria

- [ ] Gone: `rg -n 'unreachable!\("validated:' crates/bumbledb/src/image/view/apply.rs` → no Const-shape unreachables (interval-span asserts may remain as layout invariants).
- [ ] Gone: `rg -n 'ResolvedWordSource::Var' crates/bumbledb/src/image` — view code sees `ViewWordSource` only. Plan/exec still have `Var`.
- [ ] Unchanged tests: `image/view/tests.rs` and filter/interval/measure view tests green. Plan/exec Var-probe tests untouched.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Filter-order law and Ray-vs-Fails verdict algebra unchanged. Bind-time Param vs resolved Word stays; only the *type* of each site narrows.
- Plan-node `Var` membership probes stay inhabited. A fanout that `rg`-deletes `ResolvedWordSource::Var` across `crates/bumbledb/src` has broken classify/bind/key-probe.
