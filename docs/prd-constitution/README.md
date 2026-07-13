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
- `04-member-set.md` — `MemberSet` + `RowIndex` replace the raw
  `[u64; 4]` + free-function membership.
- `05-generation-id.md` — `GenerationId` + `CommitSeq` newtypes over
  the bare `u64`s.

Phase C — one word, one meaning (the vocabulary sweep)
- `06-point-in.md` — `CmpOp::Contains` → `CmpOp::PointIn`.
- `07-closed-fold.md` — `plan/chase` → `plan/closed_fold`; the
  `chase-off` feature → `closed-fold-off`; the docs stop implying the
  dependency-theory chase.
- `08-key-index.md` — the storage `guard` vocabulary → `key_index`;
  raw guard bytes → `KeyImage`; `GuardRule`/`GuardPlan` → the
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
  tuple-level dedup, glossary).
- `14-diagnostics.md` — `NoMatchingTargetKey` carries the available
  keys; redundant-superkey warning; unresolved-literal visibility;
  negated-binder lock tests.
- `15-explain-contract.md` — EXPLAIN goes from "stable-ish" to a
  versioned deterministic contract with goldens.
- `16-arg-grammar.md` — `ArgMax`/`ArgMin` enter the `query!` grammar;
  the render/parse asymmetry dies.
- `17-distinct-witness.md` — `provably_distinct` returns a typed
  witness, not a `bool` (the `DisjointWitness` precedent).

Phase E — terminal
- `18-census-close.md` — grep batteries, refusal-ledger verification,
  fingerprint pin check, full gate cash-out.

Dependency spine: 01 first (the docs PRDs cite its table). Phase B in
order 02 → 03 (03 consumes 02's types in evidence structs), 04/05 free.
Phase C after Phase B (avoid double-churn in the same files); 07 and 08
are the big sweeps and run solo. Phase D after Phase C (docs cite final
names). 18 last, always.

## Policies

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
