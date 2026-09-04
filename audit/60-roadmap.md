# Roadmap: make the strongest claims true end to end

This is proposed follow-up work, not code implemented by the audit. Stages express dependencies and exit criteria, not calendar estimates. Keep the dated findings as the baseline.

## Target

A tenant-sized database with executable relational laws, fast embedded reads, and a hosted history that can be trusted after a crash, a retry, a pause, a migration, or an operator mistake. The core engine remains. The log remains the newest deployment layer. The project gains explicit contracts at the boundaries where the current design relies on an inference the substrate does not support.

## Stage 0 — Ratify the contract and capture failing schedules

**Deliverables**

- A one-page guarantee sheet for embedded, single-host replicated, and S3-backed deployment modes. Separate local accepted, published, rejected, retry-required, and unknown outcomes.
- Decisions on published-read visibility, database incarnation, retirement authority, and exclusive allocation identity. See [91 — Decisions](91-decisions.md).
- Permanent small regressions promoted from [11](11-replication-test-evidence.md), [22](22-engine-test-evidence.md), and [32](32-sdk-test-evidence.md). Do not preserve the current incorrect results as desired goldens.
- A schedule harness with barriers around store calls, local durable writes, candidate apply, native close, lease transitions, and cleanup.

**Exit:** each reproduced release blocker has a regression demonstrating the bad behavior on this baseline; every static P1 has a concrete executable schedule or a documented reason it cannot yet be injected. Each guarantee has an owner and a falsifiable test.

**Immediate operating posture:** do not entrust irreplaceable tenant data to the current hosted durability/retention contract. Keep recovery roots independent of application cleanup and avoid promising an untested restore window. A single resident writer reduces some contention but does not repair retention, cache identity, input ownership, or close semantics.

## Stage 1 — Close local semantic and immutable-command boundaries

**Primary findings:** ENG-001/002/003/004/005/007/008; QRY-001; SDK-001/003/008/014/015; REP-016.

**Work**

1. Inventory every constructor/codec that produces a supposedly admitted value. Seal or validate the actual boundary; retain optimized generated paths only with a defensible trust contract.
2. Make compacted rows, dictionary, counters, and generation come from one held source snapshot. This is a prerequisite for trustworthy checkpoint certificates.
3. Turn recorded commands into owned immutable data; invalidate recorders after callback exit. The exact persisted command must be the command applied and replayed.
4. Make unresolved Pending an exclusive state. Every subsequent operation must settle, reseed/rollback safely, or return an explicit unresolved outcome; it must never replace the evidence.
5. Separate ordinary published snapshots from candidates. A read-only wrapper prevents unlogged writes, but does not itself prevent a dirty candidate from being read. Choose the publication/isolation mechanism deliberately.
6. Give error outputs one contract. Query failure must not leave ambiguous partial current results; semantic rejection must not conceal a durability error.
7. State whether fresh IDs can escape before durable transaction return. Distinguish provisional identity from durably reserved identity; state callback re-execution rules.

**Exit:** failed publication followed by another command is recoverable; mutable host inputs cannot change meaning; no ordinary reader sees a candidate that later rejects; no public supported ingestion path admits invalid representation; compaction metadata matches content; error and provisional-ID contracts have downstream tests.

**Avoid:** hiding the exposed database and calling visibility solved; rerunning arbitrary user callbacks to manufacture serializability; weakening set atomicity to stream huge unbounded batches.

## Stage 2 — Make ownership and database identity real

**Primary findings:** REP-005/009/010/011/017; SDK-002/004/005/006/007/013/016; ARCH-004.

This stage can run alongside Stage 1 after lifecycle and visibility responsibilities are agreed.

**Work**

1. Add authoritative logical database identity/incarnation and bind local state to it before reads, pending recovery, or scratch cleanup. Test same-schema/equal-vector mismatches and backend changes.
2. Define an injective portable tenant-to-directory mapping over every supported filesystem. Refuse unsupported assumptions rather than silently aliasing.
3. Acquire ownership before any local or remote recovery cleanup. Hold it through native close and protected-directory teardown.
4. Replace lease-check-then-rename reasoning with an actual atomic exclusion/fencing primitive. Persist object body and version/fence as one recoverable state. Test suspension after the last check, not only death at phase boundaries.
5. Return a fresh, idempotently releasable borrow capsule for each tenant acquisition. Capture the slot epoch. A borrow's disposal returns the borrow; only the pool closes the database.
6. Unify open/closing/closed/lost/poisoned lifecycle checks inside queued operations. Pool shutdown joins opens and timers; ownership loss revokes all writers.
7. Expose deterministic native release appropriate to the public ownership model. Dead C diagnostics must not retain an engine; choose a bounded handle-lifetime contract.

**Exit:** stale or disposed capabilities cannot read/publish/delete; one borrow cannot release another; cache reuse cannot cross a namespace; close permits expected reopen and bounds resource retention; a resumed old holder cannot overwrite or erase a successor's state.

**Avoid:** merely increasing lease TTL; relying on GC finalizers for fleet resource limits; writing tenant identity only into a directory name; closing wrapper objects while native owners remain alive.

## Stage 3 — Establish monotone publication and retained history

**Primary findings:** REP-001/002/003/004/006/007/008/013/014/018/019; ENG-003 prerequisite.

**Work**

