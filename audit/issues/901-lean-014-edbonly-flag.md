# lean-014: `Rule.edbOnly` + `plain_wellFormed` re-check what a plain query's type should say

- **Severity:** medium
- **Tree:** lean
- **Status:** DUPLICATE(lean-004)
- **Source:** audit/lean.md M8

The named apparatus — `Rule.edbOnly` (`Syntax.lean:327-332`, a Bool re-computing "no interior sources" per rule) and `plain_wellFormed` (`Syntax.lean:498-510`, inhabiting the `WellFormed` bundle for plain queries) — is exactly lean-004's deletion list, and the "plain is a constructor" half is lean-001. The remaining half of the original finding (a typed `PlainAtom`/EDB-only rule type for the cq arm) is refused under CONTRACT §C5 R-DENSE: interior-free-ness of a `.cq` with empty interiors is already a consequence of the boundary refusal (`UnknownInterior`), and a second atom type would split every rule theorem in two. No edit remains beyond lean-004 + lean-001.
