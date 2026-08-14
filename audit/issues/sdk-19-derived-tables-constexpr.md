# sdk-19: C++ `derived_tables` repeats the rec flag at the lowering coordinate

Severity: med
Tree: sdk (cpp)
Status: OPEN
Source: audit/sdks.md #19
Blocked-by: sdk-01
Blocks: none

## Bug

`cpp/src/query/lower.cc:113-118,127-129`: `bool has_rec` +
`rec_ir const& rec`; name lookup is
`if (has_rec && rec.name == name) return NI` — a default `rec` with
`has_rec == true` answers as the rec for the empty name.

## Why it is wrong

Same defect as sdk-01 at a second site: the lowering coordinate
carries the flag, not the phase (Insight 5).

## Fix

Cites CONTRACT C6: `if constexpr (HasRec)` on the template
parameter (NTTP-friendly), no runtime bool; the rec branch of name
lookup exists only in the `HasRec` instantiation.

## Acceptance criteria

- [ ] Grep `has_rec` over `cpp/src/query/lower.cc` returns empty.
- [ ] Wire bytes identical on goldens; cpp + bridge tests green.

## Constraints

Rider on sdk-01 (verify this site specifically). No Program
vocabulary.
