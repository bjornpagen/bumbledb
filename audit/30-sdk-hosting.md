# TypeScript SDK, replicated application semantics, and tenant hosting

Audit date: 2026-09-04. Scope: the working tree, including the current `ts-log` implementation and existing uncommitted packaging/key-grammar work. No implementation files were changed. This is a review of what an application can actually observe, not just the protocol codec in isolation.

## Executive judgment

The strongest idea is worth preserving: an application database whose schema is an executable theory, whose rejected changes are useful domain data, and whose tenant-local working copy is a materialized view of authoritative object-store history. Deriving independent braids from the theory is a substantially better foundation than asking application programmers to guess which constraints can tolerate concurrent writes.

The highest-risk gap is at the boundary between that idea and a shared asynchronous JavaScript host. The codec now has one Rust implementation, but the TypeScript writer, recovery, read visibility, directory ownership, and tenant-pool lifecycle are still independently implemented state machines. Correct bytes do not make those state machines correct.

This review reproduced accepted local/log divergence, writes after disposal, a disposed pool returning a newly opened replica, and an ordinary read observing a transaction that ultimately returns **rejected**. These are release blockers for a shared per-tenant application host. They are not arguments against the database's relational/set-semantic foundation.

Severity convention: P1 = correctness, isolation, durability, or availability defect to fix before relying on the affected hosting mode; P2 = bounded operational/scaling risk or a public contract that needs strengthening. `Reproduced` means a focused runtime probe exercised the current TypeScript sources with the installed native artifact. `Confirmed static` means the source establishes the failure path, but that particular interleaving was not executed. Recommendations are proposals, not changes made by this audit.

## Findings index

| ID | Priority | Finding | Evidence |
| --- | --- | --- | --- |
| SDK-001 | P1 | A new commit overwrites an unresolved pending commit | Reproduced |
| SDK-002 | P1 | A writer publishes after its replica has been disposed | Reproduced |
| SDK-003 | P1 | Mutable byte cells make local application differ from the published batch | Reproduced |
| SDK-004 | P1 | Tenant borrows share one handle; release is not tied to a borrow or slot generation | Reproduced |
| SDK-005 | P1 | Pool disposal does not join in-flight opens | Reproduced |
| SDK-006 | P1 | Tenant directory lease renewal fails open on lost ownership | Reproduced with clock injection |
| SDK-007 | P1 | Replica disposal does not deterministically release the native database | Confirmed static; deliberate API choice |
| SDK-008 | P1 | `replica.db` exposes unrestricted local writes that bypass the log | Reproduced; public capability defect |
| SDK-009 | P2 | A supported async commit callback can deadlock its own replica gate | Confirmed static |
| SDK-010 | P2 | Wait, open, refresh, and recovery lack cancellation/work budgets | Confirmed static |
| SDK-011 | P2 | Tenant memory/disk limits are advisory, not admission control | Confirmed static; documented limitation |
| SDK-012 | P2 | Filesystem lease cleanup grows with all historical acquisitions | Confirmed static |
| SDK-013 | P1 | C ABI destroy leaks engine-owning read handles | See `31-ffi-packaging.md` |
| SDK-014 | P1 | Other requests can read a candidate that later returns rejected | Reproduced |
| SDK-015 | P2 | Commit callback replay can duplicate application side effects | Confirmed static; intentional mechanism, insufficient public contract |
| SDK-016 | P1 | Reusing a local cache for another same-schema namespace serves the first tenant's data | Reproduced across processes |

## SDK-001 — A second commit can destroy the recovery evidence for the first

Locations: `ts-log/src/writer.ts:602`, `:657-690`, `:904-905`, `:913-924`, `:935-945`; `ts-log/src/replica.ts:225-235`.

The write discipline persists Pending, applies locally, and then publishes. A failed store call leaves the already-applied candidate in the local engine and its bytes in the sidecar. That is a legitimate recoverable state. However, the next `commit` calls `recordWithLeases` and `disciplineCommit` immediately. It does not settle the pre-existing pending batch. `holdPending` at line 677 replaces it with the new batch. Only opening a writer calls `settleInheritedPending`.

Reproduction used `Ledger`/`Holder` from the existing fixtures and a `memStore` wrapper that throws once on the first log `putCreate`, before delegating:

1. Commit `{id: 1n, name: "one"}`; inject a PUT failure.
2. Catch the error and continue using the same writer.
3. Commit `{id: 2n, name: "two"}`.
4. Open a fresh replica over the same object store.

Observed output:

```text
failed injected PUT failure
second commit: accepted, durability=published, slot=1
writer rows: [{id:1,name:"one"}, {id:2,name:"two"}]
fresh reader rows: [{id:2,name:"two"}]
```

