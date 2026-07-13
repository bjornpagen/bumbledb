# The constitution campaign

Align vocabulary, semantics, and internal/external data structures with
the formal model: `bumbledb-definitive-semantics-and-refactor-spec.pplx.md`,
the two gpt55 audits, and `GPT55DependencyTheory.lean` (checked into
`docs/formal/` by PRD 01). The audits were pinned to `98f1103`,
pre-crucible; every claim below was RE-VERIFIED against current main
(`31d82b27`+) on 2026-07-13 — the reconciliation ledger at the bottom of
this file is authoritative about what survived.

The theme in one sentence: **the code must say what the math says, in
the math's own words, and every invariant that is currently prose
becomes representation.**

## The PRDs

Phase A — the formal anchor
- `01-lean-anchor.md` — the Lean model into `docs/formal/`, the
  theorem↔evidence table into the architecture docs.

Phase B — soundness by representation (the P0s)
- `02-checked-interval.md` — `CheckedInterval` + typed upper bound;
  the encoder becomes total; `Value` loses its raw interval payloads.
- `03-coverage-evidence.md` — `Enforcement::Probe { coverage: bool }`
  splits into `ScalarProbe` / `IntervalCoverage { DisjointGuardProof }`;
  the coverage sweep consumes proof, not prose.
- `04-member-set.md` — `MemberSet` + `AxiomIndex` replace the raw
  `[u64; 4]` + free-function membership.
- `05-generation-id.md` — `GenerationId` + `CommitSeq` newtypes over
  the bare `u64`s.

Phase C — one word, one meaning (the vocabulary sweep)
- `06-point-in.md` — `CmpOp::Contains` → `CmpOp::PointIn`.
- `07-grounding.md` — `plan/chase` → `plan/ground`; the `chase-off`
  feature → `ground-off`; the pass is named what it is in the Datalog
  literature: grounding the sealed atoms.
