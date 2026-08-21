# 40 — Exact count: the cardinality that already exists becomes reachable

The engine has maintained an exact per-relation cardinality since format 8:
`StatKind::RowCount`, folded transactionally at every commit
(`storage/commit/write.rs::flush_counters`), read in O(1)
(`storage/read/row_count.rs`), pinned equal to the scan count
(`row_count_equals_scan_count_after_mixed_commits`), consulted by the
planner on every prepare (`plan/selectivity.rs`). It is implemented on both
read surfaces — `ReadInstance::row_count`, `OwnedInstance::row_count` —
and marked `pub(crate)` + `allow(dead_code)` (V6). Callers therefore count
4 M facts by decoding them (2,810 `readScan` + 340 `factOf` + 264
`isCompleteFact` samples in the accepted profile), or by full-relation
aggregate queries that still walk everything. This document makes the
existing read reachable and makes it THE spelling for cardinality.

## Why the aggregate is not this read (and stays for what it is)

`r.count()` in a query counts **distinct bindings of that query** — a
statement about a derived answer set. It is not a second way to read
stored cardinality, and after this doc it must never be used as one:

- it requires a full binding of every field to make bindings coincide
  with facts (and composing that generically trips V7 — see
  [50-generic-binding.md](50-generic-binding.md));
- an all-aggregate query over an empty input returns the **empty set**,
  not one zero row — set semantics, correct for queries, but forcing every
  cardinality caller to branch absence back into `0`;
- it may walk the relation; the maintained counter never does.

One meaning, one spelling: *cardinality of a stored relation* = `count`;
*size of a query's answer set* = `r.count()` in a `find`. Primer's
temporary full-binding count queries and its earlier scan-and-measure are
both deleted spellings ([70-deletions.md](70-deletions.md) D11).

## The surface (one name at every layer)

- **Engine (`crates/bumbledb`):** rename the two `pub(crate) fn row_count`
  API methods to `pub fn count` on `ReadInstance<S>` and
  `OwnedInstance<S>`, drop the dead-code allowance. The storage tier keeps
  its own vocabulary (`CatalogRead::row_count`, `read::row_count`,
  `StatKind::RowCount`) — that is the counter's name as data; `count` is
  the read's name as API. Closed relations answer their sealed extension
  length, exactly as the existing `OwnedInstance` body does. Errors:
  `UnknownRelation`; `Corruption` on a malformed counter. No allocation.
- **Bridge (`ts/crate`):** `instance_count(handle, relation_id) → u64`
  and `owned_count(handle, relation_id) → u64`, u64 crossing as `bigint`
  under the existing wire law ("u64/i64 always cross as `bigint` — never
  `number`"). One new read per handle kind, nothing else.
- **TypeScript (`@bjornpagen/bumbledb`):**

  ```ts
  instance.count<R extends MemberRelation<Rels>>(relation: R): bigint  // ReadInstance
  owned.count<R extends MemberRelation<Rels>>(relation: R): bigint     // OwnedInstance
  db.count<R extends MemberRelation<Rels>>(relation: R): bigint        // symmetry sugar
  ```

  `db.count(r)` === `db.read(instance => instance.count(r))` — the same
  symmetry rule as `db.scan`/`db.get`/`db.contains`, stated in the same
  words. `MemberRelation` already excludes closed relations, so counting a
  closed relation is a type error in TS exactly as scanning one is — a
  sealed extension is schema data whose length the caller already declared.
  The upstream report's `snapshot.count(Relation)` maps to
  `ReadInstance.count`: the read lease **is** the snapshot in this SDK,
  and no `snapshot` alias is introduced — a second name for the lease
  would be a second spelling of one thing.

Not added, deliberately: `WriteTx.count` (no consumer names itself; the
delta maintains `row_count_delta`, so a final-state count is derivable the
day a consumer exists — adding it speculatively would be generality without
a caller), and any batch/multi-relation count call (a loop over one read
is not a new operation).

## Laws

- **Law 9 — read scopes own snapshot lifetime:** `count` is an instance
  method inside the lease, invalidated with it (`assertLive`), never a
  free-standing handle read.
- **Law 10 — exact count observes the same snapshot as `scan`:** on
  `ReadInstance` both reads run inside the lease's one `ReadTxn`; on
  `OwnedInstance` both read the one frozen catalog. True by construction;
  pinned by test anyway (below).
- **`bigint` by law:** engine cardinality is `u64`, which is not a
  JavaScript safe integer by construction. The type states it; no
  `Number()` narrowing exists anywhere on the path.
- **Exactness:** this is a structural read of the maintained counter —
  never an estimate, never a scan, never a decoded fact object. The
  planner has trusted this counter for every cost decision since format 8;
  the public read inherits that trust, not a new mechanism.

## Pinned tests

1. TS twin of the engine pin: after mixed inserts/deletes/cancels across
   several commits, `instance.count(R)` equals `BigInt(instance.scan(R).length)`
   — same lease, both reads.
2. Snapshot law: open a lease, commit a later write from outside, the held
   lease still reports the pre-commit count; a fresh lease reports the new
   one.
3. Empty relation: `0n` — a value, not an empty result to reinterpret.
4. Closed relation: `@ts-expect-error` pin on `instance.count(ClosedRel)`;
   engine-side `count` on a closed relation returns the extension length
   (dyn-lane behavior, pinned in Rust).
5. `db.count` sugar ≡ lease read (the symmetry-rule pin the other sugars
   already have).
6. Allocation: the count lane in the primerlane (10, component 12) shows
   zero engine-side allocation per call.

## Consumers

Primer (`countRelations` becomes one `count` per relation — the ~250 ms
full-binding-query readback and the scan readback before it both die),
`dev/readback.ts`, the bench count lane (10), and the planner (unchanged,
same counter).