The first caller did not receive an acknowledgment, so merely losing its change would not by itself establish a durability bug. The stronger problem is that the second acknowledged commit is judged against unpublished state, the local generation can exceed the published vector, and the evidence needed to resolve the first change is gone. For example, a later child insertion can be admitted against an unpublished parent and then fail containment during authoritative replay. That dependent-child variant is a source-derived consequence, not an executed reproduction in this audit.

Recommendation: make Pending an exclusive writer state. Every new commit must first finish, roll back/reseed, or explicitly return an unresolved-outcome result for the prior operation. Never replace pending evidence merely because a new callback arrived. Assert the generation/vector invariant at commit entry and immediately before returning an accepted receipt.

Regression tests: failed-before-write, written-then-error, ambiguous-and-absent, GET failure while resolving ambiguity, sidecar-write failure, and contention-limit failure, each followed by a new commit on the **same live handle**, not only a process restart. Include a dependent child and a different-braid next commit.

## SDK-002 — Disposed replicas still have live publishing writers

Locations: `ts-log/src/writer.ts:896-905`, `:913-947`; `ts-log/src/replica.ts:143-155`, `:1031-1035`.

The replica's public reads and refreshes check `core.closed`. The writer's gate entry does not, and `withGate` itself does not enforce lifecycle state.

Reproduced:

```text
await writer.replica[Symbol.asyncDispose]()
await writer.commit(insert Holder(id=1,name="one"))
=> accepted, published, slot=1
fresh replica => Holder(id=1,name="one")
```

Disposal is therefore not a boundary on publication authority. In a tenant pool, a caller can retain a writer after its borrowed replica has been evicted and continue mutating or recreating the old local directory while a new owner is active.

Recommendation: check a shared lifecycle/ownership epoch *inside* every queued operation. Closing must revoke writer capabilities, including writers constructed before closure and operations queued after disposal. Use a state machine (`open`, `closing`, `closed`, possibly `poisoned`) rather than scattered booleans. Opening a writer over a disposed replica should also refuse before any side effect.

Tests: commit after dispose; `openWriter(disposedReplica)`; dispose queued before commit; tenant eviction with a retained writer; repeated disposal.

## SDK-003 — The bytes that were judged are not necessarily the bytes that were logged

Locations: `ts/src/marshal.ts:119-123`, `:134-143`; `ts-log/src/writer.ts:402-419`, `:665-681`; `ts-log/src/replica.ts:313-336`.

`rowOf` retains the original `Uint8Array` for a bytes field. The writer encodes that row, then awaits sidecar filesystem work, then applies the original recorded row. The caller can mutate the array during that await. The encoded bytes and local engine now describe different transactions.

Reproduced with `relation("R", { v: bytes(1) })` and an unconstrained schema:

```ts
const value = new Uint8Array([1])
const result = await writer.commit(batch => {
  batch.insert(R, [{ v: value }])
  setTimeout(() => { value[0] = 2 }, 0)
})
```

Observed: `result` was accepted/published at slot 1; the writer scanned `[2]`; a fresh reader scanned `[1]`. The timer executed during the real sidecar I/O window. No corrupt store or forged types were involved.

The recorder is also never marked spent. Escaped `batch` methods can append operations or draw IDs after the callback returned. Depending on timing, changes after partition/encoding can affect retained arrays or lease pools outside the intended recording phase. That companion path was identified statically; the byte-cell path above was executed.

Recommendation: make the recorded command an owned immutable value at the boundary. Copy mutable cells when recording and invalidate the recorder on all callback exits. Stronger still, apply the decoded, validated command represented by the exact persisted bytes, so local application and remote replay cannot independently interpret mutable host inputs. Benchmark the ownership copies explicitly; correctness is not an optional allocation optimization.

Tests: mutation after `insert`, mutation during callback awaits, mutation during fsync, reusable buffers in iterables, `Buffer` slices, escaped recorder operations after completion, and mutation between contention retries.

## SDK-004 — Refcounts count tenant names, not individual borrows

Locations: `ts-log/src/tenants.ts:192-212`, `:280-295`, `:339-369`, `:392-394`.

Every `get("a")` returns the same branded replica object. Its `release` closure looks up whichever slot currently lives under tenant name `a` and decrements its aggregate counter. It has no per-borrow spent state and captures no slot generation.

Reproduction:

```text
first = await pool.get("a")
second = await pool.get("a")
first === second => true
first.release(); first.release()
await pool.evict("a") => disposed
second.db => "replica is disposed"
```

The double release is caller misuse, but the advertised live-handle abstraction specifically purports to make a still-held borrow protect a replica. It cannot distinguish that misuse from a valid second borrower returning the same object. An even more dangerous consequence is that a released old handle can decrement the refcount of a newly opened slot with the same tenant name.

There is a second ergonomic trap: `LiveHandle` extends `Replica`, which is `AsyncDisposable`. `await using tenant = await pool.get(id)` invokes the replica's *close*, not the borrow's *release*. The pool continues to count a reference to a disposed shared replica.

