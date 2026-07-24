## Ordinal-alignment law re-verified on every prepare instead of once at open

category: unification | severity: low | verdict: CONFIRMED | finder: ts:core
outcome: fixed c31416e1

### Summary

The SDK's declaration-order-equals-engine-id law is a construction-time invariant — every input to it (the frozen schema value, the manifest reported at open, the id-resolution tables built from both) is fixed when `openDb` returns. Yet the function that verifies it, `assertOrdinalAlignment`, lives inside `prepare()` and runs on every prepare call, while the SDK's designated construction-time drift verifier, `tablesOf`, walks the identical theory/manifest pair once at open and checks everything *except* this law. One invariant, two verification walks, two homes, wrong time for one of them.

### Evidence (all verified against the working tree)

- **ts/src/db.ts:1363-1386** — `assertOrdinalAlignment()`: walks `Object.keys(theory.relations)` with a per-relation closure, checks `entry.id !== ordinal`, then maps `sealedFieldsOf(member)` to a fresh name array and walks it with a per-field closure checking `entry.fieldIds.get(fieldName) !== fieldOrdinal`. Its own doc comment (db.ts:1360) says "Any drift is a construction-time failure here" — the comment knows where the invariant lives; the call site doesn't.
- **ts/src/db.ts:1394** — `assertOrdinalAlignment()` is the second statement of `prepare()`, unconditional, and (grep-confirmed) this is the function's only call site.
- **ts/src/db.ts:574-613** — `tablesOf(theory, manifest)`: the existing construction-time drift walk. It verifies statement count, `statement.id === index`, statement kind, and relation membership in both directions — but stores `relation.id` (line 605) without ever checking it equals the declaration ordinal, and stores `fieldIds` (lines 595-598) without checking them against sealed ordinals.
- **ts/src/db.ts:746** — `tablesOf` runs exactly once, in `openDb`; `tables` and `theory` are closed over for the store's lifetime. The schema value is `Object.freeze`'d at construction (ts/src/schema.ts:318), and `prepare` already refuses any query built against a different schema value (db.ts:1389-1393), so the ordinal walk's inputs cannot change between prepares — after one success it can never fail again.
- **ts/src/query/lower.ts:1855-1860** — the invariant is real and load-bearing: `lowerQuery` mints relation ids from `Object.keys(theory.relations)` declaration order, independent of the manifest. So the law must be checked — once, where the manifest is admitted.
- No test in ts/test exercises a prepare-time ordinal-drift throw (nothing could — the inputs are frozen), so folding the check into `tablesOf` breaks nothing.

This is a representation-first violation in the doctrine's sense (docs/design/representation-first.md): a guard re-run per call where the construction of `Tables` — the representation — should already embody the guarantee. Once `tablesOf` refuses any manifest whose ids diverge from declaration order, a constructed `Tables` *is* the proof, and `prepare` inherits it structurally instead of re-deriving it by control flow.

### Bench impact

Not a correctness issue. Prepare-heavy hosts pay a redundant O(relations × fields) walk per `prepare`, including hidden allocations (`sealed.map` builds a fresh string array per relation; `forEach` callbacks are per-call closures) — small in absolute terms but pure waste, and against the repo's allocation doctrine for repeated paths.

### Suggested fix

Move both assertions into `tablesOf`'s existing walks: in the final `Object.keys(theory.relations)` loop (db.ts:607-611), check the stored `entry.id` against the key's declaration index; when building `fieldIds` (db.ts:595-598), check each manifest field id against the member's sealed ordinal (via `sealedFieldsOf`). Delete `assertOrdinalAlignment` and its call at db.ts:1394. The error messages carry over verbatim — they already speak in construction-time terms.
