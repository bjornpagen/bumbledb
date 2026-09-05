# 04 — Successor foundations and Effect error cutover

Date: 2026-09-04. Branch: `codex/bumbledb-1-0`; implementation parent
`96d8bec722e882ed69d625ec8dffcfca32fbafd6`. This records an incremental
implementation checkpoint, **not completion of the successor or qualification
for 1.0**. The selected `final-solution/` and historical `audit/` remain intact.
No version, release tag, package publication, or production data is changed.

## What is implemented

### Numerical foundation and independent oracles

Canonical F64 values now cross the existing Rust query, benchmark, Node input,
binding, and answer boundaries. SQLite comparison tests store exact sortable
eight-byte BLOBs: SQLite REAL is not a lossless NaN oracle. Arithmetic over those
BLOBs explicitly refuses; it must never become a silently incorrect SUM oracle.
The relational test compares LMDB, the simple set evaluator, and SQLite for all
six comparisons, literal/scalar/set bindings, min/max, joins and projection.

The numerical kernel implements guarded primitive arithmetic, integer-based
casts, exact 34-limb sum and once-rounded exact-rational mean. Its independent
fixture generator uses Python fractions and integer binary search, not the Rust
arithmetic implementation: 6,144 arithmetic cases, 317 reductions, 528 casts.
Apple Silicon debug/release and x86-64 macOS under Rosetta have been exercised;
Rosetta is **not** Linux or Graviton qualification.

Lean now has canonical F64 values, strict wire parsing, independent integer
numerical denotation, and a general physical-key/order refinement proof. It also
proves normalization, injectivity, antisymmetry and the finite magnitude bound.
The corpus contains 277 cases, including 18 seeded F64 cases and 15 direct float
query guards. Four independently authored historical measure witnesses are
preserved byte-exact and are not replaced by the seeded generator.

The unchecked generic interval `__ground_axiom` was removed. Checked integer
`const_new` constructors and checked macro construction replace it. Downstream
compile-fail tests enforce the removal and refusal of invalid constants.

These changes do **not** finish query arithmetic/exact aggregate integration,
float intervals, formal accumulator/rounding proofs, or cross-platform numerical
qualification. The safe raw-fact trust boundary also remains broader than the
removed interval escape.

### One budget authority and immutable application command

Core `WorkContext` clones share cancellation, deadline and resource accounting.
Input/row/work counters are cumulative; working/scratch/result byte reservations
are linear and remain charged while the owned data lives. Zero means zero;
counter overflow refuses. This measures logical owned bytes, not RSS or LMDB's
page cache. Existing planning, admission and image allocations are not yet all
connected to this authority.

Core `CanonicalRow` owns a strict scalar grammar and its byte reservation. The
same parser validates and materializes values. The schema-bound, Arc-backed
`ChangeSet` normalizes remove/add records to sorted unique full canonical rows,
with add winning a same-command remove. Builder failure spends the draft.
Sorting is fallible and cooperative rather than interrupting a standard sort
through a panic. Clones retain the identical allocation and charge.

The log's verified `Command` retains that actual core ChangeSet. It hashes bounded
chunks of the common framing and delegates scalar validation to core. Unverified
framing is explicitly a different type and cannot claim verified application
semantics. Repeated schema identity access uses a cached fingerprint instead of
serializing and hashing the entire validated schema for each command.

This is an in-memory foundation, not spill-backed ingestion, the final physical
format freeze, or the complete public schema-generic command API.

### Atomic core/log integration boundary

The internal writer session holds the core writer mutex across preparation,
rejection, opaque log metadata sealing, and commit or abort. Same-thread reentry
refuses; waiting for another writer checks cancellation/deadline. A rejected
candidate can be followed by an empty receipt transaction without allowing
another writer into that session.

Facts, host records and the host attachment share one LMDB transaction. Readers
observe them through the same read transaction. An effective metadata-only
commit advances core generation; idempotent metadata writes do not, and a
fact-changing transaction does not advance twice. Application fact counts remain
separate from core generation. SIGKILL tests at prepared/sealed/committed points
observe either all-old or all-new state after reopening.

