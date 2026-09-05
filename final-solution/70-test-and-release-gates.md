# Strict passing criteria

This chapter is a conjunction with the permanent [behavioral obligations](../docs/reference/behavioral-obligations.md) and [inventory](../docs/reference/obligation-inventory.json), not a replacement for their coverage. D01–D29 refine, rather than duplicate, inherited behaviors; consolidate several schedules in one purposeful test where practical. All tests below are **to be authored now and executed only in the final post-retirement qualification phase**, not claims of passing behavior.

## No substitutions

ReadyForIntegration requests review of complete owned changes and authored discriminators, with any external consumer handoff explicitly named; it is not a completion claim. Integrated requires the replacement constructor, actual production callers, failure/cleanup paths and predecessor deletion, traced by the coordinator. Qualified requires discriminating executed evidence on the exact candidate and required backend/platform. Neither a worker's completion message nor compilation proves the invariant.

For each D case below, identify a plausible defective implementation that fails its assertion. During final qualification, establish sensitivity against the known defective behavior or a narrow isolated perturbation where practical. Do not retain a legacy production path just to get a red test. Independent small oracles are retained; mocks may test wire decoding but do not qualify the producer's authority or native lifecycle.

Each owner records the actual source/test symbol and result in the existing final evidence mechanism; no document per D ID. Multiple D cases may share one meaningful test/model program. No smoke-only test is acceptable.

## D01 — Charges cannot escape bytes (L01/L02/L03/L04/L05/L06/L12/L13)

Production paths: canonical row decode, image construction, sink/pool growth, queued native result conversion. Reserve zero or less than a requested growth; the operation refuses **before** an instrumented allocation. Move an owner through decode/cache/result/delivery, retaining payload while releasing previous scopes; the responsible ledger remains charged until actual release/transfer. Failed reservation/allocation leaves correct previous state.

Reject: an unused ChargedBuffer test; checking only reported logical length; dropping a reservation while bytes remain; the current “documented hazard” test asserting the bug. Exercise capacity after clear and after operation end, not only active length.

## D02 — Shared cache meanings survive trim (L03/L04/L05/L06/L07/L12/L13)

Prepare A and B on one store; both retain a text-bearing image. Trim A, ingest different texts, execute B on its pinned version, and compare exact values to an independent read. Exercise concurrent trim/build, old/new snapshots, numeric-only images exceeding cache allowance and text cardinality above resident capacity. Pins prevent reuse or enforce exact remapping; retained numeric/text memory stays within the declared envelope, and nonresident text continues.

Reject: checking generation counter increments; separate caches per prepare; retaining old generations forever; falling back into the same unbounded text interner.

## D03 — Scratch accounting is transactional (L03/L04/L01/L02/L05/L06/L10/L11)

Force MapFull after reservation but before transaction commit, then successful growth/retry. Final retained charge must match one committed entry and the stated conservative physical capacity, not two attempts. Repeated equal-size overwrite does not linearly consume budget; shrinking/reuse obeys stated live versus reserved-page accounting. Reservation failure restores prior ledger state. Constant fingerprint collision preserves exact wide keys.

Creation races/preexisting paths and setup failures cannot adopt or delete unowned scratch. Cancellation releases environment before cleanup. Reject: adding pre-put charge while retry mutations survive; a ScratchPolicy field nobody consults; a cap checked only after all writes.

## D04 — Compiled indexes earn locality (L01/L02/L05/L06/L07)

Generate tiny independent schemas and final deltas exercising scalar/pointwise keys, containment removal/addition, selected predicates, capacity floors/ceilings and reordered cross-relation fields. Compare admission, complete violated-statement set and canonical witnesses with an independent full streaming judge, under forced routing collisions.

Count actual candidate/group visits while unrelated groups scale; eligible local laws do not scan unrelated facts. Inspect persisted index roster/key bytes for shared projection identity and compact u64 keys. Reject: testing key-size arithmetic; physical uniqueness that drops a conflicting row; making the optimized judge spill merely because reference judgment does.

## D05 — Rejection evidence is portable (L01/L02/L08/L09/L14)

