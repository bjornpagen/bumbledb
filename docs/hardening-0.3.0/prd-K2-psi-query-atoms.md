# PRD-K2 — ψ query atoms: `.match`/`not` over closed relations

Wave K · Repo: bumbledb `ts/` · depends on: K1 · blocks K7

## Objective

Close Hole B: closed relations become matchable in queries —
`r.match(Kind, { id: r.var("k"), mastered: true })` — so recipe 7/8's forced
rule-union inversion dies. The engine already handles closed atoms end to end:
prepare folds resolvable closed atoms into plan-constant id-sets attached as
`WordSet` memberships (`crates/bumbledb/src/plan/ground/evaluate.rs`); negated
closed atoms fold to the complement (domain-witness guarded); empty sets kill
the rule; unfoldable shapes (payload escaping to the head, param-bearing
filters) fall back to a join against the L1-resident virtual image — always
correct. Only the TS surface refuses.

## Work

1. **`ts/src/query/lower.ts`**:
   - `QueryRelation<Rels>` (~line 87) currently
     `Extract<Rels[keyof Rels], AnyRelation>` — widen to
     `AnyRelation | AnyClosed`.
   - Add `MatchFields<R>`: for a closed relation,
     `{ id: ClosedIdField } & Cols` (the payload columns from the typed
     `columns` carrier). Under option 2 the closed id's domain is the
     K4-computed class `` `${Name}.id` `` read off the SCHEMA type — var
     domain-typing, `JoinOk`, literal tagging (`taggedHandleId`), and result
     decoding flow through the class map with no further changes to those
     layers.
   - `lowerAtom` (~line 1351): one closed-owner branch — `id` → ordinal 0,
     payload column → declared index + 1 (the sealed shift, mirroring
     `spec.rs`'s resolver). Widen `AtomData.relation` accordingly.
   - Extend `not(...)` the same way in the same pass (the engine's complement
     fold exists; the SDK's negation typing rules — boundness, the safety
     wall — apply unchanged to closed atoms).
2. **Probes** (intrinsic, `ts/test/`):
   - compile-PASS: match on closed with a payload literal; payload column
     bound to a var whose domain joins another atom's same-domain field;
     negated closed atom; handle literal in the id position.
   - compile-FAIL (real): unknown payload column; payload var joining a
     different-CLASS field (per K4's class map); a closed atom var reused
     cross-class (the two-tier join wall must hold here identically).
   - lowering golden: a `lowerQuery` output pin showing ordinal 0 for id and
     +1-shifted ordinals for payload columns, for both positive and negated
     atoms.
   - runtime equivalence: one prepared query using a closed atom returns
     row-for-row the same result as the old rule-union spelling over the same
     store (this is a unit-shaped correctness assertion, not an e2e suite —
     one store, one comparison, in the existing test style).

## Technical direction

- No SDK-side folding, no SDK knowledge of WHETHER the engine folds or
  virtual-image-joins — transparency is the contract. One cookbook sentence
  (K7's job) documents the fold limits (payload-to-head doesn't fold;
  param-bearing filters don't fold in v0); the SDK stays oblivious.
- The `columns` carrier is the single source for payload typing — no parallel
  column table.
- Zero casts; runtime twins for every type claim (the ordinal shift is pinned
  by the lowering golden, not trusted).

## Passing criteria

- All probes green; compile-fail directives real.
- The lowering golden pins the ordinal mapping for id/payload/negated cases.
- The runtime-equivalence assertion passes.
- `tsc --noEmit` green for touched modules + probes; `biome` clean on touched
  files; zero casts in the diff. Push per the wave's commit discipline.
