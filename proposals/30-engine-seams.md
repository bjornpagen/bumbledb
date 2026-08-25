# 30 — Engine seams

The driver needs almost nothing from the engine, and this document is the
exhaustive list. The engine never learned replication exists — braids
are derived in the driver from the descriptor, contention is judged by
the ordinary engine verdicts, and no replication-shaped surface was
ever added; that record remains exactly true.

## What the engine already provides (consume, don't rebuild)

| Need | Existing surface |
| --- | --- |
| The per-braid index | `GenerationId` advances exactly once per state-changing commit; `generation(chain)` equals the store generation (50's sidecar is `Settled \| Pending` and splits the vector per braid) |
| Apply | the dyn collection write path inside one `db.write` |
| Checkpoints | `Db::compact()` — read-txn-pinned; writers keep flowing |
| Bootstrap | `Db::create` (empty-candidate admission) |
| Open-time verification | format-8 open: version → fingerprint → go |
| Rejection as data | `Admission` / `Violations` — returned to hosts, never logged |
| Schema identity | the 32-byte fingerprint (manifest + batch headers) |
| Statement roster for braids | the schema descriptor: relations, key rosters, containment mappings and projections, capacity parents and weight specs — everything the braid derivation and the drivers' descriptor lowering read |

## The one engine addition

**`catalog_digest`** — `#[doc(hidden)]`, harness-tier (the `verify_store`
class):

```rust
#[doc(hidden)]
pub fn catalog_digest(&self) -> Result<[u8; 32]>;
```

The order-quotient equality oracle: equal digests ⇒ identical judged
content regardless of LMDB page layout *and* of allocation history. The
digest renders the data map canonically before folding — each
commit-order row id is quotiented to the row's fact identity read off
the membership namespace (F keys carry the fact hash where the id sat,
M values carry nothing their key does not already say, U values name
the incumbent by hash, R keys name the source row by hash), the
rendered entries sort, and the dictionary streams raw behind them — so
two apply orders of independent commits land one digest while any real
content difference still separates. A row id with no membership entry,
or two membership entries claiming one row id, is the loud
`MembershipDesync` corruption verdict; unparseable keys digest raw, so
a corrupt store still digests deterministically. Two protocol consumers
beyond the conformance lanes:
the checkpoint json's `catalog` content claim and its open-time
verification (10) — both off any hot path (checkpoint publication and
cold open). The `OwnedInstance` twin lands with it. One sequential
pass. The surface law is reaffirmed: this one `#[doc(hidden)]`
function IS the engine seam — **the entire engine diff.**

One export is blessed beside it, on the SDK rather than the engine
crate: the ts/crate napi module's `blake3_hash` (surfaced as
`internalBlake3`) lends the engine-linked blake3 across the FFI. Its
consumer roster is named and complete: the TS driver's store etags
(computed, never stored — 40) and its descriptor fingerprinting. The
seam list ends here.

## Written guarantees to add (a law comment + one pinned test each; no code)

1. **Intern-mint determinism** — pending intern ids assign in first-use
   order during apply; identical batches against identical stores mint
   identical ids. (Replay determinism's representation-level half leans
   on this.)
2. **Fresh-in-command determinism** — fresh-keyed rows replay with the id
   carried in the command; a collision is an ordinary functionality
   rejection.
3. **Host-order independence** — the canonical `(relation, fact_hash)`
   plan sort means op order inside a batch cannot influence stored bytes
   (already law; cited here because cross-braid replay determinism, L9,
   composes with it).

## Explicitly refused engine changes

- No conflict-enumeration export from `plan_commit` — replication never
  became an engine concern: the one-path ruling deleted the driver-side
  conflict algebra outright, and the engine side was never built.
- No braid awareness — braids are descriptor-derived in the driver.
- No changefeed, no applied-index relation, no dry-run judge, no
  read-only open, no vector storage in `_meta` (the sidecar owns it; the
  engine's generation stays one counter).
- No engine knowledge of tenants, buckets, manifests, or leases.
  (Capacity reservations need no mention: they are ordinary rows the
  engine judges like any others — 60.)

## The Lean lockstep (listed here for the ledger)

Two theorems, the two the system spends: **L9** component locality — a
statement's obligation instances read and write only relations inside
one braid component, so judgment and application over one braid are
invariant under any other braid's history — and **L10** replay
idempotence, the theorem the whole recovery design stands on. Both live
in `lean/Bumbledb/Txn/Braids.lean`, proven against the engine's
existing delta-restriction and net-disposition machinery, wired into
the obligation ledger (104 rows) with the braid goldens and the
double-apply matrix as their instruments. These are theorems about the
*theory*, not about the driver: they extend the engine's own Lean
corpus and sit beside `DeltaRestriction.lean`.
