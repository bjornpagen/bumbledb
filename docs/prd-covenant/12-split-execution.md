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
