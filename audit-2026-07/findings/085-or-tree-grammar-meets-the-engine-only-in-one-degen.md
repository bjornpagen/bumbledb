## OR-tree grammar meets the engine only in one degenerate corner: single atom, scalar leaves, projection head

category: missing-free-feature | severity: medium | verdict: CONFIRMED | finder: r2:differential-apparatus-soundness
outcome: fixed a3e09fe6

### Summary

The condition-tree grammar (`ConditionTree::Or`/`And`) is the one place the query surface accepts nested disjunction, and the spec (docs/architecture/20-query-ir.md § "The input condition grammar and DNF lowering") admits the *full* leaf grammar under it — Allen, PointIn, params, param sets, membership — plus any head shape, with negated atoms cloned per disjunct. But the only place a tree is ever executed engine-vs-oracle is `dnf_ops` in the verify lane, and it exercises exactly one degenerate corner: a single fixed atom, a fixed two-variable projection head, no negation, no params, and leaves drawn from only the six scalar operators against `U64`/`I64` literals. Every other tree-touching site is non-differential. OR composed with anything rich — aggregate heads, measure, negated atoms, membership, Allen/PointIn leaves, params, multi-atom bodies — is never checked against an oracle by any test in the workspace.

### Evidence (all verified by direct reading)

- `crates/bumbledb-bench/src/verify/run_algebra.rs:186-206` — `tree_leaf` draws only `CmpOp::{Eq,Ne,Lt,Le,Gt,Ge}` against `Value::U64`/`Value::I64` literals over `var(0)`/`var(1)`.
- `crates/bumbledb-bench/src/verify/run_algebra.rs:212-217` — the rule template: `finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))]`, `atoms: vec![posting_atom()]`, `negated: vec![]`. The `query()` wrapper (run_algebra.rs:62-67) pins `params: vec![]`. Twelve trees per run (run_algebra.rs:207).
- `crates/bumbledb-bench/src/naive/tests/dnf.rs:104-117, 129-159` — the DNF property suite is the identical single-atom/scalar-leaf/projection shape, and it is naive-vs-naive: both sides of the assertion call `NaiveDb::query`; the engine never runs.
- `crates/bumbledb-bench/src/querygen.rs:180-187` and `crates/bumbledb-bench/src/translate.rs:68-75` — both `leaf()` unwrappers are `unreachable!` on `And`/`Or`: the flat generator fleet and the SQL lane emit no trees, by design.
- Indirect route closed too: since Or lowers to multi-rule programs, differential coverage of multi-rule × {negation, params, membership} would substitute — but `querygen/shapes_rules.rs` (all three `RulesVariant`s) carries `negated: vec![]` and at most one scalar-leaf condition (shapes_rules.rs:83-135), and `rules_ops` (run_algebra.rs:71-157) uses only scalar `Ge` conditions.
- All remaining `ConditionTree::Or` construction sites in the workspace (grep-verified) are non-differential: `crates/bumbledb/tests/adversarial_ir.rs` is the no-panic sweep (typed-error-or-Ok, no answer oracle); `crates/bumbledb/tests/alloc_census.rs:569` is an allocation census; `bumbledb-query/tests/notation_corpus.rs` renders macros; `conformance.rs:562,667` are serializers/counters.

### The spec/proof gap that keeps this open

The engine's `distribute` (crates/bumbledb/src/ir/normalize/dnf.rs:89-99) is leaf-opaque — finds, atoms, and negated atoms are cloned verbatim, only the condition leaves vary per disjunct — so the lowering *function* cannot itself corrupt a rich leaf. The docs lean on `lean/Bumbledb/Query/Denotation.lean: dnf_preserves_denotation` (checked at lines 736-767): it does quantify over the full `ParamEnv` and includes negated atoms, **but only for `ruleAnswers`, the projection-head denotation** (`t = r.finds.map σ`, Denotation.lean:651-654). Aggregates live in a separate `aggAnswers` (Aggregates.lean:1560-1575) and **no aggregate-DNF-preservation theorem exists** (no `lower`/`dnf` reference anywhere in Aggregates.lean). And the Lean proof binds the model, not the Rust engine — run_algebra.rs:9-12 and dnf.rs:10-12 explicitly assign the code-level proof to the differential suite ("the differential *is* the lowering proof, now inside every verify run"), which only runs the corner above. The translate.rs:65-67 comment ("The tree grammar's OR shapes are proven against the naive model by the DNF property suite") overstates what the suite proves.

### Failure scenario

Any divergence in (a) the union-of-lowered-rules semantics for non-projection heads — an aggregate or measure head under an Or condition lowers to per-disjunct aggregates whose head rows union, which is not the tree's direct reading when disjuncts overlap; this exact family already produced one confirmed divergence (the prior OR-conditioned-aggregates finding) — or (b) downstream per-disjunct execution of a rich construct (a negated atom's anti-probe attachment per lowered rule, a ParamSet lowered per disjunct, an Allen mask leaf in one disjunct of several) passes both verify lanes and every test silently: no tree containing such a construct ever reaches the engine beside an oracle.

### Suggested fix

Widen `dnf_ops`: draw `tree_leaf` from the leaf pool the flat generator already owns (Allen with a literal interval, PointIn, `Term::Param`/param-set with a params vector, measure terms), vary the rule template over 1-2 atoms with an optional negated atom, and add aggregate-head variants (Count/Max over an Or condition with deliberately overlapping disjuncts — the shape the known divergence lives in). The naive tree evaluator already evaluates all of these directly (subject to the sibling finding's fix), so the widening is corpus-only work in run_algebra.rs. Separately, correct the translate.rs:65-67 comment or make it true.
