# 30 — A three-language core and a TypeScript-only application log

Status: proposed 1.0 design, not implemented. This deliberately replaces incompatible 0.x APIs. It incorporates the full audit, including its preserved reproductions, and the representation-first brief. The objective is a small excellent database library, not a hosting platform.

## Decision

Keep two packages with a one-way dependency. The core exposes **create, open, snapshot, apply, inspect, close** for facts, laws, queries and local LMDB transactions. The log package exposes **create, open, snapshot, submit, resolve, inspect, close** for durable named application commands. Schema/query/change construction operates on values outside the database. Checkpoint, backup, restore and migration exist **only in the log package**.

The public core supports Rust, TypeScript and C. The public **`bumbledb-log` product is TypeScript-only in 1.0**: `LocalHistory`, `HostedHistory`, commands, backup/restore and migrations have no separately supported Rust or C SDK/header. Internal Rust still owns durable protocol evaluation, S3 authority, receipt handling and checked import; TypeScript owns ergonomic construction and explicit migration orchestration, not a second database state machine. Removing a public Rust log API removes a compatibility/packaging obligation, not the implementation's type safety or tests.

`LocalHistory` stores facts, decisions, receipts and its head attachment in one local LMDB transaction; `HostedHistory` uses supported S3 HEAD authority with a local LMDB materialization. They share command evaluation/outcome grammar without emulating an object store on local files. Applications needing only embedded facts/laws/query execution use the core without importing the log. No ordinary log success is a speculative local acknowledgement.

| Product | Supported public interface | Internal implementation |
| --- | --- | --- |
| `bumbledb` core | Rust, TypeScript, C | Rust/LMDB |
| `bumbledb-log` local/hosted history | TypeScript on qualified Node runtimes | Internal Rust protocol plus thin Node boundary |
| Log migrations and application integration | TypeScript migration files and explicit Node runner | TypeScript orchestration/pure transforms; internal Rust freeze, streaming import, validation and publication |

The API names and examples throughout this proposal are proposed contracts, not current exports. Rust-shaped log notation in other chapters describes internal types/protocol meaning, not a public Rust SDK promise. React Native/Expo inspires migration-file ergonomics only; no mobile, browser, Edge or Worker native runtime is promised.

The log's 1.0 unit of atomic change is the **whole tenant database**, including every affected relation, the read condition and the named receipt. Delete `commitSplit`; the former braid decomposition may inform analysis, but it is not an application transaction boundary. Multiple logical/remote writers remain supported and arbitrate through HEAD; tenant-wide atomicity does not require one permanently privileged client. The replacement publication protocol is specified by the replication chapters. Set union alone does not prove unrestricted deletes, keys, capacity constraints and read-dependent business actions merge without coordination.

This is not a new generic framework. LMDB already owns local transactions, snapshot isolation and durable pages. The thin owner layer adds only what LMDB cannot supply: remote publication certainty, language-safe resource ownership, bounded execution and immutable application command identity.

Creation is a separate explicit constructor. `open` of a missing or unreadable configured database never creates an empty replacement. `create` refuses existing authority and does not overwrite it; a retry after uncertain creation must validate the supplied stable creation identity and complete genesis, not silently adopt an unrelated database at that location.

## The small value vocabulary

| Value | Meaning and scope |
| --- | --- |
| `DatabaseId` | Stable logical database identity; not a path, bucket, tenant label or schema hash |
| `IncarnationId` | One history lineage; rewriting/rolling back into a different lineage requires a new incarnation |
| `SchemaId` | Canonical schema/theory identity; independent of database identity |
| `DatabaseIdentity` | The triple above, checked before cached facts or commands are adopted |
| `DecisionStamp` | `(seq, hash)` in one identity; every terminal decision advances it, including rejection/no-change receipts |
| `StateStamp` | `(incarnation, data_revision)`; advances only for a net change to application facts, and is the exact-state witness |
| `HeadRevision` | Changes on every authoritative HEAD CAS, including maintenance; never a read-dependent application witness |
| `CommandId` | `(receipt_epoch, request_id128)`; caller supplies stable request identity in an explicitly open epoch |
| `CommandDigest` | Hash of canonical command meaning, including identity, conditions and symbolic fresh references |
| `CommandRef` | Identity, command ID and digest; sufficient to resolve an uncertain submission without retaining a database handle |
| `EntityId` | Opaque 28-byte `(incarnation128, winning_decision_seq64, ordinal32)` identity |
| `FreshRef` | Command-local symbolic reference, never a provisional `EntityId` |