Build equal logical states in opposite insertion orders with more than the citation limit of offenders; remint IDs through export/import; run the same rejected command resident and forced-scratch. Canonical evidence bytes, cited facts, truncation flags and decision outcome agree. Replay the resulting committed decision on the independently reminted state.

Reject: sorting already-selected row-ID examples; ignoring witness comparison during replay; changing expected goldens to source order; losing violated statement directions.

## D06 — Staged means invisible until admitted (L07/L10/L11/L14)

Use a legal target schema whose empty state violates a law. Initialize, restore, hydrate, migrate and resume via incremental batches; only final judgment governs readiness. Inject failure between every ingestion/judgment/sync/install step, including after rename. Populate an unlawful stage and verify full judgment refuses despite its empty final delta; D26 isolates this admission bypass. Destination is absent or the complete identified store, never a partial ready Db. Two installers cannot overwrite each other; zero-row destination with host metadata is not fresh. Cleanup cannot delete the winning successor.

Reject: renamed ordinary Db constructor, public Copy freshness token, preflight exists check as no-clobber proof, or losing installed identity after sync failure. Exercise both core substrate and public log lifecycle.

## D07 — Bounded public work is transitive (L05/L06/L07/L12/L13/L15/L16)

Compile and run the downstream Rust and TS API specimens using normal public entry points. Tiny input/work/deadline/output limits stop actual canonical decode, keyed reads, query execution and delivery. Snapshot/session age does not donate its old deadline or unlimited work. Draft's cumulative input/work across chunks and finish is enforced; an abandoned/failed draft is terminal.

Source gate: no ordinary MAX/year fallback, unlimited twin, native bypass or internal raw operational reexport. Pure metadata remains synchronous. Explicit map ceiling never silently increases; pinned same-thread read plus MapFull produces bounded progress/refusal, not self-deadlock.

## D08 — Work without output still stops (L03/L04/L05/L06)

Use fewer than STEP_QUANTUM explored items with work cap below actual exploration: final partial poll must fail the query, not return success. Interrupt first COLT construction, a long rejecting scan, cover/probe suffix and image filter. Instrument work quanta and allocations; bounded check latency holds before full relation processing. Retained COLT capacity remains charged afterward.

Reject: polls only on emit, ignored bool/error return, post-growth accounting, or a test whose relation fits in the first trivial quantum. Performance measurement separately verifies no per-tuple allocation/atomic overhead regression.

## D09 — All derived consumers really accept scratch (L03/L04/L05/L06)

Compose aggregate/computed stage → positive join and bound negation, then restricted positive linear recursion with large seen/frontier and text. Force RAM below intermediate cardinality. Compare full results and mandated errors with independent staged evaluator and resident execution; instrument peak retained capacity and absence of whole-image resurrection. Tiny nonempty stages stay resident.

Reject: merely adding Scratch enum arm; force-spilling all small stages; clearing charge while keeping images; catching resource failure as empty output.

## D10 — Selective fallback and true stop (L01/L02/L05/L06)

Force nonresident evaluation of a key-bound query against many unrelated rows; assert exact result and bounded real source visits. An existence-only suffix stops after the first sufficient witness. Inject sink refusal and source error; no later scans/probes occur beyond the permitted bounded chunk. Column layout compilation/decode is per plan/row visit, not allocation per operand.

Reject: result-only comparisons with full scans; discarded sink stop; wrapper counters that do not cover actual storage iteration.

## D11 — Pack order is logical, not insertion-token order (L03/L04/L05/L06)

Force wide group spill over several flushes. Same group receives [10,20) then [0,15); expected maximal segment is [0,20). Interleave two groups across flushes, duplicate claims, separated and adjacent intervals, and float endpoint ordering. Narrow grouping has a first encoded word starting 0xFE. Force collisions in the wide group map. Compare exact canonical results with resident pack and independent sweep.

Peak group-head and claim retention must stay bounded beyond RAM; forbid finalize_spilled from gathering all_claims or all group headers in RAM. Stable group tokens reside in exact scratch-backed mapping. Reject: per-claim tokens, raw leading-byte mode detection, one-flush sorted input only, and any solution disabling legal wide spill.