Independent review exposed a host-sealing budget bypass: one large slice copy
or equality check could run between cooperative polls. Two new adversarial tests
failed on the original implementation. The correction compares and copies in
4 KiB charged chunks, using heed's safe reserved write; failure consumes the
prepared transaction and aborts facts plus the already-written metadata prefix.
The typed work error survives the storage callback without an allocated error
wrapper. Native LMDB page allocation and filesystem calls remain safe-point gaps.

Opaque metadata currently uses disjoint prefixes in the existing metadata
database. That is a transitional integration substrate, **not** an assertion that
the new storage format, LocalHistory, HostedHistory, receipt execution, migration
engine or recovery protocol is implemented.

### Native runtime and TypeScript direction

Both packages require exactly Effect `4.0.0-rc.112`. A bounded native worker and
operation registry now owns admission reservations, cancellation, completion,
acknowledgement and bounded drain. The singleton remains live while Closing;
timing out cleanup does not release live native resources to a successor runtime.
The TypeScript service uses Effect Context/Layer, scoped acquisition and callback
cleanup, with no hidden per-operation Effect runtime. Its first actual off-thread
consumer is a private bounded hashing operation, not a synchronous database call
disguised by an Effect wrapper.

The owner's additional hard cut removes `@superbuilders/errors` from both SDKs,
scripts, tests, package manifests and lockfiles. The implementation follows the
consumer's direct Effect `Data.TaggedError` classes with readonly fields and
preserved `cause`; native diagnostic decoding uses Effect schemas where needed.
There is no replacement `errors.new/wrap/is` compatibility framework.

The old protocol error side tables were deleted: structured details now live on
the tagged errors themselves. Log declaration emission uses explicit annotations
of Effect's public returned constructor type where TypeScript otherwise emitted
a nonportable linked-install path. Build scripts import their own small Effect
error declarations through relative paths, so a clean checkout does not need an
already-built `dist/` to run the build. The core/log version check now enforces the
selected exact peer version instead of its old caret range.

The old hot-read heap heuristic was not a handle-allocation test: after 4,000
reads, uncollected temporary rows could exceed its threshold while retained heap
was essentially flat. Its replacement counts actual SDK/native crossings over
4,000 verified reads and forbids owner-producing calls. A negative control inserts
a real, promptly disposed builder into a read and must be detected. This is a
handle-path regression, not a throughput or retained-heap qualification claim.

**The existing public database/log API is not yet fully converted to Effect.**
Removing its error-helper dependency and implementing the real native runtime
are prerequisites, not evidence that the complete scoped asynchronous database
surface, streaming pages, tenant runtime or generated migrations has landed.

## Qualification evidence

These results are scoped to the listed tests, not blanket closure of their
corresponding release-gate families. In-progress checks are deliberately named.

