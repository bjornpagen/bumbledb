# Rust log, storage, recovery, and tenant audit

Date: 2026-09-04. Scope: the current, dirty working tree, especially `crates/bumbledb-log/src`; findings are against that tree, not an inferred historical release. No production code was changed. This is an adversarial review, not a proof that the remaining implementation is correct.

## Executive assessment

The most valuable idea is real: a schema-derived arbitration domain, immutable concrete commands, deterministic invariant admission, and materialized local state can give application developers much stronger semantics than an unstructured replication log. The code contains substantial work on canonical parsing, error identities, partial service, pending recovery, and cross-language fixtures.

The weakest part is the boundary between that algebra and physical history. The implementation frequently treats a locally observed condition as a durable global fact: a cached floor as retirement authority, a vector sum as a safe checkpoint order, byte equality as proof of counter ownership, and a lease check as a fenced filesystem operation. Those substitutions are not valid. They can produce a `Published` acknowledgement whose effect a fresh replica never sees, duplicate supposedly unique identifiers, and premature loss of restore history.

Do not regard `Published` as a production RPO-zero contract until REP-001 through REP-010 have been resolved and tested together. No P0 is assigned here: these are severe but depend on deployment mode, retention, concurrency, failure, or particular APIs. Multiple P1 release blockers are justified.

### Finding index

| ID | Priority | Finding | Evidence status |
|---|---|---|---|
| REP-001 | P1 | A stale writer recreates a collected slot and acknowledges an invisible commit | Reproduced with actual Writer/Replica |
| REP-002 | P1 | Checkpoint sum order can move one braid backwards behind its deleted history | Reproduced with actual compacted snapshots/Replica |
| REP-003 | P1 | Equal clock fields switch GC into immediate checkpoint deletion | Reproduced, including normal default Checkpointer::run |
| REP-004 | P1 | S3 ambiguity resolution can grant the same ID range twice | Reproduced composition; transport outcome injected |
| REP-005 | P1 | Filesystem lease checks do not make rename a fenced CAS | Confirmed static adversarial schedule |
| REP-006 | P1 | A higher writer ID permanently fences healthy lower-ID allocators | Reproduced unchanged retries with bounded harness stop |
| REP-007 | P1 | Scratch recovery deletes a checkpoint still reachable through `prev` | Reproduced |
| REP-008 | P1 | A delayed checkpoint drops intervening published checkpoints from history | Reproduced |
| REP-009 | P1 | Opening an already-active replica performs destructive cleanup before exclusivity | Confirmed static adversarial schedule |
| REP-010 | P1 | Filesystem object bytes and their fencing generation are not one durable state | Confirmed static crash schedule |
| REP-011 | P1 | Valid case-distinct tenant/store keys alias on case-insensitive filesystems | Reproduced, including cross-tenant cached data exposure |
| REP-012 | P2 | Local lock acquisition does work proportional to every historical acquisition | Confirmed static complexity |
| REP-013 | P2 | Every GC pass starts again at slot 1; checkpoint age is not durably recorded | Confirmed static complexity/design gap |
| REP-014 | P2 | Resident checkpointing can require a whole-database quiet window | Confirmed static progress condition |
| REP-015 | P2 | Catch-up and wait APIs lack a work budget, deadline, or cancellation | Confirmed static API/liveness limitation |
| REP-016 | P1 | "Read access" exposes write-capable engine handles outside the log | Confirmed static public API |
| REP-017 | P1 | Tenant eviction releases its lease before deleting the protected directory | Confirmed static adversarial schedule |
| REP-018 | P2 | Checkpoint Merkle identity is not verified on read; replay audit can be skipped | Confirmed static verification gaps |
| REP-019 | P2 | Failed orphan deletion is forgotten by clearing its only durable discovery record | Confirmed static failure schedule |
| REP-020 | P2 | `commit_split` drops earlier successful outcomes on a later infrastructure error | Confirmed static public API |

The companion [reproduction evidence](11-replication-test-evidence.md) preserves the full external harness, ten executed scenarios, captured output, and limitations.

## Findings

### REP-001 — A stale writer can acknowledge a new value in an already-retired slot

**P1; durability and convergence.** Sources: `writer/discipline.rs:195-239,338-343`, `writer/open.rs:61-65,138-145`, `writer/mod.rs:539-557`, `gc.rs:199-218`, `replica.rs:877-887` (all under `crates/bumbledb-log/src`).

The only pre-create retirement check consults `core.floor`. That field is updated at open or full re-establishment, not before a normal commit. The create writes an unfenced log key and the object store has no retained-slot tombstone.

