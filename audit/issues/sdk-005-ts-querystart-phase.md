# sdk-005: TS `QueryStart` does not carry phase — second-rec / interior-after-rec are runtime throws

- **Severity:** high
- **Tree:** sdk (ts)
- **Status:** OPEN
- **Source:** audit/sdks.md #5
- **Depends on:** none (foundation for the ts tree; sdk-007 and sdk-018's TS half land with/after it)

## The bug

`ts/src/query/lower.ts` — after `.rule()`, `.interior`/`.recursive` are typed `never` (the `Query` type, `lower.ts:344-347`): the dialect KNOWS the move. But after `.recursive()`, the return type is still the same `QueryStart` with both methods live; the wall is a runtime throw:

```typescript
// lower.ts:1614-1617
if (rec !== null) {
    throw errors.new(
        "query: interior after recursive is unwritable — declaration order is interiors, then rec, then main"
    )
}
// lower.ts:1635-1637
if (rec !== null) {
    throw errors.new("query: a second recursive is unwritable — this cut admits one rec SCC")
}
```

The message says "unwritable"; the type says writable. The test pins the throw (`ts/test/query.test.ts:1161-1193`, `assert.throws(..., /second recursive/)`).

## Why it's wrong

Declaration order is data, and they encoded it for main and stopped (Insight 2: the rec phase kept the Program's runtime-check coordinate). A flowchart (`if (rec !== null) throw`) on a type that admits the call is validation whose proof is discarded (Insight 6); TypeScript can carry the phase for free.

## The fix

Per `audit/CONTRACT.md §C6` (TS): `QueryStart<Rels, Classes, P, Rec extends RecData | null = null>`.

- `interior` and `recursive` exist only when `Rec extends null` (conditional method types — same mechanism as the existing after-main `never`).
- `.recursive(...)` returns `QueryStart<Rels, Classes, P', RecData>`; a second call and `interior`-after-rec are `never` at compile time.
- The runtime throws MAY remain as defense against untyped callers (JS), but the typed surface can no longer spell the calls; the tests that pinned the throws become `// @ts-expect-error` type tests plus (optionally) the untyped-caller runtime pins.
- `makeQueryStart` (`lower.ts:1599+`) threads the phase parameter.

## Acceptance criteria

- [ ] Unrepresentable, compile-checked: new type-level tests (suggested `ts/test/query-phase.test-d.ts` or `@ts-expect-error` blocks in the existing suite) — `q.recursive(...).recursive(...)`, `q.recursive(...).interior(...)` fail type-check; `tsc --noEmit` over the test file is the gate.
- [ ] Unchanged tests: `cd ts && pnpm test` green; runtime behavior for legal call sequences identical (same lowered `QueryIr`).
- [ ] Existing throw-tests either preserved (untyped-caller path) or converted to type tests — NOT deleted silently.
- [ ] Green: `cd ts && pnpm test`; `cd ts && pnpm run build` (tsc clean).

## Constraints

- Semantics identical. Surviving runtime walls (untyped/JS callers) keep their *meaning*; the SCC wording in `lower.ts:1636` ("this cut admits one rec SCC") is sdk-022's vocabulary fix — this issue must not pin that substring. No Program vocabulary. Coordinate with sdk-018 (this IS its TS half) and sdk-006 (same file, land per INDEX order).
