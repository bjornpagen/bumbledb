# 20 — One tenant, one publication authority

Status: proposed breaking 1.0 design, not an implemented or verified protocol. This document supersedes the audit's conservative repair roadmap where that roadmap preserves the old log. The dated counterexamples remain evidence, not compatibility requirements.

The choice is a single, never-reused tenant `HEAD`, changed by compare-and-swap, over immutable decision records and a bounded replay tail. LMDB remains the local database. A checkpoint replaces an older recovery base without changing the application state. There is no per-braid publication, distributed writer lease, shared ID-range allocator, universal commit DAG, or custom remote page engine.

All machinery in this chapter belongs to **`bumbledb-log`**, whose 1.0 public product is the **TypeScript package and its TypeScript CLI/integrations**. Its Rust state machine is internal native implementation, not a second public Rust API. The main `bumbledb` library remains facts, laws, queries, checked admission, and LMDB, with public Rust and TypeScript bindings. The successor has no public C core or log API; the old C surface is slated for deletion during implementation. An application using only the core does not acquire a remote history protocol, a command identity system, or backup/migration orchestration.

## The alternatives, and the actual price of the choice

| Alternative | What it makes attractive | Why it is not 1.0 |
|---|---|---|
| Repair per-braid create-only logs | Independent arbitration when the schema really decomposes | Retired names, vector recovery floors, causal dependencies, split outcomes, and allocation authority remain separate obligations; braids do not partition rows or guarantee useful application parallelism |
| Make a resident writer the only authority | Cheap serialized warm writes and simpler contention | Safe takeover still needs an enforcing resource; S3 is no longer the sole publication authority, and embedded/direct modes acquire different failure semantics |
| Publish a persistent whole-state Merkle tree on every write | Direct immutable snapshots, potentially fine-grained reuse | Introduces another physical storage engine, page packing, remote index updates, and multiple object operations per small command; unnecessary beside LMDB |
| **Tenant `HEAD` + immutable decision chain + LMDB checkpoint** | One linearization point, tenant-wide atomicity, straightforward retry and causality | One tenant is one write arbitration domain; direct S3 latency is real, contention wastes candidate work, and bounded replay requires checkpoint progress |

A resident deployment is a placement/connection-reuse optimization of the selected protocol, not a different authority. **One authority is not one caller:** many hosted writers can contend through the same head; concurrent core/local-history callers serialize through LMDB's writer transaction. Arbitrary merge/union is not free because keys, deletes, and capacity laws are not generally closed under it. No additional CRDT/semilattice write protocol is introduced in 1.0. Cross-tenant atomicity is out of scope. A hot tenant is not silently split into multiple authorities.

The intended hosted workload is many per-student/per-user application databases, with their actual sizes, burst patterns and cold-start frequency measured—not a claim that every tenant is tiny. A warm successful command normally has two sequential remote stages: immutable-object upload and conditional head replacement, in addition to judgment, queueing and local settlement. The private LMDB writer is occupied during the bounded publication attempt. Durable no-op/rejection decisions also pay that path. Before fixing public performance claims or policy defaults, measure the actual applications on Apple Silicon locally, ARM Graviton/AWS, and the qualified x86 Vercel Node runtime: warm/cold latency, 1/2/4 contenders, no-op/rejection-heavy workloads, and object requests/bytes per terminal result. Resident placement cannot erase S3 round trips; a throughput or p99 requirement not met by this protocol must be reported, not hidden behind an embedded-engine benchmark.

## Identity and coordinates

These are distinct types in `bumbledb-log`, not interchangeable integers. The main engine sees ordinary canonical scalar values and an opaque transaction attachment, not these protocol types.

