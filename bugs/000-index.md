# Bug audit index

- **Audit date:** 2026-08-12
- **Scope:** entire codebase — FFI / lifetimes / unsafe; three-way Lean spec vs Rust vs normative docs; general correctness (engine, macros, persistence, C++ dialect)
- **Status:** **open.** Nothing was fixed. This directory is a read-only finding dump. Coordinators and later work must treat every file as still live unless a later change explicitly closes it.
- **Layout:** flat directory only. No nested folders.

## How files are named

| Range | Area | Manifest |
|---|---|---|
| `1NN` (`100`–`199`) | FFI / unsafe / allocation / lifetimes (C ABI, C++ dialect, NAPI) | [`_manifest-ffi.md`](_manifest-ffi.md) |
| `2NN` (`200`–`299`) | Spec / docs / Rust three-way (Lean vs engine vs architecture) | [`_manifest-spec.md`](_manifest-spec.md) |
| `3NN` (`300`–`399`) | General correctness (persistence, commit, macros, C++ wrappers) | [`_manifest-general.md`](_manifest-general.md) |

One finding per file. Combined machine list: [`_manifest-all.md`](_manifest-all.md).

## Disk vs manifests

All three auditor manifests match files on disk. No finding file is missing, empty, or malformed.

| Auditor | Manifest ids | Files on disk | Match |
|---|---|---|---|
| FFI | 100–118 (19) | 19 | yes |
| Spec/docs/rust | 200–225 (26) | 26 | yes |
| General | 301–307 (7; no 300) | 7 | yes |
| **Total findings** | **52** | **52** | **yes** |

Extra files in this directory (not findings): the three per-auditor manifests, this index, and `_manifest-all.md`.

## Totals by severity

| Severity | FFI | Spec | General | Total |
|---|---:|---:|---:|---:|
| critical | 1 | 0 | 0 | **1** |
| high | 6 | 3 | 1 | **10** |
| medium | 8 | 16 | 3 | **27** |
| low | 3 | 7 | 2 | **12** |
| info | 1 | 0 | 1 | **2** |
| **Total** | **19** | **26** | **7** | **52** |

## Totals by area

Area is the `area:` field in each finding file (not the id range).

| Area | Count | Ids |
|---|---:|---|
| `ffi` | 19 | 100–118 |
| `spec-docs-rust` | 26 | 200–225 |
| `correctness` | 5 | 302–306 |
| `persistence` | 1 | 301 |
| `other` | 1 | 307 |

Spec findings also record **wrong-side** (which artifact is wrong, or a split):

| Wrong-side | Count | Ids |
|---|---:|---|
| spec (Lean) | 8 | 201, 203, 207, 208, 213, 215, 220, 224 |
| docs | 7 | 202, 204, 205, 210, 216, 218, 221 |
| split | 4 | 200, 209, 211, 219 |
| unspecified | 5 | 214, 217, 222, 223, 225 |
| rust | 2 | 206, 212 |

## Totals by confidence

| Confidence | FFI | Spec | General | Total |
|---|---:|---:|---:|---:|
| confirmed | 14 | 25 | 6 | **45** |
| likely | 3 | 1 | 1 | **5** |
| possible | 2 | 0 | 0 | **2** |
| **Total** | **19** | **26** | **7** | **52** |

Likely: [108](108-slice-in-count-overflow.md), [109](109-napi-take-handle-refcell-panic.md), [116](116-tx-ref-mut-from-ref-aliasing.md), [225](225-origin-and-result-bytes-overflow.md), [306](306-cpp-violations-silent-truncate.md).

Possible: [113](113-cpp-exception-through-rust-callback.md), [118](118-inbound-view-unbounded-lifetime.md).

---

## Read this first

Critical, then every high finding. One line each. **Status = open.**

### Critical

- [100](100-napi-prepared-cross-thread-aliasing.md) — NAPI ships a `PreparedQuery` address to a worker while JS still holds `RefMut`; aliasing UB on every execute.

### High

