# sdk-010: polarity — EDB atoms are a sum, interior atoms are a bool

- **Severity:** medium
- **Tree:** sdk (cpp dialect + ts)
- **Status:** FIXED(88e8954f)
- **Source:** audit/sdks.md #10
- **Depends on:** none (parallel-safe; textual overlap with sdk-009/011 in cpp ir.cc/rule.cc)

## The bug

Same polarity, two encodings in BOTH dialects:

- C++: EDB atoms use `body_form::atom | negated_atom` (`cpp/src/query/ir.cc:127-132`) — a sum. Interior atoms use ONE form plus `bool negated` (`cpp/src/query/rule.cc:85-90, 107-112`); the recursive-rule wall then branches on the flag: `if (item.interior.negated)` (`rule.cc:273-280`).
- TS: EDB `kind: "atom" | "negated"`, interior one kind plus `negated: boolean` (`ts/src/query/atom.ts:148-157`).

A positive interior with `negated == true` leftover binds is representable in the recorded item.

## Why it's wrong

One concept, two representations, chosen by which table the atom reads (Insight 8): the flag encoding re-admits exactly the mismatch states the EDB sum already banned, and forces every polarity consumer to know which encoding this atom uses (Insight 9).

## The fix

Per `audit/CONTRACT.md §C6`: polarity is a sum for interior atoms too.

- C++: `body_form = atom | negated_atom | interior | negated_interior | condition`; the stored `bool negated` on interior items deletes. The `with_interior<Name, bool Negated>` TEMPLATE parameter at the call site may stay (it's the call's spelling); the recorded DATA is the sum.
- TS: `kind: "interior" | "negatedInterior"`, no boolean; the wall reads the kind.

## Acceptance criteria

- [ ] Gone: `rg -n 'negated: boolean' ts/src/query/atom.ts` → no interior-atom flag; `rg -n 'bool negated' cpp/src/query` → no matches.
- [ ] Unchanged tests: cpp `ctest` + `cd ts && pnpm test` green with zero assertion edits; lowered wire IR identical (polarity was already correct at the wire; only the dialect encoding changes).
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd ts && pnpm test`.

## Constraints

- Semantics identical, including the negation-in-rec wall behavior and its error text. No Program vocabulary.
