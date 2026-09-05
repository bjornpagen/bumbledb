# 62 — Dispatchable implementation packets

These are broad implementation assignments, not sequential microtasks. Use the
pipeline in [61](61-orchestration-and-dependency-graph.md). Every packet includes
implementation, authored tests, deletions, source review and a concrete handoff.
No agent executes verification before the global F3 barrier. No agent commits
or pushes. Default file domains below become exclusive claims at dispatch;
shared hubs are assigned explicitly, never edited by competing agents.

Every packet reads 00/01/02/61, [the frozen handoff](../implementation/06-frozen-implementation-handoff.md),
its C-contracts in 63, and all of its chapter 50 findings and chapter 70 child
gates. Read full assigned chapters, not only this summary. All tests below are
minimum examples, not replacements for chapter 70's complete inventory.

## P00 — Orchestrator and integration owner

Read: the entire proposal/audit and implementation/04–06. Own: campaign status,
packet assignment, shared source hubs, manifests/locks/toolchain/CI integration,
root docs, release-results ledger, final review/verification/commit/push. May
delegate a hub file exclusively after recording the transfer.

Shared hubs include Rust crate lib.rs/mod.rs/error/tag rosters and Cargo files;
TS index/native/tag/error barrels and package/lock files; runtime bridge
registration in ts/crate/src/lib.rs; script entrypoints and generated fixture
rosters. This is an integration role, not an excuse to serialize all work:
publish requested exports/declarations promptly while consumers implement.

Deliver:

- Real ownership map and contract registry using C01–C12, with no unclaimed
  cross-lane dependency. Freeze symbol rosters, not fictional behavior.
- Every one of the 68 audit IDs and every parent/child release gate mapped to a
  primary packet and an independent reviewer. Keep release-results as the sole
  qualification ledger; packet statuses are not duplicate green claims.
- Source integration, all selected deletions, cross-language exhaustiveness,
  final scope/performance/authority decisions and F2 barrier record.
- Final F3 evidence, exact candidate/artifact identity, honest blocked gates and
  authorized Git handoff. No version bump/tag/publication by inference.

Done means all packet contracts have real implementations and final evidence
supports every claimed status. A green compiler alone is not completion.

## P01 — Canonical values, schemas and final-state admission

Read 10/11/13/34/41. Contracts C01/C02/C03. Default writes:
`crates/bumbledb-theory/src/`, core `canonical/`, `encoding/`, `interval/`,
`schema/`, `changes/`, `scalar.rs` and their adjacent tests. P00 owns root module
wiring. P03 owns exec numeric kernels and query IR. Coordinate candidate/index
interfaces with P02; no concurrent edits to storage judgment files.

Implement canonical values and strict schema-bound external decoding, complete
F64 domain/casts/intervals, application Id128, exact tuple equality and
same-command normalization. Final-state admission must see competing candidate
rows before unique-index installation. Normalize exact grouped measures and
supported aliases; preserve empty-parent/zero-weight and all actual domain laws.
Close safe raw Fact/interval constructor trust holes. Remove fresh reservation
semantics without removing app-level key checking.

Author tests for forced fingerprint collisions, forged safe codec success,
invalid scalar bytes, long keys/values, all competing-row permutations, complete
statement diagnostics, NaN/zero/inf/casts/interval boundaries, grouped empty/zero
measures and strict refusal of unsupported float capacity/fixed intervals.

Delete unchecked public constructors, hash-as-equality assumptions, placeholder
entity authority and cosmetic weighted spelling bans. Hand P02 candidate types,
P03 typed scalar semantics, P07 schema/field roster, P09 canonical schema/scalar
codec and P11 denotation examples. Cost obligation: canonical ownership and
normalization do not multiply retained row copies or embed a 272-byte float
accumulator in integer-only groups.

Complete when every accepted public value has the same meaning in schema,
storage, query and codecs, and all semantic outcomes remain distinct from I/O.

## P02 — Physical storage, owned snapshots and elastic LMDB

Read 10/12/20/31/41 and implementation/05. Contracts C01–C04/C12. Default
writes: core `storage/`, `verify_store/`, `api/db/`, owner/snapshot modules,
physical persistence tests. P00 assigns top-level API files individually. P01
owns canonical row/judgment semantics; P06 owns Node runtime, not LMDB internals.