Failure schedule:

1. Writer A opens at braid generation 0 and remains open but inactive.
2. Writer B publishes slots 1 and 2. A checkpoint contains both. After the configured retention window, GC deletes slot 1; slot 2 is exempt as the checkpoint boundary.
3. A records a different change and tries slot 1. Its cached floor remains absent/old, so the retirement check passes and create-only PUT succeeds on the deleted key.
4. A returns `Accepted(... durability: Published, slot: 1)`.
5. A new replica seeds checkpoint generation 2 and never reads A's new slot 1. The new acknowledgement is absent from the authoritative state.

This is not merely stale reading or local-ack loss: the returned durability is `Published`. A short retention window makes the schedule easy to reproduce; at the default it needs a sufficiently long-lived stale writer, paused request, or equivalent later schedule. The clock problem in REP-003 can shorten other history-retention guarantees, but is not required for this bug.

**Direction:** retirement has to be monotonic and enforced at publication, not a cached precondition. A fresh manifest GET alone is insufficient: a writer can pause after that GET while GC removes the key. Consider epoch-qualified log namespaces, a durable per-braid floor/fence checked atomically with publication, retained tombstones, or an explicit bounded writer-lifetime protocol with server-enforced fencing. The five current verbs do not by themselves express an atomic “create only above this other object's current floor.”

**Required tests:** an actual open writer, another writer/checkpointer, GC, then the old writer's commit; also pause immediately after any future floor check and collect before PUT. Assert every `Published` effect is in a fresh replica or receives an explicit non-success outcome.

### REP-002 — Checkpoint ordering by vector sum is unsafe once GC exists

**P1; acknowledged-data loss.** Sources: `vector.rs:13-20,51-60`; `manifest.rs:459-489`; `gc.rs:170-181,199-218`; `replica.rs:738-792,877-887`.

The checkpoint order compares only sums. Therefore `[A=0,B=2]` can be replaced by `[A=3,B=0]`. Each vector is a valid serial prefix, but validity of a state does not mean it is safe to make it the sole recovery floor after another prefix has been collected.

Failure schedule: two checkpointers have those valid snapshots; `[0,2]` publishes, B's slot 1 is collected after retention, then a delayed `[3,0]` candidate publishes. A fresh replica seeds the latter; its first B probe is missing slot 1, and because its adopted B floor is zero that absence is interpreted as the tip. It returns `[3,0]` without B's previously acknowledged changes. The other checkpoint may still exist temporarily, but the ordinary current-head recovery path silently omits the data. Later retention/cleanup can remove the alternative recovery source.

**Direction:** a safe recovery floor must dominate the previous floor componentwise. The scalar sum can be a cadence metric or tie-breaker after dominance, not the safety order. Incomparable candidates need catch-up/recompaction, a componentwise checkpoint representation, or a deliberately retained recovery cover. GC must use the same partial-order invariant.

**Required tests:** publish incomparable vectors with unequal sums and collect one component before the delayed publish; repeat with every braid permutation. Include a model assertion that each published floor dominates all previously published/collected floors.

### REP-003 — GC conflates clock equality with a different deletion policy

**P1; retention/PITR contract violation.** Sources: `gc.rs:48-81,113-129,145-182,228-255`; `checkpointer.rs:98-105,164-188`.

`Age` treats `publish_ms == now_ms` as a sentinel for timestamp-based GC. In that branch, `checkpoints_old()` is `now_ms > window_ms`, i.e. “has more than 90 days passed since the Unix epoch?”, not “is this checkpoint 90 days old?” Consequently all prior checkpoint documents and snapshots are deleted, including ones just published.

The branch is reached by the public `gc()` API, by a newly opened `Checkpointer` because `publish_ms` is `None` and is replaced with `now`, and potentially by a publish/run taking place within the same millisecond. This is a normal execution path, not a malicious clock.

There is also a dual failure: when the fields differ, all old checkpoints and below-floor log entries are aged relative to one in-memory current-head publish time. Frequent new checkpoints reset that age and can postpone reclamation indefinitely. Restart loses the timestamp and flips into the destructive branch again.

**Direction:** no equality sentinel. Represent the policy explicitly, persist an authoritative publication/retirement timestamp associated with each retained unit, and define retention relative to recoverable history rather than a process-local last-publish variable. A predecessor checkpoint often must survive beyond its own age to reconstruct targets at the window boundary.

