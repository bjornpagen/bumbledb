# 21 — Bounded materialization, checkpoints, and safe deletion

Execution routing: P05 leads; P04 authority; P02 coherent export; P09 staged targets; P08 wrappers; P12/P14 schedules/cost. C06/C07/C08 own shared transitions. See [work packets](62-work-packets.md) for source ownership and complete deliverables.

Status: proposed 1.0 requirements. These are internal mechanisms behind the TypeScript `bumbledb-log` product; the main engine supplies ordinary consistent storage/snapshot primitives. No public Rust/C log storage API is implied. None of the new protocol or qualification tests is claimed implemented.

Keep LMDB. Remove the old arbitrary 32 GiB limit. A database larger than RAM is normal: LMDB pages through the operating system, performance degrades with locality, and the database continues to work when its data and temporary work fit available storage and address space. A map-size reservation is not an RSS reservation. A configured host disk quota can refuse placement or growth, but it is not a semantic maximum database size.

The remote representation is intentionally modest: a linear decision tail and **streamed complete checkpoints**, not an incremental remote B-tree or a general object DAG. The cost is O(database size) checkpoint work. Its actual frequency and amplification depend on application writes and the replay policy; calling it “occasional” is not evidence that it is cheap. It must be bounded in RAM, measurable, and completable within the qualified workload envelope during ongoing writes.

### Local-history specialization

The hosted object layout, remote tail envelope, and epoch GC below are not imposed on a local-history database. Its current facts/receipts/stamps already recover atomically through LMDB. Local named restore points use the same streamed canonical export, in a unique self-contained root directory without cross-root chunk sharing. Complete/fsync the export first, then atomically register its root metadata in local LMDB. A crash before registration leaves owned scratch, not a published point; a crash after registration leaves a durable complete root. Release removes the registry entry transactionally and performs owner-scoped directory cleanup afterward. Root IDs/directories are never reused.

Ordinary owner/task lifetime tracking protects active local exports from local scratch cleanup. After process death, the next process acquires the kernel lock before cleaning abandoned unregistered directories; the old process cannot resume publication after losing its life. A merely paused process retains that lock. Consequently local cleanup needs no epoch-closing S3 barrier, remote list protocol, or timed lease. Local named-root publication/release still receives its own kill/pause/failed-delete tests; repeated complete snapshots cost disk, explicitly, rather than a second shared-object collector.

## Objects and the one authority

```text
<configured-origin>/<database>/<incarnation>/HEAD
<configured-origin>/<database>/<incarnation>/objects/<epoch>/<kind>/<digest>
```

Protocol-generated path components are a single canonical lower-case ASCII encoding of binary identities, not user tenant names. Display names and arbitrary Unicode identifiers live in data/configuration. Every local directory also stores and verifies the original complete identity and configured origin. Hashing a directory name alone is not an origin check.

`HEAD` is never deleted or reused during the incarnation's operational life, including after logical tenant deletion: `Deleted` is a tombstone state. Normal writer credentials cannot delete it. Restoring old object versions over it is not a supported repair. A genuinely new database receives a new incarnation and path.

The Deleted variant in chapter 20 has **no active recovery root**. Only explicitly retained named roots and the roots of any already-running GC barrier continue protecting old objects. Deletion preserves that barrier's progress; the next collection can reclaim objects it conservatively protected once no retained root needs them. Cancelling a target before genesis uses this same terminal variant with no invented snapshot. Ordinary access refuses before trying to hydrate a tombstone.

An `ObjectRef` contains **epoch, kind, full 32-byte content digest, and expected length**; the storage key shown above uses epoch/kind/digest, while expected length is checked reference metadata. Readers verify the length, domain-separated digest, grammar version, identity, and nested references before interpretation. Checksums are not substitute authorization. The same digest at another epoch is a distinct storage name. Do not truncate object identity or GC marks to a fast table fingerprint: those names determine publication/reachability, rather than merely selecting candidates for full-value comparison. Reducing one chunk-reference digest from 32 to 16 bytes saves 16 bytes per eight-MiB chunk, not 16 bytes per fact; measure actual representation multiplicity before trading away the stronger identity assumption.

