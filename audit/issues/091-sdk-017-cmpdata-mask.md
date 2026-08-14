# sdk-017: TS `CmpData.mask` is an optional beside the op instead of inside it

- **Severity:** medium
- **Tree:** sdk (ts)
- **Status:** FIXED(23482d0f)
- **Source:** audit/sdks.md #17
- **Depends on:** none (localized; parallel-safe)

## The bug

`ts/src/query/atom.ts:90-96` — the builder's comparison data:

```typescript
mask: MaskData | undefined   // comment: "present exactly for `allen`"
```

Eq-with-mask and Allen-without-mask are representable. The WIRE type already got this right: `CmpOpIr` at `ts/src/native.ts:150-158` is `{ kind: "allen"; mask: number } | { kind: "eq" } | …` — the mask lives inside the op, like engine `CmpOp::Allen { mask }`.

## Why it's wrong

"Present exactly for X" written in a comment is the type failing to say it (Insight 4: an optional whose validity depends on a sibling field); the codebase itself demonstrates the correct shape one layer down, so the builder is a gratuitous second encoding of the same sum (Insight 8).

## The fix

Per `audit/CONTRACT.md §C6` (TS): runtime `CmpData` matches the wire shape — `op: { kind: "allen", mask: number } | { kind: Exclude<CmpKind, "allen"> }`; the sibling `mask` field deletes; lowering copies the op through instead of re-pairing kind with mask.

## Acceptance criteria

- [ ] Gone: `rg -n 'mask: MaskData \| undefined|present exactly' ts/src/query/atom.ts` → no matches.
- [ ] Unchanged tests: `cd ts && pnpm test` green with zero assertion edits; lowered `CmpOpIr` identical.
- [ ] Green: `cd ts && pnpm test`; `cd ts && pnpm run build`.

## Constraints

- Semantics identical; wire shape untouched (it was already right).
