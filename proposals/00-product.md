# 00 — Product ruling

## What it is

`bumbledb-log` makes a bumbledb store durable, replicated, backed up,
point-in-time recoverable, and **concurrently writable** by representing
the store's **history** as data in an object store: per-braid command
logs, checkpoints, one CAS manifest.

The braids are the product. Bumbledb's semantics are a closed, compiled
constraint set — functionality, containment, capacity — and the schema's
statement graph decomposes into connected components that provably never
conflict: statements cannot span braids, so each braid carries an
independent chain and cross-braid ordering is semantically invisible,
machine-checked (L9). Concurrency is *derived from the declared theory*,
not managed. Within a braid, contention resolves to serial truth: a CAS
loser re-judges its recorded ops against the winner-current state and
receives exactly the verdict a serial execution would have produced,
with violations as data. Set semantics does the same to recovery:
re-application is a proven no-op (L10), so recovery *is* replay-forward
through the ordinary catch-up loop.

Not a CRDT: CRDTs weaken to always-commuting operations and therefore
cannot express an FD, an IND, or a ceiling. This keeps every invariant and
derives which operations commute. Not consensus either: arbitration is
object-store compare-and-swap per log slot; there is no quorum, no term,
no leader.

There is no server in the architecture. A resident writer is a deployment
*mode* chosen for 1 ms acks, not a component.

## The laws

1. **The log is the write-ahead truth.** A commit is acknowledged when its
   log object exists. Local LMDB state is a materialized view. (The
   `ack = local` resident mode trades this law for 1 ms acks, visibly —
   the outcome says `durability: LocalPending`, and the loss window is
   the one pending batch, at most one drain's worth, by construction;
   60.)
2. **Index = applied count, per braid; generation = the sum.** The
   schema's statement graph decomposes into connected components
   (braids); each braid has its own chain, indexed by its applied count,
   and the engine's single `GenerationId` equals the counts' sum —
   exactly, because only state-changing batches are ever published (law
   6). Statements never span braids, so braids never conflict — by
   construction (L9).
3. **Losers keep their outcomes.** A CAS loser's outcome equals a serial
   execution of the submitted transaction: `Accepted` at the realized
   generation — including the net-no-op case, where the log already
   holds its effects, via the publish law — or the serial `Rejected`
   with violations as data. The loser re-judges its recorded ops at the
   winner-current tip through the one loss path (60); which racer wins
   is run-dependent, but that the outcome equals **a** serial history —
   the one the log realized — is not.
4. **Replay is deterministic.** Same checkpoint + same braid prefixes ⇒
   byte-identical catalog content (`catalog_digest`), any interleaving of
   braid application (L9).
5. **One way per question.** Slot arbitration: `If-None-Match` on the next
   log key. Tip discovery: forward probing. Checkpoint publication:
   manifest CAS. Loss resolution: byte-equal absorption, else
   discard-re-open-re-judge — the one path. Nothing else.
6. **The empty commit is not a commit.** A batch is published only if its
   local application advanced the generation; the log never contains a
   no-op slot. Consequence: `engine generation ≡ Σ vector` on every
   honest store — the identity recovery leans on.
7. **Recovery is replay.** Set-semantic net-disposition makes
   re-application of an applied batch a proven no-op (L10), so every
   crash window heals by replaying forward through the ordinary catch-up
   loop. There is no recovery procedure, no intent field, no forced-case
   table; the one residual instrument is the wholeness identity
   `generation ≡ Σ vector + |applied pending|` (50), which decides
   phantom detection, born-no-op pendings, and
   no-op-slot refusals alike — one compare, every verdict; its failure
   is a discard, never a repair.
