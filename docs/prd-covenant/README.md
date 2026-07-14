# The covenant campaign

The formal specification becomes a living Lean development in `lean/`,
and the architecture docs are aggressively thinned to ZERO duplication:
every semantic and execution-semantic fact lives in exactly one place —
the Lean tree — while the docs keep their regimented focus on what Lean
cannot hold: mechanism, measurement (Free Join realization, COLT,
Apple-Silicon laws), decisions, and operations.

The refinement chain the spec maintains:

- **Level 0 — denotations**: what every construct means.
- **Level 1 — abstract algorithms**: each semantics-bearing algorithm
  as a small pure Lean function, PROVED equal to its denotation. Where
  an algorithm needs a premise the denotation does not supply, Lean
  forces the premise to be named — those names are exactly the engine's
  witness types.
- **Level 2 — the lifecycle**: transactions, final-state judgment,
  generation witnesses, ETL — a state machine with its invariance
  theorems.
- **Level 3 — verified Rust: REFUSED, permanently.** The Rust↔Lean link
  is empirical: the differential, fuzz, and exhaustive estates, plus
  this campaign's executable-denotation conformance lane (PRD 13).

## The PRDs

- `01-scaffold.md` — the `lean/` lake project, toolchain pin, module
  tree seeded from the audited artifact (whose missing imports are
  re-derived), the CI lane, `docs/formal/` retired.
- `02-values.md` — Level 0: the value universe, order-embedding
  encodings, nonempty half-open intervals, rays, measure.
- `03-dependencies.md` — Level 0: theories, ground axioms,
  functionality (scalar + pointwise), containment, coverage, keyed
  equality, exact partition.
- `04-query-denotation.md` — Level 0: the matching equation,
  unification, range restriction, anti-join, condition lowering
  (DNF preservation), rule union, answer identity.
- `05-aggregates.md` — Level 0: folds over distinct binding sets,
  checked sums, Measure, Pack = coalesce, Allen masks.
- `06-exec-sweep.md` — Level 1: the sweep as a fold; coverage and
  Pack correctness under the disjoint+ordered premise — the
  `DisjointDeterminantProof` theorem.
- `07-exec-dedup.md` — Level 1: seen-set union; the elision licences —
  the `DistinctWitness` and `DisjointWitness` theorems; the union
  regime.
- `08-exec-rewrites.md` — Level 1: grounding as denotation-preserving
  partial evaluation; key-probe soundness; statically-empty folds.
- `09-txn.md` — Level 2: the transaction state machine; op-order
  invariance; witness conflicts ≠ dependency violations; snapshot
  isolation; the ETL identity.
- `10-bridge.md` — `Bridge.lean`: the machine-listable obligation
  ledger (each Lean premise ↔ the Rust mechanism that discharges it),
  replacing the prose theorem↔evidence table; `scripts/lean.sh` and
  the spec-census tooling.
- `11-thin-semantics.md` — the deletion: `10-data-model`,
  `20-query-ir`, `30-dependencies` become reading guides; every moved
  fact deleted from prose, cited by theorem name.
- `12-split-execution.md` — the deletion, part two: `40-execution`
  splits into mechanism-only; `50-storage`'s encoding laws and
  `70-api`'s transaction semantics thin to citations.
- `13-conformance.md` — the executable denotation: Lean evaluates
  Tiny worlds as the third differential oracle.
- `14-census-close.md` — zero-duplication batteries, zero-sorry, the
  bridge resolves, doc line-deltas recorded, gates cashed. Always last.

Dependency spine: 01 first. 02→03→04→05 in order (each imports the
prior). 06/07/08 after 04+05 (parallel-safe among themselves).
09 after 03. 10 after 02–09 (it indexes every theorem). 11/12 after 10
(docs cite what exists). 13 after 04+05 (needs computable denotations).
14 last, always.

## Laws