| Type | Meaning and lifetime |
|---|---|
| `DatabaseId` | Logical database identity, independent of a display name or object prefix |
| `IncarnationId` | One non-forking history lineage; changes for a restored branch, schema-transforming migration, or rebirth |
| `SchemaId` | Hash of the canonical admitted theory; compatibility, not tenant identity |
| `HeadRevision` | Monotone `u64`, incremented on **every** successful `HEAD` replacement, including maintenance |
| `DecisionStamp { seq, hash }` | Monotone sequence and domain-separated digest of each durable terminal command decision |
| `StateStamp { incarnation, data_revision }` | Changes only when the net application fact set changes; used by exact-state read witnesses |
| `ReceiptEpoch` | Command-admission/deduplication namespace, unrelated to object GC epochs |
| `CommandId { receipt_epoch, request_id }` | Caller-stable request identity; `request_id` is 128 opaque bits, bound to one canonical command digest |
| Application `EntityId` | Optional application-owned nominal `Bytes<16>` value; no command authority, receipt lifetime, or history coordinate is encoded in it |
| `ObjectEpoch` | Namespace generation for newly introduced remote objects; advanced only by the GC barrier protocol |
| `ObjectRef { epoch, kind, digest, length }` | Typed, verified immutable object identity; the digest alone is not its storage key |

No coordinate wraps. Exhaustion is a typed administrative refusal, not zero, saturation, or ID reuse. An identical command ID with different bytes produces `CommandIdentityConflict`, never a second meaning. Application-supplied request IDs need uniqueness; collisions are detected by digest binding rather than being treated as successful idempotency.

An entity identifier and `CommandId.request_id` may both contain 128 bits, but are distinct roles and SDK types. An entity value is ordinary persistent application data; a command ID binds one named request within the current incarnation/receipt namespace. Neither is accepted in place of the other merely because the bytes have the same width. The core continues to allow schemas with other ordinary scalar key domains; it does not require an entity-ID service or a new scalar primitive.

Authoritative content digests remain full **256-bit/32-byte cryptographic values**: schema identity, command binding, decision/parent hashes, object addresses, snapshot application/system checks, and migration plan/history commitments. The current BLAKE3 baseline is not silently replaced/truncated because a short CPU-accelerated table hash benchmarks well. A non-authoritative lookup fingerprint can be narrower only when full canonical equality independently resolves collisions; receipt identity and GC reachability are not that kind of lookup. Cryptographic content hashes detect mismatched bytes under their collision-resistance assumption; they are not credentials or message authentication. The 128-bit application/request IDs and opaque S3 version token serve different roles.

The local cache additionally binds the **configured origin**—backend/account/bucket/prefix as appropriate—and the complete database identity tuple. An origin move is an explicit log operation/runbook, not acceptance of a same-schema/equal-counter cache.

## Minimal durable representation

This is the **hosted** logical schema. `HistoryAuthority` is `LocalHistory { committed_lmdb }` or `HostedHistory { head, local_cache }`, not one structure with optional remote fields everywhere. Local specialization is specified below; it does not emulate a remote replay log merely to reopen LMDB. Exact byte framing and versioned golden files must be frozen before implementation qualification.

```text
Head {
  format, database_id, incarnation_id, schema_id,
  revision,
  access: Active | Frozen { reason, operation_id } | Deleted { operation_id },
  decision: DecisionStamp,
  state: StateStamp,
  recovery: { checkpoint: SnapshotRef, tip: DecisionRef, tail_count, tail_bytes },
  receipts: { open_epoch, retired_through },
  migration_history: MigrationHistoryRef,
  activation: NotActivated | Activated {
    operation_id, target_genesis, cause: Create | Restore | Migration { plan_set_digest }
  },
  roots: bounded list<NamedRoot>,
  object_epoch,
  gc: Idle | Marking { barrier } | Sweeping { barrier, marks, cursor }
}

Decision {
  format, database_id, incarnation_id, schema_id,
  seq, parent: DecisionStamp,
  command_id, command_digest,
  before_state, after_state,
  canonical_command,
  outcome_evidence
}

LogSnapshotCertificate {
  database_id, incarnation_id, schema_id,
  decision: DecisionStamp, state: StateStamp,
  control_at_capture: { receipt_policy, activation, access_mode },
  migration_history: MigrationHistoryRef,
  canonical_application_digest, canonical_system_digest,
  streamed_snapshot_manifest
}
```