| Check | Observed result |
| --- | --- |
| Workspace all-target compile, locked | Passed |
| Workspace all-target Clippy, warnings denied, locked | Passed |
| Workspace rustdoc | 27 passed; one existing query-macro example ignored |
| Log without default features, locked | Passed |
| Whole-workspace parallel nextest, first run | **Failed**; run `5b8f0312-708e-4105-ae6f-bea8c43d6a0d`: 2,146 passed, 2 failed, 30 skipped; 419.675 seconds |
| Full feature/allocation/check script, first run | Allocation gate passed; ground-off tests: **1,286 passed, 2 failed**, 20 skipped; subsequent script stages did not run |
| Corrected feature checks | Allocation gate passed; ground-off **1,292 passed**, 20 skipped; trace **1,332 passed with one LEAK diagnostic**, 21 skipped; obs **422 passed, 2 failed, one LEAK**, 8 skipped; shared test scratch collision corrected below |
| Post-build whole-workspace parallel nextest | **2,153 passed, 1 failed**, 30 skipped, 450.168 seconds; run `ba8ab1f4-03d2-42d2-b787-ab2b6ee701db`; mixed Rust/TS filesystem CAS lost one acknowledged increment |
| Isolated obs after scratch correction | **425 passed with one LEAK diagnostic**, 8 skipped, 146.417 seconds; flame selftest passed; not warning-free qualification |
| Non-vacuous derived-query census | Passed after replacing the always-true pattern; run `f77c6f7e-2a9f-4d4f-98d0-3f0b3fd1524c`, 1 selected test |
| Core work/canonical/ChangeSet tests | 12 passed; run `f0a18e7c-a1b7-4722-9cf6-58acaa6e287a` |
| Log verified command/admission/framing/model tests | 20 passed; run `cda5bc22-395b-4ce8-9540-fe7ee6f2f15f` |
| Core host transaction tests | 10 passed; run `0b03de38` (abbreviated runner identifier) |
| Writer-session concurrency/cancellation tests | 6 passed; run `2135858d-6979-4925-956a-3e08df4239dd` |
| Independent host budget regressions | 2 failed before fix (`2ef74e5e-aaa8-41f3-b75c-acf7c2ea4774`); 18 host/session tests passed after it (`a3b332a4-578d-4208-830a-e90a07fdcf46`); final empty-value/sentinel refinement also passed in the joint run |
| Float relational/SQLite arithmetic-refusal tests | 2 passed, 417 filtered; run `55f812c8-c3fe-411c-8dcc-9f409b438ad1`, including zero/infinity/NaN two-bound range folding |
| Lean build, proof census, cross-language replay | Passed; 277 cases, zero disagreements |
| Interval construction | 8 theory tests and 3 downstream tests passed; all 43 compile-fail fixtures passed |
| Fresh release native addon and both package builds | Passed; addon SHA-256 `697f302eb83175cd2f81ba2787253ea0fd5cb8e746bda4b199d4563e520cede6` |
| Core TS, ordinary built-package resolution | **430 passed**, 0 failed/skipped; 57 suites; actual freshly built addon, no source-condition/preload override |
| Log TS, source-condition independent packet | **168 passed**, 0 failed, **6 explicit S3 skips** |
| Log TS, ordinary built-package resolution under concurrent integration load | **167 passed, 1 failed, 6 skipped**; tenant directory renewal permitted a second owner; real ownership defect, not qualified by the earlier pass |
| Both package typechecks/lints and declaration emit | Passed; exact Effect RC |
| Fresh packed consumer | Passed on macOS: actual five tarballs, isolated install, strict TS 7.0.2 declarations without skipLibCheck, Effect tag/reason recovery, exact causes/shared peers, native log commit/read; Linux binaries remain unqualified |
| Effect runtime reason handling | 9 tests passed, including `catchReason`'s actual behavior and inferred error channel |
| Release-evidence checker regressions | 3 passed |
| Pre-promotion evidence qualification | **Refused**: 306 unresolved checks; no release declared |

The prior `lane_e_lease` parallel failure and nextest LEAK diagnostic remain part
of the record in [02](02-checkpoint.md). Focused passing reruns do not establish
their original causes or erase those observations. A LEAK report concerns output
handles after process exit, not by itself an LMDB/heap leak.

The first whole-workspace run reproduced the lease failure: a child exhausted
the filesystem emulator's hidden five-second mutation-lock wait during
`put_create` for `ids/00000002/0000`. Disjointness was not disproved, but expected
successful completion was not established. The five-second bound remains intact.
An occupied create now returns `Exists` under the held lock before staging or
syncing temporary data. This also corrects a separate deterministic bug: a staging
failure with equal bytes already present could report a second `Created`.
The regression deliberately makes the temporary namespace unavailable and checks
both equal and unequal occupied creates leave body/generation unchanged.

The six-child contention test still accounts for all 48 attempts; it permits only
the exact known pre-mutation lock refusal, checks every successful allocation's
width/alignment and contiguous disjoint prefix, requires a contended success, and
checks an exact successful tail after all children are reaped. A separate paused-
owner test forces timeout, verifies no mutation or takeover, and then succeeds
after release. This distinguishes bounded operational availability from uniqueness
instead of assuming a non-fair kernel lock guarantees a win within five seconds.
Focused verification passed 22 tests (`94f12ed6-1152-4ae5-b402-d4dd7c58d1d0`);
these cases also passed whole-workspace verification. No arbitrary timeout increase or
retry-to-green was introduced.

