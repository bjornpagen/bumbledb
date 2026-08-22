# 30 — Engine seams

The driver needs almost nothing from the engine, and this document is the
exhaustive list. The conflict algebra (15) is **deliberately not an engine
seam**: footprints are pure functions of `(descriptor, ops)` over raw
values, computed and verified in the driver — the engine never learns
replication exists.

## What the engine already provides (consume, don't rebuild)

| Need | Existing surface |
| --- | --- |
| The per-braid index | `GenerationId` advances exactly once per state-changing commit; the vector sum equals the store generation (50's sidecar splits it per braid) |
| Apply | the dyn collection write path inside one `db.write` |
| Checkpoints | `Db::compact()` — read-txn-pinned; writers keep flowing |
| Bootstrap | `Db::create` (empty-candidate admission) |
| Open-time verification | format-8 open: version → fingerprint → go |
| Rejection as data | `Admission` / `Violations` — returned to hosts, never logged |
| Schema identity | the 32-byte fingerprint (manifest + batch headers) |
| Statement roster for braids and footprints | the schema descriptor: relations, key rosters, containment mappings and projections, capacity parents and weight specs — everything `footprint()` and braid derivation read |

## The one engine addition

**`catalog_digest`** — `#[doc(hidden)]`, harness-tier (the `verify_store`
class):

```rust
#[doc(hidden)]
pub fn catalog_digest(&self) -> Result<[u8; 32]>;
```

Blake3 over the raw ordered enumeration of every `_data` then `_dict`
entry (key length, key bytes, value length, value bytes). The replication
equality oracle: equal digests ⇒ identical catalog content regardless of
LMDB page layout. Two protocol consumers beyond the conformance lanes:
the checkpoint json's `catalog` content claim and its open-time
verification (10) — both off any hot path (checkpoint publication and
cold open). The `OwnedInstance` twin lands with it. One sequential
pass. **This is the entire engine diff.**

## Written guarantees to add (a law comment + one pinned test each; no code)

1. **Intern-mint determinism** — pending intern ids assign in first-use
   order during apply; identical batches against identical stores mint
   identical ids. (L8's representation-level half leans on this.)
2. **Fresh-in-command determinism** — fresh-keyed rows replay with the id
   carried in the command; a collision is an ordinary functionality
   rejection.
3. **Host-order independence** — the canonical `(relation, fact_hash)`
   plan sort means op order inside a batch cannot influence stored bytes
   (already law; cited here because L8 composes with it).

## Explicitly refused engine changes

- No footprint export from `plan_commit` — the raw-value footprint (15)
  is strictly better: state-independent keys (no intern aliasing), pure
  driver function, recompute-verifiable, zero engine coupling.
- No braid awareness — braids are descriptor-derived in the driver.
- No changefeed, no applied-index relation, no dry-run judge, no
  read-only open, no vector storage in `_meta` (the sidecar owns it; the
  engine's generation stays one counter).
- No engine knowledge of tenants, buckets, manifests, or leases.
  (Capacity reservations need no mention: they are ordinary rows the
  engine judges like any others — 15.)

## The Lean lockstep (lives with the algebra, listed here for the ledger)

L6 footprint soundness, L7 footprint stability, L8 commutativity, L9
component independence, L10 replay idempotence — stated in 15, proven
against `Txn.lean`'s existing delta-restriction and net-disposition
machinery, gating the optimism path (L7) and the no-forced-cases
recovery (L10) per 90. These are theorems about the *theory*, not about the
driver: they extend the engine's own Lean corpus and belong beside
`DeltaRestriction.lean`.