**Required tests:** new checkpointer against existing recent history; two publications in one millisecond; restart; frequent publications for longer than retention; clock rollback/forward; PITR at both edges of the promised window.

### REP-004 — Byte equality proves content, not ownership of an ID allocation

**P1; duplicate generated identifiers on the S3 path.** Sources: `lease.rs:121-175`; `store/s3.rs:466-477,480-518`; `store.rs:405-425,445-477` (the helper locations should be read with their enclosing `prove_*`/`resolve_ambiguous_*` functions).

`lease_block` correctly explains that ambiguous counter updates must not be claimed: every contender writes the same next number, so equal bytes do not identify the winner. However `S3Store` calls `prove_create` and `prove_swap` *inside the backend*. Those helpers turn an ambiguous PUT into `Created`/`Swapped` when the GET matches. The allocation layer never sees the ambiguity it was designed to handle.

Example: two callers see an absent counter and try body `4096`. One definitively creates it and owns `[0,4096)`. The other's HTTP result is a 409 or timeout, and its follow-up GET sees `4096`. The store reports `Created` to that second caller, which also claims `[0,4096)`. The same issue exists for two CAS contenders writing an identical increment body.

The external harness simulates the unproved transport outcome and executes the actual proof/allocation helpers. It does not claim to have reproduced a real AWS timeout. The production trigger is established by the S3 call graph and the documented ambiguous outcomes.

**Direction:** preserve raw ambiguity through generic backends, and resolve it at the operation layer. If retryable allocation ownership is needed, store a unique allocation request identity/nonce alongside the counter and prove that identity, not the resulting number. Immutable content-addressed writes and exclusive allocation cannot share one “equal body means my write” law.

**Required tests:** contender wins + local 409; contender wins + local timeout; local PUT lands but response is lost; create and CAS variants. Assert non-overlap of the caller-visible ranges, not just monotonic final counter bytes.

### REP-005 — Expiring filesystem leases are not atomic fencing for file mutation

**P1; CAS safety.** Sources: `store/fs.rs:299-335`; `store/fence.rs:13-14,141-149,160-179,216-239,252-285,328-337`.

`FsStore::put_swap` acquires an expiring local lease, checks the ETag/generation, writes a temp, checks `still_current`, and then performs an unconditional rename. A process can pause after the last check. Five seconds later another process can take a successor lease and successfully publish newer data; the old process can then resume and overwrite it. Both callers can return success. Checking one instruction earlier is not a fence enforced by the write target.

There is a second lease-metadata race. `try_mint` publishes the token file, then unconditionally rewrites `~head`, then deletes predecessors. If the old minter pauses before updating `~head`, a successor can expire/take over/update the head/delete old tokens. The old minter can subsequently regress the head. Because discovery follows contiguous tokens from the hint (falling back to 1), deleted gaps can make a later live token undiscoverable and allow old token numbers to be minted again. `still_current` compares only that discovered number.

**Direction:** use a local OS ownership primitive with correct process-death semantics for the whole mutation, or a single durable transactional store that atomically couples expected version/fence and mutation. Do not claim a distributed expiring-token algorithm makes ordinary POSIX rename conditional. If an expiring lease remains, its protected operation must be fenced by a resource that atomically rejects stale holders, and head advancement/compaction must itself be monotonic.

**Required tests:** process suspension at every filesystem step, especially immediately before rename and between token publication/head rewrite; wake it only after a successor has completed. Ordinary thread races below the TTL do not exercise this failure class.

### REP-006 — Writer identity is being used as a fencing epoch for a shared counter

**P1; multiwriter liveness.** Sources: `writer/mod.rs:125-138,481-486`; `lease.rs:143-177`; `store/mem.rs:111-129`; corresponding filesystem/S3 generation checks.

Writer construction passes `options.writer_id` to `Leases::new`. These IDs are arbitrary identities, not acquired epochs. Once writer 20 stores generation 20 on a shared ID counter, writer 10 cannot CAS it ever again: its requests fail even with the freshly read correct ETag. `lease_block` retries `Moved` forever without changing the token, delay, cancellation, or a deposed result. Its comment that every retry implies somebody else's successful lease is false in this case; nothing need be progressing anywhere.

This can strand a healthy concurrent writer as soon as its cached block empties. It also undermines ordinary failover if the replacement's identity happens to be smaller.

**Direction:** distinguish writer identity, per-resource ownership epoch, and independent counter allocation. A shared multiwriter allocator does not require fencing one healthy writer behind another merely because the numeric ID is lower. If ownership is intentional, acquire a real epoch and return a terminal `FencedOut` outcome instead of treating it as ordinary contention.

