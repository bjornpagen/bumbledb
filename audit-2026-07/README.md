# bumbledb deep audit — 2026-07-23

## Tally

### By verdict × severity

| Verdict   | Critical | High | Medium | Low | Total |
|-----------|---------:|-----:|-------:|----:|------:|
| Confirmed | 0        | 26   | 60     | 72  | 158   |
| Plausible | 1        | 0    | 2      | 1   | 4     |
| Refuted   | —        | —    | —      | —   | 2     |

### By category

| Category                | Confirmed | Plausible | Refuted | Total |
|-------------------------|----------:|----------:|--------:|------:|
| incoherence             | 40        | 0         | 0       | 40    |
| perf                    | 29        | 2         | 0       | 31    |
| missing-free-feature    | 25        | 0         | 0       | 25    |
| bug                     | 23        | 1         | 1       | 25    |
| unification             | 19        | 0         | 0       | 19    |
| inelegance              | 8         | 0         | 0       | 8     |
| inappropriate-branching | 7         | 0         | 0       | 7     |
| bench-honesty           | 5         | 0         | 1       | 6     |
| lean-rust-drift         | 2         | 1         | 0       | 3     |

## Confirmed findings

| # | Title | Category | Severity | Primary file |
|---|-------|----------|----------|--------------|
| [002](./findings/002-pump-s-inner-batch-loop-never-checks-all-cancelled.md) | pump's inner batch loop never checks all_cancelled — a whole-execution D2 skip still iterates and probes the entire remaining cover | perf | high | `crates/bumbledb/src/exec/run/pump.rs` |
| [003](./findings/003-shared-probe-hash-has-no-final-avalanche-tail-padd.md) | Shared probe hash has no final avalanche: tail-padded bytes<N> keys pile into ONE bucket | perf | high | `crates/bumbledb/src/exec/swar.rs` |
| [004](./findings/004-verify-store-s-dict-pass-checks-nothing-referenced.md) | verify_store's _dict pass checks nothing: referenced-id-without-reverse and forward/reverse desync pass a clean sweep | missing-free-feature | high | `crates/bumbledb/src/verify_store/dict_stat.rs` |
| [005](./findings/005-selectivity-ladder-reads-lmdb-row-counter-for-clos.md) | Selectivity ladder reads LMDB row counter for closed containment targets, which is always 0 | bug | high | `crates/bumbledb/src/plan/selectivity.rs` |
| [006](./findings/006-dp-cost-model-ignores-all-cross-atom-residual-sele.md) | DP cost model ignores all cross-atom residual selectivity — Allen-joined atoms price as pure Cartesian products | perf | high | `crates/bumbledb/src/plan/planner/estimate.rs` |
| [007](./findings/007-or-condition-on-a-sum-count-aggregate-head-silentl.md) | OR condition on a Sum/Count aggregate head silently coarsens the fold domain; engine and naive oracle diverge | bug | high | `crates/bumbledb/src/api/prepared/build.rs` |
| [008](./findings/008-cheap-alu-residuals-run-after-expensive-hash-probe.md) | Cheap ALU residuals run AFTER expensive hash probes in every join node — r2 leaves ~50x, r1 ~2x on the table | perf | high | `crates/bumbledb/src/exec/run/probe_pass.rs` |
| [009](./findings/009-the-generic-join-half-of-free-join-is-unreachable-.md) | The generic-join half of Free Join is unreachable: no plan constructor ever splits a probe subatom, so cyclic queries always execute the binary-join closing probe | missing-free-feature | high | `crates/bumbledb/src/plan/fj/binary2fj.rs` |
| [010](./findings/010-snapshot-point-reads-allocate-per-call-the-flagshi.md) | Snapshot point reads allocate per call — the flagship keyed-get lane violates the allocation-free point-read contract | perf | high | `crates/bumbledb/src/api/db/snapshot.rs` |
| [011](./findings/011-keyed-get-re-fetches-the-key-it-was-given-decode-r.md) | Keyed get re-fetches the key it was given: decode resolves determinant string fields through the reverse dictionary | missing-free-feature | high | `crates/bumbledb/src/api/db/snapshot.rs` |
| [012](./findings/012-t2-r2-per-key-allen-self-join-enumerates-n-k-2-pai.md) | t2/r2: per-key Allen self-join enumerates n_k^2 pairs; 2.7% survive — no order-based overlap join | perf | high | `crates/bumbledb/src/exec/run/run_node.rs` |
| [013](./findings/013-olap-folds-trie-factorization-is-fold-blind-so-the.md) | OLAP folds: trie factorization is fold-blind, so the scan-fold pushdown almost never fires (o3/o5 8-19x) | missing-free-feature | high | `crates/bumbledb/src/plan/fj/binary2fj.rs` |
| [014](./findings/014-o4-the-leaf-runs-per-parent-500k-batch-of-1-run-no.md) | o4: the leaf runs per parent — 500k batch-of-1 run_node calls for a fanout-1 determinant lookup | perf | high | `crates/bumbledb/src/exec/run/probe_pass.rs` |
| [015](./findings/015-program-form-named-params-get-ids-by-group-emissio.md) | Program-form named params get ids by group-emission order, not first occurrence — silent wrong bindings | bug | high | `crates/bumbledb-query-macros/src/lib.rs` |
| [016](./findings/016-every-scoped-read-pays-a-separate-dbgeneration-ffi.md) | Every scoped read pays a separate dbGeneration FFI call (and its fault-pairing dance) that the snapshot could carry for free | perf | high | `ts/src/db.ts` |
| [017](./findings/017-coordinate-encoding-relation-field-is-not-injectiv.md) | Coordinate encoding `${relation}.${field}` is not injective — dotted names corrupt the class map at both tiers | bug | high | `ts/src/law.ts` |
| [018](./findings/018-witness-snapshot-dangles-for-the-whole-transaction.md) | Witness &Snapshot dangles for the whole transaction if snapshot closes before commit | bug | high | `ts/crate/src/lib.rs` |
| [019](./findings/019-ts-sdk-forbids-idb-grounded-variables-taxing-every.md) | TS SDK forbids idb-grounded variables, taxing every recursive query with a spurious re-grounding join the engine never asked for | missing-free-feature | high | `ts/src/query/lower.ts` |
| [020](./findings/020-bench-ephemeral-cross-matches-durability-mdb-nosyn.md) | bench --ephemeral cross-matches durability: MDB_NOSYNC engine vs fullfsync SQLite in write families | bench-honesty | high | `crates/bumbledb-bench/src/driver/write_families.rs` |
| [021](./findings/021-dbwritefrom-witness-safety-comment-states-a-borrow.md) | dbWriteFrom witness SAFETY comment states a borrow that is never held; pointer outlives its argued window | bug | high | `ts/crate/src/lib.rs` |
| [022](./findings/022-db-create-never-fsyncs-the-store-directory-s-diren.md) | Db::create never fsyncs the store directory's dirent chain — a durable store can vanish whole on power loss | bug | high | `crates/bumbledb/src/storage/env/create.rs` |
| [023](./findings/023-lean-lane-runs-the-three-way-comparator-twice-and-.md) | Lean lane runs the three-way comparator twice, and the in-script run defeats the cargo cache | unification | high | `.github/workflows/ci.yml` |
| [024](./findings/024-naive-oracle-s-measureofray-verdict-is-evaluation-.md) | Naive oracle's MeasureOfRay verdict is evaluation-order-dependent; diverges from DNF lowering | bug | high | `crates/bumbledb-bench/src/naive/query.rs` |
| [025](./findings/025-pack-is-unspellable-by-the-randomized-generator-th.md) | Pack is unspellable by the randomized generator; the random lane's type forces grammar ⊆ SQL-expressible | missing-free-feature | high | `crates/bumbledb-bench/src/querygen.rs` |
| [026](./findings/026-interval-typed-predicate-columns-engine-legal-naiv.md) | Interval-typed predicate columns: engine-legal, naive fully implements them, zero cases exist anywhere | missing-free-feature | high | `crates/bumbledb-bench/src/querygen/shapes_recursive.rs` |
| [027](./findings/027-union-regime-aggregate-fold-domain-has-no-lean-law.md) | Union-regime aggregate fold domain has no Lean law; docs cite union_regime_head_projection beyond its statement; OR+aggregate coarsening unpinned | incoherence | high | `lean/Bumbledb/Exec/Dedup.lean` |
| [028](./findings/028-every-filter-kernel-pays-a-full-width-memset-of-th.md) | Every filter kernel pays a full-width memset of the output before the cursor-write | perf | medium | `crates/bumbledb/src/exec/kernel/filter.rs` |
| [029](./findings/029-filter-chunked-hand-rolls-survivor-compaction-with.md) | filter_chunked hand-rolls survivor compaction with the exact branches the module's own kernels erased | inappropriate-branching | medium | `crates/bumbledb/src/exec/kernel/allen.rs` |
| [030](./findings/030-run-node-and-anti-probe-still-pay-the-clear-resize.md) | run_node and anti_probe still pay the clear+resize re-memset that probe_pass's grow_scratch contract already priced at 3.7% | incoherence | medium | `crates/bumbledb/src/exec/run/run_node.rs` |
| [031](./findings/031-wordmap-const-arity-dispatch-has-holes-at-0-5-7-wi.md) | WordMap const-arity dispatch has holes at 0, 5, 7 — widths its own comment declares common take the #[cold] path per row | incoherence | medium | `crates/bumbledb/src/exec/wordmap/entry.rs` |
| [032](./findings/032-colt-force-pass-re-resolves-the-column-view-per-po.md) | COLT force pass re-resolves the column view per (position, column) — the exact cost the iteration path hoisted away | perf | medium | `crates/bumbledb/src/exec/colt/force.rs` |
| [033](./findings/033-verify-store-never-sweeps-the-q-namespace-fresh-se.md) | verify_store never sweeps the Q namespace: fresh-sequence never-reissue invariant unverified | missing-free-feature | medium | `crates/bumbledb/src/verify_store.rs` |
| [034](./findings/034-permuted-determinant-image-runs-an-o-k-inverse-per.md) | permuted_determinant_image runs an O(k²) inverse-permutation search per fact on the commit hot path and the O(store) sweep | perf | medium | `crates/bumbledb/src/storage/keys.rs` |
| [035](./findings/035-encode-literal-is-the-second-per-field-encoder-the.md) | encode_literal is the second per-field encoder the module's own doc says cannot exist | unification | medium | `crates/bumbledb/src/encoding/encode.rs` |
| [036](./findings/036-key-coverage-fanout-skips-eq-pinned-key-columns-th.md) | Key-coverage fanout skips Eq-pinned key columns that provably_distinct already counts | unification | medium | `crates/bumbledb/src/plan/planner/densify.rs` |
| [037](./findings/037-fact-encode-read-is-dead-machinery-the-typed-snaps.md) | Fact::encode_read is dead machinery: the typed Snapshot::contains it exists for is missing | missing-free-feature | medium | `crates/bumbledb/src/api/db/snapshot.rs` |
| [038](./findings/038-snapshot-point-reads-allocate-per-call-while-their.md) | Snapshot point reads allocate per call while their WriteTx twins are pooled — against the engine's own point-path posture | perf | medium | `crates/bumbledb/src/api/db/snapshot.rs` |
| [039](./findings/039-schemawarning-is-computed-sealed-and-then-unreacha.md) | SchemaWarning is computed, sealed, and then unreachable through the Db lifecycle | incoherence | medium | `crates/bumbledb/src/api/db/open.rs` |
| [040](./findings/040-trace-spans-pay-two-u128-divisions-per-stamp-trace.md) | Trace spans pay two u128 divisions per stamp; TraceEvent should carry ticks, converting once at drain | perf | medium | `crates/bumbledb/src/obs.rs` |
| [041](./findings/041-countingallocator-does-not-forward-alloc-zeroed-re.md) | CountingAllocator does not forward alloc_zeroed, replacing calloc with malloc+memset under measurement | bench-honesty | medium | `crates/bumbledb/src/alloc_counter.rs` |
| [042](./findings/042-mirror-pairing-uses-raw-side-equality-while-statem.md) | Mirror pairing uses raw side equality while statement identity is normalized | bug | medium | `crates/bumbledb/src/schema/validate.rs` |
| [043](./findings/043-multi-rule-nullary-count-degenerates-to-1-per-grou.md) | Multi-rule nullary Count degenerates to <=1 per group; ir.rs contract and the docs' own example contradict the shipped semantics | incoherence | medium | `crates/bumbledb/src/ir.rs` |
| [044](./findings/044-r6-s-distinct-2-path-count-re-proves-distinctness-.md) | r6's distinct 2-path count re-proves distinctness in a 5M-entry seen-set that the COLT forced maps already hold | perf | medium | `crates/bumbledb/src/exec/sink.rs` |
| [045](./findings/045-snapshot-get-get-dyn-allocate-per-call-and-pay-thr.md) | Snapshot::get/get_dyn allocate per call and pay three LMDB descents — the flagship keyed-get lane (p5) is slower than the full query machinery it exists to bypass | lean-rust-drift | medium | `crates/bumbledb/src/api/db/snapshot.rs` |
| [046](./findings/046-determinant-row-zeroes-a-full-511-byte-keybuf-per-.md) | determinant_row zeroes a full 511-byte KeyBuf per probe — the exact oversized-zeroing waste post-mortem §25 banned and fact_row already fixed | perf | medium | `crates/bumbledb/src/storage/read/determinant_row.rs` |
| [047](./findings/047-the-lone-fresh-auto-key-s-u-tree-is-a-transcriptio.md) | The lone-fresh auto-key's U tree is a transcription of F: fresh ids and row ids are two monotone u64 allocators that are secretly one | unification | medium | `crates/bumbledb/src/storage/commit/applier.rs` |
| [048](./findings/048-parent-constant-allen-operand-re-materialized-per-.md) | Parent-constant Allen operand re-materialized per element though the const-operand kernel already exists | unification | medium | `crates/bumbledb/src/exec/run/run_node.rs` |
| [049](./findings/049-closed-relation-group-keys-hash-through-wordmap-pe.md) | Closed-relation group keys hash through WordMap per row; the dense ≤256 domain the schema already proves is unused | missing-free-feature | medium | `crates/bumbledb/src/exec/sink/aggregate/groups.rs` |
| [050](./findings/050-per-survivor-loop-invariant-searches-in-membership.md) | Per-survivor loop-invariant searches in membership/anti-probe and routing passes | inappropriate-branching | medium | `crates/bumbledb/src/exec/run/probe_pass.rs` |
| [051](./findings/051-measure-filter-path-re-partitions-and-deep-clones-.md) | Measure-filter path re-partitions and deep-clones the filter list per view build, leaking the pooled survivor buffer | perf | medium | `crates/bumbledb/src/image/view/apply.rs` |
| [052](./findings/052-run-node-kept-the-memset-scratch-sizing-its-line-p.md) | run_node kept the memset scratch-sizing its line-parallel twin measured out of probe_pass | incoherence | medium | `crates/bumbledb/src/exec/run/run_node.rs` |
| [053](./findings/053-the-filter-evaluation-match-tree-exists-twice-view.md) | The filter-evaluation match tree exists twice: view row_matches vs key-probe fact_matches | unification | medium | `crates/bumbledb/src/exec/dispatch/key_probe_fact.rs` |
| [054](./findings/054-lowercase-spelling-of-a-relation-compiles-silently.md) | Lowercase spelling of a relation compiles silently — the UpperCamel/lowercase partition is enforced on heads only | incoherence | medium | `crates/bumbledb-query-macros/src/lib.rs` |
| [055](./findings/055-commas-between-body-items-and-head-terms-are-silen.md) | Commas between body items and head terms are silently optional — the parser accepts a superset of the sacred grammar | incoherence | medium | `crates/bumbledb-query-macros/src/lib.rs` |
| [056](./findings/056-fresh-on-a-non-u64-field-emits-type-mismatched-fre.md) | `fresh` on a non-u64 field emits type-mismatched Fresh/Key impls — raw rustc errors bury the typed teaching path | bug | medium | `crates/bumbledb-macros/src/lib.rs` |
| [057](./findings/057-duplicate-field-in-a-declared-fd-projection-genera.md) | Duplicate field in a declared FD projection generates a key struct with duplicate fields — E0124 instead of the typed DuplicateProjectionField error | bug | medium | `crates/bumbledb-macros/src/lib.rs` |
| [058](./findings/058-emit-newtypes-erases-the-interval-width-one-newtyp.md) | emit_newtypes erases the interval width — one newtype silently spans two encodings, contradicting 'the width is the type' | bug | medium | `crates/bumbledb-macros/src/lib.rs` |
| [059](./findings/059-schemaspec-descriptor-panics-on-wide-relations-ins.md) | SchemaSpec::descriptor panics on wide relations instead of issuing a typed SpecIssue | bug | medium | `crates/bumbledb-theory/src/schema/spec.rs` |
| [060](./findings/060-db-write-silently-commits-when-the-callback-return.md) | db.write silently commits when the callback returns abandon(payload) | bug | medium | `ts/src/db.ts` |
| [061](./findings/061-tx-insert-discards-the-engine-s-changed-state-bool.md) | Tx.insert discards the engine's changed-state boolean that already crosses the FFI | missing-free-feature | medium | `ts/src/db.ts` |
| [062](./findings/062-statement-is-unbranded-so-schema-admits-forged-sta.md) | Statement is unbranded, so schema() admits forged statements that bypass the roster wall the engine cannot backstop | bug | medium | `ts/src/statements.ts` |
| [063](./findings/063-type-tier-roster-slot-compares-handle-name-unions-.md) | Type-tier roster slot compares handle-name unions; runtime compares roster identity — well-typed statements throw at construction | incoherence | medium | `ts/src/face.ts` |
| [064](./findings/064-answer-scan-rows-cross-the-ffi-with-two-avoidable-.md) | Answer/scan rows cross the FFI with two avoidable copies per string/bytes cell | perf | medium | `ts/crate/src/marshal.rs` |
| [065](./findings/065-wire-tags-could-emit-the-parse-direction-for-unit-.md) | wire_tags! could emit the parse direction for unit variants, closing its own admitted drift gap | unification | medium | `ts/crate/src/tags.rs` |
| [066](./findings/066-exhumehandle-has-no-close-the-store-s-exclusive-lo.md) | ExhumeHandle has no close — the store's exclusive lock is held hostage to GC | missing-free-feature | medium | `ts/crate/src/lib.rs` |
| [067](./findings/067-negation-of-a-finished-stratum-is-engine-legal-and.md) | Negation of a finished stratum is engine-legal and Rust-spellable but unwritable in the TS SDK | missing-free-feature | medium | `ts/src/query/atom.ts` |
| [068](./findings/068-documented-any-all-idiom-min-max-over-bool-is-a-ty.md) | Documented Any/All idiom (Min/Max over bool) is a typed rejection in the engine — the idiom is unspellable on every surface | incoherence | medium | `docs/architecture/10-data-model.md` |
| [069](./findings/069-closed-reference-orderability-refusal-orderability.md) | Closed-reference orderability refusal ('Orderability, complete') is enforced only by TS types — engine validation and the Rust surface order vocabularies silently | incoherence | medium | `docs/architecture/10-data-model.md` |
| [070](./findings/070-program-conformance-arm-never-runs-the-engine-s-fi.md) | Program conformance arm never runs the engine's fixpoint driver, despite claiming to | missing-free-feature | medium | `crates/bumbledb-bench/src/conformance/program.rs` |
| [071](./findings/071-two-durabilitylane-enums-encode-the-same-axis-dura.md) | Two DurabilityLane enums encode the same axis — duralane.rs and lanes/writes.rs | unification | medium | `crates/bumbledb-bench/src/lanes/writes.rs` |
| [072](./findings/072-curves-lane-times-both-engines-with-no-clock-proxy.md) | Curves lane times both engines with no clock-proxy bracket and no DVFS warm-up — published numbers carry no contamination field | bench-honesty | medium | `crates/bumbledb-bench/src/lanes/curves.rs` |
| [073](./findings/073-rng-u64-seeded-arm-emits-only-31-bits-state-33-pay.md) | Rng::u64() seeded arm emits only 31 bits (state >> 33) — payload entropy claim false, range(n>2^31) silently broken, arms diverge | bug | medium | `crates/bumbledb-bench/src/corpus_gen/rng.rs` |
| [074](./findings/074-open-for-bench-pins-mmap-size-at-1-gib-with-no-cov.md) | open_for_bench pins mmap_size at 1 GiB with no coverage check — the memory-residency parity claim silently breaks at L, the gating scale | bench-honesty | medium | `crates/bumbledb-bench/src/sqlite_run/open_for_bench.rs` |
| [075](./findings/075-colt-gather-segment-s-get-unchecked-invariant-now-.md) | colt gather_segment's get_unchecked invariant now has zero standing referees: debug-only assert, Miri-excluded, fuzz replay deleted | bug | medium | `crates/bumbledb/src/exec/colt/gather.rs` |
| [076](./findings/076-query-results-crossing-outward-copy-every-string-b.md) | Query results crossing outward copy every string/bytes payload twice — rows_out clones cells it already owns | perf | medium | `ts/crate/src/marshal.rs` |
| [077](./findings/077-miri-lane-coverage-does-not-match-the-repo-s-unsaf.md) | Miri lane coverage does not match the repo's unsafe surface: NEON never interpreted anywhere, ts crate uncovered, arena mis-filed | incoherence | medium | `scripts/miri.sh` |
| [078](./findings/078-verify-store-never-checks-dict-reverse-ids-against.md) | verify_store never checks _dict reverse ids against META_DICT_NEXT_ID — a regressed counter arms silent reverse-map overwrites and sweeps clean | missing-free-feature | medium | `crates/bumbledb/src/verify_store/dict_stat.rs` |
| [079](./findings/079-sdk-test-glob-works-only-by-accident-of-sh-non-mat.md) | SDK test glob works only by accident of sh non-match; one subdir .test.ts silently shadows all 37 suites | bug | medium | `ts/package.json` |
| [080](./findings/080-ci-s-runner-must-be-darwin-arm64-rationale-is-stal.md) | CI's 'runner must BE darwin-arm64' rationale is stale — build.ts stopped hardcoding darwin a day after the comment was written | incoherence | medium | `.github/workflows/ci.yml` |
| [081](./findings/081-filtered-test-gates-in-check-sh-and-ci-yml-can-pas.md) | Filtered test gates in check.sh and ci.yml can pass vacuously on rename — the guard lean.sh invented is not unified | incoherence | medium | `scripts/check.sh` |
| [082](./findings/082-version-lockstep-gate-blind-spots-the-bridge-crate.md) | Version lockstep gate blind spots: the bridge crate version is ungated and the smoke assertion that could catch it checks only non-emptiness | unification | medium | `ts/scripts/build.ts` |
| [083](./findings/083-61-bench-lanes-lane-registry-omits-the-shipped-cru.md) | 61-bench-lanes lane registry omits the shipped crud and lawful lanes it exists to register | incoherence | medium | `docs/architecture/61-bench-lanes.md` |
| [084](./findings/084-normative-docs-cite-bench-out-measurement-artifact.md) | Normative docs cite bench-out measurement artifacts deleted by the 2026-07-20 pin swap | incoherence | medium | `docs/architecture/00-product.md` |
| [085](./findings/085-or-tree-grammar-meets-the-engine-only-in-one-degen.md) | OR-tree grammar meets the engine only in one degenerate corner: single atom, scalar leaves, projection head | missing-free-feature | medium | `crates/bumbledb-bench/src/verify/run_algebra.rs` |
| [086](./findings/086-bind-time-allen-mask-params-maskterm-param-have-no.md) | Bind-time Allen mask params (MaskTerm::Param) have no randomized or parity coverage | missing-free-feature | medium | `crates/bumbledb-bench/src/translate/builder.rs` |
| [087](./findings/087-membership-under-additive-fold-known-lean-vs-engin.md) | Membership-under-additive-fold: known Lean-vs-engine fold-domain divergence is fenced out of the corpus instead of pinned | lean-rust-drift | medium | `crates/bumbledb-bench/src/conformance.rs` |
| [090](./findings/090-neon-allen-kernels-re-classify-the-last-full-windo.md) | NEON Allen kernels re-classify the last full window when len is lane-aligned — always, on the chunked path | perf | low | `crates/bumbledb/src/exec/kernel/neon.rs` |
| [091](./findings/091-fold-extent-asserts-the-documented-safety-guard-fo.md) | Fold extent asserts — the documented safety guard for get_unchecked — wrap in release | bug | low | `crates/bumbledb/src/exec/kernel/fold.rs` |
| [092](./findings/092-membership-probe-and-anti-probe-point-passes-do-pe.md) | Membership-probe and anti-probe point passes do per-element word_base/width_of/subatom-position searches (plus a &dyn Fn) that every other pass hoists | inelegance | low | `crates/bumbledb/src/exec/run/run_node.rs` |
| [093](./findings/093-probe-pass-never-ensure-forces-batch-constant-sibl.md) | probe_pass never ensure_forces batch-constant sibling cursors, so first-pass prefetches no-op and force time hides inside the Probe phase | perf | low | `crates/bumbledb/src/exec/run/probe_pass.rs` |
| [094](./findings/094-fixed-264-byte-chunks-make-fanout-2-keys-cost-33x-.md) | Fixed 264-byte chunks make fanout-2 keys cost ~33x their payload in the chunk pool | perf | low | `crates/bumbledb/src/exec/colt/append_child.rs` |
| [095](./findings/095-cardinalitycounter-carries-a-retired-design-stale-.md) | CardinalityCounter carries a retired design: stale shared-counter doc, dead memset, per-column reallocation | incoherence | low | `crates/bumbledb/src/image/cardinality.rs` |
| [096](./findings/096-test-only-intern-str-keeps-a-second-divergent-dict.md) | Test-only intern_str keeps a second, divergent dictionary-write implementation alive | unification | low | `crates/bumbledb/src/storage/dict.rs` |
| [097](./findings/097-restore-determinants-rescans-the-whole-pending-fac.md) | restore_determinants rescans the whole pending-fact set per key statement, making cancel-heavy transactions quadratic | perf | low | `crates/bumbledb/src/storage/delta/determinants.rs` |
| [098](./findings/098-the-bytes-n-zero-pad-law-is-implemented-three-inde.md) | The bytes<N> zero-pad law is implemented three independent times | unification | low | `crates/bumbledb/src/encoding/encode.rs` |
| [099](./findings/099-interval-decoders-validate-then-hand-back-unparsed.md) | Interval decoders validate then hand back unparsed tuples, forcing re-parse-with-expect | inelegance | low | `crates/bumbledb/src/encoding/decode.rs` |
| [100](./findings/100-estimates-carries-the-doc-comment-of-a-deleted-suf.md) | estimates() carries the doc comment of a deleted suffix-skip eligibility method | incoherence | low | `crates/bumbledb/src/plan/fj.rs` |
| [101](./findings/101-manifest-rendering-re-materializes-the-whole-state.md) | Manifest rendering re-materializes the whole statement list per statement — O(N^2) clones | perf | low | `crates/bumbledb/src/schema/manifest.rs` |
| [102](./findings/102-answers-re-validates-utf-8-on-every-string-cell-ac.md) | Answers re-validates UTF-8 on every string-cell access despite validate-at-materialization | inelegance | low | `crates/bumbledb/src/api/prepared/answers.rs` |
| [103](./findings/103-closed-target-key-resolution-rejects-declared-keys.md) | Closed-target key resolution rejects declared keys while citing them as available candidates | incoherence | low | `crates/bumbledb/src/schema/validate.rs` |
| [104](./findings/104-schemaerror-statement-is-a-45-line-variant-roster-.md) | SchemaError::statement() is a 45-line variant roster that a two-level representation would erase | inappropriate-branching | low | `crates/bumbledb/src/error/display.rs` |
| [105](./findings/105-schemamismatch-and-descriptorfingerprintdesync-car.md) | SchemaMismatch and DescriptorFingerprintDesync carry both fingerprints but Display renders neither | incoherence | low | `crates/bumbledb/src/error/display.rs` |
| [106](./findings/106-arena-oversized-spill-strands-the-open-chunk-s-fre.md) | Arena oversized spill strands the open chunk's free tail because only chunks.last() is consulted | perf | low | `crates/bumbledb/src/arena.rs` |
| [107](./findings/107-engine-allen-rs-re-tests-the-theory-crate-s-mask-l.md) | Engine allen.rs re-tests the theory crate's mask laws verbatim while its own module doc says they live theory-side | incoherence | low | `crates/bumbledb/src/allen.rs` |
| [108](./findings/108-keystatement-pointwise-is-a-bool-the-commit-plan-r.md) | KeyStatement.pointwise is a bool; the commit plan re-derives the IntervalTail per fact op | inappropriate-branching | low | `crates/bumbledb/src/schema.rs` |
| [109](./findings/109-validate-cardinality-copy-pastes-containment-s-who.md) | validate_cardinality copy-pastes containment's whole side-pair gate | unification | low | `crates/bumbledb/src/schema/validate.rs` |
| [110](./findings/110-manifest-rendering-is-o-n-2-materialized-statement.md) | Manifest rendering is O(n^2): materialized_statements re-cloned per statement | perf | low | `crates/bumbledb/src/schema/manifest.rs` |
| [111](./findings/111-atom-relation-is-a-public-panicking-accessor-on-th.md) | Atom::relation() is a public panicking accessor on the pure-data IR; the Option form already exists | inappropriate-branching | low | `crates/bumbledb/src/ir.rs` |
| [112](./findings/112-run-node-s-prefetch-comment-claims-a-residency-gat.md) | run_node's prefetch comment claims a residency gate that was ablated away — the code is width-only | incoherence | low | `crates/bumbledb/src/exec/run/run_node.rs` |
| [113](./findings/113-encode-determinant-with-validates-utf-8-twice-per-.md) | encode_determinant_with validates UTF-8 twice per string key value on the point-read hot path | inelegance | low | `crates/bumbledb/src/api/db/get.rs` |
| [114](./findings/114-run-node-re-memsets-scratch-per-batch-the-exact-pa.md) | run_node re-memsets scratch per batch — the exact pattern its 'line-parallel twin' probe_pass documents as pure loss | incoherence | low | `crates/bumbledb/src/exec/run/run_node.rs` |
| [115](./findings/115-refine-measure-reconstructs-the-erased-view-varian.md) | refine_measure reconstructs the erased View variant from an emptiness sentinel | inelegance | low | `crates/bumbledb/src/image/view/apply.rs` |
| [116](./findings/116-executor-poison-state-is-three-flags-with-a-hand-o.md) | Executor poison state is three flags with a hand-ordered precedence branch instead of one sum | inelegance | low | `crates/bumbledb/src/exec/run.rs` |
| [117](./findings/117-plan-introspection-and-profiling-always-available-.md) | Plan introspection and profiling ('always available' EXPLAIN/ANALYZE) have no TS surface | missing-free-feature | low | `crates/bumbledb/src/api/db/snapshot.rs` |
| [118](./findings/118-arg-restriction-cannot-key-on-the-measure-longest-.md) | Arg-restriction cannot key on the measure: 'longest interval per group' is unspellable though every ingredient is paid for | missing-free-feature | low | `crates/bumbledb/src/ir.rs` |
| [119](./findings/119-interval-u64-and-interval-i64-are-one-impl-written.md) | Interval&lt;u64&gt; and Interval&lt;i64&gt; are one impl written twice; a sealed element trait erases the duplication | unification | low | `crates/bumbledb-theory/src/interval.rs` |
| [120](./findings/120-compile-fail-roster-covers-9-of-the-macro-s-16-spa.md) | Compile-fail roster covers 9 of the macro's ~16 spanned refusals | missing-free-feature | low | `crates/bumbledb-query/tests/compile_fail.rs` |
| [121](./findings/121-param-index-drops-its-span-so-the-param-mixing-dia.md) | Param::Index drops its span, so the param-mixing diagnostic lands at call_site | inelegance | low | `crates/bumbledb-query-macros/src/lib.rs` |
| [122](./findings/122-integer-suffix-policing-is-a-string-suffix-check-b.md) | Integer-suffix policing is a string suffix check: bare hex refused, suffixed hex accepted | incoherence | low | `crates/bumbledb-query-macros/src/lib.rs` |
| [123](./findings/123-three-divergent-integer-literal-parsers-in-one-gra.md) | Three divergent integer-literal parsers in one grammar: radix/underscores accepted in selections, rejected in widths and window bounds | unification | low | `crates/bumbledb-macros/src/lib.rs` |
| [124](./findings/124-fact-structs-miss-free-copy-eq-derives-their-gener.md) | Fact structs miss free Copy/Eq derives their generated key-struct siblings already have | missing-free-feature | low | `crates/bumbledb-macros/src/lib.rs` |
| [125](./findings/125-closed-relation-column-values-are-expansion-time-c.md) | Closed-relation column values are expansion-time constants but only readable through runtime queries | missing-free-feature | low | `crates/bumbledb-macros/src/lib.rs` |
| [126](./findings/126-the-one-owner-of-the-synthetic-id-law-is-actually-.md) | "THE one owner of the synthetic-id law" is actually owned in five places; Resolver has three near-identical sealed-shape scans | unification | low | `crates/bumbledb-theory/src/schema.rs` |
| [127](./findings/127-resolver-side-gates-its-return-on-global-issue-sta.md) | Resolver::side gates its return on GLOBAL issue state — a behaviorally inert branch that a placeholder representation erases | inappropriate-branching | low | `crates/bumbledb-theory/src/schema/spec.rs` |
| [128](./findings/128-relationspec-splits-closedness-across-two-parallel.md) | RelationSpec splits closedness across two parallel Options, representing two illegal states its own doctrine says should be unrepresentable | incoherence | low | `crates/bumbledb-theory/src/schema/spec.rs` |
| [129](./findings/129-the-rust-text-notation-cannot-spell-the-input-cond.md) | The Rust text notation cannot spell the input condition grammar's or()/and() trees the TS SDK ships — one condition language, two unequal surfaces | unification | low | `crates/bumbledb-query-macros/src/lib.rs` |
| [130](./findings/130-recordof-copies-every-fact-object-on-the-hot-write.md) | recordOf copies every fact object on the hot write/read paths | perf | low | `ts/src/marshal.ts` |
| [131](./findings/131-membership-array-literals-are-re-verified-and-re-t.md) | Membership-array literals are re-verified and re-translated through the roster on every execute | perf | low | `ts/src/query/run.ts` |
| [132](./findings/132-ordinal-alignment-law-re-verified-on-every-prepare.md) | Ordinal-alignment law re-verified on every prepare instead of once at open | unification | low | `ts/src/db.ts` |
| [133](./findings/133-exhume-scan-s-row-descriptor-pairing-checks-shortf.md) | exhume scan's row/descriptor pairing checks shortfall but silently drops extra cells | incoherence | low | `ts/src/exhume.ts` |
| [134](./findings/134-isstatementvalue-s-no-fact-cell-ever-has-data-kind.md) | isStatementValue's 'no fact cell ever has data.kind' claim has an interval-excess-property hole | bug | low | `ts/src/db.ts` |
| [135](./findings/135-write-double-wraps-the-begin-failure-with-a-near-d.md) | write() double-wraps the begin failure with a near-duplicate context | inelegance | low | `ts/src/db.ts` |
| [136](./findings/136-a-handle-or-field-literally-named-proto-is-silentl.md) | A handle or field literally named __proto__ is silently dropped by the input object literal while the type tier admits it | bug | low | `ts/src/closed.ts` |
| [137](./findings/137-exactly-0n-s-runtime-refusal-names-the-wrong-canon.md) | exactly(0n)'s runtime refusal names the wrong canonical shape (`{0..0}` instead of `{0}`) | incoherence | low | `ts/src/count.ts` |
| [138](./findings/138-outbound-strings-use-from-utf8-lossy-silent-corrup.md) | Outbound strings use from_utf8_lossy — silent corruption where the inbound twin refuses typed | incoherence | low | `ts/crate/src/marshal.rs` |
| [139](./findings/139-bridge-crate-self-reports-0-1-0-while-the-shipped-.md) | Bridge crate self-reports 0.1.0 while the shipped package is 0.6.0 — lockstep gate misses Cargo.toml | incoherence | low | `ts/crate/Cargo.toml` |
| [140](./findings/140-platform-allowlist-spelled-twice-as-branches-one-t.md) | Platform allowlist spelled twice as branches — one table would erase both | inappropriate-branching | low | `ts/scripts/platform.ts` |
| [141](./findings/141-20-query-ir-claims-provably-disjoint-rules-elide-t.md) | 20-query-ir claims provably disjoint rules elide the spanning seen-set; 40-execution and Dedup.lean record the opposite | incoherence | low | `docs/architecture/20-query-ir.md` |
| [142](./findings/142-the-local-twin-copies-facts-per-sec-and-the-ghzsta.md) | The 'local twin' copies: facts_per_sec and the GhzStamp-to-GhzReport conversion each exist twice; Rotation is re-hand-rolled at three SQLite call sites | unification | low | `crates/bumbledb-bench/src/lanes/writes.rs` |
| [143](./findings/143-verdictof-discards-judgeb-s-proved-citation-list-a.md) | verdictOf discards judgeB's proved citation list and re-derives it with duplicated filter predicates | unification | low | `lean/Main.lean` |
| [144](./findings/144-stale-lean-theorem-citation-txn-judgeb-agrees-of-d.md) | Stale Lean theorem citation `Txn.judgeB_agrees_of_declared` in the corpus README — outside every census battery | incoherence | low | `lean/conformance/README.md` |
| [145](./findings/145-corpus-readme-counts-24-hand-judgment-cases-the-ro.md) | Corpus README counts 24 hand judgment cases; the roster and the checked-in corpus carry 26 | incoherence | low | `lean/conformance/README.md` |
| [146](./findings/146-neon-allen-kernels-trust-four-stream-length-equali.md) | NEON Allen kernels trust four-stream length equality on debug_asserts only, unlike every sibling kernel's release asserts | incoherence | low | `crates/bumbledb/src/exec/kernel/neon.rs` |
| [147](./findings/147-batchtoken-catches-force-staleness-by-representati.md) | BatchToken catches force-staleness by representation but reset-staleness only by luck | missing-free-feature | low | `crates/bumbledb/src/exec/colt/iter.rs` |
| [148](./findings/148-fingerprint-lock-tempdir-uses-a-fixed-path-concurr.md) | fingerprint_lock TempDir uses a fixed path — concurrent test runs race remove_dir_all against a live locked store | bug | low | `ts/crate/src/fingerprint_lock.rs` |
| [149](./findings/149-db-open-convicts-a-half-created-store-as-corruptio.md) | Db::open convicts a half-created store as Corruption(MetaMissing) — the state Db::create names, tolerates, and the ephemeral probe classifies correctly | incoherence | low | `crates/bumbledb/src/storage/env/open.rs` |
| [150](./findings/150-exhume-the-read-only-archival-lane-opens-the-envir.md) | exhume — the read-only, archival lane — opens the environment read-write and takes a write transaction, so it cannot read a store on read-only media | missing-free-feature | low | `crates/bumbledb/src/storage/env/exhume.rs` |
| [151](./findings/151-an-ephemeral-store-that-crossed-a-machine-crash-re.md) | An ephemeral store that crossed a machine crash reopens as 'verified' — the kind's own loss claim is undetectable on disk | incoherence | low | `crates/bumbledb/src/storage/env/ephemeral.rs` |
| [152](./findings/152-read-u64-read-u32-read-fingerprint-conflate-presen.md) | read_u64/read_u32/read_fingerprint conflate present-but-mis-sized meta values with absent keys as MetaMissing, contradicting the taxonomy read_store_kind pins | incoherence | low | `crates/bumbledb/src/storage/env/read_meta.rs` |
| [153](./findings/153-check-asm-s-flag-free-gate-greps-for-cmp-csel-adds.md) | check-asm's flag-free gate greps for cmp/csel/adds/ccmp/bl but not subs/tst/cmn/ccmn/ands/fcmp — flag writers slip through | bug | low | `scripts/check-asm.sh` |
| [154](./findings/154-no-restore-keys-on-any-of-the-six-actions-cache-st.md) | No restore-keys on any of the six actions/cache steps — every Cargo.lock or toolchain repin forces fully cold CI builds | perf | low | `.github/workflows/ci.yml` |
| [155](./findings/155-architecture-readme-still-lists-shipped-70-api-led.md) | Architecture README still lists shipped 70-api ledger rows as OPEN sub-items | incoherence | low | `docs/architecture/README.md` |
| [156](./findings/156-root-readme-says-the-cookbook-holds-twenty-nine-wo.md) | Root README says the cookbook holds twenty-nine worked schemas; it holds thirty | incoherence | low | `README.md` |
| [157](./findings/157-00-product-success-criterion-3-says-the-alloc-gate.md) | 00-product success criterion 3 says the alloc gate becomes a CI gate 'when CI exists' — CI exists and runs it | incoherence | low | `docs/architecture/00-product.md` |
| [158](./findings/158-the-root-readme-s-rust-quickstart-is-the-one-unpin.md) | The root README's Rust quickstart is the one unpinned code surface in a doc estate built on fence pins | missing-free-feature | low | `README.md` |
| [159](./findings/159-answers-fixpointbudget-is-unreachable-and-engine-p.md) | Answers::FixpointBudget is unreachable and engine_program panics where the doc promises a divergence | incoherence | low | `crates/bumbledb-bench/src/differential.rs` |
| [160](./findings/160-naivedb-apply-stages-every-delta-twice-two-full-da.md) | NaiveDb::apply stages every delta twice — two full-database clones per write op | perf | low | `crates/bumbledb-bench/src/naive.rs` |
| [161](./findings/161-lean-sh-gate-comment-pins-the-corpus-at-217-cases-.md) | lean.sh gate comment pins the corpus at 217 cases; 272 are on disk | bench-honesty | low | `scripts/lean.sh` |