- `08-determinant.md` — the storage `guard` vocabulary → the
  `determinant` family (the FD's left side, materialized); raw guard
  bytes → `DeterminantImage`; `GuardRule`/`GuardPlan` → the
  `KeyProbe` family.
- `09-measure.md` — `Term::Duration`/`FindTerm::Duration` →
  `Measure` in the IR; `Duration` stays the surface keyword.
- `10-small-renames.md` — `pitch` → `stride`, `image/distinct` →
  `image/cardinality`, `OverflowKind::Origins` → `OriginCapacity`,
  dedicated `str`/`bool` order-refusal diagnostics.

Phase D — the public meaning made honest
- `11-exact-partition.md` — the cookbook's "tiling" corrected to
  disjoint cover; the five-statement exact-partition idiom becomes a
  recipe with gap/overhang rejection locks.
- `12-keyed-equality.md` — `==` documented as key-backed unique
  correspondence; the reverse-key rejection lock.
- `13-denotation.md` — the normative denotational contract completed
  in `20-query-ir.md` (matching equation, three equality levels,
  answer-level dedup, the fact/answer/tuple glossary) + the answer
  vocabulary cutover in the API surface.
- `14-diagnostics.md` — `NoMatchingTargetKey` carries the available
  keys; redundant-superkey warning; unresolved-literal visibility;
  negated-binder lock tests.
- `15-explain-contract.md` — plan introspection goes from "stable-ish"
  versioned deterministic contract with goldens.
- `16-arg-grammar.md` — `ArgMax`/`ArgMin` enter the `query!` grammar;
  the render/parse asymmetry dies.
- `17-distinct-witness.md` — `provably_distinct` returns a typed
  witness, not a `bool` (the `DisjointWitness` precedent).

Phase E — completion of the general mechanisms (from the Lean-alignment
brief, audited 2026-07-13; see the brief reconciliation below)
- `19-arity-coverage.md` — the containment/equality generators sweep
  every legal projection arity and type mix.
- `20-maintenance-protocol.md` — derived-relation maintenance and
  conditional-write witnesses: the protocol documented and locked.
- `21-cookbook-epistemics.md` — every recipe carries its epistemic
  label; claims no stronger than their proofs.
- `22-verifier-matrix.md` — one corruption fixture per semantic index
  the offline verifier claims to rebuild.

Phase F — terminal
- `23-census-close.md` — grep batteries, refusal-ledger verification,
  fingerprint pin check, full gate cash-out. Always last.

Dependency spine: 01 first (the docs PRDs cite its table). Phase B in
order 02 → 03 (03 consumes 02's types in evidence structs), 04/05 free.
Phase C after Phase B (avoid double-churn in the same files); 07 and 08
are the big sweeps and run solo. Phase D after Phase C (docs cite final
names). 23 last, always. (Numbering note: 18 is deliberately vacant —
the census moved from 18 to 23 when Phase E landed; nothing is
missing.)

## Policies

0. **The language law (final pass, 2026-07-13).** Whenever a concept
   has a dependency-theory or Datalog name, that name wins — hard
   cutover, no SQL vernacular. The binding table:
   | concept | the word | banned alternatives |
   |---|---|---|
   | left side of an FD | **determinant** | key index, guard, index entry |
   | `R(X) -> R` declaration | **functionality** (a functional dependency; a key when it determines the whole fact) | constraint, unique |
   | `A(X\|φ) <= B(Y\|ψ)` | **containment** (inclusion dependency over selected projected views) | foreign key |
   | query output tuple | **answer** | row, result row, record |
   | stored tuple | **fact** | row |
   | closed-relation element | **ground axiom** | row (except "roster row" in ledgers) |
   | replacing sealed atoms by their finite extensions | **grounding** | chase, closed fold, join elimination |
   | negation soundness | **range restriction / safety** | scoping |
   | finite value universe | **active domain** | — |
   "Row" survives only where it names a physical artifact (row stride)
   or an external system's concept (SQLite rows in the differential).
   PRD prose, code identifiers, diagnostics, and docs all obey; the
   census (PRD 23) audits the table.

1. **The fingerprint does not move.** `schema/fingerprint.rs` hashes
   declared statements and structural types only — sealed enforcement
   data is explicitly excluded (fingerprint.rs:9-16) and the pin test
   (`63e3b480…`) exists. Every PRD in this set must leave the pin test
   byte-untouched and green. A PRD that would change a hashed input is
   mis-scoped: stop and record.
2. **Pin first (behavior-preserving changes).** Representation and
   rename PRDs land their lock tests green against CURRENT behavior
   before the refactor line, and those tests are unchanged after.
   Semantic-lock tests live INSIDE the PRD they protect — there are no
   test-only PRDs in this set.
3. **No shims, no migrations, no compatibility aliases** except where a
   PRD explicitly names one. Rip to the end state. The tree need not
   typecheck between PRDs.
4. **Grep-zero renames.** A rename PRD's criteria enumerate the dead
   tokens; `grep -rn` across `crates/`, `fuzz/`, `scripts/`, and
   `docs/` (minus this packet's own ledger files) must return zero.
5. **Conflict protocol.** Where the spec and the current tree disagree,
   the reconciliation ledger below is authoritative. A disagreement
   discovered DURING execution that the ledger missed: stop, record in
   the PRD file, proceed only if the resolution is mechanical.
6. **Doc amendments ride the PRD.** Every code change lands with its
   architecture-doc delta in the same commit.
7. **The fuzz estate re-verifies for free.** PRDs that touch engine
   semantics (02, 03, 06, 09, 16, 17) must run a bounded fuzz smoke
   (`cargo fuzz run rewrites -- -runs=10000` and `ops -- -runs=10000`)
   before their commit; renames that touch feature names (07) must
   update `fuzz/Cargo.toml`, `scripts/check.sh`, and `scripts/fuzz.sh`
   in the same commit.

## Refusal ledger (decided now, binding)

- **`partitions`/`tiles` sugar — DEFERRED.** Pure macro lowering to
  the five explicit statements, fingerprint-identical. Trigger: after
  PRD 11's manual idiom + locks have lived one dogfooding cycle.
- **FD/key closure, superkey inference — REFUSED** (spec §9.4 agrees).
  Armstrong closure changes accepted schemas = a theory-version
  project, not a refactor.
- **`ScalarValue` split of `Value` — REFUSED.** The flat one-variant-
  per-type sum is deliberate (value.rs doc: "no universal integer").
- **`Rule` → `Clause` — REFUSED.** The recursion design paper and the
  whole architecture corpus speak "rules"; the spec's rename buys
  nothing the docs don't already disambiguate.
- **`Pack` → `Coalesce` internal rename — REFUSED.** The production
  names are `sweep`/`Continuation::maximal`; "pack" survives only in
  prose and one test helper, and `ir.rs:133` already names it the
  Snodgrass coalesce. Nothing to fix.
- **Empty-interval error-family consolidation — REFUSED.** The six
  variants live in six boundary enums (corruption, schema ×2,
  validation ×2, fact-shape) — per-site precision IS the taxonomy. The
  spec's complaint fit an older, genuinely-conflated shape.
- **Blanket `DetMap`/BTreeMap sweep — REFUSED.** The fingerprint is
  already canonical-ordered and pinned; EXPLAIN feeds from `Vec`s. The
  deterministic-output obligation lands as PRD 15's contract, not as a
  container religion.
- **Surface grammar renames (`->` → `key by`, `<=` → `contained_in`)
  — REFUSED.** The symbols stay; the docs name the concepts.
- **`UpperBound::{Finite, Unbounded}` + point newtypes (brief A1) —
  REFUSED.** The recorded stance stands: `[s, MAX)` over the point
  domain IS `[s, ∞)`; the ray is a value, not a mode, and every kernel
  is uniform because of it. The Lean bridge needs non-emptiness, which
  the `Interval` constructor supplies (PRD 02). A second representation
  of the same fact adds state.
- **`Query<Phase>` type ceremony + canonical normalized form (brief
  A3) — REFUSED, trigger recorded.** The witness pipeline already
  denies the planner unvalidated input. Canonical IR form gains a
  consumer only when a plan cache keys on IR — that is the trigger.
- **`ViewContainment` / `VarName`+`BinderId` renames (brief A2, part)
  — REFUSED/SUPERSEDED.** "Containment" stops colliding once PointIn
  lands; `VarId` is already the resolved binder and raw names already
  live only in the macro layer.
- **`ClosedFoldEvidence` ceremony (brief B2) — SUPERSEDED.** The
  dual-pipeline rewrites fuzz target IS the continuous differential
  proof that folding preserves denotation; introspect already renders
  the folded picture.
- **Recursion, C0–C5 — DEFERRED under the standing census-law
  refusal.** No sighting has fired the recorded trigger
  (20-query-ir.md § engine recursion — refused). The brief's ordering
  is ADOPTED into the trigger record: when it fires, the Lean
  fixed-point development (C0: immediate-consequence operator,
  monotonicity, finite stabilization, LFP, order-independence) precedes
  any Rust work, then the design paper's seam ledger executes. No
  reachability operator, ever.
- **D1–D5 (implication/countermodels, general TGDs/EGDs, predicate
  algebra, query-defined relations) — LATER**, per the brief's own
  gating: each is a theory-version decision after Phases A–C.

## Brief reconciliation (the Lean-alignment brief, audited 2026-07-13)

Item-by-item verdicts against this packet and current main. APPROVED
additions became PRDs 19–22 and the amendments noted in PRDs 03, 05,
10, 11, 12, 16, 17. Refusals joined the ledger above.

- A1 → PRD 02 (UpperBound/point-newtype sub-items refused, above).
- A2 → PRDs 06/07/08/10 + 13's glossary; the brief's `Functionality →
  Key` descriptor rename was briefly approved and then REVERSED by the
  language law (2026-07-13 final pass): `Functionality` IS the
  dependency-theory word and stays at the declaration layer; the sealed
  layer's `KeyStatement`/`KeyId` also stays (key is equally academic —
  Codd/Fagin vocabulary); diagnostics say "functionality (functional
  dependency)". `ViewContainment` and binder-split refused (above).
- A3 → SUPERSEDED (witness pipeline); canonical-form refused with
  trigger.
- A4 → PRD 03; `FieldSet`/`Projection` carriers APPROVED into PRD 03.
- A5 → PRD 14 (diagnostics already carry available keys; subset-key
  mention included).
- A6 → PRD 12; arity ≥3 composite locks APPROVED into PRD 12.
- A7 → PRDs 03+11; adjacency + composite-prefix locks APPROVED into
  PRD 11.
- A8 → SUPERSEDED + PRD 14's hostile locks.
- A9 → PRD 17 + existing DisjointWitness.
- A10 → SUPERSEDED (contracts documented, checked, exhaustively
  enumerated) + PRDs 13/16.
- A11 → PRD 05; `FinalStateView` seam APPROVED into PRD 05.
- A12 → PRD 15 + standing fingerprint law; container sweep refused.
- B1 → APPROVED, new PRD 19.
- B2 → SUPERSEDED (dual-pipeline fuzz differential; refusal above).
- B3+B4 → APPROVED merged, new PRD 20 (write/write_from already encode
  the witness classes in signatures — the PRD documents, classifies,
  and locks; no unsafe overload survives unlabelled).
- B5 → APPROVED as PRD 17 amendment (the bool-licensed-rewrite sweep).
- B6 → APPROVED, new PRD 22 (fixture-per-index matrix).
- B7 → APPROVED as PRD 16 amendment (the full capability matrix).
- B8 → APPROVED, new PRD 21.
- C0–C5 → DEFERRED under the census-law refusal (ledger above; the
  C0-before-Rust ordering adopted into the trigger record).
- D1–D5 → LATER (ledger above).

## Reconciliation ledger (audit claim × current-main verdict)

CONFIRMED (each owns a PRD): encoder `debug_assert!(start<end)`
(encode.rs:36,45) with raw `Value::IntervalU64/I64` (value.rs:37-39) →
02; coverage disjointness precondition carried in prose
(judgment.rs:604-653), boolean `coverage` flag, no proof object → 03;
`closed_member(&[u64;4], u64)` raw (schema.rs:386-391) → 04; bare-`u64`
generation + commit_seq → 05; `CmpOp::Contains` (ir.rs:260-269,
membership-only but name still collides) → 06; `plan/chase` + the
`chase-off` feature (~56 files) misleading a DB theorist → 07; the
`guard` word carrying four meanings (storage entry, width cap, probe
path, prepared-rule variant) + raw guard bytes → 08; IR says `Duration`
while every downstream name says measure → 09; `pitch`,
`image/distinct` (one of three "distinct"s), `OverflowKind::Origins`,
generic `IllegalComparison` for str/bool order → 10; cookbook recipes
15–17 call one-way coverage "tiling — no holes" (the Lean overshoot
countermodel refutes exactly this reading; no mutual-coverage idiom or
test exists anywhere) → 11; no reverse-key `==` rejection test; README
undersells `==` (bijection theorem unstated) → 12; matching equation /
equality levels / tuple-dedup contract unstated in one place → 13;
`NoMatchingTargetKey` carries no available-keys; no redundant-superkey
diagnostic; dictionary-miss emptiness invisible in EXPLAIN; param-only/
agg-only negated binders structurally foreclosed but unpinned → 14;
`exec/explain.rs:9-10` still says "stable-ish", no goldens → 15;
`ArgMax`/`ArgMin` renderable but absent from the `query!` grammar → 16;
`provably_distinct` returns a bare `bool` (provably_distinct.rs:25)
while its sibling disjointness proof has a typed `DisjointWitness` → 17;
no Lean artifacts in the repo → 01.

SUPERSEDED (no PRD): phase-separated `Query<Phase>` — the
`ValidatedQuery`/`RuleWitness`/`Predicate` witness pipeline already
gates planning and there is no public raw-plan path (build.rs:42-48);
Sum overflow — i128/u128 accumulation + finalize range check, documented
(20-query-ir.md:172); empty-global aggregate — zero rows, documented
(60-validation.md:91); negation-safety docs — 20-query-ir already
states positive range restriction, order-independent, and params never
enter `atom_vars`; uninterned-string "silent empty → Program::Empty" —
STALE: it is the literal latch, a live per-execution miss with an
`unresolved_literals` counter (only the EXPLAIN surfacing remains, in
14); typed statement arenas / `StatementRef` / `KeyId` — already landed;
rejection completeness — the violations refactor already made
rejections the complete violation set; rule-disjointness evidence —
`DisjointWitness` already exists.
