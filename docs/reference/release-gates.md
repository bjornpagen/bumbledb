# Test and release gates (permanent reference)

This page is the authoritative release-gate contract. Executable
consumers use [`obligation-inventory.json`](obligation-inventory.json) and
[`release-results.schema.json`](release-results.schema.json). D01–D29
refine inherited behaviors in [behavioral-obligations.md](behavioral-obligations.md);
they are authored now and executed only against the **post-retirement**
candidate. They are not claims of passing behavior.

ReadyForIntegration requests review of complete owned changes and
authored discriminators. Qualified requires discriminating executed
evidence on the exact candidate and required backend/platform. Neither a
worker's completion message nor compilation proves the invariant.

## Test quality

No smoke-only tests. Every retained runtime assertion must reject a
plausible incorrect implementation. Tests whose names contain “smoke”
but compare full values/outcomes are kept until consolidated by
responsibility. Large tests that only assert the program ran are not
upgraded by size alone.

A function reference, `std::any::type_name`, or `std::mem::size_of`
is not a gate. Those claim a symbol exists; they do not fail the named
defective twin. Replace them with a compact counterexample through an
actual consumer (packed ManagedRuntime, Rust `collect`, Notes routes).

L21 packed-consumer covers D07 tiny-collect refusal and D12 same-cursor
retry. It does **not** close D08 or D09: `attemptsFor` is a keyed leaf
query, not a derived pipeline, and `used(WorkUnits) > 0` is not
retained COLT capacity. Request **L05-delivery** to replace these
`crates/bumbledb/src/api/prepared/tests/gates.rs` tests (do not treat
them as qualifying):

- `d08_successful_execute_retains_work_charges` — `WorkUnits > 0`
  after a tiny keyed execute is not retained-capacity coverage
- `d09_fallback_opens_nonresident_text_and_agrees` — tiny fallback
  query plus resolver flag
- `d09_spill_opens_via_exhausted` — intern spill plus `store.resolve`

Necessary expected bytes live beside consuming tests. No fixtures
hierarchy, implementation campaign tree, or generated evidence exhaust
in the repository. Preserve representative failures and independent
oracles. Never use test-count or line-count quotas. Do not remove an
independent oracle because it disagrees.

Lean qualification proves current premises and empirical correspondence
(`scripts/lean.sh` → kernel, conformance corpus, then
`scripts/spec-census.sh` constructor tokens). `lean.sh` is not a
cargo-test owner. Exact `dyn` counts, wording bans and deleted-path
census are deleted. Identity/surface goldens are
`python3 scripts/spec-gen.py --check`, not census.

## Execution lanes

| Lane | Content | Exclusions |
| --- | --- | --- |
| Everyday correctness | Small exact semantic/model cases, boundary goldens, native lifecycle tests | No full benchmark corpora in ordinary CI |
| Static correctness | Pinned builds, types, lint, doc compilation, absence checks | No no-op runtime wrappers around never-called generics |
| Fault and ownership | Deterministic barriers, subprocess death, corruption faults | Never equate mocked store with backend qualification |
| Scale and performance | Actual >RAM/>32 GiB data, tenant churn, controlled benchmarks | Serial per host; no wall-clock gates in unit tests |
| Target/backend/artifact | Apple Silicon, Graviton, x86 Node; real S3/IAM; packed tarballs | Missing access stays **NotRun**, not green |

## Runner order

`scripts/battery.sh` is the everyday/static/fault spine, not authority
to claim every environment passed. Final execution order:

1. Pinned Rust/TS formatting, lints, builds, type, declaration, doc and
   feature checks; compile workspace and excluded `ts/crate` against the
   same selected contract. Product absence gate.
2. Exactly one current-addon build per exact candidate/platform before
   any addon-dependent test. Reuse only a proven matching artifact:
   `BUMBLEDB_SKIP_NATIVE_BUILD=1` is refused unless
   `scripts/release-results.mjs --verify-native-provenance` binds the
   binary hash to the current candidate/spec. No second native rebuild
   from package `test` hooks.
3. Fast canonical/semantic/index/reference/collision/budget tests,
   D01–D29 composed discriminators, native ownership/Cause and
   independent protocol/failure/process tests.
