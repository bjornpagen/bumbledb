# lean-016: `RewriteStep` threads an unused `{n : Nat}` arity through every constructor

- **Severity:** low
- **Tree:** lean
- **Status:** DUPLICATE(lean-005)
- **Source:** audit/lean.md L2

Every `RewriteStep` constructor (`Exec/Rewrites.lean:2310-2359`) binds `{n : Nat}` solely to inhabit `Query.plain n (…)`. lean-005's restatement of `RewriteStep : Theory → Classify → List Rule → List Rule → Prop` deletes the wrapper and the `n` with it — there is no residual edit.