Genesis is an explicit **admitted initial snapshot**, empty for ordinary blank creation but potentially nonempty for a validated import, restore, or migration. Its sequence-zero decision sentinel hashes a versioned genesis record binding identity/schema, canonical initial application/system digests, and migration/source provenance; it is not one universal zero hash that can authenticate unrelated initial states. Avoid self-reference: the genesis preimage excludes its own stamp and the snapshot-manifest hash that will carry that stamp. Compute the initial logical digests/provenance first, then genesis hash, then the certificate/manifest binding that hash. Migration-history entries bind target identity and logical target digest, not a circular copy of the target genesis hash. A new incarnation begins with `StateStamp(incarnation,0)`, open receipt epoch 1, retired-through 0, and no executable old-incarnation command receipts. The initial application facts may already be nonempty; data revision counts subsequent changes, not imported row count. Old receipts can remain explicit archival recovery evidence. Parent hashes authenticate sequence continuity; they do **not** require retaining every parent object forever. A recovery root means “this checkpoint at S plus exactly the decisions `(S,T]`,” not an unrestricted traversal back to genesis. The retention walker uses the same stopping boundary as recovery.

The head is bounded metadata, not the receipt database. The named-root count defaults to 64 and has an explicit configured cap and encoded-byte cap. A full list returns `RootCapacityExceeded`; it never discards another root. These are metadata/resource policy bounds, not database-size limits. Large snapshot manifests are streamed/indexed in objects, not embedded in `HEAD`.

The receipt table is a `bumbledb-log` system table materialized transactionally beside application facts. A checkpoint contains it; decisions after the checkpoint are its bounded delta. Consequently a small command normally uploads one decision object and conditionally replaces `HEAD`, not a new remote receipt tree or database image.

The verified migration-history prefix is fixed at this incarnation's genesis and copied into its checkpoint/system records. Ordinary commands and maintenance cannot replace it. A migration or explicit baseline/adoption establishes a new incarnation with a new verified prefix, rather than rewriting past `Applied` entries in place. Hosted head/certificate refs and local-history metadata bind the same prefix, whose terminal schema must agree with `SchemaId`. The TypeScript runner in chapters 22/33 compares that authoritative prefix with the repository manifest before doing work.

`canonical_system_digest` hashes an explicit canonical **logical projection**: retained receipt/result rows and migration/history evidence records, ordered by their versioned schema. It does not hash arbitrary host-metadata bytes. Exclude the current head/revision, authority/access state, root/GC state, activation marker, current receipt-admission controls, core attachment stamps, certificate fields, and the digest/hash being defined. These bounded control values are separately bound by the exact head and `control_at_capture`/outer certificate hash. Receipt rows can contain already known decision hashes; a genesis migration record contains target identity and application digest, not its own genesis/system digest. Stream/hash logical rows first, compute the genesis where applicable, then assemble the certificate and its own hash. This projection is shared by export/import/replay checks so no implementation accidentally hashes a self-reference or compares metadata bytes as logical state.

Activation is a one-time bounded control record, not a mutable migration journal. `activateMigration` changes `NotActivated` to the matching `Activated` marker atomically with access becoming Active; later commands, freeze, GC and receipt retirement preserve it. A matching retry returns the recorded activation evidence and current access mode without mutation: it must not thaw a later Frozen state or revive a Deleted authority. An explicit ordinary blank creation can publish its corresponding initial activation marker with genesis. A named snapshot/backup captures control provenance; a cache hydrated from an older checkpoint installs the **captured target head's** current control projection after validating the checkpoint/tail, rather than restoring obsolete authority from the checkpoint. A restored lineage never acquires live authority from an old captured activation marker.

**Exactly one authoritative fact payload:** the decision stores the concrete canonical command, not a second delta with duplicated facts. All application identifiers already have their final bytes. `outcome_evidence` contains the terminal tag, changed summary, observed precondition state, or bounded rejection evidence; a bounded declared result is copied into the public receipt. Initial prepare and replay use the same checked command decoder and core admission, never a host callback or sequence-dependent value resolver. Replay verifies the command digest and checks the recorded outcome against evaluation at the exact predecessor. Diagnostic selection/order is codec-defined, not dependent on available RAM or a caller's timeout; insufficient work budget returns progress/failure rather than different decision evidence. After checkpointing, the receipt table retains the bounded ID/digest/outcome/result needed for lookup, not every old command body. Retired receipts can subsequently be removed under the explicit policy.