**Required tests:** writers with IDs 20 then 10 allocate multiple blocks alternately; random/reused IDs; failover with a smaller identity; count the PUTs and prove a bound or a meaningful terminal result.

### REP-007 — Checkpoint scratch recovery confuses “not head” with “orphan”

**P1; destructive restore-history loss.** Sources: `replica.rs:1038-1045,1058-1082`; `checkpointer.rs:295-304`.

If a process crashes after checkpoint C1 enters the manifest but before its local scratch record is cleared, C1 remains a valid published checkpoint. Another checkpointer can then publish C2 with `prev=C1`. Reopening the first directory compares its scratch digest only with the current head. Since C1 is not C2, it deletes both C1 objects even though C1 is reachable and within retention.

**Direction:** establish unreachability from the complete retained graph, with concurrency-safe publication/GC rules, before deletion. “Was my candidate ever published?” cannot be recovered from current-head inequality. Publication receipts/state transitions or a staged-object registry can avoid confusing historical success with an orphan.

**Required tests:** crash-after-CAS, another publication, reopen; assert both objects for the predecessor survive and restore succeeds. Also race a scratch sweep with an in-flight publisher referencing the same candidate.

### REP-008 — Checkpoint candidates retain stale backlinks across publication races

**P1 for promised history retention; also unbounded orphan storage.** Sources: `checkpointer.rs:269-295`; `manifest.rs:440-489`.

The candidate's `prev` is captured before publication. The candidate digest is fixed, and the CAS loop can reread a newer manifest and still publish the same candidate without changing `prev`. For example, candidate C3 is prepared with `prev=C1`; C2 wins first with `prev=C1`; C3's larger vector wins next but still links directly to C1. C2 was successfully published yet is absent from the only GET-discoverable history spine.

This also happens without a failed CAS: a delayed candidate may first attempt publication after C2 wins. Its stale backlink is still accepted. GET-only GC can no longer discover C2. Restore may lose its only qualifying recent base after older checkpoints/logs are collected, and a supposedly published snapshot becomes permanent unindexed storage.

**Direction:** linearize backlink construction with the pointer CAS. On a changed incumbent, rebuild/re-address the document with the actual predecessor and re-evaluate componentwise safety; handle ownership and cleanup of the old candidate explicitly. Alternatively make history a durably indexed DAG instead of pretending every successful publication is on one immutable list.

**Required tests:** independently prepared candidates with a shared old predecessor, publish them in reversed order, and traverse history to verify every retained successful publication is discoverable.

### REP-009 — Replica/writer open can delete an active owner's scratch before checking the engine lock

**P1; races against live compaction/publication.** Sources: `replica.rs:323-351,695-715,1052-1055,1085-1126`; `writer/open.rs:60-66,232-255`; `checkpointer.rs:285-295`.

Both entry points run `sweep_at_open` before opening the engine and encountering `EnvironmentLocked`. The sweep unconditionally removes sidecar temps and sibling compaction directories, and may delete remote checkpoint objects named by scratch. A second opener that ultimately refuses can already have damaged the first active owner's work.

A concrete dangerous window is after the active checkpointer uploads its snapshot but before it publishes its document/manifest: a competing open finds scratch, sees that digest is not yet head, removes the uploaded object, then fails the engine lock. The active publisher can continue and install a manifest whose snapshot is now absent. Direct `Replica::open`/`Writer::open` do not require the tenant layer's outer lease.

**Direction:** acquire stable exclusivity over the complete local lifecycle before *any* cleanup, and retain it through dispose. Make local scratch ownership explicit; remote orphan deletion must also obey the publication/reachability protocol, not rely on an attempted local open.

**Required tests:** keep the first handle alive; pause at scratch creation, after upload, and before CAS; call the second public open; verify it neither alters local files nor removes remote objects.

### REP-010 — A filesystem CAS commits body and fence generation in two separate transactions

**P1; fencing/durability gap.** Sources: `store/fs.rs:63-89,252-258,313-333`.

Creation links and fsyncs the object before writing its generation sidecar. Swap renames and fsyncs the new body before replacing the generation. A crash or sidecar I/O error leaves new durable bytes with the old/missing generation. `read_generation` silently treats unreadable, corrupt, or missing generation data as zero.

A higher-fenced body may therefore be present and accepted by byte-equality recovery while a later lower-fenced caller can overwrite it. This violates the store contract even apart from the expiring mutation lease's TOCTOU. Returning a generic I/O failure does not undo already durable data, and equal-body verification does not repair the missing fence.

