# The Lean specification

This tree is the ONLY normative home of bumbledb's semantics; the
architecture docs cite it and never restate it. The docs keep what
Lean cannot hold: mechanism, measurement (Free Join realization, COLT,
the Apple-Silicon laws), decision records, and operations.

## Toolchain record

The pin file (`lean-toolchain`) is bare by format, so the selection
record lives here. The pin moves deliberately, never implicitly — the
same law as the repository's `rust-toolchain.toml`.

- **Pinned:** `leanprover/lean4:v4.32.0` — the latest stable Lean 4
  release at selection time (`elan self update` to 4.2.3, then
  `elan toolchain install stable` resolved to v4.32.0).
- **Date:** 2026-07-14.
- **Check 1 — the tree builds:** `lake build` under the pin completes
  green (15 jobs, seconds-fast).
- **Check 2 — the version matches:** `lean --version` under the pin
  reports `Lean (version 4.32.0, arm64-apple-darwin24.6.0, commit
  8c9756b28d64dab099da31a4c09229a9e6a2ef35, Release)`.

Lakefile form: **TOML** (`lakefile.toml`) — the declarative form; this
project needs no build programmability.

## The refinement chain

- **Level 0 — denotations**: what every construct means
  (`Values`, `Schema`, `Cardinality`, `Dependencies`,
  `Subsumption`, `Query/Syntax`, `Query/Denotation`,
  `Query/Membership`, `Query/Aggregates`).
- **Level 1 — abstract algorithms**: each semantics-bearing algorithm
  as a small pure Lean function, PROVED equal to its denotation
  (`Exec/Sweep`, `Exec/Dedup`, `Exec/Rewrites`, `Exec/Plan` — the
  Free Join plan formalism at the mathematical level: plan validity,
  plan soundness against the rule denotation, plannability — and
  `Exec/Fixpoint` — which deliberately carries both levels of the one
  feature: the stratified denotation and the fueled round loop,
  proved to agree).
  Where an algorithm needs a premise the denotation does not supply,
  Lean forces the premise to be named — those names are exactly the
  engine's witness types. `Oracle` sits at this level too (placement
  recorded 2026-07-14, the admission calculus): enforcement plans as
  witness terms over an abstract ordered oracle, each accepted form's
  delta-restricted check decided at a proved consultation count —
  law 3's abstract-cost scoping, spent. `Admission` closes that
  calculus: the acceptance gate as one inhabited structure
  (`AdmissibleForm`) — per single-key form one term bundling the
  denotation, the executable judge, the delta restriction, the
  oracle plan, and quarantine compliance (five inhabitants); a
  future form enters the vocabulary by inhabiting the
  type, and the refused E1 shape is proved uninhabitable in
  `Countermodels`.
- **Level 2 — the lifecycle**: transactions, final-state judgment,
  generation witnesses, ETL — a state machine with its invariance
  theorems (`Txn`), plus the fresh-mint allocation model
  (`Txn/Fresh`).
- **Level 3 — verified Rust: REFUSED, permanently.** The Rust↔Lean
  link is empirical: the differential and exhaustive estates,
  plus the executable-denotation conformance lane.

`Bridge.lean` is the obligation ledger (each Lean premise ↔ the Rust
mechanism that discharges it); `Countermodels.lean` is the design
scratchpad — anything refused or bounded gets its countermodel there.
`Bumbledb.lean` imports everything; building it builds the tree.

## The laws

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
   enforces citation integrity.
3. **The mechanism fence.** Level 1 models the algorithmic essence
   ONLY: a sweep is a fold, grounding is substitution, dedup is set
   union. The moment a Lean file mentions batching, buffers, scratch,
   SIMD, pipelining, memos, or LMDB, it is modeling mechanism and the
   work is mis-scoped: stop. Performance content NEVER moves to Lean —
   the docs keep Free Join realization, COLT, the kernels, the
   measured laws, the refutation records, whole. **Abstract cost is
   admissible mathematics** (owner ruling 2026-07-14, the admission
   calculus): consultation counts over an abstract ordered oracle —
   complexity, stated and proved — may live here, because "the
   enforcement plan exists and is oracle-bounded" is a semantic fact
   of acceptance, not a measurement. Measured cost — pinned numbers,
   batch sizes, kernel shapes, anything with units — stays in the
   docs, forever.
4. **Mathlib-free.** The tree builds on core Lean 4 (`lake build`
   seconds-fast, CI-cheap). `Std`/`Batteries` may be adopted ONLY if a
   PRD records the specific need; heavy automation and mathlib are
   refused — proofs stay elementary (finite sets as `List`-quotients
   or `Finset`-lite structures built in-tree; decidability by
   construction).
5. **Zero placeholders, zero axiom declarations — always.** Every
   gate runs `scripts/lean.sh`: the build plus a grep battery for the
   proof-escape tokens and for `axiom` as a declaration keyword. The
   battery bans the escape tokens outright throughout this tree —
   comments and this README included, which is why this law does not
   spell them; the exact patterns and their shaping are documented in
   the script. A statement that resists proof is either wrong (a
   design finding — record it, the campaign's best outcome) or
   over-general (narrow it and record the narrowing).

## What Lean does NOT own

Mechanism and measurement (the planner's cost decisions, batching,
kernels, LMDB layout, every pinned number), durability and crash (the
crashpoint estate owns those; Level 2 models committed-state
transitions only), the notation grammar (a host-surface fact), and
operations. Those live in `docs/architecture/`, whole.

## History — the seed artifact's provenance

This tree was built from a statement inventory, not a working seed:
`GPT55DependencyTheory.lean`, produced by the gpt55 audit on
2026-07-13, pinned against repository commit `98f1103`, checked under
`leanprover/lean4:v4.32.0` with no `axiom` declarations and no proof
escapes. Its two imported precursor modules (`LeanQuerySemantics`,
which imported `DependencyTheory`) were supplied by the audit
environment and were never in this repository, so the artifact did not
check standalone here. The campaign REBUILT the base definitions
in-tree (PRDs 02–05) and PORTED the artifact's theorem statements and
proofs onto them, adapting names to the language law; the census
(PRD 14) verified every artifact theorem against the tree and retired
`docs/formal/` — the byte-pinned copy (SHA-256
`e1f09501079feb23ad93be9ab98aeba3b6b5f50a6a84cbbbf78af095c048a576`,
byte-identical to the source artifact) remains reachable in git
history forever. The port table lives in
`docs/prd-covenant/14-census-close.md`; the one recorded semantic
divergence is the empty-global aggregate (the artifact's `sum [] = 0`
is refused — `Countermodels.lean`, the SQL zero-row countermodel), and
the artifact's stratification lemma was structurally subsumed at port
time (the then-modeled syntax had no head-referencing atoms). The
stratified fixpoint model has since landed — 2026-07-14, owner
decision — in `Exec/Fixpoint.lean` over `Query/Syntax.lean`'s program
cut, the prepared home entered. The engine discharge campaign followed
on 2026-07-15 and is complete — the cut, the strata judge, the oracles,
the delta-variant plans, the transient images, the per-stratum driver,
the counted round surface, and the named-head notation
(`docs/architecture/20-query-ir.md` § engine recursion;
`40-execution.md` § the fixpoint driver) — and `Bridge.lean` carries
the fixpoint model's rows, landed with the mechanisms they ledger.
