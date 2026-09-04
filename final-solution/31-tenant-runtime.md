# 31 — Deterministic ownership and a bounded tenant runtime

Status: proposed 1.0 design. No implementation or qualification claim. This chapter covers the minimum runtime needed to embed the database safely; it does **not** introduce a fleet manager, placement service, orchestration framework or generic plugin system.

## Keep the runtime smaller than the database

There are three necessary pieces: an owner that actually closes native resources, an optional local cache registry returning independent borrows, and a bounded worker adapter for async hosts. Rust implements them once. TypeScript adds ergonomic lifetimes and structured value conversion, not another protocol implementation. The core is public Rust/TypeScript only; the entire C product is removed during implementation. Log history and generated migration execution are public TypeScript only. Internal Rust owners/protocol types are implementation details, not a public Rust log SDK.

LMDB already provides local reader snapshots, atomic transactions, durable pages and mapping. Do not reimplement these with expiring filesystem leases or custom journal files. The log package alone adds remote history, pending command reconciliation, checkpoint/backup/restore and migration. Core-only users need none of those features.

The registry is an ordinary library data structure keyed by database identity. It does not discover tenants, authenticate users, decide geographic placement, create cloud resources or migrate data automatically. A host supplies a trusted binding and resource policy. When that host cannot serve a workload, the library returns an actionable result; the application/runbook makes the deployment decision.

## One owner, independent borrows, owned results

| Object | Owns | Does not own |
| --- | --- | --- |
| `DbOwner` / log `HistoryOwner` | Native environment, local ownership lock, active tasks, registered snapshots/sessions, and log transport handles when applicable | Application authentication, remote tenant identity assignment, global scheduling |
| Registry slot | One owner and a unique `SlotId` for this open incarnation | Every borrow object ever issued |
| `TenantBorrow` | One independently spent borrow token bound to that exact slot | The right to close another borrow or the underlying shared owner |
| `Snapshot` | An owner-registered read capability at one coherent state | Mutation or publication authority |
| `ExecutionSession` | Bounded mutable per-snapshot query state and caches | A schema-wide tenant data cache |
| `CompleteResult` | Completed owned rows or independent temporary-LMDB result storage | A hidden pin on the source database after execution completes |
| `Command` / witness / receipt | Owned immutable values | A live database or directory lock |

Every acquire creates a fresh `TenantBorrow`, even when the underlying slot is reused. It captures `SlotId` and a one-shot release state. Releasing it twice is harmless; using that borrow after release returns `ClosedHandle`. It cannot decrement a successor slot. No public `release(tenantName)` exists.

Managed snapshots/sessions derived from a borrow carry that borrow's identity as well as the slot identity. Release revokes future use of those children and drops their idle native guards; it cannot leave an escaped child acting through a returned borrow. Already-running operations retain their separately counted operation leases until completion/cancellation, so zero borrow count alone does not permit eviction. Completed owned results/receipts/witnesses remain independent values. Direct Rust guards enforce the corresponding lifetime through borrowing or an explicitly counted owned guard, never an uncounted escape.

`await using borrow = await cache.acquire(binding, options)` releases **that borrow**. It never closes the shared owner. Only registry eviction/shutdown owns that decision. There is no `_shared` magic exception; ordinary explicitly held borrows express pinning. Cross-database references remain application data, not hidden transactions.

Operations take a temporary owner operation lease after checking the borrow/snapshot state. Registry eviction cannot start an active operation after marking the slot draining. Completing an operation drops its lease. No user callback or network await runs while the short bookkeeping lock is held.

An idle **managed Node snapshot token** is revocable when its owner closes. A running native operation cannot have its data freed underneath it: it holds an operation lease until a cancellation safe point and teardown complete. Managed public result values are owned; methods do not hand out arbitrary references into a mapping that an unrelated close could unmap. No C borrowed-view or public C lifetime contract remains.

Direct public core Rust `OwnedSnapshot` guards and internal Rust guards/legally borrowed page views follow Rust's actual lifetime contract: the owner must drain/wait or report `CloseIncomplete`/`ResizeBlockedByReaders` until those guards drop. It must never invalidate a live Rust borrow to force shutdown or map growth. Managed wrapper tokens hold their native snapshots inside the registry, which can drop idle native guards after revocation and active-call drain. This distinction buys language-appropriate safety without pretending a strong type can stop a caller intentionally holding a resource.

