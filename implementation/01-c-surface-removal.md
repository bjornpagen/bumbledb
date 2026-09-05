# 01 — Retire the C product without retiring shared safety obligations

Date: 2026-09-04. Scope: the first bounded implementation packet for [chapter 32](../final-solution/32-ffi-and-release-packaging.md). This records removal and retained regression coverage; it is **not** a claim that the 1.0 Rust/Node owner, asynchronous API, or release qualification is implemented.

Subsequent contract refinement: [chapter 35](../final-solution/35-effect-typescript-contract.md) selects Effect-only work and Scope for both TypeScript packages. Pending targets below use that contract: ingestion acceptance is successful Effect execution, cancellation includes joined cleanup or retained Closing evidence, and result transfer is a consuming page Stream. No Promise/AsyncDisposable twin is to be implemented. Historical commands/results below still describe the old API/artifact actually tested. [The whole-tree checkpoint](02-checkpoint.md) records later integration and current failures.

## What changed

The public C product was removed from the implementation tree: its crate, public header, exported database API, generated-header configuration, C smoke program, C-specific tests and dedicated workflow. No compatibility crate/header/feature was retained. Rust remains a public core interface; Node's internal N-API linkage is not a public C database SDK.

Deleted tracked files:

```text
.github/workflows/c-abi.yml
crates/bumbledb-c/.gitignore
crates/bumbledb-c/Cargo.lock
crates/bumbledb-c/Cargo.toml
crates/bumbledb-c/cbindgen.toml
crates/bumbledb-c/include/bumbledb_c.h
crates/bumbledb-c/src/answers.rs
crates/bumbledb-c/src/db.rs
crates/bumbledb-c/src/error.rs
crates/bumbledb-c/src/lib.rs
crates/bumbledb-c/src/query.rs
crates/bumbledb-c/src/schema.rs
crates/bumbledb-c/src/tests.rs
crates/bumbledb-c/src/value.rs
crates/bumbledb-c/tests/c_smoke.c
```

The SDK packet also removed C support/version claims from the root README, C release clauses from `ts/PUBLISHING.md`, and the C lockstep claim in `ts/scripts/build.ts`. Its pre-existing 0.20.3 publishing paragraph was preserved. No version was bumped. The removed C manifest already contained a user-owned 0.20.1 → 0.20.3 edit; deletion of that manifest was explicitly included in the authorized packet.

The integration owner handles workspace/lockfile, version roster, general CI/battery/Miri gates and the stale C-specific engine error comment separately. A declaration of whole-product completion requires those references and release inputs to agree; deletion of the leaf crate alone is not sufficient. Audit/proposal history is intentionally retained and is not an active-product reference failure.

No Rust/TypeScript safety test was deleted. In particular, `ts/test/c-sdk-2-probe.test.ts` is **not a C test**: it exercises generic TypeScript `KeyFact`/`MemberRelation` point reads and absent-row behavior. Its campaign-derived filename and temporary-directory prefix do not ship a C SDK.

### Recoverability and local build output

Tracked source/header/workflow content remains recoverable from Git history. The prior ignored C `target/` directory became untracked after its local `.gitignore` was removed. To keep it out of future release inputs and avoid an accidental broad add, its verified 428 MB build cache was moved—not erased—to:

```text
/tmp/bumbledb-retired-c-cache.OuPyU0/target
```

This is temporary local recovery storage, not a retained supported build target or durable backup. The empty C source directories were then removed; `crates/bumbledb-c` was physically absent at verification. Historical published artifacts and audit evidence were not deleted.

## Shared regression transfer map

The former C tests contained two different kinds of obligations: C-specific raw-pointer mechanics, and cross-language engine/ownership behavior. Existing independent Rust/Node tests retain much of the latter. **Retained coverage is not evidence that unimplemented successor behavior works.** The pending rows below are required before closing their associated audit/gate obligations.