Historical replay is **not a call to current command submission**. It verifies identity/sequence/parent hash, canonical command meaning, the recorded application precondition/laws/outcome, and before/after state at the decision's exact predecessor. It must not reject an already published decision because today's receipt epoch is closed/retired or today's authority is Frozen/Deleted: those maintenance transitions are not all in the decision chain. Apply historical application effects, rebuild only the receipt rows required by the selected target's retention policy, and install that captured target's current control projection for **new** admission. This separates the shared decoder/judge from current authority/receipt-admission guards without adding a second semantic evaluator.

This historical verification rule grants no public access to a deleted database. The outer open/read/admin capability enforces the captured authority's current access policy; a historical evaluator used by an authorized restore/check cannot reactivate that authority.

Core support is narrow: a checked prepared LMDB write, an owned read snapshot, and opaque attachment/system-record primitives captured/committed in the same LMDB transaction. Core has no awareness of a receipt, head, backup, or migration. A wrapper cannot manufacture a certificate by separately reading generation, facts, and attachment after the fact.

## Commands and durable results

A command is **owned canonical data**, sealed before asynchronous work. The digest covers the identity scope, declared schema, precondition, concrete operations, and bounded declared result. Copy mutable host bytes at this boundary. No ordinary command callback is rerun, retained inside a lock, or replayed on another host. Repository migration assets are likewise canonical data: the TypeScript schema library generates a checked plan AST that the log's native executor applies in an explicit offline workflow, as specified in chapters 22 and 33. There is no arbitrary TypeScript transformation callback in either replay path.

Use the core's exact per-command normalization: additions A and removals D become `(A, D \ A)`, producing `(S \ D) ∪ A`. Repeated effects deduplicate; spelling the same fact in both sets means addition wins **within that one command**, independent of iteration order. This is not an add-wins merge rule between concurrent commands; their authoritative order is still the tenant head/LMDB transaction order.

The 1.0 conditional form is deliberately narrow:

```text
Condition = Unconditional | ExactState(StateStamp)
TerminalOutcome = Committed { changed, result }
                | NoChange { result }
                | PreconditionFailed { observed_state }
                | InvariantRejected { complete_bounded_evidence }
```

All four terminal outcomes are published decisions and have stable receipts. A durable rejection costs a head CAS; this is the price of “the same named request has the same outcome.” Invalid grammar, unsupported schema, closed request namespace, overload, cancellation before dispatch, or inability to produce complete bounded rejection evidence are **nonterminal** refusals. They must not masquerade as a recorded terminal rejection. `NotSubmitted` is about this invocation's dispatch, not proof that a previous/concurrent invocation with the same command ID never published.

`ExactState` compares the current `StateStamp` while preparing against that exact head. Maintenance, no-ops, and rejections do not change it. Any intervening fact change changes it, even if later changes restore identical values: this intentionally detects ABA application histories. On CAS loss, the same immutable command is re-evaluated against the winner; a changed witness then produces a durable `PreconditionFailed` decision. This is whole-tenant optimistic serializability for read-dependent commands, not automatic inference of arbitrary application intent.

Blind set changes preserve set meaning but do not promise that an unencoded read/modify/write business decision is serializable. Two blind deletes of an old counter fact and inserts of the same replacement can still be one net effect. Use an exact-state witness for that intent.

Application entity IDs are chosen **once, before command sealing**. The TypeScript convenience helper may generate a cryptographically random 128-bit value; its canonical application encoding is 32 lower-case hexadecimal characters backed by 16 native bytes. It does not contact the database or claim protocol-proven allocation uniqueness. Applications own the uniqueness policy and use declared laws/explicit handling where identity collisions matter. A command retry, including retry on another host after response loss, retains the same entity IDs, command ID, and canonical bytes. Regenerating an entity ID under the same command ID changes the digest and refuses with `CommandIdentityConflict`.