- [101](101-stale-snapshot-ref-is-stack-uaf.md) — Snapshot/tx `alive` flag lives in a stack object; post-callback use is UAF, not MISUSE.
- [102](102-destroy-db-during-callback-uaf.md) — `bdb_db_destroy` / C++ `Db` move during read/write frees the engine under live snapshot/tx/manifest refs.
- [103](103-repr-c-enum-and-bool-unchecked.md) — Inbound `#[repr(C)]` enums and `bool`s are matched without discriminant checks; docs promised MISUSE.
- [104](104-box-out-null-outparam-leak.md) — `box_out` + null `out()` leaks `Box` (LMDB env, prepared, row sets).
- [105](105-bulk-load-null-out-committed.md) — Bulk load can commit (or partially commit) then return MISUSE if `out_committed` is null.
- [112](112-c-abi-prepared-no-exclusive-lock.md) — C `bdb_prepared*` has no exclusive lock; concurrent execute/destroy races `!Sync` scratch.
- [200](200-c20-ray-weight-absent-parent.md) — C20 refuses a ray Duration child under an absent parent; Lean `capacity_of_empty_parent` and architecture docs treat that insert as a no-op.
- [201](201-argkey-measure-missing-from-lean.md) — `ArgKey::Measure` is in Rust/docs/R5; Lean `AggOp.argMax` is VarId-only; conformance fences the shape.
- [202](202-cookbook-claims-disjoint-dedup-elision.md) — Cookbook recipe 22 (and TS twin) claims executor elides cross-rule dedup; Lean/40-execution/Rust keep a spanning seen-set.
- [301](301-escaped-fresh-id-flush-swallowed.md) — Abort-path `flush_escaped_fresh_ids` errors are discarded, so `alloc()` ids can be reissued after a failed Q burn.

---

## Full catalog

Every finding. Status is **open** for all. Spec `wrong-side` is in parentheses under Area when present.

