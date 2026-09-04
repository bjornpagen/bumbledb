# The per-tenant production contract

This document turns “embedded or hosted on AWS/S3” into concrete operating requirements. It is a proposed readiness standard, not a claim that all listed features are absent and not a deployment change.

## Recommended first production envelope

Start with small, bounded tenant databases; ordinary reads against published local snapshots; published acknowledgments only; one region; trusted application-defined schemas and queries; an explicitly tested local filesystem or S3 backend; and a tested restore path.

Do not initially market arbitrary multi-region availability, unlimited tenant sizes, transparent cross-braid transactions, hostile arbitrary query execution, or exactly-once external effects. These are different products with different obligations.

Use a resident writer for a hot tenant when measured contention justifies it. Keep many cold tenants on demand. This is a placement policy over the same history, not a reason to replace the semantic engine.

## Required contract sheet for each deployment

| Question | Required answer before release |
| --- | --- |
| What does success mean? | Published, recoverable receipt; or explicitly provisional local acceptance with a separate host API |
| What can a reader see? | Published snapshot, its vector, freshness status, and any unavailable braid |
| What may be retried? | Named command with a defined ambiguity/retry contract |
| How long can an operation take? | Deadline, cancellation behavior, queue cap, and result after cancellation |
| What does a tenant cost? | Limits for disk, native memory, retained query state, open handles, pending bytes, and CPU work |
| What is durable? | Exact failure domains covered: process, machine, availability zone, region, credentials/operator action |
| How far back can we restore? | Tested retention horizon, available vectors, and restore-time objective |
| How is deletion completed? | Live state, dictionary, log, checkpoints, remote versions, blobs, exports, and encryption keys accounted for |
| How does schema change? | Versioned migration/cutover/rollback plan and client compatibility policy |
| How does a stuck tenant fail? | Bounded refusal and operator-visible cause; no unbounded wait holding the host hostage |

## OPS-001 — Migration is a fleet operation, even if the engine calls it export/import

**Priority:** P1 before routine schema evolution in hosted applications. **Classification:** architecture/operations gap, not a discovered engine corruption defect.

The current engine intentionally identifies a theory by fingerprint. Changing a closed vocabulary or schema may require a new admitted instance and store. In one embedded process, stopping the app and rebuilding may be reasonable. Across a tenant fleet and concurrent deployments, it needs orchestration.

Required design:

1. Allocate a new database incarnation and destination namespace.
2. Export from a named source vector/snapshot.
3. Transform with a versioned, deterministic migration program.
4. Admit the complete destination state and independently compare invariants/counts/selected application queries.
5. Decide whether writes are paused or a catch-up mechanism exists; do not pretend the two approaches are equivalent.
6. Atomically switch the tenant routing record.
7. Reject obsolete writes and stale incarnation tokens.
8. Retain a rollback source for a documented interval, accounting for writes after cutover.

Evidence for the need: schema fingerprint gates in `crates/bumbledb-log/src/manifest.rs`, engine export/import/compact paths under `crates/bumbledb/src/api/db/`, and breaking-version history in `ts/PUBLISHING.md`. This audit did not inspect any external application's migration control plane.

## OPS-002 — Durable truth needs protection from mistakes as well as process failure

**Priority:** P1 before entrusting irreplaceable data to one bucket/prefix. **Classification:** deployment requirement.

`examples/lambda/README.md:25` specifies a Standard S3 bucket without versioning/lifecycle and makes application GC the retention authority. That is a concrete example choice, not a universal backup guarantee.

The identified GC defects make the distinction urgent: replication and a live checkpoint protect against losing a local worker; they do not automatically protect against an incorrect deletion algorithm or an operator deleting the authoritative namespace.

Decide an independent recovery policy: protected backup copies, a separately controlled retention mechanism, or another recovery root outside the destructive authority of ordinary writer/GC credentials. Measure restore from it. If object versioning is used, test its interaction with conditional operations, delete markers, cost, and erasure requirements; do not enable it blindly as an audit “fix.”

Separate roles where practicable: ordinary log publication, checkpoint publication, retention deletion, administrative restore, and tenant routing. A prefix string is a routing choice, not an authorization boundary.

## OPS-003 — The leaf-blob pattern requires its own commit and cleanup protocol

**Priority:** P1 for applications that put payloads in S3 and references in Bumbledb. **Classification:** product-integration requirement.

Keeping large documents/media out of narrow relational metadata is a good fit. But the relational transaction cannot atomically commit an arbitrary remote blob upload.

