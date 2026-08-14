# Plan/exec representation audit (below prepare)

Brooks: the tables make the flowcharts obvious. Pike: data dominates; algorithms follow. Applied to `crates/bumbledb/src/plan/` (fj internals, ground, planner, densify, selectivity internals) and `crates/bumbledb/src/exec/` except the prepared-query layer Wave 1 already covered (`api/prepared/*`).

The Program→Query cutover landed. Wave 1 froze the Query IR spine and the prepared pipeline sum (CONTRACT C1/C3). Below that cut, plan and exec still guard combinations a parse would have made unwritable: Option/bool products, proofs computed then discarded, dual pin vocabularies, Interior as the crash-else of EDB.

Known already — **not re-opened here:** fj/validate false "no Interior" claim (engine-030/011), selectivity floor alias (engine-018), `edb().is_none()` bind/floor dispatch (engine-017), `ground_program` (engine-034), wordmap/clear "non-recursive program" and Program vocabulary (engine-011/035), introspection display / unit-labels / delta-variant comments (engine-033/029/007). Those appear at the end as DUPLICATE stubs.

This-cut OPEN refusals, walls, C ABI essential C, naive-oracle full-lfp, and anything C1 keeps are **not** bugs.

---

## The shape that is wrong

Prepare already *has* several sums (`DedupRegime`, `SelectionLevel`, `KeyCount`, `LeafSource`, `ProjectionSources`, `GroupTable`, `Verdict3`). Construction then flattens them into independent Options and bools the hot path re-interprets. Aggregate Count is an `Option` hole on the same `Agg` product that folds require. Interior on a sealed plan is still `relation()`'s `unreachable!`. The fold evaluator parses a resolvable σ, stores a `Copy` count-plus-bool, and introspection parses it again.

The collapsing coordinate is the same as Wave 1: a trusted layer is a sum. Until the constructors keep what they parsed, the flowchart cannot get simpler than the table.

---

## Findings

### F1. `FindSpec::Agg` / `SinkSpec::Agg` keep Count as `over_slot: None`

- **Where:** `crates/bumbledb/src/exec/sink.rs:77-86,102-114`; `api/prepared/build.rs:1156-1181`; `exec/sink/aggregate/fold_row.rs:62-98`; `fold_batch.rs:132-157`; `aggregate/sink.rs:143-157`
- **What's wrong:** CONTRACT C6 already split SDK aggregate finds: Count carries no `over`; folds require it. The engine's trusted `FindSpec` (and the post-parse `SinkSpec`) still spell both as one product: `Agg { op: FoldOp, over_slot: Option<usize>, over_width, signed }`. `FoldOp::Count` with `Some(slot)`, `Sum`/`Min`/`Max` with `None`, and `Count` with `signed: true` are all representable. The IR even did the right split for *measures* (`FindTerm::AggregateMeasure { over: VarId }` is required) then put ordinary aggregates back into the Option hole. Every fold site then `over_slot.expect("validated: Sum has a variable")` — King: validation returns `()` and the sink re-checks.
- **Collapsing representation:** a sum at the trusted layer, mirroring C6.

```rust
enum AggSpec {
    Count,
    Fold { op: FoldOp /* Sum|Min|Max */, slot: usize, width: usize, signed: bool },
}
enum FindSpec { Var {..}, Duration {..}, Agg(AggSpec), AggDuration {..}, Pack {..} }
enum SinkSpec { Var {..}, Agg(AggSpec), Pack {..} }
```

The `expect` arms delete. `union_span` matches `Fold` vs `Count` instead of `over_slot: Some/None`.
- **Essential vs accidental:** nullary Count vs a fold-over-a-slot is essential. Encoding it as Option-on-the-same-struct is accidental, and C1's "every trusted layer is a sum" already forbade it — C1 keeps the *hostile* `FindTerm::Aggregate { over: Option }` on `ir.rs`, not `FindSpec`.
- **Severity:** high

### F2. `DedupRegime` is parsed at construction, then discarded into four independent fields

