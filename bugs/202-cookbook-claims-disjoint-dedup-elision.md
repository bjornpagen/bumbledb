# Cookbook claims executor elides cross-rule dedup; execution never does
- id: 202
- severity: high
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: docs
- components: docs/cookbook.md, ts/COOKBOOK.md, docs/architecture/40-execution.md, lean/Bumbledb/Exec/Dedup.lean, lean/Bumbledb/Bridge.lean, crates/bumbledb/src/api/prepared/build.rs
- status: open (do not fix)

## Summary
Cookbook recipe 22's Guarantee cites `disjoint_witness_licence` and then states that provably disjoint DU arms let "the executor elide cross-rule dedup — the free lunch." Lean, Bridge, and `40-execution.md` all record that the witness is spent diagnostically only: after a measured refutation, every multi-rule program keeps a spanning seen-set. The cookbook (and the TS cookbook twin) assert the optimization the engine deleted.

## Lean spec
`disjoint_witness_licence` proves what the witness *could* license. The module doc states the engine does not spend it:

```570:576:lean/Bumbledb/Exec/Dedup.lean
The
engine SPENDS this witness diagnostically only — plan introspection's
`disjoint_rules: proven (R.f)` line — and keeps the spanning
head-projection seen-set regardless: the measured cross-rule elision
refutation … rejected the per-rule-drain
representation on the clock
```

Bridge row (`Bridge.lean:422-425`): "proved sound, and spent diagnostically only (the measured refutation keeps the spanning seen-set)."

## Normative docs
Cookbook (`docs/cookbook.md:1018-1047`), despite architecture README calling it "illustrative, never normative," carries census-checked `Guarantee:` labels:

```1044:1047:docs/cookbook.md
One query, two rules (set union). The exclusivity theorem (recipe 2) is
spent a third time here: rules selecting different `kind` values are
provably disjoint, so the executor elides cross-rule dedup — the free lunch
(`40-execution.md` § set semantics):
```

The cited section says the opposite (`40-execution.md:326-330`): "Plan introspection retains the knowledge as `disjoint_rules: proven (R.f)`, but execution always keeps one seen-set spanning a multi-rule program." Same false sentence in `ts/COOKBOOK.md:1098-1100`.

## Rust implementation
`api/prepared/build.rs:136-137`: "A single-rule aggregate may elide its seen-set under the plan's distinct-bindings proof. Every multi-rule sink keeps one seen-set spanning all rules." `DisjointWitness` is introspection-only.

## Why this matters
A host or agent following the cookbook Guarantee will expect no cross-rule dedup cost and may write unions that depend on elision for latency. The engine always probes the spanning map. The Guarantee label is a false theorem citation.

## Related
- `docs/architecture/40-execution.md` § set semantics (the measured refutation)
