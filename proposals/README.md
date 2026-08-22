# bumbledb-log — the braided object log

The PRD set for `bumbledb-log` (Rust) and `@bjornpagen/bumbledb-log` (TS):
durability, replication, backup, PITR, per-tenant sharding, **and
theory-derived concurrency** for bumbledb, over conditional-write object
storage. No servers; a resident writer is a mode.

The thesis: bumbledb's constraint set is closed and compiled, so the
conflict relation between concurrent commits is *computable per commit* —
the schema's statement graph shards the log into independent braids, and
each commit publishes a footprint that is a machine-checked commutativity
certificate. Invariants are never weakened (this is not a CRDT);
concurrency is extracted from the declared theory, and the extraction is
a Lean theorem (L6–L10). Set semantics does the same to recovery:
re-application is a proven no-op (L10), so recovery *is* replay — no
intent fields, no forced-case tables, no applied-watermark machinery.

These documents are **normative** in the `docs/architecture/` sense.
Read [00-product.md](00-product.md) first; [15-conflict-algebra.md](15-conflict-algebra.md)
is the centerpiece; [90-rollout.md](90-rollout.md) is the fleet dispatch.

| Doc | Contract |
| --- | --- |
| [00-product.md](00-product.md) | What it is, the laws, the four deployment cases, non-goals |
| [10-protocol.md](10-protocol.md) | Braids, keys, manifest + vector, log slots, checkpoints, leases, PITR, gc |
| [15-conflict-algebra.md](15-conflict-algebra.md) | Footprints over raw values, the four commutativity matrices, the loser algebra, the reservations idiom (escrow deleted), L6–L10 |
| [20-command-codec.md](20-command-codec.md) | The batch wire format (v2), footprint section, determinism laws, IDL refusal |
| [30-engine-seams.md](30-engine-seams.md) | The one engine addition; why the footprint is deliberately not a seam |
| [40-object-store.md](40-object-store.md) | The five-verb capability, vendor matrix, verb-consumer map, dependency rulings |
| [50-replica.md](50-replica.md) | The chain sidecar (recovery is replay), catch-up, gc heartbeat, Vercel, tenants |
| [60-writer.md](60-writer.md) | One commit discipline, the publish law, loser algebra wiring, group commit, reservations idiom |
| [70-typescript.md](70-typescript.md) | The TS surface, the mirrored pure trio, temporal law, error identities |
| [80-conformance.md](80-conformance.md) | Nine lanes: determinism, commutativity oracle, matrix cells, crash matrices, contention, PITR, parity, pins, fuzz |
| [90-rollout.md](90-rollout.md) | The build lanes (incl. the Lean lane), order, gates, checklist |

House laws throughout: representation over control flow
(`audit/REQUIRED-READING.md`); one way per question; zero `dyn` in our own
Rust; sums for outcomes; parse-all-first; attribution-first; every
requirement names its consumer. The Lean gate is structural: optimism does
not merge before footprint stability (L7) builds.
