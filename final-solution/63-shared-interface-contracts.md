# 63 — One owner for every shared contract

This is a cross-lane handoff ledger, not a second API specification. Exact
semantics/signatures remain in the linked chapters. Current incomplete source
is not automatically the chosen API. P00 coordinates declarations; the named
producer owns their meaning and communicates changes to all consumers.

A contract is ready when a checked-in source declaration or finite format table
exists, its ownership/error/cost rules are stated, and consumers acknowledge the
same symbols. In F0–F2 “ready” means source-reviewed, **not compiled or tested**.
No fake implementations, dummy handles, `as any`, unsafe Send or temporary public
Promise adapters count as implementation. A declaration-only module is a
temporary dependency handoff and must be completed before F2 ends.

## C01 — Values, typed schema, scalar expressions and identities

Producer P01; consumers P02/P03/P04/P07/P09/P10/P11/P14. Sources begin in
`crates/bumbledb-theory/src/`, core canonical/schema/scalar modules and the
core-owned TS metadata. P07 owns TS authoring syntax; P01 owns denotation and
canonical native validation. Read 10, 11, 33, 34, 35.

- Canonical F64 NaN/zero and total order, strict accepted bytes, explicit casts,
  fixed-width payload, dense interval endpoint rules and ordinary Id128.
- One schema-bound row/scalar parser and canonical schema representation.
  Full bytes decide equality; local row identity is not an application scalar.
- Freeze the finite ScalarExpr roster and field/parameter typing together.
  The preserved Rust Var/Literal/arithmetic/cast/isNaN/isFinite draft is input,
  not permission to add arbitrary JS evaluation. Core owns TS ScalarExpr and
  migration imports it; no duplicate migration expression interpreter.
- Document pure authoring failures versus effectful admission/codec errors.
  No hidden schema hashing or bulk row walking at module import/construction.

Handoff: exact value/IR type declarations, tags, canonical example rows and
schema examples, scalar error table, shared TS generic signatures. Changes to
this contract notify codecs, macros, Lean, Node and migration owners together.

## C02 — Owned changes, budgets and errors

Producer P01 (ChangeSet/error semantics) + P06 (native accounting); P00 owns
central shared files. Consumers all operational lanes. Read 10, 12, 30, 31, 35.

- Immutable schema-bound ChangeSet uses exact same-command dedup/add-wins;
  retains accepted native data, never caller arrays or a new log row recorder.
- WorkContext charges input, preparation, queue retention, execution, scratch
  and conversion; deadline/cancellation propagate to actual work. Explicit
  ownership transfer carries reservations; cleanup has reserved capacity.
- Public TS ExecutionPolicy uses bigint counters and Duration.Input; no second
  AbortSignal. Zero is zero, not unlimited. Methods capture acquired runtime.
- Core DbError and log ProtocolError have direct Effect tagged/reason errors;
  no @superbuilders/errors anywhere in maintained code/manifests/locks. Use the
  pinned Effect 4 Schema.TaggedError contract in 35 for the final boundary;
  earlier Data.TaggedError scaffolding does not freeze a second error family.
  Preserve arbitrary causes without exposing sensitive payloads by default.
- Domain admission/receipt outcomes are A; operational errors are E except the
  explicit certainty unions below. Native panic faults ownership, not a
  retryable semantic rejection.

Handoff: owner/clone/close rules, policy conversion, code/reason roster with
payload bounds, complete success/error/cancel transfer table.

## C03 — Candidate judgment and grouped measures

Producer P01; consumers P02/P03/P11. Read 10, 13.

Judge a canonical proposed final state before unique index installation can
discard competing rows. Completed rejection returns all violated statement IDs
with bounded examples, or an explicit resource failure. One exact nonnegative
grouped-measure family normalizes supported aliases; parent with no children
means zero, no parent is distinct, and zero weight is not absence. Count is unit
weight, not bag multiplicity. No float capacity or occupancy subsystem.

Handoff: candidate iteration/index interface and lifetime, complete judgment
sum, stable statement identity, overflow behavior and model examples.

## C04 — LMDB owner, snapshot and private candidate

Producer P02; consumers P01/P03/P04/P05/P06/P09. Read 10, 20, 31, 32 and
implementation/05–06.

- One owner, coherent actual RO transaction for rows/generation/attachment,
  distinct owned versus borrowed snapshots, declared Send/Sync/affinity.
- Native create versus open are distinct; validate family/schema/store/origin
  before adoption or cleanup. Elastic map grows under a safe transaction gate.
