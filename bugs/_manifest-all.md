# Combined audit manifest (all findings)

Audit date: 2026-08-12. Status: **open** — nothing was fixed.

Sorted by severity (critical → high → medium → low → info), then id.

Naming: `1NN` FFI, `2NN` spec/docs/rust, `3NN` general correctness.

| id | severity | confidence | file | one-line summary |
|---|---|---|---|---|
| 100 | critical | confirmed | 100-napi-prepared-cross-thread-aliasing.md | NAPI ships a `PreparedQuery` address to a worker while JS still holds `RefMut`; aliasing UB on every execute. |
| 101 | high | confirmed | 101-stale-snapshot-ref-is-stack-uaf.md | Snapshot/tx `alive` flag lives in a stack object; post-callback use is UAF, not MISUSE. |
| 102 | high | confirmed | 102-destroy-db-during-callback-uaf.md | `bdb_db_destroy` / C++ `Db` move during read/write frees the engine under live snapshot/tx/manifest refs. |
| 103 | high | confirmed | 103-repr-c-enum-and-bool-unchecked.md | Inbound `#[repr(C)]` enums and `bool`s are matched without discriminant checks; docs promised MISUSE. |
| 104 | high | confirmed | 104-box-out-null-outparam-leak.md | `box_out` + null `out()` leaks `Box` (LMDB env, prepared, row sets). |
| 105 | high | confirmed | 105-bulk-load-null-out-committed.md | Bulk load can commit (or partially commit) then return MISUSE if `out_committed` is null. |
| 112 | high | confirmed | 112-c-abi-prepared-no-exclusive-lock.md | C `bdb_prepared*` has no exclusive lock; concurrent execute/destroy races `!Sync` scratch. |
| 200 | high | confirmed | 200-c20-ray-weight-absent-parent.md | C20 refuses a ray Duration child under an absent parent; Lean `capacity_of_empty_parent` and architecture docs treat that insert as a no-op. |
| 201 | high | confirmed | 201-argkey-measure-missing-from-lean.md | `ArgKey::Measure` is in Rust/docs/R5; Lean `AggOp.argMax` is VarId-only; conformance fences the shape. |
| 202 | high | confirmed | 202-cookbook-claims-disjoint-dedup-elision.md | Cookbook recipe 22 (and TS twin) claims executor elides cross-rule dedup; Lean/40-execution/Rust keep a spanning seen-set. |
| 301 | high | confirmed | 301-escaped-fresh-id-flush-swallowed.md | Abort-path `flush_escaped_fresh_ids` errors are discarded, so `alloc()` ids can be reissued after a failed Q burn. |
| 106 | medium | confirmed | 106-napi-tx-open-stuck-on-spawn-panic.md | `tx_open` stays true if `thread::spawn` panics; later writes refuse forever. |
| 107 | medium | confirmed | 107-unguarded-extern-panic-wall.md | Several externs skip `catch_unwind`; panic into `-fno-exceptions` C++ is UB. |
| 108 | medium | likely | 108-slice-in-count-overflow.md | `slice_in` / `from_raw_parts` does not reject `count*size` overflow or misalignment. |
| 109 | medium | likely | 109-napi-take-handle-refcell-panic.md | `take_handle` uses panicking `borrow_mut`; re-entrant close unwinds into napi. |
| 110 | medium | confirmed | 110-execute-clears-answers-on-error.md | Execute clears/partial-fills the reusable answers carrier before failure. |
| 111 | medium | confirmed | 111-cpp-answer-value-borrow-escape.md | `cell()` / `Value` copies borrowed string/bytes pointers with no lifetime. |
| 113 | medium | possible | 113-cpp-exception-through-rust-callback.md | A throwing C++ callback unwinds through Rust; `catch_unwind` cannot catch it. |
| 116 | medium | likely | 116-tx-ref-mut-from-ref-aliasing.md | `transaction(&self) -> &mut WriteTx` plus non-atomic `alive`; concurrent callback use is UB. |
| 203 | medium | confirmed | 203-bridge-abort-fresh-discarded.md | Bridge premise says aborted mint runs are discarded; `Fresh.lean` and the engine persist the high-water. |
| 204 | medium | confirmed | 204-abort-never-touched-disk.md | README/70-api claim abort never touched LMDB; abort burn writes `Q` marks. |
| 205 | medium | confirmed | 205-dnf-fold-cites-projection-theorem.md | DNF “fold-preserving” cites `dnf_preserves_denotation` (projection); fold law is `dnf_rekey_transparent`. |
| 206 | medium | confirmed | 206-fixpoint-budget-incompleteness.md | Engine `FixpointBudgetExceeded` is incomplete vs Lean `evalProgram` / `program_eval_sound`. |
| 207 | medium | confirmed | 207-closed-target-key-broader-in-lean.md | `TargetKeyAccepted` is any matching FD; Rust closed targets require synthetic `FieldId(0)`. |
| 208 | medium | confirmed | 208-closed-containment-interval-unmodeled.md | Closed+interval containment is a Lean judgment; engine `ClosedContainmentInterval` refuses v0. |
| 209 | medium | confirmed | 209-fixedbytes-word-vs-byte-encoding.md | Lean `bytes<N>` is N Words; Rust/docs store N bytes padded to ⌈N/8⌉×8. |
| 210 | medium | confirmed | 210-measure-of-ray-not-the-only-runtime-error.md | Docs call `MeasureOfRay` the one runtime type error; 70-api omits it and other query aborts exist. |
| 211 | medium | confirmed | 211-ts-argkey-measure-missing.md | TS `argMax` keys are variables only; Rust/C++/docs admit `Duration` keys. |
| 213 | medium | confirmed | 213-multi-interval-fd-lean-scalar-default.md | Two interval fields → Lean scalar `Functionality`; Rust `FunctionalityMultipleIntervals`. |
| 214 | medium | confirmed | 214-conformance-fences-shipped-shapes.md | Third oracle excludes negated membership, set membership, measure Arg — shipped elsewhere. |
| 218 | medium | confirmed | 218-api-roster-omits-capacity-ray-measure.md | 70-api write errors omit `CapacityRayMeasure`. |
| 219 | medium | confirmed | 219-hash-equality-vs-canonical-bytes.md | Lean identity is canonical bytes; store membership is blake3 with collision axiom. |
| 220 | medium | confirmed | 220-capacity-ray-junk-zero.md | Lean `durationNat` of a ray is 0; engine `CapacityRayMeasure` (undefined, not false). |
| 221 | medium | confirmed | 221-negated-complement-fold-unmodeled.md | Docs cite Lean grounding theorems for `fold_negated`; Lean leaves the complement fold unmodeled. |
| 224 | medium | confirmed | 224-membership-lowering-excludes-negated.md | Bridge-cited `membership_lowering_preserves` requires membership-free negation; engine runs `AntiProbe`. |
| 302 | medium | confirmed | 302-fresh-f-conflict-skips-other-keys.md | Occupied `F` on a fresh-keyed insert records only the auto-key and skips other Functionality keys. |
| 303 | medium | confirmed | 303-query-macro-interval-literal-arity.md | `query!` emits `Value::IntervalU64(start, end)` but the variant takes `Interval<T>` — `start..end` does not compile. |
| 304 | medium | confirmed | 304-commit-rejected-masked-by-decode.md | Citation decode uses `?` on a new read txn, so `CommitRejected` can become `ReadersFull` / `Corruption`. |
| 114 | low | confirmed | 114-store-error-overwrites-without-free.md | Reused `bdb_error**` without destroy leaks the previous error. |
| 117 | low | confirmed | 117-moved-from-error-kind-is-panic.md | Moved-from `error_handle::kind()` returns `PANIC` instead of aborting. |
| 118 | low | possible | 118-inbound-view-unbounded-lifetime.md | `as_str`/`slice_in` fabricate caller-chosen lifetimes; sound today only because inbound copies. |
| 212 | low | confirmed | 212-commitrejected-all-containment-comment.md | `CommitRejected` comment says all-containment; statement phase mixes capacity citations. |
| 215 | low | confirmed | 215-functionality-interval-not-last.md | Non-final interval FD is pointwise in Lean; Rust `FunctionalityIntervalNotLast`. |
| 216 | low | confirmed | 216-readme-omits-fixed-width-interval.md | README type table has no `interval<E,w>` row. |
| 217 | low | confirmed | 217-closed-roster-cap-unmodeled.md | Engine/docs cap closed axioms at 256; Lean `GroundExtension` is unbounded. |
| 222 | low | confirmed | 222-bulk-load-chunking-vs-scanload.md | Lean `scanLoad` is one judgment; `bulk_load` is 4096-fact commit sequence. |
| 223 | low | confirmed | 223-schema-fingerprint-unmodeled.md | Open identity is blake3 v5; Lean `Theory` has no fingerprint. |
| 225 | low | likely | 225-origin-and-result-bytes-overflow.md | `OriginCapacity` / `ResultBytesOverflow` abort queries Lean still denotes. |
| 305 | low | confirmed | 305-wrong-fact-width-reports-scan-ordinal.md | Image `WrongFactWidth.row_id` is the scan position, not the `F` key row id. |
| 306 | low | likely | 306-cpp-violations-silent-truncate.md | C++ `Error::violations()` breaks on a mid-list empty fetch and returns a prefix with no error. |
| 115 | info | confirmed | 115-stale-ref-test-is-uaf.md | `stale_snapshot_ref_is_misuse` dereferences a dropped stack ref (green-washes 101). |
| 307 | info | confirmed | 307-s-row-count-overflow-mislabeled.md | `checked_add_signed` failure on `S` is always reported as “underflow”. |

## Counts

**By severity:** critical 1, high 10, medium 27, low 12, info 2. **Total 52.**

**By area (id range):** FFI 19 (100–118), spec 26 (200–225), general 7 (301–307).

**By confidence:** confirmed 45, likely 5, possible 2.

**By auditor area field:** ffi 19, spec-docs-rust 26, correctness 5, persistence 1, other 1.
