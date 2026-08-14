# Issue index — fanout ledger

184 files. Dump reconciliation: wave-1 lean 18/18, engine 40/40, sdk 21/21, docs 28/28; wave-2 lean-rest 4/4, sdk-rest 7/7, plan-exec 15 OPEN + 9 DUPLICATE, storage-schema 23 OPEN + 2 WONTFIX, bench 9 OPEN + 3 DUPLICATE.

**Assign only OPEN.** 156 OPEN, 24 DUPLICATE, 4 WONTFIX. A fixer who lands a parent does not open the duplicate.

| Status | Ids |
|---|---|
| WONTFIX | lean-013 (C5 R-DENSE), engine-037 (`Query::single` is correct), schema-010 (descriptor `Option` is the hostile spelling), store-004 (`ForeignPreparedQuery` is essential identity) |
| DUPLICATE | lean-012→001, lean-014→004, lean-016→005, lean-017→002; engine-028→003, engine-035→011, engine-036→026, engine-038→012, engine-039→007, engine-040→013; sdk-019→001, sdk-020→004; plan-004→engine-030, plan-005→engine-018, plan-006→engine-017, plan-007→engine-034, plan-008→engine-011; exec-013→engine-011, exec-014→engine-029, exec-015→engine-033, exec-016→engine-007; bench-010→engine-041, bench-011→engine-011, bench-012→engine-020 |

Every OPEN file is the unit of work: bug + citations, why, CONTRACT citation, mechanical acceptance, constraints. Doctrine: `audit/00-representation-is-the-essence.md`. Authority: `audit/CONTRACT.md` C1–C8. Proposed C9 (sealed schema sums) and C10 (corruption variants) live in schema/err issues only — do not implement them as if they were in CONTRACT until they are pinned. Locked names: `DerivedBudgetExceeded`, `set_derived_budget`, `DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`. Corpus JSON, `ir.rs::Query`, C ABI shapes frozen (C1). Assertions never weakened.

## Fanout

**Wave 0 — one line, first.** `lean-019` un-reds `scripts/lean.sh`. Every later issue that lists that script as green waits on it.

**Wave 1 — foundations (parallel; one fixer per cluster).**

| Cluster | OPEN issues | Notes |
|---|---|---|
| Lean sum | lean-001 + lean-002 | one commit |
| Engine pipeline | engine-001 + engine-002 + engine-015 + engine-023 | one commit |
| Engine witness | engine-005 + engine-006 | same files; coordinate |
| Engine bench | engine-019, engine-020, engine-021, engine-030 | four independent |
| Engine rename | engine-034 | `ground_program` → `ground_main` |
| C++ phase | sdk-001 + sdk-002 | one commit; lowering `has_rec` included |
| C++ independents | sdk-003, sdk-004, sdk-010, sdk-021 | 004 owns Measure + dummy Var op |
| TS phase | sdk-005; sdk-007, sdk-017 | 007 same file as 005 — coordinate |
| Macros | sdk-014, sdk-015 | one fixer may take both |
| Docs, no code deps | docs-001, 003–011, 013–016, 018–026, 028, 029 | group by file: 20-query-ir, 40-execution, 60-validation, 70-api, 75-cpp-lowering, README, cookbook, feature-register, conformance README |
| Schema sealed | schema-001 + schema-002 | 002 co-lands with schema-006; C9 not yet in CONTRACT |
| Exec sums | exec-001 + exec-002 | Count-as-None; DedupRegime kept |
| C++ schema | sdk-023 + sdk-024 | closed flag + tag-plus-payloads; same files |
| Plan independents | plan-001, plan-002 | folded mark; dual `pinned_fields` |
| Bench encoder | bench-001 | twin of lean-008; C1 keeps dual *spellings* |
| Store / err independents | store-001, store-003, store-005; err-001–006 | no schema-001 wait except store-002 |

**Wave 2 — after that tree's wave-1 cluster lands.**