## D12 — Delivery admits before advancing/copying (L05/L06/L12/L13/L15/L16)

Complete a result with an oversized text cell; call collection/page with insufficient bytes, then retry within policy. Predelivery refusal returns no data and does not advance the cursor. A terminal backing/transport error explicitly closes it. No successful page is lost or repeated; EOF/early take/downstream failure close once. Result backing and queued conversion overlap are correctly charged with JS wrappers retained.

Exercise actual addon from both RAM and scratch. Independent caps intersect: maxBytes cannot enlarge work.resultBytes. Fresh delivery deadline works after execution deadline expired. Reject: collect-all-then-check, pure mocked pages, charging only wire length, or new public raw cursor.

## D13 — Evidence survives every post-publication failure (L08/L09/L14/L17/L18)

Inject real driver faults: request published then response lost; Indeterminate admin attempt then next-loop deadline; local commit failure after known HEAD success; known rejected receipt then diagnostic-decode work/allocation failure; Effect interruption/finalizer failure after dispatch. Persist a stable recovery reference before dispatch.

NotStarted is possible only before actual dispatch or separately proved nonpublication under the selected outcome vocabulary. Known decided receipt stays terminal despite health/diagnostic failure. Unresolved publication remains unknown, resolvable with original identity. Compare public native/TS outcomes and persisted truth. Reject: setting phase before encoding, phase beside contradictory outcome, catch-all not-submitted, or a mock returning exactly the desired output.

## D14 — Writer-parent coherence (L08/L09/L10/L11/L14)

Pause retirement/catch-up after its outside-writer capture; another worker commits next decision or newer control; resume. Under the actual local writer, revalidate/rebase or refuse without regressing stamps, facts or receipts. Include two concurrent replay workers, control-only HEAD changes, quiescent same-tip receipt retirement and local-prune retries.

Independent history trace checks intermediate observable snapshots, not just eventual final digest. Reject: pre-lock comparison, comparing decision but not identity/control, pruning keys captured from an unrelated generation.

## D15 — Absence requires retained coverage (L08/L09/L14/L17/L18)

This command's CAS publishes and loses acknowledgment. Another actor rotates and retires its receipt before resolve. Changed HEAD plus absence must **not** produce proved loss/not-submitted. Return uncertainty/expired-unprovable with the original reference. Separately, demonstrate a real covered losing attempt can be proved lost, and a retained matching receipt resolves decided.

Reject: relying on latest token inequality, mixing two receipt/head snapshots, retrying under new ID, or treating NotRecordedAt(T) as permanent nonpublication.

## D16 — Every locator byte and boundary is checked (L08/L09/L10/L11)

Encode decision with absent/present parent at exact computed capacity and one byte below; assert full codec length (49 bytes per current ObjectRef) and canonical roundtrip against an independent expected frame. Truncation, wrong kind, wrong parent digest, root tip mismatch and missing interior locator refuse.

Recovery/GC/backup/witness traversal must stop at the authenticated base without older fetches; no historical epoch probing. A relocated backup uses its manifest and unchanged historical commitments. Reject: only unit-testing ObjectRef in isolation while decision cap still counts the option tag twice; keeping a “helpful” missing-link fallback.

## D17 — Lifecycle and receive are genuinely bounded (L03/L04/L07/L10/L11/L14)

A transport emits unknown/changing length or exceeds declared body size while receiving; cap/deadline interrupts before full buffering, verifies digest and preserves actual dispatch evidence. Slow HEAD receive is bounded too. Source/target migration and restore exceed RAM; MapSpill::finish must not reconstruct its entire scratch result as Rows/BTreeMap; instrument Rust **and native caller** peak live chunks, transform outputs and cleanup.

Process death/failure at lifecycle boundaries preserves old authority or a resumable matching new target; valid cold-open/resume works. Real S3/IAM tests cover conditional ambiguity, lost response, immutable conflicts, missing-vs-denied, pagination, redirects/retries and provider refresh. Reject: MemStore as backend proof, Vec<Vec<u8>> caller, post-read cap, blanket “streaming” name.

