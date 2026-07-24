## Program-form named params get dense ids by group-emission order, not source order — silent wrong bindings

category: bug | severity: high | verdict: CONFIRMED | finder: query:crates
outcome: fixed 95b73b26

### Summary

The `query!` macro documents named params as getting "dense ids by first occurrence, query-global" (crates/bumbledb-query-macros/src/lib.rs:99-100). In the program (named-head) form this is broken: rules are emitted bucketed by predicate group, and a named param's dense id is minted at its first occurrence in *emission* order, not source-text order. When rules of one predicate interleave around another group — legal input, the normative grammar is `program := rule+` with no contiguity requirement (docs/architecture/20-query-ir.md § the query notation) — the param ids are silently permuted. The host binds params positionally and the macro exposes no name-to-id map, so text order is the only contract a host can follow; a permutation between same-typed params is wrong results with no error.

### Evidence (verified by reading the code and by compiled reproduction)

- **Group bucketing:** `expand()` builds predicate groups in first-appearance order, explicitly merging non-contiguous members — crates/bumbledb-query-macros/src/lib.rs:1631-1637 (`groups.iter_mut().find(...)` pushes later same-name rules into the earlier group).
- **Group-order emission:** the program form emits `for (index, (_, members)) in groups.iter().enumerate()` calling `emitter.rule(&parsed[member])` per member — lib.rs:1677-1681. The all-bare form (lib.rs:1654-1663) iterates `parsed` in source order, which is why the degenerate case never exposes the bug.
- **Id minting at first resolve call:** `Params::resolve` — lib.rs:1066-1087 — appends the name to `self.named` on first sight and returns its position; "first sight" is emission order.
- **Reproduction** (temporary probe test, compiled against the workspace and since deleted):

```rust
query!(Org {
    reach(c, a) | Parent(child: c, parent: a);
    (c, a) | reach(c, a), c == ?root;          // ?root: first in SOURCE order
    reach(c, a) | Parent(child: c, parent: m), reach(m, a), a != ?skip;
});
```

Emitted IR (probe output):

```
pred 0 rule 1 conditions: [Leaf(Comparison { op: Ne, lhs: Var(VarId(2)), rhs: Param(ParamId(0)) })]  // ?skip
pred 1 rule 0 conditions: [Leaf(Comparison { op: Eq, lhs: Var(VarId(0)), rhs: Param(ParamId(1)) })]  // ?root
output: PredId(1)
```

`?skip` got id 0 and `?root` got id 1 — the reverse of text order.
- **No safety net:**
  - Validation checks param-id *density* only ("non-dense param ids ... are validation errors", docs/architecture/20-query-ir.md § Params); a permutation is dense. Param types are inferred from anchors, and both params here anchor u64 head positions, so type checks pass.
  - `PreparedQuery::bind_params` binds strictly by slice index — crates/bumbledb/src/api/prepared/bind.rs:56-66.
  - The macro's expansion emits only the `Program` value (lib.rs:1692-1695); per the doc, predicate and param names are a "macro-local sidecar" that never survives expansion — there is no name-to-id map for the host to consult.
- **No test coverage:** the only program-form corpus case with a param, `org-reach-rooted` (crates/bumbledb-query/tests/notation_corpus.rs:635-645), has contiguous groups and its single param in the source-first rule; no notation test interleaves groups with params. (The render round-trip goldens cannot catch this either: `render_program` prints `?{id}` positionally, so the permuted program is its own fixed point.)

### Failure scenario

A host writes the query above, reads its own source text, and calls `execute(&[root_value, skip_value])`. `root_value` lands in the `?skip` slot and vice versa. Both anchors are u64 org ids, so prepare and bind type checks pass; the query returns wrong rows with no error at any stage. This also violates the doc's own rationale for the density check — "a gap would be a positional slot whose supplied value is never type-checked" — since a same-typed permutation is exactly a supplied value that type-checks against the wrong slot.

### Suggested fix

Resolve params in source order: iterate `parsed` in source order, emitting each rule's string into a per-group buffer (the grouping is then pure bucketing of already-emitted strings), instead of iterating groups and re-walking members. The `PredId` assignment (group first-appearance order) is untouched — only the order in which `emitter.rule` (and hence `Params::resolve`) runs changes. Add an interleaved-groups-with-params corpus case (`idb-*` production) pinning the ids. This is also the representation-first fix: emission order stops being a second, divergent ordering — source order is the one ordering both `PredId`s (first appearance) and `ParamId`s (first occurrence) derive from.
