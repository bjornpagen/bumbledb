# docs-F04: "today's query plus two empty fields" special-case embedding (IR chapter)

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F4
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/20-query-ir.md`:
> a query with empty `interiors` and no rec is today's query plus two
> empty fields (`lean/…: evalQuery_plain`) … Main is today's query …
> — not an embedding into another type.

## Fix (cites CONTRACT C7)

There is one `Query`. Speak: a `Query` with empty `interiors` and
`rec: None` is still a `Query`; `evalQuery_plain` (or its post-lean-H1
successor name) is that case of `evalQuery`, not an embedding of a
prior type. Update the Lean citation if lean-H1/H5 rename the
declaration (C8).

## Acceptance criteria

- [ ] Grep `today's query` over `docs/architecture/20-query-ir.md`
      returns empty.
- [ ] Cited Lean declaration resolves (census); `bash
      scripts/lean.sh` green.
