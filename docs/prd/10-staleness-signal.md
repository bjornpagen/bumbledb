# PRD 10 — Plan staleness signal (host-owned, zero hot-path cost)

**Depends on:** nothing.
**Modules:** `crates/bumbledb/src/api/prepared/` (`build.rs`, the prepared-query
struct, `api/stats.rs`), `crates/bumbledb/src/storage/read/` (`row_count`).
**Authority:** `20-query-ir.md` (pin-at-prepare decision), `00-product.md`
(host owns policy; the engine owns zero threads), `70-api.md`.

## Context (decided shape)

Plans pin statistics at prepare and are never invalidated — correct for the
write design point and correctness-safe (generational rebinding means stale
plans read current data; only optimality drifts). The gap: nothing tells the
host optimality has drifted; the classic failure is a good plan pinned against
data that grew past it, discovered as a latency incident. The transfer is the
*signal only*: pull-based, engine-policy-free, never called by the engine.
Explicitly rejected and recorded: re-planning on execute (hot-path branch for a
policy question; silent replans make performance non-reproducible), engine-side
thresholds, background anything.

## Technical direction

1. **Pin record:** `PreparedQuery` records, per occurrence, the row count the
   plan was costed with and, where a filtered view was measured at prepare, the
   survivor count — data `api/prepared/build.rs` already holds and drops.
   Store as a small boxed slice on the prepared query (cold data; not touched
   by execution).
2. **The signal:**
   ```rust
   pub fn staleness(&self, snap: &Snapshot<'_>) -> Staleness
   ```
   compares pinned row counts against the snapshot's live `S` counters
   (`read::row_count`, one O(1) LMDB get per occurrence; ≤20 by the roster
   cap). `Staleness { per_occurrence: Box<[OccurrenceDrift]>, max_ratio: f64 }`
   with `OccurrenceDrift { relation, pinned, live, ratio }`. Ratio convention:
   `max(live, pinned) / max(1, min(live, pinned))` so shrink and growth both
   read as drift ≥ 1. **No thresholds in code.** The suggested convention
   (re-prepare at ≥4×, the worst measured est/actual being 3.3×) is one
   sentence of rustdoc, explicitly labeled a convention.
   It allocates; it is not a warm-path call; document both facts.
3. **Foreign-snapshot discipline:** `staleness` checks the environment instance
   id exactly like `execute` does (`ForeignPreparedQuery` on mismatch) — same
   guard, same error.
4. **Stats/EXPLAIN fold:** the est-vs-actual report gains "estimated from
   (pinned rows at prepare)" per occurrence so a drifted plan is visible in one
   read of the existing surface.

## Passing criteria

- `[test]` Prepare at N rows, commit until ~4N, `staleness` reports the ratio
  per occurrence and the max; re-prepare resets it; a shrunk relation also
  reports ratio > 1.
- `[test]` Foreign snapshot returns `ForeignPreparedQuery`.
- `[shape]` No engine code calls `staleness`; no threshold constant exists;
  execution paths are untouched (the pin record is written at build, read
  never) — the alloc gate and its escalating variant (PRD 04) pass unchanged.
- `[test]` The stats surface carries the pinned-rows field (golden on one
  EXPLAIN report).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`20-query-ir.md` pin-at-prepare paragraph gains the signal as its compensating
control; `70-api.md` documents `staleness` and the convention sentence.