4. Current Lean/refinement (`scripts/lean.sh`, which runs
   `scripts/spec-census.sh`) and supported unsafe/Miri lanes. Do not
   pretend unsupported substrate code ran under Miri. No dyn/wording
   census. `lean.sh` does not cargo-test L20
   `correspondence::OWNED_CASES` — those seven `C-*` tests live in
   `bumbledb-bench`. Identity/surface goldens:
   `python3 scripts/spec-gen.py --check`.
5. Fresh packed core/log installed outside the workspace;
   `ManagedRuntime.make(NativeRuntime.layer(...))` for specimens that no
   longer self-provide; D07 tiny collect must fail; D27 addon-unavailable
   pure import separately; actual core/log/native-ledger copies, Rust
   consumer, and Notes specimens/routes (missing migrations fail, never
   skip green).
6. Isolated authorized real filesystem/S3/IAM provider tests; streamed
   >RAM migration/recovery/backup/results; actual >32 GiB populated data
   and tenant churn.
7. Controlled Apple Silicon, real Graviton Linux ARM64 and Linux x86-64
   Node runtime/ABI/performance cells from
   `appperf::plan::script_steps` (meanings in
   [performance.md](performance.md)). No timing qualification while
   other workers load that host. **Do not start G15 until writers are
   frozen and the post-retirement candidate exists.** Missing
   `appperf::plan::hardware_prerequisites` stay **NotRun**.
8. Exact evidence completeness:
   `node scripts/release-results.mjs pre-promotion [manifest] [digest]`.

Battery exit alone does not certify cells it never ran.

## Content-addressed qualification

Identify the candidate with the recomputed `candidateSourceDigest` and
`specificationRevision`, not old HEAD or a guessed future SHA.

Cover intended paths, file kinds, executable modes, bytes or symlink
targets, additions/deletions, tests/build scripts/manifests/locks and
permanent specs. Validate membership rather than trust a caller-supplied
path list. Link identity hashes the link target, not arbitrary
dereferenced content. Deleted tracked inputs frame as `kind=deleted` and
must not crash enumeration.

Declare non-input evidence/output exclusions narrowly
(`docs/reference/release-results.json` and generated/ignored build
trees). Never exclude permanent specs or source to retain stale
evidence. Bind actual tarballs/addons/data/reports, machine/backend
facts, executed cases and skips to this identity.

The final staged/committed source must equal the qualified candidate
inputs. An amended source file invalidates dependent results.

Populate [`release-results.json`](release-results.json) only from real
evidence. Absent or placeholder manifests fail closed. Do not fabricate
that file during implementation.

## Evidence integrity (D23)

Authored symbols live in `scripts/release-results.test.mjs` and
`scripts/release-results.mjs`. The checker recomputes candidate
membership/path/kind/mode/content/link targets on every attempt.

The following must all **refuse**:

- required-cell evidence `["garbage"]` or any nonempty string array
- nonexistent report or artifact
- wrong digest, source, spec, platform or backend
- stale dist / hash mismatch
- duplicate or unknown cell IDs
- user-supplied `digestOverride`, `candidatePaths`, `pathOverride` or
  `specificationOverride`
- `NotApplicable` on a required 1.0 cell
- source HEAD masquerading as dirty candidate identity
- conditional skips counted as passes without `skipReasons`

Missing credentials/hardware/report remain **NotRun/unqualified**.
`PKG-07B` is the only pre-promotion excuse (separately authorized
publication). A caller-supplied digest confirmation must match
recomputed identity exactly.

## Parent gates (G00–G16)

| ID | Required evidence |
| --- | --- |
| `G00` | Complete specification/audit/prior-review/discriminator/test traceability |
| `G01` | Clean pinned builds, lint, types, docs, features |
| `G02` | Canonical scalar/row/schema/command codecs |
| `G03` | Admission, final-state constraints, diagnostics; L20 `OWNED_CASES` `C-G03-mutable-support` / `C-G03-add-wins` / `C-G03-raw-commute` vs `judge_final_state` — not retired braid theorems, not the production planner |
| `G04` | Query denotation, optimizer equivalence, arithmetic; `C-G04-*` / `C-D19-*` vs `staged.rs` and the rational/float oracles — not the production planner |
| `G05` | RAM/disk/scratch equivalence; larger-than-memory operation |
| `G06` | LMDB transaction, snapshot, resize, local durability |
| `G07` | Independent log history model (`crates/bumbledb-bench/src/closure/history_model.rs`) and concurrency schedules; Lean braid theorems cannot certify this machine |
| `G08` | Actual backend contract qualification |
| `G09` | Named commands, witnesses, application IDs, receipt retirement |
| `G10` | GC, checkpoint, backup/restore and migration drills |
| `G11` | Lifecycle and ownership; native safety |
| `G12` | Work, queue, scratch, memory and cancellation |
| `G13` | Fresh native packages and real consumers |
| `G14` | Untrusted input and authority boundaries |
| `G15` | Measured performance envelope from the L20 13-cell scorecard (`appperf::plan::render`); raw distributions; cold/warm separate; blocked until writers are frozen and the post-retirement candidate exists |
| `G16` | Complete exact-artifact release packet |

