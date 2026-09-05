# Proof bridge ledger — successor guarantees, models, and empirical gaps

P11 deliverable (chapter 13 §7). One row per advertised invariant:

```text
guarantee -> exact model statement -> explicit premises
          -> concrete construction/transition site
          -> independent fixture/model test + platform/runtime gate
          -> known unsupported conditions and evidence revision
```

Term-level Lean references live in `Bumbledb/Bridge.lean` (`Obligation.row`
carries the theorem itself, so a deleted theorem fails `lake build`). This
file is the evidence-and-gaps companion: it names what is PROVED, what is a
correspondence test, and which FPU/native obligations are EMPIRICAL.

Status: the KERNEL half ran in F3 (2026-09-05): `lake build` green,
battery 1/2 escape-token greps clean, `lake exe conformance
conformance/cases` green on the checked-in corpus (277 cases, 0
disagreements). The bench/differential/platform gates named below remain
**NotRun** in this campaign. Evidence revision: branch
`codex/bumbledb-1-0`, F3 Lean-repair working tree. F3 statement changes
(P11.md F3 note has the full record): `inline_runStage` is spelled with
the typed consumer stage `⟨[k], fun tables => consume (tables.headD [])⟩`
(the F1 spelling was ill-typed; contract unchanged — value AND error
paths preserved), and Capacity's added helper is renamed
`natSum_map_le_length_mul` (collision with the pre-existing Aggregates
lemma; statement unchanged, `natSum_widened_bound` untouched).

## Retired premises (removed, not relabeled)

| Retired | Disposition |
| --- | --- |
| `Txn/Fresh.lean` (mint high-water model, `never_reissue_observable`, `resupply_legal_monotone`, `materialized_key_ordinary`) | DELETED with its mechanism: no reserve/FreshRef/issuance authority exists in the successor; entity identity is an application-owned `Bytes<16>` sealed before submission (chapter 10 §5). The key-uniqueness content survives as the ordinary functionality judgment, already carried by `functionality_unique_witness`. Gate: `E-NO-RESERVE`; bench `naive/successor/admission.rs` keeps the duplicate-key judgment tests without any mint. |
| `Txn/Braids.lean` (`ComponentClosed`, L9 component locality, L10 replay idempotence) | DELETED. ASS-001: the `ComponentClosed` premise spanned closed relation targets the Rust support derivation never consulted, and the braid theorems were cited as publication/causality evidence they never established. Successor: `Txn/Support.lean` (below). Replay-forward healing (L10) is not a successor recovery premise: the log replays exact immutable decisions (chapter 20), not batches against arbitrary states. |

## Semantic engine

