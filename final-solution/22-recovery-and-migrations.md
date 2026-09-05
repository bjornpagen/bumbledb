# 22 — Recovery, backups, erasure, and migration belong to the log

Status: proposed 1.0 design and release gates, not implemented recovery tooling. This chapter specifies the TypeScript log package's recovery/migration runner plus operator procedures, **not** a fleet control plane, scheduler, migration service, or consensus system. The Rust protocol/build machinery is internal; no public Rust or C log API/header is required for 1.0.

The boundary is explicit: `bumbledb` provides an owned consistent snapshot, checked construction/admission, queries, and durable LMDB storage. **The TypeScript `bumbledb-log` package owns schema-generated migration assets, explicit execution, export/import orchestration, retained history, backups, restore provenance, incarnation changes, schema transformations, and cutover procedures.** A generic core snapshot/copy/builder is not branded as a backup or a migration API. Chapter 33 supplies the Drizzle/Expo-style repository ergonomics and Next.js/Alchemy setup; this chapter supplies their durable meaning. Users describe schemas and resolve ambiguous intent; they do not handwrite migration programs.

## Recovery has a small number of states

All public TypeScript core/log work uses chapter 35's Effect/Scope contract. Recovery/admin orchestration is interruptible; only native ownership handshakes are masked. Preserve the original operation identity through interrupted freeze, build, publication, activation and abort. Scope cleanup releases resources, never implicitly thaws or activates history. An interrupted fiber's Cause is not proof an authority transition failed. Read-only status/verification and named durable outcomes retain the distinctions below.

```text
Closed
  -> OwnedDirectory
  -> IdentifiedOrigin
  -> ResolvingAttempts
  -> SelectingPublishedRoot
  -> BuildingOrCatchingUp
  -> Verifying
  -> Ready(PublishedSnapshot)

Any incomplete stage -> Retryable(progress) | Refused(reason) | Corrupt(evidence)
Ready -> Closing -> Closed
```

Acquire the local process-lifetime lock first. Establish the configured remote/local-history origin and identity before applying a capsule or cleaning scratch. The ordinary open path never mutates remote retention based on a local scratch filename.

`Ready` means the facts, log system records, identity attachment, and stamps agree in one committed LMDB state. A directory existing on disk, a valid schema, and a matching generation are not sufficient. Failed hydration is never an empty database. A missing `HEAD` at a configured existing origin is `DatabaseMissing`; creation is a separate explicit operation, never open-or-create after an ambiguous GET.

For a usable matching local cache, refresh to one finite captured head. For an absent or corrupt disposable cache, build a new owned staging directory from a retained root, verify it completely, close old mappings, and activate atomically under ownership. Corrupt authoritative objects cause a stopped tenant and evidence, not endless delete/reseed attempts.

An imported root held for long work is registered by a CAS against the **exact head from which it was selected**. If that head moves, select/revalidate against its successor before adding the hold. This prevents the apparent pin operation from resurrecting a previously dropped, collectible root. A crash after pin publication may leak a named hold until explicit release; its bounded count and visible owner/operation metadata are an accepted cost, not a hidden lease system.

## Crash/ambiguity decision table

| Last possible completed action | What is authoritative | Recovery and caller result |
|---|---|---|
| Command sealed, no remote dispatch | No new durable decision | `NotSubmitted`; discard private work or resume same owned command |
| Some candidate objects uploaded | Old head | Do not serve candidate; current/next GC may reclaim orphan objects |
| Complete object upload, before head CAS | Old head | Retry only with a currently valid exact parent/epoch; no Published receipt yet |
| Head CAS dispatched, response unknown | Possibly old or new head | Resolve by command ID/digest in retained receipt state; otherwise return `OutcomeUnknown` within deadline |
| Head CAS published, local transaction still private | New remote decision | Apply/rebuild exact decision; never rerun a host callback |
| Head CAS published, local LMDB commit fails | New remote decision | Preserve Published receipt and cache-unavailable health; no false rejection or automatic new-ID retry |
| Local LMDB commit succeeded, response lost | Published remote/local-history decision | Return original receipt after lookup |
| Checkpoint uploaded, not referenced | Existing recovery root | No cleanup by “not current”; epoch GC determines unreachability |
| Checkpoint head CAS published, scratch not cleared | New checkpoint or a retained successor root | Recover from complete root graph; do not delete its objects simply because another checkpoint is now current |
| GC barrier published, mark incomplete | New object epoch, old roots protected | Resume/restart marking; no deletion from partial marks |
| Some eligible deletes completed | Unchanged valid roots | Resume idempotently; retain failed deletion/progress evidence |
| New local cache verified, activation interrupted | Old or new complete cache | Reopen only a verified complete directory; no partially copied database with a ready marker |
| Pool/owner closes during remote dispatch | Result may still become published | Stop new admission; preserve the in-flight identity and bounded resolution path |

