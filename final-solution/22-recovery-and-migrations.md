# 22 — Recovery, backups, erasure, and migration belong to the log

Status: proposed 1.0 design and release gates, not implemented recovery tooling. This chapter specifies the TypeScript log package's recovery/migration runner plus operator procedures, **not** a fleet control plane, scheduler, migration service, or consensus system. The Rust protocol/build machinery is internal; no public Rust or C log API/header is required for 1.0.

The boundary is explicit: `bumbledb` provides an owned consistent snapshot, checked construction/admission, queries, and durable LMDB storage. **The TypeScript `bumbledb-log` package owns repository-authored migrations, explicit execution, export/import orchestration, retained history, backups, restore provenance, incarnation changes, schema transformations, and cutover procedures.** A generic core snapshot/copy/builder is not branded as a backup or a migration API. Chapter 33 supplies the Drizzle/Expo-style repository ergonomics and Next.js/Alchemy setup; this chapter supplies their durable meaning.

## Recovery has a small number of states

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

Application values, including existing nominal 28-byte entity IDs, are preserved unless an explicit transformation changes them. An old entity ID records its originating lineage; it is still an ordinary value in the restored state, **not** a command/witness credential requiring the current incarnation. Existing references do not need automatic remapping. Newly generated IDs use the new incarnation, preventing collision with old allocation positions. Physical LMDB row IDs and legacy-import dictionary IDs can be rebuilt without becoming application changes; 1.0 does not introduce another default global text dictionary.

Historical log receipt rows are recovery evidence, not current-lineage executable request identity. The new incarnation begins its own receipt policy and explicitly rejects old-scoped requests. Long-lived business idempotency recorded in application relations remains application data. An operator must account for external effects when restoring old state: restoring an outbox can make previously delivered actions appear pending, so downstream stable business identities are still required.

Restore validates the complete canonical theory and state, not only object hashes. A valid checksum can faithfully preserve a semantically invalid export from an old buggy version. On constraint failure, preserve evidence and refuse activation; do not quietly drop rows or auto-repair them.

## Repository migrations: pleasant TypeScript, explicit durable execution

Applications keep ordered TypeScript migration files, immutable versioned schema snapshots, and a checked generated manifest/index in their repository. The public shapes are `defineMigration({ id, from, to, async transform({ source, target }) { ... } })`, `defineMigrations({ manifest, modules })` with explicit bundled imports, `migrationStatus(binding, migrations, options)`, and `migrate(binding, migrations, { operationId, to, signal, limits })`. Results distinguish `ReadyToSwitch { deploymentBinding, activationRef, history }`, completed `Activated` evidence with current access mode, and the precise status/paused/refused/unknown outcomes in chapter 33. `activateMigration(activationRef, adminOptions)` is explicit and separate. Files can change schemas, data, or both; a same-schema data transformation is still a recorded migration. The exact folder/CLI/Next.js/Alchemy interface lives in chapter 33, rather than another independent implementation here.

Normal request handlers never auto-run pending migrations. A production open checks the expected schema and applied manifest prefix and returns `MigrationRequired`/`MigrationDrift` when necessary. The setup integration can generate configuration, code, permissions, and an explicit deployment/init migration invocation; it cannot hide a potentially long data rewrite inside a request or silently bypass a source freeze. Development opt-in convenience must invoke this same visible runner, with the same failure contract.

### Authoritative migration history, not a local lockfile claim

The repository manifest defines the ordered chain and its checksums. The database's log metadata records what actually happened. A migration history entry has this logical shape:

```text
Applied {
  seq, id, fromSchemaId, toSchemaId,
  sourceDigest, artifactDigest, operationId,
  sourceIdentity, targetIdentity, sourceStamp, targetDigest
}

Baseline {
  validatedManifestPrefix, targetIdentity, targetSchemaId,
  targetDigest, operationId, explicitReason
}
```

`sourceDigest` covers the resolved restricted local helper/schema import closure and semantic codec version, not only the top-level `.ts` file. `artifactDigest` identifies the actual compiled migration artifact verified for that execution. Applied prefix identity/source/schema checksums must agree with the repository manifest; editing, deleting, reordering, or replacing already applied source does not silently become a new migration. An artifact build may have its own recorded provenance, but a source mismatch is not excused by matching filenames. Dynamic untracked code loading cannot participate in a reproducible checked manifest.