An application may know an entity ID before any write; knowing it is not evidence that an entity exists or a command committed. There is no log allocation API, symbolic placeholder, generated-ID receipt field, ordinal, reserved range, or special allocation-only decision. A successful no-change command still has an ordinary durable receipt because its named result must remain stable. Restore and migration preserve application ID bytes unless explicitly instructed otherwise; changing incarnation changes command/witness authority, not entity identity.

## Publication state machine

There is one internal production Rust state machine behind the TypeScript package/CLI and its private native binding. Host effects are I/O and scheduling; TypeScript does not independently implement publication or recovery transitions. Internal Rust test interfaces are not a promised public Rust log SDK, and there is no public C log surface in 1.0.

```text
Ready(published_local_snapshot)
  -> Sealed(owned_command)
  -> Preparing(exact_head, private_write)
  -> Staged(exact_head, decision_ref, proposed_head, attempt_id)
  -> InFlight(staged_attempt)
  -> Published(receipt, local_apply_needed)
  -> Ready(updated_published_snapshot)

InFlight -> Unknown(owned_attempt)
Unknown  -> Published | RetrySameCommand | OutcomeUnknown
Any pre-dispatch state -> NotSubmitted(reason)
Any state -> Closing -> Closed
```

`Unknown` is not another editable pending slot. A new operation cannot overwrite its capsule. The implementation can resolve/abort the private local transaction and then accept another command, but it must preserve enough identity to resolve the original request. Persisting that capsule locally is useful crash recovery, not a prerequisite for remote durability: a caller retaining the command ID and digest can resume on a new host.

The candidate uses a private LMDB write transaction prepared against one published state. Ordinary readers retain committed LMDB read transactions and cannot observe candidate mutations or mutable candidate image caches. The bounded network attempt may hold the private writer transaction on its owning worker; there is no whole-database clone. A network timeout can abort it while preserving the owned attempt. Published remote state can subsequently be applied from its immutable decision.

The exact steps are:

1. Admit resource budgets and lifecycle capability; seal the command once.
2. Read/verify `HEAD`, database identity, active mode, and receipt policy; bring local published state to that **finite captured tip**. A warm writer may start from its already verified exact head and committed materialization, paying for a refresh only on movement; stale candidates are safe because their CAS cannot bypass a newer head/epoch.
3. Look up the command ID in the checkpointed receipt table plus tail. If found, return the recorded outcome after checking the digest; do not execute again.
4. Check the precondition and admit the concrete canonical delta. Judge the private application write and form the complete decision bytes/digest. While retaining the same exclusive writer session, seal opaque host-record changes containing receipt and attachment into the same transaction. No application facts change after judgment. For a rejected application delta, abort it and prepare an empty application delta in that same session, then seal the rejection receipt. No-change decisions likewise still seal their receipt. A host-record allocation/storage/sealing failure prevents remote CAS.
5. Verify every referenced dependency is either inherited from this exact parent recovery/root closure or newly staged in `head.object_epoch`. Upload and verify the immutable decision and any large command objects allowed by the grammar. Do not expose a success yet. The decision contains its outcome but not a circular copy of its own digest; that digest is then carried by the sealed receipt/head attachment.
6. Form `HEAD'` from **that exact `HEAD`**: increment revision and decision sequence; update state revision iff facts change; advance tip/count/bytes; retain all unrelated roots and GC fields.
7. Conditionally replace `HEAD` using its exact opaque version. **The successful atomic replacement is the hosted publication linearization point.** No cross-object atomic transaction is assumed: immutable dependencies were completed first.
8. Return the durable receipt once publication is known. Commit/apply the local LMDB transaction and attachment together before exposing the new local snapshot. If local commit fails after remote publication, return/retain `Published` plus local-cache-unavailable health; poison/rebuild the cache, never downgrade the command to rejected or automatically execute a new request.

The prepared writer and any in-memory indexes must respect the same isolation. Merely hiding `Db::write` does not solve dirty reads. Cancellation after step 7 cannot unpublish a decision. Close revokes admission immediately, cancels bounded work, and settles or records uncertainty for dispatched attempts.

