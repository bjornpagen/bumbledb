# PRD-K7 — The SDK cookbook rewritten + the cross-host lock extended

Wave K · Repo: bumbledb `ts/` (+ `ts/crate` for the lock twin) · depends on:
K1, K2, K3, K4, K5, K6

## Objective

`ts/COOKBOOK.md`'s 29 recipes become the canonical teaching surface for the
0.3.0 idioms — ψ statements and closed atoms, `ref`/`cites`/coordinates,
`vars` + free comparisons, `Kind.match`, the 3-arg `closed`, the M1 `key`
spelling — with the compile-pin updated so none of it can rot. The CrossHost
fingerprint lock gains the two construct classes it currently lacks (ψ on a
closed target; a ref-derived containment), pinned to the same new constant on
BOTH sides of the FFI.

## Work

1. **Recipes 7/8 — the ψ rewrite** (the headline): the complement-exclusion
   inversion dies. One ψ statement
   (`contained(on(Certificate,"kind"), on(Kind.where({mastered:true}),"id"))`,
   recipe 8 with `Severity.where({pages:true})`) replaces the plain
   containment + hand-folded `window(..., none, ...)` pair; the read side uses
   a closed atom (`r.match(Kind, { id: k, mastered: true })`) instead of the
   rule union. DELETE the "surface finding" notes those recipes carry — the
   gap is closed. ADD the two honesty sentences: (a) the fold limits (payload
   escaping to the head and param-bearing filters don't fold at prepare; the
   engine falls back to a virtual-image join — semantics identical), (b) for
   an ALREADY-DEPLOYED store, moving from the workaround to ψ is a NEW theory
   (different fingerprint) — recipe-28 ETL territory; humans own that.
2. **`ref`/`cites`/coordinates across the set**: every recipe whose simple
   containments are derivable (56 of 67 at last count) adopts `ref` and drops
   the hand statement (recipes are NEW-store teaching text — fingerprint
   motion is fine and invisible here); the Calendar recipe adopts
   `cites(Attendance, "id")` beside its selected `mirrors` and gains one
   sentence explaining WHY (`ref` would derive a statement the theory
   deliberately refuses). Remaining `.as` uses are exactly the legitimate
   shared-value-domain cases; each keeps a half-line justification.
3. **Ergonomics sweep**: `vars()` destructuring + shorthand punning wherever a
   rule binds ≥2 vars; free comparisons in `.where`; `Kind.match` where a
   recipe dispatches on handles; the 3-arg `closed` everywhere (the curried
   spelling no longer compiles).
4. **The `key` spelling**: unchanged — the canonical render stays the
   dependency-theoretic arrow (`R(a, b) -> R`, M1 owner ruling); the TS
   `key(R, [...])` free function is the host flavor. Verify the COOKBOOK's key
   statements render to the arrow via `renderStatement` output, not by hand.
5. **The compile-pin** (`ts/test/cookbook.test.ts`): update every recipe's
   pinned construction to the rewritten text — the pin mechanism itself is
   unchanged (recipes are constructed through the public surface; a COOKBOOK
   edit fails the pin). The reopen-stability assertion stays.
6. **T5 fixture regeneration**: rewritten recipes move fingerprints (7/8 by ψ;
   ref-adopting recipes where the derived tail order differs from the old
   hand order). Regenerate via the T5 flag; the Rust cookbook twins (M3's
   suite) must agree — if `docs/cookbook.md` recipes 7/8 still spell the old
   workaround, rewrite THEM to the macro's ψ spelling in the same commit (the
   engine cookbook already supports it: `Kind(id | mastered == true)`), so
   the two cookbooks teach one theory and T5 stays green.
7. **The CrossHost lock**: extend the composite theory in BOTH
   `ts/crate/src/fingerprint_lock.rs` (macro side) and
   `ts/test/fingerprint.test.ts` (SDK side) with a ψ-on-closed statement and a
   ref-derived containment; compute the new fingerprint once, update the
   pinned constant to the SAME hex in both files, and keep the cross-admission
   + twisted-twin assertions green.

## Technical direction

- The recipes must actually be BETTER text after the sweep, not mechanical
  substitutions — read each recipe's prose and update claims the new idioms
  falsify (e.g. any sentence describing hand-folding, string-repeated vars, or
  the curried closed).
- Never let the two cookbooks (engine `docs/cookbook.md`, SDK `ts/COOKBOOK.md`)
  teach different theories for the same recipe number — T5 is the referee;
  keep it green rather than editing around it.
- Guarantee lines citing lean theorem names are load-bearing — do not touch
  the citations except where `scripts/spec-census.sh` proves a rename.

## Passing criteria

- Zero old idioms in `ts/COOKBOOK.md` (grep set: the complement-window
  workaround comment, `r.var("` where a `vars()` destructure binds multiple,
  the curried `closed(`…`)(`, hand containments that a `ref` in the same
  recipe derives, "surface finding" notes in 7/8).
- `cookbook.test.ts` green against the rewritten text; the T5 fixture
  regenerated and BOTH sides green (TS + the Rust cookbook fingerprint test).
- The CrossHost lock: new constant identical in both files; both suites green;
  cross-admission both directions still passes.
- Every remaining `.as` in the COOKBOOK carries its justification sentence.
- `pnpm exec biome check .` clean on touched files. Push per the wave's
  commit discipline.