1. Specify the retirement mechanism before optimizing GC: a stale or paused publisher must be unable to publish into history recovery no longer visits. Evaluate epoch-qualified namespaces, durable tombstones, or another store-enforced authority. A preflight manifest GET is not enough.
2. Make allocation ownership distinct from content confirmation. Preserve ambiguous generic store outcomes; if ownership is recoverable, prove a unique request identity. Stop treating writer ID as a resource epoch.
3. Require every authoritative recovery floor to dominate its predecessor componentwise. Do not use a vector sum as the safety order.
4. Couple checkpoint predecessor construction with the actual manifest CAS incumbent. On movement, re-evaluate safety and rebuild/re-address the candidate or choose an explicitly indexed history graph.
5. Persist retention eligibility/age and discoverable cleanup progress. Retained history must remain reconstructible at both edges of the advertised window, including after restart and clock changes.
6. Classify scratch using durable publication/reachability evidence, not equality with the current head. Retain failed cleanup locators until completion; define how host loss affects discovery.
7. Verify content-addressed identities and exact replay boundaries. A coherent compacted image must carry the vector/chain identity of that same state.
8. Permit useful checkpoint progress under sustained writes without demanding that a long compaction observe no change. Safety takes precedence over publishing a stale candidate.

**Exit:** every acknowledged receipt is reconstructible from a clean directory after retention and concurrent publication; no floor component regresses; no two callers own one fresh range; every retained published checkpoint is discoverable; process restart does not shorten retention; GC cannot remove an in-flight or retained dependency.

**Required backend evidence:** memory/reference store, supported real filesystems with subprocess pause/death, and an explicitly authorized real S3 conformance/fault campaign. A simulated S3 outcome is an excellent regression but not cloud qualification.

**Avoid:** fixing clock equality while leaving time only in process memory; fixing one stale backlink without proving discovery under CAS retries; removing all cleanup indefinitely and treating storage growth as resolved.

## Stage 4 — Make application intent and host limits explicit

**Primary findings:** ARCH-001/002/003/004; QRY-002/003; REP-012/015/020; SDK-009/010/011/012; PERF-002/005.

**Work**

- Add an optional published-state precondition for read-dependent commands. Keep blind set-effect commits available. Test two concurrent decrements by business effect, not merely schema validity.
- Provide a named-command receipt pattern/helper: request ID, input digest, outcome/reference, tenant incarnation, and deduplication horizon. Validate crash-before-response retries. Coordinate external effects through an outbox/idempotent consumer rather than claiming exactly-once networking.
- Preserve successful prefix receipts on split infrastructure failure. An incomplete result must say what is known to have published and what remains unknown.
- Carry correctly scoped dependency vectors through sessions. State stale-valid, minimum-vector, and unavailable-braid read behavior separately.
- Expose execution contexts with bounded work, memory/output, cancellation, and deadline semantics. Charge before or within bounded growth, not after materialization.
- Enforce tenant admission before expensive opens; include native resources, scratch overlap, in-flight work, retained plans and results. Use bounded workers where synchronous calls cannot meet the shared-host contract.
- Make lease/GC maintenance proportional to current live state or bounded progress, not the tenant's entire operational lifetime.

**Exit:** clients can safely retry named commands, distinguish precondition movement from semantic rejection, interrupt expensive work with a bounded outcome, and observe overload as a deliberate refusal instead of an unbounded wait or process-wide failure.

**Avoid:** a timeout wrapper that leaves native work running indefinitely; a tuple setter advertised as a memory cap; a universal serializable/cross-braid transaction claim hidden behind the existing commit verb.

## Stage 5 — Build the operational product around the capsule

**Primary inputs:** [03 — Production contract](03-production-contract.md), ENG-006, SDK/FFI packaging review, ASS-003/004.

**Deliverables**

- Versioned tenant routing and migration/cutover/rollback procedures; source and destination incarnation handling.
- Independent recovery roots, least-privilege roles appropriate to the deployment, and historical restore drills that also validate external blob references.
- A data-lifetime policy covering dictionary text, local files, logs, snapshots, remote versions, backups, exports, and keys. Logical delete must not be sold as secure erasure.
- Structured runtime diagnostics: publication certainty, pending age, current vector, replica health, checkpoint/recovery age, request amplification, resource occupancy, and cleanup reasons. Redact tenant fact data by default.
- Supported OS/filesystem/Node/libc/S3 matrix and source-versus-packed artifact provenance. Fresh native builds, packed imports, compatibility/reopen tests, and C resource-lifetime tests on a controlled release tree.
- Current operational documentation separated from historical research. Keep exact finding-to-fix-to-regression links.

**Exit:** an operator can recover, migrate, delete, diagnose, and bound a tenant without reconstructing protocol rules during an incident. The published deployment envelope matches tested behavior.

## Stage 6 — Earn the performance envelope on real applications

Only now decide which optimization buys the most end-to-end value. See [40 — Performance](40-performance.md) for the measurement matrix.

Use at least three modeling styles: relational booking/capacity, graph/knowledge metadata, and identity-bearing event/ledger facts. Use actual intended application schemas when available; synthetic schemas should expose their braid structure and missing complexity.

Measure cold open, warm reads, first read after mutation, hot-braid contention, ID refill, tenant churn, checkpoint/GC overlap, and recovery. Retain all runs; report absolute p50/p95/p99/p99.9, RSS/peak disk, object calls/bytes per accepted command, and failures. Qualify dataset, hardware, warmth, read/write mix, and durability mode.

Potential decisions after evidence: chunked images or delta overlays; pressure-aware cache trimming; resident-writer placement; schema-level immutable plan templates with tenant-owned execution state; pure transition kernel; finer coordination domains. None is preapproved by this audit.

**Exit:** the database has a measured useful envelope and the host refuses or relocates work outside it. Performance improvements preserve the fault-history regression suite and do not quietly weaken success, isolation, or retention.

## Definition of done for the next campaign

The result is not “all audit rows marked green.” It is that the same few invariants explain the public API, storage protocol, host lifecycle, tests, proof premises, and runbooks. A finding is closed only with evidence; a narrowed capability is a documented product decision. New optimization work should have fewer independent correctness obligations than the system it replaces.
