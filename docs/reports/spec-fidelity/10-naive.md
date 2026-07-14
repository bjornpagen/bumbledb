# Spec-fidelity review 10 — the naive model (covenant PRD 15, pairing #10)

Subject: `crates/bumbledb-bench/src/naive/` (judgment, query evaluation, aggregate
folds, pack, substituted env) against the normative Lean
(`lean/Bumbledb/{Query/Denotation,Query/Aggregates,Dependencies,Txn}.lean`,
ledger `lean/Bumbledb/Bridge.lean`). Structural read; the conformance lane's
217-case agreement bounds runtime divergence only on sampled paths.

## Per-theorem fidelity table

| Theorem (Lean) | Naive site | Verdict |
|---|---|---|
| `Query.matches_def` / `repeated_var_unifies` | `admit` (naive/query.rs:588–623): bind-or-equality per occurrence | Exact on the resolved core; see D3 for the membership surface |
| `Query.param_selects_not_binds` / `paramSet_selects_membership` | `substitute` (query.rs:512–526): params substituted to literals/sets before enumeration, never bind | Exact (query-global substitution; `negated_matches` asserts nothing binds) |
| `Query.antijoin_over_active_domain` / `Safe` | `leaf_admits` negation loop (query.rs:656–663): `¬∃` over the finite relation, complete assignments only | Exact; safety inherited from validation, panics (not silent wrongness) outside it |
| `Query.membership_only_unsafe` | `leaf_admits` expect "every point variable has a scalar anchor" (query.rs:641–643) | Exact inheritance of the validator premise |
| `Query.pointIn_unfold`(+i64) | `point_in` (tuple.rs:100–102): `start ≤ t < end` over i128 | Exact (widening is order-faithful; `encode_*_order_embedding` makes numeric = word order) |
| `Query.allen_mask_denotation` / `allen_jepd` | `basic_holds` (query.rs:815–832) vs `AllenRel.holds` (Aggregates.lean:794–809) | Exact, all 13 endpoint characterizations identical; mask = ∃ basic ∈ mask holding (JEPD-equivalent, independently derived) |
| `Query.dnf_preserves_denotation` | `tree_holds` (query.rs:705–719): direct recursion, `and []` = true, `or []` = false | Exact match of `Condition.holds`; deliberately un-lowered (the property is tested against it, `tests/dnf.rs`). Error side: D4 |
| `Query.union_idempotent` / `answer_identity_canonical` / union regime | `query`/`union_fold` (query.rs:220–399): per-rule sets unioned; multi-rule aggregates fold the distinct head-projected union | Exact (BTreeSet carriers; head-key grouping matches `union_regime_head_projection`) |
| `Query.eval_sound` | whole evaluator vs `evalList` stages | Structurally parallel (join → negation → conditions → project); agrees clause for clause |
| `agg_over_distinct_bindings` | single-rule fold domain = `BTreeSet` of full bindings (query.rs:285–292); unused slots constant-filled | Exact (filler cannot split fibers) |
| `empty_global_no_answer` | `project` (query.rs:873–888): no bindings → no groups → empty set | Exact |
| `checkedSum_sound` / `wide_accumulator_exact` | `fold`/`fold_position`/`fold_duration`: i128 accumulate, one finalize `try_from` (query.rs:975–987, 1024–1029) | Exact (narrowing only at finalization; typed `Overflow { find }`) |
| `pack_canonical` / `pack_extensional` / `pack_adjacency` / `pack_lattice_closed` | `pack_segments` (query.rs:109–137): sort, merge on `start ≤ frontier`, `max` join | Exact — adjacency merges, strict gap breaks, endpoints selected from inputs, ray = ceiling end |
| `argmax_ties_all_kept` | `project` arg path (query.rs:906–938): restrict to extreme, project every survivor into a set | Exact |
| `measure_fold_laws` | `fold_duration` collects `Result` (query.rs:1018–1022): any ray errors the group | Exact at the fold; predicate side is D4 |
| `Dependencies.holds` — `Functionality`/`PointwiseKey` | `functionality_violated` (naive.rs:387–418): scalar equality + pointwise `overlaps` | Exact (half-open integer overlap ⇔ shared point) |
| `contains_iff_view_subset` / `Coverage` | `contained` (naive.rs:446–504): scalar probe, or collect-sort-merge target segments then subset test | Exact and deliberately stronger than the engine's assumptions (never trusts target disjointness) |
| `Txn.final_state_judgment_order_free` | `Delta` is a set pair; `staged` (naive.rs:266–280) is set algebra, insert wins | Exact |
| `Txn.rejection_is_complete` | `violations`/`judge`/`sealed` (naive.rs:250–260, 299–381, 538–542) | Complete WITHIN a phase; preemption diverges from the theorem — D1 |
| `Txn.committed_states_model` | target-side "what the delta broke" judgment (naive.rs:344–379) | Matches the engine and docs, not the Lean `State` — D2 |
| `Txn.witness_conflict_distinct` | `apply_from` (naive.rs:198–206): one integer compare before judgment, typed `Moved` payload | Exact (`writeFrom`'s `if`, verbatim) |

## Divergences (dual-cited, classed)

**D1 (class b, leaning c) — violation-set completeness vs key preemption.**
Lean: `violationSet`/`rejection_is_complete` (Txn.lean:146–147, 279–297) — a
rejection carries EVERY violated statement of the final state; the recorded
narrowing (Txn.lean:57–62) waives only order/dedup, not membership. Naive:
`judge` returns only the key violations when any functionality statement fails
(naive.rs:321–325), and `ClosedRelationWrite` as a preempting singleton
(naive.rs:250–257) — mirroring the engine's phase structure, which the docs
describe (docs/architecture/30-dependencies.md:67–73) while citing the Lean
theorem. On a final state violating both a key and a containment, Lean's set
contains both; both implementations cite only the keys. Shared reading; the
Lean spec sides with neither. The `ops` fuzz oracle (strict-equality against
the engine) cannot see it.

**D2 (class b, recorded in docs only) — the closed-source leak.** Lean `State`
carries `holds` (Txn.lean:93–97) and `committed_states_model` is a field
projection; the naive target side convicts only instances that held before and
fail after (naive.rs:363–371, own comment), so an empty store violating a
closed-source (domain-quantification) containment commits — the offline
sweeper's division of authority (30-dependencies.md:371). The Lean lifecycle
cannot even represent that initial state; the narrowing is recorded in docs and
the naive comment, not in Txn.lean.

**D3 (class b, recorded) — the membership-resolution seam has no Lean
arbiter.** Lean `Term.selects`/`Matches` (Denotation.lean:147–166) read every
binding as value equality; surface membership is assumed pre-lowered to
`PointIn` (Syntax.lean:33–38). The naive model re-derives the bivalent-position
resolution itself (`scalar_anchored`, query.rs:253–262; `admit`
query.rs:603–621; `constrains` query.rs:628–633 — including negated-atom and
param-set occurrences on interval fields), as the engine's normalize does
independently. A shared misreading of the resolution (exotic anchor shapes,
mixed-type edge selections) is invisible to the spec.

**D4 (class b) — ray-measure error effect is unarbitrated and
evaluation-order-sensitive.** Lean reads measure-of-ray as selects-nothing
(comparison false, no error — Denotation.lean module doc, recorded). Naive
raises `MeasureOfRay` via a rule-global poison set by any REACHED measure leaf
(query.rs:189–196, 744–750) under short-circuiting `all`/`any` tree
evaluation; the engine evaluates DNF-lowered disjuncts. A ray under an `Or`
whose sibling admits the binding can poison one side and not the other —
deep condition trees with measure leaves are exactly a corpus-thin path.

**D5 (class b) — measure finds as group keys are inexpressible in the Lean
grouping.** `Group`/`aggAnswers` fiber over `List VarId`
(Aggregates.lean:1146–1148, 1228–1233); naive keys groups by the projected
measure VALUE (query.rs:882–884), the engine by the derived measure word.
Two distinct intervals of equal measure are one group on both implementations,
two fibers under variable-keyed fibering (e.g. `[Measure(x), Count]` yields
Count 2 vs two collapsing rows of Count 1). Colliding measures are unlikely to
be corpus-sampled.

## Grade: A−

The naive model is exact on every path the Lean spec arbitrates: the matching
equation, anti-join, condition algebra (including the empty-tree readings),
set-union and head-projection regimes, empty-global, distinct-binding folds,
i128 checked sums, pack's four laws, all thirteen Allen basics, arg-tie set
honesty, both dependency judgment forms (with the model deliberately assuming
less than the engine), final-state order-freedom, and the compare-first
witness protocol — with genuinely independent arithmetic throughout (i128
widening, own Allen endpoint forms, own merge). Zero class-(a) bugs found.
The minus: its violation sets satisfy the docs' preempted reading, not
`rejection_is_complete` as stated (D1), and its rule-global ray poison (D4) is
an unforced error-parity risk against the engine that its own remit (typed
error identity) claims to cover. All five findings are two-sided spec gaps the
model shares with the engine — the shared-misreading surface is real but
recorded in three of five cases.