**Direction:** one atomic, durable envelope or transactional record must contain both value and generation; derive the public bytes/ETag from that record. A corrupt fence should fail closed, not reset authority to zero. Consider a genuine local KV adapter instead of maintaining a second storage protocol beside the database engine.

**Required tests:** kill after object parent fsync but before generation publication; fail generation temp creation/rename/fsync; reopen and try a lower token with a freshly read ETag.

### REP-011 — Filesystem naming can collapse distinct valid tenant identities

**P1 on case-insensitive hosts; tenant isolation and backend equivalence.** Sources: `store.rs:60-107`; `store/fs.rs:43-45`; `tenants.rs:160-165,211-247`.

The key grammar accepts `tenant-A` and `tenant-a` as distinct identities, and S3/MemStore treat them distinctly. `FsStore` directly maps them to paths. Default case-insensitive macOS filesystems can map both to the same object and local tenant directory. Unicode normalization equivalence can introduce further aliases depending on the filesystem. No mount capability check or injective escaping protects the mapping.

If used for separate tenants, this is not just a portability inconvenience: tenant B can open tenant A's local materialization or see A's manifest/log keys under a spelling that the logical API considers different.

**Executed evidence:** the harness created two isolated case-sensitive MemStore prefixes, `t/tenant-A` and `t/tenant-a`, with different facts, then opened the two tenant handles in successive `Tenants` lifetimes using the same local cache root. The second handle served the first tenant's catalog digest, not its own remote catalog. The persisted local directory is not bound to its remote prefix, and equal generation/sidecar counts pass the wholeness check. This reproduced a real data-isolation failure, not merely an `Exists` result for a similarly named file.

See also SDK-016 in [the SDK/hosting audit](30-sdk-hosting.md), which independently reproduces the more general wrong-namespace cache reuse without relying on case folding. The local materialization needs an origin identity binding as well as an injective directory name.

**Direction:** use an injective filesystem encoding over the complete key bytes (or hash plus a verified original-key record), or require and verify a case-sensitive volume. Do not silently lowercase tenant IDs unless changing the public identity model is an explicit product decision. Test full backend equivalence on the supported host filesystems.

### REP-012 — Lease cleanup makes mutation cost grow with lifetime acquisition count

**P2; asymptotic performance.** Sources: `store/fence.rs:160-164,233-238`.

Every acquired token calls `forget_predecessors`, which iterates from `current-1` all the way to 1 and calls remove even for predecessors deleted on earlier acquisitions. The Nth update to a hot key does O(N) filesystem operations; N updates do O(N²) cleanup work. Compacting old token files does not compact the amount of work.

Manifest and ID-counter keys are long-lived. Benchmarking only a newly created store misses this behavior.

**Direction/tests:** persist/derive a bounded cleanup frontier, remove only newly obsolete tokens, or use a constant-space lock protocol. Benchmark the 1st, 10,000th, and 1,000,000th acquisition on the same key; measure system calls, not merely files remaining.

### REP-013 — GC repeats the complete historical probe range and lacks durable age metadata

**P2; S3 request cost and scalability.** Sources: `gc.rs:38-45,167-173,187-222,228-248`; `checkpointer.rs:178-189,202-214`.

Although `Sweep.swept_below` is documented as a resume marker, no subsequent invocation accepts or reads it. Every braid scan begins at slot 1, including one GET for every already-deleted object. A tenant with ten million historical commits can require millions of 404 requests on each duty run. Checkpoint discovery also walks the entire still-linked history, and cadence's byte calculation rereads every log object since its floor on each run.

No durable per-unit publication clock exists in checkpoint documents, so retention cannot both be accurate across restarts and avoid repeatedly deriving age from unrelated fields. REP-003 is the destructive manifestation; the missing metadata also blocks a scalable implementation.

**Direction/tests:** store a monotonic collection frontier as part of the durable protocol and safely update it around interrupted deletes; define idempotent resume behavior. Test request counts after multiple GC passes over a large, mostly collected history and require the second pass to be proportional to new work.

### REP-014 — Resident checkpointing requires the whole compaction interval to see no commit

**P2; progress and shutdown availability.** Sources: `writer/duty.rs:92-142`; `checkpointer.rs:246-254`; `writer/mod.rs:654-664,741-756`.