- Lean: lean-003, 004, 005, 006, 007, 008, 009, 010, 011, 015, 018; lean-021 after 001/002 (and 006 for field width); lean-022 after 005; lean-023 after 008; lean-020 + lean-024 last (comment sweep).
- Engine on witness: engine-003, 004, 016, 022, 027, 041.
- Engine on pipeline: engine-007, 008, 009, 011, 012, 013, 014, 018, 024, 025, 026, 031.
- Engine on 013: engine-010, 017, 032.
- Introspection (one `INTROSPECTION_VERSION` bump): engine-029 + engine-033 with engine-012.
- SDK query: sdk-006 then sdk-016; sdk-008 with sdk-004 (ABI `has_over`); sdk-009, 011, 012, 013 after 001/002; sdk-022 after 005.
- SDK schema: sdk-025 after 023; sdk-026 after 024; sdk-027 (coord 004/008, own crate); sdk-028; sdk-029 with 014/015/027.
- Plan: plan-003 after plan-001 + engine-017.
- Exec rest: exec-003, 006, 008, 009, 010, 012 independent; exec-004 then 005; exec-007 and 011 after 001.
- Schema rest: schema-003 + 008; schema-004, 005, 007, 009 after 001.
- Store/image: store-002 after schema-001/002; image-001 after schema-001; image-002–004 independent.
- Bench: bench-002 independent; bench-003 after engine-020; bench-004 after 020/003; bench-005 and 006 after engine-021; bench-007, 009 independent; bench-008 after engine-012.

**Wave 3 — closers.**

- sdk-018 — compile-fail suite after 001/004/005/012/013.
- docs-002 + docs-017 after engine-041 (`Signature` / `signature()`).
- docs-012 + docs-027 after lean-008 (one decoder); bench-001 is the encoder twin.

Cross-tree edges: engine-041 → docs-002/017; lean-008 ↔ bench-001 → docs-012/027; sdk-008 ↔ sdk-004 (one ABI commit); sdk-023 ∥ schema-001 (closedness, different trees); exec-001 ∥ sdk-004/008/027 (Count-as-None); image-001 / store-002 → schema-001; bench-003 → engine-020. Same-file issues: one fixer or strict order.

## All issues

### lean (24)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| lean-001 | `Query` product with `Option Rec` → inductive sum | high | OPEN | lean-002 (one change) |
| lean-002 | untyped `Rec` → typed `LinearRec` | high | OPEN | none (with lean-001) |
| lean-003 | dual rec-identity coordinates | high | OPEN (scoped, §C5) | lean-001, 002 |
| lean-004 | unspent `WellFormed` bundle | high | OPEN (scoped, §C5) | lean-001 |
| lean-005 | two denotations | high | OPEN | lean-001 |
| lean-006 | orphan arity fields | high | OPEN (scoped, §C5) | lean-001, 002 |
| lean-007 | staged interior eval → fold | med | OPEN | lean-001 (file conflict) |
| lean-008 | two decoders / `CQuery` | med | OPEN | lean-001, 002 |
| lean-009 | `allRules` flatten | med | OPEN | lean-001 |
| lean-010 | naive iterators in the meaning | med | OPEN | lean-002 |
| lean-011 | `recDom` / idb vocabulary | med | OPEN | lean-002 |
| lean-012 | Option-rec flag | med | DUPLICATE(lean-001) | — |
| lean-013 | total InteriorEnv | med | WONTFIX (§C5) | — |
| lean-014 | `edbOnly` flag | med | DUPLICATE(lean-004) | — |
| lean-015 | `odd_not_stratified` name | low | OPEN | lean-002 |
| lean-016 | RewriteStep dummy arity | low | DUPLICATE(lean-005) | — |
| lean-017 | selfCount unpack | low | DUPLICATE(lean-002) | — |
| lean-018 | empty-rules rec-answer surprise | low | OPEN | lean-001 |
| lean-019 | Bridge cites deleted `translate/program.rs` | high | OPEN | **first** |
| lean-020 | "rec SCC" in Lean comments | low | OPEN | after 001/002 |
| lean-021 | Membership/key-probe collapse `AtomSource` to `RelId ⟨0⟩` | high | OPEN | 001, 002 (006 for field width) |
| lean-022 | Plan denotes `edbEnv` | med | OPEN | 005 |
| lean-023 | `HeadSlot` fourth head-shape encoding | med | OPEN | 008 |
| lean-024 | rest-of-tree Lean "program" comments | low | OPEN | after 001/005 (with 020) |