- Candidate prepare/admit → opaque adjunct seal → commit/abort is the only
  log integration. Seal cannot alter judged application facts. LocalHistory
  facts/receipt/attachment commit atomically, including no-op/rejection metadata.
- MAP_FULL before remote publication aborts/resizes/replays immutable native
  work, not user callbacks. Failed seal dispatches nothing. After known hosted
  publication, local fault cannot change the remote decision.
- A writer across remote await stays on its owning worker. No unsafe lifetime
  erasure or movement of !Send prepared/read/write capabilities.

Handoff: actual Rust signatures/lifetimes; ownership transition table; snapshot
cursor interface; nonblocking runtime integration points and map-full outcomes.
P06 must review this before designing affinity; a generic Send closure queue
alone is not sufficient.

## C05 — Query IR, execution and completed results

Producer P03; consumers P07/P09/P10/P11/P14; P06 bridges managed resources.
Read 11–13, 34, 35.

Typed nonrecursive stages include computed/aggregate output; logical names do
not force materialization. Positive linear recursive feedback remains finite,
projection-only and free of negation/aggregation/value creation. Preserve set
binding grain, dedup, empty groups, exact reductions and failure boundaries.
One charged RAM→temporary-LMDB scratch facility backs all intermediate owners;
warm Free Join/selective probes remain primary. CompletedResult publishes only
after all evaluation/finalization succeeds.

Rust consumes into_cursor; TS pages is a one-shot Stream of owned page arrays,
not row Effects or an executing-query stream. collect has a total cap and a cap
failure leaves the sealed backing available; first stream execution consumes
it. No borrowed LMDB data in JS. Release scratch on all terminal paths.

Handoff: IR roster and output type inference, complete result/transfer state,
cursor lifetime, empty/nonfinite/overflow examples, optimized/fallback oracle
fixtures. Migration must use this evaluator, not reimplement scalar arithmetic.

## C06 — History authority and certainty

Producer P04; consumers P05/P06/P08/P09/P11/P12. Read 20, 21, 22, 30, 35.

LocalHistory is one LMDB transaction; HostedHistory uses one S3 HEAD over
immutable decisions/checkpoint + bounded tail. Freeze distinct head revision,
decision identity and application-state revision. Commands retain concrete
Id128s and the core ChangeSet. Receipts resolve stable refs, including no-change,
rejection and failed preconditions. Receipt lookup precedes new admission;
retired epochs permanently refuse execution. Frozen/Deleted are authority
states, not local flags.

submit returns decided(receipt,localHealth), not-submitted(ref,error), or
outcome-unknown(ref,error) in A, E=never. Interrupt/defect/finalizer Cause remains
possible and is never rewritten to not-submitted. Resolve proves only its
captured authoritative evidence. AtLeast needs actual ancestry, not sequence
comparison. Reads expose published QueryReader only, never writable core Db.

Handoff: native state/command/outcome/ref types, transitions including all
failure points, exact retry ownership, retained evidence and bounded reports.

## C07 — Complete backend operations and local exclusion

Producer P05; consumers P04/P06/P09/P12. Read 20–22, 31 and frozen handoff.

There is one implementation of each filesystem operation, including its entire
read/compare/write/flush/rename/cleanup critical section. TS delegates through
the shared native executor; never carry a JS-held lock through await. Remove
numeric token/head CAS and age-based temp sweeping. Directory owner exclusion
is kernel-held, before cleanup and released last. Object mutation and directory
ownership are distinct scopes even if they use the same kernel mechanism.
S3 conditional responses distinguish known failure, known publication and
unknown transport result; emulator green is not S3 qualification.

Handoff: five operation signatures, bounded body ownership/Accounted transfer,
conditional-result grammar, stable lock namespace, durable ordering and refusal
rules for hostile/symlinked paths. Retain the deterministic mixed-fleet regression.

## C08 — Checkpoints, roots, GC and admin transitions

Producer P05, with P04 owning authority fields and P09 migration transitions.
Consumers P08/P09/P12/P14. Read 21, 22, 35.

One coherent streamed snapshot and validated suffix; no quiet-window restart.
Epoch barrier plus exact-parent reference introduction protects hosted roots.
LocalHistory does not inherit remote epoch machinery. Backup is independent
verified bytes, not an active-store pointer. Restore creates a new writable
incarnation. Deleted authority has no active recovery root; explicit retained
roots remain honored. All mutation progress retains discovery evidence.