All required target/backend cells must pass for the advertised 1.0
envelope: Apple Silicon, real Graviton Linux ARM64 and x86 Node, selected
local filesystem and real S3/IAM. Miri/unsafe lanes apply to their
supported components.

G03/G04/G07 and D04/D05/D19/D26 correspondence case ids live in
`lean/correspondence.md` (`C-G03-*`, `C-G04-*`, `C-G07-authority`,
`C-D04-*`, `C-D05-*`, `C-D19-*`, `C-D26-*`). Premise map:
`lean/proof-bridge-ledger.md` and `lean/Bumbledb/Bridge.lean`.
Independent oracles are `judge_final_state`,
`crates/bumbledb-bench/src/naive/successor/staged.rs`, and
`crates/bumbledb-bench/src/closure/history_model.rs` — not the
production planner. L20 executable census of seven ids is
`bumbledb-bench` `correspondence::OWNED_CASES` (cargo tests, not
`lean.sh`): `C-D04-collision-bytes`, `C-D19-cancel`,
`C-D19-mean-once`, `C-D19-merge-not-idemp`,
`C-G03-mutable-support`, `C-G03-add-wins`, `C-G03-raw-commute`.
`python3 scripts/spec-gen.py --check` holds historical v3
identity/surface bytes; it is not log authority.

## Discriminators (D01–D29)

For each case, identify a plausible defective implementation that fails
its assertion. Independent small oracles are retained; mocks may test
wire decoding but do not qualify the producer's authority or native
lifecycle. Multiple D cases may share one meaningful test. No smoke-only
test is acceptable.

### D01 — Charges cannot escape bytes (L01/L02/L03/L04/L05/L06/L12/L13)

Production paths: canonical row decode, image construction, sink/pool
growth, queued native result conversion. Reserve zero or less than a
requested growth; the operation refuses **before** an instrumented
allocation. Move an owner through decode/cache/result/delivery, retaining
payload while releasing previous scopes; the responsible ledger remains
charged until actual release/transfer. Failed reservation/allocation
leaves correct previous state.

Reject: an unused ChargedBuffer test; checking only reported logical
length; dropping a reservation while bytes remain; asserting a documented
hazard as the passing behavior. Exercise capacity after clear and after
operation end, not only active length.

### D02 — Shared cache meanings survive trim (L03/L04/L05/L06/L07/L12/L13)

Prepare A and B on one store; both retain a text-bearing image. Trim A,
ingest different texts, execute B on its pinned version, and compare
exact values to an independent read. Exercise concurrent trim/build,
old/new snapshots, numeric-only images exceeding cache allowance and
text cardinality above resident capacity. Pins prevent reuse or enforce
exact remapping; retained numeric/text memory stays within the declared
envelope, and nonresident text continues.

Reject: checking generation counter increments; separate caches per
prepare; retaining old generations forever; falling back into the same
unbounded text interner.

### D03 — Scratch accounting is transactional (L03/L04/L01/L02/L05/L06/L10/L11)

Force MapFull after reservation but before transaction commit, then
successful growth/retry. Final retained charge must match one committed
entry and the stated conservative physical capacity, not two attempts.
Repeated equal-size overwrite does not linearly consume budget.
Reservation failure restores prior ledger state. Constant fingerprint
collision preserves exact wide keys. Creation races cannot adopt or
delete unowned scratch. Cancellation releases environment before
cleanup.

Reject: adding pre-put charge while retry mutations survive; a
ScratchPolicy field nobody consults; a cap checked only after all writes.

### D04 — Compiled indexes earn locality (L01/L02/L05/L06/L07)