These are **log-layer values**, not core engine dependencies. None is interchangeable with LMDB generation, local catalog identity or a HEAD ETag. Maintenance changes HEAD without changing decision or data state; a rejected command changes decision identity without changing data state. Sequence arithmetic alone does not prove ancestry; the driver validates the corresponding chain.

IDs and tokens are small owned immutable values. A witness contains no database pointer or read transaction. A command can be stored/retried after its original snapshot and handle close. A command for one incarnation cannot accidentally execute against another. The core stores application IDs as ordinary fixed-width values/newtypes and does not issue log identities.

An `EntityId`'s embedded incarnation is **birth provenance, not current authority**. Writable restore/migration preserves existing entity bytes unless the explicit transform remaps them; queries and new-incarnation commands may reference those old-born IDs normally. Only new issuance uses the new incarnation. Do not apply command/witness/owner incarnation checks to an ordinary entity-value field. In particular, a branded entity string is schema/value-typed, not restricted to the currently open incarnation.

### Minimal standalone core interface

`Db::apply(CheckedDelta, ExpectedGeneration, WorkContext)` judges one immutable final-state change and commits an admitted state atomically in LMDB. `ExpectedGeneration` is `Any` or a core-local snapshot witness. It returns accepted/no-change, complete invariant rejection, moved witness, or infrastructure failure. A core-local witness binds catalog/store identity and generation, not a `StateStamp`. There is no named-command deduplication or remote-publication claim in this package. Concurrent logical callers are supported; LMDB serializes their actual local write transactions.

Core snapshots and compact/copy building blocks pin coherent LMDB content and generic metadata. The narrow integration path is `prepare_write(CheckedDelta)` → admitted `PreparedWrite` → `seal(HostChanges)` → `SealedWrite::commit/abort`. `HostChanges` contains bounded opaque host records/attachment under the same LMDB transaction, after the log knows the judgment and decision hash. Sealing permits no further application-fact mutation. The engine does not parse those records as receipts or import log types; this is not a public general KV framework. A rejected application candidate is aborted; the same exclusive owner prepares an empty application delta plus its log-owned rejection records. A raw snapshot copy is not exposed as a core backup product with retention/migration policy. Applications requiring stable retries, generated history identities, backup or migration select `LocalHistory` or `HostedHistory`.

## Command construction is synchronous, finite and owned

The builder accepts inserts, deletes, symbolic fresh references and one optional exact-state condition. `finish` consumes it and returns a sealed `Command`, or a typed build failure. Inputs are copied into Rust-owned canonical representation at their acceptance boundary. Finishing performs canonical sorting/deduplication and shape/type checking once, under explicit command size/work limits. Within **one** command, the canonical delta is `(add, remove ∖ add)` and application is `(state ∖ remove) ∪ add`: the exact same fact on both sides is add-wins, independent of builder call/fact iteration order. Resolving fresh references feeds the same final-state normalization in the core. Fresh declaration ordinals/result labels remain explicit command meaning; no graph-isomorphism/alpha-renaming framework is needed. This is not an add-wins CRDT across separate commands, which still follow authoritative decision order.

Generated schema APIs and dynamic APIs meet at the same checked canonical row type. External user codecs return input to that parser, not trusted persisted bytes. A generated fast path may construct the same private representation only inside the trusted crate boundary. A hidden public constructor is not a proof.

The builder itself does no I/O, acquires no database writer gate and invokes no replayable application-effect callback. Explicit input getters/iterators execute synchronously as bounded input ingestion; their side effects cannot be undone, but they are never automatically replayed. A convenience `build(fn)` may invoke a **synchronous** callback exactly once; thenables and escaped builder use are rejected. There is no automatic rerun for ID refill, conflict, cancellation or retries. An application performs HTTP calls before construction or after a receipt, never inside a database-owned callback.