Recommendation: return a fresh borrow capsule for each acquisition. Capture an immutable slot identity/epoch and an idempotent per-borrow release token. The capsule's async disposal should return the borrow; only the pool should close the underlying replica. Do not expose global `release(tenant)` as if it were equivalent to releasing a specific borrow. Existing capabilities should reject after their own release even if the pool retains the database.

Tests: two independent borrowers; duplicate release; stale release after evict/reopen; nested `await using`; two concurrent requests each with `finally`; explicit pool shutdown while requests hold borrows.

## SDK-005 — Pool shutdown can finish before a replica finishes opening

Locations: `ts-log/src/tenants.ts:302-336`, `:339-369`, `:396-408`.

`get` checks `closed` before beginning an asynchronous open. `openOne` later inserts its slot and arms a renewal timer without another lifecycle check. Pool disposal closes only `open.values()`, not the `opening` map.

Reproduction paused the store's first GET during a tenant open, awaited pool disposal, and then released the GET:

```text
pool disposed
get completed after pool disposal: live handle, count(Holder)=0
```

The open succeeds after shutdown, a native database/lease is orphaned, and a new referenced interval can keep the process alive. This is normal promise interleaving, not forged caller input. A second explicit disposal was used to clean up the probe.

Recommendation: closing must first stop admission, then cancel or join all in-flight opens, then drain/close all admitted slots, then release leases and timers. Any open that completes after the closing epoch must close itself and reject rather than return a handle. Preserve a closing promise so concurrent disposal calls join the same operation.

Tests: close before manifest GET completes; close during native open; close during directory measurement; two same-tenant open waiters; open failure during close; close with multiple tenants in flight.

## SDK-006 — A lost directory lease is treated as successfully renewed

Locations: `ts-log/src/tenants.ts:101-125`, `:232-238`, `:280-295`, `:403-405`; `ts-log/src/store.ts:344-412`.

`renewDirLease` returns success when the held token file is missing, including ENOENT after a replacement attempt. Successor acquisition removes predecessor token files. Consequently, a pool whose lease expired and was superseded can continue renewing successfully in its own view, even though its token is no longer current.

Renewal validates only the old token file's own holder/token, not the current head. Its read-then-rename is not a CAS against ownership advancement. A holder paused between read and rename can recreate an obsolete token. Even when `leaseLost` is set, only later `get` calls consult it; already handed-out replicas and writers continue operating. Shutdown releases the lease before awaiting replica disposal, creating another handover window.

A long event-loop pause, VM freeze, host scheduling delay, or debugger stop is enough to challenge the ownership claim. Renewing every TTL/3 is a liveness strategy; it is not proof that a live replica cannot lose its lease.

Focused reproduction used the real filesystem lease implementation in a temporary pool, advanced `Date.now` by 400 seconds only while acquiring a successor lease, then restored the clock. The successor acquired token 2 and removed token 1. `pool.get("a")` nevertheless returned the old token-1 replica, identical to the prior handle, because missing-token renewal returned success. This executes the fail-open branch; it does not claim a multiprocess scheduler-pause stress test was run.

Recommendation: fail closed on missing/stale ownership, validate the authoritative token before admitting operations, and revoke all derived capabilities on ownership loss. For one-machine local directory ownership, strongly consider a kernel-held lifetime lock rather than a renewable wall-clock lease. If leases remain, the storage mutation itself needs a real fencing guarantee, not a successful prior check. See the replication audit's filesystem fencing finding (REP-005) for the shared store-level issue.

Tests: pause beyond TTL while a second process opens; resume the old process; missing token; renewal read/rename interleaving with takeover; renewal error while a borrow remains in use; shutdown with a store PUT in flight.

## SDK-007 — Disposable replicas still own non-disposable native databases

Locations: `ts-log/src/replica.ts:169-179`, `:656-665`, `:1010-1035`; `ts-log/src/tenants.ts:155-158`; `ts/src/db.ts:8-20`, `:1643-1649`; `ts/src/native.ts:613-637`; `ts/crate/src/lib.rs:499-504`.

The public SDK deliberately omits a database close/dispose verb. Replica disposal only marks closed and persists the sidecar. It neither drops nor closes `core.db`. A caller retaining `replica`, `writer`, or a previously fetched `db` therefore retains the native engine. Discard/reopen rotates directory names and unlinks the old directory while the old environment is left to GC. On Unix, unlinking a mapped/open file does not guarantee its disk blocks are reclaimed.

This is not a speculative garbage-collector bug: the object graph explicitly retains `core.db`, and the public API does not revoke a previously returned `Db`. It is a deliberate engine-SDK lifetime choice that conflicts with a deterministic tenant LRU and `/tmp` resource budget. The native bridge actually *has* `db_close`; the public boundary elects not to expose a safe lifetime wrapper.