## Ambiguity: what a GET does and does not prove

| Observation | Correct conclusion |
|---|---|
| Successful qualified conditional replacement response | This proposed head published |
| Current head is the exact proposed revision/body | This attempt published; the unique revision/decision identity is evidence, unlike equal allocator counter bytes |
| Receipt index/tail contains this ID and digest | Return that terminal outcome even if many later heads or checkpoints have appeared |
| Fresh head has advanced and retained receipt lookup is complete but absent | This old exact-version attempt can no longer win; a new attempt may proceed only if its epoch still admits new commands |
| Head unchanged, receipt absent, original HTTP request may still be running | Not proof of nonpublication; retry the identical conditional request or retain uncertainty |
| Verification GET fails or is incomplete | `OutcomeUnknown`, not rejection |
| ID's epoch was retired | `ReceiptExpiredUnknown`; never re-execute that ID |

`resolve()` may return `NotRecordedAt { decision }` for an active epoch, explicitly a point-in-time observation, not “will never commit.” A bounded deadline returns an unresolved token. If the caller needs to stop a still-possible exact-version attempt, a no-op revision-advancing head CAS fences that attempt; race it against the original and then resolve. This is a documented administrative/recovery transition, not a new writer lease.

The generic S3 adapter preserves raw `Committed`, definite `PreconditionFailed`, and `Indeterminate` distinctions according to its qualified service contract. It must not reinterpret all equal bytes as exclusive ownership. An immutable-object upload may use verified equality as content evidence; a head publication needs unique transition/receipt evidence. A transport error after dispatch is not assumed to mean “nothing happened.”

## Receipt rotation and bounded retention

Only one receipt epoch admits unseen commands. Rotating `open_epoch` is a head maintenance CAS. Older, unretired epochs are closed: known IDs return their receipt, absent IDs return `CommandEpochClosed`. A delayed writer using the previous head cannot publish after this barrier.

`retired_through` is a monotone prefix. Retirement is explicit operator/application policy and advances atomically with a checkpoint that no longer promises the retired receipt rows. It is not a wall-clock timeout hidden in a client. IDs at or below the frontier always refuse execution, whether their receipt still happens to exist in an older named snapshot or backup.

Within a retained epoch, success/rejection/no-op identity survives checkpointing and response loss. After retirement, precise outcome retrieval is no longer promised, but the protocol still prevents duplicate execution under the expired ID. An application must not turn `ReceiptExpiredUnknown` into an automatic new-ID retry. Essential long-lived business idempotency belongs in application facts whose lifetime matches the business process.

Receipt bytes, results, and diagnostic evidence are budgeted before publication. Epoch rotation alone does not reclaim rows; retirement plus a new checkpoint and GC does. If receipt storage or replay budgets exhaust before maintenance completes, refuse new work predictably; do not quietly forget deduplication.

## Read and refresh contract

A public read capability exposes only a complete published local snapshot and its `StateStamp`/`DecisionStamp`, never the raw write-capable engine. `Cached` is explicitly a cached snapshot. `Latest` captures one head/tip and catches up to it under a work/deadline budget; it does not chase a moving tip forever. `AtLeast(DecisionStamp)` is incarnation-bound and returns progress, timeout, wrong-lineage, or corruption explicitly. If verifying a supplied historical hash requires history that was not retained, return `WitnessUnavailable`; do not claim that sequence inequality proves the token's exact hash was an ancestor. A future sequence-only floor API would be a separately named weaker token contract.

Hosted linearizable-read selection means reading a qualified current `HEAD` during the call and reading the exact captured decision state. It does not mean “the latest state at response completion.” If retention removes required remote data during hydration, hold the captured root as described in chapter 21 or return a typed snapshot-expired/unavailable result; never interpret a missing referenced object as an empty database or the log tip.

State witnesses are log-layer values copied from these snapshots. A witness from another incarnation is refused before any candidate effects. Cross-tenant application workflows need explicit application coordination; neither a shared SDK pool nor identical schemas creates a transaction.

## Local and S3 modes without a second filesystem protocol

