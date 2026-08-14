# sdk-009: wildcard reified as `query_term_form::absent` — absence should be absence

- **Severity:** medium
- **Tree:** sdk (cpp dialect)
- **Status:** OPEN
- **Source:** audit/sdks.md #9
- **Depends on:** sdk-011 (same structs; land together or per INDEX order)

## The bug

Engine law: no wildcard variant — absence from `bindings` IS the wildcard. C++ reflected match products are complete structs, so every field exists with default `term_data{}` where `form == absent` (`cpp/src/query/ir.cc:38-51` defines the enumerator; `cpp/src/query/rule.cc:56-58` `continue`s absent slots at record time; `cpp/src/query/lower.cc:59-60` skips them again). `wire_term_of` even has an `absent` arm that would emit a default wire term if reached. "Wildcard bound to something" — `absent` with leftover `variable`/`literal` payload — is representable in the dialect and unwritable in the engine.

## Why it's wrong

The engine already chose the right coordinate (absence in a binding LIST); the dialect re-introduced a sentinel value meaning "not really here" that every consumer must remember to skip (Insight 4: a magic enum case for absence; Insight 2: complete reflected products are the accidental coordinate of designated-init ergonomics, not of the IR).

## The fix

Per `audit/CONTRACT.md §C6` (C++): the builder PATTERN stays a complete product (designated-init ergonomics are fine); the RECORDED IR is a binding list like the engine. `record_match` pushes only mentioned slots onto `bindings`; `term_data` is only constructed for mentioned slots; `query_term_form::absent` deletes, and `wire_term_of` loses its unreachable arm.

## Acceptance criteria

- [ ] Gone: `rg -n 'absent' cpp/src/query` → no matches (enumerator, skip-branches, and the wire arm all delete).
- [ ] Unchanged tests: cpp `ctest` green with zero assertion edits; wire bytes for existing queries identical (absent slots were already skipped — the list just never contains them now).
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`.

## Constraints

- Semantics identical. Coordinate with sdk-011 (both reshape `term_data`/`binding_data`; one fixer or strict ordering).
