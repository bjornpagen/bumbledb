# 20 — A small authority layer over one real engine

Status: selected log protocol for this replacement packet, reconciled with C4–C8 and current source findings. L08–L11/L14/L17 own its implementation. No source review is a fault-test or backend qualification result.

## 1. The product boundary

The right product is a set-semantic application database with one durable history per tenant. The core owns facts, canonical values, schema laws, compiled execution, checked changes, indexes, snapshots and LMDB. `bumbledb-log` owns command identity, publication certainty, retention, recovery and explicit lifecycle transitions. Its TypeScript package is the application product; native Rust is its implementation. Neither S3 nor TypeScript becomes a second query engine.

There are two authorities, with the same application meaning:

| Placement | Authoritative commit | Local LMDB role | Restart |
|---|---|---|---|
| Embedded/local history | One LMDB transaction containing facts, receipt and control | Database of record | Open committed state; no remote-log emulation |
| Hosted history | Conditional replacement of one never-reused tenant HEAD after immutable dependencies exist | Verified materialization; private candidate until HEAD publication is known | Verify origin and identity; reuse coherent local state or hydrate a protected published root |

One tenant is one write-arbitration domain. Multiple hosted processes can contend through the same HEAD. That does not turn keys, deletes and application laws into a mergeable CRDT. Independent tenants provide parallelism; read-dependent application changes use an exact-state witness. Cross-tenant transactions and implicit tenant splitting are not features of this design.

Preserve the good parts already present: `LocalHistory`, `HostedHistory`, `decide`, `apply`, core `WriterSession`, canonical command framing, typed receipt epochs, immutable object references, and explicit freeze/activate/cancel evidence. Repair the seams between them. Do not replace them with a generic workflow engine, another executor, another value codec, or a database-sized in-memory shadow representation.

## 2. Four invariants that make the rest possible

1. **A materialized stamp describes exactly its committed facts and host records.** A remote HEAD control cannot be copied over older facts. A replay candidate cannot use a predecessor read before acquiring the local writer. Facts, receipts, decision stamp and state stamp are captured together or advanced together.
2. **Certainty is state, not an error-name heuristic.** Before authoritative dispatch, refusal is definite for this invocation. After dispatch, any transport, cancellation, work, decoding or local-settlement failure preserves uncertainty until positive publication evidence or a valid negative proof resolves it. A terminal receipt survives a later local failure.
3. **Recovery, migration and restore produce an admitted final state in private staging.** Intermediate batches need not satisfy final laws. A valid target requiring nonempty facts must not be rejected because an empty temporary database is invalid. Only the fully checked result receives public authority and a ready pathname.
4. **Bounds apply to work performed and resources retained, not only returned array length.** A page of 1,000 keys is not bounded if producing it enumerates the bucket. A 4,096-decision tail is not bounded transport work if every fetch scans all past object epochs. A streaming Rust iterator is not streaming when the native caller first collects all its chunks.

Current source has real repairs and still violates these invariants at specific boundaries; these are the principal repair axes, not independent opportunities to add new abstractions.

## 3. Narrow shared primitives, used transitively

The shared-interface chapter should expose the minimum concrete capabilities needed below. These are roles, not a mandate for additional public APIs or independently implemented wrappers.

### Coherent read and owned writer

A core read frame provides facts, host attachment, keyed host records and bounded host scans from one LMDB snapshot. Local resolve uses that frame once. A published native snapshot derives both its rows and provenance from it. Checkpoint capture already moves in this direction (`checkpointer::capture_into` uses one store snapshot).

An owned writer session reads/checks the current committed materialization while holding the writer fence, prepares canonical changes, invokes the production judge, seals host changes and commits or aborts. The log must be able to apply an already-decoded historical decision under that same owned session without recursively taking the writer lock. The repaired same-writer replay path must be preserved; retirement and every other control transition need the same lifetime boundary. Do not reintroduce a caller-supplied unchecked predecessor.

Use a short per-materialization serialization discipline for transitions, not a process-global lock. Reads remain pinned concurrent snapshots. Network fetches may happen outside the writer, but the fetched plan must be revalidated after acquiring it. If another worker has advanced, rebase or recognize already-applied evidence; never stamp the new facts with the old plan's authority.

### One staged materializer