The production S3 adapter needs only the concrete operations this protocol uses: read a versioned head, conditionally create/replace it, stream immutable objects, list actual object names for collection, and delete eligible objects. Do not create a generic storage framework to defend the old five-verbs abstraction. A memory adapter and deterministic fault adapter exist for testing; a filesystem object store is not a second production storage engine.

## The local LMDB materializer

One owning process holds the directory's supported OS lock from **before reading recovery scratch or deleting anything** until native handles and mappings close. Lock ownership does not expire while a process is paused. A competing open either waits within its budget or refuses without changing files. NFS/network filesystems and unqualified locking semantics are not silently supported.

The local cache record contains:

- Complete origin, database/incarnation/schema identity, plus local format identity.
- Log decision/state stamps in the same LMDB transaction as their facts and receipt state.
- A complete/ready marker installed only after verification; incomplete hydration lives in a separate owned staging directory.
- Optional owned pending-command capsules, whose identities can be resolved remotely but whose uncommitted effects are not readable.

Do not infer identity from equal row counts, schema, or generations. Do not adopt stale pending work under a changed origin. On mismatch, return `ForeignCache`; an explicitly selected disposable-cache policy may build a new directory, but cannot perform cross-origin remote cleanup.

Map growth is geometric and checked against address-space and disk policy, not a hardcoded 32 GiB ceiling. The implementation must coordinate resize with active transactions using LMDB's documented rules on every supported platform. Long-lived read transactions can retain old pages and increase disk use: account for this, expose snapshot age/pinned bytes where measurable, and admit/limit maintenance overlap. Do not kill a valid query simply because the database is larger than cache.

No export, digest, catch-up, or hydration path may allocate `Vec<entire_database>`, `snapshotBytes`, or a whole-catalog in-memory sort. Stream in canonical key/fact order where possible; otherwise use the engine's same bounded LMDB-backed scratch-map/scan mechanism, charged to disk and execution budgets. Do not add a separate external-sort engine. A large individual command/value still obeys its explicit request budget; removing a database-size limit is not promising infinite single-value RAM.

## Streaming checkpoint format

A `LogSnapshotCertificate` wraps **one owned core read transaction**. Its facts, system receipt rows, schema, log attachment, and digests all come from that same snapshot. The capture is not “copy now, ask generation/digest later.” The source can continue accepting commits while the old read snapshot is exported.

The log owns export/import framing. A checkpoint is:

1. A canonical stream of schema identity, application facts, and log system records, including authoritative migration-history/provenance metadata. Physical LMDB row IDs, dictionary numbering, freelist layout, and host page sizes are not authoritative identities.
2. Fixed-target byte chunks, initially 8 MiB, **uncompressed in 1.0**. Chunks may split long framed records; the decoder is a bounded streaming parser. No compression codec or decompression path is selected.
3. A streamed manifest listing chunk order, object refs/byte lengths, stream digest, logical application/system digests, and the captured stamps. The manifest itself is streamed/spooled and multipart-uploaded if necessary; it need not fit RAM.

The first implementation is full logical export, not content-defined incremental chunking, a remote page cache, or a custom persistent tree. Identical immutable chunks can be reused **only where the GC reference rule permits**; correctness does not depend on cross-checkpoint deduplication. Snapshot compression and native LMDB snapshot-image acceleration are deferred, not dormant 1.0 format variants.

Upload concurrency is bounded and charged before buffers/requests start. Default two 8 MiB chunk buffers does not become “one buffer per chunk.” Multipart requests use checked manifests, explicit complete/abort outcomes, bounded retry, and a documented incomplete-upload cleanup policy. A multipart upload identifier is not publication. No checkpoint is referenced until every required object has been durably completed and content-verified.