Recommended ordering:

- Upload immutable/content-addressed content first.
- Verify the upload's content identity and availability.
- Commit the reference plus any command receipt in Bumbledb.
- Treat abandoned uploads as orphans subject to conservative delayed cleanup.

For deletion, first remove or tombstone the reference in a published transaction. Delete the blob only after the chosen reader/backup/restore retention window no longer requires it. A restored database that references already deleted blobs is not a successful restore.

External side effects—email, payment capture, webhooks—need an outbox/effect record plus an idempotent dispatcher or equivalent application protocol. A database success receipt does not make a network side effect exactly once.

## OPS-004 — Tenant isolation must cover resources and lifecycle, not only key spelling

**Priority:** P1 for shared hosts. **Classification:** production requirement tied to confirmed SDK lifetime defects.

Path validation is valuable, but an application must still map authenticated identity to the correct tenant. Never use a raw user-supplied tenant string as the authorization decision. Keep the routing and credential boundary outside the low-level store API.

Bind every local cache to the authoritative database incarnation before serving data or recovering pending work. Opening a same-schema cache under a different namespace currently can return the previous tenant's facts (SDK-016); case-distinct tenant names can also collide on supported local filesystems (REP-011). A cache mismatch must refuse or deliberately reseed, never silently adopt. Test backend/bucket changes as well as changed prefix strings.

The required resource dimensions are:

- Local allocated disk, including scratch/copy overlap and deleted-but-open files.
- Native heap, LMDB mappings, image cache, prepared query memoization and answer buffers.
- Concurrent/opening tenants, live borrows, writer queues and outstanding object requests.
- Per-query output/derived tuples and CPU time.
- Per-command bytes/rows and reservation activity.

Eviction needs an actual borrow object, not a shared mutable pool entry. Closing the pool must account for in-flight opens and renewal tasks. Returning a handle after disposal is a lifecycle violation, not merely a cache inefficiency. See `30-sdk-hosting.md` and `31-ffi-packaging.md`.

## OPS-005 — Unavailable or stale must be distinguishable from empty

**Priority:** P1 API/host policy. **Classification:** guarantee requirement.

A corrupt/wedged braid can coexist with healthy braids. Partial service is reasonable only if a caller knows whether its query depends on the unavailable braid. A normal-looking empty or old result can trigger incorrect provisioning, deletion, or duplicate creation.

Require a read contract carrying one of:

- A published snapshot meeting a requested session minimum.
- A stale snapshot explicitly allowed by host policy, with its actual vector.
- A typed refusal because a required component is unavailable.

Do not let “wait for this vector” consume a request forever. Deadlines must also interrupt ongoing refresh/catch-up work, not merely stop sleeping between complete refresh passes.

## OPS-006 — Publish and recovery need first-class observability

**Priority:** P2 before operating a fleet. **Classification:** operational capability requirement.

The engine's trace and benchmark counters are useful, but operators need protocol and tenant signals at ordinary production cost. Required minimum:

- Tenant/incarnation and current published vector.
- Pending age, pending bytes, and whether a prior command remains unresolved.
- Commit queue length, slot losses, last contention cause, retries, ambiguous outcomes.
- Checkpoint age, vector, byte size, publication attempts, and last completed restore verification.
- Replica replay distance/bytes, cold-open duration, repeated reseeds, wedged braids.
- Resource occupancy versus limits; native handle count; eviction outcomes.
- GC deletions, eligibility reason, retained recovery floor, and failures.

Avoid logging raw facts or credentials in ordinary telemetry. Violation data is valuable but may contain sensitive application values. Use structured redaction and opt-in diagnostic detail.

## Minimum operational drills

1. Lose the acknowledgment after publication; retry the command.
2. Kill the writer at each pending/publication boundary; restore into a clean directory.
3. Pause a holder longer than its lease; let a successor progress; resume the old holder.
4. Keep a replica offline through retention, then attempt reads and writes.
5. Race checkpoint publishers while writes continue.
6. Recover a named historical vector and verify application-level references, including blobs.
7. Migrate a tenant while old and new application versions coexist.
8. Exhaust the local disk and memory admission budgets during open, commit and checkpoint.
9. Rotate credentials during an in-flight request and verify the retry outcome.
10. Dispose the tenant pool while opens, reads, and renewal callbacks are outstanding.

These drills should produce machine-readable history and bounded outcomes. A database is operable when the response to each failure is known before the incident, not after reading its source under pressure.