- **Core embedded:** normal `bumbledb` transactions, local durable LMDB commits, core generation/snapshot vocabulary. No log receipt claim is implied.
- **Local history:** optional `bumbledb-log` wrapper stores current facts, retained decisions/receipts, and its identity/stamp attachment in the same local LMDB transaction. Its linearization point is durable LMDB commit. It uses the same command evaluator/outcome grammar but does not emulate an object store with expiring token files. LMDB already contains complete authoritative state: no remote-tail envelope or full checkpoint is required merely to reopen it. Receipt retirement is an atomic local metadata/row change; independent named snapshots preserve their own older evidence.
- **S3 history:** conditional S3 head replacement is authority; local LMDB is a recoverable materialization. Local transaction success before remote publication is never ordinary success.

A local-history directory has one owning process lifetime, enforced by a supported OS lock acquired before cleanup and held through native close. In-process readers can be concurrent. A paused holder remains owner; no timeout enables another process to steal ordinary POSIX mutation rights. Remote writers can use separate local directories and arbitrate through the same S3 head. An object-store filesystem emulator is test support, not an alternate production database engine. The same canonical command/outcome evaluator does not imply identical physical crash mechanisms; the local LMDB commit and hosted S3 CAS are deliberately distinct publication variants.

## Safety argument and limits

1. **Single order:** the head is never deleted/recreated; exact conditional replacement and monotonically unique revisions prevent ABA. At most one successor publishes for an observed head.
2. **Valid state:** a decision is prepared against that parent's complete published state; a losing candidate is never served. All terminal decisions, including rejections, use the same ordered publication.
3. **Atomic intent:** one decision contains all tenant relations changed by a command. There is no successful-prefix split API to lose.
4. **At-most-once named execution:** every successful head successor adds the ID binding to the receipt state; contenders check/recheck that state. Retirement invalidates admission permanently rather than making absence reusable.
5. **Recoverability:** all dependencies precede publication; checkpoint replacement preserves an ancestor boundary and bounded successor chain; chapter 21 prevents deletion of present/future valid dependencies.
6. **Local isolation:** private candidate writes are not committed read state; local apply failure does not change authoritative history.

Premises are explicit: authenticated honest protocol writers; qualified linearizable conditional operations; durable LMDB/filesystem behavior on supported platforms; collision resistance of the selected content hash; correct canonical parser/admission; bounded resources or typed failure. Hashes detect wrong content, not a malicious authorized writer inventing a schema-valid but unwanted command. S3 object access credentials are authority; application authentication remains a host responsibility.

## Protocol release tests — all required before 1.0

These are required new tests, not claims that they already exist or pass. Feed the same bounded schedules to the production Rust machine, SDK surface, and an independently implemented tiny history model. The model must not call production transition helpers.