The core supplies a private, unactivated store builder: bounded canonical row ingestion, set deduplication, schema-aware index construction, whole-final-state judgment, canonical logical digest and final verified commit. It knows nothing about migration operations or HEAD. The log wraps that primitive with identity, provenance, receipt/history projection and publication.

Use it for all of:

- Fresh initialization with canonical seeds and the generated migration chain.
- Schema-changing migration, including resumption and verification of an already-published target.
- Cold hosted hydration from checkpoint plus exact tail.
- Writable restore into a new incarnation and read-only artifact inspection.
- Explicit adoption of an existing admitted core database.

Remove the separate whole-state `MigrationState`/`CollectedState` build pipelines as production prerequisites. A small-state memory fast path is fine when it is a strategy of the same bounded primitive and has the same semantics. Large mappings stream through the compiled core scalar evaluator into staged LMDB sets, which naturally deduplicate convergent outputs. Do not have log code hand-assemble the core ChangeSet header to gain access to this seam. Reuse the core canonical row/change ingestion boundary directly.

Adoption is not `create` over arbitrary existing facts with a blank genesis hash. An adoption operation binds the complete actual initial application/system digest, exact schema, new lineage, approved history baseline and published reconstructible snapshot. Normal create checks empty/uninitialized state under the writer fence, including absence of old host metadata: zero application rows alone does not make a target fresh. Missing files or metadata do not imply permission to adopt.

### One contextual object transport

Keep a small transport around the existing object verbs, but make their obligations enforceable: work/deadline/cancellation context, maximum bytes before allocation, streaming bodies, immutable-create semantics, typed read absence, exact conditional replacement and real bounded listing. `get_verified` must enforce a reference length and incremental digest while receiving, not after an unbounded download.

The transport reports what it observed; it does not decide whether an application command is absent or a migration succeeded. The authority driver owns that interpretation. No blanket `From<BackendError>` may turn a post-dispatch failure into `NotSubmitted`/`not-started`.

Share connection pools and a bounded I/O runtime across tenants with the same transport configuration. Keep authorization scope in each binding; sharing transport must not share tenant authority. The advertised credential-provider chain must really support the chosen AWS deployment's role mechanism, refreshing before expiry without logging secrets. An environment-only callback must not be advertised as a refreshing ECS/EKS/EC2 provider chain; qualify the actual selected provider.

## 4. Publication and materialization

For an unseen command: capture verified HEAD; bring local facts to its decision; acquire/revalidate the local writer predecessor; resolve the retained command ID from coherent state; check current epoch/access and original exact-state condition; prepare/judge/seal once; upload the immutable decision under the captured open object epoch; conditionally replace the exact HEAD version. Only a proved winner commits the private local candidate. A loser aborts and re-evaluates the same immutable command. No callback, regenerated ID, or changed command bytes appears on retry.

Each durable terminal outcome is a decision: committed, no change, precondition failed, or invariant rejected. Head revision advances on every replacement, decision sequence on every terminal command, data revision only when net facts change. GC progress, retirement and freeze cannot reuse a head revision. Coordinates exhaust with a typed refusal; they never wrap or silently remain unchanged.

Historical replay is not current admission. It ignores today's freeze/retirement checks, but independently derives the command's exact-state condition against its exact predecessor, reuses the production judge, and compares canonical terminal evidence. Rejection witness selection must depend on canonical facts and a fixed diagnostic policy—not physical row IDs, LMDB insertion history or an operation's remaining budget. Portable checkpoint import must not change replay bytes. Hash integrity is not proof that a recorded outcome was correctly evaluated.

Materialization has two distinct dimensions: the applied decision and the observed authority control revision. Control-only HEAD changes still need installation. After replay to captured tip T, atomically install the corresponding current access/receipt/activation projection only if local identity and decision still match that capture. If local state is already later, do not rewind it. If a reread HEAD is later, replay first. Deletion is an outer capability refusal, not a genesis fallback.

The proof ladder after uncertain dispatch is: retained matching receipt → decided; valid evidence that the exact conditional version was consumed by another transition and complete retained lookup excludes this attempt → proved loss; otherwise → unknown. A changed version alone is insufficient if receipts were retired or the lookup is not from the promised frontier. `NotRecordedAt(T)` is a point-in-time absence, not proof that an in-flight request cannot later publish. A same-ID retry's pre-dispatch refusal says nothing about an earlier attempt.