8. **Every read is a serial prefix.** A replica at any vector serves a
   real admitted state satisfying every declared statement — the thing
   CRDT locals cannot promise (their reads are unconstrained by their own
   literature's admission). Freshness is the only staleness dimension:
   cross-instance read-your-writes and monotone reads ride `wait_for`
   with a session vector; watermark facts ("braid c reached g") are the
   only observables stable without it.

Honesty about the residue: commits are not coordination-free in the
CALM/Bailis sense and are not meant to be — every commit pays one
conditional PUT on its braid slot, concurrent writers on one braid
serialize their slot claims, and a lost claim pays a cache-warm re-open
plus one local re-judgment. What the braids remove is every cross-braid
interaction; what the design keeps is a total order per braid, which is
exactly what makes verdicts serial and replay deterministic. Reads and
rejections touch nothing. This is the answer to feral concurrency
control: Bailis measured production Rails fleets leaking 70–6,300
duplicate keys and up to 6,400 orphans through application-level
enforcement of exactly our three statement families — and the answer
was never a conflict-avoidance fast path; it is typed serial verdicts,
delivered through the only loss path there is
(`docs/research/replication-prior-art/feral-sigmod15/`).

## The five deployment cases (each names its consumer)

1. **Next.js on Vercel Fluid** — replica singleton per instance (Fluid
   shares module state; native modules supported; `/tmp` 500 MB,
   per-instance). Microsecond local reads; serverless commits with the
   one loss path absorbing races. Consumer: SaaS on Vercel.
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
5. **Local fleet** — N writer processes, one machine, one `FsStore`
   prefix; each process a replica+writer with its own LMDB dir; no
   network anywhere in the loop. The degenerate-serial case (10) worn
   proudly: one-braid theories serialize slot claims on a link and the
   one loss path absorbs the rest. Consumer: primer-spec's parallel
   scope loops — an insert-only, content-keyed theory (no deletes, no
   capacities) whose one reachable conflict (concurrent double-mint of
   identical content under a declared FD) re-judges into the winner's
   row, and whose host retry policy is an unbounded repair loop, which
   is exactly the "retry is host policy" contract. One commit = one
   admitted document; the braid chain is the generation ledger.

## Performance envelope (vendor facts verified; measured pins from 80 supersede this section)

Standard S3 PUT p50 ≈ 20–60 ms; S3 Express One Zone single-digit ms, up to
100K writes/s per directory bucket, ≈ $0.00113 per 1 000 PUTs; R2 supports
the same conditionals with zero-egress pulls. Reads are always local.
Commit throughput = per-braid serialization × braid count × group-commit
batching; a lost slot costs a cache-warm re-open plus one local
re-judgment regardless of overlap (checkpoints are content-addressed,
so the re-open revalidates the local `.mdb` instead of re-downloading
it — the fsync floor dominates, measured in 80's loss-cost pin).

## Non-goals (v1)

- No consensus, no leases-as-truth (id-leases are an avoidance layer;
  correctness never depends on them; capacity reservations are ordinary
  rows judged by the ordinary theory — 60).
- No quantitative conflict avoidance: the interval algebra that once
  routed losers around re-judgment was deleted whole by the one-path
  ruling — its outcome was provably identical to the general path and
  its measured latency higher. Reopen trigger: a real multi-writer
  deployment on a network store measuring loss resolution as the
  dominant term in commit latency under contention; the theory lives in
  git history, and reopening is a design campaign, never a revert.
- No schema migration (fingerprint mismatch refuses; migration is its own
  future PRD). Add/delete only.
- No cross-braid atomicity: statements cannot relate braids, so partial
  application across braids is *semantically invisible to the theory* —
  though not to the application, which is why spanning writes are a
  separate verb (`commit_split`, 60) chosen at the call site, never
  inferred. A host invariant spanning unrelated relations is, by
  definition, not in the theory — declare it and the braids merge.
- No compression (reserved flag bit, 20), no frozen-value shipping, no
  per-obligation partial revalidation (recorded v2 optimization).

The demand curve for exactly this shape is measured, not believed: across
67 production Rails codebases, declared invariants outnumber transactions
37 to 1, and the declared set is almost entirely our three families —
while feral enforcement of them leaks (70–6,300 duplicate keys, 6,400
orphans in Bailis's experiments) precisely where concurrent writers
share a determinant — the contention our serial verdicts refuse with a
proof (`docs/research/replication-prior-art/feral-sigmod15/`).