## The lifecycle is one sum, not loosely related booleans

```rust
enum OwnerState {
    Opening { attempt: OpenAttempt },
    Ready { resources: OwnedResources },
    Closing { drain: DrainState },
    Closed,
    Faulted { error: DbError, resources: RecoverableResources },
}
```

The variant owns only the resources valid for that state. `Faulted` is not a secret permission to keep serving uncertain facts. It may retain resources needed for safe shutdown or diagnosis; recovery is an explicit bounded operation. A local read-only cache with a last known published snapshot may serve that snapshot only through an explicitly documented healthy cached-read policy, not by ignoring a corruption error.

Opening is registered before asynchronous work starts. Same-binding callers may share that single open attempt but receive independent borrow capsules upon completion. A cancelled waiter does not necessarily cancel another waiter's still-needed open. When the last waiter leaves, cancel the open; background warming is not in 1.0.

Shutdown first closes admission, then joins/cancels registered opens, then drains owners. An open completing in the closing epoch tears itself down and never installs a ready slot or timer. Concurrent `close` calls join one stored closing operation. Registry state must not depend on a promise that can complete after it has been forgotten.

### Exact close contract

1. Change `Ready` to `Closing` under the owner lock. No new operation can start; already queued operations are rejected or cancelled according to dispatch status.
2. Revoke idle derived capabilities, including retained writers/borrows/snapshots/sessions. An already-running operation retains its lease until its cleanup safe point.
3. Signal cancellation to cancellable work. If publication was dispatched, preserve the command reference and return a terminal receipt when proven, otherwise `OutcomeUnknown`. Close does not roll back remote history.
4. Join active tasks, dispose snapshot transactions, release execution caches and spill handles, and deterministically drop the native environment.
5. While still holding directory exclusion, finish owned temporary cleanup or retain an explicit retryable cleanup record. Never delete paths acquired by a successor.
6. Release the directory lock last. Mark `Closed` only when native ownership and lock really are gone.

`close(deadline)` reports completion or `CloseIncomplete` with outstanding work/resources. If the deadline expires, the owner remains `Closing`; another close joins it. The registry cannot count those bytes or file descriptors as reclaimed. It may not open a successor into the same directory while the old owner remains live.

No library can guarantee that a permanently stuck kernel/filesystem call returns by a wall-clock deadline. The cooperative bound covers reachable engine/transport safe points; a host needing a hard wall-clock/process-memory kill boundary runs its bounded worker in a process and may terminate it. This is a documented deployment option, not an excuse to report a fake successful close.

GC/finalizers are a leak backstop only. Correct eviction, native lock release, memory accounting and tests must work with garbage collection disabled/delayed and references intentionally retained. Closing an owner renders retained wrappers inert; it does not wait for wrappers to become unreachable.

## Directory exclusion is local and kernel-held

Use the supported operating system's lifetime file lock around the physical local environment, acquired **before** any mount, sidecar recovery, cleanup or directory replacement. Keep it held through native close and teardown. A process paused for an hour still holds the lock; time does not mint a competing local owner. Process death releases it according to the supported OS primitive.

The lock has a stable namespace identity outside replaceable staging/materialization directories; renaming a directory or unlinking a lock file must not let another process lock a different inode for the same authority. Keep namespace locks and terminal cancellation/deletion markers out of ordinary scratch/cache cleanup. Final local migration-target install and cancellation use this same exclusion, as specified in chapter 22.

No wall-clock TTL, token predecessor chain, periodic lease renewal or check-then-rename proof is needed for this local resource. The lock does not claim to fence S3; the remote HEAD protocol provides publication authority. Shared network filesystems are unsupported unless separately qualified for the required lock and durability semantics. Local filesystem support is explicit, not inferred from a path looking ordinary.

A second open that fails exclusion performs zero tenant-state mutation. Do not sweep scratch first and then discover that the active owner still exists. All cleanup operates on validated owned children, never broad paths derived from an unchecked tenant label.

## Identity before cache reuse