Migration history is authoritative log system data: transactionally stored with local-history metadata or referenced by the hosted head/genesis and included in checkpoint/backup/GC closure. It is not trusted because a developer machine says its migration ran. Each new migration incarnation carries the previously verified history prefix plus the new `Applied` entry, bound by its genesis. A record's target digest is the canonical application-state digest, not a circular reference to the genesis hash that includes the record. Receipt epochs and migration history are separate: retiring command receipts cannot erase which schema/data transforms created the active lineage.

Ordinary explicit initialization runs the initial chain from its declared empty base schema, including seed-data transforms. Creating an empty database directly at the newest schema must not falsely mark skipped seed migrations `Applied`. Explicit adoption of an already validated snapshot can use a `Baseline` with a reason and verified prefix; this is visibly different from running those transformations. Restore copies historical migration evidence, then records the new restore provenance; it does not rerun historical seeds implicitly.

### Durable runner steps

The 1.0 runner is offline **per tenant**. There is no automatic dual-write, online delta-transform engine, global migration transaction, or fleet rollout scheduler. TypeScript files provide the familiar authoring experience; the deliberate write-unavailable interval buys a much smaller correctness surface.

1. **Plan and rehearse.** Verify the repository manifest, stored applied prefix, exact source/target schemas, artifact checksums, and chapter 33's finite relation coverage (`reads`, `writes`, exact compatible `copy`, explicit `drop`/`empty`). Missing or incompatible coverage refuses before freeze; an unchanged relation is not silently omitted from the new database. Record operation ID, planned target incarnation, anticipated disk/RAM work and rollback plan. Rehearse with a verified backup. A different migration cannot take over an existing frozen operation by reusing its label.
2. **Freeze source writes durably.** Hosted history conditionally changes source `HEAD` to `Frozen` with a typed migration intent binding operation ID, source history/prefix, selected manifest entry/checksums, and planned target identity. Local history performs the equivalent mode/intent update atomically in LMDB under its writer session. Freeze waits behind a currently admitted local transaction, then prevents further ones; it is not emulated by S3 CAS or an expiring filesystem lease. The hosted revision change invalidates old remote candidates. Reads and backup remain possible. Crash, timeout, or a failed transform leaves the source frozen until explicit resume/abort; no timer thaws it.
3. **Capture final source.** Select the exact post-freeze decision/state and acquire a retained source root. Confirm the frozen operation identity before every resumption. The access mode, not a best-effort request to application writers, establishes the final source boundary.
4. **Transform into isolated staging.** TypeScript reads bounded copied pages from that fixed source and submits bounded canonical output batches to the private checked native builder. **No TypeScript transform runs inside an LMDB write transaction.** Native batch ingestion completes/closes its private transaction before control returns to user TypeScript. Partial target states are staging only; they never become public query/history state. The final complete theory is validated before activation, including constraints that necessarily span batches. Preserve application IDs/references unless the migration explicitly remaps them. Source files remain untouched.
5. **Handle transform effects honestly.** A transform is deterministic, side-effect-free data conversion under its declared source/artifact contract. No network calls, clock/random dependence, untracked dynamic imports, or external business effects belong in it. Types cannot prove arbitrary JavaScript pure: constrain the migration interface/import closure, lint and test it, record exact artifacts, and validate deterministic output. On failure, restart **the incomplete explicit migration step** from its fixed source into fresh isolated attempt staging, using the same pinned artifact. Reuse fully published and verified intermediate destinations; do not promise JavaScript stack restoration, incremental transform checkpoints, or a migration-journal DSL. An unavailable required artifact yields an explicit refusal while the source stays frozen. This never secretly reruns an ordinary write callback. A transform that needs external inputs must first materialize/version them as explicit input data in a separate authorized workflow.
6. **Validate target.** Check target theory, complete application invariants, canonical target digest, history-prefix extension, ID/reference mapping, receipt initialization, declared blob requirements, and representative query expectations. Resource exhaustion or invalid output leaves the source frozen and target unactivated; no row dropping or partial activation occurs. Reruns with the same operation/source/artifact must agree on logical output; an inconsistent target is `MigrationOutputMismatch`, not a last-writer overwrite.
7. **Publish target genesis and history together, still frozen.** Install the complete admitted target with inherited migration prefix plus one `Applied` entry and fresh-incarnation command receipt policy, under `Frozen { AwaitingCutover, operationId, planDigest }`. Hosted mode publishes its fully staged genesis/head once; local mode publishes its complete owned LMDB destination. Genesis publication binds migration record and data together—there is no “schema switched, migration journal write failed” window. Intermediate targets are read-only sources for subsequent steps, never briefly active application databases between steps. A lost response is resolved from the planned target identity, operation ID and manifest/output hashes; never run the transform again merely because the return value was lost. Concurrent runners for the same operation recover the same completed target or refuse conflicting evidence.
8. **Configure the frozen target, then activate explicitly.** `ReadyToSwitch` carries a checked activation reference binding the final target identity/genesis, operation and plan. Under application maintenance mode, deploy/configure the new authenticated binding while the target is still frozen, then perform authorized read-only validation. `activateMigration` verifies that evidence and atomically changes the corresponding frozen target to Active using its actual authority: hosted head CAS or local LMDB metadata transaction. Persist a one-time activation outcome/marker in that same transition, so a lost response remains resolvable after later writes/maintenance; this is bounded per-incarnation metadata, not a service or ordinary expiring command receipt. A matching retry returns recorded evidence plus current access mode, never re-activating a target frozen by a later operation. Verify activation/new reads and writes, replace old session/request scopes, then re-enable traffic. There is no fleet router in the database and no claim that external configuration changes atomically with native activation. Old clients receive Frozen/WrongIncarnation; keep the original source frozen through cutover.
9. **Retain evidence and clean up deliberately.** Keep the old source and independent backup for the chosen rollback period, then release/delete explicitly. An operator can use the same runner primitives manually; the CLI and `migrate()` are adapters over one implementation, not independent procedures with different durability.

