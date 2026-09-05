# Proof bridge ledger — current constructors, hypotheses, gaps

Companion to `Bumbledb/Bridge.lean` (term-level theorem references)
and `lean/correspondence.md` (expected independent results).
Verification **NotRun**. Lean does not prove LMDB, S3, native
lifetimes, or host FP control by naming a file.

## Retired premises (removed, not relabeled)

| Retired | Disposition |
| --- | --- |
| `Txn/Fresh.lean` | Deleted with the mint machine. Identity is application-owned `Bytes<16>`. |
| `Txn/Braids.lean` (`ComponentClosed`, L9/L10) | Deleted. ASS-001: the premise spanned closed targets the runtime never consulted, and the theorems were cited as publication evidence they never established. Successor: `Txn/Support.lean` — scoped admission only. Current log authority is the independent history model, not braid locality. |

## Premise → constructor map (C4 / G03)

| Premise | Lean statement | Constructor / owner that establishes it | Must not establish it |
| --- | --- | --- | --- |
| Proposed final state, every statement | `Txn.judge` / `completeAdmission` | `judge_complete` (`schema/judge.rs`); store `SchemaJudge::judge` | Empty `ChangeSet` incremental skip |
| Parent already models the theory | `State.models` on `delta_restricted_commit_sound` | `LawfulParent` — crate-private `established`, minted by complete admission, trusted persisted open, or a prior admitted commit | `UnreadyStore`, public constructor, `disarm` |
| Incremental restricted checks | `deltaCheck` / `judge_incremental` | `judge_incremental(LawfulParent, …)` + `delta_local_statements` | Calling it on an unready populated stage |
| Streaming reference | same denotation as complete | `judge_final_state` (independent stream; not the production planner) | Sharing `delta_local_statements` as the oracle |
| Exact fact identity | `value_eq_iff_encode_eq` | `encode_fact` / `fact_sort_key` | Fingerprint or LMDB key equality |
| Citation examples | representation (not in Lean sets) | `fact_sort_key` then `CitationTopK` | First-seen row ids |
| One-command add-wins | `normalize_applyTo` | `ChangeSet` / `ChangeSet::parse` | Cross-command LWW |
| Mutable-support locality | `judgment_stable_outside_mutable_support` | `delta_local_statements` + bench `mutable_support` | Relabeled `ComponentClosed` |
| Authority order | (not Lean) | Independent `history_model.rs` | Braid theorems, production writer helpers |
| Staged readiness | `completeAdmission` | `judge_complete` on the populated unready state | `prepare(empty)` as full validation |

## Semantic engine

| Guarantee | Model statement | Premises | Construction site | Independent instrument | Known gaps |
| --- | --- | --- | --- | --- | --- |
| Scoped admission work | `judgment_stable_outside_mutable_support` | Delta touches no `mutableRels` | `delta_local_statements` | `judgment_stable_under_untouched_relations`; G03 | Licenses planning only — no publication/causal-cut claim |
| Shared closed vocabulary | `disjoint_mutable_locality` | Disjoint mutable supports | Same | `shared_closed_vocabulary_does_not_merge_supports` | Same |
| One-command tie rule | `normalize_applyTo`, `add_wins` | None (set algebra) | `ChangeSet` | `same_command_tie_rule_add_wins` | All ingestion paths using one normal form is an engine-roster obligation |
| Raw commutation | `applyTo_comm_of_disjoint` | No cross add/remove | Internal reorder only | `raw_commutation_does_not_commute_admission` | Not admission, witnesses, or receipts |
| Incremental soundness | `delta_restricted_commit_sound` | `State.models` = `LawfulParent` | `judge_incremental` | `d04_*`, `d26_*` discriminators | Countermodel `incremental_verdict_needs_holds`; `UnreadyStore` readiness currently calls `judge_populated` → `SchemaJudge::judge_complete` and must not revert to `prepare(empty)` |
| Complete / unready | `completeAdmission` | None (raw instance) | `judge_complete` | `complete-key-collision.json`; `d26_complete_judgment_cannot_borrow_a_lawful_parent` | Resource errors are not rejections |
| Exact measures | `capacity_*`, `natSum_widened_bound` | ℕ weights; count ≤ 2^64 | `capacity` / `capacity_delta_local` | `d04_capacity_*`; bench capacity walk | Alias-normalization table is fixtures, not Lean |

## Floats and intervals

Unchanged mathematically. Instruments now name
`f64_oracle/tests.rs` and `finterval_oracle.rs`. Host FP environment
is **EMPIRICAL ONLY** — no theorem. Limb representation vs `Int`
totals is correspondence, not proof. `roundPosRat` all-inputs
nearest-even remains a named gap (goldens + bench oracle).

## Query / recursion

Independent oracle is `naive/successor/staged.rs` and the Lean
conformance corpus — **not** the production planner. Restricted
recursion: `iterate_subset_dom`; countermodel `value_creation_escapes`.

## History (ASS-002)

Independent executable model
`crates/bumbledb-bench/src/closure/history_model.rs`. No
`bumbledb-log` dependency. Real S3/FS/power-loss are G08 empirical
gates. Lean braid theorems cannot certify this machine.

## Substrate assumptions (explicit)

| Substrate | Status |
| --- | --- |
| LMDB single-writer, pages, crash | Unmodeled (L07 / G06) |
| S3/FS conditional replace | Unmodeled (L11 / G08); L08 interprets proof |
| Native image/token lifetimes | Unmodeled (L04 / L12) |
| Host FP rounding/FTZ/DAZ/traps | Unmodeled (`F-ENV`) |
| Hash collision resistance | Empirical; Lean identity is canonical bytes |

## Assigned away from this census

| Former census check | Owner |
| --- | --- |
| Exact `dyn` line counts | Deleted (not a proof) |
| Wording bans / comment-hygiene tokens | Deleted (not a proof) |
| Log v3 surface pins / identity emitter / `spec-gen --check` | L08/L21 (wire goldens, not authority theorems) |

## Handoffs

See `lean/correspondence.md` for case ids `C-D26-*`, `C-D04-*`,
`C-D05-*`, `C-D19-*`, `C-G04-*`, `C-G07-*`. L21 permanent scope:
this ledger, Bridge, correspondence catalog, `scripts/lean.sh`,
`scripts/spec-census.sh`. Qualification runs those scripts; L19
verification is **NotRun**.