For log history, fetch and parse bounded authoritative identity first. Under local exclusion, validate the cache binding before returning rows, adopting a pending command, interpreting scratch or publishing recovery work. The binding records `DatabaseId`, `IncarnationId`, `SchemaId`, format/codec versions, and the explicitly configured authority location. A location change requires explicit remount configuration; matching schema and generation does not establish origin.

Local names are fixed-width lowercase encodings of a cryptographic digest of the complete canonical binding, accompanied by the original binding record for exact comparison. Case folding and Unicode normalization cannot turn caller labels into aliases; a digest collision causes a binding mismatch refusal, not data sharing. Human tenant labels are display metadata and never concatenated directly into cache paths.

Refuse unexpected symlink/path redirection at the owned directory boundary on supported platforms. These measures defend ordinary misconfiguration and supported API use, not an attacker already able to replace arbitrary files as the database's OS user.

On mismatch, default to `CacheIdentityMismatch` before serving data. An explicit `discardMismatchedCache` open policy may close/quarantine the old cache and rebuild in a newly owned location, but must never submit its pending commands or delete its remote objects. Deleting/recreating a remote namespace creates a new incarnation. Receipt/session tokens from the old incarnation stay old.

Core-only stores perform their own store/catalog identity and schema checks; they do not acquire an artificial remote origin identity. Export/import and history identity policy remain in the log package.

## Bound operations, not logical database size

The old 32 GiB map cap disappears. A database larger than RAM is a first-class case. LMDB map address space is not resident memory; increasing a mapping is not allocating that amount of RAM. Optional relation images and memo state are caches, not prerequisites for querying a relation.

The runtime reports these separately:

| Quantity | Why it matters |
| --- | --- |
| Logical live/history bytes | User data and retention economics; not a RAM admission test |
| Virtual mapping extent | Address-space/LMDB map management; elastic within platform support |
| Allocated local disk bytes | Environment pages, scratch, checkpoint download/copy overlap and physical free-space feasibility |
| Engine-accounted working bytes | Buffers, images, hash/ordered-map state, decoded values and active query allocations |
| Optional retained-cache bytes | Evictable/rebuildable optimization state |
| Process RSS and OS limit | Includes page cache residency, stacks, allocator/runtime overhead and uninstrumented libraries; measured separately |
| In-flight transport/output bytes | Bounded buffers and outgoing result pages, not just final file lengths |

Enforce request work/working/spill/output limits by charging **before growth**, with a documented bounded chunk overshoot where unavoidable. A host-level total reservation prevents concurrent cold opens/queries from each independently consuming the same headroom. Reservation is released exactly once on all completion/error/close paths. An unknown measurement is an error/unknown estimate, never zero cost.

Do not pretend an engine allocation ledger is an exact hard RSS cap. Conservatively budget runtime/transport overhead and measure RSS; use OS process limits for a hard process boundary. The configured limit and effective measured/accounted values are exposed in `inspect`.

Pressure order is: trim idle optional caches, discard retained answer high-water capacity, evict unborrowed owners where useful, switch eligible work to temporary-LMDB scratch, then return a bounded queue/resource result if the request still cannot execute. Never admit unlimited work because all slots are borrowed. Never refuse a database solely because its logical bytes exceed RAM.

Local disk remains a real constraint. If an AWS Lambda or Vercel Node function's temporary filesystem cannot hold a checkpoint materialization plus needed scratch, report `InsufficientLocalDisk` with required/available estimates before attempting the large write. The host may use a larger local volume or another deployment; no automatic placement service is added. This is a worker's feasibility result, not an engine database-size cap. Vercel Node is a supported deployment target within its qualified envelope, distinct from Edge. Temporary files are disposable HostedHistory cache, never LocalHistory's promised durable store.

Map growth is coordinated at the engine owner; concurrent readers/remount and map-full retries obey the engine's snapshot contract. Retry immutable engine work without rerunning an application callback. Do not interpret a mapping allocation failure as permission to disable durability.

## Bounded workers, one implementation

Node's default asynchronous API submits native work to a fixed-capacity Rust executor shared by the runtime. A small queue and per-owner admission count prevent unbounded futures retaining command/query payloads. Network and blocking filesystem work use appropriately bounded execution lanes; there is not a separate async runtime per tenant. A prepared LMDB writer stays on its owning worker across the bounded remote attempt; it cannot migrate through an arbitrary async executor. No JavaScript application callback executes inside a native transaction.

