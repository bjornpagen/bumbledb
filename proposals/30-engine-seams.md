# 30 — Engine seams

The driver is designed to need almost nothing from the engine, and this
document is the exhaustive list. Anything not named here that an
implementer thinks the driver needs is a design error in the driver, not a
new engine surface.

## What the engine already provides (consume, don't rebuild)

| Need | Existing surface |
| --- | --- |
| The log index | `Db::generation()` — `GenerationId` advances exactly once per state-changing commit; no-op and rejected writes do not advance it |
| Apply | the dyn collection write path (insert/delete by `RelationId` with raw values) inside one `db.write` |
| Fresh floors | the floor-advance path behind `reserve_at` (FloorBump rides it) |
| Checkpoints | `Db::compact()` — read-txn-pinned, writers keep flowing |
| Store birth | `Db::create` (empty-candidate admission) for bootstrap when the manifest has no checkpoint |
| Open-time verification | format 8 open: version → fingerprint → go |
| Rejection as data | `Admission` / `Violations` — the writer returns them to the host; they never reach the log |
| Schema identity | the 32-byte fingerprint (manifest and batch headers carry it) |
| Optimistic concurrency vocabulary | `ConditionalWrite::Moved` shape — the driver's CAS-loss surfaces the same way to hosts |

## The one engine addition

**`catalog_digest`** — a `#[doc(hidden)]` harness-grade method (the
`verify_store` tier, not embedding API):

```rust
#[doc(hidden)]
pub fn catalog_digest(&self) -> Result<[u8; 32]>;
```

Blake3 over the raw ordered enumeration of every `_data` entry then every
`_dict` entry (key length, key bytes, value length, value bytes — the
existing raw-export iteration order). It is the replication equality
oracle: two stores with equal digests have identical catalog **content**
regardless of LMDB page layout. Available on the durable store; the
`OwnedInstance` twin (same stream over the frozen maps) lands with it so
the heap arm can join the same gates.

Cost: one sequential pass; harness-only; never on a hot path. This is the
entire engine diff for the project.

## Written guarantees to add (documentation + one test each, no code)

1. **Intern-mint determinism.** "Pending intern ids are assigned in
   first-use order during apply; identical batches against identical
   stores mint identical ids." Recorded as a law comment at the mint site
   plus a pinned test (apply the same batch to two fresh stores; assert
   `catalog_digest` equality). The replication protocol's correctness
   leans on this; it must be a written law, not an accident.
2. **Fresh-in-command determinism.** Fresh-keyed rows replay with the id
   carried in the command; a collision is an ordinary functionality
   rejection (which, per 20, is `ReplayDiverged` during replay). One test.

## Explicitly refused engine changes

- No changefeed / "changes since generation" export — the log supersedes
  it; the engine never learns about replication.
- No applied-index relation injected into user theories — index =
  generation makes it unnecessary.
- No dry-run commit / speculative-judge API — the two writer modes (60)
  are designed so neither needs it.
- No read-only open mode — every replica owns its private copy.
- No engine knowledge of tenants, prefixes, buckets, or manifests. The
  engine's world ends at the store directory; the driver's begins there.
