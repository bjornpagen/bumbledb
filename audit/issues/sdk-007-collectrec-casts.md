# sdk-007: `collectRec` constructs an illegal empty `RecData` and backfills through readonly-casts

- **Severity:** high
- **Tree:** sdk (ts)
- **Status:** OPEN
- **Source:** audit/sdks.md #7
- **Depends on:** none (localized; textual overlap with sdk-005 in lower.ts)

## The bug

`ts/src/query/lower.ts:1572-1595`:

```typescript
const recData: RecData = {
    name,
    finds: Object.freeze([]),
    base: Object.freeze([]),
    rec: Object.freeze([])
}
const env: DerivedEnv = { interiors, rec: recData }
// ... build base arms against env ...
;(recData as { finds: readonly FindColumn[] }).finds = first.finds
// ... build rec arms ...
;(recData as { base: readonly RuleData[] }).base = Object.freeze(base)
;(recData as { rec: readonly RuleData[] }).rec = Object.freeze(rec)
Object.freeze(recData)
```

The type says sealed nonempty rec; construction inhabits the empty triple, then lies to the type system three times to backfill. Rule scopes built against `env.rec = recData` observe empty `finds` during the base walk.

## Why it's wrong

Parse-don't-validate inverted (Insight 6): the finished value is supposed to BE the proof, but the path to it passes through exactly the illegal state (`RecData` with empty base/rec) the type was written to forbid — reachable by any consumer holding `env` during construction. The casts are the type system being overruled where it was right.

## The fix

Per `audit/CONTRACT.md §C6` (TS): build arrays first, seal once.

- Introduce the two-phase TYPE the circularity actually needs: a `RecEnv`/`RecHandle` (name + deferred head resolution) that rule scopes accept during arm building — `makeRecRuleScope` takes the handle, not a `RecData`.
- Then `const recData: RecData = Object.freeze({ name, finds, base: Object.freeze(base), rec: Object.freeze(rec) })` in ONE assignment. No `as`-casts, no mutation of a frozen-typed value.
- The essential circularity (rec arms resolve the rec's own head) lives in the handle, which the sealed `RecData` replaces at the end.

## Acceptance criteria

- [ ] Gone: `rg -n 'recData as \{' ts/src/query/lower.ts` → no matches; `rg -n 'as \{ (finds|base|rec):' ts/src` → no matches.
- [ ] Unchanged tests: `cd ts && pnpm test` green with zero assertion edits; lowered `QueryIr` for all corpus/builder cases byte-identical.
- [ ] New lock: a unit test asserting arm scopes never observe empty rec `finds` (or simply that the sealed value equals the previous output — snapshot one recursive query's lowered IR).
- [ ] Green: `cd ts && pnpm test`; `cd ts && pnpm run build`.

## Constraints

- Semantics identical; error messages (`has no base arms`, `has no rec arms`) unchanged. Coordinate textually with sdk-005 (same file).