| Guarantee | Model statement | Premises | Construction site | Independent fixture/gate | Known gaps |
| --- | --- | --- | --- | --- | --- |
| Scoped admission work: statements unaffected by a delta outside their mutable consulted support keep their verdicts | `Txn.Support.judgment_stable_outside_mutable_support`, `holds_stable_outside_mutable_support` | Delta touches no relation in `mutableRels T st` (consulted minus closed); closed extensions are theory constants | Successor admission planner support derivation (`storage/commit/plan.rs` lineage) | `judgment_stable_under_untouched_relations` (`crates/bumbledb-bench/src/naive/successor/admission.rs`); gates G03, CONC-05, P-SEMANTIC | The theorem licenses scoped admission/planning ONLY — no publication lane, causal cut, or read-visibility claim. The Rust support derivation must be tested against `mutableRels` per accepted statement form once P01/P03 land their planners (correspondence test, not proved). |
| Shared closed vocabulary does not merge mutable components | `Txn.Support.disjoint_mutable_locality`, `closed_not_mutable` | Disjoint mutable supports; locality of the delta | Same | `shared_closed_vocabulary_does_not_merge_supports` (same file) | Same correspondence obligation. |
| One-command normalization/tie rule | `Txn.Support.normalize_applyTo`, `normalize_idempotent`, `add_wins`, `add_present_noop`, `remove_absent_noop` | None (pure set algebra) | `WriteDelta` coalescing (`storage/delta.rs` lineage); every generated/dynamic/wire/scratch/replay path must lower to this one normal form | `same_command_tie_rule_add_wins` (bench admission model); gate `E-DELTA` | That ALL ingestion paths use the one normal form is an engine-roster obligation (E-DELTA permutation matrix), not provable here. |
| Raw-delta commutation, at its real strength | `Txn.Support.applyTo_comm_of_disjoint` | No cross add/remove conflicts | Internal reordering only | `raw_commutation_does_not_commute_admission` (bench: shows disjoint deltas still interacting through a capacity law); gate CONC-01/CONC-02 | Deliberately NOT sufficient for admission outcomes, exact-state witnesses, receipts. No public commutativity surface may spend it. |
| Delta-restricted incremental judgment soundness | `Txn.delta_restricted_commit_sound` (retained; premises re-audited this campaign) | Pre-state holds every statement (`State.models`) | `storage/commit/judgment.rs::judge` | `capacity_verdicts_agree_with_the_model` and the differential estate; `Db::verify_store` owns the missing-premise class | Unchanged from prior audit; countermodel `incremental_verdict_needs_holds` retained. |
| Grouped exact measures: empty-parent vacuity, empty-child zero, zero-weight distinction, widened fold | `capacity_of_empty_parent`, `capacity_zero_star`, `zero_weight_membership_distinct`, `natSum_widened_bound` (`Capacity.lean`) | ℕ weights (nonnegative by type); count ≤ 2^64 for the widened bound | Capacity checker walk (`storage/commit/judgment.rs::check_capacity` lineage) | `E-ADMIT` family; bench `naive` capacity walk (`capacity_violated`) is the independent judge | Alias normalization (equal-bound range ↔ exact window, unit-existence forms) is P01 implementation + `E-ADMIT` tests; no Lean statement of the normalization table exists yet — NAMED GAP: the alias table must be pinned by fixtures before F3, and any cross-family rewrite needs its own denotation-preservation evidence. |

## Floats (chapter 11)

| Guarantee | Model statement | Premises | Construction site | Independent fixture/gate | Known gaps |
| --- | --- | --- | --- | --- | --- |
| Canonical binary64 quotient (one zero, one NaN) | `F64.canonical_normalize`, `normalize_idempotent`, `parse_payload` (`Float64.lean`) | Integer bit arithmetic only | Value constructors/wire parser (P01) | `F-CANON`/`F-GOLDEN`; bench oracle `verify/f64_oracle.rs` canonicalization tests | Rust constructor/parser correspondence is empirical (`F-CANON` runs both against shared bit corpora). |
| Total order = physical key order | `orderKey_lt_iff`, `orderKey_le_iff`, `numericWord_injective` (`Float64/Order.lean`) | Canonical payloads | Order-key encoders, indexes | `F-ORDER`; oracle order tests | Index byte order on disk equals `orderKey` big-endian bytes: empirical roundtrip (`F-ORDER`), not modeled. |
| Exact sum/mean accumulator algebra | `NumTotal.merge_assoc/merge_comm/zero_merge/merge_finite_inv`; `Acc.merge_*`; `fold_perm`, `fold_append`; special-case table `fold_nan_of_mem`, `fold_mixed_inf_nan`, `fold_posInf_only`, `fold_all_finite` (`Float64/Sum.lean`) | Deduplicated input (distinct binding set) | Aggregate sink accumulator (P03) | `F-AGG`; bench `exact_sum_matches_rational_oracle`, `sum_is_permutation_and_partition_independent` | The engine's limb representation (34×u64) must be bit-compared against the model's `Int` totals: correspondence test, not proof. |
| Merge non-idempotence → dedup before accumulation | `merge_not_idempotent` | — | Sink dedup/`DistinctWitness` | `F-AGG` negative fixture `partial_state_replay_is_not_idempotent` | The engine's distinct-witness elision licence must be requalified against the successor sink (P03/P12). |
| 34-limb sufficiency | `fold_total_bound`, `accumulator_within_34_limbs` | count ≤ 2^64 | Accumulator sizing constant | Bench overflow-boundary tests | `CardinalityOverflow` refusal at the exact ceiling is engine behavior; model quantifies below the ceiling only. |
| One final ties-to-even rounding; mean divides exact rational | `roundPosRat`/`roundRatBits` (executable spec), kernel `#guard` goldens, `sum_max_max_overflows`, `mean_replicated_equiv` | d ≥ 1 | Sum/mean finalizers (P03) | `F-ARITH` differential: independent bench rational oracle vs engine vs these kernel goldens | **NAMED GAP (empirical + proof):** no Lean theorem yet states `roundPosRat` is nearest/ties-even for ALL inputs (goldens + executable spec only); the bench oracle carries the all-input check differentially, and hardware/IEEE correspondence on qualified targets is `F-ARITH`/`F-CROSS` evidence, never a theorem. |
| Host FPU environment control | — (deliberately unmodeled) | — | Numerical execution guard (P01/P03) | `F-ENV`: forced rounding modes/FTZ/DAZ before entry; restore on all exits | **EMPIRICAL ONLY**: no formal statement exists or is claimed; unsupported numeric environments must fail platform qualification. Uncoordinated signal handlers/foreign mutation mid-operation are outside the safe embedding contract. |
| Scalar rounded arithmetic (+,-,*,/,neg, casts) | — (independent oracle, not Lean) | — | Guarded hardware ops | `F-ARITH`: bench `verify/f64_oracle.rs` implements correctly-rounded reference arithmetic from integer/rational primitives; engine compared bitwise | **EMPIRICAL**: the bridge from spec to hardware instruction behavior is differential/architectural evidence across darwin-arm64/linux-arm64/linux-x64 (`F-CROSS`), by design. |

