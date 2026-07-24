## OR condition on a Sum/Count aggregate head silently coarsens the fold domain; engine and naive oracle diverge

category: bug | severity: high | verdict: CONFIRMED | finder: lean:query

### Summary

The engine chooses the aggregate sink's dedup regime on the **post-DNF** rule count. A single written aggregate rule whose condition tree contains an `Or` is lowered into 2+ Or-free rules by `distribute`/`collapse`, so it prepares as a "multi-rule" program and receives the union-regime `AggregateSink`, whose binding-dedup key is the **head projection** (group-variable spans + fold-input spans) rather than the full slot array. For idempotent folds (Min/Max/CountDistinct, Pack) the two domains fold identically; for **Sum** and **Count** they do not: distinct full bindings that project to equal head tuples are absorbed, shrinking the answer. The normative contract — `ir.rs:187-190`: "The fold domain of every aggregate is the group's set of distinct full bindings over all query variables" — and the naive differential oracle (which dispatches on the **written** rule count and folds full bindings) both say otherwise. Two shipped oracles return different answers for a validation-accepted query, and every differential lane is structurally blind to the combination.

### Evidence (all verified against the working tree)

- **Regime chosen post-DNF**: `crates/bumbledb/src/api/prepared/build.rs:132-136` — `let program = if rules.len() > 1 { SinkProgram::Union } else { SinkProgram::SingleRule(first.distinct_witness()) }`, where `rules` iterate the lowered survivors (`witness.rule(rule_idx)`, build.rs:93-114).
- **DNF makes the multi-rule program**: `crates/bumbledb/src/ir/validate/validate.rs:393` (`collapse(rules.iter().flat_map(distribute).collect())`); `ir/normalize/dnf.rs:120-165` — `Or` concatenates disjuncts and `collapse` merges only condition-**set**-equal bodies, so `Or(t<10, t>20)` survives as two lowered rules.
- **Union key = head projection only**: `exec/sink/aggregate/new.rs:379-399` (`union_span`: `Var` span, `Agg` fold-input span, `Pack` two-word span; nullary `Count` contributes nothing); `fold_row.rs:36-45` gates every fold on `dedup_key`, and `fold_row.rs:167-182` gathers exactly those spans. No elision escape: `new.rs:121-133` — "Multi-rule sinks always retain the spanning union representation, **even when the rules are provably disjoint**"; `make_sink` (build.rs:1043) maps `SinkProgram::Union` unconditionally to `AggregateSink::for_union`.
- **Engine behavior pinned by its own test**: `exec/sink/tests/aggregate.rs:707-776` (`the_union_seen_set_keys_head_projections_across_rule_layouts`) — a re-derived `(g=7, x=100)` with a differing existential slot is ABSORBED; "Sum folds {100, 250, 300} once each".
- **Naive oracle takes the single-rule full-binding path**: `crates/bumbledb-bench/src/naive/query.rs:268` passes the **written** `query.rules`; `rows_for` at 351-353 matches `[rule]` and calls `project` (line 1152), which groups and folds the rule's distinct **full** bindings; the `Or` tree is evaluated natively (`SubstitutedTree::Or`, line 994). Module doc at 250-252 states the single-rule contract explicitly. (For genuinely multi-**written**-rule programs, `union_fold` at 499-539 implements the head-projection semantics, so engine and naive agree there — the divergence is exclusively at the DNF seam.)
- **Validation knows this hazard class and refuses it only for Arg**: `ir/validate/validate.rs:399-418` — `ArgAcrossRules` is judged "on the LOWERED rule count (a DNF blowup of one Arg rule refuses too)". Sum and Count sail through; no other `lowered.len() > 1` refusal exists.
- **Differential lanes never exercise the combination**: `querygen.rs:176-186` — "the generator emits flat conjunctions only"; the DNF algebra lane's rules carry projection-only finds (`run_algebra.rs:213`); `naive/tests/dnf.rs` is aggregate-free (its expects read "no aggregates: no overflow", lines 133/151).
- **Spec/proof coverage (checked per audit instructions)**: `docs/architecture/20-query-ir.md` § aggregation defines the across-rules head-projection fold domain for multi-rule programs and, for Arg only, explicitly extends the refusal to "DNF-lowered rules included" — it is silent on Sum/Count at the same seam. `lean/Bumbledb/Query/Denotation.lean:746` (`dnf_preserves_denotation`) is stated for projection `ruleAnswers` only, yet its doc-comment claims "lowering-then-evaluating ≡ evaluating the tree naively; the engine never sees an Or" — a contract the engine violates for Sum/Count heads. Every fold-domain definition in `Query/Aggregates.lean` (`bindingSet` 1425, `aggAnswers` 1569) is single-rule; `Exec/Dedup.lean:567` (`union_regime_head_projection`) states the multi-rule law but never connects it to DNF clones of one written rule.

### Failure scenario

`Query::single(Rule { finds: [Var(a), Aggregate{Sum, over: amt}], atoms: [Posting(id=p, account=a, amount=amt, at=t)], conditions: [Or(t<10, t>20)] })` over two postings `(a=7, amt=100, t=5)` and `(a=7, amt=100, t=25)`:

- **Engine**: DNF yields lowered rules `{t<10}` and `{t>20}` (not set-equal, both kept; the elision path never applies to union sinks) → union regime → both bindings key `(7, 100)` → the second is absorbed → **Sum = 100**.
- **Naive oracle / ir.rs contract**: one written rule, two distinct full bindings (`p` and `t` differ) → **Sum = 200**.
- The same rule with the single leaf condition `t < 30` returns 200 from the engine too (single-rule regime keys the whole slot array) — so semantically equivalent condition spellings give different answers.
- **Count is starker**: with head `[Var(a), Count]`, the nullary Count contributes nothing to the union key, so the key is the group variable alone — every group Counts **1** regardless of its binding-set size.

### Suggested fix

Two representation-level options consistent with the house doctrine:

1. **Extend the existing refusal** (the minimal, precedented fix): `ArgAcrossRules` is already judged on the lowered rule count; add Sum and Count heads to the same `lowered.len() > 1` refusal. The idempotent folds (Min/Max/CountDistinct, Pack — whose raw claim is in the key) fold identically over full bindings and head projections and stay legal.
2. **Key DNF-clones of ONE written rule on the shared full slot array**: clones share one variable scope and one layout, so the "VarId is rule-scoped / layouts differ" objection that forces head-projection keys for genuinely distinct rules (the pinned test's exact rationale, aggregate.rs:707-712) does not apply; a binding satisfying two overlapping disjuncts still dedups because the slot arrays are identical.

Either way: state the multi-rule fold-domain definition in Lean (Aggregates.lean is single-rule-only) so `dnf_preserves_denotation` can be extended or its aggregate counterexample recorded, and add a differential row mixing `Or` with a Sum/Count head (the querygen exclusion and the projection-only DNF lane currently make this seam unreachable by construction).
