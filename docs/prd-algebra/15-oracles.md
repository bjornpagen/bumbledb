# PRD 15 — Oracles and the generator

**Depends on:** 03, 05, 10, 12 (translates and generates what they built);
06/08/09 covered by construction (they are semantics-preserving over 05's
shape).
**Modules:** `crates/bumbledb-bench/src/` (IR→SQL translator, naive model,
querygen), `bumbledb-bench` verify machinery.
**Authority:** `60-validation.md` (the two-oracle construction is normative;
no timing without the stamp).
**Representation move:** none — this PRD is the tax the axioms charge. Every
new representation must be *judged* before it is *timed*; the oracle surface
grows to match the algebra or the algebra does not exist.

## Context (decided shape)

Coverage obligations, one per landed representation:

- **Rules (05–08):** IR→SQL emits one `SELECT DISTINCT` per rule joined by
  `UNION` (set-semantic union — SQLite's `UNION` is exactly ∪ under
  `DISTINCT` discipline). The naive model evaluates rules directly (union of
  per-rule binding sets — it must *not* share the engine's sink mechanics).
  Generator: rule counts 1–4, overlapping and provably-disjoint shapes (DU-arm
  unions specifically — the elision path must be exercised adversarially),
  duplicate head rows across rules (the union's teeth).
- **DNF (06):** the naive model evaluates the *input tree* directly; the
  engine evaluates the lowered rules — the differential is the lowering proof.
  Generator: random predicate trees to depth 3, including cap-exceeders
  (verdict: both sides reject — error parity, not just result parity).
- **Allen (03–04):** translator emits endpoint-comparison SQL per basic
  (masks disjoin the basics' SQL under `OR` — SQLite is bag-semantic
  internally but `DISTINCT` restores ∪). Generator: all 13 singletons, the
  named composites, random masks, the converse property (swap operands,
  converse the mask, assert equal results), ray-bearing corpora.
- **Measure (10):** `Duration` → `(end - start)` on the two stored columns in
  the SQLite mirror schema; `MeasureOfRay` parity: the naive model raises the
  same typed verdict (error-verdict comparison, the PRD-02 pattern from the
  direction-divergence finding).
- **`Pack` (12):** SQLite cannot express it — **naive model only**, from the
  point-set definition. Optional golden: a hand-derived fixture family
  (documented, not automated against Postgres — no new bench dependencies,
  the quarantine law).
- **The str-extrema roster check** (README refusals): verify `Min`/`Max` over
  str/bytes is rejected at validation; add the roster rejection if absent —
  intern words are not order-preserving, and today's behavior would return
  dictionary-id extrema.

## Technical direction

1. Translator: rules→UNION, masks→endpoint SQL, Duration→arithmetic; the
   mirror schema carries interval columns as the two words it already stores.
2. Naive model: rules, trees, masks (via point sets), measure, pack — all from
   definitions, zero shared algorithm with the engine (the independence law).
3. Generator: the coverage families above, seeded, with the boundary-shape
   ladder (adjacent/nested/equal/ray) systematized for every interval draw.

## Passing criteria

- `[test]` `bumbledb-bench verify` green across all new families at the
  standard N; the stamp regenerates.
- `[test]` Error parity: cap-exceeding DNF, vacuous masks, `MeasureOfRay` —
  engine and naive verdicts identical including the typed identity.
- `[shape]` The naive model shares no lowering/kernel/sweep code with the
  engine (module dependency check — the independence that makes it an oracle).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`60-validation.md`: the new families, the error-parity discipline, `Pack`'s
naive-only assignment.
