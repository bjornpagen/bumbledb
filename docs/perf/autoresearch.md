# Development autoresearch notes

The allocation and memory-lifetime cleanup below was committed as `b42f5c05`
and passed all required [cross-platform CI checks](https://github.com/bjornpagen/bumbledb/actions/runs/34243728367).
The research loop is stopped. The subsequent 1.1.0 benchmark/release refresh
is separate; see the [current benchmark results](results.md).

The implementation uses ordinary
unrestricted Rust allocation and OS mmap behavior, with correctness before
performance before simplicity. The hardware targets include a Raspberry Pi
Zero 2 with 512 MB RAM and large macOS machines. No Pi performance or memory
qualification is claimed. The cleanup itself did not publish a release.

These are development experiments after the [published 1.0.1 results](https://github.com/bjornpagen/bumbledb/blob/v1.0.1/docs/perf/results.md),
not replacement release benchmarks. Priorities are hardening, correctness,
performance, then simplicity. Existing native traces supplied the leads below;
no new traces were collected for this round.

## Allocation cleanup

Starting revision: `d62a0b5c79f1b6be012510b48ddf876fe674b2f7`.
The cutover covers all eight requirements:
TypeScript prepared-plan ownership; lazy checkpoint recovery; bounded projection
gather and duplicate-safe map growth; correctly scoped reset/release; automatic
text reclamation; redundant-copy removal; unrestricted Rust/TypeScript APIs;
and removal of unjustified spill policies.

The dated checkpoints below preserve intermediate failures and incomplete
states. See the final wrap-up audit for the current local qualification;
the linked CI above is the completed qualification of the pushed revision.

The retained round87 and round89 profiles identify copy, projection, aggregate,
and output paths worth inspecting. They are CPU samples, not allocation counts.
The round91 ownership audit identifies buffer growth and lifetime boundaries,
but its quota-admission requirements are superseded by unrestricted allocation.
Historical quota tests must not dictate the new product semantics.

Initial caller inspection confirmed that production scan windows already contain
at most 256 rows. Keep that bound when removing ledgers and enforce bounded
gather for direct sink callers; do not claim an unbounded production buffer.
Two whole-data owners remained: recovery and hosted migration eagerly collected
all received checkpoint chunks, and owned-instance publication cloned canonical
rows into a temporary ordered map before sealing. WordMap also grew before
testing whether a key already exists at its load boundary. These were source
findings, not measured speedups.

### First ownership changes: local evidence

Four discriminating tests failed on the starting engine plus test-only edits
(nextest run `18a9abb7-b8f0-490a-bd04-c8288673551b`) and passed after the first
implementation (`a9bf9235-e921-46b2-b713-f4737cdfdae8`). Both used the pinned
toolchain, debug test builds, nextest-isolated processes and `alloc-counter`,
serialized through `scripts/measure.sh`. These are allocation/ownership checks,
not elapsed-time comparisons or integrated qualification.

| Operation | Baseline | First implementation |
| --- | --- | --- |
| Seal 8,192 owned rows | 9,559 allocations / 975,014 allocated bytes | 2 allocations / 334,908 allocated bytes |
| Seal 128 owned rows | 151 allocations / 14,838 allocated bytes | 2 allocations / 5,156 allocated bytes |
| Duplicate at 64-slot WordMap boundary, one-word key and `u64` value | 4 allocations / 2,311 bytes; grows to 128 slots | No allocation, free, or rehash |
| Recovery, 16,384 rows, 97 fetched chunks | Prior receive owners accumulate from 0 to 393,216 bytes before fetch | Prior receive owners released before every fetch |

Row sealing now borrows the already ordered canonical rows; the final payload
is unchanged (334,812 bytes for 8,192 rows). Allocation measurements include
Rust allocation requests, not allocator metadata or RSS. Recovery observations
use the existing receive-owner ledger, not a complete heap census: decoder
carry, import batches, manifest metadata and LMDB allocations are separate.
Recovery and hosted migration now pass a lazy verified-chunk iterator directly
to the existing decoder. A first recovery fixture incorrectly requested chunks
below the writer's 4 KiB minimum; its fixture-size failure was corrected before
the negative control above and is not counted as a production defect.

Direct projection-sink callers now gather in reusable windows of at most 256
rows. Tests cover mixed byte/word columns, reordered position lists, reset and
the existing unique direct-output path. This enforces the existing executor
bound at the owner too; it does not establish a production memory reduction.
An additional discriminator rejected the first bounded-gather patch: exact
growth on successively larger 17–256-row runs caused 240 growth events
(`60445733-98b2-4630-a27c-ea33a978b583`). Bounded geometric growth now passes
the cold bound of at most five growths and zero on warm repetition, without
exceeding one window. All four projection tests passed
(`ae30d9cb-1d9f-41e8-b11d-8b19cfc7fcd9`). This was a regression caught in the
initial local patch, not a previously shipped defect.

After removing the unused ordered-map adapter, the first combined patch passed
2,549 workspace tests (32 skipped), strict all-target/all-feature Clippy for
core/log, and three focused Miri tests for duplicate-boundary lookup, panicking
construction and scalar/bulk growth/reuse. The first full run
(`c1494d5f-aee1-4a6d-9b04-e0357d4272f8`) reported one subprocess-cleanup warning;
the complete rerun (`5882019a-7887-4b94-98ad-b8bd16df76d1`) reported none.
The original warning's cause is unknown; a clean rerun does not explain it.

Further decoder inspection found that every record drained the front of the
carry vector, repeatedly shifting the remaining chunk bytes. The decoder now
borrows complete records directly and copies only split records into reusable
carry storage, removing those per-record shifts. A pointer-identity regression
failed before this change (`827bd934-e989-4b7f-a619-5644f8920c34`); the decoder,
recovery, restore and hosted-migration selection then passed all 123 tests
(`b34f1735-1753-4680-9c57-30274f991cd7`). The boundary tests now rechunk the
fixture explicitly at every width, rather than using writer sizes that both
produced a single chunk. No elapsed-time improvement is claimed for this change.
The subsequent combined workspace run passed 2,550 tests, with 32 skipped and
no cleanup warning (`fb598056-33bb-4587-a6aa-ef3e059bb451`).

### Scoped native preparation

The old TypeScript execution-session handle was an alias of its snapshot:
each execute parsed and prepared again, and retained the unused plan under
that operation's ID. The one-shot regression failed with one retained plan
after its first operation (`e3c08e05-9e2f-42ed-8893-32cf3c0d4d2e`).

One-shot plans now drop before the completed result returns. The public
replacement is `reader.prepare(query, work)` followed by
`prepared.execute(params, work)`: one worker-owned compiled object, with
ordinary scoped cleanup. Core and log readers expose the same API. No
execution-session compatibility alias remains. Plans share the pinned read
on its worker, close independently, and are all drained by database close.
Completed results own their storage independently of these readers.

Tests cover repeated reuse of the same compiled object, two independently
closed preparations, snapshot closure with a live preparation, coherent
answers after later writes, invalid arguments, cancellation after installation
and execution, abandoned outputs, scope escape, and database drain while
wrappers remain reachable. All 99 native tests passed both normally and with
allocation counters; the latter run is
`ac9662af-f426-4945-bdce-4463eefac93a`. Strict native all-target/all-feature
Clippy and both SDKs' builds, tests, type checks and lint passed locally.

The allocation test executes 16 native jobs against one 512-row snapshot,
comparing one-shot preparation with an already-warmed retained plan. Query
IR/job and operation-context fixture creation are outside the windows; engine
preparation/execution and owned completed results are inside. Nextest isolates
the process; these are debug allocation counts, not timing or JavaScript/RSS
measurements. Two runs reported the same allocation totals:

| Native job bodies | Allocations | Allocated bytes |
| --- | ---: | ---: |
| Prepare and execute each time | 2,480 | 1,108,944 |
| Execute the warmed preparation | 128 | 391,680 |

The source delta from `d62a0b5c79f1b6be012510b48ddf876fe674b2f7` for the
core/log crates, native bridge and both SDK sources is retained locally as
`bench-out/allocation-cleanup.QCvqv9/candidate.patch`, SHA-256
`af00623bb81e3a8026c82a2a9f13f52926f6dcb871717c854a6131d5d68cf7a6`.
The directory also holds the native test log. This delta is not a committed
or cross-platform-qualified revision.

SDK verification also reproduced a macOS build-tool defect: overwriting the
existing `.node` inode caused kernel code-signing rejection of cached pages
and SIGKILL, despite valid on-disk signatures. The compiler output with the
same bytes loaded successfully. Installation now copies to a fresh staged
inode and atomically renames it. The inode/old-reader regression failed with
the old copy, then both replacement/failure-cleanup tests passed, the same
native bytes loaded, and the full package build completed. No signing check
was disabled. The addon SHA-256 was unchanged across the rebuilds:
`ef2228b3fb5c33df67f43942b626d2343a891358ffbea7b02d1013a0b7bd98cc`.

The pre-delivery cleanup also passed the release warm-allocation gate
(`047cb3b1-edb1-435e-b7ff-f980e9a256df`) and the combined workspace suite:
2,551 passed, 32 skipped, no cleanup warning
(`32fa7522-d61b-41f3-bef8-16785b9e4ebd`).

### Borrowed native result delivery

Native collection previously copied the sealed backing into a complete
`Answers` carrier, then copied that carrier into native output. Native pages
did the same for each page. Both now reuse the backing's borrowed-row visitor
and convert directly into the final worker-to-JavaScript owner. Rust exposes
`CompleteResult::visit_rows` and the fallible `ResultRow::values` iterator;
ordinary owned Rust pages use that same traversal. No separate streaming
execution engine or asynchronous iterator framework was introduced.

Collection is still non-consuming. One ticket covers page visitation through
publication acceptance; cancellation or rejection discards its pending advance.
Scratch values decode from the same mapped borrow used for sizing, without
another read transaction or intermediate text/blob owner. Native paging's
outer row vector grows geometrically. JavaScript-owned text and bytes still
require their final copy: this is not zero-copy JavaScript or early-result
streaming.

The 512-row scalar allocation discriminator failed on the old delivery path
(`aaa1b3b1-f0ed-4641-a7cd-675f9c46752a`) and passed after the change
(`aa808638-3639-4d66-8d03-1e79a537727f`). Each row contains two `u64` cells.
The window covers collection only, after all runtime workers have drained,
in a nextest-isolated process with the pinned debug toolchain and allocation
counters. Fixture construction and execution are excluded.

| Native collection | Allocations | Allocated bytes |
| --- | ---: | ---: |
| Old full-`Answers` intermediate | 522 | 94,112 |
| Direct borrowed conversion | 513 | 45,056 |

The candidate repeats these exact totals on a second collection. All
allocations are final output: one outer vector plus one vector per row;
there are no intervening frees. This is a reduction in Rust allocation
requests, not a timing, RSS or JavaScript-heap claim.

Candidate-only text measurements likewise match final native output exactly,
twice per fixture (`4ee1689e-d28e-4984-a6b4-fd458451fbf5`):

- 128 rows × 1,024 UTF-8 bytes: 257 allocations / 142,336 bytes.
- 129 rows × 65,536 UTF-8 bytes, crossing the current result spill threshold:
  259 allocations / 8,465,496 bytes.

These include the final text allocation per row, with no allocation/free for
an intermediate answer heap. The RAM borrowed-row traversal itself makes
zero allocation requests. Core tests cover every value kind, borrowed payload
pointer identity, every truncation boundary, invalid UTF-8, noncanonical
floats, invalid intervals, trailing bytes, and cancellation on the last row.
The refactored scratch decoder also rejects invalid boolean bytes that the
old decoder treated as true. All 26 completed-result tests passed
(`bc2029eb-c69c-4969-ba0a-cf87544f19f7`).

The combined candidate passed strict workspace and native all-target/all-feature
Clippy, all 101 native tests under the CI profile
(`633799a9-6420-4e7c-99ac-5acc6b681c46`), and 2,555 workspace tests with 32
skipped and no cleanup warning (`eadd08d3-4817-487f-8758-a969f458b2bf`).
The native workspace needs the root nextest configuration passed explicitly
with `--config-file .config/nextest.toml` to select that profile.
Both SDKs subsequently passed their complete builds, package checks, tests,
type checks and lint: 219 core SDK tests and 168 log SDK tests, none skipped.
The new end-to-end test compares collection with page streams over UUIDs,
full-range integers, text, byte arrays, intervals and floats; mutating a
delivered byte array cannot change later reads, and all values survive
native scope closure. The rebuilt macOS addon SHA-256 is
`23740a7ce4dc6d845fac3528652786aa0955575d7718a4b69546fee220650e40`.
Workspace doctests passed (37 passed, one ignored), documentation built with
warnings denied, and the release warm-allocation gate passed
(`b950a5df-2c96-4fb0-bedb-39f366ed39ef`). No new profiler captures or full
benchmark suite were run for this change; controlled timing comparisons and
cross-platform qualification remain outstanding.

The combined source delta for core/log, native, SDK sources and tests from
`d62a0b5c79f1b6be012510b48ddf876fe674b2f7` is retained locally as
`bench-out/allocation-cleanup.QCvqv9/visitor-candidate.patch`, SHA-256
`376b85f868fb2dd0dc6bb4a37572a58c2979e6f3980d3c7a9746767ab9326c6c`.
This excludes the separate native-artifact build helper and documentation;
it is not a committed or cross-platform-qualified revision.

### Scoped execution-memory release

The round91 ownership audit correctly identified active COLT pools and nested
Pack claims as retained owners. Its quota design remains superseded. A new
8,192-row query regression reproduced the old release defect
(`e15f52ad-3337-4053-bb3d-48cbbb0cf1f6`): `trim` reduced the join-pool footprint
from 159,008 to 126,240 bytes, rather than releasing it. It also cleared the
shared database cache, an unrelated owner's responsibility.

The Rust API now separates `PreparedQuery::release_memory()` from
`Db::clear_cache()`. Cache-retention inspection also belongs to `Db`, not a
single prepared query. The old query `trim` is removed. Ordinary resets keep
capacity for warm reuse. Explicit release drops active and parked join pools,
executor scratch, projection and aggregate state (including nested Pack
buffers and dense tables), computed-output bindings, derived/recursive state,
parameter resolutions, row scratch and text-finalization scratch. Compiled
plans, schema/layout tables and uniqueness proofs remain. The next execution
recreates only its working buffers; no plan revalidation or recompilation is
needed. Fixed-schema row scratch no longer retains an obsolete prepare-ledger
reservation; the broader quota removal is still unfinished.

TypeScript exposes `prepared.releaseMemory(work)` and `db.clearCache(work)`
through the existing registered worker jobs and Effect cancellation/cleanup.
They are optional operations, not a new memory-management mode. The mandatory
`work` arguments remain part of the pending unrestricted-API cutover. Log
readers return the same prepared handle. The native test checks that release
preserves the exact prepared object, even after closing its original snapshot;
owned completed results survive query release, cache clearing and closure.

Two isolated allocation-counter runs repeated exactly
(`26283099-7b0f-4dfe-a117-2adc29f424c7`,
`7c2d69f5-ec8e-4ff8-90b6-a674fd09d945`):

| Measured owner/workload | Ordinary retained state | After explicit release |
| --- | ---: | ---: |
| Join pools, 8,192 input rows | 159,008 bytes warm | 0 bytes |
| Pack directory + nested claims, 8,192 claims then one claim | 132,608 bytes after both large and small calls | 0 bytes |

The Pack sink's complete release window freed 1,284,406 allocation bytes,
including its maps and other buffers, with zero allocation requests. This is
a within-candidate reset-versus-release comparison, not an old-version timing
benchmark. Compiled metadata and the allocator's own accounting are excluded
from the owner-capacity figures; neither those figures nor deallocation
requests establish an RSS reduction. The query test also verifies unchanged
shared-cache identity/membership, identical answers after reuse and idempotent
release. Additional regressions cover exact dense float folds, spills with
cancelled work, unique and hashed projection routes, computed outputs,
recursive/interior plans and rejection of pre-release iteration tokens.

The combined candidate passed 203 focused engine tests
(`80e751c0-1c51-475b-a99a-c3dfd31f250c`), strict workspace and native
all-target/all-feature Clippy, all 101 native tests under the CI profile
(`0c97e74b-5e51-45c1-97de-b47a2f360288`), and 2,555 workspace tests with 33
skipped and no cleanup warning (`6e469f80-996c-4f61-b0aa-0fc2aeec7483`). Both
SDKs passed complete builds/package checks, tests, type checks and lint:
219 core tests and 168 log tests. The macOS addon SHA-256 is
`ed555498ae05e2a090b33dc6f6022480212d7723aac76bf437620f4851840a88`.
Logs are in `bench-out/allocation-cleanup.QCvqv9/release-sdk.log` and
`release-rust-verification.log`. The release warm-allocation gate passed
(`24a77e56-35cb-48c8-9fa3-991adbed2a2d`), as did workspace doctests (37 passed,
one ignored) and Rust documentation with warnings denied. Formatting and
`git diff --check` passed. No new Miri run was performed in this stage.

The source delta from `d62a0b5c79f1b6be012510b48ddf876fe674b2f7` hashes to
`bc64832576b3c2bd79d4385a14d1a55ce41fa84d633ccc50c6c93936cad0ef48`:
`git diff` over `crates/bumbledb`, `crates/bumbledb-log`, `ts/crate`, `ts/src`,
`ts/test`, `ts-log/src`, and `ts-log/test`, followed by the `git diff --no-index`
addition of `crates/bumbledb/src/exec/sink/tests/memory.rs`. The hash was
unchanged before and after the native/workspace test runs. As with the previous
delta, native-artifact helper files and documentation are separate. This is
not a committed or cross-platform-qualified revision. No new traces or full
benchmark suite were run. The exact delta is saved as
`bench-out/allocation-cleanup.QCvqv9/release-candidate.patch`.

### Text reclamation: image-owner cutover in progress

A new parameter-churn regression reproduced historical interner retention:
128 distinct absent parameters of about 1 KiB, against unchanged live data,
left 140,886 ledger-accounted interner bytes and the first obsolete token
resolvable (`7aa393d7-d0c9-4a9c-9881-2828fed4405f`). This is the existing
payload/ledger estimate, not a complete allocation or RSS measurement.

The current local foundation replaces the append-only token vector with a
removable token index. Token numbers are monotonic within a namespace and
never reused after collection; crossing the resident-token range refuses
before minting or charging an entry. Each image now shares one canonical
string owner per distinct resident text, acquired under the resolver lock.
It does not clone full strings or keep another image's text alive. Derived
images acquire owners as their existing row drain copies the tokens. A unique
refill reuses the ownership table and drops strings absent from its new rows;
a reader holding the old image forces separate storage and keeps its original
text. Generation changes cannot confuse equal token bits, and a failed drain
leaves no published partial image or retained partial text owners.

**Automatic reclamation is not enabled yet.** The reclamation operation is
currently test-only: cached literals, parameters/sets, selection keys,
row/fallback state, and scratch aliases still need complete ownership coverage
before concurrent reclamation is safe. The end-to-end parameter-churn test
still fails, unchanged at 140,886 bytes
(`de70fd8f-ed25-4426-ab58-93d149a9063b`). It is neither ignored nor weakened.
No production retention reduction is claimed for this staged cutover.

Seven image-owner tests cover distinct-text rather than per-row ownership,
independent images and readers, derived-row ownership after source release,
large-to-small reuse, namespace changes, failed drains, missing resident text,
and concurrent collection with live owners. They and the nine interner tests
passed (`7179e197-36b9-4f6b-b01c-9926f8ec582f`). In the derived-image fixture,
explicit test collection after shrinking 1,024 distinct strings to one leaves
one text entry, and 32 subsequent same-shape refills make **zero allocation
requests / zero allocated bytes**. The ownership table and slabs deliberately
retain capacity until their owner is released. This is a within-candidate
ownership/allocation check, not a before/after timing or RSS comparison.

A broader engine selection passed 263 tests
(`0adf5eb7-3c92-4624-b9e8-6df4c866d09e`), excluding the explicitly unfinished
parameter-churn regression. Strict workspace all-target/all-feature Clippy
passed after fixing a new test's confusingly similar variable names. The
existing allocation-scaling/warm-reuse integration gate also passed in the
debug test build (`248af1e5-68a4-48ed-9050-aeb4b2ffd157`). The extra distinct-text
ownership table and hash-based token lookup need integrated allocation/timing
qualification; passing warm-refill allocation checks does not establish their
overall cost. No new traces or full benchmark suite ran during this stage.

The source delta from the starting revision is saved in
`bench-out/allocation-cleanup.QCvqv9/text-image-candidate.patch`, SHA-256
`d29618d00ea9c4aad77ff59cc7684931934daffd12cf21ee8780c1b8742d6da5`.
It uses the same source paths as the scoped-release delta above, followed by
the additions of `crates/bumbledb/src/exec/sink/tests/memory.rs` and
`crates/bumbledb/src/image/tests/text_owners.rs`, in that order. Documentation
and the separate native-artifact helper remain outside this source delta.
The saved patch hash matches the live delta after the final focused test run.
This stage has not been committed, pushed, or integrated across the native SDK.

### Text reclamation: resolved query values and decoded rows

The next local cutover gives resolved scalar text a canonical shared owner
(`Const::Text`). Resolved sets and selection keys retain their resident text
owners alongside the execution words. Filters, active/parked view bindings,
fallback resolution tables, and key-probe scratch now copy those owners with
their words. Set ownership is deduplicated by token. The executor still borrows
ordinary word slices, and the existing vectorized resident text-filter path
is preserved. This does not change the query language or the SDK surface.

Scalar parameter memos now share the canonical string rather than keeping
a separate `String` copy. Pointer-identity tests establish sharing between the
dictionary, resolved parameter, and memo; query release drops these owners
while an unrelated cached image's text remains live. These checks establish
ownership, not an aggregate heap or RSS reduction. Text-set storage is boxed
so the scalar constant remains 32 bytes on the supported 64-bit targets;
that adds a cold set-container allocation whose overall cost still needs
controlled qualification.

Decoded probe/fallback rows retain their text until replacement or release.
A malformed or refused decode clears both partial words and partial owners.
The new row test checks replacement, a malformed trailing-byte failure, and
subsequent reuse. It does **not** establish ownership of tokens copied out of
a row into a later sink or scratch stage.

A regression caught an allocation problem in the first version of the new
resolved-word container: derived `Clone::clone_from` replaced both vectors.
Thirty-two filter-plus-selection updates made 128 allocations / 4,096 allocated
bytes (`5b80cdfc-3918-4f24-a25c-9b548baae460`). Explicit field-wise cloning into
the existing vectors reduced that window to zero allocations / zero bytes.
All four new ownership/reuse tests passed
(`4e24bcf8-5e8b-46a9-a315-908141002770`). This was a regression in the initial
local cutover, not a previously published defect or a release speedup.

The focused engine selection plus existing warm-allocation/scaling gate passed
295 tests (`fdf17424-8820-4eba-9d93-17b63349f3c9`), excluding the still-unfinished
automatic parameter-churn regression. Strict workspace all-target/all-feature
Clippy passed after the value-owner changes. A subsequent review removed an
unnecessary temporary set-box allocation from the filter writer. Strict native
all-target/all-feature Clippy also passed against that resulting source.

The workspace run passed 2,574 tests with 33 skipped but reported one
subprocess-cleanup warning in the dense-interval arithmetic oracle
(`1dfdb7ae-afa5-4a3c-aa17-d18025eb3619`). The inspected test body starts no
processes, and no matching test/runner process remained afterward; the cause
of the warning is unknown. A complete rerun, including the filter-writer
cleanup, passed all 2,574 selected tests with 33 skipped and no cleanup warning
(`54940279-bf75-45d8-8bfd-128ae54662c9`). Both selections explicitly exclude
the unfinished automatic parameter-churn test; neither is a fully green goal.
All 101 native Rust tests also passed with allocation counters and the root CI
profile (`f1105d4f-8bc9-49f3-b0c7-d4acbc70537f`). The JavaScript package builds
and full SDK test suites have not been rerun for this internal cutover.

The source delta is saved in
`bench-out/allocation-cleanup.QCvqv9/text-query-candidate.patch`, SHA-256
`f06586c2f14b5e26422b90139d76705222fb132bc4ca7421f2bc6462a90722c3`.
It uses the same paths and appended untracked-test order as the image-owner
delta. The saved hash matches the live delta before and after the final
workspace rerun. Documentation and the native-artifact helper are separate.

Reclamation remains test-only. Temporary sink/scratch-stage token ownership,
scratch alias lifetime, automatic scheduling and real concurrent execution
churn still need completion or removal of their obsolete quota-driven paths.
The end-to-end parameter-churn regression was rerun against this delta and
still fails at 140,886 ledger-accounted bytes
(`563e8305-75a7-4e14-9e55-e534f52e5661`); no production collector is being
claimed. Nonresident text is opened by resident-admission refusal, so its
namespace/alias machinery overlaps the required unrestricted-allocation
cutover. Cursor fallback also handles the resident image's `u32` position
representation limit, which is a separate responsibility and must survive
quota removal. Do not conflate removal of admission policies with deletion
of every fallback or shared scratch capability.
No new trace capture or full benchmark suite ran during this stage.

The full goal remains incomplete: automatic text reclamation, unrestricted
APIs, spill simplification, controlled timing and broader qualification,
commit/push and exact-revision CI are still outstanding.

### Unrestricted allocation cutover: runtime aggregate admission removed

The runtime no longer adds up each operation's declared input, working,
scratch and result byte ceilings before admitting it. The aggregate ledger,
per-operation aggregate receipt, retained-route byte receipt and directory
byte receipt are deleted, including their configuration and inspection fields
in Rust, N-API and TypeScript. Directory acquisition now uses the existing
owner-slot constructor instead of duplicating its admission implementation.
Inspection reports outstanding work and handles, not a purported memory total.
Worker/queue limits, retained-operation slots, native-handle limits, cleanup
lanes, cancellation and ownership through output transfer remain intact.

The new Rust and actual-addon JavaScript regressions retain three tiny completed
operations whose declared byte ceilings cannot even be summed in `u64`. They
complete and remain individually takeable. Rust additionally proves that the
next job refuses before input preparation when all outstanding-operation slots
are occupied, and admission works again after taking the completed results.
Native handle-capacity tests retain their refusal/rollback assertions, now
filling actual handle slots instead of forcing the deleted aggregate-byte gate.
These are executed correctness checks of the policy deletion, not a measured
allocation or throughput improvement; no baseline allocation/timing comparison
was run for this step. Existing CPU traces do not measure these admission costs.

All 102 native Rust tests passed with allocation counters and the root CI profile
(`86c45e5d-a57c-4c87-9d31-8c7598cb05fb`); strict native all-target/all-feature
Clippy also passed. Both package builds, including staged-tarball validation,
passed. The full SDK suites passed **220 core / 168 log tests**, with no skips;
both SDKs passed type checking and lint. The first log run passed 167/168:
the API-fixture edit had removed a mixed-property line containing the child
process's still-required `chunkBytes`. Restoring that property and removing
only the obsolete byte fields made the complete suite pass; the cross-process
kernel-lock assertions and their timing were not weakened.

The final tracked source delta from `d62a0b5` is saved as
`bench-out/allocation-cleanup.QCvqv9/runtime-admission-verified.patch`, SHA-256
`3ebf47f8f5066e4cc9fd30d7c76ba078aa142835d380c3356b34fca9980a8042`.
It covers core/log Rust, native and SDK code/tests/scripts, and examples.
The four untracked additions have separate patches; their hashes and the
rebuilt addon identity are recorded in `runtime-admission-evidence.json` in
that directory. The earlier `runtime-admission-candidate.patch` preserves the
source with the failing child fixture, not the final verified version.
Documentation is outside these source snapshots.

This completes only the aggregate-runtime-admission part of requirement 7.
Per-operation quotas, deadlines, result sizing charges, cache admission,
quota-driven text spill/restarts and the automatic-reclamation ownership gaps
above still need removal or completion. The original automatic text-churn test
remains unresolved; no collector was enabled. No new traces, full benchmarks,
commit, push, tag or release occurred during this step. Full-goal and exact-CI
completion are not claimed by these narrower passing checks.

### Unrestricted ownership and result cutover: not yet compiling

The next source delta removes per-operation byte/work/deadline accounting
from canonical rows, transaction drafts, change sets, shared scratch maps,
image slabs and judgment scratch. Canonical bytes move into their next owner;
ordered change records and snapshot collision ordering borrow existing rows.
Completed results now own one `Answers` and expose borrowed row/value
iterators. Delivery tickets visit bounded pages directly and advance only
after successful adoption. This is paged delivery of materialized results,
**not streaming query execution**. Cancellation polling is retained, including
chunked fixed-column finalization and failed/aborted page attempts.

This is an incomplete cutover, not a verified improvement. The last core
library check failed with 12 errors in remaining quota-driven text machinery
and a deleted sink threshold default. Core test compilation and native/SDK
consumers have not yet been migrated or rerun. Formatting and whitespace
checks passed; new ownership/delivery regressions have not run. Production
text reclamation remains disabled until raw-token lifetime gaps are closed.

The tracked transitional source from `d62a0b5` is preserved in
`bench-out/allocation-cleanup.QCvqv9/rows-results-cutover-source.patch`, SHA-256
`240c6f9d126ec81c2f8e3001b2b4ed8cec1455301f3a329162ae5c2fc7791874`.
Six new source/test files are outside that tracked patch. The installed native
addon and the preceding green verification belong to the earlier
runtime-admission stage, not this source. No allocation/timing comparison,
new trace, full benchmark, commit, push or release occurred in this step.

### Single text namespace and representation-only sink spill

The core library now compiles after the next part of the unrestricted cutover.
The cache ledger, resident-admission sum type, scratch text lookup/store,
tagged secondary token namespace, alias cache and text-triggered query restart
are deleted. Text interning now returns one shared canonical owner; images,
resolved constants and decoded rows retain those owners. Tokens never reuse a
number, and the reserved miss token is never minted. Explicit cache clearing
still rotates the resolver without invalidating live owners. The removed
`retained_cache_bytes` API was quota accounting, not an allocator measurement;
it is not replaced by a fabricated zero or RSS estimate.

Sink contexts now carry cancellation directly, without `SinkBudget` or a RAM
allowance. Deduplication retains its scratch algorithm at the actual `u32`
map-index limit, checking an existing key before that transition. Aggregate
partitions retain the corresponding representation-limit transition. The
estimated group/Pack-byte counters are removed. Recursion has no configured
round or derived-tuple ceiling and polls cancellation between frontiers. The
dead resident-then-fallback retry branch and derived-budget error vocabulary
are removed. Explicit recovery/staging disk responsibilities still need their
consumer audit; this is not a claim that the entire spill cutover is done.

The new public integration target `allocation-ownership` passed **3/3 tests,
no skips** under nextest's CI profile with allocation counters
(`6f997cdd-e55c-4855-ae01-aae585c7ef8b`). Its 513 distinct 4-KiB-text rows were
checked against their original payloads. Borrowed full-result iteration,
17-row page visitation and consuming transfer changed neither allocation nor
deallocation counters; consuming transfer also preserved the text pointer.
These are executed allocation regressions, not a controlled latency comparison
or a throughput claim. Separate tests proved that visitor failure, cancellation
and panic cannot commit a cursor prefix, and that repeated old/current snapshot
queries and completed results survive query-memory release plus cache clearing.
The snapshot test alternated old/current reads sixteen times after a real write.

Strict all-feature **core-library** Clippy and strict Clippy for this integration
target passed. This does not qualify all unit tests or other crates. The initial
full core test-compilation inventory reported 686 library-test errors from
obsolete APIs, before the subsequent interner-test rewrite; that is not the
remaining-error count of the final snapshot. The log is preserved as
`bench-out/allocation-cleanup.QCvqv9/text-cutover-test-check.log`. The later
workspace-library inventory reports **13 log / 13 benchmark-library errors**;
see `text-cutover-workspace-check.log` in the same directory. Benchmark code was
compiled for this inventory, but no benchmark was executed. Native/SDK consumers
still need migration and have not been requalified.

**Production text reclamation remains disabled.** The new interner and WordMap
unit regressions have not run: the unfinished full unit-test cutover prevents
their compilation. Cursor-emitted tokens, recursive accumulated state and
scratch-backed derived stages still need their complete lifetime proof/owners
before reclamation can be enabled. The original automatic text-churn regression
is neither removed nor declared fixed.

Tracked source from `d62a0b5` is preserved in
`bench-out/allocation-cleanup.QCvqv9/text-sink-cutover-source.patch`, SHA-256
`3d208d17db619d119ef3c5dee9c978583081f13ddb5104141191b68ed27f31b1`.
The two new files in this step are outside that tracked patch:
`image/text_eq.rs` has SHA-256
`dd24477f8bd552cc680210e29e35e9ceabdd29d080ef19e3ebed9bea8f428d27`;
`tests/allocation-ownership.rs` has SHA-256
`70777e75e7877bbe7942276607fdfcdf3127263be32cc297a6700e300f5aad0b`.
The six prior untracked additions also remain outside the tracked patch.
No new traces, full benchmarks, commit, push, tag or release occurred. The
installed native addon still belongs to the earlier runtime-admission stage.

### Ordinary received buffers and explicit disk staging

Received objects now have one representation: an ordinary `Vec<u8>`
(`ReceivedBody`), moved directly from the receive accumulator. The charged/plain
enum, conversion wrapper and reservation-only path arithmetic are gone. Capacity
grows geometrically within the input envelope; finishing neither copies nor
shrinks the buffer. Direct large pushes copy/hash in cancellation quanta, and
finishing checks cancellation too. Reference lengths, envelope bounds, exact
digests and typed transport observations remain validation, not memory quotas.
Verified composition also checks cancellation while hashing received chunks.

Checkpoint restore, backup tails and history visitors consume these ordinary
owners. The unused compatibility whole-tail collector and its export are
deleted. Migration relations and reverse recovery tails now choose temporary
LMDB on their **first insertion**, with no environment for an empty set. They
must not silently become database-sized RAM maps after threshold removal.
The shared scratch transition is exposed to those callers, and failures
propagate before publication. Its exact-key semantics, disk cleanup and
map-growth protocol are unchanged. The filesystem adapter's five-second
contended-lock wait is retained as scheduling backpressure, not an execution
deadline or an expiring owner lock.

The recovery discriminator now uses real allocation counters instead of the
removed reservation ledger. Its 16,384 rows span 97 chunks; the fixture uses
4-KiB chunk/import windows. A deliberately eager `collect::<Vec<_>>()` before
decoding failed the same assertion (`d5d98cc2-c117-45f7-b8b9-95f9306f579f`):
additional live allocation observed before successive fetches reached
**405,248 bytes**. The lazy version reached **21,704 bytes**, and passed
(`a729093b-d208-43e1-ba54-4e0d0e5097d1`). These process-allocation observations
include active import storage and the test adapter's retained request log;
they are not pure payload accounting, RSS, timing, or Pi qualification.
The negative-control collection was removed immediately after the run.

Receiver tests additionally establish bounded geometric growth for one-byte
chunks, identical pointer/capacity and no allocation on finishing, exact
deallocation on drop, cancellation between chunks and at finish, and digest,
length and empty-object checks. Decoder tests use drop-observed owned iterator
items, borrow complete records, and stop fetching on supplier/decoder/visitor
failure. Command-sealing tests preserve the original canonical payload pointer
and observe allocation-free hashing and final-owner release. A checkpoint
regression cancels at its first chunk upload and proves no checkpoint head
publishes; a fresh attempt succeeds. Disk-staging tests exercise the first
insertion, duplicate and wide keys, early visitor failure, forward tail replay
and cancellation before opening scratch.

Two initial allocation-test failures counted the memory store's intentionally
retained operation log as a buffer leak. Measuring the buffer's actual drop
boundary fixed the discriminator; no product leak or speedup is inferred from
those two failures. Obsolete quota-failure fixtures now use explicit cancellation
while retaining their no-publication/retry assertions. One vacuous, local
type-only test of the deleted charged-tail representation was removed; actual
restore/replay integration tests remain.

All **343 log tests passed** with allocation counters, including process/crash
lanes. Three separately ignored subprocess entrypoints are invoked by their
parent tests. Run `95d8ee9b-2134-4c49-98ad-ee01ef36dea4` reported one nextest
subprocess-output cleanup warning despite all assertions passing. The selected
output verbosity did not retain the affected test's identity. A full unchanged
rerun with every status retained (`26a0a475-c02b-44a4-99a9-6a0e6aee848d`) passed
343/343 without that warning; its cause remains unestablished. It is recorded,
not reclassified as a fixed bug. Strict log all-target/all-feature Clippy,
workspace-library Clippy, the log doctest and the three core public ownership
regressions also passed. The last core run was
`a769b008-ea5d-41d4-9ec1-2f39fc7aa557`, zero skips.

Benchmark-library charge-only instrumentation and its obsolete constructors
were removed rather than reporting fabricated zero measurements. Benchmark
code was compiled, not executed. Native restore's received-body type was
migrated, but the remaining native/SDK cutover is **not qualified**. Full core
unit-test migration, production text reclamation and its missing raw-token
owners, remaining log policy/format-boundary audit, integrated SDK verification,
commit/push and exact-revision CI remain open. No new traces, full benchmarks,
tag or release occurred.

Logs, the one-line eager negative-control patch and the source hash manifest
are in `bench-out/allocation-cleanup.QCvqv9/log-*`. Tracked source from
`d62a0b5` is preserved as `log-owned-cutover-source.patch`, SHA-256
`729b683bc00d10e3bdc7ae5aa7d605784a41b362e0f5f7176f9b0fef7cb322d9`.
`log-owned-cutover-sha256.txt` records the eight unchanged untracked additions
outside that patch. This remains a development snapshot, not a pushed revision.

### SDK consumers, iterator cleanup and storage diagnostics

The native bridge, both TypeScript SDKs, installed-package consumers and Notes
now use the unrestricted API. Obsolete positional execution policies and
quota-only doubles are removed; cancellation, durable evidence, repository
locks, Effect ownership and scheduling admission remain. In particular, Notes
still maps real scheduling/handle `ResourceLimit` errors to HTTP 429.

Migration seed ingestion acquires the caller's iterator once and processes at
most 512 rows before yielding. It lowers directly into the final document's
array, without an intermediate chunk and spread-copy. Interruption and invalid
input close a suspended iterator; exhaustion does not close it twice. A
20,000-row cancellation fixture pulls exactly 512 rows, closes once and writes
no files. Invalid input at row 513 likewise publishes no prefix. Removing only
the iterator finalizer makes both regressions fail (`closed = 0`); restoring it
makes both pass. Logs are `seed-iterator-cleanup-negative-control.log` and
`seed-iterator-cleanup-restored.log`. The final migration document still owns
all rows: this is incremental ingestion, not streaming serialization.

Core and log inspection now share `StorageInspection`: `virtualMapBytes`,
`populatedFileBytes`, `nonFreePageBytes`, and nullable `allocatedDiskBytes`.
These are storage measurements, not heap or resident RAM. The old resident
label actually described non-free LMDB pages. Unknown allocated-block counts
now remain null rather than silently substituting file length. Real-addon
tests compare file length and allocated blocks with filesystem metadata. The
unenforced tenant-cache byte budget and literal-zero slot disk metric are
removed; actual `maxOpen` admission and borrowed-slot eviction safety remain.
Cache open count and slot inventory are observed under the same registry lock.

Final local qualification completed successfully:

- Native all-target/all-feature strict Clippy and **101/101 tests**, run
  `dca34b61-0603-4b95-b9f1-ece13240847d`.
- Core TypeScript typecheck, lint and **220/220 tests**; log TypeScript
  typecheck, lint and **170/170 tests**.
- Installed tarball declaration checks and runtime consumers, addon-unavailable
  pure authoring, standalone Rust consumer, generated Notes migrations and
  **8/8 Notes route tests**, on darwin-arm64 only.
- Workspace-library all-feature strict Clippy and **3/3 public core ownership
  regressions**, run `84d042aa-a787-45fd-8251-399a0797a801`.

Native allocation windows observed 513 requests / 45,056 bytes for 512
two-scalar rows, 257 / 142,336 bytes for 128 × 1,024-byte text rows, and
259 / 8,465,496 bytes for 129 × 65,536-byte text rows. Repeated collection has
identical allocation request counts and bytes. Sixteen 512-row queries use
2,368 requests / 910,672 bytes one-shot versus 16 / 196,608 bytes explicitly
prepared. These are isolated-process allocation observations, not timing,
RSS, historical benchmark or Pi comparisons. Some windows include small
unrelated frees; they do not establish leaks or exact net-live changes.

Logs and the cumulative tracked source patch are under
`bench-out/allocation-cleanup.QCvqv9/`: `native-sdk-final-qualification.log`,
`native-sdk-final-build.log`, `sdk-consumers-final-qualification.log`,
`workspace-libs-ownership-qualification.log`, and
`sdk-consumers-cutover-source.patch`. The patch is against `d62a0b5`, excludes
this evidence document, and has SHA-256
`ddfbe5ab11a6bdce0f9e8bfcf22a7728d438eccd7106a8c7bea021650d991398`.
`sdk-consumers-cutover-sha256.txt` also records the eight untracked additions
outside the patch. The rebuilt darwin-arm64 addon has SHA-256
`8311c0a7f68e31a3c71ae55d7fe2aff3b823b4cb5d8565c1933a53223f211689`.

This does **not** qualify the complete workspace: core unit tests still have
566 compile errors from the unfinished API migration, and automatic text
reclamation remains disabled pending the raw-token ownership gaps. The full
Notes standalone typecheck and other platforms have not run. No new traces,
full benchmarks, commit, push, tag or release occurred. The one stale native
rustdoc link removed after this snapshot is documentation-only and is not
included in its hash.

### Core test cutover checkpoint — September 8, 2026

The next source checkpoint migrates core COLT, scratch, sink, image, text,
pending-write and binding regressions away from deleted quota APIs. Tests now
use explicit cancellation, explicit scratch transitions, exact independent
row/numeric oracles, shared-owner lifetimes and real allocation observations.
Policy-only cases for mechanisms deliberately removed by this cutover are
deleted, not restored behind fake unlimited allowances. Lazy scratch append
tests check exact source pulls and distinguish a committed complete batch from
an abandoned tail. Completed sink drains are not streaming query execution.

At this initial checkpoint the newly migrated tests had not executed:
`core-unrestricted-stage-compile.log` reported 260 errors and 23 warnings.
This checkpoint does not inherit a green qualification from the earlier SDK
build. Automatic text reclamation remained disabled pending raw-token owners.

`bench-out/allocation-cleanup.QCvqv9/core-tests-cutover-source.patch` captures
tracked source against `d62a0b5`, excluding this evidence document, with SHA-256
`5df3505db17e810a6e9aa4219588361c598c9e9627070abcdda9fa0c8c2374b5`.
The adjacent `core-tests-cutover-sha256.txt` also records untracked source and
the earlier addon. No timing, new trace, full benchmark, commit or push is
claimed by this checkpoint.

### Core test execution checkpoint — September 8, 2026

Continued consumer migration cleared the compile errors (260 → 196 → 109 →
13 → zero). The all-feature core library test run then executed **1,322 tests:
1,313 passed, nine failed, 18 skipped** (nextest exit 100). No test was disabled.
The failures comprise the still-disabled automatic text reclamation regression,
three temporary-allocation windows including lazy schema compilation, a stale
full-reference visit counter, two deleted-quota refusal expectations, and two
reader-blocked resize expectations now driven by host cancellation. The latter
eight are being investigated and repaired, not treated as passing results.

This run used `nightly-2026-08-15` and the measurement lock. Its log is
`bench-out/allocation-cleanup.QCvqv9/core-unit-cutover-tests.log`; the exact tracked
source against `d62a0b5`, excluding this document, is
`core-tests-execution-source.patch`, SHA-256
`543349dca1d7aa26269118f75013ee118f17b5d145dc605d98f8dfff35f1ca75`.
The adjacent `core-tests-execution-sha256.txt` identifies the untracked sources.
Test durations are not performance comparisons. Full workspace qualification,
automatic text reclamation, commit, push and exact-revision CI remained unfinished
at that checkpoint.

### Automatic text reclamation and live owners — September 8, 2026

The eight migrated-test failures were repaired without disabling tests. The
three judgment allocation windows now exclude the schema's explicitly warmed
shared compilation; the no-scan doubles and exact temporary-release assertions
remain. Host-cancelled resize returns cancellation, restores admission, and
preserves the pinned reader and its age/count diagnostics. The repeated full
core run passed 1,321 tests, leaving only the real text-churn failure.

Automatic reclamation is now active. A round-robin queue of token IDs visits
at most 64 entries after each 16 new texts; hits neither sweep nor allocate.
Only entries with no consumer-owned canonical text are removed. Full-byte
identity, monotone non-reused tokens, generation identity, and atomic
intern-and-pin remain intact. There is no cache byte allowance or global flush.

Cursor-fed sinks retain text only after predicates accept a binding, and only
for output/dedup slots the sink needs. This includes non-output text in exact
aggregate dedup keys. Sealed scratch stages pin their own text during the
existing row drain, with no second payload collection; recursive accumulators
retain text across frontier reuse. Resident numeric scan and batch kernels are
unchanged. These are ownership fixes, not a claim of streaming query results.

The first automatic-reclamation source passed **1,322/1,322 core library tests**,
with 18 existing skips. Its tracked-source patch is `text-auto-reclaim-source.patch`,
SHA-256 `4796962890db334ec0a128da95af1bbc2f6624b89f77de1c63f05263b49c9023`.
Six new regressions then passed. Temporarily withholding the new fallback,
scratch-stage and recursive-accumulator text owners made all four targeted
ownership regressions fail; the fallback output case reported a dangling token.
Those negative-control changes were restored before positive qualification.

The expanded all-feature core suite passed **1,328/1,328**, with 18 existing
skips. A separate allocation regression observed canonical interner payload
of **3,115 bytes after 128 distinct ~1 KiB parameters**, versus **132,630 bytes**
in the earlier failed, reclamation-disabled checkpoint. On the new source,
actual process live heap was **36,511 bytes after 128 parameters** and **38,111
bytes after 4,224**: another 4,096 historical parameters added 1,600 retained
bytes, not their roughly 4 MiB payload. This measures the isolated test process,
including its fixture, not database RSS, mmap residency, or a throughput gain.

All evidence remains in `bench-out/allocation-cleanup.QCvqv9/`: the negative
control log is `text-owner-negative-control.log`; the final positive logs are
`text-reclaim-full-core.log` and `text-churn-live-heap.log`. Their tracked source
against `d62a0b5`, excluding this document, is `text-reclaim-full-source.patch`,
SHA-256 `93b831070e506381f5994ce70764ddaf3cd8a6cfdf81615868e08ce255668fa9`.
The new untracked `api/prepared/tests/text_retention.rs` SHA-256 was
`fe6a0d9f047b0b4930571a4e6493b0d6b58a420c54cd1291b683895100790d29`;
The other eight untracked sources were not individually rehashed at this
checkpoint; the earlier blanket claim that they all matched the preceding
manifest was not established. Subsequent owner-type naming and fixture cleanup
are not included in these hashes. Strict all-target Clippy and the integration-test migration are still
in progress. No new trace, full benchmark, commit, push, tag or release occurred.

### Core integration qualification checkpoint — September 8, 2026

Strict core all-target/all-feature Clippy passed after fixture cleanup. The
full core package then ran 1,518 tests across 27 binaries: 1,517 passed, one
failed, and 20 were skipped. The remaining close-under-load test still expected
an ordinary context to expire; it now explicitly requests cancellation while
holding its reader. The complete rerun passed **1,518/1,518**, with the same
20 skips (`core-integrated-tests-repaired.log`). All cleanup and retained-reader
assertions remain. The initial run also reported a nextest process-cleanup
`LEAK` warning for `cloned_contexts_observe_cancellation`; its source creates no
thread or child process, and the complete rerun was clean. The original warning
remains recorded; its cause has not been established.

Integration allocation gates now observe actual allocation requests/bytes,
explicitly not physical-read counters; core no-scan doubles remain separate.
Dynamic read owners are checked for independent deallocation after their read
frames close. The deleted reservation ledger's isolated accounting test was
removed with that mechanism. A formerly quota-forced scratch test now verifies
that ordinary complete judgment succeeds with an unusable temporary directory,
then aborts without publication; explicit scratch I/O/cancellation regressions
remain in the core suite.

The initial integration snapshot against `d62a0b5`, excluding this document,
is `core-integrated-source.patch`, SHA-256
`7c6b49dc8bdcb12b6b7d66036f0978adf3fe36df578f045bb721ae96a01f4d30`.
Untracked `canonical/ownership.rs` was
`603eaaf258b3ac260b73a97fae2d7fd85949a66d1628a0146da6ca0003a2e3fb`, and
`api/prepared/tests/text_retention.rs` was
`d9ef5f85b4faf12ed042c1445d6187480eb2b3a7e6087a31de379af656e1a25f`.
The remaining seven untracked sources were not individually captured here;
in particular, the sink-memory and image-text-owner tests also changed during
fixture migration. Their current hashes are recorded in the integrated-battery
manifest below, not retroactively certified for earlier runs. The later
explicit-cancellation test repair and obsolete production-rustdoc cleanup are
not included in that source hash. Workspace doctests and warning-free Rust
documentation subsequently passed; the feature/release-allocation check script
is still running. This is not full workspace or exact-pushed-revision CI
qualification, and nothing has been committed or pushed in this cutover yet.

### Iterator and policy completion — September 8, 2026

The same allocation-site audit found duplicate-triggered `WordSet` growth and
a failed-rehash mutation analogous to the earlier `WordMap` issue. Both new
tests failed before their fixes (`distinct-growth-negative-control.log`): a
duplicate at half capacity allocated another 256-byte table, and cancellation
during rehash replaced the original set with partial state. Lookup now precedes
growth; rehash builds locally and publishes only after the cancellation check.
Both regressions pass, including zero allocation/free on 4,096 duplicate inserts
and intact contents with successful reuse after interrupted growth.

Scratch-backed nested joins now reuse one encoded/decoded row buffer per depth.
Negation uses the existing borrowed, early-stoppable visitor directly. A
discriminating negative control restored fresh encoded storage per row and
failed: 2,048 reads requested 2,048 allocations / 16,384 bytes. The reusable
RAM-backed path requests **zero** after warm-up. Both RAM and disk tests verify
independent nested rows, missing/malformed rows, early stop before later
corruption, and consumer cancellation without poisoning the producer. All
22 selected statistics/derived/recursive-stage tests passed with allocation
counters (`distinct-and-scratch-qualification.log`). The negative edit was
removed before integrated qualification. These are allocation tests, not
throughput measurements or a claim of streaming query execution.

The policy audit removed the GC mark-memory quota and unrelated fixed walk
limits in GC, recovery and checkpoint suffix validation. These walks use the
captured sequence distance; validation and cancellation still stop malformed
or abandoned work. A cancellation-during-fetch regression proves GC publishes
no incomplete mark certificate and a fresh operation marks the entire closure.
Input/format envelopes, representation widths, bounded copy/delivery windows,
queue/cleanup admission, publication contention retries and the hosted log's
durable-tail/checkpoint scheduling policy remain separate responsibilities;
none is reported as a heap/RSS limit. No custom allocator or pressure framework
was introduced.

The old 66,000-edge round-budget test now verifies every endpoint in the full
finite closure, rather than expecting a deleted error. The harness passed all
520 tests and its renderer selftest. Its first run had a process-output cleanup
warning for `calendar::tests::chains_are_valid_under_the_pointwise_key`; the
later complete harness run had none. The initial integrated battery passed
**2,512 workspace tests** (33 existing skips), 37 doctests (one ignored), strict
workspace/native linting, warning-free Rust docs, and the release warm-allocation
gate. The `ground-off` variant passed 1,508 tests (21 skips) but reported a
cleanup warning for `alternating_param_envelopes_reuse_the_pools_correctly`.
These warnings concern nextest's post-exit stdout/stderr closure check, not
heap-allocation measurements; their cause remains unestablished. No timeout or
retry policy was loosened.

Lean built and all 277 comparison cases agreed. The correspondence census then
correctly refused references to the deleted round-budget error/test. The
bridge references now name explicit cancellation, complete finite closure and
their current regressions; no theorem was removed or weakened. The initial
battery stopped there, before SDK and packed-consumer qualification. Its
tracked source patch against `d62a0b5`, excluding this document, is
`integrated-battery-source.patch`, SHA-256
`029ed371165be47861dd48ca1f101e016019d618d4f99088f2a62af76e829bf9`.
All nine untracked source hashes are recorded in
`bench-out/allocation-cleanup.QCvqv9/integrated-battery-sha256.txt`.
The subsequent Lean-reference and native-rustdoc cleanup are not included in
that snapshot. Full integrated verification and exact-pushed-revision CI remain
open. No new traces, full benchmarks, tag or release occurred.

### Final wrap-up audit — September 8, 2026

The research loop is stopped. This cutover completes the eight implementation
requirements without a new allocator, execution quota or pressure framework:

1. One-shot native preparation drops after execution; scoped Rust/TypeScript
   preparations reuse one plan and release independently of snapshots/results.
2. Recovery and hosted migration fetch, verify, import and release checkpoint
   chunks lazily. Complete records borrow their input; split records use carry.
3. Hashed projections gather at most 256 rows per reusable window. Proven-unique
   output remains direct. WordMap and WordSet probe duplicates before growth;
   cancelled WordSet rehashing leaves the original map intact.
4. Explicit query-memory release drops nested execution storage while keeping
   preparation. Database-cache clearing is separate; retained readers and
   completed results stay valid. Ordinary reset preserves reusable capacity.
5. Automatic round-robin text reclamation preserves canonical live owners and
   never reuses token IDs. Churn, large-to-small reuse, concurrent reclamation,
   derived images, aggregates, scratch and completed results have regressions.
6. Row sealing borrows ordered storage. Native delivery visits the completed
   backing directly into its final output. Scratch scans reuse buffers and
   negation stops through borrowed visitors. None is called streaming execution.
7. Core and both SDKs use ordinary unrestricted allocation and cooperative
   cancellation. Obsolete quota-only code and consumers are removed. The final
   audit also removed `FlatRows.bytes` and `cellBytes`: every encoded cell was
   still doing unused bigint accounting after all consumers had stopped reading
   it. The shape regression failed before removal and passed after; iterator
   failure cleanup and owned byte-cell copies remain covered.
8. Spill remains for representation boundaries and explicit durable staging,
   not estimated heap allowances. Input envelopes, checked widths, scheduling
   admission and hosted durable-tail/checkpoint policy retain their own roles.

The repaired integrated battery completed on darwin-arm64: 2,512 workspace
tests (33 skips), 37 doctests (one ignored), strict workspace/native lint,
warning-free Rust documentation, the release warm-allocation gate, 1,508
ground-off tests (21 skips), 520 harness tests (eight skips), Lean's 277
agreeing comparisons and 120-row/367-reference census, 100 default-feature
native tests, 220 core TypeScript tests, 170 log TypeScript tests, both SDK
typechecks/lints, and installed-package/Rust/Notes consumers. Its log is
`integrated-battery-repaired.log`. Ground-off emitted one process-output
cleanup warning for `cancelled_reach_publishes_no_prefix_and_allows_reuse`.
Three isolated reruns passed without that warning; the cause remains unknown.
No timeout, retry, assertion or declaration check was relaxed.

The final focused Rust run passed 46 ownership/map/scratch tests with all
features, 11 recovery tests with allocation counters, all 101 native tests
with allocation counters, and three Miri tests covering duplicate-boundary,
panicking-constructor and scalar/bulk-growth behavior. Evidence is in
`wrap-ownership-qualification.log`. The SDK cleanup passed 222 core tests,
170 log tests and both SDK typechecks/lints (`wrap-sdk-packages.log`); that
run then exposed previously unchecked Notes application/dependency type errors.

The package gate now checks the entire Notes app and production build, not
only its route specimens. Repairs cover runtime-layer error responses, UUID
generator typing, immutable registry updates and validated JSON types. Next.js
and Alchemy pins now match the example's API, missing declaration dependencies
are explicit, and a narrow Alchemy declaration patch fixes its deleting-state
optional attribute without disabling library checks or changing runtime code.
The repaired packaged app passed its full typecheck, nine route/specimen tests
and production build (`wrap-notes-repaired-2.log`). Alchemy deployment, actual
S3/IAM and Raspberry Pi performance remain unqualified; building the example
does not establish them.

`wrap-up-source.patch` includes every staged source addition/deletion against
`d62a0b5`, excluding only this evidence document, with SHA-256
`0dd2aa6206571323f376d6772d792eb721e212f13d2abdd29c86958aaaa9dc88`.
This snapshot also aligns the Notes compiler settings with the successful
Next.js build. The current darwin-arm64 addon is
`ed04418c5ad189faefc587e593153a7aa1e6cb1a02083f4f227ecdfa59475449`;
unchanged native inputs are rechecked before refreshing package provenance.
All logs and the patch are under `bench-out/allocation-cleanup.QCvqv9/`.
Final staged-source checks passed (`wrap-staged-final.log`): strict
all-target/all-feature workspace and native lint, formatting, a matching
release addon, isolated package declarations/consumers, and the complete
Notes typecheck, nine tests and production build. Next.js no longer rewrites
the copied compiler settings. This host evidence does not replace CI for the
exact pushed revision. No new performance claim, benchmark suite, native
trace, tag or publication is part of this wrap-up.

## Shared scratch-owner hardening — September 7, 2026

Five negative-control tests failed before the accounting fixes, exposing three
defect classes:

- RAM upsert growth was counted twice.
- Disk writes could be refused despite fitting already-reserved capacity.
- Rounded reservations and spill overlap could exceed a narrower scratch policy.

Growth is now counted once, paid disk capacity is reusable, and reservations
atomically enforce the smaller of the caller's cap and the operation cap.
Tests cover rollback, spill failure, cancellation, concurrent reservations,
retained capacity and exact refunds. Durable formats and public APIs did not
change; no unsafe code was added.

The same change retains the earlier shared insertion optimization: a fresh
inline staged key uses LMDB's insert-if-absent operation instead of separate
lookup and insertion traversals. Existing-key replacement, wide-key collisions,
insertion order and transaction retry semantics remain covered.

### Local source validation

The combined source passed strict workspace all-target/all-feature Clippy,
2,545 tests (32 skipped), 37 doctests (one ignored), three focused Miri tests,
and the release warm-allocation contract. One nextest cleanup warning occurred
in a generated enum roundtrip test; three isolated reruns passed cleanly. Its
cause is unknown, and the original warning is retained.

The local suite completed at **22:38:50 UTC**, including 32 read families and
34 scenario queries. Source/binary identity and report digests were checked.

This does not qualify the manifest's six separate prerequisites, turn the
informational read latency budget green, or replace cross-platform CI.
Recorded clock contamination and capped comparisons remain limitations.

### Matched large-result timing

The test constructs and delivers 100,000 rows per operation through the real
completed-result and paged-delivery path. Each of 12 fresh processes measured
32 operations (3.2 million delivered rows), with a fresh canonical-row oracle.
The order was A/A, ABBA, BAAB, A/A; A is the baseline and B the candidate.

| Comparison | Mean ratio | Median ratio | Execution median ratio | Delivery median ratio |
| --- | ---: | ---: | ---: | ---: |
| Initial A/A | 0.9943 | 0.9888 | 1.0036 | 0.9119 |
| ABBA B/A | 0.8198 | 0.8224 | 0.7911 | 1.0346 |
| BAAB B/A | 0.7985 | 0.8036 | 0.7758 | 0.9788 |
| Final A/A | 1.0064 | 0.9961 | 0.9931 | 1.0278 |

These controls support approximately **18–20% lower end-to-end time for this
large-result workload**, with execution medians 21–22% lower. Delivery has no
established improvement. This is shared-host, scheduler-boosted timing, not an
isolated-host guarantee, confidence interval, or database-wide speedup claim.

### Broader results remain unqualified

The full run showed historical slowdowns in several reads and scenarios.
Those runs were not alternated, and several scaling cells were clock-flagged.
A follow-up four-family read experiment was stopped after its initial
identical-binary control showed **6.59× mean / 7.08× median drift** in the
24-byte displaced-probe lane, with severe clock contamination even after retry.

Both fresh dataset verifications and three baseline processes completed; the
first candidate process was deliberately terminated. All raw evidence is
retained as **inconclusive**, not passed. No completed candidate comparison
exists from that aborted experiment. It establishes neither broad neutrality
nor a candidate regression. The separate completed large-result controls above
are not replaced by these observations.

### Measurement identity

- Base source: `df737a6f290edf306a9806aacfa796abdd91c085`.
- Combined engine patch SHA-256: `b0c8f7d44cf2eaa0150d7b3e73b73de64d84f350d74716191c65b012bc8b9a19`.
- Baseline executable: `f25c5bfcb2fc89bb24c4eef351c2914a89f32353f8247810499311acc1ae9bf0` (byte-identical to the published 1.0.1 measurement executable).
- Candidate executable: `462143bb1f3f0450fd7fc83932c11bac7f1953529291676c1d448867dd383ce3`.
- Host: Apple M2 Max; pinned `nightly-2026-08-15`; ordinary optimized release build.

Raw patches, reports, logs and experiment dispositions are retained locally
under `bench-out/autoresearch-90-scratch-budget`. All heavy work was serialized
through the repository measurement lock. No release, tag or npm publication
is part of this development round.

## Historical resident-memory proposal — superseded

The quota-based experiment below is preserved as historical evidence only.
Its proposed ledger/admission requirements were rejected and must not be
restored as requirements of the unrestricted-allocation cleanup above.

The scratch changes above are committed and pushed as
`bca930fb95a4713222324e3693e337a750941b14`. A subsequent source audit found
resident query buffers that can grow without working-byte admission. Twelve
regression tests compiled and failed at their expected assertions on that
commit. They cover projections, aggregate groups, exact-float and Pack
storage, shared allowances, retained charges, and scan gathering. These are
related accounting gaps, not twelve independent defects or evidence of public
data corruption. No production fix has been implemented.

The tests are parked outside the active suite while implementation is paused.
Their exact [source](experiments/resident-budget/resident_budget.rs.txt),
[module binding patch](experiments/resident-budget/binding.patch),
[run identity and failure roster](experiments/resident-budget/run.json), and
[original test output](experiments/resident-budget/tests.log) are preserved.
The run finished September 7, 2026 at 22:50:10 UTC with nextest exit 100.
That is an expected negative-control failure, not a passing check or a fix.

To restore the experiment after approval, put the saved source at
`crates/bumbledb/src/exec/sink/tests/resident_budget.rs`, apply the binding
patch, then run under the measurement lock:

```sh
scripts/measure.sh cargo nextest run -p bumbledb --lib --no-fail-fast -E 'test(resident_budget::)'
```

The broad target is growing maps, group banks, and retained Pack storage.
Production scan gathering already uses windows of at most 256 rows, and
aggregate batch survivors are bounded by the configured batch size (128 by
default). Initial preparation allocations and LMDB's resident page cache are
separate audit boundaries; a working-byte ledger is not a process RSS limit.
Any future fix must preserve the existing 24 KiB same-ledger fallback test,
charge retained allocations until they are actually released, and admit group
storage before publishing an index into it. Do not increase test allowances
or refund live storage to conceal a refusal.