When a receipt epoch has been retired, `ReceiptExpiredUnknown` is the honest answer to an unresolved historical request. A historical backup may be inspected for evidence, but active submission must still refuse that old ID. The current system cannot know a lost response's outcome indefinitely while also deleting all of its evidence; the API does not hide this tradeoff.

## Backup: independent bytes and authority, not another pointer

A named restore point protects a root from ordinary GC **inside the active store**. It is not an independent backup against operator credentials, bucket loss, destructive lifecycle changes, ransomware, or encryption-key loss.

The minimal `bumbledb-log` backup procedure is:

1. Acquire a named restore point at an exact published decision/state. Record a stable backup operation ID before copying.
2. Stream its complete declared dependency closure into a separate backup destination: a supported filesystem export directory/archive or separately authorized S3 prefix/bucket. Copy the checkpoint plus the exact bounded tail; do not require a full-RAM materialization.
3. Verify every copied object by hash/length. Build a bounded-stream backup manifest containing original identity/schema/stamps, framing versions, copied refs, application/system digests where present, and explicit external-blob inclusion/exclusion.
4. Install a **complete backup manifest last**, with no-overwrite publication or atomic completed-directory activation. Partial uploads are incomplete operations, not usable backups. A lost completion response is resolved by operation identity and manifest digest.
5. Restore into a fresh isolated directory and validate before declaring the backup qualified. Only then may the source restore point be released according to the operator's policy.

Normal writer/GC credentials cannot delete the backup destination or its keys. Backup retention and encryption-key retention are explicit separate policy. An operator with authority over both can still destroy both; do not advertise impossibility of administrative loss.

The manifest declares whether external blobs were copied and under what root enumeration. Arbitrary URLs in facts are not automatically backed up. If an application needs database-plus-blobs recovery, its manifest/copy job must enumerate and verify those blobs; a database-only backup remains clearly labeled database-only.

S3 versioning, Object Lock, secondary-account copies, and regional replication may strengthen a deployment, but 1.0 does not invent wrappers for every AWS product. Qualify the selected configuration and document which failures it covers. A versioned active bucket controlled by the same cleanup credentials is not called an independent backup merely because old versions might exist.

## Restore always states whether it is read-only or a new lineage

- **Read-only inspection/export:** retain original provenance and stamps; no publication authority is granted.
- **Writable restore:** create a new `IncarnationId`, validate into a fresh directory/origin, and establish a new genesis with the source backup/root, source stamp, and copied migration history recorded as provenance. Its admitted initial state may be nonempty. New decision/state counters start at zero and executable command-receipt state starts empty under the new incarnation, as defined in chapter 20. The logical `DatabaseId` may remain the same tenant identity. Old clients, witnesses, command scopes, and caches cannot continue unnoticed.

There is no default “rewind the old head.” Reusing an old incarnation at another location while the old authority might still accept writes creates two histories with the same identity. The initial tool refuses that operation rather than pretending that a hostname change proves exclusive ownership.

Application values, including ordinary application-owned 128-bit entity IDs, are preserved unless an explicit transformation changes them. Entity bytes encode no originating lineage and are **not** command/witness credentials requiring the current incarnation. Existing references do not need automatic remapping. The application chooses any new identifiers before sealing a command and retains them across retries; restore introduces no allocator or special generation rule. Physical LMDB row IDs and legacy-import dictionary IDs can be rebuilt without becoming application changes; 1.0 does not introduce another default global text dictionary.

Historical log receipt rows are recovery evidence, not current-lineage executable request identity. The new incarnation begins its own receipt policy and explicitly rejects old-scoped requests. Long-lived business idempotency recorded in application relations remains application data. An operator must account for external effects when restoring old state: restoring an outbox can make previously delivered actions appear pending, so downstream stable business identities are still required.