## D18 — Close owns the payload, queue and thread (L12/L13/L14/L15/L16/L17/L18)

Keep every JS capability wrapper strongly reachable and prohibit reliance on GC. Fill the normal queue, interrupt directory-acquire→Db-open and output-delivery gaps, open many idle snapshots/sessions and initiate close/eviction. Actual payloads/transactions/locks drain on fixed workers whose idle resource entries do not park their scheduler; close revokes new admission and reports honest remaining resources until joined. No heavy JS-thread destructor or per-session OS-thread growth.

Concurrent operations cannot use stale-generation handles; abandoned outputs have cleanup owners. Session close leaves parent usable; repeated close joins one transition. A small tenant progresses alongside a slow tenant under allowed fairness. Reject: natives==0 manufactured by counter change, queue-full teardown failure, a test that explicitly frees wrappers/forces GC before close.

## D19 — Shared typed scalar semantics (L01/L02/L05/L06/L10/L11/L14/L15/L16)

Static negatives reject invented field types, invalid query leaf scope, known I64/U64 mixing and incompatible known numeric operators without casts/any. Migration field names are symbolic: valid field arithmetic must construct, and missing/wrong-kind source fields must refuse in native schema-bound compilation before effects, even with zero input rows. Valid literals/operators/casts go through both native query and generated migration compilation/execution, including empty-input checking.

Assert canonical F64 bits for NaN/zero, exact sum/mean cancellation/ties/subnormals, overflow/cast refusal and stage-rounding boundaries against independent oracle. Check host floating-control save/set/restore on required architectures. Reject: only sharing NumericCast alias, tagged JSON test without execution, epsilon arithmetic comparisons or using the implementation to derive expected bits.

## D20 — Verify all schemas and mappings before side effects (L10/L11/L14/L17/L18)

Supply missing, foreign, edited or wrong-order snapshots; a well-hashed plan with absent source field, wrong target kind, invalid expression/cast; and an empty source database. Generate/verify refuses before writing a new authoritative manifest or freezing source. Every required intermediate source/target is bound and compiled. Valid prefix retry appends only the intended suffix.

Reject: optional snapshots, final-hash-only checking, compile only at execution, trusting an empty iterator to validate mapping, or handwritten migration callback escape.

## D21 — Generated history survives contention and crash (L17/L18)

Run two generators in the same process and two in different processes against one repository. Pause the first after old-manifest read; the second cannot enter the protected repository operation until ownership releases, or must refuse busy. Incompatible work cannot overwrite/delete winner artifacts. Kill after each durable file/sync/manifest step; retry finds either previous history or the complete committed new chain and repairs derivative files. Grow a file while bounded read is in progress; actual receiving/aggregate cap stops it.

Reject: PID-only temp naming, stat then whole read as bound, stale cleanup without ownership, swallowed durability failure, multiple competing authoritative manifests. Keep test inputs beside the test.

## D22 — Packed application, not source-only resemblance (L15/L16/L17/L18/L19/L20/L21)

Import schema/query/scalar authoring with native package deliberately unavailable and an addon-load detector that would fail if invoked. Separately install fresh built tarballs outside the workspace and run core and log consumer: generate history, initialize, mutate with sealed IDs/changes, witnessed correction, query/collect/pages, migrate, reopen, backup/restore, close. Use the native-ledger-shaped application and Notes/Next.js/Alchemy Node path; Rust core consumer exercises public API too.

No private imports, force casts, handwritten plan bytes, stub native module, stale dist, Promise wrapper or missing-peer duplicate Effect runtime. Publication check after actual package promotion is separately authorized; local packing alone is not registry publication proof.

## D23 — Evidence cannot manufacture green (L19/L20/L21/coordinator)

Pass a required qualification cell evidence [“garbage”], nonexistent report/artifact, wrong digest/source/spec/platform/backend, stale dist and duplicate/unknown cell IDs. All refuse. Modify intended added/deleted file, executable bit, symlink target or lock input: candidate identity changes/refuses appropriately. Deleted tracked files do not crash ordinary candidate enumeration; arbitrary caller path/digest overrides cannot omit production inputs.

