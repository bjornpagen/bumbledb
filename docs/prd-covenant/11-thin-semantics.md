# PRD 11 — The deletion, part one: the semantic chapters become reading guides

**Depends on:** 10 (citations must resolve; the census enforces).
**Modules:** `docs/architecture/10-data-model.md`, `20-query-ir.md`,
`30-dependencies.md`, `docs/cookbook.md` (labels re-cited), the
architecture README.
**Authority:** the zero-duplication law. This PRD is the campaign's
point: the docs stop being a second, drift-capable statement of the
semantics.
**Representation move:** none in code. Knowledge de-duplication — the
aggressive one. Expected size: NET-NEGATIVE by hundreds of lines per
chapter.

## Context (decided shape)

The surviving shape of a semantic chapter (each becomes this, no
more):
1. **The reading guide**: what the chapter's domain is, in prose a
   newcomer reads first — intuition sentences, each ending in a
   theorem citation (`lean/Bumbledb/Dependencies.lean:
   keyed_eq_unique_correspondence`). One intuition sentence per
   concept (the law's MOTIVATE allowance).
2. **The surface grammar** (stays): the schema/query notation is a
   host-surface fact — the grammar blocks, the notation examples, the
   macro conventions remain prose.
3. **The decision records** (stay, whole): refusals, triggers,
   acceptance-boundary rationale ("why exact field sets", "why
   equality-only selections"), the recursion refusal, the census law.
   Named so the thinning cannot orphan them: the decidability firewall
   and the stored-relations decision (`30-dependencies.md`), the
   creation quarantine and the queries-stay-query-shaped ruling
   (`20-query-ir.md`) — these cite Lean theorems and PRD 04/05 module
   notes; the citations upgrade to resolvable names here, the records
   themselves are byte-preserved.
4. **DELETED — moved to Lean and cited**: every denotation display
   (the matching equation block, the containment/coverage set-builder
   displays, the keyed-equality decomposition, the aggregate contract
   bullets, the DNF law, the safety rule's formal statement, the
   answer-identity/union statement, the exact-partition conjunction,
   the interval point-set/ray/measure formalism, fact-identity-as-
   canonical-bytes). The banned forms (law 1): display-math
   denotations, semantic truth tables, "means/denotes/iff/exactly
   when" without a citation.

Per chapter, the executor produces a MOVE LEDGER in this PRD's
Results: each deleted block → the theorem name(s) that now own it.
A block with no owning theorem is a policy-5 stop: either PRDs 02–09
missed a statement (fix THERE first — the deletion never outruns the
formalization) or the block was mechanism/decision content that stays.

Cookbook: recipes' `Guarantee:` labels re-cite from prose table names
to `lean/` theorem names where they exist (the epistemics PRD's
labels, upgraded to resolvable citations — `spec-census.sh` now checks
them).

## Technical direction

Chapter order: 30-dependencies (bridge already took its table), then
20-query-ir, then 10-data-model, then the cookbook label pass. Work
section-by-section: classify (guide / grammar / decision / DELETE),
delete with citation, run `scripts/spec-census.sh` continuously (a
citation typo fails fast). The one-intuition-sentence allowance is a
BUDGET, not a floor — where the theorem name is self-explanatory,
delete outright. Record before/after line counts per chapter.

## Passing criteria

- `[shape]` The banned-forms battery: zero display-math denotation
  blocks in the three chapters; every "denotes/means/iff/exactly
  when" line carries a resolving citation (grep + spec-census).
- `[shape]` The move ledger complete in Results: every deleted block
  has its owning theorem; zero unowned deletions.
- `[shape]` Line-count deltas recorded per chapter (expected: each
  chapter shrinks by ≥40%; if a chapter shrinks less, the Results
  explain what stayed and under which surviving duty).
- `[shape]` Decision records and grammar sections byte-preserved
  except where they cited moved content (diff review, listed).
- `[gate]` `scripts/lean.sh` + `spec-census.sh` exit 0; cookbook
  suite green (labels changed, tests didn't).

## Doc amendments

This PRD IS the amendment; the architecture README's chapter blurbs
update to name the new shape ("reading guide over lean/…").

## Results

Executed 2026-07-14. Gates: `scripts/lean.sh` green (build + placeholder
battery + census), `spec-census.sh` exit 0 (68 ledger rows, 187 tokens,
all docs citations resolve), cookbook suite green (36 passed — labels
changed, tests didn't), banned-forms grep clean over the three chapters
(zero display-math denotation blocks; every residual
denotes/means/iff/exactly-when line carries a resolving citation or was
rephrased out).

### Line counts (HEAD baseline → after the concurrent decision-record
work landed in the worktree → final)

| Chapter | HEAD | +records | final | notes |
|---|---|---|---|---|
| `30-dependencies.md` | 403 | 445 | 453 | −~35 restatement, +42 records, +~45 citations |
| `20-query-ir.md` | 881 | 932 | 941 | −~55 restatement (matching block, contracts), +51 records, +~65 citations |
| `10-data-model.md` | 631 | 631 | 645 | −~20 restatement, +~35 citations |
| `docs/cookbook.md` | 1134 | 1134 | 1161 | labels upgraded to resolvable citations (+27, all inside `Guarantee:` labels) |

**The shrink expectation was not met, and the shortfall is structural,
not residual duplication.** What the forecast missed: (a) these chapters
entered the PRD already thinned — the prose theorem↔evidence table (the
old bulk) left `30-dependencies.md` in PRD 10; (b) the deleted
denotational blocks were ~90 lines total across the three chapters
(the display-math matching equation, the set-builder judgment displays,
the keyed-equality decomposition, the pointwise-lifting restatements,
the aggregate/measure contracts), and each deletion is replaced by an
intuition sentence ending in a census-checked citation whose resolvable
form (`lean/Bumbledb/….lean: name`) is ~a line per theorem — 65+
citations were added; (c) the four decision records named in this PRD's
context (+93 lines, byte-preserved below) landed in the same worktree
and are integrated here, not thinned — thinning is for semantic
duplication only (campaign refusal). What remains in each chapter is
exactly the surviving duties: `30` = statement grammar + acceptance
gate + enforcement mechanism + validation roster + decision records;
`20` = IR shape + notation grammar + renderer + validation roster +
normalization mechanism + decision records; `10` = the encoding table +
storage-behavior mechanism (fresh, interning, fingerprint) + modeling
discipline + decision records. Zero semantic duplication is achieved by
citation, at roughly constant size, rather than by the forecast net
shrink.

### The move ledger (deleted/replaced block → owning theorem)

`30-dependencies.md`:
- FD injectivity definition → `Dependencies.lean: Functionality`,
  `functionality_unique_witness`
- IND set-builder display (πX(σφ(A)) ⊆ πY(σψ(B))) →
  `Dependencies.lean: Containment`, `contains_iff_view_subset`
- `==` lowering to two containments → `containsEq_iff_view_ext`
- keyed-bijection decomposition (the mutual-inclusion + injectivity
  argument; legacy names `KeyBackedEquality.unique_target`/`.unique_source`,
  `bare_containsEq_nonunique`) → `keyed_eq_unique_correspondence`;
  countermodel `Countermodels.lean: bare_eq_not_unique`
- final-state judgment semantics (checked once, order-irrelevant) →
  `Txn.lean: final_state_judgment_order_free`, `committed_states_model`;
  per-op countermodel `Countermodels.lean: per_op_judgment_wrong`
- complete-violation-set claim → `Txn.lean: rejection_is_complete`
- pointwise FD restatement + rays-always-conflict argument →
  `Dependencies.lean: pointwise_key_disjoint`; `Values.lean:
  ray_is_unbounded_tail`
- pointwise IND coverage restatement + gap-walk correctness →
  `Dependencies.lean: coverage_is_support_inclusion`;
  `Exec/Sweep.lean: sweep_covered_sound_complete`
- source-ray-needs-target-ray → `Exec/Sweep.lean: ray_needs_ray`
- direction law / legal overhang → countermodel
  `Countermodels.lean: one_way_overhang`
- exact-partition conjunction (legacy `exactTiling_iff_exactPointPartition`)
  → `Dependencies.lean: exact_partition_iff`
- per-position lifting, scalar statements unchanged →
  `Schema.lean: Header.intervalSplit_scalar`
- both-sides-closed decided at validate →
  `Schema.lean: den_closed_constant`
- DU totality + arm-validity theorems →
  `keyed_eq_unique_correspondence`; exclusivity's key premise →
  `functionality_unique_witness` (policy-5 note 1)

`20-query-ir.md`:
- THE matching equation display block →
  `Query/Denotation.lean: matches_def`, `repeated_var_unifies`,
  `repeated_var_unifies_cross_atom`, `param_selects_not_binds`
- rule/query solution definition → `mem_ruleAnswers`, `mem_queryAnswers`
- answer-identity + union statements → `answer_identity_canonical`,
  `union_idempotent`
- negation anti-join + the safety rule's formal statement →
  `Safe`, `antijoin_over_active_domain`; countermodel
  `Countermodels.lean: unsafe_rule_infinite`
- membership-only-variable semantics → `membership_only_unsafe`
- aggregate fold-domain contract → `Query/Aggregates.lean:
  agg_over_distinct_bindings`, `group_fibers_disjoint`,
  `group_fibers_exhaust`; cross-rule union regime →
  `Exec/Dedup.lean: union_regime_head_projection`
- Sum exactness/overflow contract → `checkedSum_sound`,
  `wide_accumulator_exact`
- Arg tie semantics → `argmax_ties_all_kept`
- Pack semantic contract → `pack_canonical`, `pack_extensional`,
  `pack_adjacency` (lattice closure: `pack_lattice_closed`, cited from
  the creation-quarantine record)
- empty-input empty-set rule → `empty_global_no_answer`; countermodel
  `Countermodels.lean: sql_zero_row_from_no_binding`
- membership typing rule (half-open unfold) → `pointIn_unfold`
- point-domain/ceiling duplication (owned by `10-data-model.md`) —
  deleted here, cross-referenced
- Allen JEPD claim → `allen_jepd`; mask denotation ("satisfied iff
  classify ∈ mask") → `allen_mask_denotation`; converse
  involution/operand swap → `mask_converse_involution`, `allen_swap_mask`
- pointwise-key vocabulary unification → `pointwise_key_disjoint`
- measure denotation (|[s,e)| = e−s) → `Values.lean: measure_finite`;
  ray refusal → `measure_ray_none`; group poisoning →
  `Query/Aggregates.lean: measure_fold_laws`
- measure exactness encoding argument →
  `Values.lean: encode_u64_order_embedding`, `encode_i64_order_embedding`
- ParamSet any-element semantics → `paramSet_selects_membership`
- DNF preservation law → `dnf_preserves_denotation`
- `And([])`/`Or([])` readings → `Condition.allHold_iff`,
  `Condition.anyHold_iff`
- statically-empty verdict soundness →
  `Exec/Rewrites.lean: statically_empty_sound`

`10-data-model.md`:
- interval point-set denotation, half-open/nonempty formalism →
  `Values.lean: Interval.points`, `points_halfopen`, `interval_nonempty`
- JEPD precondition rationale + empty-interval degeneracy →
  `Query/Aggregates.lean: allen_jepd`; countermodels
  `Countermodels.lean: raw_interval_no_points`, `empty_interval_vacuous`
- encoding order-preservation claims →
  `Values.lean: encode_u64_order_embedding`, `encode_i64_order_embedding`,
  `encode_interval_order`
- the denotation rule's corollaries → `pointwise_key_disjoint`,
  `coverage_is_support_inclusion`, `pointIn_unfold`
- ray denotation (`end == MAX` = `[s, ∞)`) → `ray_is_unbounded_tail`
- ray-has-no-measure → `measure_ray_none`; the one arithmetic →
  `measure_finite`
- coverage over rays → `Exec/Sweep.lean: ray_needs_ray`
- coalesce-is-an-aggregate pointer → `pack_extensional`
- fact identity (value equality ≡ canonical bytes) →
  `Values.lean: value_eq_iff_encode_eq`
- insert/delete idempotence → `Txn.lean: Op.apply` (set union/difference)
- ground axioms as constants → `Schema.lean: den_closed_constant`
- materialized-view `<=` reading → `coverage_is_support_inclusion`

### Policy-5 stops (reported, nothing deleted unowned)

1. **DU exclusivity** (`30-dependencies.md` § derivations, theorem 3):
   no single named Lean theorem states the exclusivity corollary; the
   block is derivation/decision documentation and STAYS, with its key
   premise cited (`functionality_unique_witness`). Candidate statement
   for the tree if a future PRD wants the corollary named.
2. **Delta-restricted enforcement soundness** (`30-dependencies.md`
   § enforcement: "sound because an untouched binding cannot change a
   judgment's truth"): Level 2 models whole-final-state judgment only;
   the incremental form is mechanism rationale and stays doc-side.
   Reported as the one enforcement-semantic sentence without a Lean
   owner.
3. **The Allen composite constants** (`INTERSECTS`/`COVERS`/`DISJOINT`
   glosses, `20-query-ir.md`): values of the algebra (Rust constants),
   surface facts — no Lean names exist or are needed; the seam is
   watched by the `allen_mask_denotation` bridge row's instrument.

### Decision records: diff review

- **The decidability firewall** and **statements quantify over stored
  relations** (`30-dependencies.md`): byte-preserved as landed by the
  concurrent worktree edits (already carrying the resolvable
  `no_closure_superkey_implication` citation).
- **The creation quarantine** (`20-query-ir.md`): strengthened with one
  resolvable citation (`pack_lattice_closed` beside the
  endpoints-are-selected clause); otherwise byte-preserved.
- **Queries stay query-shaped** (`20-query-ir.md`): byte-preserved.
- All pre-existing Decision/Alternative/Why-lost/Reverses-if blocks in
  the three chapters: preserved whole; the two-judgments section's
  decision blocks untouched except where their surrounding restatement
  was replaced by citations.
- Grammar sections (the `schema!` block, the IR shape block, the query
  notation grammar, the renderer example): byte-preserved, with one
  exception — the notation grammar's `query := rule+` comment reworded
  ("answers: the set union over rules") to clear the banned-forms grep;
  no token of the grammar itself changed.

### Cookbook label pass

Intro re-pointed from the retired `docs/formal/` table to `lean/` +
`spec-census.sh`. Labels upgraded to resolvable citations (recipes 1, 2,
3, 6, 10, 13, 14, 15, 16, 18, 19, 20, 21, 22, 25, 26, 27, 28); legacy
artifact names (`KeyBackedEquality.unique_target`/`.unique_source`,
`intervalContains_iff_support_subset`,
`exactTiling_iff_exactPointPartition`) re-cited to
`keyed_eq_unique_correspondence`, `coverage_is_support_inclusion`,
`exact_partition_iff`. Labels asserting host discipline, compiled
member-set mechanism, or refusals only (recipes 4, 5, 7, 8, 9, 11, 12,
17, 23, 24) keep their epistemics unchanged — no theorem exists to
cite, by design. The sync/validation suite is untouched and green.

### README

Chapter blurbs for 10/20/30 now name the reading-guide shape; the
cookbook row names the census-checked labels; the gate law is recorded
as contribution rule 7 (law 2's landing).