### engine (41)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| engine-001 | interiors beside `PreparedBody` → pipeline sum | high | OPEN | none (w/ 002, 015, 023) |
| engine-002 | `PreparedRule::Recursive` → `RecArm` | high | OPEN | 001 (co-lands) |
| engine-003 | rec id `len()` pun; store once | high | OPEN (scoped, §C1/C2) | 005, 016 |
| engine-004 | empty rec arms on the witness | high | OPEN | 005 |
| engine-005 | witness sum + `self_occ` | high | OPEN | none |
| engine-006 | `Option<Predicate>` sealing holes | high | OPEN | none (coord. w/ 005) |
| engine-007 | `DeltaVariant` → `prepare_rec_arm` | high | OPEN | 002 |
| engine-008 | execute/profile fork | high | OPEN | 001 |
| engine-009 | `run_reach` re-matches | high | OPEN | 001 |
| engine-010 | rec-bind Option soup | high | OPEN | 013, 001 |
| engine-011 | zombie Program vocab + false invariant | high | OPEN | 001, 012 |
| engine-012 | `ExecutionStats` product | high | OPEN | 001 |
| engine-013 | one `DerivedImages` + PingPong | med | OPEN | 001 |
| engine-014 | `rounds_budget` on Reach only | med | OPEN | 001 |
| engine-015 | main not in the driver | med | OPEN | 001 (co-lands) |
| engine-016 | prepare `is_some`/`expect` | med | OPEN | 005 |
| engine-017 | `edb().is_none()` bind | med | OPEN (scoped, §C1) | 010, 018 |
| engine-018 | planning-floor alias | med | OPEN | 007 |
| engine-019 | naive oracle flags | med | OPEN | none |
| engine-020 | querygen side entry | med | OPEN | none |
| engine-021 | translator two-flag gate | med | OPEN | none |
| engine-022 | rec parser three walks | med | OPEN | 005, 004 |
| engine-023 | `Empty` not a variant | med | OPEN | 001 (co-lands) |
| engine-024 | dual ray-probe loops | med | OPEN | 001, 013 |
| engine-025 | accessor forest | med | OPEN | 001, 002 |
| engine-026 | rule enum per sink | med | OPEN | 002 |
| engine-027 | nonempty witness lists | med | OPEN | 005, 004 |
| engine-028 | derived-count restated | med | DUPLICATE(engine-003) | — |
| engine-029 | `unit_labels` as mode bit | med | OPEN | 012, 001 |
| engine-030 | dead `normalize()` | med | OPEN | none |
| engine-031 | key-probe rematch → `Ok(())` | med | OPEN | 001, 008 |
| engine-032 | `occ_images` Option slots | med | OPEN | 013, 010 |
| engine-033 | `predicate p{id}` strings | low | OPEN | 012/029 (one bump) |
| engine-034 | `ground_program` → `ground_main` | low | OPEN | none |
| engine-035 | "program" in tests | low | DUPLICATE(engine-011) | — |
| engine-036 | `_either_sink_marker` | low | DUPLICATE(engine-026) | — |
| engine-037 | `Query::single` is correct | low | WONTFIX | — |
| engine-038 | stats/JSON drift | low | DUPLICATE(engine-012) | — |
| engine-039 | `delta: Option<OccId>` | low | DUPLICATE(engine-007) | — |
| engine-040 | ping-pong "Size 1" | low | DUPLICATE(engine-013) | — |
| engine-041 | `Predicate` → `Signature` | low | OPEN | 005/006 |