Implement live tuple text ownership, exact collision buckets and long-key-safe
indexes, local physical row IDs, coherent snapshot identity/metadata, safe
owned snapshot lifetime, geometric map growth and transaction coordination.
Distinguish virtual map, populated file, resident memory and disk availability.
Implement private candidate prepare/admit/opaque-seal/commit with no writable
escape, including metadata-only decisions and all map-full phases. Declare
thread/lifetime constraints before P06 session design.

Author crash/resize/reader/owner schedules, candidate visibility, old-family
refusal before cleanup, snapshot metadata/row coherence, forced-collision exact
lookup and map-full-before/after-seal regressions. Author large-data fixtures
with P14; execute only F3. Storage layouts remain provisional until final probes.

Delete immortal dictionary, 32 GiB policy, separately committed generation,
ordinary no-sync escape and incoherent snapshot copying. Coordinate directory
lock ownership with P06/P05; there must be one authoritative owner, not nested
independent locks. Hand P03 cursor/snapshot access, P04 atomic adjunct and P05
snapshot export. No unsafe lifetime extension to make borrowed reads “owned.”

Complete when core is log-independent and safely supports real disk-backed
state beyond memory with owned snapshots and exact admitted transactions.

## P03 — Free Join, derived relations and bounded query execution

Read 10–13/34/40. Contracts C01–C05. Default writes: core `ir.rs`, `ir/`,
`plan/`, `exec/`, `image/`, `api/prepared.rs`, `api/prepared/`, result/scratch
modules. P00 assigns shared work-policy files; P01 owns scalar AST/denotation.
P11/P12/P14 own independent benchmark/oracle directories by explicit split.

Complete preserved computed-find/sink wiring and whole-query numeric guard.
Generalize nonrecursive relation stages; do not leave Interior.rules projection
only. Keep positive linear projection-only feedback and prove finite predecessor
boundaries with P11. Preserve binding grain, dedup, group emptiness, stage errors
and exact scalar rounding under optimizer rewrites.

Build one charged RAM/temporary-LMDB scratch map and complete cursor fallback
across distinct/grouping/stages/recursion/results. Retain warm Free Join/SIMD and
selective probes; no full-relation mandatory image or post-write rebuild tax.
Bound prepare/Boolean structure/parameters/growth, not just result delivery.
Finish exact widened integer and F64 sum/mean with independent final rounding;
publish only complete owned results, consuming cursor ownership safely.

Author warm/scalar/cursor/forced-spill equivalence, partial-output failure,
overflow, nonfinite arithmetic, recursive boundary refusal, reused plans after
writes, limited working memory and every operator's disk path. Collaborate on
independent oracles without copying production numeric/equality helpers.

Delete mandatory DNF expansion, projection-only nonrecursive walls, partial
published answers, unbudgeted high-water state and order-sensitive reductions.
Hand P07/P09 exact IR/operators/output types and P06 result/session lifetimes.
Complete means full operator capability matrix works under the selected model;
a numeric kernel or one fitting join is not the packet.

## P04 — One internal durable history machine

Read 02/20–22/30/35. Contracts C01/C02/C04/C06/C07. Default writes: internal
`crates/bumbledb-log/src/history/`, `writer/`, command/receipt/authority modules
and their direct tests. P00 assigns existing top-level log files; P05 owns stores,
checkpoint/recovery/GC, P09 migration. No public Rust log compatibility surface.

Implement LocalHistory's same-transaction facts/receipts/attachment and
HostedHistory's one-HEAD publication over immutable decisions. Use three distinct
coordinates, concrete IDs, exact-state precondition, retained named receipts
and durable no-op/rejection. One bounded immutable attempt crosses catch-up,
prepare, seal, publish and materialization; losing candidate is never readable.

Preserve certainty through delayed/lost responses, interruption, next command
on a live handle, subsequent decisions, checkpoint/retirement and local failure
after publication. Reads capture finite tips; AtLeast checks ancestry. Receipt
lookup works before Frozen/epoch admission refusal. Closed epochs cannot execute
an absent request again. Frozen/Deleted authority coordinates with P05/P09.