| Id | Sev | Conf | Area | Title | File |
|---|---|---|---|---|---|
| 100 | critical | confirmed | ffi | NAPI prepared-query pointer is used as `&mut` on a worker while JS still holds `RefMut` | [100-napi-prepared-cross-thread-aliasing.md](100-napi-prepared-cross-thread-aliasing.md) |
| 101 | high | confirmed | ffi | Stale snapshot/tx `alive` flag cannot implement documented MISUSE; post-callback use is stack UAF | [101-stale-snapshot-ref-is-stack-uaf.md](101-stale-snapshot-ref-is-stack-uaf.md) |
| 102 | high | confirmed | ffi | Destroying or moving the db handle during its own read/write callback frees the engine under live refs | [102-destroy-db-during-callback-uaf.md](102-destroy-db-during-callback-uaf.md) |
| 103 | high | confirmed | ffi | Inbound `repr(C)` enums and bools are matched without validating discriminants (UB, not MISUSE) | [103-repr-c-enum-and-bool-unchecked.md](103-repr-c-enum-and-bool-unchecked.md) |
| 104 | high | confirmed | ffi | `box_out` then failed `out()` leaks the `Box` (engine, row sets, prepared queries) | [104-box-out-null-outparam-leak.md](104-box-out-null-outparam-leak.md) |
| 105 | high | confirmed | ffi | `bdb_db_bulk_load` can commit facts then return MISUSE if `out_committed` is null | [105-bulk-load-null-out-committed.md](105-bulk-load-null-out-committed.md) |
| 106 | medium | confirmed | ffi | Node write-begin can permanently stick `tx_open` if `thread::spawn` panics | [106-napi-tx-open-stuck-on-spawn-panic.md](106-napi-tx-open-stuck-on-spawn-panic.md) |
| 107 | medium | confirmed | ffi | Several `extern "C"` entry points skip `catch_unwind` despite the panic-into-C++ policy | [107-unguarded-extern-panic-wall.md](107-unguarded-extern-panic-wall.md) |
| 108 | medium | likely | ffi | `slice_in` builds slices without rejecting `count*size` overflow or unaligned pointers | [108-slice-in-count-overflow.md](108-slice-in-count-overflow.md) |
| 109 | medium | likely | ffi | NAPI `take_handle` uses `RefCell::borrow_mut` and panics into Node on re-entrant close | [109-napi-take-handle-refcell-panic.md](109-napi-take-handle-refcell-panic.md) |
| 110 | medium | confirmed | ffi | Execute clears (and may partially refill) the answers carrier before the call can fail | [110-execute-clears-answers-on-error.md](110-execute-clears-answers-on-error.md) |
| 111 | medium | confirmed | ffi | C++ `cell()` returns a copyable `Value`/`bdb_value` that borrows the carrier with no lifetime | [111-cpp-answer-value-borrow-escape.md](111-cpp-answer-value-borrow-escape.md) |
| 112 | high | confirmed | ffi | C ABI does not serialize `PreparedQuery`; concurrent execute is a data race on `!Sync` scratch | [112-c-abi-prepared-no-exclusive-lock.md](112-c-abi-prepared-no-exclusive-lock.md) |
| 113 | medium | possible | ffi | A C++ exception escaping a read/write callback unwinds through Rust (UB) | [113-cpp-exception-through-rust-callback.md](113-cpp-exception-through-rust-callback.md) |
| 114 | low | confirmed | ffi | `store_error` overwrites a live `bdb_error*` without freeing it | [114-store-error-overwrites-without-free.md](114-store-error-overwrites-without-free.md) |
| 115 | info | confirmed | ffi | Unit test `stale_snapshot_ref_is_misuse` is itself use-after-free | [115-stale-ref-test-is-uaf.md](115-stale-ref-test-is-uaf.md) |
| 116 | medium | likely | ffi | `bdb_tx_ref::transaction` yields `&mut WriteTx` from `&self`; nested FFI entries alias it | [116-tx-ref-mut-from-ref-aliasing.md](116-tx-ref-mut-from-ref-aliasing.md) |
| 117 | low | confirmed | ffi | Moved-from C++ `error_handle::kind()` returns Panic instead of aborting | [117-moved-from-error-kind-is-panic.md](117-moved-from-error-kind-is-panic.md) |
| 118 | low | possible | ffi | Inbound `bdb_string_view::as_str` fabricates an unbounded lifetime from a raw pointer | [118-inbound-view-unbounded-lifetime.md](118-inbound-view-unbounded-lifetime.md) |
| 200 | high | confirmed | spec-docs-rust (split) | C20 write-time ray refusal vs Lean empty-parent vacuity | [200-c20-ray-weight-absent-parent.md](200-c20-ray-weight-absent-parent.md) |
| 201 | high | confirmed | spec-docs-rust (spec) | `ArgKey::Measure` exists in Rust and docs, not in Lean `AggOp` | [201-argkey-measure-missing-from-lean.md](201-argkey-measure-missing-from-lean.md) |
| 202 | high | confirmed | spec-docs-rust (docs) | Cookbook claims executor elides cross-rule dedup; execution never does | [202-cookbook-claims-disjoint-dedup-elision.md](202-cookbook-claims-disjoint-dedup-elision.md) |
| 203 | medium | confirmed | spec-docs-rust (spec) | Bridge ledger says aborted mint runs are discarded; `Fresh.lean` persists them | [203-bridge-abort-fresh-discarded.md](203-bridge-abort-fresh-discarded.md) |
| 204 | medium | confirmed | spec-docs-rust (docs) | Docs claim abort never touched disk; abort burn writes `Q` marks | [204-abort-never-touched-disk.md](204-abort-never-touched-disk.md) |
| 205 | medium | confirmed | spec-docs-rust (docs) | DNF fold-preservation cites the projection theorem, not the fold theorem | [205-dnf-fold-cites-projection-theorem.md](205-dnf-fold-cites-projection-theorem.md) |
| 206 | medium | confirmed | spec-docs-rust (rust) | Fixpoint budget makes the engine incomplete versus Lean `evalProgram` | [206-fixpoint-budget-incompleteness.md](206-fixpoint-budget-incompleteness.md) |
| 207 | medium | confirmed | spec-docs-rust (spec) | `TargetKeyAccepted` accepts any declared FD; Rust closed targets require `FieldId(0)` | [207-closed-target-key-broader-in-lean.md](207-closed-target-key-broader-in-lean.md) |
| 208 | medium | confirmed | spec-docs-rust (spec) | Closed+interval containment is a Lean judgment; Rust refuses it v0 | [208-closed-containment-interval-unmodeled.md](208-closed-containment-interval-unmodeled.md) |
| 209 | medium | confirmed | spec-docs-rust (split) | Lean `FixedBytes` is N words; Rust/docs encode N bytes padded to ⌈N/8⌉ words | [209-fixedbytes-word-vs-byte-encoding.md](209-fixedbytes-word-vs-byte-encoding.md) |
| 210 | medium | confirmed | spec-docs-rust (docs) | Docs call `MeasureOfRay` the one runtime type error; 70-api omits it | [210-measure-of-ray-not-the-only-runtime-error.md](210-measure-of-ray-not-the-only-runtime-error.md) |
| 211 | medium | confirmed | spec-docs-rust (split) | TypeScript ArgMax keys are variables only; Rust/docs/C++ admit Duration keys | [211-ts-argkey-measure-missing.md](211-ts-argkey-measure-missing.md) |
| 212 | low | confirmed | spec-docs-rust (rust) | `CommitRejected` comment says all-containment; statement phase mixes capacity | [212-commitrejected-all-containment-comment.md](212-commitrejected-all-containment-comment.md) |
| 213 | medium | confirmed | spec-docs-rust (spec) | Multi-interval FD is scalar `Functionality` in Lean, a validate error in Rust | [213-multi-interval-fd-lean-scalar-default.md](213-multi-interval-fd-lean-scalar-default.md) |
| 214 | medium | confirmed | spec-docs-rust (unspecified) | Conformance third oracle fences shapes Lean and the engine already denote | [214-conformance-fences-shipped-shapes.md](214-conformance-fences-shipped-shapes.md) |
| 215 | low | confirmed | spec-docs-rust (spec) | Non-final interval FD is pointwise in Lean, refused in Rust | [215-functionality-interval-not-last.md](215-functionality-interval-not-last.md) |
| 216 | low | confirmed | spec-docs-rust (docs) | README type table omits `interval<E, w>` | [216-readme-omits-fixed-width-interval.md](216-readme-omits-fixed-width-interval.md) |
| 217 | low | confirmed | spec-docs-rust (unspecified) | Closed-relation 256-axiom cap is engine law; Lean `GroundExtension` is unbounded | [217-closed-roster-cap-unmodeled.md](217-closed-roster-cap-unmodeled.md) |
| 218 | medium | confirmed | spec-docs-rust (docs) | 70-api write-error roster omits `CapacityRayMeasure` | [218-api-roster-omits-capacity-ray-measure.md](218-api-roster-omits-capacity-ray-measure.md) |
| 219 | medium | confirmed | spec-docs-rust (split) | Fact identity is canonical bytes in Lean, blake3 of those bytes in the store | [219-hash-equality-vs-canonical-bytes.md](219-hash-equality-vs-canonical-bytes.md) |
| 220 | medium | confirmed | spec-docs-rust (spec) | Lean Duration weight of a ray is 0; engine refuses the commit | [220-capacity-ray-junk-zero.md](220-capacity-ray-junk-zero.md) |
| 221 | medium | confirmed | spec-docs-rust (docs) | Docs cite Lean grounding theorems for a negated complement fold Lean does not model | [221-negated-complement-fold-unmodeled.md](221-negated-complement-fold-unmodeled.md) |
| 222 | low | confirmed | spec-docs-rust (unspecified) | Lean `scanLoad` is one judgment; `bulk_load` is a sequence of 4096-fact commits | [222-bulk-load-chunking-vs-scanload.md](222-bulk-load-chunking-vs-scanload.md) |
| 223 | low | confirmed | spec-docs-rust (unspecified) | Schema fingerprint bytes are engine/docs law; Lean has no hash of a theory | [223-schema-fingerprint-unmodeled.md](223-schema-fingerprint-unmodeled.md) |
| 224 | medium | confirmed | spec-docs-rust (spec) | `membership_lowering_preserves` assumes membership-free negation; the engine does not | [224-membership-lowering-excludes-negated.md](224-membership-lowering-excludes-negated.md) |
| 225 | low | likely | spec-docs-rust (unspecified) | `OriginOverflow` and `ResultBytesOverflow` are runtime errors Lean does not denote | [225-origin-and-result-bytes-overflow.md](225-origin-and-result-bytes-overflow.md) |
| 301 | high | confirmed | persistence | Escaped fresh-ID flush failures are silently discarded | [301-escaped-fresh-id-flush-swallowed.md](301-escaped-fresh-id-flush-swallowed.md) |
| 302 | medium | confirmed | correctness | Fresh-row `F` conflict returns early and skips other key violations | [302-fresh-f-conflict-skips-other-keys.md](302-fresh-f-conflict-skips-other-keys.md) |
| 303 | medium | confirmed | correctness | `query!` interval literals emit a two-argument `Value::Interval*` constructor | [303-query-macro-interval-literal-arity.md](303-query-macro-interval-literal-arity.md) |
| 304 | medium | confirmed | correctness | `CommitRejected` can be replaced by a later `read_txn` / decode error | [304-commit-rejected-masked-by-decode.md](304-commit-rejected-masked-by-decode.md) |
| 305 | low | confirmed | correctness | Image decode reports scan ordinal as LMDB row id in `WrongFactWidth` | [305-wrong-fact-width-reports-scan-ordinal.md](305-wrong-fact-width-reports-scan-ordinal.md) |
| 306 | low | likely | correctness | C++ `Error::violations()` silently truncates a partial citation list | [306-cpp-violations-silent-truncate.md](306-cpp-violations-silent-truncate.md) |
| 307 | info | confirmed | other | `S` row-count arithmetic overflow is always labeled “underflow” | [307-s-row-count-overflow-mislabeled.md](307-s-row-count-overflow-mislabeled.md) |