Mutable buffers, `Buffer` slices, shared backing arrays and one-shot iterables cannot change an accepted command. In 1.0 shared-memory byte views must be copied from a stable input or refused; the SDK must not claim a coherent snapshot of a concurrently written `SharedArrayBuffer`. Iterator/getter exceptions terminate construction before dispatch. Builders cannot accept infinite input without consuming a configured work limit.

`FreshRef`s are allocated by builder ordinal, usable only in the same builder and only in compatible entity-reference fields. The driver resolves them using the winning decision coordinate without invoking user code. Successful receipts carry the stable mapping. Rejected receipts expose no allocated entity IDs. Offline application identifiers use an explicit `Bytes<16>` (128-bit) field or another chosen application domain; they are not silently confused with database-generated IDs. Public integer block reservation and premature escaped-ID promises disappear from the ordinary API.

The canonical digest is over the symbolic command, not candidate-specific generated IDs. A lost CAS can change the candidate's decision sequence without changing the caller's command identity. The protocol persists sufficient winning-decision evidence to derive the mapping from incarnation, sequence and ordinal; the client receipt/materialized receipt index expands it as needed. The remote decision need not duplicate the symbolic fact payload as a second resolved payload or store a redundant fresh-map structure.

## Conditional application intent

Two modes suffice for 1.0:

```ts
type Precondition =
  | { kind: "blind" }
  | { kind: "exact-state"; at: StateStamp }
```

An unconditional command is an immutable set effect judged against the eventual winning predecessor. It does **not** promise that arbitrary application reasoning from an earlier read is still valid.

An exact-state command checks its witness in the same authoritative transition that decides effects and receipt. If a net application-fact change intervened, it produces `PreconditionFailed`, even if subsequent changes restored the original fact set. The application may read again and create a **new command ID** for a genuinely revised decision. Reusing the old ID with altered input returns `CommandIdentityConflict`.

This coarse condition intentionally conflicts on any intervening net application change, even to an unrelated relation. Rejection/no-change decisions and maintenance do **not** invalidate it. Keep the three coordinates explicit rather than forcing unrelated meanings into a single generation. Do not add a generalized predicate/read-set engine in 1.0. The exact-state operation is already sufficient for serializable read-dependent single-tenant workflows.

Constraints remain essential: a condition protects the application's observation; schema laws protect the admitted state. Neither substitutes for the other. Cross-tenant actions use explicit application outbox/compensation patterns, not an implied global transaction.

## Submission and durable outcomes

```ts
type SubmitOutcome =
  | { kind: "decided"; receipt: TerminalReceipt }
  | { kind: "not-submitted"; command: CommandRef; error: DbError }
  | { kind: "outcome-unknown"; command: CommandRef; error: DbError }

type TerminalOutcome =
  | { kind: "committed"; changed: ChangeSummary; fresh: FreshMap; result: CommandResult }
  | { kind: "no-change"; result: CommandResult; fresh: FreshMap }
  | { kind: "precondition-failed"; expected: StateStamp; observed: StateStamp }
  | { kind: "invariant-rejected"; violations: Violations }

interface TerminalReceipt {
  readonly command: CommandRef
  readonly decisionAt: DecisionStamp
  readonly stateAt: StateStamp
  readonly outcome: TerminalOutcome
}
```

`CommandResult` is a bounded canonical selection of fresh-reference results and caller-declared scalar metadata, not an arbitrary host closure return value. Its grammar and digest contribution are fixed by the command codec. No closures, host objects or nondeterministic response calculations enter durable receipts.

Every terminal arm is a durable decision in the **log** package. Hosted history requires its authoritative HEAD CAS; `LocalHistory` requires its single durable LMDB transaction. Raw core `Db::apply` does not manufacture receipts. A no-change command still receives an identity and decision, so business-action retries do not depend on whether a set happened to change. A fresh-only command may return its published stable IDs as `NoChange { result, fresh }`; this issues identities but does not assert that any entity fact exists. It advances decision identity, not application state, and adds no allocator authority. Invariant rejection has complete statement-level diagnostics according to the engine contract; if the bounded diagnostic representation cannot be produced, return a resource failure before deciding, not a falsely complete rejection.

