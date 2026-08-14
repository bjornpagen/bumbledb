# sdk-006: public `QueryIr` is Program-shaped and `lowerQuery` is a second builder

- **Severity:** high
- **Tree:** sdk (ts)
- **Status:** FIXED(558dac6e)
- **Source:** audit/sdks.md #6
- **Depends on:** sdk-005 (same files; the branded parse is the phase machine's output side)

## The bug

`ts/src/native.ts:78-96` — the wire type is a structural bag:

```typescript
interface QueryIr {
    readonly interiors: readonly InteriorIr[]
    readonly rec: RecIr | null
    readonly head: readonly HeadTermIr[]
    readonly rules: readonly RuleIr[]
}
```

Nothing says: interiors form a DAG, rec has nonempty base AND rec, main `rules` is nonempty, heads are bound-var-only on interiors/rec, Count has no `over`. `FindTermIr`'s aggregate is `{ kind: "aggregate"; op: AggOpIr; over?: number }` (`native.ts:115`) — Count-with-over and Sum-without-over are both legal TypeScript. `ts/src/index.ts:113,145` exports `QueryIr` and `lowerQuery` publicly, so any host code can hand `dbPrepare` a Datalog program stuffed into `interiors` with `rules: []`. Meanwhile the notation corpus keeps hand-written `QueryIr` cases flagged `"builder": false` (`ts/test/notation-corpus.test.ts:241-247`) because the builder's join-position law cannot spell queries the IR (and `query!`) can — two constructors, two languages, one engine.

## Why it's wrong

Re-exporting the engine's deliberately-loose boundary IR as a host CONSTRUCTOR makes every illegal state a public API (Insight 6: the engine validates at prepare; the host type carries no proof), and keeping a stricter builder beside it that cannot spell legal IR is accidental dualism (Insight 8: two languages for one meaning). The optional `over` is the flag-with-payload defect in the one place TS already got sums right (`AtomSourceIr`).

## The fix

Per `audit/CONTRACT.md §C6` (TS):

- `QueryIr` stops being an open public constructor: `dbPrepare` accepts a branded `ParsedQuery` (unique-symbol brand) that only two producers inhabit — `lowerQuery` (the builder path) and an exported `parseQueryIr(ir: QueryIr): ParsedQuery` structural parse (rec base/rec nonempty, main nonempty, aggregate finds split, head/find alignment). The raw `QueryIr` TYPE may stay exported for interop; the brand is what `dbPrepare` demands.
- Split aggregate finds in `FindTermIr`: `{ kind: "aggregate", op: { kind: "count" } } | { kind: "aggregate", op: FoldOpIr, over: number }` — Count carries no `over`, folds require it (mirror `CmpOpIr`'s allen-mask precedent at `native.ts:150-158`).
- Corpus `"builder": false` cases route through `parseQueryIr` — the back door becomes a front door with a parser on it. Builder walls that refuse legal IR are a separate widening question; do NOT delete corpus cases.

## Acceptance criteria

- [ ] Unrepresentable: a type test proving `dbPrepare` rejects an unbranded `QueryIr` object literal (`@ts-expect-error`); `rg -n 'over\?: number' ts/src/native.ts` → no matches.
- [ ] Runtime refusals: `parseQueryIr` rejects `rec: { base: [] }`, empty main with populated interiors, Count-with-over — new unit tests named (suggested `ts/test/parse-query-ir.test.ts`).
- [ ] Unchanged tests: notation corpus passes UNCHANGED case files; ffi tests green (mechanical rewiring to `parseQueryIr` allowed, assertions untouched).
- [ ] Green: `cd ts && pnpm test`; `cd ts/crate && PATH="$HOME/.cargo/bin:$PATH" cargo test`.

## Constraints

- The wire JSON shape crossing to `ts/crate` marshal does NOT change (CONTRACT §C1: boundary IR frozen) — the brand is host-typing only, except the `FindTermIr` aggregate split which must match what marshal accepts (sdk-008 owns marshal; the split is representable in the same JSON: `over` present iff fold). Coordinate the two so the corpus round-trips.
- Engine remains the one refusal authority; `parseQueryIr` refuses SHAPE it can see, it does not duplicate the validator.
