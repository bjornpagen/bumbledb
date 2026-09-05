# Correspondence cases — current Rust, independent oracles

Authored by L19. Verification **NotRun**. These are expected results and
counterexamples for L02/L05/L08/L20; they are not a second engine and
not a production-planner twin. Lean theorems name hypotheses only.
Substrate (LMDB, S3, native lifetimes, host FP control) is unmodeled.

A case fails if the named current constructor disagrees with the
independent oracle, or if a deleted filename/symbol count is treated as
evidence.

## C4 / D26 — complete vs incremental vs unready

| Id | Input | Independent expected | Current constructor | Negative |
| --- | --- | --- | --- | --- |
| `C-D26-collision-empty-delta` | Two distinct canonical rows sharing a declared scalar key; terminal readiness with no further changes | `judge_complete` / `judge_final_state` **Reject**; destination absent | `judge_complete` (`schema/judge.rs`); store `SchemaJudge::judge` must call that entry | `judge_incremental(LawfulParent, empty delta)` **Accepts** under a minted parent — staging must not call it. `UnreadyStore` cannot mint `LawfulParent`. Lean: `incremental_verdict_needs_holds`. Corpus: `lean/conformance/cases/complete-key-collision.json` |
| `C-D26-containment-capacity` | Populated stage violating containment or a capacity floor/ceiling; empty final delta | Complete **Reject** | `judge_complete` | Empty-`ChangeSet` incremental skip |
| `C-D26-nonempty-required` | Schema whose empty state violates a required-parent law; then valid rows across batches | Empty complete **Reject**; filled complete **Accept** | `judge_complete` on the populated final state | Incremental batches must not mint readiness; only final complete judgment does |
| `C-D26-unready-cannot-mint` | Any `UnreadyStore` / populated stage | No `LawfulParent` value exists | Readiness calls `judge_populated` → `SchemaJudge::judge_complete` and does not mint. `LawfulParent::established` is crate-private; only complete admission, trusted open, or a prior admitted commit may mint | Public constructor, `disarm` to `(Store, Path)`, or reverting readiness to `prepare(empty)` |

Premise map: incremental soundness is `Txn.delta_restricted_commit_sound`
and spends `State.models`. The owner that establishes that premise is
`LawfulParent`, not an unready directory and not an empty `ChangeSet`.

## D04 — collision equality and compiled locality

| Id | Input | Independent expected | Current constructor | Negative |
| --- | --- | --- | --- | --- |
| `C-D04-agree-three-judges` | Lawful parent + a conflicting add (same email, new id) among many unrelated groups | `judge_complete` = `judge_incremental(LawfulParent, …)` = `judge_final_state` on verdict and cited statement ids | Those three entries; incremental may use `delta_local_statements` | Sharing the production planner as the oracle; counting visits instead of comparing the independent stream |
| `C-D04-collision-bytes` | Forced fingerprint/routing collision of two unequal canonical rows | Distinct facts; neither merge nor wrong delete | `value_eq_iff_encode_eq` ↔ `encode_fact` / `fact_sort_key`; bench `collision_pair_judgment_is_exact_bytes_not_fingerprints` | Treating blake3/LMDB key equality as logical identity |
| `C-D04-citations-topk` | More offenders than the citation budget, opposite insertion order | Same statement ids; examples are canonical-byte top-k **before** truncation | `fact_sort_key` + `CitationTopK`; L02 `d05_rejection_evidence_is_portable` | First-seen row ids, then a cosmetic sort |

## D05 — portable rejection

| Id | Input | Independent expected | Current constructor | Negative |
| --- | --- | --- | --- | --- |
| `C-D05-remint-spill` | Same logical rejected command, reminted ids, resident vs forced scratch | Equal `encode_judged` bytes, cited facts, truncation flags | `encode_judged`; `judge_complete` | Golden files keyed on insertion order |

## D19 / G04 — numeric quotient, exact folds, restricted recursion

| Id | Input | Independent expected | Current constructor | Negative |
| --- | --- | --- | --- | --- |
| `C-D19-cancel` | `{1e16, 1, -1e16}` distinct bindings | Canonical sum bits from the rational oracle, not epsilon | `Float64/Sum.lean` fold; bench `exact_sum_matches_rational_oracle` | Host-rounded `f64` add chain |
| `C-D19-mean-once` | Mean of two `MAX_FINITE` | Once-rounded exact mean = `MAX_FINITE`; sum overflows | `mean_divides_exact_rational_not_rounded_sum` | `rounded_sum / count` |
| `C-D19-merge-not-idemp` | Merge one finite partial with itself | Doubled total and count | `merge_not_idempotent` | Replaying a spill partition without dedup |
| `C-G04-error-surfaces` | Producer overflow, consumer filter | Consumer errors | Independent staged evaluator `naive/successor/staged.rs` | Production fusion planner as oracle |
| `C-G04-frozen-domain` | Linear recursion over frozen computed/aggregate predecessors | Iteration ⊆ frozen domain | `iterate_subset_dom`; `value_creation_feedback_is_refused` | Arithmetic/aggregation in the cycle |

## G03 / G07 — admission support and authority order

| Id | Input | Independent expected | Current constructor | Negative |
| --- | --- | --- | --- | --- |
| `C-G03-mutable-support` | Delta outside one statement's mutable consulted relations | That statement's verdict unchanged | `mutableRels` / bench `mutable_support`; `delta_local_statements` | Relabeling retired `ComponentClosed` / `Txn/Braids.lean` as this license |
| `C-G03-add-wins` | Same exact fact on both sides of one `ChangeSet` | Present (add wins); parse refuses a second action for the row | `ChangeSet::parse` / `ChangeSetBuilder`; `normalize_applyTo` | Cross-command last-writer-wins |
| `C-G03-raw-commute` | Disjoint add/remove sets that still share a capacity parent | Set application may commute; **admission may not** | `applyTo_comm_of_disjoint` at set strength only | A public “commutative command” flag |
| `C-G07-authority` | Two writers, lost CAS, receipt retirement | Independent history model observations | `crates/bumbledb-bench/src/closure/history_model.rs` (no `bumbledb-log` dep) | Lean braid/component theorems certifying hosted publication |

## Substrate (explicitly unproved)

| Assumption | Owner | What Lean does not claim |
| --- | --- | --- |
| LMDB single-writer, pages, crash | L07 / G06 | Durability, map growth, CoW readers |
| S3/FS conditional replace, power loss | L11 / G08 | Transport observations; L08 interprets proof |
| Native image/token lifetimes | L04 / L12 | Generation ownership |
| Host FP rounding/FTZ/DAZ/traps | L01/L05 / `F-ENV` | IEEE instruction behavior under foreign control |

## Deliberate wrong-model sensitivity

A correspondence assertion must fail if: empty-delta incremental is
treated as complete validation; `UnreadyStore` mints `LawfulParent`;
citations are row-id order; braid theorems are cited as log
certification; or the production planner is the only oracle.