`NotSubmitted` is permitted only when the library knows **this invocation** dispatched no authoritative publication attempt. It does not prove that a prior/concurrent invocation of the same named command never committed. After dispatch, cancellation, timeout or a lost response may yield `OutcomeUnknown`. A read-back that proves this specific command's terminal receipt may instead return `Decided`. Finding equal counter/body bytes is not exclusive ownership evidence. Once remote publication is proven, a failed local apply still returns/retains that decided receipt with explicit unavailable-materialization health; it cannot become a rejection or unknown publication.

For a raw core local transaction, report any uncertainty permitted by LMDB's returned outcome conservatively, but do not promise `resolve` without the log layer. Do not reinterpret an I/O error as a semantic rejection.

The normal client retry rule is simple: **resolve or resubmit the identical sealed command under the identical command ID**. Never generate a replacement ID merely because a timeout occurred. The database guarantees deduplication within retained receipt epochs, not exactly-once external networking.

`resolve(CommandRef, options)` returns one of:

- `Found(TerminalReceipt)`;
- `NotRecordedAt { decision_at }`: absent in an open epoch at the captured frontier; a dispatched attempt may still win, so this is not proof of failure;
- `CommandEpochClosed`: closed epoch, no receipt for this ID, and no new command in it can execute;
- `ReceiptExpiredUnknown`: epoch retired; prior outcome unavailable and submission permanently refused;
- a typed operational error without an invented decision.

Same ID/different digest always refuses. Epoch closure/retirement are explicit maintenance actions, never local wall-clock expiry. A closed epoch retains existing receipts; a retired epoch cannot be resurrected. Closing a database owner does not retire receipt epochs.

## Public reads are published snapshots

```ts
type ReadConsistency =
  | { kind: "cached" }
  | { kind: "at-least"; at: DecisionStamp }
  | { kind: "latest" }
```

The log's `snapshot` returns a read-only snapshot carrying identity, actual decision and state stamps, and freshness provenance. `Cached` is potentially stale, but never speculative. `Latest` captures the authoritative target once, then catches up to that target under the supplied budget; a continuously hot writer cannot move the goal forever. `AtLeast` validates same-incarnation ancestry and catches up to the supplied coordinate. Neither method silently substitutes an older state when its requested condition is unmet. A core snapshot is simply the current pinned local LMDB state and has no remote consistency option.

An invalid, unrelated, future, retired or unavailable coordinate has an explicit result. Session tokens from HTTP are untrusted input: parse bounded canonical tokens before I/O, check database binding and charge any verification/replay work. The application authenticates and authorizes the caller separately. A token is not a permission grant and a hash is not an authorization signature.

`AtLeast(DecisionStamp)` means exact same-lineage stamp validation, not merely comparing sequence integers. If historical hash evidence has been pruned and cannot be established from retained receipt/root evidence within the budget, return `WitnessUnavailable`; do not silently claim an exact witness was verified. A separately named sequence-floor policy would be a different contract and is not smuggled into this method.

Snapshot facts, metadata, counters and stamps refer to the same physical read transaction. Ordinary log readers never point at a candidate LMDB generation awaiting publication. A snapshot acquired before a later successful command may continue observing the earlier published state; a snapshot requested `AtLeast(receipt.decision_at)` must meet that receipt or fail explicitly.

A snapshot has no mutation method and never exposes the private engine handle. There is no `replica.db.write` escape hatch. Candidate preview is not in 1.0. Copied result data and witnesses may outlive the snapshot; database-backed cursors may not silently do so.

## Query construction, execution and result ownership

Use one immutable schema-level `QueryTemplate`. It contains validated logical IR, not tenant rows, dictionary IDs or unbounded per-tenant memo state. A snapshot-bound `ExecutionSession` owns mutable planning/execution/cache state and is explicitly closable. It may be reused within its documented identity/snapshot scope; schema-level templates can be shared across same-schema tenants.