Author independent trace schedules with P11/P12, including duplicate IDs/digest
conflicts, ABA/state witnesses, key/capacity multiwriter counterexamples,
no-change state stamps and result loss after known publication. Do not run yet.

Delete braids/vector floors/split commits, issued-ID allocator/writer fencing,
callback replay and writable replica escape. Hand P08/P06 outcome/ref/read
contracts and P05/P09 exact authority mutation protocol. Complete means both
backends expose the same selected command semantics without LocalHistory
simulating an object store or JS duplicating the machine.

## P05 — Backends, checkpoint/retention, recovery and independent backup

Read 20–22/31/41 and frozen CAS failure. Contracts C04/C06/C07/C08/C12.
Default writes: log `store/`, backend adapters, snapshot/checkpoint/GC/recovery/
backup/restore/erasure modules and direct native tests. P09 owns migration, P04
authority types, P08 TS wrappers. Allocate new module files before parallel edits.

Finish bounded complete FS operations through Rust and the shared runtime;
remove TS CAS authority with P08. Preserve deterministic paused-read/Rust-CAS
regression and hostile mutation-lock cases. Qualify S3 conditions/uncertainty,
durable write ordering, object kind/epoch/length/digest checks and finite I/O.

Implement coherent streamed checkpoints, bounded validated suffix rebase,
hydration/activation and named roots; checkpoint progresses under writes.
Implement durable epoch barrier/mark/sweep with exact-parent references and
resumable deletion evidence; late old uploads become orphans, not publishable
dependencies. LocalHistory export/restore points stay simpler. Backup is
independent complete bytes and verification; restore yields new incarnation.
Erase uses Deleted authority and preserves explicitly retained roots correctly.

Author process/crash/lost-ack/corruption/ref-introduction schedules and backup
restore without origin/cache. Author real S3 and power-failure lanes for F3,
not claims based on emulators. Coordinate P09 staged targets/cancellation fences.

Delete age leases/temp sweep/PITR default, body/fence split authority, scratch
deletion authority, token history scans, quiet-window checkpoint restart and
partial-hydration-as-empty. Cost: bounded buffers/tail/progress and current-work
cost rather than lifetime token-count cost. Complete means recovery remains
discoverable and no published/retained dependency can be prematurely collected.

## P06 — Native scheduling, ownership, affinity and private Node bridge

Read 10/12/20/31/32/35 and implementation/05–06. Contracts C02/C04–C09.
Default writes: `ts/crate/src/runtime.rs`, `runtime/`, `runtime_wire.rs`, bridge
owner/session/operation modules. P00 explicitly transfers `ts/crate/src/lib.rs`
and other bridge hubs before edits. Coordinate marshal/tag functions with P07.

Replace transitional Legacy/Managed paths with one native owner/operation
registry. Provide actual worker-affine persistent sessions where resources are
!Send; no unsafe Send or raw-pointer workarounds. Register opening operations,
directory ownership, DB children, snapshots, prepared sessions, drafts, changes,
results and log operations under bounded accounting. Handle resources retained
across hosted await without blocking unrelated tenants or moving LMDB guards.

Close stops admission, drains/retains Closing, releases resources then lock.
Retained JS wrappers cannot retain native DBs after completed close. Separate
one-shot borrows and generation checks; runtime shutdown joins opens and late
completions. Use one executor for FS/schema/hash/row work; no AsyncTask bypass.
Model transfer/cancel/finalizer installation races and cleanup reserved capacity.

Author foreign-addon/kind/generation/stale-scope attacks, double release, open
versus shutdown, paused process, close failure/incomplete, GC-disabled teardown,
held read/write session affinity, event-loop fairness and queue pressure tests.

Delete GC-owned lifecycle, expiring local ownership, duplicate counters/cache,
legacy sync/AsyncTask paths and cancellation that only stops waiting. Hand P07/
P08/P10 exact private callback runtime bridge, not a second public SDK. Complete
means every native operation/resource is accounted and close reports reality.

## P07 — Effect-only core TypeScript and shared Rust/TS ergonomics

