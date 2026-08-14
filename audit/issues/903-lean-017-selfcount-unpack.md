# lean-017: `selfCount_eq_one_mem` filters and cases to recover the self atom the type should carry

- **Severity:** low
- **Tree:** lean
- **Status:** DUPLICATE(lean-002)
- **Source:** audit/lean.md L3

`selfCount_eq_one_mem` (`Exec/Reach.lean:207-222`) unpacks `selfCount self = 1` into `∃ a ∈ r.atoms, a.source = .interior self` by filtering on `decide (a.source = .interior self)` and casing on the filtered list's length; `reachOp_empty` (`239-258`) spends it. Under lean-002's `RecStep`, the self occurrence is the `selfBindings` field — the existence is definitional, `selfCount` is deleted, and both proofs are rewritten as part of lean-002's restatement of the reach lemmas. No residual edit.
