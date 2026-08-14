# docs-F03: rec taught as "one linear SCC"

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F3
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/20-query-ir.md`:
> `rec` is at most one linear SCC — `Rec { head, base, rec }`
> `rec: Option<Rec>,  // at most one linear SCC`
> the rec SCC as **one** pool … refuses negation in the rec SCC

## Fix (cites CONTRACT C7)

SCC is a Tarjan/condensation artifact. Speak: `rec: Option<Rec>` —
at most one linear rec. The rec pool is `base.len() + rec.len()`.
`NegationInRec` refuses negation in that rec.

## Acceptance criteria

- [ ] Grep `SCC` over `docs/architecture/20-query-ir.md` returns
      empty.
- [ ] `bash scripts/lean.sh` green.