---

## Related clusters

Not duplicates unless noted. Clusters are for anyone fixing or triaging: treat the group as one design conversation.

### 1. Prepared-query exclusive access (100 ↔ 112)

Same `!Sync` `PreparedQuery` scratch, two bridges.

- [100](100-napi-prepared-cross-thread-aliasing.md) — NAPI *always* aliases: JS `RefMut` plus worker `&mut`.
- [112](112-c-abi-prepared-no-exclusive-lock.md) — C ABI has no lock; concurrent execute/destroy is a data race. C++ `prepared_handle` is move-only (mitigates dialect, not the C pointer).

Nearby: [109](109-napi-take-handle-refcell-panic.md) (same NAPI handles, panicking close), [116](116-tx-ref-mut-from-ref-aliasing.md) (same missing exclusive on `bdb_tx_ref`).

### 2. Stale snapshot/tx refs and destroy-during-callback (101, 115, 102)

- [101](101-stale-snapshot-ref-is-stack-uaf.md) — documented MISUSE via stack `alive` is UAF after the callback.
- [115](115-stale-ref-test-is-uaf.md) — the unit test that “proves” 101 is itself UAF (green-wash). Not a second product bug; do not cite the test as evidence 101 works.
- [102](102-destroy-db-during-callback-uaf.md) — destroy/move of `Db` *during* the callback; C++ `Snapshot` also borrows `manifest_`.

