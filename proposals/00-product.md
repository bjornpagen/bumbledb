# 00 — Product ruling

## What it is

`bumbledb-log` makes a bumbledb store durable, replicated, backed up,
point-in-time recoverable, and **concurrently writable** by representing
two things as data in an object store: the store's **history** (per-braid
command logs, checkpoints, one CAS manifest) and each commit's **conflict
footprint** — the set of constraint obligations it touches, derived from
the declared theory itself.

The second half is the product. Bumbledb's semantics are a closed,
compiled constraint set — functionality, containment, capacity — and the
engine's delta-restriction theorem already proves that a commit's validity
depends only on the obligations it touches. `plan_commit` computes that
enumeration on every commit today and throws it away. This design keeps
it: **published, it is a commutativity certificate**. Two commits whose
footprints don't collide provably cannot invalidate each other, in either
order. Concurrency is not managed here; it is *extracted from the theory,
with the extraction machine-checked* (15, Lean L6–L9).

Not a CRDT: CRDTs weaken to always-commuting operations and therefore
cannot express an FD, an IND, or a ceiling. This keeps every invariant and
derives which operations commute. Not consensus either: arbitration is
object-store compare-and-swap per log slot; there is no quorum, no term,
no leader.

There is no server in the architecture. A resident writer is a deployment
*mode* chosen for 1 ms acks, not a component.

## The laws

1. **The log is the write-ahead truth.** A commit is acknowledged when its
   log object exists. Local LMDB state is a materialized view.
2. **Index = generation, per braid.** The schema's statement graph
   decomposes into connected components (braids); each braid has its own
   chain, and within it the engine's `GenerationId` is the log index.
   Statements never span braids, so braids never conflict — by
   construction (L9).
3. **Footprints are carried and checked.** Every batch publishes the
   footprint the driver computed from raw values and the descriptor;
   every replica recomputes it during replay and refuses a mismatch.
4. **Losers keep their work.** A CAS loser with a disjoint footprint
   republishes without re-judging (L7 — footprint stability). Only a
   genuine conflict cell forces re-judgment, and the re-judged rejection
   is exactly what serial execution would have said.
5. **Replay is deterministic.** Same checkpoint + same braid prefixes ⇒
   byte-identical catalog content (`catalog_digest`), any interleaving of
   braid application (L8).
6. **One way per question.** Slot arbitration: `If-None-Match` on the next
   log key. Tip discovery: forward probing. Checkpoint publication:
   manifest CAS. Conflict detection: the four matrices of 15. Nothing
   else.

## The four deployment cases (each names its consumer)

1. **Next.js on Vercel Fluid** — replica singleton per instance (Fluid
   shares module state; native modules supported; `/tmp` 500 MB,
   per-instance). Microsecond local reads; serverless commits with the
   loser algebra absorbing races. Consumer: SaaS on Vercel.
2. **Embedded macOS (Apple Silicon)** — the engine as today; the log as
   optional sync/backup (resident mode, the app is the writer). Consumer:
   desktop apps via napi or the C ABI.
3. **Long-lived server (OCI or any box)** — resident mode: 1 ms local
   acks, the same log for RPO≈0, PITR, bucket-as-backup. Consumer: the
   side project.
4. **Distributed per-tenant** — tenant = prefix; braids shard *within* a
   tenant; the control-plane tenant carries shared reference data;
   cross-tenant analytics is the heap arm (scan → builder → `admit` →
   query). Consumer: the eventual multi-tenant deployment of 1/3.

## Performance envelope (vendor facts verified; measured pins from 80 supersede this section)

Standard S3 PUT p50 ≈ 20–60 ms; S3 Express One Zone single-digit ms, up to
100K writes/s per directory bucket, ≈ $0.00113 per 1 000 PUTs; R2 supports
the same conditionals with zero-egress pulls. Reads are always local.
Commit throughput = per-braid serialization × braid count × group-commit
batching; contention costs one intersection + one PUT when disjoint (the
common case), one local re-judgment when not.

## Non-goals (v1)

- No consensus, no leases-as-truth (escrow and id-leases are avoidance
  layers; correctness never depends on them).
- No schema migration (fingerprint mismatch refuses; migration is its own
  future PRD). Add/delete only.
- No cross-braid atomicity: statements cannot relate braids, so partial
  application across braids is *semantically invisible to the theory*;
  `commit` auto-splits spanning batches and returns per-braid outcomes
  (60). A host invariant spanning unrelated relations is, by definition,
  not in the theory — declare it and the braids merge.
- No compression (reserved flag byte), no frozen-value shipping, no
  per-obligation partial revalidation (recorded v2 optimization).