Generate tiny independent schemas and final deltas exercising
scalar/pointwise keys, containment removal/addition, selected
predicates, capacity floors/ceilings and reordered cross-relation
fields. Compare admission, complete violated-statement set and canonical
witnesses with the independent streaming judge `judge_final_state`
(`C-D04-agree-three-judges`). Forced routing collisions keep exact
canonical-byte identity (`C-D04-collision-bytes`). Citations are
`fact_sort_key` top-k before truncation (`C-D04-citations-topk`). Count
actual candidate/group visits while unrelated groups scale. Inspect
persisted index roster/key bytes for shared projection identity and
compact u64 keys.

Independent oracle: `judge_final_state` — not the production planner.
`C-D04-collision-bytes` is L20 `correspondence::OWNED_CASES` (bench
cargo test, not `lean.sh`).

Reject: testing key-size arithmetic; physical uniqueness that drops a
conflicting row; making the optimized judge spill merely because
reference judgment does; sharing the planner as the oracle.

### D05 — Rejection evidence is portable (L01/L02/L08/L09/L14)

Build equal logical states in opposite insertion orders with more than
the citation limit of offenders; remint IDs through export/import; run
the same rejected command resident and forced-scratch
(`C-D05-remint-spill`). Canonical evidence bytes, cited facts,
truncation flags and decision outcome agree. Replay the resulting
committed decision on the independently reminted state.

Independent oracle: `judge_complete` / `encode_judged` against
`judge_final_state` — not insertion-order goldens.

Reject: sorting already-selected row-ID examples; ignoring witness
comparison during replay; changing expected goldens to source order;
losing violated statement directions.

### D06 — Staged means invisible until admitted (L07/L10/L11/L14)

Use a legal target schema whose empty state violates a law. Initialize,
restore, hydrate, migrate and resume via incremental batches; only final
judgment governs readiness. Inject failure between every
ingestion/judgment/sync/install step, including after rename. Populate
an unlawful stage and verify full judgment refuses despite its empty
final delta; D26 isolates this admission bypass. Destination is absent
or the complete identified store. Two installers cannot overwrite each
other; zero-row destination with host metadata is not fresh. Cleanup
cannot delete the winning successor.

Reject: renamed ordinary Db constructor, public Copy freshness token,
preflight exists check as no-clobber proof, or losing installed identity
after sync failure.

### D07 — Bounded public work is transitive (L05/L06/L07/L12/L13/L15/L16)

Compile and run the downstream Rust and TS API specimens using normal
public entry points. Tiny input/work/deadline/output limits stop actual
canonical decode, keyed reads, query execution and delivery.
Authored paths: `collectUnderTinyBudget` and
`collectPublishedUnderTinyBudget` (`resultBytes: 8n`) must fail;
Rust `oversized.collect(8, &tiny)` must be `Err`.
Snapshot/session age does not donate its old deadline or unlimited work.
Draft's cumulative input/work across chunks and finish is enforced; an
abandoned/failed draft is terminal.

Source gate: no ordinary MAX/year fallback, unlimited twin, native
bypass or internal raw operational reexport. Pure metadata remains
synchronous. Explicit map ceiling never silently increases; pinned
same-thread read plus MapFull produces bounded progress/refusal, not
self-deadlock.

Reject claim-only: mentioning `UNBOUNDED_POLICY`, `typeof execute`, or
`size_of` of a work struct. The defective twin is an unlimited collect
that returns a complete page under `resultBytes: 8`. After that
refusal, the same cursor must still deliver the rows (`scripts/packed-consumer.ts`).

### D08 — Work without output still stops (L03/L04/L05/L06)

Use fewer than STEP_QUANTUM explored items with work cap below actual
exploration: final partial poll must fail the query, not return success.
Interrupt first COLT construction, a long rejecting scan, cover/probe
suffix and image filter. Instrument work quanta and allocations;
bounded check latency holds before full relation processing. Retained
COLT capacity remains charged afterward.

Reject: polls only on emit, ignored bool/error return, post-growth
accounting, or a test whose relation fits in the first trivial quantum.
`work.used(Resource::WorkUnits) > 0` after a successful execute is
**not** retained-capacity coverage. L21 does not own a production-path
COLT-retention consumer; request L05-delivery replace
`d08_successful_execute_retains_work_charges`. The work-without-output
half remains: a cap below exploration must fail the query (L20
`work-without-output-stops`). Do not drop the D08 id.

### D09 — All derived consumers really accept scratch (L03/L04/L05/L06)

Compose aggregate/computed stage → positive join and bound negation,
then restricted positive linear recursion with large seen/frontier and
text. Force RAM below intermediate cardinality. Compare full results and
mandated errors with independent staged evaluator and resident
execution; instrument peak retained capacity and absence of whole-image
resurrection. Tiny nonempty stages stay resident.