Recommendation: design a database owner that deterministically drops its native handle, while read instances and plans remain safe and fail clearly after owner closure. Separate a borrow from an owner. Close old engines before treating their on-disk working sets as reclaimed. If engine internals truly require process-lifetime environments, make the host architecture explicit: a bounded worker process owns each tenant group and recycling a worker is the reclamation mechanism.

Tests: repeated tenant churn with references intentionally retained; RSS, mapped files, file descriptors, and physical disk allocation after eviction; same-path reopen; failed/opened replica cleanup; forced-GC versus no-GC comparisons. Do not accept directory-entry counts as a memory/disk-release proof.

## SDK-008 — The replicated reader exposes an unrestricted local writer

Locations: `ts-log/src/replica.ts:94-100`, `:313-336`, `:1010-1012`; `ts/src/db.ts:364-370`, `:1318-1327`.

`Replica.db` is the complete engine `Db`, including `write` and `writeFrom`. A fully typed caller can execute `replica.db.write(...)` without producing a log command. The returned engine admission looks successful, but the change is not authoritative, may affect later admission, and may be silently removed by replica repair. The writer's own code cannot distinguish an illicit local write from its intended local candidate solely by exposing this same unrestricted value.

Reproduced on an empty writer replica: `replica.db.write` accepted a Holder row at engine generation 1, an immediate scan returned the row, and one `await replica.refresh()` removed it. Observed output was `direct: accepted`, `before refresh: [{id:1,name:"one"}]`, `after refresh: []`.

This contradicts the role claim that a writer is a replica plus the right to publish changes. Reusing one vocabulary is valuable; sharing unrestricted mutation authority is not necessary to reuse that vocabulary.

Recommendation: expose a read capability on a replica, with the mutable engine held privately by the driver. A read-only TypeScript projection is useful but insufficient by itself if runtime objects still leak mutable authority through normal APIs. If local speculative editing is a deliberate feature, give it an explicit staging object and commit/rebase contract, not a normal `Db.write` on the authoritative replica.

Tests: compile-time and runtime rejection of direct replicated-DB writes; inherited write methods through widened types; retained DB handles across repair/disposal. Document whether prepared plans belong to a replica generation, an engine instance, or a theory.

## SDK-009 — Supported async callbacks can await their own serialized gate

Locations: `ts-log/src/writer.ts:169-173`, `:448-474`, `:913-947`; `ts-log/src/replica.ts:143-155`, `:994-997`, `:1017-1021`.

`commit` accepts `R | Promise<R>` and runs the callback inside the replica's exclusive promise gate. Therefore this natural composition never completes:

```ts
await writer.commit(async batch => {
  await writer.replica.refresh()
  // record changes
})
```

The inner refresh queues behind the outer commit, while the outer commit awaits the callback. Nested commit and waitFor needing refresh have the same problem. Another request can also leave all future work queued behind an arbitrary never-settling application promise. No reentrancy error or timeout identifies the cycle.

Recommendation: establish an explicit callback contract and enforce it. Either require synchronous pure recording, detect/reject same-replica reentrancy, or move application awaits outside the commit gate and then atomically admit a captured command. A host-level request deadline must not leave the gate permanently occupied.

Tests: nested refresh/commit/waitFor, thrown callback, callback awaiting an aborted request, and disposal racing an indefinitely blocked callback.

## SDK-010 — There is no way to bound the cost of waiting for a world that may never arrive

Locations: `ts-log/src/replica.ts:90-100`, `:497-528`, `:661-681`, `:977-1004`; `ts-log/src/writer.ts:347-373`; `ts-log/src/store-s3.ts:167-183`, `:273-376`.

`waitFor` polls until dominance with a fixed 10 ms delay and no deadline or signal. `runPass` continues until each braid reaches a tip; a braid receiving writes faster than the replica applies them can keep a refresh/open running indefinitely. Repeated `reseed` outcomes make `discardAndReopen` repeat forever, logging alarms without changing policy. Object-store verbs have no signal parameter; the S3 client has no host-supplied per-request deadline or explicit socket disposal surface here.

Consequences for a per-tenant application host: a malicious or simply mistaken future generation can generate persistent S3 GET traffic; a cancelled HTTP request continues work; a permanently unreplayable authoritative slot can keep rebuilding the same database; a busy tenant can monopolize its serialized gate. A 10 ms idle cadence alone can approach 100 polling rounds/second/client before network latency, each potentially checking multiple braids.

Recommendation: add cancellation, elapsed-time and slot/byte work budgets, and resumable catch-up. Return an explicit not-yet-reached/lagged outcome with the observed vector. Separate request-time bounded work from background maintenance. Rate-limit session-token waits, validate token provenance/size, and budget retries by operation type. Preserve ambiguous-write semantics when cancellation occurs; timeout does not mean rollback.

