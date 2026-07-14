# Spec-fidelity review 05 — Aggregates (covenant PRD 15, pairing #5)

Reviewer 5 of 10, blind. Normative side: `lean/Bumbledb/Query/Aggregates.lean`
(+ Bridge rows, `lean/Bumbledb/Bridge.lean:256-321`). Implementation side:
`crates/bumbledb/src/exec/sink/aggregate/{sink,fold_row,fold_batch,finalize,groups}.rs`,
`crates/bumbledb/src/interval/sweep.rs`, `crates/bumbledb/src/allen.rs`.
Zero code changes made.

Divergence classes: **(a)** observable semantic divergence (engine behavior
contradicts a theorem); **(b)** recorded narrowing/abstraction (the model
deliberately states less; the gap is documented on one side or both);
**(c)** representational (same semantics, different encoding or canonical
form).

## Per-theorem fidelity table

| Theorem (Aggregates.lean) | Engine site | Verdict |
|---|---|---|
| `checkedSum_sound` (L220-239) | `finalize.rs::finalize_acc` L181-192: `i64::try_from`/`u64::try_from` → `Error::Overflow(Aggregate{find})` | Faithful. Emitted Sum is the exact wide sum, overflow a typed error, never a wrap. See D2 (signed face). |
| `wide_accumulator_exact` (L272-288) | `fold_row.rs` L81-91 (`i128`/`u128` unchecked adds), `fold_batch.rs` L99-123, `sink.rs` L99-110/L147-154 (value×count in 128-bit) | Faithful. All accumulation paths are 128-bit and check-free; the once-per-group narrowing at finalize is the model's single poisoning point. Worst cases (2⁶⁴−1 terms of ±2⁶³/2⁶⁴−1) fit i128/u128, matching the theorem's arithmetic. |
| `agg_over_distinct_bindings` (L1202-1218) | `fold_row.rs::fold_scratch_row` L36-45 (seen-set before any fold), `fold_batch.rs::fold_batch_dedup_constant_group` L33-67 | Faithful. Dedup precedes every accumulator touch; elision only under `distinct_witness` (PRD 07's licence, `sink.rs` L303-306). See D4 (first-vs-last occurrence). |
| `empty_global_no_answer` (L1235-1251) / the fibering `Group` (L1146-1162) | `groups.rs::probe_group` L49-87 (creation on first sight), `finalize.rs::finalize_into` L42 (iterates only the group map) | Faithful. Every `probe_group` call is dominated by an actual binding: `fold_scratch_row` folds after dedup, `fold_batch_constant_group` is gated on non-empty survivors (`sink.rs` L190-192, `fold_batch.rs` L63-65), and `end_scan` returns before probing when `count == 0` (`sink.rs` L128-131). No zero row is constructible. |
| `measure_fold_laws` / `measure_fold_poisons` (L1265-1307) | `fold_row.rs` L15-25 (ray test before everything; poisoned sink folds nothing), `run/execute.rs` L400-401 (typed `MeasureOfRay`) | Faithful iff-shape per group (`measure(start,end)` is `None` exactly at `end == u64::MAX`, `exec/sink.rs` L151-153); poisoning **altitude** diverges — see D1. |
| `pack_canonical`/`pack_extensional`/`pack_adjacency` (L490, L630, L665) | `finalize.rs` L37-41 (sort pass) + `interval/sweep.rs::sweep` L59-111 | Faithful. Gap law identical: Lean breaks on `f < iv.start` (coalesce L419), sweep breaks on `start > frontier` (L92); `start == frontier` continues in both — half-open adjacency coalesces. Frontier join is `maxE` vs `frontier.max(end)` (L104). See D6 (sort tie order). |
| `pack_lattice_closed` (L740-761) | `sweep.rs` L96/L104: `maximal(run_start, frontier)` — both words copied from input segments, never computed; `finalize.rs::PackEmit::maximal` L106-126 pushes them verbatim | Faithful. No endpoint arithmetic exists on the Pack path. |
| `classifyI` vs `classify_bounds` (L865-883 vs `allen.rs` L266-287) | case-for-case | **Identical.** All 7 outer cells (`eq/eq→equals`, `eq/lt→starts`, `eq/gt→startedBy`, `lt/eq→finishedBy`, `gt/eq→finishes`, `gt/lt→during`, `lt/gt→contains`) and both refined cells (`lt/lt` on `cmp a.end b.start → before/meets/overlaps`; `gt/gt` on `cmp b.end a.start → after/metBy/overlappedBy`) match by name. `cmp3` (L813-814) is `Ord::cmp`'s trichotomy. |
| `allen_jepd` (L987-989) | `allen.rs::classify` L255-259; sampled by `classify_matches_the_point_set_oracle_jepd` | Faithful; the theorem is proved generally over `LinearElem`, engine over `T: Ord` — same bound. |
| `allen_converse_involution`/`mask_converse_involution`/`classify_swap` (L1032, L1037, L1089-1091) | `allen.rs::Basic::converse` L69-86 (table = mirrored pairs), `AllenMask::converse` L203-205 (13-bit reversal), 8192-mask exhaustive test | Faithful. The Lean and Rust converse tables agree name-for-name on all 13 pairs. See D5 (constructor order vs bit order). |
| `argmax_ties_all_kept` (L1370-1379) | `fold_row.rs::fold_arg` L123-157 | Faithful. Worse keys return early; equal keys insert into the never-elided per-group row set (`arg_answers` is a `WordMap` — equal projected rows collapse, matching the `Set` carrier); strictly-better clears then inserts. Identity seeds (`u64::MIN`/`MAX`, `groups.rs` L130-135) make the first binding land via the equal arm — unobservable. Key comparison over encoded words is the recorded order-embedding narrowing (module doc L58-62). |
| Min/Max seeds (adversarial) | `groups.rs` L67-68, `sink.rs` (scan seeds L66-67) | Unobservable: a group only exists with ≥1 folded binding (above), and scan partials are identity-seeded and merged only when `count > 0` (`sink.rs` L128-131). `min(MAX, MAX) = MAX` keeps a genuine `u64::MAX` value honest. |
| Count vs CountDistinct domains (adversarial) | `fold_row.rs` L80 vs L102-113; `finalize.rs` L190 | Faithful. Count counts distinct bindings (post seen-set); CountDistinct counts the group's distinct value spans in a per-group `WordMap` that is *never* elided — exactly the Lean note "distinct bindings ⊇ distinct values" (L102-110 comment mirrors Aggregates.lean L69, L1207-1211). |

## Divergences

**D1 (b) — MeasureOfRay poisoning altitude: group in the model, query in the
engine.** `measure_fold_laws` poisons one group's measure column
(`Aggregates.lean` L1258-1276: `measureColumn v σs = none ↔ ∃ σ ∈ σs …`);
the engine poisons the whole sink — `fold_row.rs` L15-17 stops all folding
for every group, and `run/execute.rs` L400-401 turns the execution into the
typed `MeasureOfRay`, erasing ray-free groups' answers too. The model
records this deliberately ("this level has no effect to carry", module doc
L70-74) and never states a query-level answer set for the poisoned case, so
nothing provable is contradicted — but the model is strictly weaker than the
behavior.

**D2 (b) — `checkedSum` is the unsigned face only, checked per-add.**
`checkedSum` folds `Nat` against a limit per addition (L191-201); the engine
also has a signed face (`Acc::SumSigned(i128)`, `fold_row.rs` L81-85;
`finalize.rs` L183-185) with one final check. For `Nat` the two are
equivalent (monotone prefixes; `wide_accumulator_exact` makes intermediate
checks vacuous), but the signed path — including biased-word decode
`word_to_i64` (`exec/sink.rs` L142-144) — has no direct theorem; it rides on
the analogy stated in the `wide_accumulator_exact` bridge note (L272-278).

**D3 (b) — the fibering is single-rule; the union regime is delegated.**
`Group`/`bindingSet` fiber one rule's deriving assignments
(`Aggregates.lean` L1139-1148); the engine's multi-rule sink keys dedup on
the head projection across rules (`fold_row.rs::dedup_key` L167-182,
`sink.rs` L388-398). Recorded in the theorem's bridge note (L1206-1210) and
owned by PRD 07's `union_regime_head_projection` (Bridge.lean L372-375) —
in-scope here only as a boundary, honestly marked.

**D4 (c) — dedup canonical element: last vs first occurrence.** Lean `dedup`
keeps the last occurrence (L1169-1171, noted "only membership matters");
the engine's seen-set keeps the first (`fold_row.rs` L42-44).
`agg_over_distinct_bindings` quantifies over arbitrary folds, where list
order could matter; every engine fold is order-insensitive
(sum/min/max/count/sets — `Acc` doc, `exec/sink.rs` L271-273), so only
membership is spent. Benign, but the theorem is stated over a different
canonical list than the engine produces.

**D5 (c) — AllenRel constructor order ≠ the normative bit order.** Lean:
`before, meets, overlaps, finishedBy, contains, starts, equals, startedBy,
during, finishes, overlappedBy, metBy, after` (`Query/Syntax.lean` L66-69);
Rust bit order: `…, Starts=3, During=4, Finishes=5, Equals=6, FinishedBy=7,
Contains=8, StartedBy=9, …` (`allen.rs` L27-57) — positions 3↔5 and 7↔9
swapped. Both orders are palindromic, so both make converse a mirror; the
Lean mask is a name-keyed list ("the bit order being the encoding's
business", Aggregates.lean L1005-1007), so no theorem reads positions.
Harmless now; a hazard only for anyone equating constructor index with bit
index.

**D6 (c) — Pack sort: insertion sort on start vs `sort_unstable` on
`[start, end]`.** Lean sorts by start only (L307-315, kernel-evaluable by
recorded choice, L296-299); the engine sorts claim pairs lexicographically
(`finalize.rs` L38-40). Equal-start tie order differs, but coalesce is
tie-insensitive (any equal-start claim continues the open run since
`start ≤ frontier`), and every theorem reads only `Pairwise (start ≤)` —
which both orders satisfy.

## GRADE: A

No class (a) divergence found. Every theorem in the module maps to a real,
correctly-shaped engine mechanism, and the three substantive gaps (D1-D3)
are each explicitly recorded in the Lean module doc or bridge notes — the
covenant's "narrow and record" discipline held. The classifier is a
line-for-line port; the Pack production path is the one shared sweep with
the exact gap/adjacency/frontier laws; group creation-on-first-sight is
enforced on all four fold paths including the scan pushdown's empty-scan
early return; Arg ties and CountDistinct dedup match their theorems
adversarially (seeds unobservable, value-set never elided). The grade is A
rather than A+ because D1's model deliberately cannot state the engine's
query-level abort, and D2 leaves the signed accumulator proved only by
analogy — both recorded, neither refuted.
