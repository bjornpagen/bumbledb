# lean-018: `evalQuery_empty_rules` is a product-field surprise ("the rec is never the answer")

- **Severity:** low
- **Tree:** lean
- **Status:** DUPLICATE(lean-001)
- **Source:** audit/lean.md L4

lean-001 restates `evalQuery_empty_rules` over the sum's total `Query.rules` accessor and rewrites the Bridge caption (`Bridge.lean:583-586`) from warning-tone to structural-tone. The theorem's language content (empty main denotes ∅; a derived table is never the result) is essential and already owned there. No residual edit.
