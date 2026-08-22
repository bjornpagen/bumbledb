# Turso diskless — industrial findings and the steal list

Source: [Turso Cloud Goes Diskless](https://turso.tech/blog/turso-cloud-goes-diskless)
(read 2026-08-21). They independently converged on our substrate: commits
durable in S3 Express One Zone before ack; periodic fold into the main
file on standard S3; "no difference between writing and backing up"; an
idle database is files costing only storage. Validation, plus measured
numbers our envelope can cite until F11's own pins land.

## Their numbers (EC2, us-east-1, 1000 ops)

- S3 Express same-AZ: 4 kB upload avg **6.4 ms**, p99 ≤ 10; downloads
  ~4 ms; cross-AZ adds ~1 ms. Local fsync is ~2 ms, so Express costs only
  ~4 ms over local durability.
- Standard S3: 4 kB upload avg 31 ms (p99 102) — the two-tier split is
  not optional at latency-sensitive write rates.
- Cost: Express PUTs at half the standard price; their viability came
  from **batching ~100 databases' commits into one PUT**, amortizing to
  ~$0.57/database/month.
- Express durability is 11 nines but **single-AZ availability (99.95%)**;
  they contemplate dual-zone writes (costing ≈ one standard PUT).

## The steal list (applied to the PRDs where marked)

1. **Two storage classes, explicit** — hot log objects on Express (or
   R2), checkpoints on standard S3 (cheap, multi-AZ). *Applied: 10, 40.*
2. **Availability honesty + optional dual-PUT** — Express's single-AZ
   availability means a zone event pauses writes (never loses them);
   deployments that care dual-write log objects to a second zone's bucket.
   *Applied: 40 (recorded option).*
3. **The linger knob** — their deliberate "wait a couple extra ms" to
   batch across databases justifies a linger for tenant-dense writers;
   stays default-off for single-store (our no-linger v1 ruling holds
   where it was made). *Applied: 60 (knob recorded, default 0).*
4. **Degraded serve-during-rehydrate** — their fresh pod serves reads
   from S3 while the cache rehydrates. Our replica equivalent: reads are
   legal the moment the checkpoint opens, while the tail replays. *Applied:
   50 (stated explicitly).*
5. **Keep old checkpoints** — standard S3 is cheap; not deleting old
   versions makes continuous backup literally free. Matches our gc-window
   design; the default window for checkpoints should be generous.
   *Applied: 10 (default noted).*
6. **Cross-tenant staging segment** — their biggest structural trick
   (many databases, one PUT) does not fit our per-braid CAS arbitration
   directly; the faithful translation is a *resident-mode* Express staging
   segment for durability-ack with async fan-out to canonical braid logs.
   **Recorded v2 nicety, trigger: tenant-dense resident deployments where
   per-tenant PUT costs are measured to matter.** Not applied — v1 stays
   one-object-per-commit.
7. **DST culture** — they credit deterministic simulation testing; our
   FsStore crash matrices are the same spirit. No change; noted so nobody
   proposes replacing the matrices with ad-hoc chaos.

## What we have that they structurally cannot

Their conflicts are physical (MVCC abort-retry; CDC frames trusted on
sync). Ours are semantic: theory-disjoint writes provably never conflict,
colliding writes get the serial verdict with violations as data, and
replicas re-verify footprints instead of trusting frames. The substrate
is now common ground; the algebra is the moat.