| Gate | Required schedule and assertion |
|---|---|
| `PROTO-01` | Two to four contenders on one parent; exactly one successor per version and all returned terminal receipts have one history position |
| `PROTO-02` | Reuse one ID/digest across hosts, retries, no-op and rejection; exactly one stable outcome, not merely one final fact set |
| `PROTO-03` | Same ID with different bytes before/after checkpoint; identity conflict without a second execution |
| `PROTO-04` | Drop response before dispatch, after object upload, during CAS, after CAS, during receipt GET/body; preserve certainty correctly at each boundary |
| `PROTO-05` | Unknown CAS, later writers, checkpoint, receipt retirement; resolve precisely while retained and refuse expired re-execution afterward |
| `PROTO-06` | Pause the original CAS indefinitely; GET shows old head/absence; no false definitive nonpublication; fence revision or competing submission resolves safely |
| `PROTO-07` | Candidate prepared/applied privately, ordinary reads, candidate loses or rejects; no candidate fact is ever observed |
| `PROTO-08` | Two witnessed decrements from one state; one changes state, the other records precondition failure; blind variant documents different meaning |
| `PROTO-09` | Maintenance/no-op/rejection between witness and submission does not move StateStamp; change-and-restore does move it |
| `PROTO-10` | Multiple relation changes reject atomically or publish atomically; no partial-prefix receipts exist |
| `PROTO-11` | Application-owned 128-bit entity values survive CAS losses, response loss, cross-host retries, replay and restore unchanged; same command ID with regenerated entity bytes conflicts; entity IDs cannot substitute for command/witness capabilities; no allocator/placeholders/generated-ID result remains |
| `PROTO-12` | CAS wins, local commit/fsync fails, process dies; fresh materializer returns original receipt and facts without re-executing command |
| `PROTO-13` | Continuous append while refresh runs; captured-tip refresh terminates or returns budgeted progress rather than chasing infinity |
| `PROTO-14` | Close/revoke while sealing, preparing, uploading, in-flight, and settling; no new post-close dispatch and no discarded uncertainty |
| `PROTO-15` | Corrupt parent/hash/length/schema/sequence/wrong incarnation; fail closed before rows become readable |
| `PROTO-16` | Rotation racing unseen old-epoch command, known retry and retirement; exact new-admission boundary and monotone retired prefix; hydrate a published old-epoch tail after rotation/freeze and prove its historical effects are replayed despite current admission refusing that epoch |
| `PROTO-17` | Core embedded and LocalHistory are separately tested; local history atomically persists facts/receipt/head attachment across process kill |
| `PROTO-18` | Tiny work/memory/tail/receipt budgets at every loop; charge before growth, bounded cancellation, no unbounded retry or abandoned worker; inject `MAP_FULL`/allocation/I/O failure while sealing host records after decision hashing and prove no CAS was dispatched |
| `PROTO-19` | Empty creation and nonempty restore/migration genesis; canonical acyclic logical-digest projection excludes self/control fields while outer certificates bind them; counters start at zero, fresh receipt epoch starts empty, old-scoped commands cannot acquire current admission, and hydration installs captured-head controls instead of old checkpoint authority |
| `PROTO-20` | Qualify application write latency/throughput on the supported hosted targets with 1/2/4 contenders and 0/50/90% no-op/rejection mixes; record writer occupancy, p50/p99, requests/bytes and retry waste against explicit workload targets; no embedded-only performance substitution |

Finite exhaustive schedules should cover at least two writers, two command IDs, two receipt epochs, a checkpoint, response loss, one reader, and GC barriers. Randomized longer histories complement this bound. Preserve minimized counterexamples and every client-visible read/outcome, not only final convergence.

## Audit disposition at this layer

| Audit IDs | Replacement/removal | Closure gate and cost |
|---|---|---|
| REP-001/002/006 | No vacant slot reuse, vector floor, or writer-ID fence; one head order | PROTO-01/11 plus GC gates; tenant-wide serialization |
| REP-004; ENG-004/007 | Delete database allocation/reservation; application-owned 128-bit values fixed before sealing | PROTO-04/11; ordinary application uniqueness policy, no formal allocation-issuance claim |
| REP-008/020 | Exact-parent single chain; no independent split commit | PROTO-01/10; cross-relation conflicts share the tenant head |
| REP-015; SDK-010 | Captured finite targets and execution budgets | PROTO-13/18; callers handle progress/overload |
| REP-016; SDK-008/014 | Published read capability plus private candidate transaction | PROTO-07/12 and compile-fail tests; explicit ownership/worker discipline |
| SDK-001/002/003/009/015 | Owned sealed command, explicit Unknown, lifecycle-aware machine, no callback replay | PROTO-02/04/14; input copy and retained bounded attempt evidence |
| ARCH-001/002/003/005/006 | Exact-state intent, tenant total order, named receipts, one Rust machine | PROTO-02/08/09/10; no automatic cross-tenant transaction or hot-tenant sharding |
| ASS-001 | Braid proof is no longer a publication premise; any remaining decomposition is an engine optimization with its own proof | Engine semantic gates; do not claim the retired theorem verifies this protocol |
| ASS-002 | Independent client-visible history model and deterministic fault schedule | All PROTO/GC/REC gates; substantial test work is required, not waived by a small implementation |

Storage, retention, cache ownership, backups, and migration closure are specified in chapters 21 and 22. The intended simplification is fewer independent authorities, not fewer adversarial tests.