Before **target activation**, an explicit abort can preserve/discard the still-frozen target and thaw the source after checking operation identity and proving activation did not occur. An unknown activation response must be resolved first. After activation, do not infer safe rollback from an unchanged `StateStamp` or zero changed facts: a no-change command can issue fresh IDs, record receipts, or be followed by external business effects. Rollback then requires an explicit decision/effect audit and reverse/repair workflow or documented loss acceptance, not a config toggle. The operator must choose; the library cannot merge arbitrary application meanings automatically.

A converter may preserve the original logical data digest only for a representation-only rebuild. A schema-changing transform needs its own expected-state/differential oracle. Converter reproducibility means the same pinned input and converter produce the same logical output, not necessarily byte-identical LMDB files. Multi-step plans validate and record each applied prefix; an interrupted plan can resume from the actual last completed destination rather than guessing from a directory name or replaying already applied seeds.

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
| `RESTORE-01` | Writable restore creates a new incarnation; old witness/request/cache refuses, old application IDs preserved, new IDs cannot collide |
| `RESTORE-02` | Read-only inspection has no mutation capability; explicit attempt to rewind/reuse a live incarnation is refused |
| `RESTORE-03` | Restore old outbox/application idempotency facts; example tests document duplicate-delivery hazards and stable receiver deduplication |
| `MIG-01` | Hosted candidate paused before freeze CAS and local transaction racing atomic LMDB freeze; correct mode boundary, old work cannot publish afterward, reads remain valid, and no crash/time passage implicitly thaws |
| `MIG-02` | Deterministic converter on a pinned source, including >RAM input; independent expected facts/theory, ID/reference mapping and resource bounds |
| `MIG-03` | Crash at every freeze/export/transform/validate/genesis/switch step; resumable operation identity, original source preserved, no accidental dual authority |
| `MIG-04` | Target validation rejection, new format mismatch, converter mismatch; no target activation and no automatic row dropping |
| `MIG-05` | Pre-activation abort/thaw checks operation identity and activation certainty; after activation, even unchanged StateStamp/no-change receipts cannot justify an automatic config-only rollback |
| `MIG-06` | Golden pre-1.0 valid/invalid fixtures, unresolved pending and historical loss evidence; importer never claims retroactive durability or silently upgrades old state machines |
| `MIG-07` | Edit/delete/reorder applied migration files, imported local helpers, schema snapshots, codec version or execution artifact; checked manifest/prefix validation detects drift before freeze/output writes |
| `MIG-08` | TypeScript page transform yields, throws, cancels and processes >RAM data; no user code executes with native RW transaction held, partial batches remain private, complete global constraints govern final activation |
| `MIG-09` | Two runners/resume after lost target-publication response, same operation and conflicting operations/artifacts; recover one planned target or refuse conflict, never rerun an already completed step blindly |
| `MIG-10` | Crash between complete target validation, migration-history construction and genesis activation; no target can claim a schema/Applied prefix whose admitted data was not atomically bound to genesis |
| `MIG-11` | Fresh initialization runs initial schema/seed chain; explicit Baseline adopts only validated state and is distinguishable from Applied; repeated runner never duplicates seed actions |
| `MIG-12` | Checkpoint, command-receipt retirement, GC, backup and writable restore preserve required migration history; old request scopes stay invalid and old application entity references remain valid data |
| `MIG-13` | Same pinned source/artifact produces different staged logical output, or attempts untracked/dynamic dependencies; reject inconsistent/unreproducible execution, preserve published target/evidence, and never change ordinary command callback semantics |
| `MIG-14` | Intermediate/final targets remain Frozen until explicit activation; wrong/stale activation refs refuse; lost activation response resolves from durable marker after later metadata/commands; matching retries report completed activation/current mode without thawing a later freeze or reviving deletion; external binding switch is never claimed atomic with native activation |
| `ERASE-01` | Delete facts then rebuild; current logical state excludes values while named history still retains them until explicit release |
| `ERASE-02` | Whole-tenant deletion with delayed old writer and old local owner; tombstone prevents new publication and stale cache owner cannot erase successor files |
| `ERASE-03` | Active objects, noncurrent versions, delete markers, backups, exports and declared blobs enumerated; report residual copies rather than false secure-erasure success |
| `ERASE-04` | Distinct whole-tenant key deletion versus individual-user erasure; examples/API documentation cannot confuse the scopes |
| `OPS-TEST-01` | Status/event fixtures for stale/empty/unavailable/corrupt/frozen/closed; credentials/row payloads absent from default logs |
| `OPS-TEST-02` | Bounded shutdown with in-flight opens/writes/checkpoint/import/GC; native work quiesces or yields explicit unknown state, with no new post-close installation |