Reject: merely adding Scratch enum arm; force-spilling all small stages;
clearing charge while keeping images; catching resource failure as empty
output; `size_of::<SealedStage>()` or naming a Scratch variant. D09 is
**derived-pipeline boundedness** (aggregate/computed → join → bound
negation → restricted recursion, RAM below intermediate cardinality).
A tiny fallback query plus a direct resolver test does not close D09.
L21 packed-consumer must not grow that mismatch. Request L05-delivery
replace `d09_fallback_opens_nonresident_text_and_agrees` and
`d09_spill_opens_via_exhausted`. Compare full derived results with
`staged.rs`. Do not drop the D09 id.

### D10 — Selective fallback and true stop (L01/L02/L05/L06)

Force nonresident evaluation of a key-bound query against many unrelated
rows; assert exact result and bounded real source visits. An
existence-only suffix stops after the first sufficient witness. Inject
sink refusal and source error; no later scans/probes occur beyond the
permitted bounded chunk.

Reject: result-only comparisons with full scans; discarded sink stop;
wrapper counters that do not cover actual storage iteration; constructing
`DistinctnessWitness` / `VisitControl` / `VisitOutcome` values without
walking storage. The defective twin is a full-scan key fallback that
still returns the exact row.

### D11 — Pack order is logical, not insertion-token order (L03/L04/L05/L06)

Force wide group spill over several flushes. Same group receives
`[10,20)` then `[0,15)`; expected maximal segment is `[0,20)`. Interleave
two groups across flushes, duplicate claims, separated and adjacent
intervals, and float endpoint ordering. Narrow grouping has a first
encoded word starting `0xFE`. Force collisions in the wide group map.
Compare exact canonical results with resident pack and independent sweep.

Peak group-head and claim retention must stay bounded beyond RAM; forbid
`finalize_spilled` from gathering `all_claims` or all group headers in
RAM. Reject: per-claim tokens, raw leading-byte mode detection,
one-flush sorted input only, and any solution disabling legal wide spill.

### D12 — Delivery admits before advancing/copying (L05/L06/L12/L13/L15/L16)

Complete a result with an oversized text cell; call collection/page with
insufficient bytes, then retry within policy. Predelivery refusal
returns no data and does not advance the cursor. A terminal
backing/transport error explicitly closes it. No successful page is lost
or repeated. Exercise actual addon from both RAM and scratch.
Independent caps intersect: `maxBytes` cannot enlarge `work.resultBytes`.
Fresh delivery deadline works after execution deadline expired.

Reject: collect-all-then-check, pure mocked pages, charging only wire
length, or new public raw cursor; `type_name::<DeliveryTicket>()` or
`type_name::<ResultCursor>()`. The defective twin advances on
predelivery refusal. Authored consumer: same-cursor tiny collect fails,
then `collect` under `work.resultBytes` still returns the rows
(`scripts/packed-consumer.ts`).

### D13 — Evidence survives every post-publication failure (L08/L09/L14/L17/L18)

Inject real driver faults: request published then response lost;
Indeterminate admin attempt then next-loop deadline; local commit
failure after known HEAD success; known rejected receipt then
diagnostic-decode work/allocation failure; Effect interruption/finalizer
failure after dispatch. Persist a stable recovery reference before
dispatch.

NotStarted is possible only before actual dispatch or separately proved
nonpublication. Known decided receipt stays terminal despite
health/diagnostic failure. Unresolved publication remains unknown,
resolvable with original identity.

Reject: setting phase before encoding, phase beside contradictory
outcome, catch-all not-submitted, or a mock returning exactly the
desired output.

### D14 — Writer-parent coherence (L08/L09/L10/L11/L14)

Pause retirement/catch-up after its outside-writer capture; another
worker commits next decision or newer control; resume. Under the actual
local writer, revalidate/rebase or refuse without regressing stamps,
facts or receipts. Include two concurrent replay workers, control-only
HEAD changes, quiescent same-tip receipt retirement and local-prune
retries.

Independent history trace checks intermediate observable snapshots, not
just eventual final digest. Reject: pre-lock comparison, comparing
decision but not identity/control, pruning keys captured from an
unrelated generation.

### D15 — Absence requires retained coverage (L08/L09/L14/L17/L18)

