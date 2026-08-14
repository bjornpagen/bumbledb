# sdk-016: TS `isQueryValue` type-predicates on schema identity alone — validates then forgets

- **Severity:** medium
- **Tree:** sdk (ts)
- **Status:** FIXED(23482d0f)
- **Source:** audit/sdks.md #16
- **Depends on:** sdk-006 (the branded-parse story defines what the seam should be)

## The bug

`ts/src/query/lower.ts:1516-1521`:

```typescript
function isQueryValue<Rels extends SchemaRelations, Row, P extends ParamsRecord, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>,
	value: RawQuery
): value is RawQuery & Query<Rels, Row, P, Classes> {
	return value.schema === theory
}
```

The predicate promotes a `RawQuery` to a fully-typed `Query<Rels, Row, P, Classes>` while checking ONE fact: schema object identity. Interiors/rec/main well-formedness, head alignment, and param anchors were established in `makeRawQuery` and are not re-established here — any `RawQuery` sharing the schema object passes. `makeQuery` (`lower.ts:1524-1535`) then throws "query value construction incomplete" if the identity check fails, which cannot detect a malformed value.

## Why it's wrong

King's validator (Insight 6): the real proof is "this value was constructed by `makeRawQuery` just now" — held by the CALLER's control flow — and the type predicate substitutes a weaker checkable fact (schema identity) while claiming the full type. The proof exists; the seam throws it away and asserts a stronger claim than it verified.

## The fix

Per `audit/CONTRACT.md §C6` (TS): don't type-predicate.

- `makeQuery` returns `Query` because it CONSTRUCTED one — the constructor's result type is the proof. `makeRawQuery`'s return flows to the typed result by construction (an internal cast at the single mint site, or better: build the typed shape directly), and `isQueryValue` + the unreachable throw delete.
- If an admission seam is genuinely needed (values crossing from untyped storage), it is sdk-006's `parseQueryIr`-style structural parse into the unforgeable brand — schema identity may be one of its checks, never the whole.

## Acceptance criteria

- [ ] Gone: `rg -n 'isQueryValue|construction incomplete' ts/src/query/lower.ts` → no matches.
- [ ] Unchanged tests: `cd ts && pnpm test` green with zero assertion edits; lowered output identical.
- [ ] Green: `cd ts && pnpm test`; `cd ts && pnpm run build`.

## Constraints

- Semantics identical. Coordinate with sdk-005/006 (same file); land after sdk-006 settles the brand.
