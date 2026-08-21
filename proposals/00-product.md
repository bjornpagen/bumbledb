# 00 — Product ruling

## What it is

`bumbledb-log` makes a bumbledb store durable, replicated, backed up, and
point-in-time recoverable by representing its write history as **data in an
object store**: one create-only object per committed generation (the log),
periodic compacted store artifacts (checkpoints), and one small CAS-guarded
pointer (the manifest). The engine's own `GenerationId` is the log index —
the counter that already advances exactly once per state-changing commit is
the replication sequence number. Nothing is invented; the protocol is the
engine's existing invariants, published.

There is no server in the architecture. A resident writer (a long-lived
process that owns a local store and appends to the log) is a *deployment
mode* chosen for write latency, not a required component. The other mode —
the serverless writer — needs only the object store's compare-and-swap.

One sentence per pillar:

- **Durability**: a commit is durable when its log object exists (RPO = 0).
- **Replication**: a replica is a checkpoint pull plus a deterministic
  replay of the log tail.
- **Backup**: the bucket *is* the backup; retention is a lifecycle policy,
  not a job.
- **PITR**: pin any generation `g`; restore = checkpoint ≤ g + replay to g.
- **Sharding**: a tenant is a prefix. The theory vocabulary cannot express
  a cross-instance dependency, so the judgment boundary is the shard
  boundary — partial replicas pull only the tenants they serve.

## The four deployment cases (each names its consumer)

1. **Next.js on Vercel Fluid** — module-level replica singleton per
   instance (Fluid shares module state across concurrent invocations;
   native modules are supported; `/tmp` is 500 MB, per-instance,
   ephemeral). Reads are local microseconds; writes are serverless-mode
   commits (one conditional PUT). Consumer: case 1 apps (SaaS on Vercel).
2. **Embedded macOS (Apple Silicon)** — the engine as today; the log is an
   optional sync/backup target (resident mode with the app as the writer).
   Consumer: desktop apps via napi or the C ABI.
3. **Long-lived server (OCI or any box)** — resident mode: local-fsync
   writes (~1 ms) with the same log appended for RPO≈0, PITR, and
   bucket-as-backup. Single writer needs no CAS; plain create-only PUTs
   suffice. Consumer: case 3 side project.
4. **Distributed per-tenant** — one prefix per tenant, one control-plane
   tenant for shared reference data, replicas LRU the tenants they serve,
   cross-tenant analytics via the heap arm (scan N tenants → builder →
   `admit` → query the `OwnedInstance`). Consumer: the eventual multi-tenant
   deployment of cases 1/3.

## Performance envelope (verified vendor facts; measured pins come from 80)

- Standard S3: PUT p50 ≈ 20–60 ms; conditional writes at no extra charge.
- S3 Express One Zone: consistent single-digit-ms access, up to 100K
  writes/s per directory bucket, conditional writes supported;
  post-April-2025 pricing ≈ $0.00113 per 1 000 PUTs.
- R2: conditional PUT (`If-Match`/`If-None-Match`) via S3-API extension and
  Workers bindings; zero egress for checkpoint pulls.
- Therefore: serverless-mode write latency ≈ one conditional PUT (2–10 ms
  Express, 20–60 ms standard) + local judge; resident-mode ≈ 1 ms local +
  async publish; reads are always local. Commit throughput is serialized by
  the log and multiplied by group commit (60).

## Non-goals (v1)

- No consensus, no multi-writer concurrency beyond CAS arbitration, no
  leases (a lease object is a recorded v2 nicety if contention churn is
  ever measured).
- No schema migration (fingerprint mismatch refuses; migration is its own
  future PRD). Add/delete only.
- No cross-tenant statements — structurally impossible in the theory
  vocabulary, and this PRD does not add them.
- No compression in v1 (a reserved flag byte exists; zstd is a measured
  later decision).
- No shipping of frozen `OwnedInstance` bytes (checkpoints are compacted
  LMDB stores; a frozen-value artifact is a recorded later nicety).

## Laws

1. **The log is the write-ahead truth.** A commit is acknowledged only
   after its log object exists. Local LMDB state is a materialized view of
   the log prefix.
2. **Index = generation.** Log object `g` holds exactly the command batch
   whose application produced `GenerationId g`. Rejected writes never
   produce log objects (rejection is judged before publish and never
   touches the network).
3. **Replay is deterministic.** Same checkpoint + same log prefix ⇒
   byte-identical catalog content on every replica (gated by
   `catalog_digest`, 30/80).
4. **One way per question.** Tip discovery is forward probing; head
   arbitration is `If-None-Match` on the next log key; checkpoint
   publication is manifest CAS. No alternates.