Admin operations use their existing operation/root/barrier identities and the
chapter 35 completed/not-started/outcome-unknown sum. Do not add a generic admin
journal or manufacture command receipts for maintenance. Read-only verification
uses typed E. Handoff includes exact snapshot/root framing, authority mutations,
resumption states, operation certainty and ownership of staging paths.

## C09 — One Node runtime and managed lifecycle

Producer P06; consumers P07/P08/P09/P10/P13. Read 31, 32, 35.

Core NativeRuntime Context.Service/Layer owns one bounded native registry.
Shared exact addon identity, kind/generation/owner checks, bounded operation and
resource registration, real worker affinity, no AsyncTask bypass. Close stops
admission, drains, drops DB and cleanup, releases directory lock last; concurrent
close joins. Incomplete/failed retains Closing accounting. Distinct cache
borrows release only themselves; opening operations count during shutdown.

Effect finalizers surface structured CloseFailure in Cause for incomplete or
failed close. Private callback adaptation installs finalizers atomically with
ownership transfer. The app owns Effect runtime; library never runs a hidden
per-operation runtime. No public low-level handle, unmanaged escape or duplicate
addon. Marshal in charged chunks that permit real Node event-loop turns.

Handoff: exact private entrypoint/version handshake, owner/operation messages,
affinity design, cancellation/late completion transfers, CloseReport and
diagnostic limits. Only P06 edits native bridge hubs after P00 assigns them.

## C10 — Public TS aesthetics and porous composition

Producer P07; P08 owns only log additions. Consumers P10/P13. Read 30, 34, 35
completely and pinned Effect 4 docs/source, not Effect 3 examples.

Chapter 35 is the signature roster and A/E/R authority. Schemas/query/intent are
pure metadata; execution is Effect-only. One core QueryReader, ChangeSet,
ScalarExpr, codec, result and execution policy. Log imports those actual values,
not structurally similar brands, parsers or builders. Small fixed-size fallible
parsers use Result; absent lookup uses Option. Package barrels expose no
Promise/sync/disposal twin. Errors remain direct core/log tagged reasons.

Handoff: authored public declarations and same-schema Rust/core-TS/log-TS
consumer fixtures. Validate them against exact pins in F3. No first-class float
or query operator may exist solely in the native kernel while public SDKs lag.

## C11 — Generated migration plans and repo history

Producer P09 for native codec/executor, P10 for pure intent/diff/repo writer.
Consumers P04/P08/P13. Read 22, 33, 35.

Before either implements beyond draft types, jointly declare the finite plan
roster, source/target coverage, ordered validation, stable label versus digest,
schema/plan/prefix domains and authoritative applied-history record. P09 owns
canonical native framing/hashing; P10 calls it. Metadata has one canonical
schema source. Rename/backfill/destructive ambiguity requires declarative intent,
not guessed data loss, authored migration callbacks or a second scalar DSL.

Native execution freezes source, builds one final target with necessary
intermediate checks, verifies admission, returns still-frozen ReadyToSwitch.
Activation is explicit. Abort fences delayed genesis/activation durably before
thawing the matching source. Every operation has a stable ref before dispatch;
status resolves uncertainty. Repo generator/checker and runtime consume the
same canonical data; never trust local history as proof of application.

Handoff: exact plan JSON/data and canonical byte examples; intent-to-plan
ambiguity table; native entrypoints; replay/divergence rules; cutoff states.
The stopped TS migration files are incomplete drafts, not a usable generator.

## C12 — Formats, artifacts and measured choices

P00 coordinates P01/P02/P04/P05/P09/P13/P14. Read 32, 40, 41, 64.

Core/log/command/snapshot are distinct v1 families with layout counter 1;
recognizing integer 1 alone is forbidden. Physical bytes remain provisional
until final-phase probes and golden selection. Default is 16-byte exact-checked
local fingerprints and 32-byte authoritative BLAKE3. AEGIS is a measured
candidate, not an alternate CPU-dependent format or promised speedup.

One exact-version Node artifact houses core and internal log. Rust core stays
log/AWS-independent. Pins include Effect 4.0.0-rc.112 unless explicitly requalified.
P13 owns artifact roster/build design; P00 owns lock/version hub edits. Source
packing must not mutate checkout. Production credentials/deployment/publication
require explicit authority beyond this implementation prompt.

Handoff: family/tag/length/digest tables, selected physical layout after F3
probes, toolchain/platform pins, immutable staging manifest and evidence paths.
