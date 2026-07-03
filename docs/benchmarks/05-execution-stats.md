# PRD 05 — ExecutionStats and cache/storage statistics surfaces

Authority: `30-execution.md` (EXPLAIN owns the mechanism), `60-api.md`
(observability surface), PRD 02.

## Purpose

The bench report needs *structured* per-execution statistics (estimates vs actuals,
cover choices, probe hit rates, skips, batching) and store-level numbers (cache
state, image bytes, db file size) — not a rendered string.

## Technical direction

- Promote the counting data to a plain public struct in `api/prepared.rs` (or a new
  `api/stats.rs`): `pub struct ExecutionStats { pub nodes: Vec<NodeStats>, pub
  emits: u64, pub guard: Option<GuardStats> }`, `pub struct NodeStats { pub entries:
  u64, pub batches: u64, pub batch_entries: u64, pub estimate: u64, pub actual:
  u64, pub covers: Vec<CoverStats>, pub residual_pass: u64, pub residual_fail: u64,
  pub skips: u64 }`, `pub struct CoverStats { pub subatom: usize, pub chosen_exact:
  u64, pub chosen_estimate: u64, pub probes_hit: u64, pub probes_miss: u64, pub
  hashes: u64 }`, `pub struct GuardStats { pub hit: bool }`. Built from
  `CountingCounters` by a conversion the explain path shares.
- `Snapshot::profile(&self, prepared, params) -> Result<(ResultBuffer,
  ExecutionStats)>` — exactly `explain` minus the string; `explain` becomes render
  of `ExecutionStats` (one source of truth: `Report` takes the struct). Public,
  always available (it is the ANALYZE path — allocation-sanctioned like explain).
- Image bytes: `RelationImage::byte_size(&self) -> usize` (sum of both slab
  capacities); `image_build` trace span's `a1` flips to bytes (PRD 04 handoff).
- Cache stats (feature `trace` only — atomics are per-op costs): `ImageCache`
  gains relaxed `AtomicU64` hits/misses/builds/evicted and `pub fn stats() ->
  CacheStats` + `pub fn resident() -> (u64 images, u64 bytes)` computed under the
  lock. Reader: the bench report. Under default features the fields and methods do
  not exist (`#[cfg(feature = "trace")]` on fields and impl — this module is not a
  hot path's inner loop; cfg here is acceptable and contained).
- Storage numbers via `Db` (always-on, they are one-shot reads): `pub fn
  disk_size(&self) -> Result<u64>` (heed's env info / file metadata of `data.mdb`)
  and `pub fn generation(&self) -> Result<u64>` (snapshot-sourced via a temp read
  txn). Readers: the report.

## Non-goals

Rendering changes to the EXPLAIN string beyond re-plumbing through the struct.
Persisting stats anywhere.

## Passing criteria

- Unit tests: `profile` on the existing skew fixture yields NodeStats matching the
  pinned cover-choice expectations and `emits` equal to the row count; `explain`'s
  string is unchanged for a pinned fixture (golden substring assertions);
  `byte_size` of a built image ≥ rows × row width and aligns with slab math; cache
  stats count a hit/miss/build sequence exactly (feature `trace` test); `disk_size`
  is > 0 and grows after a 10k-fact commit.
- Default-feature build: no cache atomics exist (compile check via cfg test),
  gate green.