The duty captures `Arc<Db>` and chain entries, compacts/copies the entire store, separately computes the current catalog digest, then checks that the live chain/handle/generation are still exactly the captured state. Any commit during that potentially long interval discards all that work and retries from scratch. On a continuously active tenant, the system can compact repeatedly without ever producing a checkpoint. `Writer::drop`/`quiesce` synchronously joins these threads, so a stalled/unbounded duty can also stall release.

This may be logically safe snapshot rejection, but it does not satisfy the “commits never wait” promise at a system level: CPU, I/O, space, recovery lag, and shutdown can all suffer. Errors are turned into `Deferred` and the spawned caller drops the result, without a first-class health signal.

**Direction:** pin one actual read snapshot whose generation, chain heads, compacted content, and catalog digest agree by construction. New commits should not invalidate an older safe snapshot. Expose duty status, failures, age, bytes copied, attempts, and cancellation. Measure checkpoint completion under sustained writes over databases much larger than cache.

### REP-015 — Catch-up/wait can consume unbounded work without returning control

**P2; serving and operational liveness.** Sources: `replica.rs:405-421,488-503,839-874`; `writer/open.rs:350-435`; `lease.rs:143-177`.

`refresh` performs catch-up until all braids have returned a missing next slot. Round-robin avoids braid starvation inside the loop, but a sufficiently hot braid can prevent the call itself returning. `wait_for` has no deadline/cancellation and does not reject a braid ID from a different decomposition if one is constructed through another codec. Repair loops can repeatedly discard/rebuild without a resource budget; warnings are not an API result. The ID loop has the separate hard failure in REP-006.

**Direction:** give each pass a byte/slot/time budget and return progress; offer cancel/deadline-aware wait and opening APIs with typed stalled/corrupt/fenced outcomes. Infinite retry can be a caller policy, not an inescapable synchronous library behavior.

**Tests:** a store wrapper that appends faster than replay, unreachable/future session vectors, schema-mismatched vectors, storage that repeatedly triggers the same repair signature, cancellation while replaying a large checkpoint tail.

### REP-016 — Read-only log roles hand out mutation capability

**P1; public API integrity.** Sources: `replica.rs:355-360`; `writer/mod.rs:592-599`; engine `Db::write` API.

`Replica::db()` and `Writer::with_db()` return/pass `&Db<T>` and describe it as read access. The engine's write operation takes a shared receiver, so these references permit direct unlogged mutation. The replica and checkpointer role restrictions are therefore conventions, not a property enforced by the handle types. A normal application can write through what looks like its read handle; the next wholeness repair can discard that change, or a resident snapshot attempt can be perturbed outside the chain lock.

This is not a defense against a malicious host with arbitrary filesystem access. It is about making the safe, advertised SDK path safe for ordinary application code.

**Direction/tests:** return a read-only instance/transaction/query capability, not the whole engine database. Add compile-fail coverage that a replica/read closure cannot call mutation. Keep any raw engine access explicitly unsafe/maintenance-only and outside the production role vocabulary.

### REP-017 — Tenant eviction unlocks before disposing the protected resource

**P1; local multi-process ownership race.** Sources: `tenants.rs:270-281,313-327`; `replica.rs:428-433`; `store/fence.rs:365-380`.

Explicit and budget eviction drop `entry.lease` before `entry.replica.dispose()`. Disposal closes the database and removes its directory. There is therefore a window after engine close but before removal in which a successor can acquire the released lease and open the same directory, then have that directory removed by the old owner. Eviction also does not verify that a long-idle entry still owns the current token before deleting.

Borrow pins prevent in-process eviction while a `Live` borrow exists; they do not fix cross-process ownership order. The tenant lease is refreshed only on `tenant()` calls, not throughout a long-lived `Live` handle, which makes expiry/ownership assumptions especially important.

**Direction/tests:** close/remove while still holding verified exclusivity, and release last; a stale holder must never delete the successor's directory. Test a coordinated successor open exactly between engine close and directory deletion, and expiry during a long-running tenant operation.

### REP-018 — Several claimed checkpoint integrity checks are not actually on the normal path

**P2; corruption detection and evidence quality.** Sources: `manifest.rs:361-367`; `replica.rs:535-542,658-690,839-874,936-949`; `gc.rs:238-247,275-280,356-375`; `writer/open.rs:219-229`.

Checkpoint documents are addressed by their digest, but readers parse them without verifying the digest of the bytes against the requested key. A corrupt/wrong document can therefore supply a false vector/backlink/catalog claim. Since the backlink walkers do not maintain a visited set, a syntactically valid cyclic or absurdly long graph can produce unbounded walking; digest verification would rule out practical forged cycles in content-addressed honest data.