The other failure was the source census rejecting the new standard
`HostSealError::source` trait signature. Its exact path/signature was added to the
existing error-source-only exemption, not a blanket dynamic-dispatch exemption;
the focused regression passed (`3f9e0231-3128-47c2-893a-172b349bd665`).

The ground-off run also observed a library disappear during the compile-fail
suite while another Cargo invocation rebuilt the shared target. The runner's
compatibility probe incorrectly accepted any compiler error except a version
mismatch. Both schema/query runners now require a successful positive compile,
with a new valid/missing/malformed-library regression. This does not make
concurrent replacement of build artifacts safe: final qualification sequences
build phases and runs tests in the parallel pool against those artifacts.

An intermediate whole-workspace run was intentionally interrupted before package
freeze because its interop children would otherwise race a distribution rebuild;
it is not counted as qualification. The post-build run
`ba8ab1f4-03d2-42d2-b787-ab2b6ee701db` ran 2,154 tests, with 30 skipped:
2,153 passed and `mixed_fleet_cas_linearizes` failed. Its four contenders reported
32 successful swaps but the shared counter contained 31. The Rust and TypeScript
filesystem mutation protocols still need one interoperable exclusion authority;
the exact failure is under investigation, not attributed to package timing.

The obs failures were independent of that CAS defect. Benchmark verification
tests used fixed temporary directory names across default/obs binaries, so an
overlapping run could delete another run's mismatch bundle. They now retain the
existing unique RAII temporary-directory owner, and a same-label regression
checks independent paths and cleanup lifetimes. The corrected obs run passed all
425 assertions, but emitted another LEAK on a pure calendar test. The derived-query
coverage assertion also no longer uses an always-true struct pattern: it checks
an actual interior relation or recursive component.

The trace and obs stages additionally reported LEAK on unrelated core/count and
pure calendar-generation tests. Nextest's output-handle detection is under
investigation; passing assertions and a zero exit code do not erase these warnings.

Read-only inspection of nextest 0.9.143 / nextest-runner 0.122.1 confirms its
split-output detector waits 200 ms for EOF **after process exit**. A source-backed
but not yet incident-proven hypothesis is Darwin pipe inheritance: Rust creates
a pipe and marks its endpoints close-on-exec in separate operations, while
nextest can launch another test concurrently in that interval. An inherited
sibling writer could delay EOF for a completely childless test. Actual descriptor
ownership still needs a witness; no timeout/configuration/dependency change is
justified by the hypothesis alone. If established, serialize launch critical
sections, not test execution, or close non-explicit descriptors at spawn.

The final built-package log run exposed a separate **real** ownership gap:
`tenants.test.ts`'s still-open replica could lose its renewable 90 ms directory
lease under scheduling/I/O load, admitting a second owner. TypeScript's old tenant
pool has not yet adopted the Rust kernel-held directory ownership introduced in
the earlier packet. Do not repair this by increasing TTL, accepting takeover, or
relabeling the test flaky. Replace expiration-based directory ownership with the
shared native capability before claiming local owner safety or release readiness.

## Remaining work and release boundary

The release inventory records all 68 audit obligations, 17 parent gates and 220
child gate families as unqualified. The checker detects absent/duplicate/stale
evidence, empty execution, unexplained skips and artifact/report hash mismatch;
its own regression pass is not a product release pass. Full lane applicability
and artifact provenance still need qualification. Before deleting the proposal,
the inventory source must move to a permanent qualified contract rather than
leaving a checker that reads a deleted directory.

Next work follows the selected milestone dependencies: complete numerical query
semantics and float intervals; the physical row/text/hash rewrite; transaction-
gated LMDB growth; bounded cursor/scratch execution; actual history authority and
recovery; the single Effect database API and generated TypeScript migrations;
fresh packed-platform, cloud and performance qualification. Keep Free Join,
direct probes, lazy COLT and the specialized integer kernels; do not replace them
with a compulsory resident image or a new general-purpose framework.