Restore validates the complete canonical theory and state, not only object hashes. A valid checksum can faithfully preserve a semantically invalid export from an old buggy version. On constraint failure, preserve evidence and refuse activation; do not quietly drop rows or auto-repair them.

## Repository migrations: generated plans, explicit durable execution

Users maintain application schemas with the TypeScript relation/schema library. Generation compares canonical previous/current schema descriptions and emits an ordered **canonical migration plan AST**, immutable schema snapshots, a manifest and a static asset index. This is the same design style as constructing a query AST from the existing TypeScript query library: no SQL/text parser, handwritten migration function, helper-import closure, or general-purpose TypeScript interpreter. The proposed generation-time entry point is `generateMigrations({ schema, hints })`; exact folder/CLI/types live in chapter 33.

Structural changes with one safe interpretation generate automatically. Ambiguous rename versus drop/add, data destruction, lossy conversions, required backfills, and ID/reference remapping require **explicit typed intent** before the generator emits an executable plan. A backfill is a checked constant or supported expression/query AST, not arbitrary JavaScript. Generated coverage accounts for every source/target relation and required value: preserve, transform, explicitly drop, or explicitly initialize. An unchanged relation cannot vanish merely because the developer did not mention it. If the finite supported grammar cannot express an intended conversion, generation returns an actionable refusal; 1.0 does not add a callback escape hatch or invent a business decision.

That expression/query AST is the core's typed representation, not a lookalike migration language. The log plan adds source/target schema mapping, coverage, operation identity and ordered execution; core prepare/execute, canonical values, change encoding and checked building supply the actual relational semantics. Nonrecursive composition, aggregate binding grain, empty-group behavior, numerical rounding/errors and restricted linear recursion obey the same core contract. A migration context can allow only a declared subset of those operators; it cannot silently give an imported operator a different meaning or implement a second evaluator.

`migrationStatus(binding, generatedPlans, options)` is read-only. `migrate(binding, generatedPlans, { operationId, to, signal, limits })` executes the selected unapplied suffix through one native log executor. Results distinguish `ReadyToSwitch { deploymentBinding, activationRef, history }`, `Activated` evidence with current access mode, and the precise status/paused/refused/unknown outcomes in chapter 33. `activateMigration(activationRef, adminOptions)` is explicit and separate. A same-schema data plan is still a recorded migration.

Normal request handlers never auto-run pending migrations. A production open checks the expected schema and applied manifest prefix and returns `MigrationRequired`/`MigrationDrift` when necessary. The setup integration can generate configuration, permissions and an explicit deployment/init migration invocation; it cannot hide a long data rewrite inside a request or silently bypass a source freeze. Development opt-in convenience invokes this same visible runner and failure contract.

### Authoritative migration history, not a local lockfile claim

The generated manifest defines the ordered chain and canonical plan identities. The database records what actually happened. One execution of a contiguous pending suffix has **one `Applied` batch record**, not a fictitious independently published database for every file:

```text
Applied {
  operationId, planSetDigest,
  sourceIdentity, sourceStamp,
  targetIdentity, targetSchemaId, targetDigest,
  steps: [ { seq, id, fromSchemaId, toSchemaId, planDigest } ]
}

Baseline {
  validatedManifestPrefix, targetIdentity, targetSchemaId,
  targetDigest, operationId, explicitReason
}
```

Each `planDigest` binds the exact canonical plan, from/to schema identities and plan/semantic codec versions. `planSetDigest` binds the selected contiguous ordered suffix and its starting prefix; prefix/manifest hashes use acyclic versioned domain-separated framing as specified in chapter 33. Hash canonical data, not raw TypeScript helper source or a custom executable migration bundle. The ordinary SDK/native release artifact still needs normal provenance and qualification; that is not a second migration compiler/launcher pipeline. Unknown plan operators/codecs, altered plan bytes, missing/reordered entries and prefix mismatches refuse before mutation where knowable. Resume pins the same source, operation and plan bytes under a qualified executor for that codec; it never silently substitutes a newly generated plan.

Migration history is authoritative log system data: transactionally stored with local-history metadata or referenced by the hosted head/genesis and included in checkpoint/backup/GC closure. Flatten the ordered `steps` of applied batches to verify the exact manifest prefix. Every batch's source is the original captured database, its target is the one final published database, and intermediate step schemas are logical boundaries, not fake incarnations. The new incarnation carries the verified prior history plus this one complete `Applied` record. Its `targetDigest` covers canonical application state, excluding the history record/genesis itself; no self-referential hash is introduced. Receipt retirement cannot erase migration history.