### sdk (29)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| sdk-001 | C++ `query_value` phase machine | high | OPEN | none (w/ 002) |
| sdk-002 | one C++ IR | high | OPEN | 001 (co-lands) |
| sdk-003 | `wire_atom` bool + both ids | high | OPEN | none |
| sdk-004 | `find_form` Measure (+ dummy Var op) | high | OPEN | none |
| sdk-005 | TS `QueryStart` phase | high | OPEN | none |
| sdk-006 | branded `ParsedQuery` | high | OPEN | 005 |
| sdk-007 | `collectRec` casts | high | OPEN | none (file w/ 005) |
| sdk-008 | ABI `has_over` + marshal parse | high | OPEN | coord. 004, 006 |
| sdk-009 | wildcard as `absent` | med | OPEN | 011 |
| sdk-010 | interior polarity bool | med | OPEN | none |
| sdk-011 | tag-plus-all-payloads IR | med | OPEN | 001/002 |
| sdk-012 | sugar caps | med | OPEN | 001 |
| sdk-013 | condition trees | med | OPEN | 011 |
| sdk-014 | `ParsedRule` sum | med | OPEN | none |
| sdk-015 | param style two bools | med | OPEN | none |
| sdk-016 | `isQueryValue` forgets | med | OPEN | 006 |
| sdk-017 | `CmpData.mask` | med | OPEN | none |
| sdk-018 | compile-fail suite | med | OPEN | 001/004/005/012/013 — last |
| sdk-019 | `derived_tables` rec flag | med | DUPLICATE(sdk-001) | — |
| sdk-020 | dummy Var `op` | low | DUPLICATE(sdk-004) | — |
| sdk-021 | empty-interiors dummy array | low | OPEN | none |
| sdk-022 | SDK comment vocabulary | low | OPEN | after 005 |
| sdk-023 | C++ `relation_data` closed flag + leftover `closed_info` | high | OPEN | none (w/ 024) |
| sdk-024 | schema-lane tag-plus-all-payloads | med | OPEN | 023 |
| sdk-025 | schema sugar caps (`max_closed_handles = 8`) | med | OPEN | 023 |
| sdk-026 | `==` flattened to `bidirectional: bool` | med | OPEN | 024 |
| sdk-027 | `query!` `HeadTerm::Agg` `over: Option` | med | OPEN | coord. 004/008 |
| sdk-028 | violation / statement-slot optionals | med | OPEN | none |
| sdk-029 | `query!` interior-atom style `Option<bool>` | low | OPEN | with 014/015/027 |

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| docs-001 | "multi-rule programs" (20-query-ir) | high | OPEN | none |
| docs-002 | main as anonymous predicate | high | OPEN | engine-041 |
| docs-003 | rec as SCC | high | OPEN | none |
| docs-004 | "today's query" embedding (IR) | med | OPEN | none |
| docs-005 | deleted cap names (IR) | med | OPEN | none |
| docs-006 | "not a Tarjan condensation" | med | OPEN | none |
| docs-007 | fuel hyphen ghost | med | OPEN | none |
| docs-008 | "no program renderer" | med | OPEN | none |
| docs-009 | "former named-head sneak" | med | OPEN | none |
| docs-010 | one-sink contradiction | med | OPEN | none |
| docs-011 | "program" in 40-execution | high | OPEN | none |
| docs-012 | "CQuery arm" | high | OPEN | lean-008 |
| docs-013 | "program whose every disjunct vanishes" | high | OPEN | none |
| docs-014 | "cte-list" emission | low | OPEN | none |
| docs-015 | "today's query" on prepare | med | OPEN | none |
| docs-016 | "data-modifying CTEs" (API) | med | OPEN | none |
| docs-017 | `predicate()` buffer authority | high | OPEN | engine-041 |
| docs-018 | ForeignPreparedQuery horizon | med | OPEN | none |
| docs-019 | cpp-lowering caps / today's query | med | OPEN | none |
| docs-020 | output-last denial | high | OPEN | none |
| docs-021 | README OPEN items in SCC coords | med | OPEN | none |
| docs-022 | cookbook CTE | med | OPEN | none |
| docs-023 | cookbook `Program` relation | low | OPEN | none |
| docs-024 | `AggregateInteriorPredicate` | high | OPEN | none |
| docs-025 | "zero stratification impact" | high | OPEN | none |
| docs-026 | "idb re-grounding tax" | high | OPEN | none |
| docs-027 | conformance two types | high | OPEN | lean-008 |
| docs-028 | "never idb" | med | OPEN | none |
| docs-029 | cookbook "not a second SCC" | low | OPEN | none |

### plan (8)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| plan-001 | `FoldedMark` discards parsed σ | med | OPEN | none |
| plan-002 | two `pinned_fields`; they disagree | med | OPEN | none |
| plan-003 | `PlanOccurrence::relation()` panics on Interior | med | OPEN | 001, engine-017 |
| plan-004 | fj/validate "no Interior" claim | med | DUPLICATE(engine-030) | — |
| plan-005 | planning-floor alias | med | DUPLICATE(engine-018) | — |
| plan-006 | plan-side `edb() else` forest | med | DUPLICATE(engine-017) | — |
| plan-007 | `ground_program` name | low | DUPLICATE(engine-034) | — |
| plan-008 | Program vocabulary in plan | low | DUPLICATE(engine-011) | — |