Prepared statistics are an optimization, not a correctness premise. A trimmed execution session can rebuild its optional caches. Physical catalog identity remains separate from logical database identity so remount, migration and a rebuilt materialization cannot reuse stale physical ordinals or cached row references. The selected core stores inline text; there is no new dictionary-lifetime framework here.

`execute(template, parameters, context)` returns a sealed `CompleteResult`, possibly backed by temporary LMDB scratch. Any execution failure publishes no result. Mutable reused out-parameters are not the default API. A lower-level reusable buffer, if retained internally, is cleared atomically on **every** error family. No aggregate binding means no group: a global aggregate over empty input returns an empty answer set, not an invented zero/NaN row.

`CompleteResult.collect(maxBytes)` materializes a bounded owned array/vector while leaving the result owner intact. It must refuse before allocating more than its budget. `CompleteResult.intoCursor(pageBytes)` **consumes** the result owner and transfers its sealed backing storage into one cursor carrying result identity/continuation position. Rust moves the value; Node/C spend the source handle. There is no clone/shared-cursor API in 1.0. Closing the spent source cannot close the moved cursor. This is not an unbounded `scan(): Fact[]`. Page failures may interrupt transport of a complete result; they cannot turn an unfinished query prefix into a claimed complete set. An abandoned cursor closes its own scratch resource after active page operations drain; copied result values remain independent.

The engine uses one RAM-to-temporary-LMDB ordered-map abstraction for needed scratch, not a new external-sort/hash storage engine. Ordering here describes bounded physical LMDB keys; long logical keys use exact-checked candidate buckets, and answers remain unordered sets. Large data does not require large result arrays or resident relation images. Exact answers must agree with and without optional caches, at high and low memory, including deterministic floating aggregates.

No database-size limit is inferred from RAM. `workingBytes`, `spillBytes`, `outputBytes`, work units, deadline and cancellation describe **this operation's** envelope. Logical database bytes, mapped virtual address space, physical disk and RSS are different measurements. An operation can be too costly for the supplied policy without the database being unsupported.

## Proposed TypeScript use

This example is contract pseudocode, not a claim that these names currently compile. It assumes application authentication has already produced a trusted database binding and that `requestKey` is retained across retries.

```ts
await using db = await HostedHistory.open(source, theory, runtimeOptions)
await using snap = await db.snapshot({ consistency: { kind: "latest" }, ...requestOptions })
await using rows = await snap.execute(balanceQuery, { account }, queryOptions)
const previous = (await rows.collect({ maxBytes: 64_000 }))[0]

const draft = Command.builder(theory, db.identity, {
  id: { receiptEpoch: db.receiptEpoch, requestId: requestKey },
  precondition: { kind: "exact-state", at: snap.stateStamp },
  limits: commandLimits
})
const entry = draft.fresh(LedgerEntry.id)
draft.delete(Balance, [previous])
draft.insert(Balance, [{ account, amount: previous.amount - debit }])
draft.insert(LedgerEntry, [{ id: entry, account, amount: debit }])
draft.returning({ entry })
using command = draft.finish()

const outcome = await db.submit(command, requestOptions)
switch (outcome.kind) {
  case "decided": return encodeReceipt(outcome.receipt)
  case "not-submitted": return reportNotSubmitted(outcome)
  case "outcome-unknown": return reportPending(outcome.command)
}
```

Async database work runs outside the JS event loop. `using` releases local sealed command values; `await using` closes owners, snapshots, completed results (which may own disk scratch) or runtime operations requiring a drain. Disposing a command only releases its owned bytes; it cannot cancel a command already submitted. The native runtime retains its own immutable reference until the operation ends. An HTTP adapter preserves an already decided receipt separately from a later cleanup/close error; standard disposal failure cannot be reclassified as command rejection. Use the explicit close report when the host must return known publication plus unavailable local-health detail.

A single-owner embedded convenience adapter may expose blocking `snapshot/apply/execute` for the core and `snapshot/submit/execute` for local log history. It delegates to the same implementation and must not be the default in a shared Node request loop. No sync callback is silently permitted to become async.

