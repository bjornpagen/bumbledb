# docs-003: the IR chapter calls the rec "one linear SCC"

- **Severity:** high
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F3
- **Depends on:** none (prose; same file as docs-001..010)

## The bug

`docs/architecture/20-query-ir.md:96` — "`rec` is at most one linear SCC — `Rec { head, base, rec }`"; the IR shape at `:374` — "`rec: Option<Rec>,  // at most one linear SCC`"; plus "the rec SCC as **one** pool" and "refuses negation in the rec SCC (`NegationInRec`)".

## Why it's wrong

An SCC is a Tarjan/stratum artifact — the coordinate system of the deleted Program condensation (Insight 1). The representation is `Option<Rec>`: at most one linear rec, judged by `Query.recLinear`. Calling it an SCC in the NORMATIVE IR chapter keeps condensation alive as the mental model.

## The fix

Per `audit/CONTRACT.md §C7`: "`rec: Option<Rec>` — at most one linear rec." The pool sentence: "the rec pool is `base.len() + rec.len()`." The wall sentence: "`NegationInRec` refuses negation in that rec." Comment in the shape block: `// at most one linear rec`.

## Acceptance criteria

- [ ] Gone: `rg -n 'SCC' docs/architecture/20-query-ir.md` → no matches.
- [ ] `MAX_RULES` pooling claim and `NegationInRec` name unchanged.

## Constraints

- Prose only. `NegationInRec` and all locked names untouched. Note: the TS builder's runtime message "this cut admits one rec SCC" (`ts/src/query/lower.ts:1636`) is CODE, charged to sdk-022 — do not edit code from this issue.
