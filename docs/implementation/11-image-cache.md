# PRD 11 — The Environment Image Cache

Authority: `docs/architecture/40-storage.md` (cache: key, generation source,
insert-if-absent race rule, retain-newest eviction, pinned readers, no
memory-pressure eviction).

## Purpose

The cross-transaction image cache — the mechanism whose absence was v5's quietest
failure (post-mortem §26).

## Technical direction

- `image::cache`. `ImageCache` owned by the environment handle:
  `Mutex<HashMap<(RelationId, u64 /*generation*/), Arc<RelationImage>>>` (a plain
  mutexed map — the lock covers map ops only, never builds).
- `get_or_build(&ReadTxn, rel) -> Result<Arc<RelationImage>>`: generation = the
  reader's snapshot-sourced tx id (PRD 04 accessor — **never** an ambient counter);
  lock, lookup, unlock; on miss, build (PRD 10) *outside* the lock, then
  insert-if-absent: if another thread won, adopt the winner's `Arc` and drop ours.
- **Eviction**: `evict_older_than(generation)` retains only entries at ≥ the given
  generation; called by the write path after a state-changing commit (PRD 28 wires
  `CommitReport` → eviction; this PRD exposes the method and unit-tests it directly).
  Readers pinned at older generations keep their `Arc`s (the map drop only releases
  the map's reference); an old-generation reader that misses builds **without
  inserting** (generation < newest ⇒ skip insert — query-local image, per the doc).
- No size cap, no LRU, nothing else (documented decision).

## Non-goals

Multi-process anything. Background eviction threads (the engine owns zero threads).

## Passing criteria

- Unit tests: two sequential read txns, no intervening write → identical `Arc`
  (ptr_eq); after a state-changing commit + eviction, a new reader builds a new image
  and the map holds only the new generation; an old reader holding its `Arc` still
  reads its image after eviction; old-generation miss does not populate the map;
  concurrent same-generation `get_or_build` from two threads yields ptr-equal results
  (spawn-two-threads unit test — this is a module contract, not an e2e suite);
  an all-no-op commit (PRD 08 `changed: false`) followed by no eviction keeps the
  cache warm.
- Global commands green.