## 5. Lifecycle transitions and failure recovery

| Workflow | Durable boundary | Failure before it | Failure after it |
|---|---|---|---|
| Initialize/adopt | Installed local target or created hosted genesis with complete snapshot | Remove only exact owned scratch; no ready authority | Reopen/resolve the same operation's evidence; never reseed |
| Migration source freeze | Exact operation/plan/target-bound freeze | Source remains available | Resume same operation; no automatic thaw because work failed |
| Migration target ready | Complete admitted target, durable Frozen/AwaitingCutover evidence | Staging is disposable | Verify and reuse recorded target; do not rebuild by name alone |
| Activation | Target's one-time activation transition | Target remains frozen | Matching retry reports evidence/current access; never thaws a later freeze |
| Abort | Target cancellation tombstone wins against activation | Source must not be thawed yet | Thaw only matching source freeze after cancellation is proven |
| Restore | Fresh lineage installed from verified backup at exact backed-up tip | Private staging only | Reopen matching operation; old command/witness authority stays invalid |
| Receipt retirement | Checkpoint/control promise advances atomically | Promised receipts remain available | Local pruning may resume; no stamp advance without facts |
| Named-root release | Root deregistration is durable | Root stays protected | Cleanup is retryable and scoped; never resurrect registration |

A target cancellation tombstone exists even before genesis, so a delayed builder cannot publish afterward. If activation won, abort does not automatically reactivate the old source. Source and target are not one cross-database transaction: routing cutover is an explicit application/deployment step. Generated migration history records the exact suffix and source snapshot. Existing repair work already implements much of this shape; qualification must exercise races across the actual source/target storage boundary.

Staged materialization must support the same legal final theories in every path. The latest source replaced create_staged with begin_staged/StagedPopulation, but install_judged_store still uses empty-delta incremental judgment and disarm can shed readiness/cleanup ownership. Finish the private complete-state contract instead of rebuilding the repaired staging entry. A target that can be created but not safely cold-opened, restored or resumed is not supported.

## 6. Retention and recoverability are one design

Keep the checkpoint-plus-tail model. A recovery root protects checkpoint S and exactly decisions (S,T]; roots and GC traverse the same closure. Receipt epochs and object epochs are unrelated. Closure is explicit, bounded metadata; provenance alone does not pin old content.

Ordinary checkpointing moves the recovery base. Retirement may legitimately replace a checkpoint at the **same decision** to change which receipts are promised. Equality of base sequence must not suppress that maintenance. Never require a fake user command to make a quiescent tenant maintainable.

GC closes an object epoch by exact HEAD CAS, captures immutable protected closure, marks it, and sweeps only old unmarked parseable object keys. New references must be inherited from the captured closure or introduced under the current open epoch. Every progress update increments head revision. The selected durable listing coordinate is a canonical `start_after` object key in bytewise order. S3 uses bounded ordered provider pages; adapters translate any opaque provider page state internally. Persist the last fully processed key only after all required deletions through it have completed. Never order provider tokens or store them as this key coordinate.

Replace epoch probing with these two authenticated locators in the new format: `RecoveryRoot.tip_object: Option<ObjectRef>` and `Decision.parent_object: Option<ObjectRef>`. `ObjectRef` carries epoch, Decision kind, full digest and bounded length. The current encoding is 49 bytes (8+1+32+8); length accounting and parser validation must derive from one owner, not duplicate constants. An optional reference adds one tag, so absence is 1 byte and presence is 50, not 51. The walker retains the initial tip locator and admits exactly its stated fetch count, including a budget of one. A checked root distinguishes checkpoint-only from suffix; a checkpoint-only root may have any valid nonzero decision. The root locator is absent exactly when `tip == base`; otherwise its digest is the tip stamp's hash. A decision's digest commits its parent stamp and optional parent locator, never its own locator, so commitments remain acyclic. A present parent locator must name the parent stamp's digest. Walkers stop at the captured root's base **before** requiring another object; a missing parent locator anywhere before that stopping boundary is corruption. Thus a decision immediately after a checkpoint-only base needs no retained parent object, and a later checkpoint does not accidentally retain every ancestor.