| Former C property/test family | Retained Rust/Node evidence | Successor limitation or pending work |
| --- | --- | --- |
| `create_refuses_existing_destination`, `create_open_close`, open failure, admitted-create result | Rust `tests/api.rs`: `create_refuses_a_foreign_lmdb_environment`, `a_second_handle_on_a_live_path_is_locked_out`; `api/db/open/tests.rs`: `empty_that_does_not_hold_is_violations_and_mints_no_lease`, `from_instance_refuses_an_occupied_path`. TS `db.test.ts` create/schema refusal/live-path locking; `ffi.test.ts` open outcomes. | Async failed/open-cancelled owner registration, native cleanup and same-path reopen still need the new owner harness. A raw C null-output argument has no public JS equivalent. |
| `schema_spec_crossing_admits_and_fingerprints` | Rust `tests/schema_spec.rs` schema-spec/macro fingerprint equivalence and roster pins; TS `ffi.test.ts` name-to-ID manifest and schema refusal checks. | New schema/value/format changes require regenerated, independently checked fixtures; C deletion does not authorize changing their meaning. |
| Committed reads, insert/delete/contains/get, exported scans, empty collections | TS `ffi.test.ts`, `db.test.ts`, `keyed-get.test.ts`, `c-sdk-2-probe.test.ts`, `marshal-bijection.test.ts`; Rust `tests/api.rs` and `tests/point_reads.rs`. | Existing tests use the old API/native artifact. Preserve set outcomes when migrating tests to async `ChangeSet`/snapshot operations. Do not carry the retired reserve API forward just because it appears in an old test. |
| Write abort, fresh/moved witness, stale witness/instance, nested write refusal | TS `db.test.ts` spent/stashed/leaked scopes and witnessed/abandoned writes; `bughunt.test.ts` thrown write/witness callbacks, async-callback refusal and spent/nested/native handle checks. Rust `tests/api.rs`: `aborted_writes_leave_prior_state_intact`, `nested_write_panics_instead_of_deadlocking`; `api/db/tests.rs` foreign witness tests. | New API has no sync or async application transaction callback. Transfer abort/owner/witness guarantees to immutable async apply and capability tests; do not preserve old callback syntax as compatibility. |
| `nested_reads_are_concurrent`, safe snapshot execution | Rust `tests/api.rs`: `concurrent_readers_while_writing`, `pinned_snapshot_reads_its_generation_across_later_commits`, `prepared_executions_observe_exactly_one_generation`; TS `read-scope-leak.test.ts` throwing read followed by successful reads and eight nested scopes. | These do not establish Node worker affinity, concurrent async close/cancel, or new managed-snapshot revocation. FFI-02/04/07 and RUN-04 remain required. |
| Scalar/set parameters, answer decoding, pointwise key violations | Rust `api/prepared/tests/params.rs`, `sets.rs`; `tests/dyn_surface.rs`: `a_rejection_renders_statement_spelling_kind_and_decoded_facts`. TS `ffi.test.ts` query/FD/containment/recursive/bind outcomes and `marshal-bijection.test.ts`. | New complete-result/cursor transfer, all-error output atomicity and async conversion require their own successor tests. |
| `foreign_prepared_is_refused_at_the_bridge`, `prepared_exclusive_execute_and_destroy` | Rust `tests/api.rs`: `a_prepared_query_refuses_a_foreign_snapshot`; `api/db/owned/tests.rs`: `foreign_prepared_query_is_rejected`. | Node parallel execute/close/cancel, same-version duplicate-addon ownership and slot reuse are **pending**, not covered by removed C exclusivity mechanics. |
| Collection shape refusal without a staged prefix; consumed builder | Rust `api/db/builder/tests.rs`: `dyn_parse_all_first_does_not_stage_a_prefix`, `poisoned_builder_admits_as_err`; `api/db/owned/tests.rs`: `rejected_builder_never_yields_an_instance`; `api/db/tests.rs`: `the_collection_builder_is_the_one_shape_judgment`, `a_fieldless_row_push_is_refused_typed`. TS `bughunt.test.ts` malformed values and `ffi.test.ts` hostile capacity before store touch. | New asynchronous ChangeSet ingestion must spend/drain on failed/cancelled/overlapping/reentrant calls and release native buffers before rejection. Rust consuming ownership already excludes double-admit; that does not prove a JS wrapper has the same behavior. |
| `marshal_refusals_are_typed_fact_shape`, malformed enum/bool/length/zero-width cases | Rust `tests/dyn_surface.rs`: `dyn_writes_refuse_malformed_input_typed_never_panicking`; TS `bughunt.test.ts` integer/byte/interval/Unicode/native input edges; `ffi.test.ts` raw-wire refusal checks. | Keep defensive Node input validation for untyped callers, including detached/resized buffers and raw shared-memory rejection. No public C alignment/null-pointer contract needs replacement. |
| `panic_maps_to_bdb_error_panic`, safe destruction during active callbacks | Existing Rust panic/drop tests and TS thrown-callback tests retain related cleanup behavior. | An actual native panic/worker failure contained at the new async Node boundary still needs a fault-injection harness. A JS exception is not evidence that a Rust unwind is safely contained. |
| `stashed_instance_ref_after_db_destroy_is_misuse` and stale read handles | TS stale-scope tests remain; Rust `api/db/tests.rs`: `dropping_the_handle_never_leaks_an_env_already_opened_window` retains the real drop-order/open race. | **SDK-007/013 native resource reclamation remains open.** The old C stale-pointer test verified misuse detection, not release of the engine, mappings, FDs or directory lock. See the explicit successor campaign below. |
| C version symbols, header compiles as C, manually overwritten/freed C errors, raw null outputs and callback tags | TS `native-loader.test.ts` and `ffi.test.ts` retain actual addon load/version evidence; typed errors remain independently tested. | C-only layout/header/symbol/error-carrier tests disappear with their product. PKG-06 must prove absence; remaining Node binary/bootstrap/platform qualification is still required. |

### Why SDK-013 is not a shared-lifecycle closure

The preserved [C audit](../audit/31-ffi-packaging.md) establishes the original leak: the C read capability retained an engine `Arc`; invalidation cleared its pointer/alive state but not the owner; retired callback boxes accumulated; destroy intentionally leaked them. Removing that C product removes this C path. It does **not** establish that the TypeScript replica/engine now closes correctly.