Nearby: [111](111-cpp-answer-value-borrow-escape.md) (stash-the-borrow), [118](118-inbound-view-unbounded-lifetime.md) (lifetime lie, currently copied).

### 3. `out()` after side effect (104, 105, 110)

- [104](104-box-out-null-outparam-leak.md) — mint `Box`, then null `out()` leaks it.
- [105](105-bulk-load-null-out-committed.md) — durable import, then MISUSE if `out_committed` is null.
- [110](110-execute-clears-answers-on-error.md) — carrier cleared/partially filled before failure (C++ `execute_into` vs AGENTS.md failure-transparency).

Nearby: [114](114-store-error-overwrites-without-free.md) (overwrite leak), [222](222-bulk-load-chunking-vs-scanload.md) (chunked bulk_load vs one-shot Lean `scanLoad`).

### 4. Abort / Q-mark persistence (203, 204, 301) — cross-auditor

Same durability story, three angles. **Not duplicates.**

- [203](203-bridge-abort-fresh-discarded.md) — Bridge *premise* says aborted mint runs are discarded; `Fresh.lean` and the engine persist the high-water.
- [204](204-abort-never-touched-disk.md) — README / 70-api / crate root say abort never touched LMDB; abort burn writes `Q` marks. (`10-data-model.md` is already correct.)
- [301](301-escaped-fresh-id-flush-swallowed.md) — the burn exists, but abort-path `let _ = flush_escaped_fresh_ids(...)` **discards I/O failure**, so `never_reissue_observable` can break after ENOSPC/`CommitSync`. This is the only *runtime identity* hole in the cluster.