Ordinary explicit initialization evaluates the initial generated chain from its declared empty base schema, including canonical seed operations. Creating an empty latest-schema database must not falsely mark skipped seeds `Applied`. Seed IDs/values are fixed in the generated assets or derived by a supported deterministic expression over versioned input; execution does not generate random IDs or call an external service. Explicit adoption of an already validated snapshot can record a `Baseline` with a reason and verified prefix; this is visibly different from applying the plans. Restore copies historical evidence and adds restore provenance without rerunning seeds.

### One final build/publication; preserve the meaning of the ordered steps

The executor plans the whole pending suffix against one frozen source and builds one final target. It may copy unchanged relations once, compose compatible structural rewrites and fuse compatible expressions using the existing native query/admission/builder machinery. It must preserve the declared logical step order, intermediate schemas, required constraint checks and refusal behavior. A later step cannot hide an earlier invalid intermediate state just because the final state would satisfy its schema. Optimization is valid only when it has the same complete result **and error semantics** as straightforward ordered execution.

Where a step genuinely requires an intermediate relation/state, use bounded private LMDB staging/scratch and validate that boundary; do not upload a complete intermediate checkpoint or create an intermediate public history authority. Initial 1.0 does not promise every sequence can be fused into one scan, but neither does it require k whole-database uploads/incarnations for k small changes. Measure bytes read/written and peak source/target/scratch overlap on representative multi-step migrations, including a >RAM large tenant. The deliberate offline interval and remaining full-target export are real costs, not erased by familiar CLI ergonomics.

### Durable runner steps

The 1.0 runner is offline **per tenant**. There is no automatic dual-write, online migration engine, global migration transaction or fleet scheduler. The deliberate write-unavailable interval buys a smaller correctness surface.

1. **Generate, review, plan and rehearse.** Verify canonical generated assets, stored applied prefix, source/target schemas, supported operators and exact pending suffix. Require unresolved destructive/ambiguous intent to be settled before an executable plan exists. Validate generated coverage, reserve estimated disk/RAM work, and record a stable operation ID, `planSetDigest` and one planned final target incarnation. Rehearse with a verified backup. Different plan bytes cannot take over a frozen operation by reusing its label.
2. **Freeze source writes durably.** Hosted history conditionally changes source `HEAD` to `Frozen` with a typed migration intent binding operation ID, source history/prefix, plan set and final target identity. Local history makes the equivalent mode/intent change atomically in LMDB under its writer session. Freeze waits behind an admitted local transaction, then prevents further ones; no expiring filesystem lease is involved. The hosted revision invalidates old remote candidates. Reads and backup remain possible. Crash, timeout or failed execution leaves the source frozen until explicit resume/abort; no timer thaws it.
3. **Capture final source.** Select the exact post-freeze decision/state and retain its source root. Confirm the frozen operation identity before every resumption. The access mode, not a best-effort request to application writers, establishes the final source boundary.
4. **Execute the checked plan into isolated staging.** The native executor reads the fixed source, applies supported plan/query operators and writes bounded batches into the private checked builder. It observes work/cancellation limits between bounded native steps. **No user TypeScript row callback, getter, iterator or `await` runs inside a native write transaction.** The caller passes owned canonical plans, not live functions. Partial states are staging only. Preserve entity IDs/references by default; perform a remap only if explicitly described by a checked plan covering every reference. Never mutate source files.
5. **Resume the operation, not a callback stack.** Resolve whether the final target already published before starting work. A complete verified target is reused. Otherwise restart the **unpublished plan execution** from the fixed original source into fresh owned staging using the same plan bytes; there is no per-page transformation journal or durable intermediate-incarnation chain. Private scratch is not an applied prefix and cannot be adopted merely because a directory exists. This intentionally can redo expensive work after interruption. A required missing plan or unsupported codec refuses while the source remains frozen. Plans cannot invoke network, clock, random allocation or external business effects; necessary external inputs must already be explicit versioned data/literals.
6. **Validate target and semantic boundaries.** Check the complete final theory and invariants, required intermediate-law checks, canonical target digest, exact history-prefix extension, ID/reference mapping, empty current receipt state, declared blob requirements and representative query expectations. Resource exhaustion/invalid output leaves the source frozen and target unactivated. The same operation/source/plan must produce the same logical output; inconsistency is `MigrationOutputMismatch`, not overwrite permission.
7. **Publish the final target genesis and whole applied suffix together, still frozen.** Install the admitted final state, inherited history and one complete `Applied { steps }` record under `Frozen { AwaitingCutover, operationId, planSetDigest }` with the new incarnation's command scope. Hosted mode uploads/publishes its final genesis/head once; local mode publishes its complete owned LMDB destination. There is no “schema switched, journal write failed” window and no intermediate public target. Resolve a lost response from planned target identity, operation, plan and output/genesis evidence; never redo a completed plan merely because its response was lost. Concurrent matching runners recover that same final target or refuse conflicting evidence.
8. **Configure the frozen target, then activate explicitly.** `ReadyToSwitch` carries an activation reference binding final target identity/genesis, operation and plan set. Under application maintenance mode, configure/deploy the new authenticated binding while the target stays frozen, then perform authorized read-only validation. `activateMigration` verifies that evidence and atomically changes the matching target to Active through its actual authority: S3 head CAS or local LMDB metadata transaction. Persist a one-time activation marker in that same transition, so a lost response remains resolvable after later writes/maintenance. A matching retry returns recorded evidence plus current access mode, never thawing a later freeze or reviving deletion. Verify activation/new reads and writes, replace old command/witness scopes and re-enable traffic. External configuration is not atomically changed by native activation; keep the source frozen throughout.
9. **Retain evidence and clean up deliberately.** Keep the old source and independent backup for the chosen rollback period, then release/delete explicitly. The CLI and TypeScript `migrate()` are adapters over one native plan executor and durable workflow, not independent implementations.