1. **The zero-duplication law (this campaign's whole point).** A
   semantic or execution-semantic FACT lives in `lean/` and nowhere
   else. The architecture docs may MOTIVATE (one intuition sentence
   per concept) and CITE (`lean/…` theorem name), never restate.
   Banned in docs after PRDs 11–12: display-math denotations,
   semantic truth tables, matching/typing equations, and any
   "means/denotes/iff/exactly when" sentence that does not carry a
   theorem citation. Mechanically: deleting any semantic sentence from
   the docs must lose nothing that `lean/` does not already state.
   The census (PRD 14) greps for the banned forms.
2. **The gate law.** A change to accepted schemas, query denotation,
   or execution semantics is not done until the Lean side moves in the
   same commit; the CI lean lane enforces buildability, the census
   enforces citation integrity. (This law lands in the packet and is
   recorded in the docs' contribution notes by PRD 11.)
3. **The mechanism fence.** Level 1 models the algorithmic essence
   ONLY: a sweep is a fold, grounding is substitution, dedup is set
   union. The moment a Lean file mentions batching, buffers, scratch,
   SIMD, pipelining, memos, or LMDB, it is modeling mechanism and the
   PRD is mis-scoped: stop. Performance content NEVER moves to Lean —
   the docs keep Free Join realization, COLT, the kernels, the
   measured laws, the refutation records, whole.
4. **Mathlib-free.** The tree builds on core Lean 4 (`lake build`
   seconds-fast, CI-cheap). `Std`/`Batteries` may be adopted ONLY if a
   PRD records the specific need; heavy automation and mathlib are
   refused — proofs stay elementary (finite sets as `List`-quotients
   or `Finset`-lite structures built in-tree; decidability by
   construction).
5. **Zero sorry, zero axioms, zero `admit` — always.** Every PRD's
   gate includes `scripts/lean.sh` (build + a grep battery for
   placeholder tokens). A statement that resists proof is either
   wrong (a design finding — record it, the campaign's best outcome)
   or over-general (narrow it and record the narrowing).
6. **Countermodel-first.** Anything refused or bounded gets its
   countermodel in `Countermodels.lean`, ported or new — the design
   scratchpad is part of the spec.
7. **The docs' surviving duties are named**: mechanism + measured laws
   + decision records + operations + the notation grammar (a
   host-surface fact, not a semantic one) + the cookbook (intuition,
   labeled by PRD-21-style epistemics, citing theorems). Nothing else
   survives in the semantic chapters.
8. **Standing campaign rules**: no test-only PRDs (locks ride their
   PRD); no migrations; no shims; tree need not typecheck between
   PRDs; commit `--no-verify`; the fingerprint pin and the corpus
   digest pin never move (nothing here touches the engine except PRDs
   10/13's bench/test additions — engine SOURCE changes are out of
   scope for this campaign except where PRD 13's serializer needs a
   `pub` accessor, recorded).

## Refusals (binding)

- **Level 3 / verified Rust** (Aeneas, coq-of-rust): refused forever;
  the conformance lane is the link.
- **Mathlib**: refused; see law 4.
- **Modeling the planner's cost decisions**: heuristics are not
  semantics; the DP estimator stays entirely doc-side.
- **Modeling durability/crash**: the crashpoint estate owns it;
  Level 2 models committed-state transitions only.
- **Recursion in this campaign**: the census-law refusal stands.
  `Exec/` is where C0 lands WHEN the trigger fires — the tree is its
  prepared home, not its premature birthplace.
- **Deleting the decision records or measured ledgers from docs**:
  never — thinning is for semantic duplication only.

## The seed-artifact fact (binding on PRD 01–05 executors)

`docs/formal/GPT55DependencyTheory.lean` imports `LeanQuerySemantics`
(which imported `DependencyTheory`) — neither was ever in this
repository. The artifact therefore does NOT check standalone; it is a
statement inventory, not a working seed. The campaign REBUILDS the
base definitions in-tree (02–05) and PORTS the artifact's theorem
statements and proofs onto them, adapting names to the language law
(PointIn, answers, ground axioms, determinant, functionality). PRD 14
retires `docs/formal/` once every ported statement checks in `lean/`.
