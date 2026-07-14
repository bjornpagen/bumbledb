# PRD 06 — The sweep, proved: coverage and Pack under the disjointness premise

**Depends on:** 05 (pack, points), 03 (Coverage).
**Modules:** `lean/Bumbledb/Exec/Sweep.lean`, `Countermodels.lean`.
**Authority:** the engine's one shared sweep (`interval/sweep.rs`
drives BOTH `check_coverage` and Pack's coalesce — the reuse the docs
brag about becomes a proved fact); the `DisjointDeterminantProof`
token, whose obligation this PRD turns into a named theorem.
**Representation move:** Level 1 begins. The first algorithm proved
equal to its denotation — and the first place a Rust witness type gets
its theorem.

## Context (decided shape)

Definitions (the algorithmic essence per the mechanism fence — a
sweep is a fold, nothing else):
- `sweepCovered : Interval α → List (Interval α) → Bool` — the
  forward one-pass frontier walk over a segment list: start at the
  source's start, each segment must touch-or-overlap the frontier and
  advance it; covered when the frontier passes the source's end.
  Mirror the Rust control shape (predecessor entry, advance,
  early-exit) at the fold level.
- `Ordered`, `Disjoint` — the premises: the segment list is start-
  sorted and pairwise point-disjoint (exactly what a pointwise key
  guarantees per prefix group — 03's `pointwise_key_disjoint`).
- `sweepPack : List (Interval α) → List (Interval α)` — the same fold
  emitting maximal runs (05's `pack`, now shown to BE this fold).

Theorems:
1. `sweep_covered_sound_complete` — under `Ordered ∧ Disjoint`:
   `sweepCovered src segs = true ↔ points src ⊆ ⋃ points segs`. THE
   `DisjointDeterminantProof` theorem: the premise is precisely the
   token's meaning (Bridge row: `DisjointDeterminantProof` +
   `judgment.rs::check_coverage`).
2. `sweep_premise_load_bearing` — a COUNTERMODEL: an unordered or
   overlapping segment list where `sweepCovered` returns a wrong
   verdict (both directions if constructible: false-accept and
   false-reject). This is the audit's "wrong verdict without erroring"
   made concrete — and the formal justification for the verifier's
   `pointwise_overlap_is_found_by_the_ordered_walk` fixture (Bridge
   row).
3. `sweep_early_exit_sound` — the frontier-passes-end early exit
   loses nothing (the Rust optimization's licence).
4. `pack_is_the_sweep` — `sweepPack = pack` (05's spec function): one
   fold, two consumers — the code-sharing claim proved.
5. `ray_needs_ray` — a source ray is covered only if the segment list
   reaches a ray (the "coverage to ∞" doc claim, now a lemma).
6. `adjacent_segments_cover` — touching segments cover across the
   seam (half-open composition).

## Technical direction

State over an abstract linearly-ordered element domain with a ceiling
(02's shape). The fold is structural recursion on the segment list —
keep it under ~30 lines; if the Rust's two-phase entry (predecessor
probe then walk) resists a clean fold, model the walk from the
predecessor as the fold's initial frontier and record the seam. The
countermodel (item 2) is REQUIRED — a Level 1 module without its
premise countermodel has not earned the witness type it justifies.

## Passing criteria

- `[shape]` All six theorems + the countermodel checked; zero
  sorry/axioms; `scripts/lean.sh` 0.
- `[shape]` `sweepCovered`'s correctness theorem carries `Ordered ∧
  Disjoint` as hypotheses (grep the statement — the premise must be
  visible, not baked into a subtype).
- `[shape]` The module doc names the Rust consumers (`check_coverage`,
  the Pack finalize) and the witness (`DisjointDeterminantProof`).
- `[gate]` CI green.

## Doc amendments

None yet — PRD 12 thins `40-execution`'s sweep prose against these
names.