Before **target activation**, explicit abort must first **durably prevent target activation and delayed genesis publication**, then thaw the matching source. Observing Frozen/NotActivated or an absent target is insufficient: a paused activation/genesis can still win after that observation.

- Hosted abort races activation on the target's existing HEAD authority: CAS the matching unactivated Frozen target to terminal `Deleted/MigrationAborted`, or conditionally create that exact cancellation tombstone if target genesis has not published. Bind source, operation, plan and planned target identity. Resolve uncertain cancellation; if activation won or target evidence conflicts, refuse automatic abort/thaw.
- Local abort and final-target publication use the **same stable target-namespace kernel lock**, outside staging or materialization directories that can be renamed/replaced. Under that lock, commit the matching terminal cancellation in existing local control, or durably install an absent-target cancellation tombstone before any genesis exists. Complete fsync/directory durability before releasing exclusion. Final genesis installation is no-overwrite under this same lock and refuses the tombstone; a precomputed rename cannot bypass it. The terminal marker and stable lock identity are not scratch or cache-eviction targets. Live LocalHistory facts/control still commit in one LMDB transaction; a pre-creation cancellation marker is not a second live head/generation sidecar.
- Only after that irreversible target fence is known durable may the source's matching Frozen operation change to Active through its own CAS/LMDB transaction. A crash between the two leaves the source frozen and safely resumable. Retrying the cancelled operation reports `Aborted`; it cannot resume target creation or activation. An uncertain target cancellation never authorizes thaw.

After activation, unchanged `StateStamp` is not proof of safe rollback: no-change commands can record durable results and be followed by external business effects. Require an explicit decision/effect audit and reverse/repair plan or documented loss acceptance, not an automatic config toggle. An aborted target's data may be retained by an explicit named root/backup before cancellation or collected afterward; its terminal namespace cannot be reused.

Reproducibility means the same pinned input and canonical plan under its semantic codec yield the same logical output/error, not byte-identical LMDB files. A representation-only rebuild may preserve the original logical digest; a schema/data change needs its own expected-state/differential oracle. Tiny reference execution of ordered steps is the oracle for native fusion, including intermediate constraint failures, seeds, row-order independence and cancellation/restart. No documentation claim of automatic generation replaces those tests.

## Migration from the audited pre-1.0 implementation

There is no in-place upgrade of old braid manifests, allocator counters, lease files, or pending sidecars into the new authority representation. They encode different guarantees.

