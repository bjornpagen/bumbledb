# PRD 14 — Census close: the covenant is counted

**Depends on:** 01–13 all landed. Terminal, always last.
**Modules:** read-mostly; write access to this packet's ledgers,
`docs/formal/` (deleted here), the architecture README.
**Authority:** the campaign audit discipline; the zero-duplication
law's terminal enforcement.
**Representation move:** none — the campaign's claims become counted,
dated evidence.

## The batteries (results recorded IN THIS FILE: command, count, date)

1. **Zero-sorry battery**: the placeholder greps over `lean/` — sorry,
   admit, axiom declarations → zero; `lake build` from clean (delete
   the build dir, rebuild) exit 0; build wall time recorded.
2. **Zero-duplication battery** (the campaign's point): the banned-
   forms greps over ALL of `docs/architecture/` and `docs/cookbook.md`
   — display-math denotations zero; "means/denotes/iff/exactly when"
   lines all citation-carrying; the spec-census citation-resolution
   check exit 0; and the REVERSE check: sample ten theorems across
   02–09, grep the docs for restatements of their content (the
   executor judges each hit: motivation-sentence = legal, restatement
   = violation to fix).
3. **Bridge completeness**: every `Obligation` row's three fields
   resolve (build + spec-census); the ledger count vs 02–09's premise
   inventory — zero orphaned premises (a theorem whose Rust discharge
   exists but is unlisted).
4. **Preservation battery**: the measured-numbers grep (every pinned
   figure in 40/50-execution/storage present, byte-identical); the
   refutation/deviation records byte-identical; the decision records
   (refusals + triggers) intact; the recursion refusal + its trigger
   record intact and now pointing at `Exec/` as the prepared home.
5. **Line-delta ledger**: per-chapter before/after counts from PRDs
   11/12 consolidated; the total semantic-prose deletion recorded
   (the campaign's headline number).
6. **Engine-untouched battery**: `git diff <campaign-start>..HEAD --
   crates/` shows only PRD 13's recorded serializer additions and any
   recorded visibility changes; fingerprint pin + corpus digest pin
   byte-untouched across the whole campaign.
7. **CI battery**: the lean job (build + placeholders + spec-census +
   conformance exe) green on an actual push; wall time recorded; lane
   placement per the measured number.

## The terminal acts

- **Delete `docs/formal/`** — the statement inventory is fully ported
  (verify: every theorem name mentioned in the artifact's audit
  exists in-tree, modulo the recorded renames and the recorded
  empty-global divergence); the provenance note moves to
  `lean/README.md`'s history section; git history keeps the artifact.
- The architecture README's map updates to the final shape: which
  chapter owns what, the lean/ pointer, the gate law stated where
  contributors will see it.
- The covenant README's ledgers marked CLOSED with the final commit.

## Passing criteria

- `[shape]` All seven batteries green, recorded with commands, counts,
  dates; the reverse-duplication sample's ten judgments listed.
- `[shape]` `docs/formal/` gone; the port-completeness verification
  recorded (artifact theorem inventory → in-tree names table).
- `[gate]` Full workspace gates green (check.sh, check-asm, fuzz
  tests) AND the lean lane green — the two spec halves cashed
  together, once, at the end.

## Doc amendments

The README map; nothing else — the campaign's amendments were the
deletions.

## Results

Executed 2026-07-14, on the pinned M2 Max, against the campaign range
`78845f98..072d3a7b` (PRDs 01–13, all landed) plus this closing
commit. Foreign worktree edits present during the census
(`00-product.md`, PRD packets 04/05/10, `recursion-design.md` — the
concurrent recursion-docs work) were left unstaged, per the standing
worktree discipline.

### Battery 1 — zero-sorry (green)

- `grep -rnE --include='*.lean' --include='*.md' --include='*.toml'
  --exclude-dir='.lake' '(^|[^[:alnum:]_])(sorry|admit)([^[:alnum:]_]|$)'
  lean/` → **0** (2026-07-14).
- `grep -rnE --include='*.lean' --exclude-dir='.lake'
  '^\s*((private|protected|noncomputable|unsafe|scoped|local)\s+)*axiom\s'
  lean/` → **0**.
- Bonus sweep: `native_decide`/`ofReduceBool` → **0** (no kernel
  bypass anywhere).
- Clean build: `rm -rf lean/.lake && scripts/lean.sh` → exit 0,
  **7.67 s wall** (26 jobs: full tree + conformance exe + placeholder
  battery + census + the 217-case corpus run at 955 ms). The
  seconds-fast law (law 4) holds from clean.

### Battery 2 — zero-duplication (green)

- Display-math denotations over ALL of `docs/architecture/` +
  `docs/cookbook.md`: `grep -n '⟦\|⊨\|$$'` → **0**; leading
  set-operator display lines (`^[σπ⋈∪∩⊆∈]`) → **1** hit
  (`40-execution.md:361`, a mechanism-order note INSIDE a sentence
  carrying its `pack_is_the_sweep` citation — not a denotation block).
- Banned-word lines (`iff`/`denotes`/`means`/`exactly when`, word-
  bounded): **10** hits total, every one judged (see the reverse
  sample below for the judging rule); zero uncited semantic
  restatements. `exactly when` → **0** hits outright.
- `scripts/spec-census.sh` → exit 0: **68 ledger rows, 188 tokens
  resolved, docs citations intact** (2026-07-14).
- **The REVERSE check — ten theorems sampled across 02–09, docs
  grepped for restatements of their content, each hit judged:**
  1. `interval_nonempty` (02) — hits in `10-data-model.md:115` and
     cookbook intro: one-sentence motivations carrying the citation.
     LEGAL.
  2. `encode_i64_order_embedding` (02) — the `10-data-model.md`
     byte-layout table says "sign-flipped big-endian
     (order-preserving)": the TABLE is the docs' surviving encoding
     duty (bytes, not semantics) and the order-preservation CLAIM at
     line 21 cites the order-embedding theorems; `50-storage.md:75`
     cites in-sentence. LEGAL.
  3. `functionality_unique_witness` (03) — `30-dependencies.md:19`
     "at most one fact per determinant tuple" carries the citation
     in-sentence; cookbook hits are recipe schema comments (modeling
     intent, not engine semantics). LEGAL.
  4. `exact_partition_iff` (03) — cookbook recipe 26's `Guarantee:`
     label carries the resolving citation; following prose is
     intent-level intuition. LEGAL.
  5. `antijoin_over_active_domain` (04) — `20-query-ir.md` § negation
     carries `Safe`, the theorem, and the countermodel citations
     in-sentence; the IR struct comment is grammar (a surviving
     duty). LEGAL.
  6. `empty_global_no_answer` (05) — `20-query-ir.md:275` carries the
     citation plus the countermodel citation. LEGAL.
  7. `sweep_covered_sound_complete` (06) — `50-storage.md:196`
     ("under exactly that premise the one-pass verdict equals the
     point-subset denotation") cites the theorem in-sentence. LEGAL.
  8. `distinct_witness_licence` (07) — `40-execution.md:253` (the
     elision bullet) carries the citation; neighboring
     `syntactic_disjointness_sound` and `disjoint_witness_licence`
     likewise. LEGAL.
  9. `grounding_preserves_answers` (08) — both hits
     (`40-execution.md:385,493`) carry the citation; the complement
     block's uncited `iff`/`means` lines sit INSIDE the recorded
     policy-5 exception whose status note names the recorded
     narrowing (ruling (a) below). LEGAL (as the recorded exception).
  10. `final_state_judgment_order_free` (09) — `30-dependencies.md`
      § judged on final states cites the theorem, the countermodel,
      `committed_states_model`, and `rejection_is_complete`
      in-sentence; `70-api.md:271` cites `rejection_is_complete` and
      points to the owning section. LEGAL.
  **Verdict: ten of ten legal — zero restatements to fix.**

### Battery 3 — bridge completeness (green)

- All **68** `Obligation` rows resolve: the Lean half by the build
  (every row carries the term-level `@theoremName` reference), the
  Rust/docs half by the census (188 tokens; exit 0). `ledger_count`
  asserts 68 = the grep-derived row count.
- **Orphaned-premise sweep**: the module docs carry **103** inline
  `Bridge:` notes (the allowed residue); all **156** distinct Rust
  tokens they name were resolved against the tree (paths exist, line
  ranges within file bounds, symbols grep word-bounded) → **zero
  stale notes, zero orphaned premises**. Every note maps to a ledger
  row directly or through a recorded companion clause (the
  `_i64`/`_u64` companions, `repeated_var_unifies_cross_atom`,
  `etl_identity` beside `etl_lands_valid`, the sweep component
  lemmas under THE witness-token row); countermodel notes are design
  pointers, not premises, by construction.
- PRD 10's two stale-note fixes: the pair is not separately
  identifiable in-repo (PRD 10's packet carries no Results section —
  the one gap in the campaign's written ledgers, noted here for
  honesty); the verifiable form of "they landed" is the sweep above —
  **no stale note exists today**.

### Battery 4 — preservation (green)

- Measured-numbers grep (PRD 12's inventory): every pinned figure
  present exactly once where expected — 32.1/32.6/32.4 %,
  1396.5/1393.2/1408.3 µs, 948.0/938.8/952.1 µs, 1376.9/937.2,
  82,983, 691.2×, 4761.9× (×2, both sites), 6.35×, 2.7×, 2.65×,
  +164%, 8.8 (×2, `40-execution.md`). **Zero loss.**
- `40-execution.md`, `50-storage.md`, `70-api.md` are byte-untouched
  since PRD 12 landed (`git diff 062ae630..HEAD` — no hunks);
  `60-validation.md` gained exactly PRD 13's recorded oracle-roster
  paragraph (+16 lines). Refutation/deviation records (D1–D5, the
  cross-rule dedup refutation, the estimator record, the wrong-cover
  record, the crashpoint table) therefore remain byte-identical to
  PRD 12's diff review. Decision records intact (`Decision:` blocks
  present across 00/10/20/30/40/50/70).
- The recursion refusal + trigger: intact in `20-query-ir.md`
  § engine recursion (three derivation legs, three trigger clauses,
  the surviving ruling). **The one allowed edit was made**: the
  refusal record now names `lean/Bumbledb/Exec/` as the fixpoint
  model's prepared home when the trigger fires — both halves of the
  pre-payment (the seam ledger and the spec tree) now have named
  landing sites.

### Battery 5 — line-delta ledger (consolidated from PRDs 11/12)

| chapter | before | after | delta |
|---|---|---|---|
| `10-data-model.md` | 631 | 645 | +14 |
| `20-query-ir.md` | 881 | 941 | +60 |
| `30-dependencies.md` | 403 | 453 | +50 |
| `40-execution.md` | 853 | 871 | +18 |
| `50-storage.md` | 450 | 459 | +9 |
| `60-validation.md` | 658 | 665 → 681 | +7 (PRD 12) +16 (PRD 13) |
| `70-api.md` | 563 | 579 | +16 |
| `docs/cookbook.md` | 1134 | 1161 | +27 |

**The honest outcome, recorded as the campaign's headline:** the
forecast net shrink did not materialize and was the wrong metric.
Zero duplication was achieved **by citation at roughly constant
size**: the deleted denotational mass (~90 display-math lines in
10/20/30, sentence-scale restatements in 40–70) was replaced by
intuition-sentence-plus-citation, the four decision records landed
concurrently (+93 lines, preservation-exempt), and the census-checked
citations are net-additive by construction. The numbers that ARE the
headline: **139 `lean/` citations across the docs (113 in resolvable
declaration form, census-checked every push), banned-forms count
ZERO, 68 bridge rows, 260 theorems / 9,627 lines of Lean** owning
every semantic fact the prose used to restate.

### Battery 6 — engine untouched (green)

- `git diff 78845f98..HEAD --stat -- crates/ fuzz/` →
  `crates/bumbledb-bench/src/conformance.rs | 1805 +` and
  `crates/bumbledb-bench/src/lib.rs | 1 +` — exactly PRD 13's
  recorded conformance lane (a bench-crate addition; PRD 13 recorded
  "engine `pub` needs: none"). `fuzz/` untouched. Nothing else under
  `crates/` in the whole campaign.
- The two pins: `git log 78845f98..HEAD --
  crates/bumbledb-bench/src/schema.rs` (the golden fingerprint,
  `the_fingerprint_is_pinned`) and `--
  crates/bumbledb-bench/src/corpus_gen/tests.rs`
  (`the_corpus_digest_is_deterministic_and_pinned`) → **zero
  commits, zero hunks** — byte-untouched across the campaign.

### Battery 7 — CI (green)

- `.github/workflows/ci.yml` parses (checked 2026-07-14). The lean
  job carries all four batteries through `scripts/lean.sh` (build,
  placeholder greps, spec-census, the conformance corpus run) plus
  the three-way comparator step; wall numbers recorded in the
  workflow comments (12.4 s engine+naive replay + 1.2 s Lean, and
  the ~1.0 s corpus run in `lean.sh`'s own comment) — per-push
  placement per the measured numbers. First-run-on-a-real-push
  remains the recorded pending item (the PRD-16-of-crucible
  precedent: this workflow cannot be exercised from the local
  machine; this commit is deliberately unpushed per the census
  instruction).

### Census rulings (the campaign's open policy-5 items, closed)

1. **The `fold_negated` complement rule** (PRD 12 policy-5 stop 1;
   the recorded narrowing in `lean/Bumbledb/Exec/Rewrites.lean`
   § narrowings): **RULED — it stays a recorded narrowing, not a
   blocker.** Modeling the complement fold requires the domain
   guarantee (`domain_within_ids`) as a named premise and a negated
   membership the condition grammar cannot write — a future spec
   obligation, not a census failure. The `40-execution.md` complement
   block remains the semantic authority with its status note; the
   grounding differential (`fuzz/fuzz_targets/rewrites.rs`) is its
   empirical check. *Trigger: the first change to `fold_negated`'s
   direction rules or witness forms in
   `plan/ground/evaluate.rs:203-273` — the gate law then demands the
   Lean model move in the same commit, which means formalizing the
   domain guarantee.*
2. **Rule subsumption's UCQ-containment claim** (PRD 12 policy-5
   stop 2; `40-execution.md` § rule subsumption): **RULED — a
   recorded spec gap.** No `Exec/Rewrites` theorem owns "the keeper
   contains the sibling — a body homomorphism at the identity
   mapping"; the doc block stays whole as the semantic authority with
   its status note. Candidate future theorem: the restricted-witness
   containment (sibling answers ⊆ keeper answers under the recorded
   shape). *Trigger: any widening of the subsumption witness beyond
   the current restricted shape, or a subsumption-attributed
   differential trophy — either forces the owning theorem before the
   change lands.*
3. **DU exclusivity** (PRD 11 policy-5 stop 1; `30-dependencies.md`
   § derivations, theorem 3): **RULED — same treatment.** The
   exclusivity corollary has no single named theorem; the block is
   derivation/decision documentation and stays, with its key premise
   cited (`functionality_unique_witness`). Candidate future theorem:
   the named exclusivity corollary over the DU arm union. *Trigger:
   any schema-macro or validator change touching the DU derivation's
   acceptance path — the corollary gets its name in the same
   commit.*

### Port-completeness — `docs/formal/` retired

The artifact (`GPT55DependencyTheory.lean`, 582 lines, SHA-256
`e1f09501…48a576`-pinned in its README, byte-identical to the source)
carried **23 theorems**. Every one exists in-tree, modulo the
recorded renames and the two recorded divergences:

| artifact theorem | in-tree home |
|---|---|
| `contains_iff_view_subset` | `Dependencies.lean: contains_iff_view_subset` (same name) |
| `containsEq_iff_view_ext` | `Dependencies.lean: containsEq_iff_view_ext` (same name) |
| `KeyBackedEquality.unique_target` | `Dependencies.lean: keyed_eq_unique_correspondence` (recorded rename, PRD 11 move ledger) |
| `KeyBackedEquality.unique_source` | `Dependencies.lean: keyed_eq_unique_correspondence` (both directions, one correspondence) |
| `bare_containsEq_nonunique` | `Countermodels.lean: bare_eq_not_unique` (recorded rename) |
| `bare_containsEq_target_not_key` | folded into `bare_eq_not_unique`'s two-row model (its doc records the target projection is NOT a key) |
| `contains_source_selection_strengthen` | `Dependencies.lean: selection_monotonicity` (both monotonicity directions, one theorem) |
| `contains_target_selection_weaken` | `Dependencies.lean: selection_monotonicity` |
| `intervalContains_iff_support_subset` | `Dependencies.lean: coverage_is_support_inclusion` (recorded rename) |
| `exactTiling_iff_exactPointPartition` | `Dependencies.lean: exact_partition_iff` (recorded rename) |
| `overshoot_pointwiseKey` | `Countermodels.lean: overhang_tile_pointwise_key` |
| `overshoot_isTiling_not_exact` | `Countermodels.lean: one_way_overhang` |
| `empty_nat_interval_has_no_points` | `Countermodels.lean: raw_interval_no_points` |
| `positive_range_restriction_implies_wellscoped` | `Query/Denotation.lean: Safe` + `safe_negated_bound` (safety AS the range-restriction spec; spent by `antijoin_over_active_domain`) |
| `ruleUnion_set_idempotent` | `Query/Denotation.lean: union_idempotent` |
| `repeated_var_forces_equal` | `Query/Denotation.lean: repeated_var_unifies` (+ `repeated_var_unifies_cross_atom`) |
| `constant_match_forces_value` | `Query/Denotation.lean: matches_def` (the literal arm of term selection) |
| `parameter_match_forces_value` | `Query/Denotation.lean: param_selects_not_binds` |
| `point_membership_unfold` | `Query/Denotation.lean: pointIn_unfold` (+ `pointIn_unfold_i64`) |
| `allen_meets_unfold` | `Query/Aggregates.lean: classify_holds`/`allen_mask_denotation` (the one-relation unfold generalized to the proved JEPD classification + mask denotation) |
| `atom_match_row_from_snapshot` | `Query/Denotation.lean: matches_def` + `mem_ruleAnswers` (matching quantifies over the one snapshot's facts; `snapshot_single` pins the one-instance reading) |
| `checkedAdd_sound` | `Query/Aggregates.lean: checkedAdd_sound` (same name; `checkedSum_sound`/`checkedSum_complete` extend it) |
| `stratified_no_direct_negative_self` | structurally subsumed: the modeled syntax has no head-referencing atoms (`Query/Syntax.lean: Atom.relation : RelId` — rules are one step short of the fixpoint), so the property the lemma excludes is unwritable; the recursion refusal (campaign README; `20-query-ir.md`) owns it, with `Exec/` the prepared home when the trigger fires |

The two recorded divergences: the **empty-global aggregate** (the
artifact's `aggEval sum [] = some 0` is REFUSED — the engine is the
authority; `Query/Aggregates.lean: empty_global_no_answer`,
countermodel `Countermodels.lean: sql_zero_row_from_no_binding`,
recorded in the Aggregates module doc) and the **stratification
subsumption** above. **Verdict: port-complete.** `docs/formal/`
deleted this commit; the provenance note (audit date, pinned commit
`98f1103`, toolchain, SHA-256, the never-in-repo imports fact) moved
to `lean/README.md` § history. Two module docs
(`Dependencies.lean`, `Query/Aggregates.lean`) cite the artifact by
its historical path as provenance — those now resolve through git
history via the README's history section, deliberately unedited
(module docs are outside this PRD's write scope).

### Terminal acts

- `docs/formal/` deleted (the SHA-pinned artifact remains reachable
  in git history forever).
- `lean/README.md` gained the history section (provenance,
  divergences, the port-table pointer).
- The architecture README map updated to the final shape: the
  `../formal/README.md` row retired in favor of the `lean/` tree row;
  the 40/50/70 blurbs now name the mechanism-only shape and the 60
  blurb the THREE oracles; the gate law was already contribution
  rule 7 (PRD 11's landing).
- The covenant README's ledgers marked **CLOSED** with the final
  content commit of the pre-census campaign (`072d3a7b`); the census
  itself closes in the commit carrying this Results section. PRD 15
  (reports-only) remains the campaign's spend.

### The gate, cashed once at the end (2026-07-14, pinned M2 Max)

- `scripts/check.sh` → exit 0, **232 s** wall.
- `scripts/check-asm.sh` → exit 0, **8.3 s** wall (warm from the
  check build).
- The fuzz corpus replay (`cargo test` in `fuzz/`) → exit 0,
  **466 s** wall (the query three-way replay dominates, per the CI
  comment's measured shape).
- The lean lane (`scripts/lean.sh`) → exit 0 twice: from clean
  (7.67 s, battery 1) and again after every census edit (68 rows,
  188 tokens, docs citations intact, 217/217 conformance cases
  agree). The two spec halves cashed together.