Read 30–35/11 and version-matched Effect docs cited in 35. Contracts C01/C02/
C05/C09/C10. Default writes: `ts/src/` except P00-owned barrels/shared rosters;
core TS tests/cookbook; `crates/bumbledb-query/` and query macros only if P00
assigns them (otherwise provide typed AST change requests). P06 owns native
runtime internals; explicitly assign marshal files, never edit them concurrently.

Implement the complete chapter 35 roster, not wrappers around old Promise API:
pure schema/query metadata; lazy bounded compilation/codec/change ingestion;
scoped Db/snapshot/session/change/result; common QueryReader; one-shot page
Streams; Option/Result and direct error reasons. Complete typed floats, intervals,
ScalarExpr, nonrecursive composition and Rust syntax parity. No per-row fibers,
hidden runtimes, Proxy rows or whole-input copy at effect construction.

Read exact installed Effect 4 source/AI docs; use callback/acquireRelease/layer
semantics from that version. Preserve Cause interruption and finalizer failures;
do not turn every failure into DbError or mask whole operations. Derive boundary
schemas from core descriptors, not a second hand-maintained row roster.

Author typed/inference/negative consumers and runtime fixtures for lazy mutation,
repeated/exhausted iterables, cumulative budgets, failed builders, transfer races,
page take/EOF/failure, stable row shape, integer/float mismatch and scoped misuse.

Delete Promise/sync/disposal/cursor twins, optional Effect adapters and duplicate
error wrappers. Hand P08/P10 literal exported core primitives and shared helper
fixtures. Complete means P08 can use core values unchanged; never add a log
dependency to core or writable owner to QueryReader for convenience.

## P08 — Thin Effect log, independent tenant borrows and admin surface

Read 20–22/30–35. Contracts C06/C08/C09/C10/C11. Default writes:
`ts-log/src/` except `schema.ts`, `migrations/` (P10), shared barrels/manifests
(P00); log SDK/tenant tests excluding independently assigned P12 files.

Replace TS protocol with thin native calls. Implement scoped LocalHistory/
HostedHistory/Command/PublishedSnapshot, typed submit/resolve certainty and
bounded inspect; use actual core ChangeSet/QueryReader/result/scalar/codec types.
Expose existing maintenance/admin outcomes without inventing another journal.

Implement one native-backed typed TenantCache with distinct borrow releases,
opening-work accounting, stable binding/origin identity, pressure and joined
close. Delete renewable tenant TTL and pre-lock cleanup. Replace all complete
TS filesystem store operations with P05/P06 native entrypoints; no JS critical
section. Credentials/host configuration cannot create a second machine/cache.

Author effect interruption after publication, known receipt plus finalizer
defect, retained ref resolution after reopen, closed/foreign capability refusal,
double/stale borrows, same-schema cross-origin isolation and no hidden genesis.

Delete duplicate row lifting/descriptors/scalar tags/log builders, callback
writers, JS braid/manifest/CAS implementation and raw mutable replica export.
Hand P13 installed-package app flows. Complete means the log adds durable
identity and scoped host ergonomics, not another representation of facts.

## P09 — Native canonical migration execution and durable cutover

Read 22/33/35 plus core operators in 10–12. Contracts C01/C04/C05/C06/C08/C11.
Default writes: new internal log migration/initialization/activation modules,
native plan/schema-history codec and direct tests. P04 owns authority record
declarations, P05 snapshot/backup/recovery, P06 bridge wiring. Agree module split.

Publish the exact plan opcode/data/coverage/history contract jointly with P10
before either invents a local schema format. Provide native canonicalization/
digest entrypoints so TS generator does not duplicate binary encodings. Compile
finite declarative plan data using core scalar/query operators, not a new engine.

Implement authoritative history comparison, freeze, ordered pending-plan
evaluation with necessary intermediate checks, one final staged incarnation,
complete target admission/verification and ReadyToSwitch. Activation/genesis
status is durable. Abort irrevocably fences delayed target activation/genesis
before matching-source thaw. Stable ref exists before dispatch; uncertainty
survives interruption and restart. Ordinary open never initializes or migrates.

Author tampered/branched plans, edited applied prefix, intermediate constraints,
invalid final target, no-op history, cross-version families, staged partial data,
kill/lost-response at every transition and delayed activation versus abort.
Coordinate old-format converter as explicit log-only import if required by 22;
never add compatibility branches to ordinary core reads.