## Float intervals (dense denotation)

| Guarantee | Model statement | Premises | Construction site | Independent fixture/gate | Known gaps |
| --- | --- | --- | --- | --- | --- |
| Checked constructors: NaN refused, strict numeric order, infinity placement derived, `[-0,+0)` refused | `FInterval` structure invariants; `start_not_posInf`, `stop_not_negInf`, `zero_zero_refuses` | — | Interval constructors/wire parser (P01) | `F-INTERVAL`; bench `verify/finterval_oracle.rs` | Wire rejection of noncanonical endpoint bits is parser work (`F-CANON` analog), tested not proved. |
| Dense nonempty denotation; `[-Inf, -MAX_FINITE)` nonempty | `FInterval.nonempty`, `negInfRay_witness` | — | Same | `neg_inf_to_neg_max_ray_is_nonempty` | — |
| Membership: exact finite embedding, nonfinite probes false, physical order-key execution agrees | `containsF64_iff_orderKey`, `nonfinite_probe_false`, `neg_inf_probe_needs_guard` | Strictly finite probe for the bridge | Membership kernels/indexes (P01/P03) | `order_key_execution_matches_dense_membership`, `nonfinite_probes_return_false` | The engine MUST guard nonfinite probes before key comparison (`neg_inf_probe_needs_guard` is the countermodel); this guard's presence in the shipped kernels is a test obligation. |
| Adjacency coalesces; representable-neighbor gaps never coalesce | `FInterval.join_points`, `gap_uncovered`, `adjacent_endpoint_gap` | Shared endpoint / strict gap | Pack/sweep kernels | `adjacent_coalesces_and_representable_gap_does_not`; existing generic sweep proofs apply via `instance : LinearElem F64` | SIMD sweep bit-parity with the scalar kernel is `F-INTERVAL`/`Q-TEMPORAL` empirical evidence. |
| Length: one rounding; unbounded vs overflow distinct | `FInterval.measure`, `exactLength_pos`, `#guard` goldens (whole-finite-span overflow; unit spans) | Bounded interval for finite length | Length operator (P03) under the numerical guard | `F-INTERVAL` length fixtures | Same rounding-correctness gap as sum (shared `roundRatBits`). No `FixedInterval<F64>` exists to prove anything about — refusal is a schema-validation test. |

## Query composition and recursion

