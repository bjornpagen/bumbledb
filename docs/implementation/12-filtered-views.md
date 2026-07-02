# PRD 12 — Filtered Views (Scalar)

Authority: `docs/architecture/30-execution.md` (per-atom filters → survivor views;
cold dual-output build), `40-storage.md` (views query-local, never cached).

## Purpose

Per-atom filter evaluation producing survivor-position vectors over images.

## Technical direction

- `image::view`. `FilterPredicate` — the lowered per-atom filter form produced later by
  PRD 15: (field, op, constant-word) triples plus same-fact field-equality pairs;
  defined here as a plain struct so this PRD is self-contained.
- `View { image: Arc<RelationImage>, survivors: Vec<u32> }` (u32 positions — 10⁷ scale
  axiom; debug_assert row_count < u32::MAX), arena-backed via a caller-supplied arena
  (PRD 06's bump arena type).
- `apply(image, &[FilterPredicate], &mut arena) -> View`: scalar loop over positions
  evaluating the conjunction on column words; **branchless survivor write** (cursor +
  conditional increment, no if-push) — the NEON version replaces this inner loop in
  PRD 22 behind the same signature.
- **Cold dual-output**: `build_with_filters(&ReadTxn, rel, &[FilterPredicate], arena)`
  — one scan producing the full image (for the cache; caller inserts it) *and* the
  survivor view, without a second pass (evaluate predicates per row during decode).
- An unfiltered "view" is represented as `survivors: None` (all positions) — a
  two-variant enum, not a sentinel vector.

## Non-goals

Comparison semantics/validation (PRD 14 owns legality; this executes pre-validated
predicates). String predicate resolution (ids are already words by here). Caching
views (never).

## Passing criteria

- Unit tests: conjunction over mixed 1/8-byte fields matches a naive per-row decode
  filter oracle written in the test; same-fact field-equality pairs work; empty
  survivor set; all-survivors unfiltered variant; dual-output build's image is
  byte-identical to PRD 10's build of the same relation and its view equals `apply` on
  that image; branchless loop verified by inspection comment (no `if` in the inner
  loop body).
- Global commands green.