- Preserve an independent copy of the old origin/local materialization and document unresolved commands and known retention gaps before transformation.
- Stop/fence all old writers using deployment controls appropriate to the old system; the new protocol cannot retroactively enforce a fence in an old binary that ignores it. Retire or revoke its write credentials before making a successor authoritative.
- Export an explicit old state selected with the old tool's supported checks. The migration tool should reject unexplained pending state or inconsistent recovery metadata, not “fix” it by taking whichever copy opens first.
- Validate canonical values, all constraints, and logical content with the new checked importer. Preserve business identifiers; record any deliberate transformations and rejected data separately.
- Create a new incarnation and new-format genesis. Do not transfer old schema/vector equality into an identity proof or promise old command deduplication when old receipts did not exist.

The audit demonstrated cases where old `Published` effects are not in an ordinary fresh replica. Importing the current visible state cannot prove those past acknowledgments were never lost. Preserve forensic history/backups and make that limitation explicit. A successful migration is a verified chosen starting state, not retroactive verification of the old protocol.

## Erasure is a lifecycle across representations

Logical fact deletion is not secure erasure. It does not immediately remove receipt results, text history, snapshots, old LMDB pages, backups, exports, object versions, logs, application blobs, or keys held by another service.

Two separate procedures are required:

| Request | What the system actually does |
|---|---|
| Remove application facts | Normal admitted command; later checked rebuild/checkpoint can omit unreachable values; retained history may still contain them |
| Erase an entire tenant according to a retention policy | Freeze, settle/report outstanding uncertainty, tombstone active head, revoke writers, close/remove owned local caches, release allowed restore roots, collect active objects/versions, expire separately governed backups/exports/blobs, and handle encryption keys under explicit policy |

Keep a minimal tombstoned head/incarnation marker to prevent recreation by stale writers. It contains no application facts or receipt payloads. If compliance requires removing even that identity metadata, first revoke all old write authority permanently and document that namespace removal is outside the ordinary never-reused-head protocol; a future database must use a new identity.

User-level erasure inside a surviving tenant cannot be claimed by deleting a whole-tenant encryption key. It needs an application-aware retention/redaction policy covering the specific facts and historical copies. Full secure overwrite of SSD blocks is not promised by deleting LMDB files. Encryption-key destruction has its own guarantees and caveats, including copies and backup keys; report those instead of inventing a blanket secure-delete claim.

Rebuilding from only live canonical facts reclaims unreachable text/dictionary entries from the **new** materialization, but an old snapshot intentionally retains old text until released. The main engine can support live-only rebuild/compaction as a storage primitive; backup/retention/erasure orchestration stays in the log.

## Health and evidence without building an observability platform

Expose a structured redacted status record and events sufficient for a host to integrate:

- Identity/origin/format, access mode, decision/state/head revisions.
- Last verified local stamp, captured refresh target, current progress/lag, and corruption/refusal reason.
- Unknown command IDs/digests and age/work attempts; no raw fact payloads by default.
- Replay tail count/bytes, checkpoint progress/last success/failure, current GC barrier/cursor/failures, held roots/capacity.
- Local owned handles, pinned snapshots, disk reservations, queue and network work, overload/cancellation state.
- Backup operation completion and most recent restore validation result, as caller-supplied/log-tool evidence rather than a fabricated always-on fleet service.

Status should distinguish `stale but valid`, `not yet hydrated`, `unavailable`, `corrupt`, `frozen`, `closed`, and `empty`. Do not log secret rows, credentials, or full command bodies to make retries debuggable. Attach stable operation IDs and redacted hashes to retained test/incident artifacts.

## Complete recovery/operations test suite required for 1.0

These are release criteria, not completed work. The gate graph in the proposal's release plan must require them in addition to semantic, SDK, protocol, and adapter gates. A platform feature not exercised is unsupported; a skipped test cannot supply evidence.