Delete ad hoc callback migrations and per-file full-incarnation publication.
Complete means generation/execution share canonical data and failed/uncertain
migration cannot silently thaw, activate or erase its only evidence.

## P10 — TypeScript schema diff, intent and repo-local migration workflow

Read 33/34/35/22. Contracts C01/C05/C10/C11. Default writes:
`ts-log/src/schema.ts`, `ts-log/src/migrations/`, generator CLI modules and
dedicated migration authoring tests/fixtures. P00 owns exports/package wiring;
P09 native bytes/execution. Preserved drafts import a missing ScalarExpr and
must not be mistaken for a working foundation.

Implement pure declarative rename/drop/backfill/convert/seed intent using core
types; generate/check Effects using native canonical schema/plan codec and
bounded filesystem work. Infer only unambiguous changes; require intent for
ambiguous rename/backfill/loss. Keep stable human labels separate from digests.
Emit deterministic reviewable repo-local plans/history with complete source and
target coverage, prefix verification and interruption-safe file writes.

Expose initialize/migrate/status/activate/abort by importing P08/P09 operations,
not a second runtime executor. No runtime import of user migration functions,
arbitrary code hash, helper-purity framework or manual coverage list. A generated
file is evidence of intent, not proof that production applied it.

Author Drizzle-like edit/generate/review/apply flows, deterministic output,
ambiguous/destructive refusal, drift/fork/tamper, bounded seed ingestion,
history replay and exact core ScalarExpr reuse. Hand P13 a complete example
repo history from initial schema through field/backfill evolution.

Complete means users author schemas plus necessary declarative business intent,
not executable migrations, and checked-in plans work in the native log.

## P11 — Lean denotation and independent semantic/history models

Read 10–13/20–22/50/70 and all relevant C-contracts. Default writes: `lean/`,
independent naive/reference model modules in `crates/bumbledb-bench/`, proof
bridge ledger and numeric oracle fixtures. P00 partitions bench files among
P03/P11/P12/P14. Corpus regeneration is deferred to F3 if it executes a generator.

Update actual mutable-support theorem premises, canonical F64 quotient/order,
integer/float interval semantics, grouped exact measures, complete query-stage
denotation and restricted recursive finite domain. Establish what is proved,
what is a correspondence test and which FPU/native obligations are empirical.
Remove fresh/braid premises rather than relabeling old proofs as successor proof.

Author independent small-state admission/query/history models and adversarial
trace corpus. Reuse data declarations only where it cannot conceal the bug;
never reuse production equality, scalar arithmetic, hash identity or transition
helpers as the oracle. Exact reductions require independent bit/rational checks.

Pair review P01/P03 and P04/P09 against models, without changing production
files. No sorry/admit/axiom escape or reduced generator coverage to pass. Hand
P12 finite counterexamples and P00 actual bridge obligations. Complete when
proof statements match implemented domains and all empirical gaps are named.

## P12 — Adversarial integration and complete gate coverage

Read entire audit/50/70/64 and relevant contracts. Default writes: explicitly
assigned independent integration tests, process harnesses, fault schedules and
release-checker tests. P00 owns ledger updates and scripts; no editing production
code to make an independent oracle agree. Subsystem authors still write their
own unit/behavior tests; P12 is not a late single tester for the whole project.

Map every audit ID and detailed child gate to concrete tests/lanes. Preserve
baseline repros and port safety properties of deleted APIs. Build real process
pause/kill/restart, delayed I/O, hostile codec/capability, foreign tenant, lost
publication, close/drain, retained result, migration and GC schedules. Cross the
actual Rust/Node and packed-package boundary, not just mocked helper functions.

Investigate existing nextest stdio LEAK reports without suppressing warnings,
changing timeout or serializing the suite to hide them. The separate Darwin FD
race is a hypothesis aid, not historical attribution. Missing credentials,
zero tests, skip or stale artifacts remain NotRun/failure as appropriate.

Deliver a final execution manifest using existing commands/harnesses, independent
review of each packet and evidence routes. Only run in F3 under P00 coordination.
Complete means no selected guarantee disappears between subsystem ownerships.

