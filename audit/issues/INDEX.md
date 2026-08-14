# Issue index — fanout ledger

112 files (one per wave-1 finding plus five adversarial). Dump reconciliation: lean 18/18, engine 40/40, sdk 21/21, docs 28/28.

**Assign only OPEN.** 98 OPEN, 12 DUPLICATE, 2 WONTFIX. A fixer who lands a parent does not open the duplicate.

| Status | Ids |
|---|---|
| WONTFIX | lean-013 (C5 R-DENSE), engine-037 (`Query::single` is correct) |
| DUPLICATE | lean-012→001, lean-014→004, lean-016→005, lean-017→002; engine-028→003, engine-035→011, engine-036→026, engine-038→012, engine-039→007, engine-040→013; sdk-019→001, sdk-020→004 |

Every OPEN file is the unit of work: bug + citations, why, CONTRACT citation, mechanical acceptance, constraints. Doctrine: `audit/00-representation-is-the-essence.md`. Authority: `audit/CONTRACT.md` C1–C8. Locked names: `DerivedBudgetExceeded`, `set_derived_budget`, `DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`. Corpus JSON, `ir.rs::Query`, C ABI shapes frozen (C1). Assertions never weakened.

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

**Wave 2 — after that tree's wave-1 cluster lands.**

- Lean: lean-003, 004, 005, 006, 007, 008, 009, 010, 011, 015, 018; lean-020 last (comment sweep).
- Engine on witness: engine-003, 004, 016, 022, 027, 041.
- Engine on pipeline: engine-007, 008, 009, 011, 012, 013, 014, 018, 024, 025, 026, 031.
- Engine on 013: engine-010, 017, 032.
- Introspection (one `INTROSPECTION_VERSION` bump): engine-029 + engine-033 with engine-012.
- SDK: sdk-006 then sdk-016; sdk-008 with sdk-004 (ABI `has_over`); sdk-009, 011, 012, 013 after 001/002; sdk-022 after 005.

**Wave 3 — closers.**

- sdk-018 — compile-fail suite after 001/004/005/012/013.
- docs-002 + docs-017 after engine-041 (`Signature` / `signature()`).
- docs-012 + docs-027 after lean-008 (one decoder).

Cross-tree edges: engine-041 → docs-002/017; lean-008 → docs-012/027; sdk-008 ↔ sdk-004 (one ABI commit). Same-file issues: one fixer or strict order.

## All issues

### lean (20)

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

### sdk (22)

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

### docs (29)

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

## Green (every OPEN fix commit)

`bash scripts/check.sh` and `bash scripts/lean.sh` (after lean-019), plus the tree-local suites the issue names. Corpus unchanged. Locked names unchanged.
