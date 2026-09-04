# Architecture and guarantee boundaries

## The system that exists today

`bumbledb-log` is the newest deployment/replication layer, not a replacement relational engine. Both Rust and TypeScript ultimately use `bumbledb` for schema judgment and query execution.

```text
Application schema / query / concrete write intent
  ├─ Rust macros and public API
  └─ TypeScript DSL → native bridge
                 ↓
  bumbledb-theory + validated engine schema/query representation
                 ↓
  bumbledb: admission, LMDB/owned instances, prepared Free Join execution
                 ↑
  bumbledb-log Rust driver / ts-log async driver
    ├─ one Rust protocol grammar exposed through the native bridge
    ├─ per-braid immutable log slots
    ├─ manifest → checkpoint document → compact LMDB image
    ├─ local chain/pending state, leases and ID counters
    └─ filesystem / memory / S3-compatible object-store adapters
                 ↑
  tenant pool / host integration / scheduled checkpoint duty
```

The architecture already has a valuable split: the semantic engine and byte grammar are centralized; the asynchronous host machines remain language-specific. The dangerous seam is that both machines must independently implement the same publication, ownership, and recovery obligations.

Primary evidence: `crates/bumbledb/src/lib.rs:62`, `crates/bumbledb-log/src/lib.rs:1`, `ts-log/src/codec.ts`, `ts/crate/src/log.rs`, `proposals/one-core/00-thesis.md`.

## What each layer can honestly promise

| Layer | Intended guarantee | Boundary that must remain visible |
| --- | --- | --- |
| Schema/value admission | Supported values and constraints have validated representations | Public unchecked construction paths invalidate the premise; see engine report |
| Local engine write | A transaction's committed facts satisfy declared laws | Read-compute-write outside the transaction needs a witness |
| Local snapshot | Consistent LMDB state | It may be a speculative log-writer state rather than published history |
| Single-braid log | Concrete batches acquire an order through create-only slots | Requires safe handling of ambiguity, floors, pending state and exact bytes |
| Multiple braids | Declared obligations do not cross components | No automatic global transaction, causal cut, or cross-component read dependency |
| Session vector | Reader has reached minimum per-braid positions | Needs the correct tenant/incarnation and all relevant dependencies |
| Checkpoint | A durable materialization of a specific vector and chain heads | Sum is not a substitute for vector dominance; metadata must match one snapshot |
| Retention | Restore history remains available for the declared window | Time and reachability must be durable protocol data |
| Tenant pool | Bound local resource ownership and reuse | Disk size alone does not bound native memory, live readers or in-flight work |

These are intended contracts, not a certification of the current implementation. Several are falsified by the companion reports.

## ARCH-001 — Schema-valid replay is not serializable application read-modify-write

**Priority:** P1 architectural/API gap for general application use. **Confidence:** confirmed contract gap; the example below is a semantic counterexample, not a newly executed test.

**Evidence:** `ts-log/src/writer.ts:143` explicitly makes the batch a write-only recorder; `crates/bumbledb-log/src/writer/discipline.rs:56` rejudges the same recorded operations and never reruns the host body. By contrast, local `Db::write_from` has an explicit catalog/generation witness (`crates/bumbledb/src/api/db/write.rs:62`, `:115`). The log surface does not offer that contract over published braid state.

**Counterexample:** a keyed counter is 10. Two hosts read 10 and each records delete `(id,10)` plus insert `(id,9)` for “decrement once.” The first reaches 9. Replaying the second fixed fact transformation is a net no-op and can be accepted at the current slot. The result is 9, not the 8 required by two successful decrement commands. Every key constraint still holds. This is legal serial replay of the recorded effects, not a serial execution of both original read-dependent commands.

**Why it matters:** booking and capacity constraints catch some invalid intent, but arbitrary business preconditions are not automatically represented in the schema. A valid final state is necessary, not sufficient, for correct command execution.

**Direction:** define an opt-in conditional commit with a published-state witness or explicit expected facts. On movement, return a distinct retry-required outcome. Start braid-local; do not smuggle cross-braid atomicity into an existing verb. Keep blind set writes cheap. Teach immutable event facts and derived totals where that is the better model.

**Acceptance test:** two concurrent named decrements either both have their intended effect or one explicitly reports a precondition movement; two published success receipts must not be mistaken for two executed decrements.

## ARCH-002 — Braid independence is about declared obligations, not all observations

**Priority:** P1 guarantee/documentation decision. **Confidence:** confirmed model boundary.

**Evidence:** `crates/bumbledb-log/src/braids.rs:104` partitions by statement edges. `lean/Bumbledb/Txn/Braids.lean:175` proves locality of application and statement judgment. General queries can read several relations regardless of whether the schema has a dependency edge. `replica.rs:837` catches up one slot per braid per round and stops probing a braid once it saw a tip.

**Counterexample:** a host publishes A, then publishes B after observing A. A reader can probe A before A exists and B after B exists, obtaining A0/B1. That cut can satisfy every declared constraint while violating the host's causal expectation. A receipt carrying only B's coordinate does not by itself encode the dependency on A.

The existing explicit `commit_split` is a good API decision: normal per-braid accepted/rejected outcomes are explicit and deserve a distinct verb. Infrastructure failure after an earlier success can still discard accumulated outcomes from the return value (REP-020), so partial-success recovery needs strengthening. Describing all cross-braid interleavings as universally “semantically invisible” is too broad.

**Direction:** state the default as a product of valid per-braid prefixes, not a globally freshest snapshot. Define session semantics using captured and propagated dependency vectors. A full vector enforces the dependencies it contains; it does not discover missing causal dependencies or establish global causal consistency by itself. Specify whether a query requires a session minimum and what happens if any relation's braid is wedged. Cross-braid transactions are a separate capability, not something to infer from L9.