The scheduling policy is deliberately simple: bounded FIFO queues with per-tenant concurrent-operation limits and round-robin admission among ready tenants. Long query loops poll the same work/cancellation context at bounded checkpoints. This is a small executor, not a pluggable scheduler architecture. LMDB's local writer exclusion remains the actual writer transaction rule.

Native query work, integrity judgment, replay and checkpoint encoding do not run synchronously on the Node event loop. Result conversion is page-bounded; a completed million-row query is not materialized into one enormous JS array. Command copying is synchronous, charged during checked finite ingestion, and limited before dispatch. Limits bound accepted data/completed ingestion steps, not the duration of arbitrary host getters or iterators. Async bulk-command ingestion is deferred; there is no second public ingestion protocol in 1.0.

Cancellation propagates through queue admission, reads, stream bodies, replay, query operators, LMDB scratch, checkpoint copy and backoff. It is checked at growth/work boundaries, not just once at function entry. Bounded retry policies apply per operation; a repeatedly identical corrupt replay never silently reseeds forever. Cached-read freshness and maintenance progress are observable even when a bounded pass does not finish.

The engine supports larger-than-RAM execution through disk-native access and temporary-LMDB scratch. A memory-constrained worker can be slower, but its answers cannot change. Whole-database **in-memory** hydration, full result collection and full-relation resident images are not mandatory intermediate steps. A cold S3 materialization still requires the selected full canonical checkpoint on local disk; this is not a remote demand-paged database design. The application reference workload is many small per-user/student databases, measured at cold, warm and eviction boundaries. Preserve the Free Join hot path for appropriate application queries; do not turn fallback correctness into an analytics-first product or promise local latency for S3-published writes.

## Streaming checkpoint/open and maintenance

These operations live in the **log** package. Snapshot metadata is fetched/parsing-limited separately from snapshot content. Authoritative content is the canonical chunked row/system-record stream, not a platform-dependent raw `.mdb` file. Bounded buffers/spool files, incremental hashes and declared-size limits feed the core's checked builder into an owned staging LMDB environment. A missing/untrusted content length never authorizes unbounded buffering. Verify the entire stream/certificate/digests before publishing its local mounted identity; interrupted candidates remain invisible.

Checkpoints pin an actual coherent source snapshot and its stamps once. They make progress while writes continue; a later commit does not force copying the entire store again. Copying, uploading, verifying and named-root publication each have a bounded work context and resumable protocol state as defined in the log chapters. No in-memory full-snapshot `Uint8Array` exists on this path.

Temporary space reservation includes source plus destination overlap, optional compact copy, transport staging and query scratch. One configured maintenance task per owner/runtime is enough; do not create an independent unaccounted duty subprocess for every application request. If a separate CLI process is used, the runbook includes its disk/CPU envelope and deadline and uses the same log implementation.

Retention is current recovery root plus explicitly named restore points. There is no silent 90-day promise or wall-clock expiration policy. Named-root pins are a bounded registry in authoritative metadata (default maximum 64 as specified by the protocol); capacity refusal is explicit. Dropping a pin is an authorized maintenance operation, not an LRU eviction heuristic.

## Minimal hosting adapter, including the Lambda example

The existing example must be replaced or explicitly kept as a historical non-production example. The production example is a small request adapter, not a database service framework.