Separately, replay's independent catalog comparison runs only after catch-up and only when the final vector equals the checkpoint vector exactly. If newer log slots already exist, catch-up can pass the checkpoint and never compare at that boundary. This is a weaker audit than the comments' “audited from independent directions” suggest. Snapshot seeding does compare catalog content and generation, which is good, but that checks consistency of the pair, not independent replay agreement with the published claim.

**Direction/tests:** verify document-key hashes before interpretation; add bounded/visited traversal as defense in depth; perform boundary audits on the actual pinned generation or explicitly classify them as sampled/best-effort. Plant a wrong-key but well-formed checkpoint and a bad catalog claim followed by newer valid slots.

### REP-019 — Best-effort cleanup can permanently lose the only orphan locator

**P2; remote storage leak and operability.** Sources: `checkpointer.rs:300-304`; `replica.rs:999-1019,1026-1045`.

On `Kept` or refused publication, the result of deleting the checkpoint pair is ignored, and the scratch record is cleared regardless. If deletion fails on a transient S3/IO error, the candidate is no longer reachable from the manifest and no longer named by scratch. GET-only GC cannot discover it. A subsequent candidate also overwrites the single scratch slot rather than maintaining all unresolved cleanup work.

The locator also lives only in the local replica directory. Losing ephemeral host storage after uploading a candidate loses this discovery record even if the local write was fsynced. A design that treats local replicas as disposable cannot rely exclusively on their survival to make remote orphan collection complete.

**Direction/tests:** only acknowledge/clear durable cleanup work after both deletes succeed, retain a retryable staged-object ledger, and expose unresolved orphan count. Inject failure separately at snapshot delete and document delete, then reopen and verify cleanup resumes.

### REP-020 — A split commit loses its successful-prefix receipt on later failure

**P2; application recovery contract.** Source: `writer/mod.rs:559-589`.

The API promises explicit independent per-braid outcomes, but returns plain `Err` if a later submit fails. Earlier successful `BraidOutcome`s and the closure's return value are dropped. Callers cannot learn which parts were definitely published from that result and may blindly retry all parts. Set insert/delete idempotence helps only in limited cases; generated IDs, external actions, and read-derived decisions are not made safe by erasing the receipt.

**Direction/tests:** return completed per-braid receipts plus the interrupted/pending remainder on the failure arm, with explicit ambiguity semantics. Inject a store failure on the second braid after the first is `Published`; assert the first receipt remains available to the caller.

## Philosophy carried to its logical conclusion

### Preserve the algebra; stop substituting nearby facts for its premises

The set-semantic, schema-closed theory gives valuable facts: independent components can commute, every admitted batch has a checkable meaning, and a vector identifies a legal combination of prefixes. It does **not** give these additional facts for free:

| Fact actually established | Stronger fact currently assumed in places |
|---|---|
| A candidate is an invariant-valid prefix | It safely replaces every prior recovery floor |
| A fetched value equals my desired counter value | I exclusively own the corresponding allocation |
| My lease token was current before an operation | The resource atomically rejects my later stale operation |
| My local cache has a floor | No global retirement occurred since I read it |
| A candidate is not the current head | It is unreachable and safe to delete |
| The batch's database generation matches a sum | The entire database is the intended prefix rather than another same-sized history |

The design should name which layer proves each right-hand claim, or avoid claiming it. This is the highest-leverage architectural improvement in this part of the system.

### One recovery domain needs one coherent authority model

Choose explicitly among multiwriter optimistic arbitration, a fenced resident writer with takeover, and local exclusive ownership. These can coexist as modes, but `writer_id` cannot quietly serve as ownership epoch for some objects while any writer can publish unfenced log slots. The winner is determined by create-only log occupancy; the ID allocator currently installs a different winner ordering. That contradiction is a root cause, not a missing retry parameter.

Likewise, retention is part of the write protocol, not a detachable housekeeping function. Once log keys are reusable after delete, create-only arbitration is no longer append-only unless tombstones/epochs/floors make that property durable. “Five verbs, no LIST” is attractive minimalism only if those verbs can express the safety conditions the product actually needs.

### Make the physical local adapter simpler than the database

The local object adapter currently implements expiring distributed-style lease acquisition, head compaction, temp-file publication, separate generation records, and recovery/sweeping. This is a substantial second storage engine. For a same-host filesystem, simpler kernel locks or an embedded transactional KV store can be safer and faster. The implementation should earn a production-tier claim through process-kill and power-loss tests, not only successful in-process contention tests.

