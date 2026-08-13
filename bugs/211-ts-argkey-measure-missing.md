# TypeScript ArgMax keys are variables only; Rust/docs/C++ admit Duration keys
- id: 211
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: split
- components: ts/src/query/find.ts, ts/src/native.ts, crates/bumbledb/src/ir.rs, docs/architecture/20-query-ir.md, cpp/bridge/src/query.rs, lean/Bumbledb/Query/Aggregates.lean
- status: open (do not fix)

## Summary
R5's exhaustive Arg-key roster includes `ArgKey::Measure`. The Rust IR, C++ bridge, and query-IR docs implement it. The TypeScript query surface types `argMax`/`argMin` keys as `AnyVar` only, and the TS IR mirror stores `key: number` (a var id) with no measure arm. Lean also lacks the denotation (201). TS hosts cannot write `ArgMax(w, Duration(w))` even as raw IR.

## Lean spec
Silent on a measure Arg key (`AggOp.argMax : VarId × VarId`). See 201.

## Normative docs
`20-query-ir.md:339-343`, `:607-609`: key positions are "a bound variable of orderable type … or the interval measure — `ArgMax(w, Duration(w))`." The TS cookbook and `75-cpp-lowering.md` inherit the IR shapes; C++ lowering is supposed to be field-for-field with TS (`75-cpp-lowering.md:12-20`, `:26-28`).

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
Cross-host cookbook recipes using "longest interval per group" type-check in Rust and C++ and are unrepresentable in TS. Byte-exact IR parity claimed by `75-cpp-lowering.md` fails for this operator. Combined with 201, only the Rust/C++ engine path actually implements the documented R5 roster.

## Related
- 201 (Lean denotation missing)