Import builds a new owned LMDB directory, streams and checks every chunk, validates canonical values/theory/system invariants, and recomputes digests. It activates only after the complete certificate and replay boundary agree. The main engine's checked builder is a generic storage primitive; choosing source schema, backups, transformations, and lineage is log-layer work.

Full export's temporary disk is a real cost. It can stream chunks directly with a bounded retry spool instead of simultaneously requiring a second complete LMDB file and a second full in-memory image. Hydration normally needs the final local database plus bounded chunk/work scratch. Safe replacement may temporarily require old and new directories. Hosts must reserve that overlap or refuse/relocate before starting; do not delete the old live mapping to squeeze under a quota.

## Checkpoint publication without a quiet window

Let the captured certificate describe decision S. Let a newly read current head describe T, with current checkpoint B.

- Require `B.seq <= S.seq <= T.seq`, same lineage, and a verified ancestor relationship. Retain the exact decisions `(S,T]` in the new recovery root. Equality at B is harmless but should not cause repeated pointless publication.
- Build the proposed replacement from the **current exact head**, preserving all its roots, receipt policy, mode, and GC state. Its application state/decision stamps remain T; only its recovery representation changes.
- The exported snapshot may be older than current writes. New writes alone do not invalidate it. A changed head causes bounded head rebase and tail validation, not a new whole-database copy.
- If another checkpoint has passed S, discard the candidate or use it only through an explicitly retained named root. Do not move the current recovery base backwards.
- If the object epoch moved during export, new dependencies staged under a now-closed epoch must be restaged under the current epoch unless already inherited from the exact parent closure. Relabeling a manifest without restoring its child objects is insufficient.
- A successful head CAS is the checkpoint linearization point. Its crash recovery never asks only whether this candidate is still the current checkpoint; it may now be a retained ancestor/root.

There is one configurable finite tail envelope in count **and bytes**. The earlier 4,096-decision/64-MiB values are **unqualified illustrative policy, not universal defaults, correctness constants, or tenant-size limits**. Select and qualify the envelope and checkpoint trigger against the application's decision rate, command/result sizes, checkpoint throughput and recovery budget. Start checkpointing before either bound, reserving measured headroom. When admission would exceed the configured envelope, return `MaintenanceRequired`/backpressure until a checkpoint advances. This includes no-op/rejection decisions and large receipt results. A bounded individual refresh call does not itself justify a particular global tail cap; its finite captured target, work budget and progress result are separate mechanisms.

Continuous writes can slow export through pinned-page/disk pressure, but do not force restart on every commit. If checkpoint work cannot finish within resource/deadline limits, expose health and refuse further growth at the envelope. Fair progress assumes the storage/CPU eventually supplies work and maintenance is not adversarially invalidated forever; individual requests still have bounded exit under contention.

### Qualify the cost against the application, not an invented small-tenant promise

Necessary no-stall headroom is approximately `decision_rate × checkpoint_duration` in count and `tail_byte_rate × checkpoint_duration` in bytes, with additional margin for bursts, retries and competing maintenance. Both limits apply. Starting earlier cannot overcome an envelope smaller than the tail produced during one checkpoint. A full export may also evict useful cached pages and retain old LMDB pages while its read snapshot remains open; measure foreground-query latency and peak local disk alongside upload throughput.

Illustrative arithmetic, **not a benchmark**: exporting 40 GiB at an effective 100 MiB/s takes about 410 seconds. At 100 terminal decisions/s, about 41,000 decisions arrive during that interval, already ten times the earlier 4,096 example. A full 40-GiB export for each 64 MiB of new tail is 640 times that tail's bytes before compression/reuse; the count trigger may fire earlier. These numbers establish why policy requires measurement, not that these rates describe a user's database or AWS.