### High-performance per-tenant does not mean merely small tenants in a benchmark

The present loss path deletes/rebuilds the whole local database (`writer/loss.rs:78-82`, `writer/open.rs:138-184`). It does not implement a pairwise commute fast path just because the research thesis discusses one. This may be a sound simplicity tradeoff for small tenants and rare races, but benchmark claims must include tenant size, checkpoint size, tail length, contender count, and request latency/cost per losing commit.

Test the large but still plausible tenant: a few GB, long retention history, bursty writes, checkpoint overlap, several app instances, and an idle instance waking after months. Small-tenant isolation is valuable; it is not permission to make every “rare” path unbounded in lifetime history or whole-database size.

### Explicit contracts to settle before AWS production

1. Is `Published` irrevocable after acknowledgement, including GC, stale writer resumption, and ambiguous responses? It should be if it is advertised as RPO zero.
2. Are ID ranges unique among all concurrently live writers, and what terminates a fenced allocator?
3. Is retention a guaranteed reconstructible time/vector window or best effort? What metadata proves age across restart?
4. Can a tenant read handle mutate state? If not, the type should say so.
5. What bounded work/deadline does an app request, cold open, refresh, shutdown, or checkpoint owe its host?
6. Are filesystem and S3 stores equivalent for every accepted key and race, or are there explicitly narrower supported modes?
7. What is the corruption trust model: trusted authenticated writers only, accidental storage damage, buggy writer, or hostile writer? Checks and claims should match that model.

## Missing verification campaigns

- A small executable history model spanning **write, checkpoint, GC, takeover, reopen**, not separate happy-path suites for each subsystem.
- Every success response paired with a fresh-replica observation after injected crashes and retention.
- Real process suspension (`SIGSTOP`/resume or equivalent) across lease expiry; fault hooks after coarse writer steps do not stop inside backend atomicity gaps.
- Backend contract tests against actual S3-compatible servers and AWS: conditional headers, 409, lost response, metadata/ETag semantics, prefix/key equivalence, credential renewal, and body-stream failure.
- Independent allocation histories checking caller-visible interval uniqueness across ambiguous transport results.
- Retention boundary tests that construct actual checkpoint ancestry and verify restore, not just expected deleted object names.
- Long-lifetime request/syscall counts for GC and filesystem CAS, plus sustained-write checkpoint completion.
- A safe refusal on opening an active directory that demonstrably performs **zero mutation** before ownership is established.
- Fault injection at every filesystem fsync/link/rename/generation write, not only `WriterStep` boundaries.

## Things checked and not promoted to bugs

- **Concrete command replay rather than rerunning host closures is deliberate.** `record` executes the body once and loss rejudges recorded ops. Do not casually “fix” this by rerunning application side effects. The application still needs a documented read-dependency/optimistic-precondition contract.
- **No-op log suppression is deliberate.** Avoiding a log slot for a net no-op is necessary for the generation/sum identity; returning the current slot is not itself data loss.
- **Ambiguous raw counter allocations are intentionally abandoned.** That is the correct uniqueness-preserving behavior in `lease_block`; the bug is the backend hiding the ambiguity first.
- **Componentwise session vectors are sound for independent braids.** The bug is using a scalar sum as the *recovery/collection* order, not the existence of vector restore points.
- **Catalog comparison is stronger than hashing physical LMDB bytes.** Page layouts/allocation order can differ for the same facts. The logical digest is a sensible primitive; its execution point, anchoring, and completeness need to be precise.
- **Local acknowledgement is explicitly provisional.** `LocalPending` is not being scored as equivalent to a published commit. The severe findings above include failures of `Published` or retained recovery history.
- **Malformed-input parsing is generally defensive.** The codec/document work deserves preservation. Canonical grammar tests do not substitute for the concurrency and durability schedules described here.

## Suggested order of work

1. Freeze the `Published`, ID uniqueness, checkpoint dominance, and retention invariants as model-level properties.
2. Fix the publication/retirement authority model and counter ambiguity/identity model together; do not patch only cached-floor reads.
3. Replace/repair filesystem CAS and establish lifecycle ownership before cleanup.
4. Make checkpoint history, timestamps, scratch ownership, and GC one crash-safe protocol.
5. Narrow read capabilities and preserve partial completion receipts.
6. Add bounded progress and lifecycle-long performance measurements, then tune the common path.

The ambition is compatible with a much smaller trusted mechanism. The goal should be fewer places where a comment says “by law” and more places where the type, atomic operation, or executable history model actually carries the law.