## Plausible findings

| # | Title | Category | Severity | Primary file |
|---|-------|----------|----------|--------------|
| [001](./findings/001-multi-rule-nullary-count-degenerates-to-a-constant.md) | Multi-rule nullary Count degenerates to a constant; Lean glue was edited to match, no theorem covers it | lean-rust-drift | critical | `lean/Bumbledb/Conformance.lean` |
| [088](./findings/088-pump-s-tail-drain-fires-on-every-mid-stream-recurs.md) | pump's tail drain fires on every mid-stream recursion, collapsing batch means at nodes ≥2 in deep plans | perf | medium | `crates/bumbledb/src/exec/run/pump.rs` |
| [089](./findings/089-ring-closing-joins-use-min-single-column-fanout-ig.md) | Ring-closing joins use min single-column fanout, ignoring conjunctive multi-variable constraints | perf | medium | `crates/bumbledb/src/plan/planner/estimate.rs` |
| [162](./findings/162-nested-start-capture-silently-discards-all-previou.md) | Nested start_capture silently discards all previously recorded events in release builds | bug | low | `crates/bumbledb/src/obs.rs` |

## Refuted claims

- **spawn_tx leaks tx_open=true if thread::spawn panics — permanent write lockout on the Db handle** — the cited arming order is real, but the spawn-failure path does not produce the claimed permanent lockout.
- **alloc_counter neighbors: counters drift on failed alloc/realloc, and the un-forwarded alloc_zeroed changes what the feature build measures** — the code shape is real, but neither mechanism reaches any number the module or bench actually produces (no fallible-alloc callers exist in the workspace).

## Coverage

Finder missions run:

- `engine:kernel`
- `engine:run`
- `engine:colt`
- `engine:storage`
- `engine:encoding`
- `engine:plan-ir`
- `engine:schema-api`
- `engine:interval-allen`
- `query:crates`
- `macros:core`
- `theory`
- `ts:core`
- `ts:types`
- `ts:bridge`
- `lean:schema-values`
- `lean:query`
- `lean:txn-oracle`
- `bench:honesty`
- `perf:rings`
- `perf:points`
- `perf:olap-temporal`
- `cross:branching`
- `cross:free-features`