## P13 — Packaging, public examples, deployment contract and permanent docs

Read 30–35/40/64 and C09–C12. Default writes: `ts/scripts/`, `ts-log/scripts/`,
platform packaging staging code, examples, README/cookbook/runbooks/permanent
architecture docs as assigned. Propose manifest/lock/CI changes to P00 or take
exclusive ownership; never run package/build hooks during F0–F2.

Finish affirmative deletion of public C source/header/export/artifact/CI/examples
and public Rust log product. Preserve Rust/Node native-safety coverage. Build
one exact matched Node artifact; Rust core remains log/AWS-free and core-only
Node imports start no remote work. Replace source-mutating pack hooks with
immutable staging and exact handshake/pins. Remove obsolete package descriptions.

Author installed-package Rust/core-TS/log-TS syntax fixtures and a server-only
Next.js+Alchemy example: authenticated tenant binding, one app native layer,
local history dev, hosted materialization, retained command refs, explicit
generated migrations, refreshed credentials and attached IAM permissions.
Read Edullm bronze and explanation/learner storage for idiom, but do not edit
the sibling repository without explicit authority. Preserve business idempotency
and outbox semantics; generic Promise wrappers disappear from the example.

Document Node24/26/native OS/libc/CPU/disk support, measured Vercel envelope,
unsupported Edge/browser/Expo, real backup restore and admin runbooks. Author
deployed request/migration tests; running cloud changes requires explicit scope.
Move completed proposal contracts to permanent docs before eventual retirement.

Complete means clean installed consumers exercise the intended SDK without
source aliases or stale binaries; compilation/deployment proof is collected F3.

## P14 — Performance, physical accounting and magic-number review

Read 40/41/10–12/35, repository README/benchmarks and sibling bumblebench
evidence. Contracts C01/C02/C04/C05/C09/C12. Default writes: explicitly assigned
benchmark/report/probe modules and measurement documentation. P00 owns entrypoint
scripts; other agents own production constants. Submit measured changes to them.

Author matched warm/cold/post-write/selective/large-result/tenant-churn/hosted-
contention/maintenance workloads. Separate native, bridge, Effect and complete
app cost. Count copies, bytes/namespace/index, allocation, live resources, queue
wait, conversion, event-loop delay and full publication requests/bytes/retries.

Explain indexed-SQLite comparison with matching schema/index/durability rather
than assuming every extra byte is inherent. Classify each policy constant as
derived arithmetic bound, backend limit or measured tuning. Plan AEGIS/BLAKE3
role-specific probes, long-key physical choices and collision-forcing tests;
do not substitute AEGIS or shrink authoritative commitments without evidence.

Preserve baseline source revision and raw evidence now; no measurement until
F3. Use isolated baseline checkout/artifacts during final comparison. Actual
>40 GiB populated data and separately enforced >RAM workload are distinct gates,
not sparse files or a large map setting. Apple Silicon first; Graviton/x86
portable qualification does not inherit M2 constants.

Deliver reproducible harnesses and final measured cost decisions. If format
selection changes in F3, notify C12 consumers, update implementation/goldens and
rerun affected qualification. No alternate permanent backends/algorithms kept
just to avoid a decision. Complete means claims have matched evidence, not
“performance-aware” code style.

## Primary audit ownership — all 68 IDs

This is responsibility assignment, not closure. P00 maps all detailed child
gates separately from chapter 70; P12 independently checks coverage. Shared
review never removes the primary owner's obligation.

| Primary | Audit IDs |
| --- | --- |
| P00 | ASS-004 |
| P01 | ENG-001, ENG-002, ENG-004, ENG-005, ENG-007 |
| P02 | ENG-003, ENG-006, ENG-008 |
| P03 | QRY-001, QRY-002, QRY-003, PERF-001, PERF-002 |
| P04 | REP-001, REP-002, REP-004, REP-006, REP-015, REP-016, REP-020; SDK-001, SDK-008, SDK-009, SDK-014, SDK-015; ARCH-001, ARCH-002, ARCH-003, ARCH-004, ARCH-006; OPS-005 |
| P05 | REP-003, REP-005, REP-007, REP-008, REP-010, REP-011, REP-012, REP-013, REP-014, REP-017, REP-018, REP-019; SDK-016; OPS-002; PERF-004 |
| P06 | REP-009; SDK-002, SDK-004, SDK-005, SDK-006, SDK-007, SDK-010, SDK-011, SDK-012; OPS-004; PERF-005 |
| P07 | SDK-003 |
| P08 | OPS-006 |
| P09 (P10 generator co-owner) | OPS-001 |
| P11 | ASS-001, ASS-002 |
| P13 | SDK-013, ARCH-005, OPS-003, ASS-003 |
| P14 | PERF-003 |

