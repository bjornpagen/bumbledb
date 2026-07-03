# PRD 07 — Distinct-count statistics and planner honesty

Authority: `30-execution.md` (the STATS phase, the DP), suite README
(estimate dishonesty: worst est/actual 114,679× on fk_walk, 4,762× on string,
102× on balance). Depends on PRDs 00/02 (selections exist to be estimated).

## Purpose

The DP orders joins and the report judges honesty with estimates that ignore
selection selectivity entirely — a `memo = ?0` occurrence is estimated at the
full relation count. Give the planner real selectivities from three sources,
strongest first: schema structure (free and exact), resident image distinct
counts (exact when available), documented constants (the honest floor).

## Technical direction

- **Per-column distinct counts on images.** `RelationImage` build
  (`image.rs` / `image/`): while columns are decoded, count distincts —
  word columns through a reused scratch word-set (the `wordmap` machinery, or
  a sort-based pass over the column slab if simpler — build is the cold path;
  the 2.4 ms S-scale build may grow ≤ 50 %, and the `image_build` span makes
  the cost visible); byte columns (bool/enum) through a 256-bit bitmap.
  Expose `RelationImage::distinct(field: FieldId) -> u64`.
- **Stats acquisition at prepare.** The STATS phase reads relation row counts
  from the storage counters today. Extend it with per-field distinct counts,
  resolved in priority order:
  1. **Schema-derived, always available:** a field under a single-field unique
     constraint has `distinct == rows`; a single-field FK's distinct is
     ≤ the target relation's row count (use it); an enum/bool field's distinct
     is ≤ its variant count.
  2. **Image-resident exact:** if the image cache holds the relation at the
     current generation, read `image.distinct(field)`. Peek only — **prepare
     must never trigger an image build** (add `ImageCache::peek` if only
     `get_or_build` exists; a cold prepare falls through to 3).
  3. **The documented floor:** `DEFAULT_EQ_DISTINCT = 64` (i.e., an unknown
     Eq selection keeps `rows / 64`), `DEFAULT_RANGE_SELECTIVITY = 1/4` for
     residual range predicates, `DEFAULT_NE_SELECTIVITY = 1` (Ne filters
     almost nothing). Constants live in one `mod selectivity` with the
     rationale on each.
- **Estimation rule.** An occurrence's estimated cardinality =
  `rows × ∏ per-selection (1 / distinct(field)) × ∏ per-residual selectivity`,
  clamped to `[1, rows]`. Feed it wherever the DP consumes per-occurrence
  cardinalities today, and into `NodeStats::estimate` so `profile()`/EXPLAIN
  and the bench report's est/actual factor measure the *new* model.
- Determinism: estimates must be a pure function of (schema, counters,
  resident-image set). The resident-image set varies by warmth — acceptable
  and documented: prepare-time estimates are best-effort; runtime cover choice
  (PRD 06) is the load-bearing decision. The pinned tests below control
  warmth explicitly.

## Non-goals

Histograms, sketches, sampled statistics, cross-column correlation. Persisting
distinct counts to storage (images are the cache; counters stay as they are —
**no storage-format change, no migration**). Estimating join output sizes
(the DP's structural cost model is out of scope; this PRD fixes *input*
cardinalities only).

## Passing criteria

- Unit tests for the resolution ladder: a unique field reports
  `distinct == rows` with a cold cache; an enum field reports ≤ variants; a
  plain string field reports the floor when cold and the exact image count
  when the image is resident (test both by controlling the cache).
- Image distinct counts are exact: hand-built image with known cardinalities
  per column type (words, biased i64, interned strings, bytes, bool, enum) —
  `distinct()` equals the ground truth for each.
- **Honesty pins over the pinned S corpus** (deterministic: pinned corpus
  digest + warmed images + fixed plans): run `profile()` for each of the eight
  read families with all images resident and assert the worst per-node
  est/actual factor: ≤ 16 for point, string, fk_walk, balance; ≤ 64 for chain,
  range, stats, skew. (Today's values: 114,679 / 4,762 / 102 — these pins are
  the "for good" part. If a pin fails after a legitimate generator change,
  re-derive and re-pin deliberately, exactly like a corpus digest.)
- Zero-warm-allocation: distinct counting uses reused scratch inside the
  image-build window (already sanctioned); prepare allocates as before.
- `scripts/check.sh` green.