Tests: abort while queued; abort while GET is in flight; future-but-valid target; continuously hot braid; repeated deterministic replay rejection; S3 response body stalled after headers; no further network calls after a cancelled wait.

## SDK-011 — Budget options do not protect a serverless host from overload

Locations: `ts-log/src/tenants.ts:38-46`, `:128-140`, `:161-168`, `:243-265`, `:326-336`; `ts-log/src/replica.ts:590-604`; `ts-log/src/store-s3.ts:132-152`.

The code honestly calls `budgetBytes` advisory, measured once at open. The implementation also admits an oversized kept tenant, admits additional tenants when all existing ones are pinned, does not reserve capacity for concurrent in-flight opens, and measures only local file lengths. Failed directory/stat reads are treated as zero contribution. Checkpoint download first materializes the full response in a `Uint8Array`, then writes it to disk. Native memory, answer arrays, duplicate buffers, mmap retention, and active working-set growth are outside the budget.

This is an architecture limitation, not an accusation that the documented advisory contract was violated. It does mean the README's `/tmp budget gate` is not a safety gate for a 512 MB execution environment. `maxOpen` likewise is an eviction target, not a hard maximum.

Recommendation: define hard admission limits separately from soft eviction targets; reserve estimated capacity *before* cold opens; cap concurrent hydration; account for transient peak copies and native memory; periodically reweigh growing tenants; fail explicitly when capacity cannot be reclaimed. Stream/check size limits for checkpoints before buffering. Leave headroom for duty/checkpoint work that may coexist with application requests.

Tests: single tenant larger than budget; all tenants pinned; 100 simultaneous cold opens; post-admission growth; directory measurement failure; checkpoint plus working-copy plus query peak; slow eviction with a retained native handle.

## SDK-012 — A mature filesystem lease can take linear work per operation forever

Locations: `ts-log/src/store.ts:294-298`, `:373-376`; use sites `:505-514`, `:595-620`.

Every successful lease mint calls `forgetPredecessors(current)`, which sequentially tries to delete **all** token paths `current-1` down to `1`, even though previous acquisitions already deleted almost all of them. Acquiring one hot key N times therefore performs O(N²) deletion attempts over its lifetime. The cost depends on history, not live state.

This is particularly relevant to the ID counter and checkpoint manifest keys; log-slot keys themselves normally acquire only their first lease. It also means a high token restored in a legitimate lease can turn a single operation into a very long loop, potentially exceeding the lease TTL before its actual mutation begins.

Recommendation: reclaim only a bounded number of known predecessors, persist a reclamation watermark, or enumerate actual old token files in a maintenance path if listing is allowed for the local lock implementation. Keep lease acquisition cost independent of total historical traffic. The append-token protocol still needs a separate correctness/fencing review; making this loop faster does not fix REP-005.

Tests: 1, 1,000, and 1,000,000 prior acquisitions with only one current token physically present; measure syscall counts and p99 latency, not only time for fresh directories.

## SDK-014 — A read can observe a transaction that ultimately does not exist

Locations: `ts-log/src/writer.ts:678-690`, `:602-647`; `ts-log/src/replica.ts:143-155`, `:1010-1012`.

The writer installs its candidate in `core.db` before awaiting object-store publication. The promise gate serializes writes and refreshes, but ordinary synchronous `replica.db.read` bypasses it. Other requests sharing this replica can read speculative facts. A lost slot can subsequently discard that candidate and return a domain rejection.

Reproduced with two writer replicas initially at generation zero:

1. A publishes `Holder(id=1, name="winner")`.
2. B, still stale, begins `Holder(id=1, name="loser")`.
3. Pause B's log PUT after B's local application.
4. B's ordinary `db.read` returns `name="loser"`.
5. Resume the PUT; it finds A's slot, replays A, and re-judges B.

Observed:

```text
visible before publication: [{id:1,name:"loser"}]
B commit eventually: rejected
visible afterward: [{id:1,name:"winner"}]
```

The dirty read can escape to an HTTP response, an authorization decision, a message, or a dependent action in another tenant. This is not merely eventual replica staleness: it is visibility of a change absent from the accepted history. The module-scope shared-host example makes this interleaving relevant, even though a strictly sequential single-invocation host may not expose it.

Recommendation: define one published read frontier. Keep the accepted/published reader snapshot separate from a speculative writer candidate, or provide a driver-owned read operation that waits for the gate and expose no bypass. If speculative reads are intentionally useful, expose them as a distinct consistency mode that cannot be confused with committed reads. The synchronous embedded engine API can remain intact internally; the replicated host needs a different ownership/visibility boundary.

Tests: concurrent HTTP-style read during a delayed publish that succeeds, loses-and-rejects, loses-and-rebases, times out, and is cancelled; assertions must compare observed reads against the accepted-history model, not just compare final stores.

## SDK-015 — The callback may run repeatedly even without a slot conflict