| Gate | Required test and success oracle |
|---|---|
| `REC-01` | Kill/restart at every recovery-state boundary in the table; only complete identified published snapshots become Ready |
| `REC-02` | Same live handle after unknown failure, same directory reopened, entirely fresh directory, and other host; one original command result, no overwritten pending evidence |
| `REC-03` | Wrong origin/schema/incarnation with matching counts and cached pending/scratch; zero foreign reads/replay/remote cleanup |
| `REC-04` | Published command followed by local fsync/native close failure; caller/fresh host still resolves Published and local health is unavailable rather than rejected |
| `REC-05` | Lost checkpoint receipt, advance head/checkpoint, GC, restart; current and named historical roots remain reconstructible |
| `REC-06` | Missing/corrupt authoritative chunk/decision vs corrupt disposable local cache; correct differentiated failure, bounded retries, no empty fallback |
| `REC-07` | Hold root, crash owner, capacity exhaustion, explicit revocation, hydrate completion; leak is visible/bounded and partial imports never serve |
| `BACKUP-01` | Backup a captured root while writes/checkpoint/GC continue; restore exact captured facts/receipts/stamps from destination only |
| `BACKUP-02` | Interrupt every object copy, manifest completion and final activation; incomplete backup is never listed as complete; operation-ID retry is idempotent |
| `BACKUP-03` | Restore with source bucket/local cache unavailable and ordinary GC credentials denied access to backup; independence is actually tested |
| `BACKUP-04` | Corrupt one backup object/manifest; missing external blob; wrong key; unsupported version; fail before activation with precise evidence |
| `BACKUP-05` | Backup/restore beyond RAM and across different supported platforms; bounded stream memory and logical equality, no reliance on physical LMDB byte layout |
| `RESTORE-01` | Writable restore creates a new incarnation; old witness/request/cache refuses, ordinary application entity/reference bytes are preserved without lineage remapping, and no database allocation guarantee is implied |
| `RESTORE-02` | Read-only inspection has no mutation capability; explicit attempt to rewind/reuse a live incarnation is refused |
| `RESTORE-03` | Restore old outbox/application idempotency facts; example tests document duplicate-delivery hazards and stable receiver deduplication |
| `MIG-01` | Hosted candidate paused before freeze CAS and local transaction racing atomic LMDB freeze; correct mode boundary, old work cannot publish afterward, reads remain valid, and no crash/time passage implicitly thaws |
| `MIG-02` | Canonical native plan on a pinned source, including >RAM input; independent expected facts/theory, preserved or explicitly mapped references, bounded working memory and measured source/target/scratch disk |
| `MIG-03` | Crash at every freeze/capture/plan-build/validate/final-genesis/switch step; resumable operation identity, original source preserved, no accidental dual authority or published intermediate incarnation |
| `MIG-04` | Target/intermediate validation rejection, unsupported operator/codec, missing destructive intent or plan mismatch; no target activation, guessed rename/backfill or automatic row dropping |
| `MIG-05` | Race abort with paused activation and delayed genesis, including an absent target; source thaws only after the matching terminal target fence is durable. Lose cancellation response or kill before source thaw, then resume; never two active authorities. After activation, even unchanged StateStamp/no-change receipts cannot justify automatic config-only rollback |
| `MIG-06` | Golden pre-1.0 valid/invalid fixtures, unresolved pending and historical loss evidence; importer never claims retroactive durability or silently upgrades old state machines |
| `MIG-07` | Edit/delete/reorder applied canonical plans, schema snapshots, IDs or codec versions; exact generated manifest/prefix validation detects drift before freeze/output writes; semantically identical source formatting does not change a plan digest and no helper-source compiler is required |
| `MIG-08` | Native plan execution with >RAM source/target, tiny batches, disk exhaustion, cancellation and kill; no user callback/function is accepted, no JS runs under native RW ownership, partial batches stay private and complete/global constraints govern activation |
| `MIG-09` | Two runners/resume after lost final-target response, matching operation and conflicting operations/plans; recover one planned final target or refuse; incomplete execution restarts from fixed original source without claiming scratch/intermediate steps Applied |
| `MIG-10` | Crash between target validation, whole-suffix Applied record construction and final genesis publication; target data and exact complete history extension publish together once; no partial batch prefix or fake intermediate identity |
| `MIG-11` | Fresh initialization runs initial schema/seed chain; explicit Baseline adopts only validated state and is distinguishable from Applied; repeated runner never duplicates seed actions |
| `MIG-12` | Checkpoint, command-receipt retirement, GC, backup and writable restore preserve required migration history; old request scopes stay invalid and old application entity references remain valid data |
| `MIG-13` | Fused native plan versus independent ordered-step oracle across page/order/memory variants; identical final facts and required intermediate-law failures, no skipped seed/error boundary; same operation/source/plan with conflicting completed output refuses; small compatible changes do not force a full published target per step |
| `MIG-14` | Private intermediates never become public; final target remains Frozen until explicit activation; wrong/stale refs refuse; lost activation response resolves after later metadata/commands; matching retries report prior activation/current mode without thawing later freeze or reviving deletion. Local final install/cancellation share stable namespace exclusion, no-overwrite and durable tombstones across rename/restart/cleanup; cancelled operation retries report Aborted. Binding switch is never claimed atomic with native activation |
| `ERASE-01` | Delete facts then rebuild; current logical state excludes values while named history still retains them until explicit release |
| `ERASE-02` | Whole-tenant deletion with delayed old writer and old local owner; tombstone prevents new publication, has no active recovery root, and ordinary access refuses before hydration. Explicit retained roots survive until released; former live objects become collectible after barrier/retention obligations end; stale cache owner cannot erase successor files |
| `ERASE-03` | Active objects, noncurrent versions, delete markers, backups, exports and declared blobs enumerated; report residual copies rather than false secure-erasure success |
| `ERASE-04` | Distinct whole-tenant key deletion versus individual-user erasure; examples/API documentation cannot confuse the scopes |
| `OPS-TEST-01` | Status/event fixtures for stale/empty/unavailable/corrupt/frozen/closed; credentials/row payloads absent from default logs |
| `OPS-TEST-02` | Bounded shutdown with in-flight opens/writes/checkpoint/import/GC; native work quiesces or yields explicit unknown state, with no new post-close installation |

