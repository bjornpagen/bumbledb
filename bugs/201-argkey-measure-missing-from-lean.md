# ArgKey::Measure exists in Rust and docs, not in Lean AggOp
- id: 201
- severity: high
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: spec
- components: lean/Bumbledb/Query/Aggregates.lean, lean/Bumbledb/Conformance.lean, crates/bumbledb/src/ir.rs, docs/architecture/20-query-ir.md, crates/bumbledb-bench/src/conformance.rs
- status: open (do not fix)

## Summary
R5 admits two Arg-restriction keys: an orderable variable, or the interval measure (`ArgMax(w, Duration(w))`). Rust IR and the query-IR architecture doc implement both. Lean `AggOp.argMax` / `argMin` take only `VarId` keys. The conformance lane fences measure-keyed Arg cases until "the denotation lands," so the third oracle cannot check a shipped, documented operator.

## Lean spec
```2098:2107:lean/Bumbledb/Query/Aggregates.lean
inductive AggOp where
  | count
  ...
  | argMax (v k : VarId)
  | argMin (v k : VarId)
  | measureFold (op : ScalarFold) (v : VarId)
```

`argmax_ties_all_kept` orders keys as `Assignment → Nat` observers over variable values, not derived measures. Conformance decode (`Conformance.lean:397-400`) reads `"key"` as a `VarId` only. Measure *finds* and measure *folds* exist (`KeyTerm.measure`, `measureFold`); measure *Arg keys* do not.

## Normative docs
```607:609:docs/architecture/20-query-ir.md
**Arg-restriction key** (`ArgKey::Measure` — `ArgMax(w, Duration(w))`, "the
longest interval per group"; the restriction sweeps the derived measure
word, ray poisoning included — ruled 2026-07-23, R5);
```

Same exhaustive roster at `20-query-ir.md:339-343`. Rust query notation accepts `ArgMax(span, Duration(span))` (`crates/bumbledb-query/tests/notation.rs:483-488`).

## Rust implementation
```218:230:crates/bumbledb/src/ir.rs
/// An Arg-restriction key position — the two, exhaustively (ruled
/// 2026-07-23, R5): ... or the **interval measure** …
pub enum ArgKey {
    Var(VarId),
    Measure(VarId),
}
```

Engine tests: `api/prepared/tests/measure.rs` (measure-keyed Arg). Naive model evaluates `ArgKey::Measure` (`bumbledb-bench/src/naive/query.rs`). Conformance explicitly excludes the shape (`conformance.rs:143-147`, `excluded_measure_arg_key`).

## Why this matters
`ArgMax(w, Duration(w))` is a shipped query form with ray-poisoning semantics. Lean cannot state or prove its denotation; the checked-in corpus never includes it. A wrong engine reading of measure-key ties or ray groups would not fail the Lean conformance lane.

## Verification (2026-08-12)
Re-read `AggOp`, the query-IR roster, and the engine IR. **Confirmed.** `wrong-side: spec` is right: Rust and `20-query-ir.md` ship R5 measure keys; Lean `argMax` is `VarId`-only.

**Lean** (`lean/Bumbledb/Query/Aggregates.lean:2098-2107`): `| argMax (v k : VarId) | argMin (v k : VarId)`. Conformance decode (`lean/Bumbledb/Conformance.lean:397-400`) reads `"key"` as a `VarId` only. Measure *folds* exist (`measureFold`); measure Arg keys do not.

**Docs** (`docs/architecture/20-query-ir.md:339-343`, `:607-609`): exhaustive key positions include `ArgMax(w, Duration(w))` / `ArgKey::Measure`.

**Rust** (`crates/bumbledb/src/ir.rs:218-230`): `ArgKey::{Var, Measure}`. Conformance builder fences the shape (`crates/bumbledb-bench/src/conformance.rs:143-147`, `:1209-1210` `excluded_measure_arg_key`).

## Related
- 211 (TypeScript surface also cannot express `ArgKey::Measure`)
- 214 (other conformance fences)