Locations: `ts-log/src/writer.ts:429-475`, `:278-312`, `:169-173`.

If `reserve` cannot serve a draw from cached ranges, `recordWithLeases` discards that attempt's operations, acquires more blocks, and calls the entire body again. IDs already drawn are intentionally abandoned. The internal comment describes this, but the public callback type permits arbitrary asynchronous application code and the README examples do not make at-least-once callback execution a prominent contract.

An application that sends a message, charges a customer, increments an in-memory counter, or consumes a one-shot iterator in that callback can execute the action more than once or record different facts on replay. Re-judgment after a slot conflict correctly reuses recorded ops rather than rerunning the body, so application developers are especially likely to assume that property also applies during ID allocation.

Recommendation: decide whether the public body is an exactly-once command builder or an explicitly replayable pure function. If replayable, make the restriction visible in naming/docs and exclude external side effects from examples. Prefer a reservation-planning/ID-allocation API that lets the host obtain stable inputs before recording, or a recorder that resolves placeholders without invoking user code again. Keep application effects behind an admitted outbox relation consumed after publication.

Tests: multiple fresh fields, multi-block draws, side-effect counter, non-repeatable iterable, callback exception after a draw, and deterministic recorded-command equality across allocation retries.

## SDK-016 — A local materialization is not bound to the database it materializes

Locations: `ts-log/src/replica.ts:822-866`, `:880-890`, `:456-466`, `:537-541`; `ts-log/src/manifest.ts:28-31`; `ts-log/src/chain.ts:37-44`.

On open, the driver reads the configured prefix's manifest and checks its theory fingerprint. It then independently opens an existing local engine/sidecar and adopts the sidecar's vector. Neither the manifest nor the local sidecar names a database incarnation or binds the cache to a tenant/store namespace. Matching schema and generation arithmetic do not establish that the local facts came from this log.

Reproduction used the existing Ledger/Holder fixtures, a real `fsStore` rooted in a new temporary directory, and **two separate Node processes** so native handles were genuinely released between phases:

1. Process 1 created prefixes `tenant-a` and `tenant-b` with the same schema.
2. It published `Holder(id=1,name="tenant-a-secret")` to A and `Holder(id=1,name="tenant-b-data")` to B. Both reached vector `{c00000000:1,c00000002:0}`. A used local directory `cache`; B used `other-cache`. The process disposed the replicas and exited successfully.
3. Process 2 opened prefix B using A's existing `cache` directory, then opened B again using a fresh `fresh-b` directory.

Observed:

```text
reused cache opened for tenant-b: [{id:1,name:"tenant-a-secret"}]
fresh cache opened for tenant-b:  [{id:1,name:"tenant-b-data"}]
both vectors: [[c00000000,1],[c00000002,0]]
```

The reused cache was accepted as whole. Its next probe asked for B's slot 2, which did not exist, so no hash-chain comparison exposed the wrong predecessor. No files were forged, no byte corruption was injected, and no case-insensitive alias was needed.

The initiating action is a **configuration/ownership error**: reusing a local directory for a different logical database. The SDK must either refuse or reseed that configuration; silently serving the prior tenant's facts is an unacceptable failure mode for a disposable cache. Practical triggers include a changed tenant-directory mapping, a changed bucket/store prefix, deployment configuration drift, restoration into an old directory, and the filesystem naming aliases in REP-011. This is separate from authenticating a client's session vector, although both need the namespace/incarnation identity described in ARCH-004.

Recommendation: give every logical database an immutable incarnation identity in authoritative metadata and bind every sidecar/local store to it. Validate that binding before adopting local facts, pending commands, or scratch-recovery actions. A schema fingerprint is not a database identifier; a local path or prefix alone is insufficient if the configured object-store backend changes. On mismatch, explicitly refuse or discard/reseed according to a documented policy. If retaining an old incarnation for migration/rollback, require an explicit mapping rather than silently reassigning the cache.

Tests: same-schema/different-prefix; same-prefix/different bucket or backend; restoration with an old cache; deleted-and-reborn namespace; equal generation/vector but different rows; tenant directory alias. Assert that no response or pending publication from the old namespace can cross into the new one.

Rust has the analogous static path: `crates/bumbledb-log/src/replica.rs:519-528` validates only the schema fingerprint, while `:697-706` mounts the local engine and adopts `Chain::read`; `sidecar.rs:11-17` describes a format without namespace/incarnation. The Rust variant was **not** executed by this probe.

## Cross-cutting replication issues inherited by TypeScript

The replication report owns the detailed protocol findings. These TypeScript locations should be part of the same repairs and conformance tests, not left as a second implementation:

