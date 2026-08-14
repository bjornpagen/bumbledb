# docs-F02: main taught as "one anonymous predicate"

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F2
Blocked-by: eng-F41 (the type renames to Signature first)
Blocks: none

## Bug

`docs/architecture/20-query-ir.md`:
> **Main defines one anonymous predicate; rules derive it.** … sealed
> in the witness (`ir/validate`'s `Predicate`) … **The predicate is
> anonymous and engine-internal**

## Fix (cites CONTRACT C7, C3 amendment)

Speak: main owns the answer signature (arity, types, folds), sealed
once at validation (`ir/validate`'s `Signature` after eng-F41).
Interiors and the rec are derived tables addressed by `InteriorId`.
Do not call main a predicate.

## Acceptance criteria

- [ ] Grep `anonymous predicate|the predicate is` over
      `docs/architecture/20-query-ir.md` returns empty.
- [ ] The cited symbol matches the post-eng-F41 name; census green
      (`bash scripts/lean.sh`).