The product target is per-student/per-user application databases. Qualify the observed small/median/large tenant distribution, bursty writes, idle-tenant reactivation and shared-host churn on Apple Silicon, ARM Graviton/AWS, and the selected x86 Vercel Node runtime. Report cold materialization separately from warm reads: a 40-GiB uncompressed snapshot contains 5,120 eight-MiB chunks and requires complete local construction/verification before serving, regardless of the available RAM. A host without sufficient local disk cannot serve that tenant with this representation. Larger tenants remain valid on appropriately provisioned hosts; “works beyond RAM” is not “instant cold start on any serverless host.”

Use those measurements to set explicit write/query latency, cold-open/recovery time, maintenance headroom and cost targets before claiming a qualified deployment profile. Failure to meet them means adjust policy/placement or disclose an unsupported workload; it does not authorize quietly weakening durable receipts or introducing a new remote storage engine. LocalHistory remains the direct-LMDB durability option without synchronous S3 publication or the hosted tail envelope.

## Retention: deliberately smaller than a time-travel platform

The 1.0 promise is **the current recovery root plus explicitly named restore points**. There is no automatic 90-day PITR window, wall-clock GC sentinel, arbitrary historical-vector API, or fleet retention orchestrator. A user wanting a timed policy can explicitly create/copy/release named points with an external scheduler and a documented runbook; the library does not pretend its clock proves a recovery interval.

`NamedRoot` contains a unique root ID, kind (`RestorePoint` or temporary `HydrationHold`), exact recovery root, captured stamps and bounded control projection (receipt policy/activation/access provenance), bounded label/owner metadata, and an operation identity. Root creation/removal is a head CAS and participates in the same GC barrier. Register a captured root against its exact captured head; after movement, reselect/revalidate before trying to attach it to a successor. This is essential to prevent a pin from resurrecting a root already eligible for deletion. A restore point survives until explicit release. Root deletion reports the lost recovery capability before objects are later collected. Captured control is provenance, not permission to re-activate a restored old authority.

A long remote hydration can hold its root. A failed client may leave a hold: report it, count it against capacity, and allow explicit administrative revocation. No lease timer silently turns a backup into an orphan. Revoking a hydration hold can make an incomplete import fail `SnapshotExpired`; it cannot turn partial imported facts into a readable snapshot. Once a full local LMDB snapshot is verified and owned, it no longer depends on remote retention and can release the hold.

This is intentionally not a general pin service: the same bounded head list serves both named restore points and temporary holds, with explicit capacity and cleanup. A normal short open may choose bounded retry without a hold, but missing dependencies cause failure/reselection, never incomplete state. Long-running backups must acquire a durable restore point before copying.

An old restore point continues to contain its original receipt/system state for faithful recovery/export, but it does not change current request admission. An expired command ID cannot regain execution rights because a historical snapshot contains it. Creating a writable branch from any old point creates a new incarnation.

## Why epoch-qualified object names are necessary

A plain head CAS does not solve garbage collection:

1. A writer uploads object X and pauses before head publication.
2. GC sees X unreferenced and deletes it.
3. The writer's still-valid head CAS publishes a reference to missing X.

Reading `HEAD` once more before step 3 leaves the same pause between check and write. Nor is global `objects/<hash>` enough: a new writer can reuse the same hash/name while a delayed collector still intends to delete it.

The representation needs one additional fact: **which object namespace was open when this exact head was read**. The CAS that closes that namespace invalidates every old publication attempt at once. This is retention's unavoidable coordination, not a distributed lease system.

## GC protocol: one active barrier, immutable mark evidence

The complete persistent GC state is in the existing head. No collector owns authority by a wall-clock lease. Multiple collectors may duplicate safe work; head CAS selects durable progress. Every proposed progress transition carries its barrier ID and expected phase/cursor. Rebase may preserve intervening data changes, but cannot regress a cursor, revive a completed barrier, clear a newer barrier, or swap in mark evidence from another job. A stale collector receives `CollectionMoved`/`AlreadyFinished` rather than unconditionally installing its old progress.

