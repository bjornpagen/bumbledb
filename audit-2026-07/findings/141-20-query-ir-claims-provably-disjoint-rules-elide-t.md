## 20-query-ir claims provably disjoint rules elide the spanning seen-set; every other authority records the opposite

category: incoherence | severity: low | verdict: CONFIRMED | finder: lean:query

### Summary

The aggregation bullet in the normative query-IR doc says, of the executor's spanning head-projection seen-set: "provably disjoint rules elide it, § set semantics" (docs/architecture/20-query-ir.md:287-288). The section it cites says the opposite, backed by a measured refutation; the Lean dedup record says the opposite; and the engine has no elision arm for the multi-rule union regime. The sentence asserts an optimization the project measured (~32% slower), deleted, and deliberately does not perform.

### Evidence (all verified against the working tree)

- **The stale claim** — docs/architecture/20-query-ir.md:287-288: "the executor's spanning seen-set keys exactly that head projection — `40-execution.md` § the rule loop; provably disjoint rules elide it, § set semantics".
- **The cross-reference target contradicts it** — docs/architecture/40-execution.md:286-287: "execution always keeps one head-projection seen-set spanning a multi-rule program". Lines 275-278 restrict seen-set elision to "Single-rule only: the multi-rule union keeps its spanning head-projection seen-set even when every rule has its own witness — deliberately distinct from the measured cross-rule elision refutation below."
- **The measured refutation** — docs/architecture/40-execution.md:289-300, "Refutation — cross-rule dedup removal": three pre-isolation scale-S runs 32.1%/32.6%/32.4% slower, isolated repro at commit `39f6bee` (−31.9%), root-caused to extra O(n) per-rule drain/copy passes; "It was deleted."
- **The Lean record agrees with 40-execution** — lean/Bumbledb/Exec/Dedup.lean:61-71: `DisjointWitness` is minted by `provably_disjoint_rules` and spent "**diagnostically only** — plan introspection renders `disjoint_rules: proven (R.f)`, but execution always keeps the one spanning head-projection seen-set", explicitly citing the 40-execution refutation as the doc-side authority. `disjoint_witness_licence` proves only what the witness *could* license.
- **The engine has no elision arm for the union regime** — crates/bumbledb/src/exec/sink/aggregate/new.rs:131-132: `for_union` constructs with `DedupRegime::Union`; in `build` (lines 147-151), `DedupRegime::Bindings | DedupRegime::Union => None` for the distinct witness, and line 227 (`seen: distinct_witness.is_none().then(...)`) therefore builds the seen-set unconditionally for Union. Only `without_seen_set` (line 138, `DedupRegime::Elided(DistinctWitness)`) omits it, and it takes the single-rule `DistinctWitness`, not `DisjointWitness`.
- **Routing** — crates/bumbledb/src/api/prepared/build.rs:1043 sends every `SinkProgram::Union` to `AggregateSink::for_union`; `DisjointWitness` in that file (lines 83, 118, 282, 411) only populates the `disjoint_rules` introspection field. No code path spends it on sink construction.

### Failure scenario

A reader (or tool) trued against the normative IR doc expects `disjoint_rules: proven` to remove the union seen-set and reasons about allocation, dedup timing, or `distinct_seen()` observability accordingly — the engine never does this, and the doc's own cross-reference target explains, with numbers, why it must not. Worse, the stale sentence invites re-implementing a representation the project already measured at a ~32% loss and deleted.

### Suggested fix

Invert the clause in docs/architecture/20-query-ir.md:287-288 to match 40-execution.md and Dedup.lean, e.g.: "the executor's spanning seen-set keys exactly that head projection — `40-execution.md` § the rule loop; provable rule-disjointness is recorded diagnostically only (`disjoint_rules: proven`) — execution always keeps the spanning seen-set, § set semantics (the measured cross-rule elision refutation)."
