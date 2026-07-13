# PRD 11 — Exact partition: the cookbook stops calling a cover a tiling

**Depends on:** Phase C complete (docs cite final vocabulary).
**Modules:** `docs/cookbook.md` (recipes 15-17 + the § "Time and
tilings" heading at :451, the "disjoint + covering = a tiling — no
overlaps, no holes" claim at :537-538, and the tile-at-write prose),
`crates/bumbledb-query/tests/cookbook.rs` (compiled copies + roster),
`docs/architecture/30-dependencies.md` (coverage direction prose),
`crates/bumbledb/src/schema/tests/` (the new locks).
**Authority:** the Lean overshoot countermodel
(`overshoot_isTiling_not_exact`: domain `[0,10)`, tile `[0,20)` — the
one-way form holds while point 15 escapes the domain) and the deep
audit's corrected §3.5: one coverage direction + a pointwise key is a
DISJOINT COVER of the source (target overhang legal); EXACT PARTITION
needs the reverse containment too, and it IS expressible — five
ordinary statements — but no recipe, no test, and no doc says so today.
The cookbook currently teaches the over-read verbatim.
**Representation move:** none in the engine. The cookbook's claims
become exactly as strong as the theorems.

## Context (decided shape)

1. **Vocabulary correction, cookbook-wide:** the one-way idiom is a
   "disjoint cover" (no overlaps within the cover, no holes over the
   SOURCE; the covering side may overhang). Recipe 16 retitles from
   "Tilings" to "Disjoint covers"; the § heading follows ("Time and
   coverage"); every "no holes" claim names its direction ("no holes
   in the fiscal year's span; pay periods may extend beyond it —
   overhang is legal under this statement"). Recipes 15 and 17's
   "tile"/"TILE" wording follows the same correction.
2. **New recipe: "Exact partition"** (roster 25 → 26) — the
   five-statement idiom on the audit's Policy/Version shape:

   ```text
   Version(policy)        <= Policy(id);             // FK for intent
   Version(policy, valid) -> Version;                // pointwise key
   Policy(id, live)       -> Policy;                 // pointwise key
   Policy(id, live)       <= Version(policy, valid); // no gaps
   Version(policy, valid) <= Policy(id, live);       // no overhang
   ```

   The recipe text derives WHY each statement is load-bearing (the
   explicit `Policy(id, live) -> Policy` exists because containment
   targets resolve by EXACT field-set — the `{id}` fresh key does not
   satisfy a `{id, live}` projection; no closure is inferred), names
   what the pair proves (equal point supports per group + disjointness
   = genuine partition), and cross-references the Lean theorem row
   (`exactTiling_iff_exactPointPartition`).
3. **The locks** (schema/commit tests, in this PRD):
   - the five-statement schema VALIDATES (both coverage directions
     accepted, two distinct pointwise keys coexist beside the fresh
     `{id}` key);
   - a gap-only violation rejects citing the forward statement;
   - an overhang-only violation rejects citing the reverse statement
     (the audit's decisive pair: source `[0,10)` / target `[0,20)`
     REJECTED here, ACCEPTED under the one-way recipe — both pinned);
   - the one-way overhang-acceptance pin (recipe 16's semantics,
     asserted so the correction can never silently strengthen it).
4. `30-dependencies.md` § coverage: one added paragraph stating the
   direction law and the exact-partition conjunction, citing the
   recipe and the table row. The refusal ledger note: `partitions`
   sugar deferred (README of this packet).

## Technical direction

Cookbook house format: doc block + token-identical compiled copy +
sync test; the compiled recipe drives real data through commit
judgment for all four lock cases (accept / gap / overhang / one-way
overhang-accept). Hand-compute the point sets in comments. Keep recipe
numbering appended; update the roster count and repo README recipe
line.

## Passing criteria

- `[shape]` `grep -rni "tiling\|tile" docs/cookbook.md` → only inside
  the new recipe's "why this is not mere tiling language" explanation
  and the corrected historical note (each surviving hit listed in the
  commit body); `grep -rn "no holes" docs/cookbook.md` → every hit
  names its direction.
- `[test]` Cookbook suite green at roster 26; the four locks green
  with hand-computed values; full schema/commit suites green.
- `[shape]` `30-dependencies.md` carries the direction law; the
  theorem↔evidence partition row cites the recipe.
- `[gate]` Fingerprint pin untouched (new schema statements exist only
  in tests); clippy; fmt.

## Doc amendments (rule 6)

This PRD is its amendments; README recipe count follows.