## Floating values and cross-language fidelity

`f64` is a real schema scalar in 1.0. TypeScript uses `number`, Rust uses the engine's canonical `F64`, and C uses `double` at ingestion. Integer fields remain `bigint`/`u64`/`i64`; there is no implicit integer-to-float coercion, even for small values.

- All NaN payloads become `0x7ff8000000000000`; `-0` becomes `+0`.
- Set equality, keys and hashing use canonical value equality. NaN equals NaN **as a database value**, independently of JS `===`.
- Relational total order is `-Infinity < finite < +Infinity < NaN`. `is_nan` and `is_finite` are explicit predicates.
- Arithmetic is the engine's deterministic binary64 contract: round-to-nearest ties-to-even, gradual underflow, no implicit FMA/reassociation. Canonicalize after each arithmetic node, so `1 / neg(0)` is `+Infinity`. The 1.0 arithmetic roster is `+`, `-`, `*`, `/`, unary negation, explicit casts/comparisons/predicates and documented aggregates; no automatic transcendental expansion.
- `sum`/`mean` use the engine's mergeable exact accumulator and one final rounding; partition order, disk spill and worker count cannot change result bits. `min`/`max` follow the declared total order.

JS built-in `JSON.stringify` is **not** the database value wire codec: it loses infinities/NaN and does not encode BigInt. The supplied HTTP/export value codec uses schema-tagged canonical scalars; every `f64`, including finite values, uses `{"$f64":"7ff8000000000000"}` with its own canonical 16-lowercase-hex-digit bit image. Integer values are canonical decimal strings, bytes use one strict encoding, and `EntityId` uses 56 lowercase hex digits. This keeps human-facing JS ergonomics without inventing a second numeric semantics in TypeScript. Decoders reject malformed widths, unknown tags, noncanonical representations and wrong-schema values.

Host float equality is not a database predicate builder. Documentation must show `field.eq(NaN)` rather than evaluating `NaN === NaN` in application code. Returned JS values are canonicalized, so `Object.is(value, -0)` is false. Do not claim decimal financial arithmetic from binary64; monetary examples retain scaled integers.

## Errors are data with one vocabulary

All core languages carry the same stable error code, operation, retry classification and bounded diagnostic detail. The TypeScript log extends that vocabulary with its protocol-specific outcomes; it does not create corresponding public Rust/C log APIs. Families: `InvalidInput`, `Misuse`, `Incompatible`, `Unavailable`, `ResourceLimit`, `Cancelled`, `DeadlineExceeded`, `Corruption`, `Internal`. Specific codes distinguish foreign identity, stale/closed handle, command mismatch, unsupported artifact, local lock busy, insufficient disk and unmet read consistency. Host authentication failure belongs to the host boundary, not a forged engine semantic refusal.

Core Rust uses enums/results, core C uses numeric stable codes plus owned error detail, and TypeScript core/log operational methods return their documented tagged outcomes and typed errors. Human messages are explanatory and not a stable retry API. Receipt decisions remain distinct from errors. `OutcomeUnknown` is a submission certainty arm, not merely `retryable: true` on an exception.

Detailed violations and query parameters contain tenant data. Default logs include IDs, codes, work/cost counters and redacted causes, not facts, request bodies or credentials. Debug payload capture is an explicit host action with its own retention policy.

## Compatibility and migration boundary

Opening an incompatible 0.x store/remote history with 1.0 must refuse with a precise import requirement; never silently reinterpret existing scalar IDs as 28-byte identities. The **log-package** migration tool reads the old format read-only, exports live canonical facts and required application ID mappings, validates them against the destination theory, and publishes a new-incarnation destination. It does not weaken schema identity to get an open to succeed. The core contains no backup/migration API, command identity or lineage policy.