### 1. Close an object epoch

From exact head H with epoch E and `gc=Idle`, form a barrier containing its Live recovery root, if any, and all explicitly retained named roots in H. A Deleted head contributes no active root; an empty root set is valid. Publish H' by CAS, incrementing `object_epoch` to E+1 and installing `Marking { cutoff_epoch:E, protected_roots, barrier_id }`.

The root set is immutable for this collection. It need not include future roots because of the reference rule below. Commands and maintenance continue after the barrier using the new epoch. If a candidate CAS races the barrier, either the candidate wins first and is in the captured roots, or it loses and must rebuild against the new epoch.

### 2. Mark exact dependency closure

Walk each protected recovery root: its certificate/manifest/chunks and its decisions only down to the certificate's stopping stamp. Include authoritative migration-history/genesis provenance records, system receipt/result records, and supported blob dependencies where they are part of that root's declared format. A provenance citation of an external old origin is historical metadata, not an implicit promise to retain that entire origin forever; required copied history records are explicit refs. Validate every object hash/length and bounded grammar before following it. Maintain visited/work limits; malformed/missing required dependencies stop GC **without deletion**.

Store the complete set of protected `ObjectRef`s using bounded disk-backed work, then stream an immutable checked mark manifest. Only a complete verified mark set can transition the head to `Sweeping`. Partial work or a crashed mark task is not a deletion certificate. New mark-work objects live in the current epoch, outside the collection cutoff.

### 3. Sweep only closed names

List actual `objects/` keys in bounded pages. Delete a key only when its parsed epoch is `<= cutoff_epoch` **and** it is absent from the complete mark set. Never delete `HEAD`, an unknown/unparseable namespace, backup storage, a newer epoch, or an object required by a protected root. Persist sweep progress in the head using exact CAS; concurrent data publications are preserved when rebasing a progress update.

Delete failures leave retryable progress/error evidence. A transient failure must not advance durable progress past a required failed deletion as if it succeeded. Repeating an already successful deletion is harmless. A service pagination token is an optimization, not a perpetual promise: if invalidated after restart, restart a bounded pass and use idempotence.

At the end, CAS `gc=Idle`, retaining monotone object epoch. The previous mark objects then become ordinary unreachable objects eligible in a later collection. Do not keep every historical empty epoch, token number, or deleted slot as a probe loop.

### 4. Enforce the reference-introduction rule

For every head transition, every newly reachable object must be either:

- In the validated dependency closure of its **exact parent head**, or
- Staged under that parent's currently open object epoch, with all of its dependencies satisfying this same rule.

This applies to decisions, checkpoints, root additions, imported blobs, and receipt metadata—not only ordinary writes. A raw old hash found in local scratch or an external manifest is not eligible merely because GET succeeds. Reintroduce it by restaging under the current epoch, including its necessary child objects, or use an already retained root.

### The safety argument

At the epoch-closing CAS, every publishable pre-barrier reference is either included in the barrier closure or its old expected version is invalidated. Afterward, every newly introduced object uses a newer, noneligible name. Inherited old references are a subset of the protected closure, inductively through every exact-parent head successor. Therefore an object selected by this collector—old epoch and unmarked—cannot be required by any valid current or future head reached during this collection. Adding/removing roots later cannot violate the induction because root addition obeys the same rule and root removal only shrinks reachability.

An old client may still upload an old-epoch object after GC passes its name, even after arbitrarily long suspension. It cannot publish it against the changed head. This can leave an orphan, not lose acknowledged data. A later bounded listing pass finds the actual object and collects it. Complete reclamation requires quiescence/fair eventual listing; the design does **not** claim a finite pass can prove no paused client will upload another orphan later. S3 listing pagination is not assumed to be a global snapshot. Missing a late key costs storage until reconciliation, not safety.