**Acceptance test:** scripted interleavings demonstrate stale-valid reads, session-constrained reads, and explicit split failure. Documentation states which are guaranteed and which are not.

## ARCH-003 — End-to-end command identity is missing from the default hosted workflow

**Priority:** P1 before retryable billing, provisioning, or other consequential commands. **Confidence:** architectural gap already acknowledged in historical research.

**Evidence:** a log receipt carries braid/slot/value/durability (`ts-log/src/writer.ts:104`); fresh IDs are allocated separately; `docs/research/replication-prior-art/IMPOSSIBLE.md` explicitly records host-retry duplication as deferred.

**Scenario:** a purchase insert with a freshly reserved ID is published, but the response is lost. The application retries and reserves a different ID. Both rows are unique facts and both can be admitted. The log did not duplicate a batch; the application duplicated a command.

**Direction:** a named request receipt stored atomically with the application's facts. Include request ID, request-body digest, outcome/reference, and tenant incarnation. Reuse with the same digest returns the recorded outcome; reuse with a different digest refuses. A keyed receipt relation may express the first implementation without a new engine primitive, but the SDK must make winner retrieval and retry behavior straightforward. Retention of receipts is an explicit deduplication horizon.

**Acceptance test:** kill the host at every point before/after publication and before response delivery, then retry the same command. Exactly one business effect, stable returned identity, and mismatched-body rejection.

## ARCH-004 — Session positions need a namespace and an incarnation

**Priority:** P1 before migration/restore and externally supplied session tokens. **Confidence:** confirmed API boundary gap, not evidence of a current authorization bypass.

**Evidence:** `crates/bumbledb-log/src/vector.rs:22` and `ts-log/src/vector.ts:17` represent only braid/count maps. The current schema fingerprint protects grammar compatibility, not “this tenant's history before versus after restoration.”

**Scenario:** a session token from another tenant with the same schema, or from a replaced/restored history, can name plausible braid IDs and unreachable or misleading generations. Blind waiting can hang; accepting an equal coordinate can satisfy the wrong historical obligation.

**Direction:** host-level receipt envelope `{tenant, databaseIncarnation, schemaFingerprint, vector}`. Authenticate it when it crosses an untrusted client boundary; validate it before waiting. Restoration into a new logical history gets a new incarnation even if the schema fingerprint is identical; exact recovery of the same authoritative history need not rotate it. An internal vector can legitimately rely on host scoping, and TS already checks braid membership. The missing contract is provenance for transferred tokens and changed histories. Decide how old sessions are invalidated, not just how bytes are copied.

The same identity must bind **local materializations**, not just transferred tokens. SDK-016 reproduced a process opening tenant B with tenant A's same-schema, equal-vector cache and serving A's facts. That is a configuration error the current open path fails to detect; REP-011 supplies a filesystem alias that can cause it without explicitly reusing a directory. Validate authoritative incarnation against the local cache before adopting facts, pending commands, or cleanup authority. A correct routing string, schema fingerprint, and scalar generation do not establish history identity. This is a demonstrated isolation failure, distinct from the session-token contract gap above.

## ARCH-005 — The shared core stops before the hardest state machines

**Priority:** P2 architectural improvement, after urgent correctness fixes. **Confidence:** confirmed structure and recurring defect pattern.

One production parser is an excellent consolidation. Yet Rust and TypeScript still independently encode pending settlement, stale-floor handling, gate/disposal behavior, and resource ownership. Byte-identical goldens cannot discover that one driver overwrites a pending batch after an exception.

Do not force JavaScript I/O into synchronous Rust calls. Instead consider a small pure transition kernel or executable reference machine:

- Input: explicit state plus an event/result.
- Output: next state plus required effects.
- Host effects: read/write/sync/network/clock/timer only.
- Recovery: feed durable state and observed objects through the same transition law.

At minimum, drive both existing machines with the same schedule corpus and compare receipt histories, visibility, vectors and recovered state—not just encoded bytes. A third reference model should be simpler than either production machine.

## ARCH-006 — Schema-derived braids are not tenant or row sharding

**Priority:** P2 capacity-planning requirement. **Confidence:** confirmed mechanism.

A normal relational application often has a large connected component through its references. Two writes to completely different customer/account keys within that component still claim the same next log slot. `serial_at` reports special global-determinant cases; an empty `serial_at` does not mean the braid has no serial publication bottleneck.

Current losses reopen and replay, so competing hosts can pay far more than one object PUT per successful command. The old per-key footprint thesis must not be used as a throughput forecast for the current implementation.

**Direction:** measure braid sizes and contention using actual application schemas. Keep per-tenant partitioning as the coarse scaling unit. Use a resident writer for sustained hot tenants if measurements justify it. Only consider finer partitioning after quantifying the coordination and migration consequences of every cross-partition law.

## Coherent target architecture

The smallest useful target is not more database machinery everywhere. It is explicit capabilities around one engine:

1. A trusted, admitted theory/value boundary.
2. A command that owns immutable facts and optional published-state preconditions.
3. An internal candidate state invisible to ordinary readers.
4. A permanent published receipt or an explicit unresolved/retry state.
5. A published snapshot with a named vector/incarnation.
6. A monotone checkpoint chain and conservative, auditable retention.
7. A tenant borrow whose release deterministically releases the resources it claims to own.

The safest order is to repair existing contracts before adding speculative optimizations. That preserves the project's compactness while making its strongest claims true across the entire stack.