[SDK-007](../audit/30-sdk-hosting.md) describes a separate still-relevant ownership problem: retained replica/writer/Db wrappers retain the native database, while public disposal does not deterministically drop it. `read-scope-leak.test.ts` only proves subsequent reads work after an exception. `owned-read.test.ts` checks direct read shape and a JS heap-growth bound. Neither measures native engine owners, mapped files, open descriptors or physical disk reclamation.

The existing `ts/test/db.test.ts` test named **“no close verb exists anywhere — lifetimes are disposables (R12)”** passed in the baseline run below. It asserts the old contract, not the desired one. Replace it when implementing explicit async ownership; do not preserve it as a requirement or count it as native-close evidence.

## Concrete pending successor targets

These are planned test paths, **not files or passing tests claimed by this packet**. Integration may group them differently, but must retain the stated scenarios and audit IDs.

1. `ts/test/native-owner-lifecycle.test.ts` — SDK-007/013, FFI-01/02, RUN-03/04. Retain db/snapshot/session/result/writer wrappers with GC disabled; read, close, reopen the exact same path. Repeat bounded churn and prove native owner/FD/mapping/lock counts return to baseline and accounted memory/disk plateau. Queue/open/close/evict/cancel interleavings must not return a new live capability after shutdown or admit a successor while the old lock remains held. Preserve the Rust drop-order race above.
2. `ts/test/async-capabilities.test.ts` — FFI-02/04/07/08. Concurrent execute/apply/close/cancel; worker affinity; stale slot generation; foreign resource kinds; deliberately duplicated native runtime instances at the same package version; native panic/worker failure. Active operation leases protect data until actual drain; idle capabilities revoke safely.
3. `ts/test/change-set-ownership.test.ts` — SDK-003, API-01/02/10, FFI-05/07. Awaited insert/delete acceptance, mutation after fulfillment, hostile mutation/detachment during yielded copying, raw shared-memory refusal, one-shot iterators, getter failure mid-chunk, oversized cells/rows, cancellation during native finalization/hash, repeated finish, overlap/reentrancy and caught errors. Every failed draft becomes spent and releases native state before rejection without GC. The locally applied change and log command retain exactly the same native canonical value.
4. `ts/test/async-api.test.ts` — API-12/FFI-07. Typecheck/export inventory forbids sync data/close twins and sync resource disposal. Small schema/query/scalar/intent constructors and already-owned metadata remain synchronous. Large admitted ingestion, parameter sets, finalization, envelope hashes and row conversion meet an explicit event-loop-delay envelope; returning a promise around blocking work fails.
5. `ts/test/result-ownership.test.ts` — API-07/FFI-05/07. Every bind/query/overflow/corruption/cancel error exposes no current partial result. Completed results are independent of source snapshot close; asynchronous cursor transfer consumes once; spent result close cannot close the moved cursor; page interruption and cleanup preserve identity and release scratch.

No C ABI implementation or old C pointer promise is retained merely to satisfy these shared obligations. Do not add passing placeholders or skipped-green tests for missing successor APIs.

## Checks actually run

The removal packet ran the following against the current TypeScript **sources** and an **already installed native artifact**, without rebuilding or packing it:

```sh
node --conditions=bumbledb-src --test \
  test/ffi.test.ts test/db.test.ts test/bughunt.test.ts \
  test/read-scope-leak.test.ts test/owned-read.test.ts \
  test/keyed-get.test.ts test/marshal-bijection.test.ts \
  test/native-loader.test.ts test/c-sdk-2-probe.test.ts
pnpm exec tsc --noEmit
git diff --check
```

Observed: **79 tests passed, 0 failed, 0 skipped** across ten suites; TypeScript typecheck and whitespace/diff checks passed. The local runner reported Node **v26.4.0**. The loaded addon reported **`bumbledb-node 0.20.3 (bumbledb storage format v8)`**. An artifact digest was not captured at that test invocation; the version string is not proof of fresh source/binary correspondence.

These checks show that removal did not delete the retained TS tests or break their baseline source/type execution. They do not qualify new async APIs, new native owner code, Rust safety tooling, Linux/Node 24, AWS/Vercel deployments, packed artifacts, registry distribution or any new protocol/format. No C test/header generation was run after deleting its product. The larger integration gates remain separately required.

Read-only reference scans found no remaining C product in the SDK-owned README/build/publishing files after cleanup; root-owned manifest/roster/workflow/error-comment coordination was reported separately. The intentionally retained TS campaign test and a banned historical token are not supported C exports. A final release-tree/export scan must classify historical audit/proposal evidence separately from live product inputs.

The current build census uses `git ls-files`, which still lists unstaged deletions. Before a full build of this packet, the integration owner must stage the selected deletions so the index reflects the intended tree, or deliberately change the census semantics. Do not weaken version checks or silently recreate the deleted C manifest to make an unstaged build pass.

## Disposition

The public C implementation is removed. Shared ownership, canonical-input, async safety, output and package obligations remain in the integration ledger until their actual successor tests and implementation pass. A deleted interface, baseline green run, proposal edit or version string is not closure evidence for those obligations.