Listing actual extant names means an empty old namespace costs no millions-of-404 historical-slot scan. Mark/sweep is O(current retained objects + actual orphan objects), plus resumable overhead. It is not O(every commit ever made) after those objects have disappeared. Full retained-state scans remain a real cost of this simple design.

## Blob and external-effect boundary

The database stores ordinary application references, not a magic distributed transaction with arbitrary objects. The initial supported backup format does not recursively chase arbitrary URLs from user data. If the application uses external blobs, its log-layer backup manifest/runbook must explicitly declare which content-addressed refs are included.

Upload and verify such a blob before publishing its reference. Collect it only under an explicit policy that accounts for current facts, restore points, and independent backup roots; otherwise leave blob GC out of 1.0. Never assume a text field containing an S3 URL is complete reachability evidence. External messages/payments use application outbox facts and receiver idempotency; database receipt semantics do not promise exactly-once external networking.

## Backend qualification, not a compatibility checkbox

### S3

Production qualification is for a specific AWS S3 configuration: supported region/bucket class, same authority origin, conditional headers, access roles, encryption settings, SDK version, and object limits. Strong HEAD/object-read consistency and correct conditional replacement are required. “S3-compatible” is not automatically qualified. Cross-region asynchronous replicas cannot be publication authority under the same claim.

Test actual ETag/version semantics; keep ETags opaque. Ensure every proposed head body differs through its monotone revision even when application state does not. IAM prevents ordinary delete/overwrite bypass for the head and distinguishes ordinary publication from destructive maintenance/backup roles. The library still assumes authorized writers obey the protocol; IAM does not validate a relational law or the content of a head transition.

Object versioning is not automatically a backup. Delete markers, noncurrent-version retention, lifecycle rules, encryption-key access, and restore permissions change the result. Automatic bucket lifecycle rules must not delete live current/restore objects. If the active namespace is versioned, old versions and delete markers need explicit qualified handling and cost accounting; GC deleting the current key alone does not prove erasure or bounded billable storage. Ordinary GC must never own the independent backup namespace.

### Local filesystem

Qualify actual supported OS/filesystem pairs: the intended matrix includes Apple Silicon macOS/APFS, ARM Graviton Linux storage, and the exact x86 Vercel Node environment selected for hosting. CPU architecture alone does not qualify its filesystem, ephemeral-disk quota, process lifetime, or native package. Ownership uses a process-lifetime kernel primitive, not “lease checked before rename.” Local-history head, fact changes, receipt, and metadata are one durable LMDB transaction; there is no separately fsynced generation sidecar. Cache install/removal is owner-scoped and happens before releasing its lock.

File/directory fsync and rename guarantees remain essential during creation/checkpoint activation. Failed activation leaves old or new complete state, never a ready marker over partial state. Mapped files are not renamed/replaced/deleted while native owners still use them. Platform support is earned by process/power-failure tests, not borrowed from MemStore.

## Required storage and GC test suite

All gates below are release-blocking for the corresponding supported backend; skipped credentials/platform tests are **not** qualification. Use disposable fixtures and explicit authorization for real cloud fault work. This proposal performs no AWS actions.