For each backup/migration test, retain the source identity, operation/converter digest, target identity, expected and actual logical digest, dependency list, fault position, returned user outcome, and supported platform. Do not collapse “copied bytes,” “verified complete backup,” and “successfully restored application state” into one green test name.

The process-kill lane and real-S3 lane must execute these histories against the **packaged fresh native implementation**, not an already-installed unknown artifact or a forgiving test store. Fault simulation supplies reproducible coverage; actual backend qualification supplies evidence that its premises are true. Both are required.

## Audit disposition and explicit scope limits

| Audit IDs | 1.0 disposition | Closure and remaining cost |
|---|---|---|
| REP-007/009/017/019; SDK-004/005/006/007/013 | Ownership before cleanup, no scratch-based remote deletion, explicit complete-state activation and deterministic native close | REC-01/05/07, OPS-TEST-02 plus SDK/FS gates; no expiry takeover of paused processes |
| REP-011; SDK-016; ARCH-004 | Origin/incarnation-bound recovery, new writable restore lineage | REC-03, RESTORE-01/02; explicit application config switch |
| ENG-003/006 | Single-snapshot export and live-data rebuild, with historical retention stated | BACKUP-05, ERASE-01; full streamed rebuild costs I/O/disk |
| ENG-004/007; REP-004/006 | No old allocator/reservation state migration; published-decision-derived IDs | MIG-06 and PROTO-11; old generated IDs remain data, not new allocation authority |
| SDK-001/002/014; REP-016 | Published-only recovery/reads, explicit unresolved identity and local-cache-failure health | REC-02/04, OPS-TEST-02; durable unknown-result handling remains essential |
| OPS-001 | Repository TypeScript migrations, authoritative checked history, explicit freeze/stage/validate/new-incarnation/activate/switch runner | MIG-01–14; tenant write downtime, no per-request auto migration or online/fleet migration engine |
| OPS-002 | Independent verified backup manifests and restore drills | BACKUP-01–05; backup storage/credentials/retention cost remains operator-owned |
| OPS-003 | Explicit declared blob closure and application outbox boundary | BACKUP-04, RESTORE-03; no arbitrary-URL backup or exactly-once external effects |
| OPS-004/005/006 | Scoped origin authority, host budgets, explicit freshness/health/redacted events | REC-03/06, OPS-TEST-01/02 and SDK gates; authentication/routing service is not supplied |
| ASS-003/004 | Versioned operational docs, preserved dated audit/counterexamples and resolution evidence | Packed examples + finding-to-test release ledger; no deleting evidence when a fix lands |

No finding is “fixed” by this document. The proposed representation either replaces its failing mechanism, narrows the product contract explicitly, or supplies a required gate. Implementation closure requires those gates to fail on the old failure shape and pass on the successor, followed by independent review of the supported deployment assumptions.