- Filesystem mutation fencing: `store.ts` checks ownership before awaited work, then performs rename/link later. A preceding check is not atomic fencing of a filesystem replacement (REP-005).
- Cached GC floor (REP-001): `writer.ts:599-602` reads `core.checkpoint`; `commit`/`commitSplit` at `:913-947` do not first refresh the manifest. Even a caller's explicit `replica.refresh()` only polls the manifest every 16th pass (`replica.ts:498-501`), so the Lambda example's per-request refresh does not remove this exposure. A fresh poll would still not atomically fence a concurrent GC deletion.
- Unsafe checkpoint order (REP-002): `writer.ts:825` uses `checkpointVector(candidate).order(...)`. Despite having a separate correct `dominates` method, `vector.ts:64-86` implements `order` by scalar sums. The TypeScript publisher therefore also permits `[A=3,B=0]` to replace `[A=0,B=2]`, with the same component-retirement problem. It is not protected merely because it has a method named `Vector` or checks for `"after"`.
- Counter acknowledgment identity (REP-004): `store-s3.ts:260-268` promotes byte-equal ambiguous `putSwap` verification to `swapped`; `writer.ts:364-373` interprets `swapped` as ownership of the ID range. Equal counter values do not identify which contender advanced it.
- Checkpoint scratch cleanup (REP-007): `replica.ts:711-718` distinguishes only current head versus not-current-head. A prior checkpoint may remain reachable and needed even after a newer head is published.

The analogous Rust findings for unbounded lease cleanup (REP-012), missing work budgets (REP-015), and mutation authority escaping a reader (REP-016) overlap SDK-012, SDK-010, and SDK-008 respectively. Treat these as cross-language instances of shared obligations, not independent product problems to count twice. TypeScript pool shutdown releases a directory lease before replica disposal (`tenants.ts:403-405`), while explicit TypeScript eviction uses a different order; the Rust eviction schedule is detailed separately in REP-017.

## What the architecture should mean for applications

The desirable product contract is not just “the files eventually agree.” It is:

1. A tenant is a security, lifecycle, resource-accounting, recovery, and scheduling unit.
2. A theory names immutable relation/value vocabulary plus admitted integrity constraints.
3. A command is immutable and has one stable operation identity and one outcome that survives uncertain network acknowledgment.
4. An accepted receipt names a durable position in the tenant's authoritative history.
5. A normal read sees a published frontier, optionally after meeting a caller's validated session vector.
6. A rejection never becomes visible through the committed-read surface.
7. Host cancellation and eviction stop work/release resources without redefining whether a commit happened.

This requires separating concepts currently merged in one convenient handle:

| Concern | Recommended authority |
| --- | --- |
| Schema and wire grammar | One Rust core, versioned and cross-language tested |
| Local mutable engine | Private driver-owned capability |
| Published read frontier | Explicit snapshot/read capability |
| Candidate command and pending outcome | Writer state machine with immutable bytes and stable identity |
| Tenant lifetime | Pool-owned slot plus per-request borrow tokens |
| Directory ownership | Actual enforceable lock/fence, separate from an expiring timer |
| Resource accounting | Host-level admission controller with native/disk/transient reservations |
| Request budgets | Deadline/signal propagated through queue, network, catch-up, and query work |

Do not force every deployment into distributed transactions across all tenants. Per-tenant isolation is a strength. Cross-tenant workflows and `_shared` data need an explicit outbox/saga or versioned-reference model. A cache prefix called `_shared` is not by itself a transactional join or a permission boundary.

### The missing application transaction question

The embedded SDK offers `writeFrom(witness, ...)`, so an application can reject a stale read/modify/write decision. The log recorder intentionally has no reads and reruns *integrity judgment*, not the application's read-dependent logic. Those are different guarantees. A command that was valid under the schema may still be wrong for a business decision made from a stale snapshot if the corresponding precondition was never encoded as data/constraint.

The product should choose and demonstrate a first-class conditional-command pattern: expected facts/version as a precondition, a reservation relation, or a compare-and-change command that the authoritative execution validates. Integrity constraints alone do not make every arbitrary application read/modify/write serializable. This is an architectural question, not a demonstrated engine bug.

### Performance goals worth measuring before claiming a hosting envelope

- Full published commit latency: recording/marshaling, local pending fsync, local engine judgment/apply, log arbitration, settled-sidecar fsync, contention replay. “One conditional PUT” is only the network component; fresh-field lease allocation may add multiple sequential object-store operations.
- Cold tenant hydration: manifest + checkpoint metadata + full database bytes + open verification + tail decode/apply + per-slot sidecar persistence. Report peak memory and disk, not just elapsed time.
- Warm tenant request overhead: `get` performs directory lease renewal on every pin, with temp write, sync, rename, and directory sync. Measure that fixed cost against point reads.
- Contention: each lost candidate can discard and rehydrate the whole local working database. Report replay bytes and directory churn per accepted command, not only number of retries.
- Fairness: one gate covers all braids in a replica. Theory-derived braid independence does not currently imply parallel local commits. Benchmark hot-braid interference with cold-braid requests in the same tenant.
- Fleet churn: steady state after hours of tenant opens/evictions, retained promises, GC pauses, lease token aging, and peak concurrent hydration.
- Query boundary cost: synchronous native execution and complete JS result arrays can block the event loop and inflate peak memory. Fast kernels do not automatically imply good request p99; use worker isolation or bounded query APIs where required.