Schema/data migration is explicit application downtime/cutover in 1.0: freeze source admission, settle/report unknown commands, pin the source restore point, transform/import, verify, then change the application's configured binding and explicitly activate the completed target. Chapter 33 defines ordered checked repository-local TypeScript files, an explicit `migrate()` runner, immutable applied history and Next.js/Alchemy integration. The log package provides these finite primitives; it does not provide a fleet coordinator, automatic online dual-write engine or generalized migration language. A pure migration transform may repeat after an interrupted staging attempt, but an ordinary business-effect callback is never replayed. Rollback after target activation requires explicit history/effect analysis, not pointing old session tokens at unrelated rows. Retention in 1.0 means the current recoverable root plus explicitly named restore points, not an automatic time/PITR promise.

## Required API gates before 1.0

Every row is a required test family, not work this proposal has already executed.

| Gate | Exact obligation |
| --- | --- |
| API-01 Canonical ownership | Mutate array/slice/Buffer after each builder call, during submit and CAS retry; one-shot iterators; getter throws; escaped/spent builders; cross-builder fresh refs. Permute duplicate add/remove calls: same-fact add wins within the command; separate commands remain ordered. Local and replayed canonical commands remain identical or construction fails before dispatch. |
| API-02 Closed ingestion | Downstream custom codecs, invalid bool/interval/fixed-width/UTF-8/relation/float images never admit corrupt state. Generated and dynamic APIs have the same accepted domain. |
| API-03 Conditions | Two stale read/decrement commands: only the permitted exact-state decision applies. Blind effects retain documented set semantics. Maintenance/no-change/rejection do not move a `StateStamp`; every terminal receipt moves `DecisionStamp`. |
| API-04 Named retries | Crash before/after dispatch and before response; duplicate simultaneous submissions; same ID/different digest; live-handle continuation after unknown; closed/retired receipt epochs. No repeated business effect or silently refreshed request ID. |
| API-05 Published reads | Pause candidate application/PUT/CAS at every point, then succeed, lose, reject, cancel or crash. Every ordinary observed snapshot belongs to published history, not merely the converged final state. |
| API-06 Fresh identities | Every placeholder maps once in a committed/no-change receipt, including fresh-only commands, through response loss/restart/CAS loss; rejected commands leak no ID. Distinct winning coordinates/ordinals never overlap; ordinal/sequence exhaustion refuses without wrapping. Restored old-born IDs remain usable ordinary values while old command/witness scopes refuse. |
| API-07 Output atomicity | All query, bind, overflow, foreign-template, decode, cancellation and resource errors expose no current partial result. Success/error/success reuse and page interruption preserve result identity. |
| API-08 Identity | Same schema/equal sequence/different data, changed bucket/prefix, case aliases, reborn namespace, remount and migration cannot reuse cache, receipt, witness or mutable execution state across identity boundaries. |
| API-09 Float boundary | Full scalar golden corpus including every sign/exponent class, subnormals, sNaN/qNaN payloads, infinities and signed zeros; JS/Rust/C/wire/query/key/aggregate results agree bit-for-bit after canonicalization. |
| API-10 Bounded work | Tiny command/query/catch-up limits at every allocation/growth boundary; cancellation before queue, during I/O/compute/spill/finalization. No hidden native work continues after a reported completed cancellation. Ambiguous publication remains unknown. |
| API-11 Large database | Identical answers and receipts with database > RAM, above the old 32 GiB map cap, caches off, forced LMDB scratch, small result pages and low resident-memory allowance. No artificial logical-size rejection. |
| API-12 Public examples | Core Rust/TS/C and log TypeScript examples compile and run against the actual packaged 1.0 artifacts; no public Rust/C log export is advertised. Compile-fail tests reject mutation through snapshots and foreign typed references; runtime tests repeat the same attacks through dynamic input. |

## Audit disposition

`SDK-001/003/008/014/015`, `REP-016/020`, `ENG-001/002/004/005/007/008`, `QRY-001/002/003` and architecture gaps `ARCH-001/002/003/004/005` are addressed by new contracts, not claimed fixed. `SDK-009/010` and `REP-015` also require the runtime machinery in chapter 31. `SDK-016` requires both this identity vocabulary and the mount protocol. Float semantics are an added 1.0 obligation, not an audit finding. Original counterexamples must remain and be inverted into regressions; deleting their old API does not delete the obligation.
