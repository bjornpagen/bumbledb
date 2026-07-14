# PRD 08 — The rewrites, proved: grounding, key probes, static emptiness

**Depends on:** 04 (denotation), 03 (keys, ground axioms).
**Modules:** `lean/Bumbledb/Exec/Rewrites.lean`, `Countermodels.lean`.
**Authority:** the three prepare-time rewrites whose continuous
empirical verification already exists (the rewrites fuzz target's
dual pipeline) and now get their theorems: grounding, the key-probe
plan, and statically-empty folds. The rewrites target remains the
empirical arm; this module is the formal arm of the same claim —
"rewrites are semantics-preserving" — from two independent directions.
**Representation move:** Level 1 for plan rewrites.

## Context (decided shape)

Definitions:
- `groundAtom : Atom → GroundExtension → List Assignment` — replacing
  a sealed (closed) atom by its finite satisfying extension under the
  current partial assignment (grounding's essence: substitution
  against ground axioms).
- `groundRewrite : Rule → Rule ⊕ Grounded` — the pass abstractly: a
  closed atom either folds to a finite constant contribution, an
  eliminated atom (its satisfaction proved), or a refutation
  (statically empty).
- `KeyProbeShape r` — the shape the engine lowers to a point probe: a
  rule whose atoms resolve through one determinant lookup (read
  api/prepared's KeyProbe lowering first; model the shape it actually
  accepts).
- `StaticallyEmpty r` — a rule whose conditions refute against sealed
  constants (the fold's kill rule).

Theorems:
1. `grounding_preserves_answers` — `ruleAnswers (groundRewrite r) =
   ruleAnswers r` on every instance agreeing with the theory's ground
   axioms (closed extensions are instance-invariant — 03's sealed
   constants; Bridge: plan/ground, the `ground-off` dual pipeline).
2. `elimination_sound` — an atom eliminated by grounding contributes
   no constraint: dropping it preserves answers under the theory's
   containment premises (the `Role::Eliminated` law — read the
   grounding evaluator's elimination rule and model exactly it;
   Bridge: `Role::Eliminated(statement)` evidence).
3. `keyprobe_equiv_join` — under `KeyProbeShape` + the key's
   uniqueness (03's `functionality_unique_witness`), the point-probe
   evaluation equals the join denotation (Bridge:
   `PreparedRule::KeyProbe`).
4. `statically_empty_sound` — a refuted rule contributes the empty
   answer set on EVERY instance (Bridge: `Program::Empty` and the
   fold-death records); plus the latch note: an UNRESOLVED literal is
   not static emptiness — model the distinction as two constructors
   (the miss is per-instance emptiness of one selection, the fold is
   instance-independent refutation) so the latch's design decision is
   structural in the model.
5. `rewrite_composition` — the three rewrites compose: any sequence
   preserves `queryAnswers` (the prepare pipeline's licence to chain).
6. Countermodel: `elimination_needs_containment` — dropping a
   containment-backed atom WITHOUT the containment premise changes
   answers (why elimination consults the theory, not just the shapes).

## Technical direction

Read the Rust first, model second (items 2 and 3 depend on the actual
accepted shapes — the module doc records the reading with file
anchors). Reuse 04's `evalList` machinery for the grounded finite
contributions. Elementary proofs; the composition theorem (5) should
fall out of 1–4 by rewriting — if it needs real work, the individual
statements are mis-shaped (fix them, don't grind).

## Passing criteria

- `[shape]` All six items checked; zero sorry/axioms;
  `scripts/lean.sh` 0.
- `[shape]` The latch distinction (item 4) is two constructors, not a
  comment (grep the type).
- `[shape]` Module doc records the Rust readings (file:line anchors
  for the elimination rule and the KeyProbe shape).
- `[gate]` CI green.

## Doc amendments

None yet — PRD 12 thins the grounding/key-probe semantic prose
against these names.
