## Interval-typed predicate columns: bench apparatus never constructs one; naive derived-typing code is dead weight; membership-through-Idb has no oracle

category: missing-free-feature | severity: medium | verdict: CONFIRMED | finder: r2:differential-apparatus-soundness
outcome: fixed bc9c698e

### Summary

The engine legally accepts interval-typed Idb head columns — the validation doc states "an interval-typed predicate column participates in point membership exactly as an interval field does" (`crates/bumbledb/src/ir/validate/context.rs:465-470`, citing `docs/architecture/20-query-ir.md` § engine recursion) — and the naive oracle implements a whole monotone-fixpoint derived-typing pass for them. But no program anywhere in the bench apparatus ever constructs one, so the naive model's subtlest typing code (`predicate_intervals`) always computes all-false, its Idb branch is unreachable-in-practice and untested, and one specific engine behavior — point membership of a scalar variable against an interval-typed Idb column, and Pack/ArgMax-carried interval head typing through a predicate — has no oracle anywhere in the repo.

One correction to the finding as filed: the engine's basic interval-through-Idb carriage is NOT oracle-free. The fixed engine test `typed_payload_propagates_through_the_recursive_accumulator` (`crates/bumbledb/tests/fixpoint_finalize_hunt.rs:485-607`) builds a `Program` whose predicate head carries `span: interval<u64>` at position 3, reads it back through an `AtomSource::Idb` atom across fixpoint rounds, and checks results against an in-test naive closure — covering 2-word slot carriage through rounds and the column-major finalize for the value-equality case. That downgrades severity from high to medium: the untested residue is the membership binding and aggregate-carried typing, plus the entire differential/randomized lane.

### Evidence (all verified against the working tree)

Naive oracle code that never runs meaningfully:
- `crates/bumbledb-bench/src/naive/query.rs:379-436` — `predicate_intervals`: the per-predicate, per-head-position interval-typing fixpoint; the Idb branch at ~394 (`AtomSource::Idb(pred) => interval[usize::from(pred.0)]...`) can only return `true` if some Edb interval column feeds a predicate head first — which never happens in any bench program. Called unconditionally at `query.rs:302`.
- `crates/bumbledb-bench/src/naive/query.rs:628-641` — `source_field_is_interval`'s Idb arm, the membership trigger read through predicate columns; same dead-in-practice status.

No producer anywhere in the apparatus:
- `crates/bumbledb-bench/src/querygen/shapes_recursive.rs` — the only Edb atoms in all six recursive variants are `ORG_PARENT(child, parent)` (lines 77-85) and `ORG(id)` (lines 179-180, 268-269), all u64; zero grep hits for `Interval` or `Mandate` in the file.
- `crates/bumbledb-bench/src/translate/program.rs:26-33` (module doc) and `:183` → `refuse_interval_columns` at `:254` — the translator refuses interval predicate columns and documents the limit as "generator-unreachable".
- `crates/bumbledb-bench/src/conformance/program.rs:282-287` — the corpus fence: `assert!(program_mentioned(program).iter().all(|relation| *relation == target::ids::ORG || *relation == target::ids::ORG_PARENT))`.
- `crates/bumbledb-bench/src/closure.rs` — the third program lane (deep chain / wide tree); zero interval mentions.
- `crates/bumbledb-bench/src/differential/tests/recursive.rs` — the only differential test file containing `Program`; zero interval mentions. `naive/tests/fixpoint.rs` — the only naive test calling `.program(...)`; zero interval mentions. So even the naive model's own unit tests never exercise `predicate_intervals` producing a `true`.

What IS covered (the correction):
- `crates/bumbledb/tests/fixpoint_finalize_hunt.rs:485-607` — interval-typed Idb head column (`span`) carried through a recursive accumulator across rounds, verified against an in-test `BTreeSet` naive closure over three repeat executions.

What is NOT covered anywhere (searched engine tests exhaustively):
- Point membership: a scalar variable bound at an interval-typed Idb head position (the exact rule `ir/validate/context.rs:465-470` documents). No engine test, no bench test, constructs this shape.
- `Pack`/`ArgMax`-carried interval head typing through a predicate (`predicate_intervals`' `FindTerm::Aggregate` arms): zero `Pack` hits in `fixpoint_finalize_hunt.rs`; `adversarial_ir.rs:643-680` injects hostile Idb reads but only asserts prepare returns Ok-or-typed-error, never execution results.

Doc cross-check: the chain-window fence (`docs/architecture/20-query-ir.md:173-196`) fences only CREATED interval heads (`w = w₁ ∩ w₂` — endpoint-inventing); BOUND interval variables projected through predicate heads sit inside the landed recursion surface. So the shape is engine-legal, exactly as the finding claims, and its absence from the differential is a coverage hole, not a fenced feature.

### Failure scenario

A defect in the engine's Idb membership probe against an interval-typed predicate column (e.g. evaluated as word equality instead of point membership in a second stratum) leaves every bench lane green: no generated program, no conformance case, no golden, no fixed differential test constructs the shape, and the deleted fuzzer no longer backstops it. Symmetrically, a bug in naive's `predicate_intervals` (whose Idb branch has never once returned `true` under test) silently corrupts the oracle for the first day someone writes such a program.

### Suggested fix

Add one differential case (fixed test in `differential/tests/recursive.rs`, or a seventh recursive shape fenced off the SQLite lane): a first predicate projecting an interval variable from an interval-bearing target relation (the Mandate `active: interval<u64>` column already exists in the bench target schema — `naive/tests/query.rs:21`, `conformance.rs:1277`), consumed by a second predicate through (a) value-equality of the interval variable and (b) a point variable bound at the interval-typed Idb position (membership). Engine-vs-naive; the translator's existing `refuse_interval_columns` already routes it away from SQLite as typed data, so the gate reporting stays honest. That single case brings `predicate_intervals`' Idb branch, `source_field_is_interval`'s Idb arm, and the engine's membership-through-Idb path under the differential for the first time.