- **Where:** `crates/bumbledb/src/exec/sink/aggregate/new.rs:195-275,304-331,348-353`; `exec/sink.rs:328-408`
- **What's wrong:** Construction already has the sum (`DedupRegime::Bindings | Union | DnfUnion | Elided(DistinctWitness)`). `build` immediately flattens it into `distinct_witness: Option`, `seen: Option<WordMap>`, `union_spans: Option<Vec<_>>`, `dnf_rekey: bool`. Legal-looking combos: DNF rekey with no union spans, a witness *and* a seen-set, elision on a multi-rule sink. `aim` re-tests `self.dnf_rekey` beside `union_spans.as_mut()`. `distinct_seen` `debug_assert`s `seen.is_none() == distinct_witness.is_some()` — the pairing the type should have enforced. King: the constructor parsed the regime and threw the proof away.
- **Collapsing representation:** store the regime. `seen` lives only in `Bindings`/`Union`/`DnfUnion` arms; `distinct_witness` only in `Elided`; `union_spans` + the DNF-vs-head choice only in the two union arms (the spans' *provenance* is which arm, not a sidecar bool).

```rust
enum DedupState {
    Bindings { seen: WordMap<()> },
    Union { seen: WordMap<()>, spans: Vec<(usize, usize)> },      // head projection
    DnfUnion { seen: WordMap<()>, spans: Vec<(usize, usize)> },   // VarId-ordered slots
    Elided { witness: DistinctWitness },
}
```

`aim`'s `if self.dnf_rekey` becomes a match. The debug_assert deletes.
- **Essential vs accidental:** four dedup regimes are essential (R2). Four independent fields are accidental packaging.
- **Severity:** high

### F3. `FoldedMark` is a `Copy` count-plus-bool; introspection re-parses σ

- **Where:** `crates/bumbledb/src/plan/ground/evaluate.rs:60-64,146-200,217-230`; `ir/normalize.rs:79-92`; `exec/introspection/into_stats.rs:82-118`
- **What's wrong:** The evaluator `parse_resolvable`s the occurrence's filters, computes `S`, then stores `FoldedMark { ids: u16, negated: bool }` because "the fold mark remains `Copy`, so it cannot carry the parsed filter set." Polarity is a bool on Folded ("the `!` polarity the role no longer carries") — except it does, as a flag. Introspection then `parse_resolvable`s *again*, `debug_assert`s success, and `unwrap_or_default`s to empty handles on failure (release: silent empty picture). The id-set also exists as `WordSet` filters attached to sibling occurrences — three encodings of one σ (mark count, sibling WordSet, re-eval). King plus Minsky's three-boolean: Folded+not-negated / Folded+negated are two states stuffed into a product, and the proof of *which rows* was discarded.
- **Collapsing representation:** parse the fold into a mark that carries what evaluation learned.

```rust
enum FoldedMark {
    Positive { ids: u16, /* or the handle list, diagnostic-sized: n ≤ 256 */ },
    Negated  { ids: u16 },
}
```

Better: store the surviving id vec (already `u16`-capped) so introspection does not re-run σ. Sibling `WordSet` attachment stays the *execution* rewrite; the mark is the *record*. `into_stats` reads the mark, no `parse_resolvable` on the cold path, no `unwrap_or_default`.
- **Essential vs accidental:** folding a closed atom to a set (or its complement) is essential. `Copy` as the reason to discard the parse, and polarity as a bool, are accidental.
- **Severity:** med

### F4. Two `pinned_fields` functions; they disagree on sets

- **Where:** `crates/bumbledb/src/plan.rs:13-35`; `plan/fj/provably_disjoint.rs:117-126`
- **What's wrong:** `plan.rs::pinned_fields` is documented as "**the one** pinned-field vocabulary, shared by the distinctness witness and the DP's key-coverage translation so the two coverage predicates cannot diverge." It excludes `ParamSet`/`WordSet` (a set matches any element — two facts can differ on the field). `provably_disjoint.rs` has a *second* `fn pinned_fields` that yields every `Eq` compare, sets included, then hopes `provably_different` returns false for them. Dual encoding of "what is a pin," and the copies already diverge. If `provably_different` ever grows a set case, disjointness would fire on pins that distinctness says do not pin.
- **Collapsing representation:** one function returning `(FieldId, &Const)` for scalar Eq only (the `plan.rs` filter). Disjointness maps that. Delete the private copy.
- **Essential vs accidental:** "sets pin nothing" is essential (both docs agree). Two functions is accidental, and the disagreement is a defect the dual invited.
- **Severity:** med

### F5. `PlanOccurrence::relation()` panics on Interior

- **Where:** `crates/bumbledb/src/plan/fj.rs:218-237`; callers `exec/introspection/into_stats.rs:76,96-114` (`occurrence.relation()` for eliminated/folded)
- **What's wrong:** After the cut, an occurrence's source is `Edb | Interior`. The plan witness still offers a panicking EDB-only accessor: `unreachable!("caller asserted a stored-relation (Edb) occurrence")`. Folded/eliminated occurrences *happen* to be stored (grounding refuses Interior — evaluate.rs:136-141, 208-211), so `into_stats` is safe only because a different module's guard exists. The type does not say so. Fowler's type-code: Interior is still the crash case of "everything is a stored relation." (The `edb() else { floor/false }` forest in selectivity/densify/evaluate/provably_distinct is the same coordinate — already engine-017; this is the panicking helper that forest uses when it "knows" it is EDB.)
- **Collapsing representation:** delete `relation()`. Callers match `occurrence.source`. Folded/eliminated marks carry the `RelationId` they folded against (parse at ground time — F3), so introspection does not re-assert.
- **Essential vs accidental:** derived tables have no `U`/`M` store — essential. A panicking accessor on the sealed plan is accidental leftover of the EDB-only world.
- **Severity:** med

### F6. Sink scan/skip protocol is bools plus `unreachable!`

- **Where:** `crates/bumbledb/src/exec/run.rs:105-146`; `run/scan_table.rs:57-58`; `sink/projection/sink.rs:26-67,80`; `sink/aggregate/sink.rs:21-76`
- **What's wrong:** `begin_scan -> bool` (opened vs declined). `scan_run`/`end_scan` panic if the bool was false. `emit_batch(..., stop_on_skip: bool)` is a second protocol stuffed into the same trait: when true, stop at first SkipSuffix; when false, consume the batch. Four states of two bools; the type admits calling `scan_run` after a decline (the `unreachable!` is the typechecker). King: the executor asks, the sink answers with a discarded bit, later calls re-validate.
- **Collapsing representation:**

```rust
enum ScanOffer { Declined, Open }
fn begin_scan(...) -> ScanOffer;
fn scan_run(session: &mut Open, ...);
fn emit_batch(...) -> Flow; // D2: the leaf node's SuffixSkip is already on the plan;
                            // don't re-pass it as a bool — the sink's SkipCapability
                            // plus the node's SuffixSkip are the two real facts
```

Or a typestate so `scan_run` is not on the declined arm. `stop_on_skip` dies: projection's `emit_batch` reads `node.suffix_skip` / `skip_capability()` the executor already knows.
- **Essential vs accidental:** not every sink can scan-fold; D2 is projection-only. Those are essential capabilities. Encoding them as bools on one trait is accidental.
- **Severity:** med

### F7. `Executor.pipe: Option` — one executor, then take/put the tables back

- **Where:** `crates/bumbledb/src/exec/run.rs:637-639`; `run/execute.rs:306,380-384,410-441`
- **What's wrong:** "The one executor: multi-node plans pipeline… and single-node plans are one leaf pass." Construction: `pipe: (plan.nodes().len() >= 2).then(|| PipeTables::of(plan))`. `execute` rematches `if self.pipe.is_some()`, then `run_pipeline` does `self.pipe.take().expect("dispatched on Some")` and stuffs it back at the end — Wave 1 F9's split-borrow tax (ReachDriver stolen from the enum every iteration), one layer down. Single-node with `Some(pipe)` and multi-node with `None` are representable; the expect is the typechecker.
- **Collapsing representation:** a construction sum, not an Option sidecar.

```rust
enum Drive {
    Leaf,                           // one node: run_node at 0
    Pipeline(PipeTables),           // ≥2 nodes
}
```

`execute` matches once and stays matched. No take/put.
- **Essential vs accidental:** one-node vs multi-node execution is essential (no parent batch to pipeline). Option-plus-expect is accidental layout.
- **Severity:** med

### F8. `carried_col` is an Option-padded reverse index of `carried`

- **Where:** `crates/bumbledb/src/exec/run.rs:687-692`; `run/pipe_tables.rs:27-43`; readers `pump.rs:81,122`; `probe_pass.rs:333,417`
- **What's wrong:** `carried: Vec<Vec<usize>>` is the list of occurrences this node carries. `carried_col: Vec<Vec<Option<usize>>>` is the same fact, dense in occurrence-id, `None` for "not carried," `Some(i)` = index into `carried[node]`. Two encodings of one sparse set. Hoare: null in every occ slot. Every reader `match tables.carried_col[node][occ]`.
- **Collapsing representation:** one dense map, or the list plus a small `occ -> col` table *only for carried occs* (a `Vec<(occ, col)>` / hashmap). Not `Vec<Option>` sized to all occurrences including those that never appear. Dijkstra: the empty range is the empty list, not a vec of Nones.
- **Essential vs accidental:** carried cursors are essential pipeline data. The Option-padded reverse index is accidental dual.
- **Severity:** med

### F9. `KeyProbePlan.statement: Option` is U vs M as a hole

- **Where:** `crates/bumbledb/src/exec/dispatch.rs:44-59`; `dispatch/classify.rs:102,137-166`
- **What's wrong:** `None` means every field is bound (full-fact `M` membership); `Some(id)` means a `U` determinant get. Option-as-tag. `key` is "projection order for U, declaration order for M" — the same vec, two meanings, distinguished by the hole. Classify already computed which arm it was (`key_probe_candidate` returns `(Option<StatementId>, Vec<FieldId>)`).
- **Collapsing representation:**

```rust
enum KeyProbeKind {
    Uniqueness { statement: StatementId, key: Vec<(FieldId, Const)> },
    Membership { key: Vec<(FieldId, Const)> },
}
```

`execute_key_probe` / `key_probe_fact` match the kind. No `statement.is_some()`.
- **Essential vs accidental:** U vs M are essential access paths. Option is accidental spelling (C ABI essential-C is C1; this type is trusted Rust).
- **Severity:** med

### F10. `batch_sources` / `scan_sources` are `Vec<Option<usize>>` after `LeafSource` exists

- **Where:** `crates/bumbledb/src/exec/sink.rs:237-244,420-423`; `sink/projection/sink.rs:34-37,85`; `sink/aggregate/sink.rs:45-56`
- **What's wrong:** `LeafSource { Key(usize) | Outer }` is the plan's word-source sum (`run.rs:57-63`). Projection then writes `Some(word) | None` into `batch_sources`. Aggregate scan writes `Option<usize>` into `scan_sources` (`over_slot.and_then(|slot| key_slots.position(...))`). The sum was parsed, then flattened to Option so every row loop re-tests `if let Some`. Null in every projected word.
- **Collapsing representation:** `Vec<LeafSource>` (or `MeasuredSource`, which already exists for the measured path). Scan sources: `enum FoldSource { Outer, Column(usize) }` — Count contributes nothing (F1), not a None in a parallel array.
- **Essential vs accidental:** some output words come from the leaf, some from outer bindings — essential. Option as that fact is accidental; the enum already exists.
- **Severity:** med

### F11. `LeafPrecompute.single: bool` plus empty vecs for the other arm

- **Where:** `crates/bumbledb/src/exec/run.rs:618-621,736-741`; `run/leaf_precompute.rs:24-44`; `run/execute.rs:300`; `run/run_node.rs:35`
- **What's wrong:** `single: bool` means "the last node is a one-subatom leaf eligible for fast paths." When false, `residual_sources`, `scan_residuals`, `const_residuals`, and `row` are empty and must not be read. When true, they are the precompute. A flag plus four payloads — Minsky's product. `run_node` tests `self.leaf_single` then trusts the vecs.
- **Collapsing representation:**

```rust
enum LeafShape {
    Generic,
    Fast {
        residual_sources: Vec<(Source, Source)>,
        scan_residuals: Vec<(CmpOp, Source, Source)>,
        const_residuals: Vec<(CmpOp, usize, usize)>,
        row: Vec<u64>,
    },
}
```

`run_node` matches. Empty-vec-as-None dies.
- **Essential vs accidental:** a conservative fast-path decline is essential. The bool-plus-ghost-fields encoding is accidental.
- **Severity:** med

### F12. `SelectionLevel.set: bool`, then `Colt` stores a parallel `set_levels: Vec<bool>`

- **Where:** `crates/bumbledb/src/exec/colt.rs:290-293,324-337`; `exec/colt/new.rs:19`; `exec/colt/select.rs:32-37`
- **What's wrong:** A selection level is point-probe or set-union. The type says `{ columns, set: bool }`. Construction then *projects* `set` into `Colt.set_levels: Vec<bool>` — a second array that must stay aligned with `schema_columns`'s selection prefix. `select` branches `if self.set_levels[level]`. The structured form was parsed and discarded (F2 in miniature). `selected: bool` is a third flag ("select ran; always true for selection-free tries" — vacuous success as a pretend-ran bit).
- **Collapsing representation:**

```rust
enum SelectionLevel { Point { columns: Vec<usize> }, Set { columns: Vec<usize> } }
```

Colt stores `Vec<SelectionLevel>` (or a per-level enum next to columns), not a bool strip. `selected` is `enum SelectState { Vacuous, Pending, Done }` — selection-free is Vacuous, not `true`.
- **Essential vs accidental:** set-bound vs scalar selection is essential (plan fact). Bool plus a projected bool array is accidental.
- **Severity:** med

### F13. `all_cancelled: bool` and `poison: Option<Poison>` are one stop, two fields

- **Where:** `crates/bumbledb/src/exec/run.rs:646-657`; `run/cancel.rs:5-14`; `run/execute.rs:387-392`; `run/probe_pass.rs:40,670`
- **What's wrong:** Comments: poison is "one sum, not parallel flags" and `all_cancelled` is "the ONE stop condition." Then there are two fields. `poison()` writes both (`get_or_insert` + `all_cancelled = true`). D2 root-skip writes only `all_cancelled`. Representable: `poison: Some` with `all_cancelled == false` (a direct field write). Loops test the bool; `execute` drains the Option. Dual encoding of "stop," with the reason as a sidecar that can go missing.
- **Collapsing representation:**

```rust
enum DriveState { Running, SkipDone, Poisoned(Poison) }
```

Loops test `!= Running`. `execute` matches SkipDone vs Poisoned. Unpaired poison is unrepresentable.
- **Essential vs accidental:** D2 skip vs typed overflow are essential different answers. Two fields that must be paired by convention is accidental.
- **Severity:** med

### F14. `row_fold_only: bool` restates `pack` and `measures`

- **Where:** `crates/bumbledb/src/exec/sink.rs:377-381`; `sink/aggregate/new.rs:227`; `sink/aggregate/sink.rs:28,198`
- **What's wrong:** `row_fold_only = pack.is_some() || !measures.is_empty()`. A flag that is a function of two other fields, stored, then tested on the scan path. Drift if `aim` updates `pack`/`measures` without the flag (today `aim` does not recompute it — Pack-with-measures is validation-refused, so the flag is sticky-correct only because the illegal combo cannot arrive).
- **Collapsing representation:** don't store it. Test `self.pack.is_some() || !self.measures.is_empty()`, or make Pack/measures a sum that *is* the row-fold arm (`enum AggBody { Folds(...), Pack { slot }, Measured { .. } }`).
- **Essential vs accidental:** per-row fold for derived measure words and Pack claims is essential. The cached bool is accidental.
- **Severity:** low

### F15. `cover_choice(..., exact: bool)` throws away `KeyCount`

- **Where:** `crates/bumbledb/src/exec/run.rs:198-199`; `exec/colt.rs:51-67` (`KeyCount::Exact | Estimate`)
- **What's wrong:** Cover choice compares `KeyCount` magnitudes and uses the label only to break ties. The counters seam then accepts `exact: bool` — the tag without the magnitude. Introspection histograms Exact vs Estimate from that bit. The sum exists; the observability seam flattened it to a bool.
- **Collapsing representation:** `fn cover_choice(&mut self, node: usize, subatom: usize, count: KeyCount)`. Histogram matches the enum. No bool.
- **Essential vs accidental:** Exact vs Estimate is essential (docs: label-first preference was the bug). Passing only the tag is accidental.
- **Severity:** low

---

## Overlaps (DUPLICATE — not re-opened)

### F16. `fj/validate.rs` claims a sealed ValidatedQuery has no Interior occurrence

- **Where:** `plan/fj/validate.rs:198-217`
- **Duplicate of:** engine-030 (dead `normalize()` + the lie), engine-011 (the same sentence as a false invariant)
- **Severity:** duplicate

### F17. `INTERIOR_PLANNING_ROWS` aliases `ACCUMULATED_PLANNING_ROWS`; "delta-variant" comment

- **Where:** `plan/selectivity.rs:89-110,134-135`
- **Duplicate of:** engine-018 (alias + floor side channel), engine-007 ("delta-variant" vocabulary)
- **Severity:** duplicate

### F18. Plan-side `source.edb() else` / `edb()?` forest (Interior as not-EDB)

- **Where:** `plan/selectivity.rs:139`; `plan/planner/densify.rs:82`; `plan/ground/evaluate.rs:139,210`; `plan/fj/provably_distinct.rs:47`; `plan/fj/provably_disjoint.rs:63`; `exec/dispatch/classify.rs:94`
- **Duplicate of:** engine-017 (accepted half: bind/floor role on the occurrence; `AtomSource` shape stays C1)
- **Severity:** duplicate

### F19. `ground_program` / `grounded_program` / "the grounded program"

- **Where:** `plan/ground.rs:402`; `plan/ground/tests.rs:688-691`
- **Duplicate of:** engine-034
- **Severity:** duplicate

### F20. Program / strata / predicate-p vocabulary in plan tests and exec below prepare

- **Where:** `plan/ground/evaluate/tests.rs:809`; `exec/wordmap/clear.rs:47`; `exec/sink.rs:7,31`; `exec/sink/aggregate/fold_row.rs:108`; `exec/dispatch/execute_key_probe.rs:11`; `exec/sink/projection/new.rs:74`; `exec/introspection.rs:15,66-94,114`; `exec/introspection/into_stats.rs:10`; `exec/introspection/counting_counters.rs:29`; `exec/introspection/display.rs:23,153,186`
- **Duplicate of:** engine-011 (load-bearing sweep; its `rg -inw 'program'` already covers these), engine-035 (stub), engine-029 (unit_labels), engine-033 (`predicate p{}`), engine-007 (delta-variant / strata comments on the report)
- **Severity:** duplicate

---

## Not counted as bugs

- **Mutual / nonlinear / stacked rec, walls.** This-cut OPEN refusals.
- **C ABI essential C.** C1.
- **Naive-oracle full-lfp.** Required definitional oracle.
- **Hostile `FindTerm::Aggregate { over: Option }` on `ir.rs`.** C1 boundary; F1 is the *trusted* `FindSpec` copy.
- **`FjPlan` as unvalidated plain data.** The hostile plan; `ValidatedPlan` is the parse. Correct C1-shaped split.
- **Grouped-by-kind residual lists on `PlanNode`.** Recorded refusal in `fj.rs:251-260` (batching law); do not merge into one `RejectionFilter`.
- **DP `Vec<Option<State>>`.** Essential subset-DP table, not a Query coordinate.
- **COLT `Cursor` / `NodeState` / `KeyCount`, `GroupTable`, `ProjectionSources`, `Verdict3`, `Bindings` epoch-not-Option.** These are already the collapsing sums. Not findings.
- **Grounding's Interior refusal (no sealed extension, no keys).** Essential (undecidable predicate containment / no `U` on a derived table). The *encoding* as `edb() else` is F18/engine-017, not the refusal.

---

## Counts

| Severity   | Count |
|------------|------:|
| high       |     2 |
| med        |    12 |
| low        |     2 |
| duplicate  |     5 |
| **OPEN**   | **16** |
| **total**  | **21** |

High: F1–F2. Med: F3–F13 + exec-017 absorb. Low: F14–F15. Duplicate: F16–F20. Post-validation OPEN count is in the adversarial section below.

Issue files: `plan-001`..`plan-008`, `exec-001`..`exec-017`. Duplicates are stubs. `INDEX.md` not edited.

---

## Adversarial validation (2026-08-14)

Citations opened against live `plan/` and `exec/`. No product-code edits. Wave-1 `api/prepared/*` engine-* parents still OPEN — duplicate stubs kept.

### Verdicts

| Id | Verdict | One line |
|----|---------|----------|
| plan-001 | REWRITE | Diagnosis holds (`FoldedMark { ids, negated }` at `ir/normalize.rs:84-92`; reparse `into_stats.rs:97-101`). Fix was a dual (`ids: u16` *and* a vec) and did not drop `Copy`. Mark is now polarity sum + capped σ-survivors + `RelationId`; `Role` loses `Copy`. |
| plan-002 | KEEP | Two `pinned_fields` (`plan.rs:21-35` scalar-Eq only; `provably_disjoint.rs:117-126` every Eq). Verdicts agree today because `provably_different` returns false for sets; the dual is still the defect. |
| plan-003 | REWRITE | Interior cannot reach `relation()` post-validate (ground/classify/selectivity/run_join guard first) — still a type-level hole. Sibling `Occurrence::relation()` (`ir/normalize.rs:159`) was uncited. Do not accept Interior as a relation id. |
| plan-004 | already-DUPLICATE | engine-030 still OPEN; `validate.rs:198-217` still the false "no Interior" sentence. |
| plan-005 | already-DUPLICATE | engine-018 still OPEN; `INTERIOR_PLANNING_ROWS` still aliases at `selectivity.rs:110`. |
| plan-006 | already-DUPLICATE | engine-017 still OPEN; `edb() else` forest still in selectivity/densify/evaluate/provably_distinct/disjoint/classify. |
| plan-007 | already-DUPLICATE | engine-034 still OPEN; `grounded_program` at `ground/tests.rs:691`, "grounded program" at `ground.rs:402`. |
| plan-008 | already-DUPLICATE | engine-011 still OPEN; `multi_rule_programs_fold_per_rule_independently` at `evaluate/tests.rs:809`. |
| exec-001 | KEEP | `FindSpec`/`SinkSpec` `Agg { over_slot: Option }` live at `sink.rs:77-86,102-114`; `over_slot.expect` at fold_row/fold_batch/aggregate sink. Hostile `ir.rs::FindTerm::Aggregate { over: Option }` stays (C1). |
| exec-002 | KEEP | `DedupRegime` parsed at `aggregate/new.rs:398-408`, flattened onto `distinct_witness`/`seen`/`union_spans`/`dnf_rekey` (`sink.rs:328-408`). Live state is not the enum. Do not drop `DistinctWitness`. |
| exec-003 | REWRITE | `begin_scan -> bool` + `unreachable!` holds. Dropping `stop_on_skip` and always SkipSuffix after first row would drop Forbidden-node batch rows (`run_node.rs:616-619`). Split methods / pass `SuffixSkip`; do not AND two sums into a bool. |
| exec-004 | KEEP | `pipe: Option<PipeTables>` at `run.rs:639`; `take().expect` at `execute.rs:410`; stuffed back at `:441`. |
| exec-005 | KEEP | `carried_col: Vec<Vec<Option<usize>>>` at `run.rs:692`; `vec![None; n_occ]` at `pipe_tables.rs:31`. |
| exec-006 | KEEP | `KeyProbePlan.statement: Option<StatementId>` at `dispatch.rs:44-59`; `key_probe_fact.rs:255-287` re-tests the hole. |
| exec-007 | KEEP | `batch_sources: Vec<Option<usize>>` at `sink.rs:244` after `LeafSource` exists (`run.rs:58-63`); `scan_sources` Count-as-None rides exec-001. |
| exec-008 | KEEP | `LeafPrecompute.single: bool` plus ghost vecs (`run.rs:736-741`; `leaf_precompute.rs:37-44`); `run_node.rs:35` trusts the flag. |
| exec-009 | KEEP | `SelectionLevel.set: bool` projected to `Colt.set_levels` (`colt.rs:290-325`; `colt/new.rs:19`); `selected = selection_levels == 0`. |
| exec-010 | KEEP | `all_cancelled` + `poison: Option` at `run.rs:646-657`; D2 writes only the bool (`probe_pass.rs:670`); unpaired poison is representable. |
| exec-011 | KEEP | `row_fold_only = pack.is_some() \|\| !measures.is_empty()` stored at `aggregate/new.rs:227`, tested at `aggregate/sink.rs:28,198`. |
| exec-012 | KEEP | `cover_choice(..., exact: bool)` at `run.rs:199`; `KeyCount` exists at `colt.rs:52-57`; scan/leaf sites pass `false` (Estimate). |
| exec-013 | already-DUPLICATE | engine-011 still OPEN; `wordmap/clear.rs:47` "non-recursive program"; `sink.rs:7,31` "program". |
| exec-014 | already-DUPLICATE | engine-029 still OPEN; `unit_labels` emptiness mode at `introspection.rs:83-97`. |
| exec-015 | already-DUPLICATE | engine-033 still OPEN; `display.rs:153` `predicate p{}`. |
| exec-016 | already-DUPLICATE | engine-007 still OPEN; `introspection.rs:87-90` `stats.strata` / delta variants. |
| exec-017 | NEW | `PipeTables.absorb: Option<usize>` (`run.rs:693-699`; `pipe_tables.rs:45-52`) is Root vs Node as a hole; `probe_pass.rs:667-670` re-decodes it. |

### Deleted paths

None.

### Rewritten

- `audit/issues/plan-001-folded-mark-discard.md`
- `audit/issues/plan-003-relation-panic-interior.md`
- `audit/issues/exec-003-begin-scan-bool.md`

### New files

- `audit/issues/exec-017-absorb-option.md`

### Remaining OPEN (this tree)

16 OPEN (plan-001..003, exec-001..012, exec-017) + 9 DUPLICATE stubs. Was 15 OPEN + 9 DUPLICATE.

### CONTRACT tension

- C1 keeps hostile `ir.rs::FindTerm::Aggregate { over: Option }`; exec-001 is the *trusted* `FindSpec`/`SinkSpec` sum. C1 does not freeze plan/exec trusted types.
- C6 already named Count-vs-folds for SDKs; the engine never parsed the same split (exec-001).
- C3 prepared sums stay engine-*; these issues do not re-open engine-001/002/007/011/017/018/029/030/033/034 as new OPENs.
- Assertions never weakened: plan-001's `unwrap_or_default` dies, not becomes another silent empty; plan-003 does not replace `unreachable!` with a fallback; exec-003 must not change which batch rows a Forbidden node delivers.