Missing credentials/hardware/report remain NotRun/unqualified. The final checker recomputes input identity and validates every cell's evidence using the same substantive checks as audit/gate records. Reject: nonempty evidence array, unchecked platform strings, conditional skips counted as passes or source HEAD masquerading as dirty candidate identity.


## D24 — Session acquisition is a schedulable operation (L07/L12/L13/L14)

Actual addon/runtime with **one worker**: open Db/history, capture snapshot, prepare/read, close child execution session, read parent again, close parent/runtime. Each operation must complete without another user operation releasing its prerequisite. Repeat with worker initially asleep and an opening job routed to that same worker. Open more idle snapshots than worker count within the declared handle/memory limit; neighboring ordinary work still executes.

With multiple workers, fill normal queues and start close while snapshots stay reachable. Controls wake workers, current bounded jobs observe cancellation, actual resources drain. Use deterministic barriers, not an arbitrary timing microbenchmark. A deadline is a deadlock safety net, not the success assertion.

**Sensitivity:** current ready-after-reactor-exit and missing inbox wakeup fail. Merely moving ready.send earlier still fails the idle-snapshot/same-pool cases. A larger worker count or reserved hidden per-session thread is not a fix.

## D25 — Native batch cursor commits exactly once (L05/L13/L16)

Complete at least three variable-size rows. Choose pageBytes such that row1 and row2 each fit individually but together do not. Pull returns row1 as a successful nonempty page; next pull returns row2, then row3, each exactly once. Repeat with pending conversion expansion, RAM and scratch, and both TS Stream and direct private-addon cursor testing.

Inject predelivery resource refusal/cancellation after copying row1 but before the native output registration/commit. That failed invocation returns no data and retry begins at row1. Inject terminal scratch corruption there instead: cursor closes explicitly and no apparently complete page/EOF is returned. Retain queued output while draining the source; charge remains owned until transfer/cleanup.

**Sensitivity:** current inner core page advancement plus outer error propagation drops row1. Test must fail that composition, even if the single-row core cursor unit test passes. Registering an uncharged output or retaining all pages to make rollback easy also fails D01.

## D26 — Complete judgment cannot borrow a lawful-parent premise (L02/L07/L10/L14)

Through the actual internal staged population seam, insert two different canonical tuples with the same declared scalar key. Call terminal admit without further changes. It must reject, leave destination absent and release owned staging after abandonment. Repeat a violated containment and capacity floor/ceiling. No bypass via empty ChangeSet, unchecked Store/disarm accessor, metadata-only prepare or the new log install_judged_store wrapper.

Positive dual: a schema with a nonempty-required final law starts with invalid empty staging, receives valid rows across multiple batches, then admits and survives install/cold reopen/restore/migration. Intermediate invalidity is allowed only while unready.

**Sensitivity:** the present staging.admit → prepare(empty) → delta-local-skip chain incorrectly accepts the invalid target. A test merely asserting the AdmittedStore type exists does not cover readiness.

## D27 — Useful unresolved scalar authoring (L10/L14/L15/L17/L18)

Construct Scalar.add(Scalar.field("units"), Scalar.u64(1n)) without native loading. Generate/compile a migration from a verified source schema with u64 units and a matching target; execute a real row units=2 → 3. Include nested explicit cast to f64, a rename/backfill and zero input rows. Query-scoped equivalent uses its typed variable with the same operator/literal grammar.

Wrong source field, I64/U64 mismatch and incompatible target must refuse during native chain compilation before manifest write/freeze, including empty data. Known-invalid literal-only combinations refuse at their promised authoring boundary. No generic field<T> assertion, any/force cast or JS arithmetic evaluator.

**Sensitivity:** current field-node throw fails positive construction; an “accept all unknown” patch fails native negatives. A scalar-only constant backfill does not establish this gate. Count construction work on a nested expression to exclude whole-tree revalidation per constructor.

## D28 — Kernel lock and joined I/O, not stale-file guessing (L11/L14/L17)