| Gate | Required scenario and oracle |
|---|---|
| `STORE-01` | Stream export/import of a database several times greater than the test machine's enforced RAM budget; exact application/system digests, bounded RSS, no full-catalog allocation |
| `STORE-02` | Grow across 32 GiB and several elastic map-resize boundaries with concurrent readers/writes; correct facts and typed real disk/address exhaustion |
| `STORE-03` | Capture snapshot, continuously commit while exporting; complete export matches the captured attachment/stamps, not later generation; no quiet-window restart |
| `STORE-04` | Staged checkpoint S, newer T writes, other checkpoint B; preserve `(S,T]`, reject backwards base, and keep all retained roots |
| `STORE-05` | Chunk corruption, truncation, wrong key/hash/length, unsupported compression framing, malformed manifest and cyclic refs; bounded refusal before activation or deletion, with no decompressor invoked |
| `STORE-06` | Abort each chunk/multipart request, lose completion response, retry, crash; never publish an incomplete snapshot; orphan discovery remains possible |
| `STORE-07` | Replay envelope reached during sustained writes/failed maintenance; bounded refusal, then progress after checkpoint, with no acknowledged loss |
| `STORE-08` | Cache has same schema/counters under another prefix/account/incarnation; refuse before reads, replay, or cleanup; include case/Unicode aliases on actual filesystems |
| `STORE-09` | Hold read mappings during resize, replace, close, eviction and cache reinstall; no use-after-close or deletion of successor files |
| `STORE-10` | Actual per-user/per-student tenant size/burst/churn profiles, including a >RAM/>32-GiB lane on a fitting host: sustained writes through repeated full checkpoints plus cold hydration; report p50/p99 reads/writes/open, throughput, checkpoint/tail amplification, object requests, RSS and peak disk; select tail/trigger policy from measured headroom and pass declared targets on each claimed platform |
| `LOCAL-01` | Local named export: kill before/after chunk/fsync/complete-directory/root-registration boundaries; registered points are complete, unregistered scratch is not a restore point, and active owner exports survive unrelated cleanup |
| `LOCAL-02` | Release a local point, fail deletion, crash and reopen; resume owner-scoped cleanup without touching another registered point or active export; distinct root directories share no collectible files |
| `LOCAL-03` | LocalHistory grows beyond hosted tail policy, retires receipts, reopens, and restores a named point; LMDB recovery needs no remote replay checkpoint, current retirement is atomic, old point preserves its original evidence |
| `GC-01` | Old writer stages X, barrier marks/deletes X, old writer resumes; old CAS cannot publish; restaged new-epoch X is never hit by old collector |
| `GC-02` | Pause before/after epoch CAS on both writer and collector; every interleaving either protects the candidate or invalidates its expected head |
| `GC-03` | Drop current checkpoint while a named restore point still needs it; restore still succeeds after complete collection; tombstone an authority during a barrier, preserve its progress, then collect former live objects in a later pass after explicit roots release; the tombstone itself remains |
| `GC-04` | Crash after checkpoint CAS, advance head twice, reopen scratch; no immediate remote delete based on current-head inequality |
| `GC-05` | Add/remove roots concurrently with mark/sweep; include repinning an unretained old ref; rule rejects unsafe introduction |
| `GC-06` | Kill during mark, upload partial mark set, corrupt one mark page; no sweep from incomplete proof |
| `GC-07` | Fail each individual delete and progress CAS; restart/resume, preserve failed work, and converge under eventual successful storage |
| `GC-08` | Old client uploads after collector cursor/finish; next actual-object reconciliation finds it; no claim of one-pass perfect reclamation |
| `GC-09` | Repeated GC over large prior history with mostly empty old epochs; request count tracks extant retained/orphan keys, not historical slot count |
| `GC-10` | Two collectors duplicate work, move head between progress writes, cancel/close; no regressed epoch/roots or data head overwritten |
| `GC-11` | Root list at capacity; failure does not replace another root; stale release cannot remove a different root ID |
| `GC-12` | Revoke hydration hold midstream; no readable partial cache, typed failure; previously complete local reader remains correct |
| `GC-13` | Receipt retirement racing checkpoint/GC/unknown CAS; no live receipt below retained policy disappears and no expired request executes |
| `FS-01` | Real subprocess SIGSTOP before/after every lock/transaction/fsync/rename/install step; successor cannot take the still-held lock or be erased by resumed predecessor |
| `FS-02` | Real subprocess SIGKILL at each coarse and backend-internal durable boundary; reopen sees old/new complete state, never mixed facts/attachment |
| `FS-03` | Inject ENOSPC, EIO, short write, failed fsync/rename/directory sync, corrupt metadata; fail closed and retain a recoverable owner-specific next action |
| `FS-04` | Competing open against live scratch performs zero mutation before ownership; native close releases lock/mapping deterministically |
| `FS-05` | Filesystem/power-failure harness on supported platforms, not only SIGKILL; record actual durability assumptions and excluded mounts |
| `S3-01` | Real conditional-create and exact-version replacement races; verify one winner and no ABA under identical fact states |
| `S3-02` | Real lost response/body, timeout, 409/412, abort, retry and HEAD/GET failure; check client-visible receipt histories, not only final object values |
| `S3-03` | Credentials rotate mid-operation; prefix authorization denied; checksum/ETag/header behavior; no wrong-origin cache reuse or leaked secrets |
| `S3-04` | Real multipart completion ambiguity and incomplete-upload cleanup; published checkpoints fully hydrate with bounded RAM |
| `S3-05` | Real LIST pagination concurrent with late PUT/delete and interrupted resume; missing listing entries only defer orphan collection |
| `S3-06` | Actual configured versioning/lifecycle/encryption/backup-role policy; restore and erasure semantics match the declared deployment |