1. Authenticate the request using the deployment's actual protected entry point. Resolve an authorized principal-to-database binding in application code; never trust a path/session token/body field as tenant authority. An unauthenticated `duty: true` body must not invoke administrative work.
2. Parse an explicit finite route/method grammar. Supported GET/POST paths are listed; unsupported methods get 405 and unknown paths 404. Do not default arbitrary events/non-POST verbs to a full read.
3. Apply raw event/body size limits **before** decoding. Honor `isBase64Encoded` with strict bounded base64 decoding, validate UTF-8, then parse bounded JSON and schema-tagged values. Reject arrays where objects are expected, duplicate/unknown fields according to the documented request grammar, malformed numeric encodings and invalid route IDs.
4. Create a request deadline from the platform's remaining execution time with an explicit cleanup margin. Propagate it to native work and AWS calls. A timed-out write response carries `CommandRef` and publication certainty; HTTP timeout is not a database rollback.
5. Acquire a borrow, execute a bounded query/command, and release it in `finally`/`await using`. Expose bounded result pages, not a scan of every note into one response. Define application response codes for decided rejection, precondition failure, overload and unknown publication.
6. Use the supported credential provider chain/refresh callback in Rust. Do not capture one static credential object for the lifetime of a resident host. Attach the actual least-privilege role to the deployed function; merely returning an intended role ARN is not permission configuration.
7. Keep administrative checkpoint/GC/restore invocations on an authenticated separate route/CLI identity, with only the permissions they need. If a subprocess is used, set timeout/output bounds, terminate/join it on cancellation and report interrupted work without pretending success.
8. On an owner fault after a successful open, stop issuing new operations from that slot, drain/close it and explicitly reopen/recover according to typed error policy. A forever-memoized successful writer is not a recovery strategy.

The example's package versions, Node runtime, native ABI and Linux libc target must all name the released artifact it actually deploys. Deployment tests inspect the attached role and function-URL/API authentication policy. Apple Silicon is the canonical local tuning target; AWS Graviton and Vercel Node x86-64 require portable correctness and real host tests without premature separate tuning projects. Hosted owner caching is safe under concurrent warm requests, but is never assumed to survive eviction or a new function instance. No AWS/Vercel test missing required deployment access counts as passed.

## Migration and deletion are finite log operations, not a platform

The core provides admitted-state construction and coherent snapshot copy primitives. The TypeScript log package exposes named-source pin, read-only export, validated import to a new incarnation, verification, and explicit source write closure/cutover primitives. Chapter 33 generates canonical migration plans from high-level TypeScript schema values and typed declarative intent. The migration runner consumes inert checked plans and the native log executor performs bounded transformations; it never evaluates schema authoring or migration callbacks. Ordinary compiled app modules may construct/import typed schema/query values exactly as the current SDK does. There is no runtime TypeScript compiler, mandatory generated runtime-type layer or helper-import purity framework.

A pending suffix is planned as one operation and one final destination/publication. Native fusion/copy planning avoids automatically rebuilding and publishing the entire database once per file; necessary intermediate checks or private scratch remain explicit and preserve declared step semantics. An incomplete operation restarts from its pinned source/plan, without JavaScript stack checkpoints. A documented downtime/cutover runbook sequences this; no fleet scheduler or dual-write engine is added. App open, requests, React hooks and cache acquisition never run migrations implicitly.

Closing an epoch, pinning a restore point and switching application routing are different actions. Unknown commands must be resolved or explicitly recorded as unknown before source closure; old IDs cannot be retried under new identity by automatic client fallback. Core schema checks stay strict.

Logical deletion, local cache eviction, live-data rebuild and erasure of retained logs/snapshots/backups are separately named outcomes. An owner close does not imply erased tenant data. The host runbook must include external blob references, credential/key retention and remote versioned objects when making an erasure claim. These policies do not belong in the core query engine.

## Inspection without an observability platform

One bounded `inspect()` snapshot exposes owner state, identity, published stamps, queued/active operation counts, unknown-command count/oldest age, progress toward a requested frontier, current root/pins, last maintenance error, accounted memory/cache/scratch/disk, mapping extent, and open handle counts. Structured events may be passed to a caller-supplied sink outside critical sections.

No unbounded in-memory event history, dynamic dashboard service or plugin system is necessary. Log messages are redacted by default. Repeated failure signatures become typed health state, not an endless `console.error` loop. Internal metrics label database IDs only when the host accepts cardinality/privacy consequences.

## Mandatory runtime and hosting release gates