Use the actual native lock seam with same-process and subprocess callers. Pause owner after opening/holding the lock but before any optional lock body write; a second generator cannot enter. Lock body may be empty or garbage and is irrelevant. Pause the owner arbitrarily long; it remains exclusive. Kill it; a new generator acquires the **same persistent inode** without deleting/replacing it. A stale token’s repeated release cannot unlock a successor.

Interrupt generation while an underlying filesystem promise is still in progress. The lock remains owned until that I/O is joined; no late write occurs after a successor begins. Kill after immutable artifact sync, manifest rename and directory sync; retry recovers one committed chain or the previous one and repairs derivative files.

Growing-file reads must stop at the receiving/aggregate bound on the same opened descriptor, reject invalid UTF-8 and close the descriptor. Assertions use actual recorded plans/snapshots, not file existence.

**Sensitivity:** current readLockPid(null)→rm steals a live empty lock. UUID temp naming alone fails. A generic “lock acquired” mock and stat→readFile check cannot qualify this gate.

## D29 — Resource ownership does not serialize or accumulate tenants (L03/L04/L12/L13/L20)

Two workers/owners: pause one inside an instrumented payload conversion/scratch read, then run another owner’s independent resource operation. It must not need the first owner’s registry mutex; only short shared routing/admission work is allowed. No payload closure, destructor, I/O or callback under that lock.

Fail retained-byte admission immediately before insertion: no payload, row or charge survives. Keep JS wrappers alive, close resources and repeat many bounded cycles: worker table/tombstone count and actual retained capacity return to the admitted baseline, independent of operation history. In-flight use drains before release; repeated/stale close never consumes a successor.

For cache, retain an image across eviction: its bytes stay charged until the final strong owner releases it; old text resolves exactly after new-generation admission. Include closed/numeric images and full cache allowance with legal old pins.

**Sensitivity:** current global with_payload lock, insert-before-admission, permanent revoked rows and cache-entry refund all fail distinct assertions. Zeroing statistics, forcing GC or using only independent runtimes is not a fix.

## Efficient suite, no ritual tests

Use a small everyday lane for exact semantics, budget counters and deterministic schedules; static lanes for compile/type/doc/ABI assertions; purpose-built subprocess/backend ownership lanes; explicit scale/performance lanes. Share expensive setup where isolation allows, avoid duplicated native builds, and run current-addon build before any native-dependent test. Serialize performance measurement per host; parallelize independent correctness cases.

Delete inert smoke checks, arithmetic-only storage tests, asserted bugs, no-op generic runtime wrappers and stale source-word/symbol counts. Preserve meaningful tests even if their filename says smoke until consolidated/renamed by responsibility. Remove obsolete compatibility cases only after affirmative unsupported-family refusal remains. Do not remove an independent oracle because it disagrees.

Lean qualification proves current premises and empirical correspondence, not text mentioning a theorem/file. Retain semantic/codegen/golden comparisons that detect wrong implementations; remove exact `dyn` counts, wording bans and deleted-path census. No test-count or line-count quotas.

## Final gate families

G00 traceability; G01 pinned builds/lints/types/docs; G02 codecs; G03 admission; G04 query meaning; G05 disk/large-state equivalence; G06 LMDB/durability; G07 independent authority model; G08 real backends; G09 commands/witnesses/retention; G10 lifecycle/GC/migrations; G11 native ownership; G12 bounded resources; G13 actual packages/consumers; G14 untrusted boundaries; G15 measured performance; G16 exact release evidence.

All required target/backend cells must pass for the advertised 1.0 envelope: Apple Silicon, real Graviton Linux ARM64 and x86 Node, selected local filesystem and real S3/IAM. Miri/unsafe lanes apply to their supported components; unsupported substrates are documented rather than pretending Miri executes S3/LMDB. Pre-promotion public-registry checks remain pending until separately authorized promotion; do not block ordinary local implementation by inventing permission to publish.

Approved narrowing only: no pre-1.0 importer (early untouched refusal instead), no mandatory AEGIS research lane, no pointless millions-of-cycles default. Within-family migrations, exact floating results, recovery, receipt safety and larger-than-memory behavior remain mandatory. See [90](90-evidence-and-retirement.md) for the final barrier.