### exec (16)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| exec-001 | Agg Count as `over_slot: None` | high | OPEN | none (w/ 002) |
| exec-002 | `DedupRegime` flattened to four fields | high | OPEN | none (rides 001) |
| exec-003 | sink scan/skip as bools + `unreachable!` | med | OPEN | none |
| exec-004 | `Executor.pipe: Option` take/put | med | OPEN | none |
| exec-005 | `carried_col` Option-padded reverse index | med | OPEN | 004 |
| exec-006 | `KeyProbePlan.statement: Option` | med | OPEN | none |
| exec-007 | `batch_sources` / `scan_sources` Option holes | med | OPEN | 001 |
| exec-008 | `LeafPrecompute.single: bool` | med | OPEN | none |
| exec-009 | `SelectionLevel.set: bool` | med | OPEN | none |
| exec-010 | stop flags product (`all_cancelled` + `poison`) | med | OPEN | none |
| exec-011 | `row_fold_only: bool` | low | OPEN | 001 |
| exec-012 | `cover_choice(..., exact: bool)` | low | OPEN | none |
| exec-013 | Program vocabulary in exec | high | DUPLICATE(engine-011) | — |
| exec-014 | unit-labels mode bit | med | DUPLICATE(engine-029) | — |
| exec-015 | `predicate p{id}` strings | low | DUPLICATE(engine-033) | — |
| exec-016 | delta-variant comments | high | DUPLICATE(engine-007) | — |

### schema (10)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| schema-001 | sealed `Relation.extension: Option` | high | OPEN | none (foundation) |
| schema-002 | `KeyStatement` flag product → `KeyForm` | high | OPEN | none (w/ 006) |
| schema-003 | capacity tails as sidecar Options | high | OPEN | none (w/ 008) |
| schema-004 | capacity reuses containment `Enforcement` | med | OPEN | none |
| schema-005 | `IntervalTail.width: Option` | med | OPEN | none |
| schema-006 | dual `fresh_row` coordinates | med | OPEN | 002 |
| schema-007 | sealed `mirror` vs render Option holes | med | OPEN | none |
| schema-008 | sealed `hi: Option<Bound>` (`*` as absence) | med | OPEN | 003 |
| schema-009 | `SealedField.declared: Option` | med | OPEN | none |
| schema-010 | descriptor `extension: Option` is hostile spelling | low | WONTFIX | — |

### store (5)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| store-001 | `FactOp` one product for insert and delete | med | OPEN | none |
| store-002 | point-read path is `is_closed × fresh_row × U-tree` | med | OPEN | schema-001, 002 |
| store-003 | `Environment` modes as Option pair | med | OPEN | none |
| store-004 | `ForeignPreparedQuery` essential identity | med | WONTFIX | — |
| store-005 | `CommitReport { changed, new_generation }` | low | OPEN | none |

### image (4)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| image-001 | `closed_slots: Box<[Option<u32>]>` | high | OPEN | schema-001 |
| image-002 | `Const` too wide; `ResolvedWordSource::Var` then `unreachable!` | med | OPEN | none |
| image-003 | `View::image` / `position_at` panic on `Unbound` | low | OPEN | none |
| image-004 | `TransientImage.image: Option` | low | OPEN | none |

### err (6)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| err-001 | `RenderedViolation` tag-plus-payloads | med | OPEN | none |
| err-002 | `Violation::Functionality.incumbent: Option` | med | OPEN | none |
| err-003 | `Violations.cited` empty until `attach_cited` | med | OPEN | none |
| err-004 | `MalformedValue(&'static str)` catch-all | med | OPEN | none (proposed C10) |
| err-005 | `TraceEvent` `dur_ns == 0` ⇒ point event | low | OPEN | none |
| err-006 | "program" vocabulary in error/obs | low | OPEN | none |

### bench (12)

| Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|
| bench-001 | two JSON emitters (CQuery vs Query) | high | OPEN | none (twin of lean-008) |
| bench-002 | irgen never draws interiors/rec | high | OPEN | none |
| bench-003 | stamp/fuzz/seeded/contradict call only `random_query` | high | OPEN | engine-020 |
| bench-004 | CQ-shaped consumers walk `query.rules` only | med | OPEN | engine-020, bench-003 |
| bench-005 | two sqlite expressibility gates | med | OPEN | engine-021 |
| bench-006 | derived CTEs still `p{id}` | med | OPEN | engine-021 |
| bench-007 | closure lane teaches delta-variants | med | OPEN | none |
| bench-008 | `exec_digest` is a CQ stats consumer | med | OPEN | engine-012 |
| bench-009 | "program" names a Query in the remaining crate | low | OPEN | none |
| bench-010 | `.predicate()` accessors | low | DUPLICATE(engine-041) | — |
| bench-011 | closure `exec: None` profile skip | low | DUPLICATE(engine-011) | — |
| bench-012 | querygen tests `RecursiveVariant` tag | low | DUPLICATE(engine-020) | — |

## Green (every OPEN fix commit)

`bash scripts/check.sh` and `bash scripts/lean.sh` (after lean-019), plus the tree-local suites the issue names. Corpus unchanged. Locked names unchanged.