### 5. C20 / CapacityRayMeasure / ray weights (200, 218, 220, 210)

- [200](200-c20-ray-weight-absent-parent.md) — absent parent: Lean/docs no-op vs engine C20 refuse (**high**, split).
- [220](220-capacity-ray-junk-zero.md) — present parent: Lean junk-0 vs engine `CapacityRayMeasure`.
- [218](218-api-roster-omits-capacity-ray-measure.md) — 70-api write-error roster omits the typed error.
- [210](210-measure-of-ray-not-the-only-runtime-error.md) — “one runtime type error” slogan + 70-api query roster incomplete (`MeasureOfRay`, Overflow, budget, …).

### 6. ArgKey::Measure (201, 211, 214)

- [201](201-argkey-measure-missing-from-lean.md) — Lean `AggOp` is VarId-only; Rust/docs ship R5 measure keys (**high**).
- [211](211-ts-argkey-measure-missing.md) — TS query surface also cannot express measure Arg; C++/Rust can.
- [214](214-conformance-fences-shipped-shapes.md) — third oracle fences measure Arg (and negated/set membership).

### 7. Closed-relation admission (207, 208, 217)

Lean judgments broader than engine gates.

- [207](207-closed-target-key-broader-in-lean.md) — any matching FD vs synthetic `FieldId(0)`.
- [208](208-closed-containment-interval-unmodeled.md) — closed+interval containment refused v0.
- [217](217-closed-roster-cap-unmodeled.md) — 256-axiom `MemberSet` cap unmodeled.

### 8. Interval FD gates vs Lean `Statement.judgment` (213, 215)

- [213](213-multi-interval-fd-lean-scalar-default.md) — two interval fields → Lean scalar `Functionality`; Rust `FunctionalityMultipleIntervals`.
- [215](215-functionality-interval-not-last.md) — non-final interval → Lean pointwise; Rust `FunctionalityIntervalNotLast`.

### 9. Docs cite the wrong Lean theorem (202, 205, 221)

- [202](202-cookbook-claims-disjoint-dedup-elision.md) — cookbook “free lunch” elision; engine keeps spanning seen-set (**high**).
- [205](205-dnf-fold-cites-projection-theorem.md) — DNF fold cites `dnf_preserves_denotation` instead of `dnf_rekey_transparent`.
- [221](221-negated-complement-fold-unmodeled.md) — `fold_negated` cited against positive-only `grounding_preserves_answers`.

### 10. Negation / membership unmodeled or fenced (214, 224, 221)

- [224](224-membership-lowering-excludes-negated.md) — Bridge-cited `membership_lowering_preserves` requires membership-free negation; engine runs `AntiProbe`.
- [214](214-conformance-fences-shipped-shapes.md) — corpus drops negated membership (and set membership, measure Arg).
- [221](221-negated-complement-fold-unmodeled.md) — prepare-time complement fold.