This command's CAS publishes and loses acknowledgment. Another actor
rotates and retires its receipt before resolve. Changed HEAD plus
absence must **not** produce proved loss/not-submitted. Return
uncertainty/expired-unprovable with the original reference. Separately,
demonstrate a real covered losing attempt can be proved lost, and a
retained matching receipt resolves decided.

Reject: relying on latest token inequality, mixing two receipt/head
snapshots, retrying under new ID, or treating `NotRecordedAt(T)` as
permanent nonpublication.

### D16 — Every locator byte and boundary is checked (L08/L09/L10/L11)

Encode decision with absent/present parent at exact computed capacity
and one byte below; assert full codec length (49 bytes per current
ObjectRef) and canonical roundtrip against an independent expected
frame. Truncation, wrong kind, wrong parent digest, root tip mismatch
and missing interior locator refuse. Recovery/GC/backup/witness
traversal must stop at the authenticated base without older fetches.

Reject: only unit-testing ObjectRef in isolation while decision cap
still counts the option tag twice; keeping a “helpful” missing-link
fallback.

### D17 — Lifecycle and receive are genuinely bounded (L03/L04/L07/L10/L11/L14)

A transport emits unknown/changing length or exceeds declared body size
while receiving; cap/deadline interrupts before full buffering, verifies
digest and preserves actual dispatch evidence. Slow HEAD receive is
bounded too. Source/target migration and restore exceed RAM;
`MapSpill::finish` must not reconstruct its entire scratch result as
Rows/BTreeMap. Process death/failure at lifecycle boundaries preserves
old authority or a resumable matching new target. Real S3/IAM tests
cover conditional ambiguity, lost response, immutable conflicts,
missing-vs-denied, pagination, redirects/retries and provider refresh.

Reject: MemStore as backend proof, `Vec<Vec<u8>>` caller, post-read cap,
blanket “streaming” name.

### D18 — Close owns the payload, queue and thread (L12/L13/L14/L15/L16/L17/L18)

Keep every JS capability wrapper strongly reachable and prohibit
reliance on GC. Fill the normal queue, interrupt directory-acquire→
Db-open and output-delivery gaps, open many idle snapshots/sessions and
initiate close/eviction. Actual payloads/transactions/locks drain on
fixed workers whose idle resource entries do not park their scheduler.
No heavy JS-thread destructor or per-session OS-thread growth.
Concurrent operations cannot use stale-generation handles. Session close
leaves parent usable; repeated close joins one transition.

Reject: `natives==0` manufactured by counter change, queue-full teardown
failure, a test that explicitly frees wrappers/forces GC before close.

### D19 — Shared typed scalar semantics (L01/L02/L05/L06/L10/L11/L14/L15/L16)

Static negatives reject invented field types, invalid query leaf scope,
known I64/U64 mixing and incompatible known numeric operators without
casts/any. Migration field names are symbolic: valid field arithmetic
must construct, and missing/wrong-kind source fields must refuse in
native schema-bound compilation before effects, even with zero input
rows. Assert canonical F64 bits for NaN/zero, exact sum/mean
cancellation/ties/subnormals, overflow/cast refusal and stage-rounding
boundaries against an independent oracle (`C-D19-cancel`,
`C-D19-mean-once`, `C-D19-merge-not-idemp`). Query/error surfaces
(`C-G04-error-surfaces`, `C-G04-frozen-domain`) use
`crates/bumbledb-bench/src/naive/successor/staged.rs`. Check host
floating-control save/set/restore on required architectures.

Independent oracles: rational/float goldens and `staged.rs` — not the
production planner. The three `C-D19-*` ids above are L20
`correspondence::OWNED_CASES` (bench cargo tests, not `lean.sh`).

Reject: only sharing NumericCast alias, tagged JSON test without
execution, epsilon arithmetic comparisons or using the implementation to
derive expected bits.

### D20 — Verify all schemas and mappings before side effects (L10/L11/L14/L17/L18)

Supply missing, foreign, edited or wrong-order snapshots; a well-hashed
plan with absent source field, wrong target kind, invalid
expression/cast; and an empty source database. Generate/verify refuses
before writing a new authoritative manifest or freezing source. Every
required intermediate source/target is bound and compiled. Valid prefix
retry appends only the intended suffix.

Reject: optional snapshots, final-hash-only checking, compile only at
execution, trusting an empty iterator to validate mapping, or
handwritten migration callback escape.

### D21 — Generated history survives contention and crash (L17/L18)