| Gate | Required scenario and assertion |
| --- | --- |
| RUN-01 Borrow isolation | Concurrent same-tenant acquires produce distinct borrows. Double release, stale release after reopen, nested `await using`, escaped child snapshot/session, abandoned caller and retained writer cannot affect another borrow/slot. An active operation lease blocks eviction after its borrow releases. |
| RUN-02 Close interleavings | Pause each open phase; close before/after registration/native open/identity verification/slot installation. No live slot/timer/handle appears after completed shutdown. Repeated close joins one operation. |
| RUN-03 Native reclamation | Thousands of open/query/read/close cycles with GC disabled and inert Node core/log wrappers retained. Engine owners, locks, FDs and mappings return to baseline; same-path reopen succeeds; allocated disk is actually reclaimable. Separately held legal Rust snapshot guards cause explicit incomplete close until dropped, never unsafe revocation. Shared former-C lifecycle bugs remain Rust/Node regressions. |
| RUN-04 Revocation | Queue and run read/submit/query/maintenance, then close/release. Idle capabilities refuse; running work drains safely; known/unknown publication classification is preserved. Deadline failure leaves `Closing`, not false `Closed`. |
| RUN-05 Local lock | Real subprocess `SIGSTOP`/resume across long waits, second open, abrupt death and close/delete overlap. No successor is admitted under the old lock; failed opener mutates nothing; lock release is last. |
| RUN-06 Cache isolation | Case-distinct labels, Unicode spellings, symlinks, hash/binding mismatch, same-schema/equal-sequence different tenant, changed bucket and reborn namespace. No read, pending publication or cleanup crosses identity. |
| RUN-07 Budget accounting | All pinned, one oversized *request*, 100 cold opens, failed measurements, native allocation failure, post-open growth, simultaneous checkpoint/query and slow close. Reservations never double-spend or release early. |
| RUN-08 Larger than RAM | Actual store above 32 GiB and separately store far above restricted process memory; point reads, joins, recursion, floats, writes, reopen and checkpoint with tiny caches. Same answers/digests as ample-memory oracle, bounded working buffers and no logical-size rejection. |
| RUN-09 Disk/native faults | ENOSPC, short write/read, fsync failure, mmap/map-growth failure, permission errors and corrupted staging bytes at each stage. Existing published state remains readable or explicitly faulted; no partial mount or acknowledged loss. |
| RUN-10 Cancellation/fairness | Cancel queue, AWS request/body, replay, query, scratch insert, result transfer and maintenance. Measure bounded engine safe-point latency and event-loop delay; another ready tenant progresses under a hot tenant. |
| RUN-11 Streaming | Checkpoints larger than RAM, missing/false Content-Length, truncated/stalled bodies, checksum mismatch and interrupted copy. Peak buffers bounded; incomplete files never become mounted published state. |
| RUN-12 Request grammar | Every supported/unsupported method/path/event shape, base64 true/false and malformed padding, invalid UTF-8, too-large pre/post-decoding body, arrays/duplicates/unknown fields, integer/float edge encodings. Exact 400/404/405/413 behavior, zero database calls on invalid input. |
| RUN-13 Auth/IAM | Anonymous and cross-tenant calls fail before open. Body/query/header tenant injection cannot change trusted binding. Actual function uses intended role; denied permissions are typed; credentials refresh without reopening tenants. |
| RUN-14 Invocation lifecycle | AWS/Vercel Node remaining-time exhaustion, post-open failure, admin-job timeout/kill, warm reuse, concurrent requests, cold materialization and abrupt exit. All work is accounted/joined or leaves recoverable unknown state; temporary local files never become a durability claim. |
| RUN-15 Generated migration | Source freeze, named pin, rejected generated plan, bounded native execution, interrupted import/verification, coalesced pending steps and cutover rollback. Old cache/token/command cannot reach new incarnation; application Id128 values remain unchanged; no core log/migration dependency or production schema/migration callback. |

Resource gates assert both logical counters and process-level measurements. Allocator RSS need not return to its exact initial byte count after every call, but live owners/FDs/mappings/reservations must return to baseline and repeated bounded churn must plateau within the declared envelope. A steadily rising per-callback/per-open count fails even if the process has not yet exhausted memory.

## Audit disposition

`SDK-002/004/005/006/007/009/010/011/012/016`, `REP-005/009/010/011/012/014/015/017/019`, `QRY-002/003`, and `PERF-001/002/004/005` require these ownership, bounds and large-data contracts plus their engine/protocol counterparts. Lambda auth/IAM/base64/method/version/credential/deadline issues were unindexed observations in the original audit; RUN-12–14 make them explicit release blockers. No finding is closed by this proposal alone.