### Observability that should be part of the contract

Expose structured tenant-scoped metrics for published/applied/pending vectors, candidate age, unresolved outcomes, local-native generation, replay lag, slot losses, discarded/reseeded bytes, native handle count, checkpoint age, pool admission pressure, lease identity/loss, and cancellation. Repeated `console.error` signatures are useful during development but are not a recovery policy. The host must be able to distinguish expected contention, a slow catch-up, an unreplayable authoritative slot, and resource exhaustion.

## Lambda example and AWS readiness

The example is explicitly non-normative and should remain labeled as such until its deployment gaps are closed.

- `examples/lambda/alchemy.run.ts:15-40` creates an intended least-privilege role, but `:47-66` does not attach it to the function. The README correctly says the deployed function will get `AccessDenied` until the owner resolves IAM. This is an acknowledged release-readiness blocker, not a hidden permissions bug discovered here.
- `handler.ts:17-29` captures static environment credentials. The store itself supports a per-request credentials callback, but the example does not demonstrate rotating credentials for a long-lived host. Do not generalize the Lambda example to ECS/EC2/Fluid workers without revisiting credential refresh and scoped IAM.
- There is no application authentication/tenant authorization in `handler.ts`; the example uses one fixed prefix and one fixed local directory. Its deployment must add a trusted identity-to-tenant mapping and protect the function URL. The audit did not deploy or inspect the provider's default function-URL authentication settings, so it does not claim an externally reachable vulnerability already exists.
- `request.ts:48-80` interprets all non-POST methods as reads and ignores `isBase64Encoded`; valid base64 Function URL POST events are therefore not decoded before JSON parsing. Add explicit method/path/event validation, input-size limits, and correct base64 handling before treating the example as production request grammar.
- `handler.ts:73-90` refreshes without a request deadline and scans every note into one response. It is a demonstration, not a bounded query endpoint.
- The handler retains a successful writer forever; failures after successful acquisition have no eviction/reopen policy in `holdReplica`. The first failed open retries correctly; later operational failures need their own lifecycle decision.
- Duty subprocess execution has no deadline tied to remaining invocation time; checkpoint, replay, and application working copies may compete for the same ephemeral disk. Model them in one budget, and ensure interrupted duty is safe and observable.
- Current package manifests are 0.20.3 while the example pins and prose describe 0.19.0. Do not use successful example tests as evidence for the current release's native/driver interoperability.

No AWS requests, deployments, credential reads beyond checked-in source text, or remote mutations were performed by this audit.

## Validation performed

The exact executed test commands, inline source for all nine probes, actual outputs, and limitations are preserved in [32-sdk-test-evidence.md](32-sdk-test-evidence.md).

Runtime: Node v26.4.0 on darwin-arm64. Existing native artifact reports `bumbledb-node 0.20.3 (bumbledb storage format v8)`. Source imports used `--conditions=bumbledb-src`; no package build, generated-source update, or lockfile update was run.

Executed 132 existing TypeScript-log tests across writer, replica, tenant, recovery, store, codec, chain, keys, fingerprint, identity, temporal-gate, and checkpoint-orphan files: **132 passed**. This is useful positive evidence, but these suites do not cover the reproduced request/lifecycle interleavings above. The newly discovered probes were run as isolated one-off programs over temporary local directories and in-memory object stores, not added as product tests.

Also executed 77 existing core SDK tests across `read-scope-leak`, `owned-read`, `marshal-bijection`, `native-loader`, `db`, `ffi`, `type-kernel`, and `keyed-get`: **77 passed**. Total for this agent's selected existing suites: **209 passed, zero failures**. This was not the entire repository battery.

Executed probes reproduced SDK-001, SDK-002, SDK-003, SDK-004, SDK-005, SDK-006 (controlled clock injection), SDK-008, SDK-014, and SDK-016 (two ordinary processes and temporary filesystem-backed storage). Shared protocol defects and AWS behavior beyond local construction remain static review unless the companion replication report states otherwise. Total distinct SDK findings with an executed reproduction: **nine**.

## Suggested repair sequence

First define and test the published-read/candidate-write boundary and immutable command ownership (SDK-001/003/008/014). Then make closure/borrowing/native ownership coherent (SDK-002/004/005/006/007 and SDK-013). Add request/work budgets and hard tenant admission next. Only after those contracts survive fault injection should throughput tuning remove allocations, increase batching, or introduce more parallelism.

The philosophical direction is soundest when “one semantic core” means one observable application contract, not only one parser. The thin TypeScript layer should be thin in *proof obligations* too.