The deterministic scheduler must inject outcomes **inside** adapters, not merely after writer phases. Keep an independent expected object/root set and a record of every published receipt. After every generated crash/GC history, bootstrap a fresh directory and verify all effects that should remain in current state plus every retained restore point.

Resource tests need quantitative limits: bytes buffered, disk reserved/used, native owners, outstanding requests, retries, mark entries, pages listed, CPU work, cancellation-to-quiescence time, and tail amplification. In the >RAM lane, page faults/slower execution are acceptable; silent wrong answers, process-wide unbounded heap growth, or an invented semantic size refusal are not.

## Audit disposition

| Audit IDs | Replacement | Closure/cost |
|---|---|---|
| REP-001/002 | Exact-head epoch barrier; one ordered recovery boundary | GC-01/02, STORE-04; an epoch transition and real retained-state marking |
| REP-003 | No implicit age-based retention in 1.0; named roots are explicit | GC-03/11; timed PITR is deliberately not promised |
| REP-005/009/010/017; SDK-006 | Process-lifetime local lock, ownership before cleanup, one LMDB metadata/facts transaction | FS-01–05; no takeover of a merely paused process |
| REP-007/008/019 | Complete retained-root closure, no scratch-triggered remote delete, LIST-based orphan reconciliation | GC-03–10; O(extant retained objects) scans, eventual rather than instantaneous orphan reclamation |
| REP-011; SDK-016; ARCH-004 | Canonical generated paths plus verified identity/origin envelope | STORE-08; explicit relocation/reseed operation |
| REP-012/013; SDK-012 | No historical lease-token chain/slot probing; bounded actual-key listing and progress | GC-07/09; remote LIST and durable collection metadata are accepted costs |
| REP-014; ENG-003 | Owned snapshot certificate, streaming export, exact-parent checkpoint rebase | STORE-01/03/04; pinned LMDB pages and full-export I/O |
| REP-018 | Hash/length checking and bounded typed traversal at every root boundary | STORE-05; validation work is not optional on cold paths |
| SDK-011; QRY-002/003; PERF-002/005 | Admission for disk, native, transient and I/O resources; no database-size/RAM conflation | STORE-01/02/07/09 and engine/SDK resource gates |
| OPS-002/003/005/006 | Named roots, explicit blob boundary, missing != empty, maintenance health | S3/GC gates and chapter 22; no silent cloud/platform guarantees |

The scope cut is intentional: a small correct retention mechanism, not a smaller test obligation. Deletion is the only reason the epoch/mark representation exists; remove safe GC from the product and it could be removed, but ordinary storage growth and erasure would then be explicitly unsolved.