### 11. Engine incompleteness vs Lean eval (206, 225, 210)

- [206](206-fixpoint-budget-incompleteness.md) — `FixpointBudgetExceeded` vs `program_eval_sound`.
- [225](225-origin-and-result-bytes-overflow.md) — `OriginCapacity` / `ResultBytesOverflow`; Lean still denotes the tuples.
- [210](210-measure-of-ray-not-the-only-runtime-error.md) — roster that should list these.

### 12. Documented MISUSE, actual UB (103, 108)

- [103](103-repr-c-enum-and-bool-unchecked.md) — invalid enum/`bool` discriminant is UB before MISUSE.
- [108](108-slice-in-count-overflow.md) — `from_raw_parts` without overflow/align check.

### 13. Panic / exception walls (107, 113, 109, 117)

- [107](107-unguarded-extern-panic-wall.md) — scalar accessors skip `guard`/`catch_unwind`.
- [113](113-cpp-exception-through-rust-callback.md) — C++ throw through Rust (in-tree `-fno-exceptions`; mixed-TU hazard).
- [109](109-napi-take-handle-refcell-panic.md) — panicking `borrow_mut` into napi.
- [117](117-moved-from-error-kind-is-panic.md) — moved-from `kind()` impersonates `PANIC`.

### 14. C++ dialect vs FFI (111, 113, 117, 102, 306, 110)

C++ SDK issues that sit next to, but are not the same as, the C ABI holes.

- [111](111-cpp-answer-value-borrow-escape.md) — `Value` holds `string_view`/`span` into the carrier.
- [102](102-destroy-db-during-callback-uaf.md) — `std::move(db)` inside `read`.
- [110](110-execute-clears-answers-on-error.md) — dialect `execute_into` vs AGENTS.md §26.
- [113](113-cpp-exception-through-rust-callback.md), [117](117-moved-from-error-kind-is-panic.md).
- [306](306-cpp-violations-silent-truncate.md) — `Error::violations()` breaks on a mid-list empty fetch (wrapper completeness, not UAF).

### 15. Incomplete `CommitRejected` / citation sets (302, 304, 306, 212)

Same *host diagnosis* contract (complete sealed violation set), three implementations plus a wrong comment.

- [302](302-fresh-f-conflict-skips-other-keys.md) — occupied fresh `F` records only the auto-key and skips other Functionality keys.
- [304](304-commit-rejected-masked-by-decode.md) — citation decode `?` can replace `CommitRejected` with `ReadersFull`/`Corruption`.
- [306](306-cpp-violations-silent-truncate.md) — C++ list can return a prefix with no error.
- [212](212-commitrejected-all-containment-comment.md) — public error comment says all-containment; statement phase mixes capacity (comment-only; runtime sealing matches Lean).

304 and 301 share the abort-path `write.rs` block (`flush` then decode).

### 16. Encoding / identity (209, 219, 216)

- [209](209-fixedbytes-word-vs-byte-encoding.md) — Lean `bytes<N>` is N Words; Rust stores N bytes padded to words.
- [219](219-hash-equality-vs-canonical-bytes.md) — Lean identity is canonical bytes; store membership is blake3 (collision axiom).
- [216](216-readme-omits-fixed-width-interval.md) — README type table missing `interval<E, w>`.

### 17. Lifetime fabrication (118, 100, 101)

- [118](118-inbound-view-unbounded-lifetime.md) — caller-chosen `'a` on inbound views; sound today only because inbound copies. Same shape as old Node `&'static Snapshot`.
- [100](100-napi-prepared-cross-thread-aliasing.md), [101](101-stale-snapshot-ref-is-stack-uaf.md) — the lies that *do* escape.

---

## Explicit: nothing was fixed

Every finding file has `status: open (do not fix)`. This index and the manifests do not close, downgrade, or merge any item. Source trees were not modified as part of this audit.