Run two generators in the same process and two in different processes
against one repository. Pause the first after old-manifest read; the
second cannot enter the protected repository operation until ownership
releases, or must refuse busy. Incompatible work cannot overwrite/delete
winner artifacts. Kill after each durable file/sync/manifest step; retry
finds either previous history or the complete committed new chain and
repairs derivative files. Grow a file while bounded read is in progress;
actual receiving/aggregate cap stops it.

Reject: PID-only temp naming, stat then whole read as bound, stale
cleanup without ownership, swallowed durability failure, multiple
competing authoritative manifests. Keep test inputs beside the test.

### D22 — Packed application, not source-only resemblance (L15/L16/L17/L18/L19/L20/L21)

Import schema/query/scalar authoring with native package deliberately
unavailable (`scripts/packed-pure-authoring.ts`); resolving a platform
package fails the cell. That file must not import or invoke
`NativeRuntime.layer`. Separately install fresh built tarballs outside
the workspace and run copied core/log/native-ledger consumers under
`ManagedRuntime.make(NativeRuntime.layer(...))` (`makeConsumerRuntime`):
mutate with sealed IDs/changes, query/collect, D07 tiny collect refusal,
mint intent/command. Notes `specimens.test` + `routes.test` fail if
generated migrations are missing — never skip green. Rust core consumer
runs in the same packed-import path.

No private imports, force casts, handwritten plan bytes, stub native
module, stale dist, Promise wrapper or missing-peer duplicate Effect
runtime. Publication check after actual package promotion is separately
authorized; local packing alone is not registry publication proof.

Authored consumer paths: `scripts/packed-import.sh`,
`scripts/packed-consumer.ts`, `scripts/packed-pure-authoring.ts`,
`examples/consumers/{core-ts,log-ts,native-ledger,rust}`,
`examples/notes/test/{specimens,routes}.test.ts`.

### D23 — Evidence cannot manufacture green (L19/L20/L21/coordinator)

Pass a required qualification cell evidence `["garbage"]`, nonexistent
report/artifact, wrong digest/source/spec/platform/backend, stale dist
and duplicate/unknown cell IDs. All refuse. Modify intended
added/deleted file, executable bit, symlink target or lock input:
candidate identity changes/refuses appropriately. Deleted tracked files
do not crash ordinary candidate enumeration; arbitrary caller
path/digest overrides cannot omit production inputs.

Missing credentials/hardware/report remain NotRun/unqualified. The final
checker recomputes input identity and validates every cell's evidence
using the same substantive checks as audit/gate records.

Reject: nonempty evidence array, unchecked platform strings, conditional
skips counted as passes or source HEAD masquerading as dirty candidate
identity.

Authored symbols: `scripts/release-results.mjs` (`validateResults`,
`describeCandidateEntry`, `frameCandidateEntry`,
`computeCandidateSourceDigest`, `verifyNativeProvenance`) and
`scripts/release-results.test.mjs` (D23 cases below).

### D24 — Session acquisition is a schedulable operation (L07/L12/L13/L14)

Actual addon/runtime with **one worker**: open Db/history, capture
snapshot, prepare/read, close child execution session, read parent
again, close parent/runtime. Each operation must complete without
another user operation releasing its prerequisite. Repeat with worker
initially asleep and an opening job routed to that same worker. Open
more idle snapshots than worker count within the declared handle/memory
limit; neighboring ordinary work still executes.

With multiple workers, fill normal queues and start close while
snapshots stay reachable. Controls wake workers, current bounded jobs
observe cancellation, actual resources drain. Use deterministic
barriers, not an arbitrary timing microbenchmark.

Sensitivity: ready-after-reactor-exit and missing inbox wakeup fail.
Merely moving `ready.send` earlier still fails the idle-snapshot/same-pool
cases. A larger worker count or reserved hidden per-session thread is
not a fix.

### D25 — Native batch cursor commits exactly once (L05/L13/L16)

Complete at least three variable-size rows. Choose `pageBytes` such that
row1 and row2 each fit individually but together do not. Pull returns
row1 as a successful nonempty page; next pull returns row2, then row3,
each exactly once. Repeat with pending conversion expansion, RAM and
scratch, and both TS Stream and direct private-addon cursor testing.

Inject predelivery resource refusal/cancellation after copying row1 but
before the native output registration/commit. That failed invocation
returns no data and retry begins at row1. Inject terminal scratch
corruption there instead: cursor closes explicitly and no apparently
complete page/EOF is returned.

