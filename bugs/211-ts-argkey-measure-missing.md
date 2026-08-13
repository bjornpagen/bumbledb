# TypeScript ArgMax keys are variables only; Rust/docs/C++ admit Duration keys
- id: 211
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: split
- components: ts/src/query/find.ts, ts/src/native.ts, crates/bumbledb/src/ir.rs, docs/architecture/20-query-ir.md, docs/architecture/75-cpp-lowering.md, cpp/bridge/src/query.rs, cpp/src/query/aggregate.cc, lean/Bumbledb/Query/Aggregates.lean
- status: closed (obsolete) — ArgMax/ArgMin killed; do not resurrect

## Summary
R5's exhaustive Arg-key roster includes `ArgKey::Measure`. The Rust IR, C++ dialect, C++ bridge, and query-IR docs implement it. The TypeScript query surface types `argMax`/`argMin` keys as `AnyVar` only, and the TS IR mirror stores `key: number` with no measure arm. Lean also lacks the denotation (201). TS hosts cannot write `ArgMax(w, Duration(w))` even as raw IR. `75-cpp-lowering.md` schema field-for-field is about `SchemaSpec`, not this operator; its query section lists engine `ArgKey::Measure` and TS `key: number` side by side and still tells C++ to reproduce `lower.ts` exactly.

## Lean spec
Silent on a measure Arg key (`AggOp.argMax : VarId × VarId`). See 201.

## Normative docs
`20-query-ir.md:339-343`, `:607-609`: key positions are "a bound variable of orderable type … or the interval measure — `ArgMax(w, Duration(w))`." `75-cpp-lowering.md:407-409` lists engine `ArgKey = Var | Measure`; `:437-438` records TS `AggOpIr` as `argMax`/`argMin` carry `key: number`; `:446-496` tells C++ to reproduce `ts/src/query/lower.ts` (`argMax/argMin(over, key)` → `keyVarId` only). Schema fingerprint parity (`:1-20`) is a different claim.

## Rust implementation
`ir.rs:227-230` `ArgKey::{Var, Measure}`. C++ bridge maps `bdb_arg_key_kind::Measure` (`cpp/bridge/src/query.rs:280`). Rust `query!` accepts `ArgMax(span, Duration(span))`.

TS:

```100:101:ts/src/query/find.ts
function argMax<const V extends AnyVar, const K extends AnyVar>(value: V, key: K): Agg<"argMax", V, K> {
	return aggregate("argMax", value, key)
```

```118:121:ts/src/native.ts
	| { readonly kind: "argMax"; readonly key: number }
	| { readonly kind: "argMin"; readonly key: number }
```

No `{ kind: "measure", var }` on the Arg key. Contrast `FindTermIr`, which does have a measure find arm (`native.ts:106-110`).

## Why this matters
Cross-host recipes using "longest interval per group" type-check in Rust and C++ and are unrepresentable in TS. Combined with 201, only the Rust/C++ engine path implements the documented R5 roster. C++ did *not* reproduce the TS hole — it implemented `is_measure_ref` keys (`cpp/src/query/aggregate.cc:174-178`).

## Verification (2026-08-12)
Re-read TS builders/IR, C++ `arg_max`, engine `ArgKey`, and both docs. **Confirmed**, rewritten: the schema field-for-field citation does not cover Arg keys. `wrong-side: split` (TS vs Rust/C++/20-query-ir). Lean silent on the measure key (201).

**Lean:** Silent. `AggOp.argMax : VarId × VarId` (201).

**Docs:** `20-query-ir.md:339-343`, `:607-609` admit `ArgKey::Measure`. `75-cpp-lowering.md:407-409` vs `:437-438` documents the engine/TS split without calling it a hole.

**Rust / C++ / TS:** `crates/bumbledb/src/ir.rs:227-230`; `cpp/bridge/src/query.rs:279-280`; `cpp/src/query/aggregate.cc:174-178` (`is_orderable_var || is_measure_ref`). TS `ts/src/query/find.ts:100-101` (`K extends AnyVar`); `ts/src/native.ts:118-121` (`key: number` only). `FindTermIr` *does* have a measure find arm (`native.ts:106-110`).

## Related
- 201 (Lean denotation missing)

## Resolution (2026-08-13)

Obsolete: ArgMax/ArgMin (including measure-keyed R5) were killed. Remaining folds: Count, Sum, Min, Max, Pack. Do not resurrect. Hosts that want "the row at max(key)" compose Max then keyed get.