Published live decisions are not rehomed: inherited references remain in their original protected epoch. A losing unpublished candidate may be staged again under the new open epoch after revalidating the exact captured parent; the object's own epoch is not in its frame. Recovery, witness checking, backup and GC all use the same verified locator walk. Backup's existing destination manifest records the relocated, ordered decision ObjectRefs: artifact replay reads those refs and verifies the original parent-stamp/hash chain without rewriting historical decision bytes or chasing their source-storage locators. Writable restore starts a new genesis after that replay. Remove the lookup-only epoch-floor search from the new format rather than retaining an alternate slow path. Publish limits for tail count, tail bytes, root count, manifest bytes and work—not an undocumented tenant-size ceiling.

Local named restore points remain self-contained directories and a small transactional registry. That simple specialization is preferable to a second local object collector. Registry read-modify-write happens under one writer session; directories become durable before registration, deregistration before deletion. Read errors are not an empty registry. Root IDs are never reused; an idempotent retry must verify existing evidence instead of overwriting or guessing.

## 7. Tenant/runtime ownership

The hierarchy is runtime → directory owner → managed database → history/snapshot/operation capabilities. A cache borrow is a revocable application capability; an operation lease pins actual work; neither is a TTL. A directory fence releases only after LMDB/session cleanup. Every acquisition exit consumes its opening ticket or hands its resource to teardown, including metadata-check errors and cancellation.

Closing revokes admission atomically with registry state. A waiting acquire cannot pass an old `closing == false` observation and create a successor after close begins. Closing slots remain accounted until cleanup proves release; removing a map entry is not resource reclamation. After the last operation lease drains, closing must progress automatically. `Incomplete` reports still-owned resources and how cleanup continues; it must not mean a forgotten slot that only a second manual close can rescue.

Use finite default tail policy on cache opens, just as on direct opens. Shared runtime memory, queueing, snapshots, live query images, mappings, borrowed handles and transport clients need an aggregate budget. A `maxOpen` counter alone does not bound per-tenant workers, retained unknown maps or in-flight teardown.

Inspection reports observed truth with provenance: local applied tip, last verified authority control, observed remote tail/GC or an explicit unavailable state. Do not turn a failed HEAD read into zero tail/idle GC. Unknown health entries distinguish unresolved, closed-but-unrecorded and expired-unprovable cases; diagnostic bookkeeping never becomes the authority.

## 8. AWS/S3 performance and release gates

The current successful hosted submit normally performs a HEAD GET, decision PUT and conditional HEAD PUT in sequence, in addition to local work and any catch-up. A resident process can reuse connections and reduce repeated cold work; it cannot erase remote publication latency. Holding the local private writer across that bounded attempt is an explicit tradeoff. High contention wastes complete candidate work and object uploads. No-op and rejected commands still incur durable decision cost.

Measure the actual package, not only the Rust engine: warm and cold Lambda/Graviton, resident Node, target x86 deployment, many tenants and 1/2/4 same-tenant contenders. Record command p50/p95/p99, queue delay, judgment time, transport stages, GET/PUT/LIST/delete counts, bytes, hydrated rows, tail depth, directory-lock hold time, RSS and temporary disk. Include >RAM recovery/migration/restore and repeated open/close/evict cycles. Request amplification and tail recovery cost are first-class outputs.

Qualify actual S3 semantics with raw fault evidence: missing key versus AccessDenied/NoSuchBucket; wrong region/redirect; typed 412 versus 409 conflict; response lost after a committed conditional PUT; reset before response; 5xx; missing ETag; stale exact version; concurrent same-key identical-body operations; immutable conflict; provider pagination over more than one page; role credential expiry. An emulator or `MemStore` test does not establish those facts. Adjudicate the inherited duplicate-create/conditional-ambiguity expectations against the actual adapter retry semantics. Replace smoke-only assertions with real-transport semantic tests of the evidence-preserving contract. Keep independent expected inputs beside their consuming tests, not in new fixture/exhaust/implementation-report folders.

The implementation can retain its small architecture. The work is to make one engine's semantics, one authority's evidence, one materializer's boundedness and one runtime's ownership hold end to end. That is the route from an impressive prototype to a database applications can safely depend on.
