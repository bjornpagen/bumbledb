# 08 — `ImageCache`: closed relations in a second map; the cache API keyed off raw txn generations

- **Status:** **keep** — closed vs generation-keyed maps are
  `ViewEpoch` arms. One `RelationSlot` would make store generations
  representable on a closed image. Brooks: two maps are the coordinate
  change, not a dual cache.
- **Severity:** should-fix.
- **Supersedes:** PROP-007, REP-09, SPINE-14, SPINE-15, SPINE-16.
- **Adjudication (third pass): keep CONTESTED — owner ruling required.**
  The keep-ruling has the mechanism inverted. "One `RelationSlot` would make
  store generations representable on a closed image" is false of the
  proposed sum: `Closed(OnceLock<…>)` carries no generation field, so a
  generation on a closed image is *unrepresentable by the arm shape*. It is
  the CURRENT two-map layout that enforces the partition by convention —
  the schema-body re-match in `get_or_build` and the tautological
  `expect("Closed body implies a closed cache slot")` are the guards the
  sum deletes. Two maps are not the coordinate change; they are the same
  partition stated twice. SPINE-16 (the cache API inventing epochs from a
  raw txn instead of receiving them from `ImageBind`) is untouched by the
  ruling either way and remains open.

## Principle

Insight 7 (a match on a tag that data should carry) plus one partition, not
two: `ViewEpoch::{Closed, Frozen, Store}` is the engine's three-way epoch
sum, and the image side re-derives the same partition twice — a
`closed: HashMap` beside the generation-keyed map (dispatched by a
schema-body match ending in a tautological `expect`), and `FrozenSlot` as a
third container of the same slot machinery on the heap arm.

## Evidence

- `crates/bumbledb/src/image/cache.rs:112` —
  `closed: HashMap<RelationId, OnceLock<Arc<RelationImage>>>` beside the
  `(RelationId, GenerationId)` map.
- `image/frozen.rs` — `FrozenSlot`: the same once-lock slot shape, third
  container.
- `image/cache/get_or_build.rs` / `peek.rs` — the cache API takes a raw
  `&ReadTxn` and invents the generation itself (`txn.generation()`), instead
  of receiving the epoch from `ImageBind` (SPINE-16).

## The fix

1. One `Box<[RelationSlot]>` indexed by `RelationId`:

   ```rust
   enum RelationSlot {
       Closed(OnceLock<Arc<RelationImage>>),
       Ordinary(GenerationCache),   // the existing generation-keyed entry
   }
   ```

   The schema-body match happens once at cache construction (the key set is
   already built from the schema); `get_or_build`'s body match and the
   `expect("Closed body implies a closed cache slot")` delete.
2. The cache API takes `RelationId` + `ViewEpoch` (or `&LmdbSource`), never
   a raw txn — the epoch is `ImageBind`'s to mint, not the cache's to
   re-derive (SPINE-16).
3. The heap arm's `FrozenSlot` becomes the same `RelationSlot` vocabulary
   with the store arm unrepresentable (`Frozen` sources cannot hold
   `Ordinary(GenerationCache)` — the slot table for a frozen source is built
   without that arm, or `FrozenSlot` is a newtype over the `Closed`-shaped
   slot; either way one slot type, not three containers).

## Acceptance

- One slot container per source; no second `HashMap`; no schema-body match
  inside `get_or_build`.
- `grep "txn.generation()" crates/bumbledb/src/image/` is empty — epochs
  arrive as parameters.
- Advance/eviction behavior pinned unchanged by the existing cache lineage
  tests; scenario lanes byte-identical.