| Guarantee | Model statement | Premises | Construction site | Independent fixture/gate | Known gaps |
| --- | --- | --- | --- | --- | --- |
| Stage reads only declared inputs; later stages cannot rewrite history | `Stages.runStage_congr`, `evalFrom_stable` | Positional stage functions (by construction) | Successor stage IR/evaluator (P03, C05) | `Q-IR`; bench `naive/successor/staged.rs` independent staged evaluator | The REAL IR's stages must be shown to be functions of their declared reads (validator obligation); model bakes it in by construction. |
| Producer errors surface through consumers | `Stages.consumer_of_error`, `runStage_error_of_read` | Consumer reads the producer | Stage error propagation (P03) | `consumer_filter_cannot_hide_producer_error`; `Q-GROUP`/`F-OPT-NEG` error-hiding fixtures | Optimizer rewrites must carry value AND error equivalence witnesses; only the model-level law is proved. |
| Names don't force materialization; unreferenced stages invisible | `Stages.unread_stage_invisible`, `evalFrom_agree_except` | No later stage reads the slot | Inline/stream/materialize planner choices | `naming_does_not_force_materialization` | — |
| Inlining preserves values and errors | `Stages.inline_runStage` | Consumer reads exactly the produced table | Fusion rewrites | `inline_and_materialized_stages_agree` | Streaming fusion with batching/spill is mechanism; `Q-FALLBACK` compares all paths empirically. |
| Restricted recursion stays in the frozen finite domain (aggregate/computed predecessors admitted) | `Stages.iterate_subset_dom`, `iterate_grows`; countermodel `value_creation_escapes` | Base ⊆ dom; step selects only dom values on dom-contained states | Recursive node validation + semi-naive driver (P03) | `frozen_computed_predecessors_stay_in_domain`, `value_creation_feedback_is_refused`; `Q-RECUR` | Termination/round bounds carried by the retained `Exec/Reach.lean` lfp lemmas for the concrete IR plus engine budgets; the model proves containment induction only. The dependency check "no computed node depending on recursion feeds back" is validator work. |
| Retained rule-level semantics (matching, DNF, membership lowering, aggregates over distinct bindings, empty-group rule, pack/Allen, semi-naive) | Existing `Query/*`, `Exec/*` theorems, unchanged | As before | As before | Existing conformance corpus (277 cases) + successor regeneration | Corpus regeneration for successor forms is DEFERRED to F3 (executes a generator): see `crates/bumbledb-bench/src/corpus_gen/` notes; the old corpus does not prove new derived-stage or mutable-support contracts. |

## History (independent model — ASS-002)

| Guarantee | Model statement | Premises | Construction site | Independent fixture/gate | Known gaps |
| --- | --- | --- | --- | --- | --- |
| One authority order, at-most-once named execution, certainty, receipts, epochs, ExactState/ABA, Frozen/Deleted, roots/GC barrier | Independent executable Rust model `crates/bumbledb-bench/src/closure/history_model.rs` (NOT Lean: the protocol's value is adversarial schedule coverage; a small executable model feeds exhaustive/randomized traces) | Honest authenticated writers; linearizable conditional replacement; hash collision resistance for content refs | `bumbledb-log` internal machine (P04/P05) | `PROTO-01..-20` schedules via `closure/history_model.rs` trace enumerator + trace checker; G07 | **The model does not call production transition helpers** (compile-time: no `bumbledb-log` dependency exists in the bench crate). Real S3 conditional semantics, filesystem durability, and power loss are `S3-*`/`FS-*` gates — empirical by definition. The production-machine-vs-model differential requires P04's machine to exist (F2 integration) and executes in F3. |

## Assurance-lane routing (P-\*)

- `P-KERNEL`: `lake build` of this tree + axiom audit. No proof-escape token
  and no new `axiom` declaration exists in `lean/` (checked by the battery
  1/2 greps of `scripts/lean.sh` in F3; the only `decide`-style kernel
  computations are the goldens named above; **no `native_decide` anywhere**
  — no compiler trust extension).
- `P-SEMANTIC`: bench `naive/` + `naive/successor/` staged/admission models vs
  engine (F3).
- `P-FLOAT`: `verify/f64_oracle.rs` + `Float64/Sum.lean` goldens + F-* roster.
- `P-REPRESENTATION`/`P-DISK`/`P-MEMORY`/`P-SCHEDULE`/`P-ARTIFACT`/`P-PERF`:
  owned by P01/P02/P12/P13/P14 lanes per chapter 62 routing; P11 supplies the
  models/fixtures named above and reviews premises.

Preserved counterexamples: nothing in `Countermodels.lean` was deleted; the
retired braid/fresh theorems were deleted with their mechanisms, and their
audit trail remains in `audit/` and this ledger.