For each backup/migration test, retain the source identity, operation/plan digest, target identity, expected and actual logical digest or refusal, dependency list, fault position, returned user outcome and supported platform. Include Apple Silicon, ARM Graviton and the qualified x86 Vercel runtime where the operation is advertised; a long migration may run on a separate adequately provisioned admin host. Do not collapse “copied bytes,” “verified complete backup” and “successfully restored application state” into one green test name.

The process-kill lane and real-S3 lane must execute these histories against the **packaged fresh native implementation**, not an already-installed unknown artifact or a forgiving test store. Fault simulation supplies reproducible coverage; actual backend qualification supplies evidence that its premises are true. Both are required.

## Audit disposition and explicit scope limits

| Audit IDs | 1.0 disposition | Closure and remaining cost |
|---|---|---|
| REP-007/009/017/019; SDK-004/005/006/007/013 | Ownership before cleanup, no scratch-based remote deletion, explicit complete-state activation and deterministic native close | REC-01/05/07, OPS-TEST-02 plus SDK/FS gates; no expiry takeover of paused processes |
| REP-011; SDK-016; ARCH-004 | Origin/incarnation-bound recovery, new writable restore lineage | REC-03, RESTORE-01/02; explicit application config switch |
| ENG-003/006 | Single-snapshot export and live-data rebuild, with historical retention stated | BACKUP-05, ERASE-01; full streamed rebuild costs I/O/disk |
| ENG-004/007; REP-004/006 | Delete allocator/reservation and sequence-derived entity IDs; application owns identifiers fixed before command sealing | MIG-06 and PROTO-11; preserved business IDs remain data, not command/receipt authority |
| SDK-001/002/014; REP-016 | Published-only recovery/reads, explicit unresolved identity and local-cache-failure health | REC-02/04, OPS-TEST-02; durable unknown-result handling remains essential |
| OPS-001 | Schema-library-generated canonical plans, authoritative checked history, one final freeze/build/validate/genesis/cutover runner | MIG-01–14; explicit ambiguity/data-loss intent, tenant write downtime, no handwritten callbacks, per-step published copies or online/fleet engine |
| OPS-002 | Independent verified backup manifests and restore drills | BACKUP-01–05; backup storage/credentials/retention cost remains operator-owned |
| OPS-003 | Explicit declared blob closure and application outbox boundary | BACKUP-04, RESTORE-03; no arbitrary-URL backup or exactly-once external effects |
| OPS-004/005/006 | Scoped origin authority, host budgets, explicit freshness/health/redacted events | REC-03/06, OPS-TEST-01/02 and SDK gates; authentication/routing service is not supplied |
| ASS-003/004 | Versioned operational docs, preserved dated audit/counterexamples and resolution evidence | Packed examples + finding-to-test release ledger; no deleting evidence when a fix lands |

No finding is “fixed” by this document. The proposed representation either replaces its failing mechanism, narrows the product contract explicitly, or supplies a required gate. Implementation closure requires those gates to fail on the old failure shape and pass on the successor, followed by independent review of the supported deployment assumptions.
