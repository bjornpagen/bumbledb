## The measure law (Weight.apply / Bound.resolve) has two engine definitions: validate's closed-constant arm re-implements judgment.rs's child_weight and resolve_hi inline

unification | medium | CONFIRMED | capacity-surface
outcome: fixed d831280a

### Summary

`lean/Bumbledb/Capacity.lean` defines the measure law once — `Weight.apply` (Capacity.lean:460) and `Bound.resolve` (Capacity.lean:469), under § "Syntax resolved against rows" (Capacity.lean:61). The engine defines it twice. The sealed copy lives in `crates/bumbledb/src/storage/commit/judgment.rs`: `child_weight` (judgment.rs:116-140) and `resolve_hi` (judgment.rs:1187-1212), serving the commit judge, the sweeper, and the applier's slot law (`expected_slot_weight`, judgment.rs:147-156; `plan.rs:449`). The second copy is inlined in `validate_capacity`'s closed-constant decidability arm (`crates/bumbledb/src/schema/validate.rs`): per-parent bound resolution at validate.rs:946-962 and per-child weight application at validate.rs:979-996 restate the same two matches byte-for-byte — minus one arm.

### Evidence (verified)

- judgment.rs routes every Duration measure through `interval_measure` (judgment.rs:164-182), whose `end == u64::MAX` arm is the typed C10 ray refusal naming the row (judgment.rs:175-180).
- The validate copy computes `end - start` bare under an `expect` at validate.rs:960 (bound) and validate.rs:994 (weight). The expects cover only malformed bytes ("sealed rows hold canonical interval bytes") — a ray IS canonical bytes with `end == u64::MAX`, so nothing panics: the copy would silently produce `u64::MAX - start` where judgment.rs produces `Error::CapacityRayMeasure`.
- The only thing making that unreachable today is `validate_extension`'s `ExtensionIntervalRay` refusal (validate.rs:1753-1764) — enforced in a different function, with no cross-reference from the closed-constant arm; the arm's safety rests on a guarantee it never names.
- The ray law is expected to move: judgment.rs:103-106 records an OWNER RULING OWED on the write-time-vs-judge-time ray corner (the C17 slot refuses rays at write time, strictly stronger than C10's judge-time refusal). When that ruling lands, it needs a single landing spot; today it has two.
- A tree-wide grep for other copies of the measure law finds none — every other `end - start` (exec/sink.rs:164, exec/verdict.rs:393, exec/run/probe_pass.rs:185, run_node.rs:370, image/view/apply.rs:416,459) carries an inline `end != u64::MAX` guard. The validate closed-constant arm is the lone unguarded restatement.
- The suggested unification is type-coherent: judgment.rs's `CapacityBound` is theory `Bound` renamed (judgment.rs:48, to dodge `std::ops::Bound`), so both sites match on identical `Weight`/`Bound` types, and both hold the helper's full inputs — layout, fact bytes, sealed `IntervalTail`s (validate seals `weight_tail`/`bound_tail` at validate.rs:854-913 and both sites read them).

### Failure scenario / impact

No wrong answer today. The hazard is drift: the next change to the measure law — the owed C10 ray ruling, an i64-interval encoding tweak, an R6-precedent widening — lands at the judgment site and misses the validate site (or vice versa), and closed-constant theories are then admitted or refuted under a different measure than commits enforce. That is a validate-vs-commit wall break invisible to any single-site test, and the specific ray divergence is already written: lifting the extension ray refusal (validate.rs:1753-1764) without touching the closed-constant arm silently turns a typed refusal into a `u64::MAX - start` measure. Representation-first doctrine (docs/design/representation-first.md) and the file's own "no judge re-walks the field rosters" sealing framing both argue the law should have one body.

### Suggested fix

Hoist weight application and bound resolution into schema-layer helpers taking (weight-or-bound, sealed tail, layout, fact bytes) — everything both sites already hold — returning the measure or a shared ray-refusal signal each caller maps to its own error type (`Error::CapacityRayMeasure` at the judge, a `StatementErrorKind` at validate). Make `child_weight`/`resolve_hi` and the closed-constant arm both call them, so `interval_measure`'s ray arm is the one place the C10 ruling lands.