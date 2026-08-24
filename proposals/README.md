# bumbledb-log — the braided object log

The PRD set for `bumbledb-log` (Rust) and `@bjornpagen/bumbledb-log` (TS):
durability, replication, backup, PITR, per-tenant sharding, **and
theory-derived concurrency** for bumbledb, over conditional-write object
storage. No servers; a resident writer is a mode.

The thesis: the schema's statement graph shards the log into
independent braids that provably never conflict (L9) — concurrency is
derived from the declared theory, machine-checked, never managed.
Invariants are never weakened (this is not a CRDT): contention within a
braid resolves through one loss path to the verdict a serial execution
would have produced, with violations as data. Set semantics does the
same to recovery: re-application is a proven no-op (L10), so recovery
*is* replay — no intent fields, no forced-case tables, no
applied-watermark machinery.

These documents are **normative**: they bind the build the way the
engine's laws bind the engine — the code implements them or reports the
gap; it never improvises past them.
Read [00-product.md](00-product.md) first; [10-protocol.md](10-protocol.md)
is the centerpiece; [90-rollout.md](90-rollout.md) is the fleet dispatch.

| Doc | Contract |
| --- | --- |
| [00-product.md](00-product.md) | What it is, the laws, the five deployment cases, non-goals |
| [10-protocol.md](10-protocol.md) | Braids, keys, manifest + vector, log slots, checkpoints, leases, PITR, gc |
| [20-command-codec.md](20-command-codec.md) | The batch wire format (v2, header + ops), determinism laws, IDL refusal |
| [30-engine-seams.md](30-engine-seams.md) | The one engine addition; the engine never learned replication exists |
| [40-object-store.md](40-object-store.md) | The five-verb capability, vendor matrix, verb-consumer map, dependency rulings |
| [50-replica.md](50-replica.md) | The chain sidecar (recovery is replay), catch-up, gc heartbeat, Vercel, tenants |
| [60-writer.md](60-writer.md) | One commit discipline, the publish law, the one loss path, group commit, reservations idiom |
| [70-typescript.md](70-typescript.md) | The TS surface, the mirrored pure pair, temporal law, error identities |
| [80-conformance.md](80-conformance.md) | The lanes: determinism, braid convergence, serial verdicts, crash matrices, contention, PITR, parity, pins, fuzz, interop, multi-process |
| [90-rollout.md](90-rollout.md) | The build lanes (incl. the Lean lane), order, gates, checklist |

The grail directory is retired; numbered docs are the law.