P10/P12 and secondary contributors also own required non-indexed/new-feature
work. No absence of a private audit ID excuses missing float/migration/API/
backend/performance gates. Chapter 70 remains the larger finite release contract.

## Complete child-family routing

Ranges are exactly the chapter 70 families, not a new test inventory. A primary
owner is accountable for the whole listed group even when another packet authors
some tests. P12 reviews evidence independently; the additional review partner
below challenges semantics, ownership or cost before qualification. P00 records
individual expanded children and concrete tests in the existing result ledger.

| Chapter 70 group | Primary packet | Additional implementation/review partner |
| --- | --- | --- |
| CONC-01 through CONC-06 | P04 | P01/P03 algebra; P11 independent history |
| All 12 E-* engine families | P01 | P02 owns snapshot/storage/durability cases; P11 reviews denotation |
| All 13 F-* float families | P03 | P01 value/interval/casts; P07 cross-language; P11 independent bits/proofs |
| All 13 Q-* query families | P03 | P02 disk/snapshots; P06 lifetime; P11 independent query oracle |
| All nine P-* assurance families | P11 | P01/P03 review proof premises; P12 independent bridge evidence |
| PROTO-01 through PROTO-20 | P04 | P05 backend/GC; P08 public client; P11 history model |
| STORE-01 through STORE-10; LOCAL-01 through LOCAL-03 | P05 | P02 snapshot; P04 local authority |
| GC-01 through GC-13 | P05 | P04 publication; P09 target races |
| FS-01 through FS-05; S3-01 through S3-06 | P05 | P06 complete native dispatch; P08 TS interop |
| REC-01 through REC-07 | P05 | P04 certainty/replay; P09 migration recovery |
| BACKUP-01 through BACKUP-05; RESTORE-01 through RESTORE-03 | P05 | P13 independent-recovery runbook; P09 incarnation/history |
| MIG-01 through MIG-14 | P09 | P04 authority; P05 staged roots; P10 generator |
| ERASE-01 through ERASE-04; OPS-TEST-01 through OPS-TEST-02 | P05 | P08 admin reports; P09 activation/abort exclusion |
| API-01 through API-12 | P07 | P08 log reuse/certainty; P13 installed consumers |
| RUN-01 through RUN-15 | P06 | P08 borrows/cache; P13 hosting; P14 fairness/cost |
| FFI-01 through FFI-08 | P06 | P02 actual Rust lifetimes; P07 typed bridge; P13 artifacts |
| PKG-01 through PKG-06; PKG-07A; PKG-07B | P13 | P00 pins/evidence; PKG-07B only after authorized publication |
| TS-MIG-01 through TS-MIG-10 | P10 | P09 native canonical plan/execution; P07 literal scalar reuse |
| APP-01 through APP-08 | P13 | P08 app history; P10 generated repo; P14 deployment envelope |
| APP-FAST/MUTATE/NUMERIC/LARGE/TENANTS/TARGETS/METHOD/MAGIC | P14 | P03 hot path; P06 fairness; P13 targets; P00 cost decision |
| SPACE-01 through SPACE-02; HASH-01 through HASH-04 | P14 | P02 layout; P01 exactness; P04 authoritative threat model |

Parent accountability: P00 G00/G16; P13 G01/G13; P01 G02/G03; P03 G04/G05;
P02 G06; P04 G07/G09; P05 G08/G10; P06 G11/G12; P12 G14; P14 G15.
Parents are conjunctions of their required evidence, not separate shallow smoke
tests. P00 consolidates all applicable child results and external blockers.
