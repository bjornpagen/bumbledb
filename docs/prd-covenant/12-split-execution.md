# PRD 12 — The deletion, part two: execution docs keep only the machine

**Depends on:** 10 (citations), 11 (the method is proven on the
easier chapters first).
**Modules:** `docs/architecture/40-execution.md` (the split),
`50-storage.md` (encoding laws thin), `70-api.md` (transaction
semantics thin), `60-validation.md` (spot citations).
**Authority:** the zero-duplication law + the mechanism fence's dual:
just as Lean never models mechanism, the docs never restate
semantics. `40-execution.md` currently does double duty; after this
PRD it is purely the machine — Free Join realization, COLT, the
pipelined executor, the kernels, the Apple-Silicon measured laws, the
refutation records — and every semantic sentence in it is a citation.
**Representation move:** none in code. The docs' regimented focus,
delivered.

## Context (decided shape)

`40-execution.md` — the classification:
- STAYS (the chapter's true content, untouched): the Free Join
  paper-fidelity narrative and deviation records; COLT and the trie
  machinery; pump/probe_pass mechanism; batching; the sanctioned
  kernel shapes and the portable/intrinsic verdict matrix; the
  port-topology and flag-free measured laws with their numbers; the
  memo; every refutation record (union elision, estimator); the
  microbench pin discipline.
- THINS TO CITATIONS: what the executor MUST COMPUTE — set-semantic
  union/dedup (→ `Exec/Dedup` names), the elision licences (→
  `distinct_witness_licence`, `disjoint_witness_licence`), the sweep's
  correctness and premises (→ `Exec/Sweep` names), grounding's
  preservation and elimination laws (→ `Exec/Rewrites`), key-probe
  equivalence, statically-empty soundness, the latch's two-constructor
  distinction. The pattern per site: one sentence of what the
  mechanism achieves + the theorem that says so + the mechanism prose
  itself (which stays).
`50-storage.md`: the encoding order-preservation section thins to
citations of 02's embedding theorems (the byte layouts, namespaces,
LMDB discipline, determinant-index mechanics, crashpoint table all
STAY — physical facts). Fact-identity's formal statement cites
`value_eq_iff_encode_eq`.
`70-api.md`: the transaction-semantics prose (final-state judgment,
witness classes, conflict-vs-violation, snapshot isolation, the
maintenance protocol's division of authority, ETL laws) thins to
citations of `Txn.lean` names; the API usage documentation (how to
call things, the witness-class table's operational half) stays.
`60-validation.md`: the measurement/testing doctrine is operations
and stays whole; the fuzzing-charter oracle descriptions gain theorem
citations where the oracle IS a theorem's sample (rewrites target →
`grounding_preserves_answers`; the ops verdict-parity oracle →
`rejection_is_complete`).

Move ledger + line-count deltas in Results, as PRD 11.

## Technical direction

Same method as 11, harder judgment calls: when a paragraph interleaves
mechanism and semantics, SPLIT it — the semantic clause becomes the
citation sentence, the mechanism prose survives verbatim. The measured
laws are untouchable (grep the numbers before/after — identical). The
refutation records are untouchable. When in doubt whether a sentence
is semantics or mechanism, ask: could the fuzzer distinguish an engine
that violates it? Yes → semantics → cite; no (it's about HOW fast/
WHERE bytes live) → mechanism → stays.

## Passing criteria

- `[shape]` The banned-forms battery green over all four chapters;
  every citation resolves (spec-census).
- `[shape]` The measured-numbers battery: every number present before
  is present after (grep the pinned figures — zero loss).
- `[shape]` Refutation/deviation records byte-preserved (diff review,
  listed in Results).
- `[shape]` Move ledger + line deltas in Results (40-execution
  expected to shrink ~25-35% — it was always majority-mechanism; the
  others per their content).
- `[gate]` `scripts/lean.sh` + spec-census exit 0; full doc-reference
  batteries (no dangling intra-doc links to deleted sections — grep
  the anchors).

## Doc amendments

This PRD IS the amendment.

## Results

### Line deltas (before → after)

| chapter | before | after | delta |
|---|---|---|---|
| `40-execution.md` | 853 | 871 | +18 (+2.1%) |
| `50-storage.md` | 450 | 459 | +9 |
| `70-api.md` | 563 | 579 | +16 |
| `60-validation.md` | 658 | 665 | +7 |

**The expected 25–35% shrink did not materialize, and the honest reason is
recorded:** these chapters were already the machine. The crucible-era
rewrites left them majority-mechanism — the semantic duplication present
was single-sentence scale (the display-math denotational mass lived in
10/20/30, PRD 11's chapters), so the mandated pattern per site (one
sentence of what the mechanism achieves + the theorem + the mechanism
verbatim) is net-ADDITIVE: 34 citation sites gained `lean/` names while
the deletions were sentences, not blocks. The chapters are now
zero-duplication not because they got shorter but because every semantic
sentence they still carry cites its theorem.

### Move ledger — 40-execution

| deleted/thinned restatement | owning theorem(s) |
|---|---|
| key-probe get = the rule's join denotation | `Exec/Rewrites.lean: keyprobe_equiv_join` |
| statically-empty rule deletion is instance-independent | `Exec/Rewrites.lean: statically_empty_sound` |
| ray-end kernel reasoning (`t = MAX−1` walk-through, deleted) | `Values.lean: ray_is_unbounded_tail` |
| "two facts equal on bound vars ⇒ one binding; the solution is a set" (deleted) | `Exec/Dedup.lean: seenfold_is_set_semantics` |
| Arg tie set-honesty (was a `20-query-ir.md` pointer) | `Query/Aggregates.lean: argmax_ties_all_kept` |
| Pack semantics pointer (was `20-query-ir.md` § aggregation) | `Query/Aggregates.lean: pack_extensional`, `pack_canonical` |
| the sweep code-sharing claim (one fold, two consumers) | `Exec/Sweep.lean: pack_is_the_sweep` |
| the elision law "distinct facts ⇒ distinct bindings" (deleted) | `Exec/Dedup.lean: distinct_witness_licence` |
| the disjointness witness form + its soundness argument (deleted) | `Exec/Dedup.lean: syntactic_disjointness_sound`, `disjoint_witness_licence` |
| "one sink hearing several rules means exactly ∪" (deleted) | `Exec/Dedup.lean: union_regime_head_projection` |
| head-projection dedup-key law + the quoted `20-query-ir.md` sentence (deleted) | `Exec/Dedup.lean: union_regime_head_projection` |
| single-rule fold domain = distinct full bindings | `Query/Aggregates.lean: agg_over_distinct_bindings` |
| Pack-crosses-rules quote (deleted) | `Exec/Sweep.lean: pack_is_the_sweep` over the union regime |
| "rewrites are semantics-preserving" (now proved, not only fuzzed) | `Exec/Rewrites.lean: grounding_preserves_answers`, `elimination_sound`, `rewrite_composition` |
| elimination conditions 1–3, restated in full (deleted, ~14 lines) | `Exec/Rewrites.lean: ElimStep` + `elimination_sound` |
| "every readable snapshot satisfies every accepted statement" | `Txn.lean: committed_states_model` |
| the fold's rule-death honesty (`folded to ∅`) | `Exec/Rewrites.lean: ground_refuted_empty` |
| "the fold is never semantic" | `Exec/Rewrites.lean: grounding_preserves_answers` |
| latch miss (per-execution) vs fold refutation (per-plan) | `Exec/Rewrites.lean: EmptyAt` |

Condition 4 of elimination (scalar positions only) stays doc-side whole:
it is a recorded v0 refusal with an OPEN trigger, not a theorem premise.

### Move ledger — 50-storage

| thinned restatement | owning theorem(s) |
|---|---|
| fact identity = canonical bytes | `Values.lean: value_eq_iff_encode_eq` |
| determinant key order = value order | `Values.lean: encode_u64_order_embedding`, `encode_i64_order_embedding` |
| prefix-group interval-start order | `Values.lean: encode_interval_order` |
| coverage-walk verdict = point-subset denotation under the token's premise | `Exec/Sweep.lean: sweep_covered_sound_complete`, `ray_needs_ray` |
| closure op-order irrelevance | `Txn.lean: final_state_judgment_order_free` |

Byte layouts, namespaces, LMDB discipline, determinant-index mechanics,
and the crashpoint table are untouched (diff reviewed).

### Move ledger — 70-api

| thinned restatement | owning theorem(s) |
|---|---|
| snapshot isolation (a read is a function of one state) | `Txn.lean: snapshot_reads_one_state` |
| delta op-order irrelevance | `Txn.lean: final_state_judgment_order_free` |
| the COMPLETE violation set (complete, sound, nonempty) | `Txn.lean: rejection_is_complete` |
| witness compare invisible on success / abort-before-run | `Txn.lean: writeFrom_unmoved`, `writeFrom_moved` |
| witness conflict ≠ dependency violation | `Txn.lean: witness_conflict_distinct` |
| the division of authority (soundness vs freshness) | `Txn.lean: derived_soundness_vs_freshness` |
| the ETL laws (round-trip no-op; lands-valid-or-rejects) | `Txn.lean: etl_identity`, `etl_lands_valid` |

Banned-form fix: the borrowed-struct "one lifetime iff" reworded (a
host-surface Rust fact, no theorem applies). The witness-protocol slogan
now carries its citations. All usage documentation (grammar, idioms,
error taxonomy, ETL surface, bindings) stays verbatim.

### Move ledger — 60-validation (spot citations; doctrine whole)

| oracle description | theorem it samples |
|---|---|
| `ops` commit-verdict parity (the sealed set on both sides) | `Txn.lean: rejection_is_complete` |
| `rewrites` dual-pipeline differential | `Exec/Rewrites.lean: grounding_preserves_answers` (+ `rewrite_composition`) |
| the converse-property lane | `Query/Aggregates.lean: allen_swap_mask` |
| the naive Pack point-set definition | `Query/Aggregates.lean: pack_extensional` |
| exact-abutment coverage subfamily | `Exec/Sweep.lean: adjacent_segments_cover` |
| the ray-end boundary subfamily | `Exec/Sweep.lean: ray_needs_ray` |

Banned-form fixes: `verify-store` exit rule and the net-disposition
"source side" definition reworded (operations prose, no semantics moved).

### Policy-5 stops (unowned semantic blocks — kept, reported)

1. **The complement rule** (40-execution, negated closed-atom fold): the
   negated fold is explicitly UNMODELED in the spec — the recorded
   narrowing in `Exec/Rewrites.lean` (the domain guarantee and a negated
   membership the condition grammar cannot write). The block stays whole
   as the semantic authority; an in-doc note now records that status and
   the grounding differential as its empirical check. Formalizing
   `fold_negated` is a census candidate for PRD 14.
2. **Rule subsumption** (40-execution, the restricted UCQ-minimization
   witness): no `Exec/Rewrites` theorem owns the denotational-containment
   claim ("the keeper contains the sibling — a body homomorphism at the
   identity mapping"). Block untouched; second census candidate.

### Batteries

- **Measured-numbers battery: zero loss.** The 40-execution numeric
  inventory deltas are exactly: deleted doc-file-reference digits
  (`20-query-ir.md` ×4, `10-data-model.md` ×1, `30-dependencies.md` ×1)
  and the deleted elimination-list ordinals (1./2./3./4.); 50-storage
  gains two "64" occurrences from added theorem names; 70-api and
  60-validation inventories are identical. Every pinned figure
  (32.1/32.6/32.4%, 1396.5/1393.2/1408.3 µs, 948.0/938.8/952.1 µs,
  1376.9/937.2, 82,983, 691.2×, 4761.9×, 6.35×, 2.7×, 8.8 vs 4.0–4.6,
  2.65×, +164%, …) present before is present after.
- **Refutation/deviation records byte-preserved** (diff reviewed): the
  cross-rule dedup refutation, the estimator record, Deviations
  D1–D5 + the paper-§ deviation blocks, the wrong-cover record, the
  crashpoint table — no hunk intersects any of them.
- **Banned forms**: the remaining `iff`/`means` instances in the four
  chapters each carry a `lean/` citation in-sentence or sit inside the
  reported policy-5 complement block (whose citation note names the
  recorded narrowing).
- **Gates**: `scripts/lean.sh` exit 0 (build green, placeholder battery
  clean); `scripts/spec-census.sh` exit 0 (68 ledger rows, 187 tokens,
  docs citations intact) after every edit. The one markdown anchor
  leaving these chapters (`60-validation.md` →
  `30-dependencies.md#formal-claims-and-runtime-evidence`) resolves.
