# bumbledb-log — the object log

The PRD set for `bumbledb-log` (Rust crate) and `@bjornpagen/bumbledb-log`
(TS package): replication, durability, backup, point-in-time recovery, and
per-tenant sharding for bumbledb, implemented as a single-writer command log
over conditional-write object storage. No servers required; a resident
writer is one deployment *mode*, not a component.

These documents are **normative** in the same sense as
`docs/architecture/`: they are the contract the implementation is verified
against, written before the code, updated in lockstep with it. Read
[00-product.md](00-product.md) first. [90-rollout.md](90-rollout.md) is the
self-contained build plan for an agent fleet.

| Doc | Contract |
| --- | --- |
| [00-product.md](00-product.md) | What it is, the four deployment cases, non-goals, laws |
| [10-protocol.md](10-protocol.md) | Keys, manifest, log objects, checkpoints, CAS rules, PITR, truncation |
| [20-command-codec.md](20-command-codec.md) | The command wire format and the determinism laws |
| [30-engine-seams.md](30-engine-seams.md) | The one engine addition and the guarantees the engine already provides |
| [40-object-store.md](40-object-store.md) | The store capability trait, vendor matrix, dependency rulings |
| [50-replica.md](50-replica.md) | Replica lifecycle: pull, replay, refresh, Vercel, per-tenant LRU |
| [60-writer.md](60-writer.md) | The two writer modes, group commit, crash recovery, failover |
| [70-typescript.md](70-typescript.md) | The TS package surface and its temporal law |
| [80-conformance.md](80-conformance.md) | Determinism lanes, crash matrix, contention lane, cross-codec goldens, gates |
| [90-rollout.md](90-rollout.md) | The overnight build: lanes, file ownership, order, acceptance |

House laws apply throughout: representation over control flow
(`audit/REQUIRED-READING.md`), one way to do each thing, zero `dyn` in our
own Rust (dependency internals exempt, like `heed`), no allocation on
steady-state paths that are not already network-bound, attribution-first for
any performance claim, and every requirement names its consumer.
