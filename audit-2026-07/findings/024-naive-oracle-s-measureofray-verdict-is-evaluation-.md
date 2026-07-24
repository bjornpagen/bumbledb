## Naive oracle's MeasureOfRay verdict is evaluation-order-dependent; diverges from the engine's DNF lowering

category: bug | severity: high | verdict: CONFIRMED | finder: r2:differential-apparatus-soundness

### Summary

The naive model — the differential lane's definitional oracle — decides `MeasureOfRay` via a `Cell<bool>` side effect inside short-circuiting boolean tree evaluation. A measure leaf poisons the rule only if the evaluator happens to *reach* it, so the oracle's error verdict depends on the order of a condition tree's children — an order the engine's own IR declares semantically irrelevant (`collapse` compares condition lists "as **sets** — order- and multiplicity-insensitive", citing the Lean lemma `ruleAnswers_conditions_congr`, `crates/bumbledb/src/ir/normalize/dnf.rs:127-139`). Consequently the naive tree evaluation is inequivalent to the engine's DNF lowering for measure leaves under `Or`: the lowered measure-only disjunct rule evaluates the measure on every binding and raises, while the naive `any` short-circuits past it whenever an earlier child holds. The module header claims "the model never distributes to DNF; the engine's lowering is proven *against* this evaluation" (`naive/query.rs:9-12`), but every generator that exercises trees uses scalar-only leaves, so the incoherence is silent today. **Reproduced executably** with a temporary differential test (since reverted).

### Evidence (all verified against the code)

Naive side — the poison is reach-dependent:
- `crates/bumbledb-bench/src/naive/query.rs:991-997` — `tree_holds`: `And` via `iter().all(...)`, `Or` via `iter().any(...)` — both short-circuit.
- `crates/bumbledb-bench/src/naive/query.rs:929-933` — `leaf_admits` returns on the first failing conjoined tree, skipping later trees entirely.
- `crates/bumbledb-bench/src/naive/query.rs:1019-1030` — a `Substituted::Measure` leaf calls `measure_value`; on a ray it does `ray.set(true)` — only if evaluation reached this leaf.
- `crates/bumbledb-bench/src/naive/query.rs:485-487` — `rule_bindings` returns `Err(QueryError::MeasureOfRay)` iff the flag was set.

Engine side — order-insensitive, measure disjunct always evaluated:
- `crates/bumbledb/src/ir/validate/validate.rs:393` — `collapse(rules.iter().flat_map(distribute).collect())`: every rule's condition trees distribute to DNF, one lowered rule per disjunct (`crates/bumbledb/src/ir/normalize/dnf.rs:89-126`).
- `crates/bumbledb/src/exec/run.rs:591-597` — the `measure_of_ray` poison flag: "A measure residual reached a ray … `execute` raises the typed `Error::MeasureOfRay`".
- `docs/architecture/20-query-ir.md` § "The measure", "The ray error" + "The filter-order law" (lines 584-601): the subtraction tests `end == MAX` and raises; only the *same atom's other filters* run before the subtraction. A DNF-lowered rule whose sole condition is the measure comparison has no other filters, so every ray binding reaches the subtraction. (I checked this doc as the normative spec; it pins the engine's evaluation order but nowhere sanctions child-list-order short-circuiting as the error semantics.)
- Validation permits measure leaves anywhere in a tree: leaves are shaped per-comparison (`OrdMeasureVar`/`OrdMeasureConst`, `crates/bumbledb/src/ir/validate/context.rs:750-820`) with no tree-position restriction — the diverging query class is legal.

Coverage gap — nothing quantifies the equivalence over measure leaves:
- `crates/bumbledb-bench/src/querygen.rs:175-186` — "The generator's queries carry flat conjunctions … The tree grammar's OR shapes are the DNF property suite's territory (`naive/tests/dnf.rs`), never the generator's."
- `crates/bumbledb-bench/src/naive/tests/dnf.rs:64-86` and `crates/bumbledb-bench/src/verify/run_algebra.rs:186-205` — both DNF tree generators draw leaves from scalar `Eq/Ne/Lt/Le/Gt/Ge` against `u64`/`i64` literals only; no `Term::Measure`.
- `crates/bumbledb-bench/src/querygen/shapes_interval.rs:464-472` — the querygen measure shape runs over a lane where "no window is a ray" by construction; its ray-parity rows (`run_algebra.rs:298-336`) are flat single-condition queries.

### Reproduction (executed, then reverted)

One `STAY` fact `(room=3, span=[3,∞), cap=0)`; query finds `room` with conditions `[Or([room < 5, Duration(span) < 10])]`. The differential runner (`differential.rs:130-143` / `:279`) reported:

```
ORDER-A DIVERGED: Query { op: 1, engine: MeasureOfRay, naive: Ok({Tuple([U64(3)])}) }
```

With the `Or`'s children swapped (measure child first): `ORDER-B AGREED` — both sides `MeasureOfRay`. The naive verdict flips between `Ok` and `Err` on child order for the identical denotation; the engine's verdict is stable. The oracle therefore has no well-defined verdict to arbitrate with for this legal query class.

Note the hole is wider than `Or`: `leaf_admits`'s early return (`naive/query.rs:929-933`) makes a plain conjunction `[Duration(w)<10, x<5]` poison on a ray the engine would never measure (the filter-order law runs `x<5` first), while the reversed order agrees — same defect, no `Or` required.

### Bench impact

Today: none fires — the divergence panics only if a generator or hand-written case emits a measure leaf inside a tree over ray-bearing data, and none does. That is exactly the problem: the DNF-lowering equivalence the naive module claims to prove is quantified over a leaf vocabulary that excludes the IR's one partial predicate, and any future extension of the tree generators (or a hand case) produces spurious divergences with no principled arbiter.

### Suggested fix

Make the error semantics order-independent by representation, per the repo's own doctrine (reify control flow as data; no side-effecting flag threaded through short-circuit `bool` operators): define in `20-query-ir.md` and the model together that a binding raises iff the order-insensitive denotation requires the measure of a ray — evaluate leaves three-valued (`Holds`/`Fails`/`Ray`) and fold `And`/`Or` in the corresponding Kleene-style lattice aligned with the DNF semantics (a disjunct-set reading: raise iff some DNF disjunct containing the ray measure has all its other conjuncts satisfied), replacing the `Cell<bool>` poison. Then add `Term::Measure` leaves to the `dnf_ops`/`naive/tests/dnf.rs` tree generators (with ray-bearing rows) so the lowering equivalence is actually quantified over the partial predicate.