Sensitivity: inner core page advancement plus outer error propagation
drops row1. Registering an uncharged output or retaining all pages to
make rollback easy also fails D01.

### D26 — Complete judgment cannot borrow a lawful-parent premise (L02/L07/L10/L14)

Through the actual internal staged population seam, insert two different
canonical tuples with the same declared scalar key
(`C-D26-collision-empty-delta`). Call terminal admit without further
changes. It must reject, leave destination absent and release owned
staging after abandonment. Repeat a violated containment and capacity
floor/ceiling (`C-D26-containment-capacity`). No bypass via empty
ChangeSet, unchecked Store/disarm accessor, metadata-only prepare or
the new log `install_judged_store` wrapper. `UnreadyStore` cannot mint
`LawfulParent` (`C-D26-unready-cannot-mint`).

Positive dual: a schema with a nonempty-required final law starts with
invalid empty staging, receives valid rows across multiple batches, then
admits and survives install/cold reopen/restore/migration
(`C-D26-nonempty-required`). Intermediate invalidity is allowed only
while unready.

Independent oracle: `judge_complete` / `judge_final_state` on the
populated unready state — not `judge_incremental(LawfulParent, empty)`
and not the production planner.

Sensitivity: `staging.admit` → `prepare(empty)` → delta-local-skip
accepting the invalid target. A test merely asserting the AdmittedStore
type exists does not cover readiness.

### D27 — Useful unresolved scalar authoring (L10/L14/L15/L17/L18)

Construct `Scalar.add(Scalar.field("units"), Scalar.u64(1n))` without
native loading (`scripts/packed-pure-authoring.ts` and core-ts
`incrementUnits`; `result === "unresolved"`). Generate/compile a
migration from a verified source schema with u64 units and a matching
target; execute a real row `units=2 → 3`. Include nested explicit cast
to f64, a rename/backfill and zero input rows. Query-scoped equivalent
uses its typed variable with the same operator/literal grammar. The
packed-import D27 cell is the addon-unavailable authoring half; full
chain execute remains a production discriminator.

Wrong source field, I64/U64 mismatch and incompatible target must refuse
during native chain compilation before manifest write/freeze, including
empty data. No generic `field<T>` assertion, any/force cast or JS
arithmetic evaluator.

Sensitivity: field-node throw fails positive construction; an “accept
all unknown” patch fails native negatives. A scalar-only constant
backfill does not establish this gate.

### D28 — Kernel lock and joined I/O, not stale-file guessing (L11/L14/L17)

Use the actual native lock seam with same-process and subprocess
callers. Pause owner after opening/holding the lock but before any
optional lock body write; a second generator cannot enter. Lock body may
be empty or garbage and is irrelevant. Pause the owner arbitrarily long;
it remains exclusive. Kill it; a new generator acquires the **same
persistent inode** without deleting/replacing it. A stale token’s
repeated release cannot unlock a successor.

Interrupt generation while an underlying filesystem promise is still in
progress. The lock remains owned until that I/O is joined; no late write
occurs after a successor begins.

Sensitivity: `readLockPid(null)`→`rm` steals a live empty lock. UUID
temp naming alone fails. A generic “lock acquired” mock and
stat→readFile check cannot qualify this gate.

### D29 — Resource ownership does not serialize or accumulate tenants (L03/L04/L12/L13/L20)

Two workers/owners: pause one inside an instrumented payload
conversion/scratch read, then run another owner’s independent resource
operation. It must not need the first owner’s registry mutex; only short
shared routing/admission work is allowed. No payload closure,
destructor, I/O or callback under that lock.

Fail retained-byte admission immediately before insertion: no payload,
row or charge survives. Keep JS wrappers alive, close resources and
repeat many bounded cycles: worker table/tombstone count and actual
retained capacity return to the admitted baseline. For cache, retain an
image across eviction: its bytes stay charged until the final strong
owner releases it; old text resolves exactly after new-generation
admission.

Sensitivity: global `with_payload` lock, insert-before-admission,
permanent revoked rows and cache-entry refund all fail distinct
assertions. Zeroing statistics, forcing GC or using only independent
runtimes is not a fix.

## Final-only execution

During implementation, tests are authored but not executed until the
coordinator barrier and the post-retirement candidate. One integrated
qualification phase may contain several runs; it is not permission to
accept a failing final attempt. See
[qualification-checklist.md](qualification-checklist.md).
