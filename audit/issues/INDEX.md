# Issue index — work queue

188 files. **Assign only OPEN.** 159 FIXED, 25 DUPLICATE, 4 WONTFIX. Verified against disk. Dump reconciliation: wave-1 lean 18/18, engine 40/40, sdk 21/21, docs 28/28; wave-2 lean-rest 4/4, sdk-rest 7/7, plan-exec 15 FIXED + 9 DUPLICATE, storage-schema 23 FIXED + 2 WONTFIX, bench 9 FIXED + 3 DUPLICATE. Final-pass validation: Fix rewrites across trees; four new findings (sdk-030, docs-030, exec-017, schema-011); lean-018 demoted DUPLICATE(lean-001).

A fixer who lands a parent does not open the duplicate. **Ids stay stable** (`lean-019`, `engine-001`, …). Sequence prefixes on filenames are the queue, not new ids.

| Status | Ids |
|---|---|
| WONTFIX | lean-013 (C5 R-DENSE), engine-037 (`Query::single` is correct), schema-010 (descriptor `Option` is the hostile spelling), store-004 (`ForeignPreparedQuery` is essential identity) |
| DUPLICATE | lean-012→001, lean-014→004, lean-016→005, lean-017→002, lean-018→001; engine-028→003, engine-035→011, engine-036→026, engine-038→012, engine-039→007, engine-040→013; sdk-019→001, sdk-020→004; plan-004→engine-030, plan-005→engine-018, plan-006→engine-017, plan-007→engine-034, plan-008→engine-011; exec-013→engine-011, exec-014→engine-029, exec-015→engine-033, exec-016→engine-007; bench-010→engine-041, bench-011→engine-011, bench-012→engine-020 |

Every FIXED file was the unit of work: bug + citations, why, CONTRACT citation, mechanical acceptance, constraints. Doctrine: `audit/00-representation-is-the-essence.md`. Authority: `audit/CONTRACT.md` C1–C9. C9 (sealed schema sums) is pinned.  Do not mint a CONTRACT C10 for corruption variants (`err-004`): `capacity-laws` already uses C10 for rays. Locked names: `DerivedBudgetExceeded`, `set_derived_budget`, `DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`. Corpus JSON, `ir.rs::Query`, C ABI shapes frozen (C1). Assertions never weakened.

## Work order (topological)

Primary interface: `ls audit/issues` and this table. Start at seq **001** and go down. Co-landing clusters are adjacent (one commit). `9xx-` files are DUPLICATE/WONTFIX — not assignments. A node appears only after all FIXED dependencies.

| Seq | Id | Title | Sev | Depends on | Notes |
|---|---|---|---|---|---|
| 001 | lean-019 | Bridge cites deleted `translate/program.rs` | high | — | first; un-reds `scripts/lean.sh` |
| 002 | docs-001 | "multi-rule programs" (20-query-ir) | high | — |  |
| 003 | docs-003 | rec as SCC | high | — |  |
| 004 | docs-005 | deleted cap names (IR) | med | — |  |
| 005 | docs-007 | fuel hyphen ghost | med | — |  |
| 006 | docs-008 | "no program renderer" | med | — |  |
| 007 | docs-009 | "former named-head sneak" | med | — |  |
| 008 | docs-010 | one-sink contradiction | med | — |  |
| 009 | docs-011 | "program" in 40-execution | high | — |  |
| 010 | docs-013 | "program whose every disjunct vanishes" | high | — |  |
| 011 | docs-014 | "cte-list" emission | low | — |  |
| 012 | docs-016 | "data-modifying CTEs" (API) | med | — |  |
| 013 | docs-018 | ForeignPreparedQuery horizon | med | — |  |
| 014 | docs-019 | cpp-lowering caps / today's query | med | — |  |
| 015 | docs-020 | output-last denial | high | — |  |
| 016 | docs-022 | cookbook CTE | med | — |  |
| 017 | docs-023 | cookbook `Program` relation | low | — |  |
| 018 | docs-024 | `AggregateInteriorPredicate` | high | — |  |
| 019 | docs-026 | "idb re-grounding tax" | high | — |  |
| 020 | docs-029 | cookbook "not a second SCC" | low | — |  |
| 021 | lean-001 | `Query` product with `Option Rec` → inductive sum | high | — | one commit with lean-002 |
| 022 | lean-002 | untyped `Rec` → typed `LinearRec` | high | — | one commit with lean-001 |
| 023 | lean-003 | dual rec-identity coordinates | high | lean-001,lean-002 |  |
| 024 | lean-004 | unspent `WellFormed` bundle | high | lean-001 |  |
| 025 | lean-005 | two denotations | high | lean-001 |  |
| 026 | lean-006 | orphan arity fields | high | lean-001,lean-002 |  |
| 027 | lean-007 | staged interior eval → fold | med | lean-001 |  |
| 028 | lean-008 | two decoders / `CQuery` | med | lean-001,lean-002 |  |
| 029 | lean-009 | `allRules` flatten | med | lean-001 |  |
| 030 | lean-010 | naive iterators in the meaning | med | lean-002 |  |
| 031 | lean-011 | `recDom` / idb vocabulary | med | lean-002 |  |
| 032 | lean-015 | `odd_not_stratified` name | low | lean-002 |  |
| 033 | lean-021 | Membership/key-probe collapse `AtomSource` to `RelId ⟨0⟩` | high | lean-001,lean-002,lean-006 |  |
| 034 | lean-022 | Plan denotes `edbEnv` | med | lean-005 |  |
| 035 | lean-023 | `HeadSlot` fourth head-shape encoding | med | lean-008 |  |
| 036 | lean-020 | "rec SCC" in Lean comments | low | lean-001,lean-002,lean-021,lean-022,lean-023 | comment sweep; last lean with 024 |
| 037 | lean-024 | rest-of-tree Lean "program" comments | low | lean-001,lean-005,lean-020 | comment sweep; last lean with 020 |
| 038 | engine-001 | interiors beside `PreparedBody` → pipeline sum | high | — | one commit with 002/015/023 |
| 039 | engine-002 | `PreparedRule::Recursive` → `RecArm` | high | engine-001 | one commit with 001/015/023 |
| 040 | engine-015 | main not in the driver | med | engine-001 | one commit with 001/002/023 |
| 041 | engine-023 | `Empty` not a variant | med | engine-001 | one commit with 001/002/015 |
| 042 | engine-005 | witness sum + `self_occ` | high | — | coord. with engine-006 (same files) |
| 043 | engine-006 | `Option<Predicate>` sealing holes | high | — | coord. with engine-005 (same files) |
| 044 | engine-004 | empty rec arms on the witness | high | engine-005 |  |
| 045 | engine-007 | `DeltaVariant` → `prepare_rec_arm` | high | engine-001,engine-002 |  |
| 046 | engine-008 | execute/profile fork | high | engine-001 |  |
| 047 | engine-009 | `run_reach` re-matches | high | engine-001 |  |
| 048 | engine-012 | `ExecutionStats` product | high | engine-001 |  |
| 049 | engine-011 | zombie Program vocab + false invariant | high | engine-001,engine-012 |  |
| 050 | engine-013 | one `DerivedImages` + PingPong | med | engine-001 |  |
| 051 | engine-010 | rec-bind Option soup | high | engine-001,engine-013 |  |
| 052 | engine-014 | `rounds_budget` on Reach only | med | engine-001 |  |
| 053 | engine-016 | prepare `is_some`/`expect` | med | engine-005 |  |
| 054 | engine-003 | rec id `len()` pun; store once | high | engine-005,engine-016 |  |
| 055 | engine-018 | planning-floor alias | med | engine-007 |  |
| 056 | engine-017 | `edb().is_none()` bind | med | engine-010,engine-018 |  |
| 057 | engine-019 | naive oracle flags | med | — |  |
| 058 | engine-020 | querygen side entry | med | — | INDEX independent; broke issue-file cycle with bench-004 |
| 059 | engine-021 | translator two-flag gate | med | — |  |
| 060 | engine-022 | rec parser three walks | med | engine-004,engine-005 |  |
| 061 | engine-024 | dual ray-probe loops | med | engine-001,engine-013 |  |
| 062 | engine-025 | accessor forest | med | engine-001,engine-002 |  |
| 063 | engine-026 | rule enum per sink | med | engine-002 |  |
| 064 | engine-027 | nonempty witness lists | med | engine-004,engine-005 |  |
| 065 | engine-029 | `unit_labels` as mode bit | med | engine-001,engine-012 |  |
| 066 | engine-030 | dead `normalize()` | med | — |  |
| 067 | engine-031 | key-probe rematch → `Ok(())` | med | engine-001,engine-008 |  |
| 068 | engine-032 | `occ_images` Option slots | med | engine-010,engine-013 |  |
| 069 | engine-033 | `predicate p{id}` strings | low | engine-012,engine-029 |  |
| 070 | engine-034 | `ground_program` → `ground_main` | low | — |  |
| 071 | engine-041 | `Predicate` → `Signature` | low | engine-005,engine-006 | unlocks docs-002/017 |
| 072 | sdk-001 | C++ `query_value` phase machine | high | — | one commit with sdk-002 |
| 073 | sdk-002 | one C++ IR | high | sdk-001 | one commit with sdk-001 |
| 074 | sdk-003 | `wire_atom` bool + both ids | high | — |  |
| 075 | sdk-005 | TS `QueryStart` phase | high | — | co-land sdk-005+sdk-007 |
| 076 | sdk-007 | `collectRec` casts | high | sdk-005 | co-land sdk-005+sdk-007 |
| 077 | sdk-006 | branded `ParsedQuery` | high | sdk-005 |  |
| 078 | sdk-004 | `find_form` Measure (+ dummy Var op) | high | — | ABI `has_over` with sdk-008 |
| 079 | sdk-008 | ABI `has_over` + marshal parse | high | sdk-004,sdk-006 | ABI `has_over` with sdk-004 |
| 080 | sdk-010 | interior polarity bool | med | — |  |
| 081 | sdk-011 | tag-plus-all-payloads IR | med | sdk-001,sdk-002 |  |
| 082 | sdk-009 | wildcard as `absent` | med | sdk-011 |  |
| 083 | sdk-012 | sugar caps | med | sdk-001 |  |
| 084 | sdk-013 | condition trees | med | sdk-011 |  |
| 085 | sdk-014 | `ParsedRule` sum | med | — | co-land sdk-014+sdk-015+sdk-027+sdk-029+sdk-030 |
| 086 | sdk-015 | param style two bools | med | — | co-land sdk-014+sdk-015+sdk-027+sdk-029+sdk-030 |
| 087 | sdk-027 | `query!` `HeadTerm::Agg` `over: Option` | med | sdk-004,sdk-008,sdk-014,sdk-015 | co-land sdk-014+sdk-015+sdk-027+sdk-029+sdk-030 |
| 088 | sdk-029 | `query!` interior-atom style `Option<bool>` | low | sdk-027 | co-land sdk-014+sdk-015+sdk-027+sdk-029+sdk-030 |
| 089 | sdk-030 | `query!` diagnostics still say "predicate" | med | sdk-027,sdk-029 | co-land sdk-014+sdk-015+sdk-027+sdk-029+sdk-030 |
| 090 | sdk-016 | `isQueryValue` forgets | med | sdk-006 |  |
| 091 | sdk-017 | `CmpData.mask` | med | — |  |
| 092 | sdk-021 | empty-interiors dummy array | low | — |  |
| 093 | sdk-022 | SDK comment vocabulary | low | sdk-005 |  |
| 094 | sdk-028 | violation / statement-slot optionals | med | — |  |
| 095 | sdk-018 | compile-fail suite | med | sdk-001,sdk-004,sdk-005,sdk-012,sdk-013 | compile-fail suite; last in SDK query |
| 096 | schema-001 | sealed `Relation.extension: Option` | high | — | one commit with 002; 002 co-lands 006 |
| 097 | schema-002 | `KeyStatement` flag product → `KeyForm` | high | schema-001 | co-lands with schema-006 |
| 098 | schema-006 | dual `fresh_row` coordinates | med | schema-002 | co-lands with schema-002 |
| 099 | schema-003 | capacity tails as sidecar Options | high | — | co-land schema-003+schema-008 |
| 100 | schema-008 | sealed `hi: Option<Bound>` (`*` as absence) | med | schema-003 | co-land schema-003+schema-008 |
| 101 | schema-004 | capacity reuses containment `Enforcement` | med | schema-001 |  |
| 102 | schema-005 | `IntervalTail.width: Option` | med | schema-001 |  |
| 103 | schema-007 | sealed `mirror` vs render Option holes | med | schema-001 |  |
| 104 | schema-009 | `SealedField.declared: Option` | med | schema-001 |  |
| 105 | schema-011 | containment `source_tail` sidecar | med | — | independent (IntervalTail is schema-005) |
| 106 | sdk-023 | C++ `relation_data` closed flag + leftover `closed_info` | high | — | co-land sdk-023+sdk-024 |
| 107 | sdk-024 | schema-lane tag-plus-all-payloads | med | sdk-023 | co-land sdk-023+sdk-024 |
| 108 | sdk-025 | schema sugar caps (`max_closed_handles = 8`) | med | sdk-023 |  |
| 109 | sdk-026 | `==` flattened to `bidirectional: bool` | med | sdk-024 |  |
| 110 | store-002 | point-read path is `is_closed × fresh_row × U-tree` | med | schema-001,schema-002 |  |
| 111 | image-001 | `closed_slots: Box<[Option<u32>]>` | high | schema-001 |  |
| 112 | exec-001 | Agg Count as `over_slot: None` | high | — | one commit with exec-002; Count-as-None |
| 113 | exec-002 | `DedupRegime` flattened to four fields | high | exec-001 | rides exec-001 |
| 114 | exec-003 | sink scan/skip as bools + `unreachable!` | med | — |  |
| 115 | exec-004 | `Executor.pipe: Option` take/put | med | — |  |
| 116 | exec-005 | `carried_col` Option-padded reverse index | med | exec-004 |  |
| 117 | exec-006 | `KeyProbePlan.statement: Option` | med | — |  |
| 118 | exec-007 | `batch_sources` / `scan_sources` Option holes | med | exec-001 |  |
| 119 | exec-008 | `LeafPrecompute.single: bool` | med | — |  |
| 120 | exec-009 | `SelectionLevel.set: bool` | med | — |  |
| 121 | exec-010 | stop flags product (`all_cancelled` + `poison`) | med | — |  |
| 122 | exec-011 | `row_fold_only: bool` | low | exec-001 |  |
| 123 | exec-012 | `cover_choice(..., exact: bool)` | low | — |  |
| 124 | exec-017 | `PipeTables.absorb` Option is Root vs Node | med | exec-004,exec-010 |  |
| 125 | plan-001 | `FoldedMark` discards parsed σ | med | — |  |
| 126 | plan-002 | two `pinned_fields`; they disagree | med | — |  |
| 127 | plan-003 | `PlanOccurrence::relation()` panics on Interior | med | engine-017,plan-001 |  |
| 128 | store-001 | `FactOp` one product for insert and delete | med | — |  |
| 129 | store-003 | `Environment` modes as Option pair | med | — |  |
| 130 | store-005 | `CommitReport { changed, new_generation }` | low | — |  |
| 131 | image-002 | `Const` too wide; `ResolvedWordSource::Var` then `unreachable!` | med | — |  |
| 132 | image-003 | `View::image` / `position_at` panic on `Unbound` | low | — |  |
| 133 | image-004 | `TransientImage.image: Option` | low | — |  |
| 134 | err-001 | `RenderedViolation` tag-plus-payloads | med | — |  |
| 135 | err-002 | `Violation::Functionality.incumbent: Option` | med | — |  |
| 136 | err-003 | `Violations.cited` empty until `attach_cited` | med | — |  |
| 137 | err-004 | `MalformedValue(&'static str)` catch-all | med | — |  |
| 138 | err-005 | `TraceEvent` `dur_ns == 0` ⇒ point event | low | — |  |
| 139 | err-006 | "program" vocabulary in error/obs | low | — |  |
| 140 | bench-001 | two JSON emitters (CQuery vs Query) | high | — | encoder twin of lean-008; either order if corpus identical |
| 141 | bench-002 | irgen never draws interiors/rec | high | — |  |
| 142 | bench-003 | stamp/fuzz/seeded/contradict call only `random_query` | high | engine-020 | after engine-020; INDEX broke 003↔004 |
| 143 | bench-004 | CQ-shaped consumers walk `query.rules` only | med | engine-020,bench-003 | after 020/003 (INDEX; not before 020) |
| 144 | bench-005 | two sqlite expressibility gates | med | engine-021 |  |
| 145 | bench-006 | derived CTEs still `p{id}` | med | engine-021 |  |
| 146 | bench-007 | closure lane teaches delta-variants | med | — |  |
| 147 | bench-008 | `exec_digest` is a CQ stats consumer | med | engine-012 |  |
| 148 | bench-009 | "program" names a Query in the remaining crate | low | — |  |
| 149 | docs-002 | main as anonymous predicate | high | engine-041 | after engine-041 |
| 150 | docs-004 | "today's query" embedding (IR) | med | lean-001 |  |
| 151 | docs-006 | "not a Tarjan condensation" | med | lean-002 |  |
| 152 | docs-012 | "CQuery arm" | high | lean-008 | after lean-008 |
| 153 | docs-015 | "today's query" on prepare | med | lean-001 |  |
| 154 | docs-017 | `predicate()` buffer authority | high | engine-041 | after engine-041 |
| 155 | docs-021 | README OPEN items in SCC coords | med | lean-002 |  |
| 156 | docs-025 | "zero stratification impact" | high | lean-002 |  |
| 157 | docs-027 | conformance two types | high | lean-008 | after lean-008 |
| 158 | docs-028 | "never idb" | med | docs-027 | same file as docs-027 |
| 159 | docs-030 | docs still teach `DeltaVariant` / `PreparedBody::Empty` | med | engine-007,engine-023,docs-011 | after engine-007/023 |

## Cycles broken

Do not invent an order through a cycle; INDEX cluster grouping wins.

1. **lean-001 ↔ lean-002.** Issue 001 lists 002 as a dependency; they are one commit. Queue: **001 then 002** adjacent (INDEX: 002 may be immediately after 001).
2. **engine-020 ↔ bench-004** (and **bench-003 ↔ bench-004**). engine-020's issue depends on bench-004; bench-004's issue says land *before* 020; bench-003's issue depends on 004. INDEX: engine-020 independent; bench-003 after 020; bench-004 after 020/003. Queue uses INDEX: **020 → 003 → 004**. Dropped 020's issue-file deps on 004/005/021 and 003's deps on 004/005.

No other cycles. sdk-023 ∥ schema-001 and exec-001 ∥ sdk-004/008/027 stay parallel (no fake edges). lean-008 precedes docs-012/027; bench-001 (encoder twin) is later in the bench block.

## All issues

### lean (24)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 021 | lean-001 | `Query` product with `Option Rec` → inductive sum | high | FIXED | lean-002 (one change) |
| 022 | lean-002 | untyped `Rec` → typed `LinearRec` | high | FIXED | none (with lean-001) |
| 023 | lean-003 | dual rec-identity coordinates | high | FIXED | lean-001, 002 |
| 024 | lean-004 | unspent `WellFormed` bundle | high | FIXED | lean-001 |
| 025 | lean-005 | two denotations | high | FIXED | lean-001 |
| 026 | lean-006 | orphan arity fields | high | FIXED | lean-001, 002 |
| 027 | lean-007 | staged interior eval → fold | med | FIXED | lean-001 (file conflict) |
| 028 | lean-008 | two decoders / `CQuery` | med | FIXED | lean-001, 002 |
| 029 | lean-009 | `allRules` flatten | med | FIXED | lean-001 |
| 030 | lean-010 | naive iterators in the meaning | med | FIXED | lean-002 |
| 031 | lean-011 | `recDom` / idb vocabulary | med | FIXED | lean-002 |
| — | lean-012 | Option-rec flag | med | DUPLICATE(lean-001) | — |
| — | lean-013 | total InteriorEnv | med | WONTFIX (§C5) | — |
| — | lean-014 | `edbOnly` flag | med | DUPLICATE(lean-004) | — |
| 032 | lean-015 | `odd_not_stratified` name | low | FIXED | lean-002 |
| — | lean-016 | RewriteStep dummy arity | low | DUPLICATE(lean-005) | — |
| — | lean-017 | selfCount unpack | low | DUPLICATE(lean-002) | — |
| — | lean-018 | empty-rules rec-answer surprise | low | DUPLICATE(lean-001) | — |
| 001 | lean-019 | Bridge cites deleted `translate/program.rs` | high | FIXED | **first** |
| 036 | lean-020 | "rec SCC" in Lean comments | low | FIXED | after 001/002 |
| 033 | lean-021 | Membership/key-probe collapse `AtomSource` to `RelId ⟨0⟩` | high | FIXED | 001, 002 (006 for field width) |
| 034 | lean-022 | Plan denotes `edbEnv` | med | FIXED | 005 |
| 035 | lean-023 | `HeadSlot` fourth head-shape encoding | med | FIXED | 008 |
| 037 | lean-024 | rest-of-tree Lean "program" comments | low | FIXED | after 001/005 (with 020) |

### engine (41)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 038 | engine-001 | interiors beside `PreparedBody` → pipeline sum | high | FIXED | none (w/ 002, 015, 023) |
| 039 | engine-002 | `PreparedRule::Recursive` → `RecArm` | high | FIXED | 001 (co-lands) |
| 054 | engine-003 | rec id `len()` pun; store once | high | FIXED | 005, 016 |
| 044 | engine-004 | empty rec arms on the witness | high | FIXED | 005 |
| 042 | engine-005 | witness sum + `self_occ` | high | FIXED | none |
| 043 | engine-006 | `Option<Predicate>` sealing holes | high | FIXED | none (coord. w/ 005) |
| 045 | engine-007 | `DeltaVariant` → `prepare_rec_arm` | high | FIXED | 002 |
| 046 | engine-008 | execute/profile fork | high | FIXED | 001 |
| 047 | engine-009 | `run_reach` re-matches | high | FIXED | 001 |
| 051 | engine-010 | rec-bind Option soup | high | FIXED | 013, 001 |
| 049 | engine-011 | zombie Program vocab + false invariant | high | FIXED | 001, 012 |
| 048 | engine-012 | `ExecutionStats` product | high | FIXED | 001 |
| 050 | engine-013 | one `DerivedImages` + PingPong | med | FIXED | 001 |
| 052 | engine-014 | `rounds_budget` on Reach only | med | FIXED | 001 |
| 040 | engine-015 | main not in the driver | med | FIXED | 001 (co-lands) |
| 053 | engine-016 | prepare `is_some`/`expect` | med | FIXED | 005 |
| 056 | engine-017 | `edb().is_none()` bind | med | FIXED | 010, 018 |
| 055 | engine-018 | planning-floor alias | med | FIXED | 007 |
| 057 | engine-019 | naive oracle flags | med | FIXED | none |
| 058 | engine-020 | querygen side entry | med | FIXED | none |
| 059 | engine-021 | translator two-flag gate | med | FIXED | none |
| 060 | engine-022 | rec parser three walks | med | FIXED | 005, 004 |
| 041 | engine-023 | `Empty` not a variant | med | FIXED | 001 (co-lands) |
| 061 | engine-024 | dual ray-probe loops | med | FIXED | 001, 013 |
| 062 | engine-025 | accessor forest | med | FIXED | 001, 002 |
| 063 | engine-026 | rule enum per sink | med | FIXED | 002 |
| 064 | engine-027 | nonempty witness lists | med | FIXED | 005, 004 |
| — | engine-028 | derived-count restated | med | DUPLICATE(engine-003) | — |
| 065 | engine-029 | `unit_labels` as mode bit | med | FIXED | 012, 001 |
| 066 | engine-030 | dead `normalize()` | med | FIXED | none |
| 067 | engine-031 | key-probe rematch → `Ok(())` | med | FIXED | 001, 008 |
| 068 | engine-032 | `occ_images` Option slots | med | FIXED | 013, 010 |
| 069 | engine-033 | `predicate p{id}` strings | low | FIXED | 012/029 (one bump) |
| 070 | engine-034 | `ground_program` → `ground_main` | low | FIXED | none |
| — | engine-035 | "program" in tests | low | DUPLICATE(engine-011) | — |
| — | engine-036 | `_either_sink_marker` | low | DUPLICATE(engine-026) | — |
| — | engine-037 | `Query::single` is correct | low | WONTFIX | — |
| — | engine-038 | stats/JSON drift | low | DUPLICATE(engine-012) | — |
| — | engine-039 | `delta: Option<OccId>` | low | DUPLICATE(engine-007) | — |
| — | engine-040 | ping-pong "Size 1" | low | DUPLICATE(engine-013) | — |
| 071 | engine-041 | `Predicate` → `Signature` | low | FIXED | 005/006 |

### sdk (30)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 072 | sdk-001 | C++ `query_value` phase machine | high | FIXED | none (w/ 002) |
| 073 | sdk-002 | one C++ IR | high | FIXED | 001 (co-lands) |
| 074 | sdk-003 | `wire_atom` bool + both ids | high | FIXED | none |
| 078 | sdk-004 | `find_form` Measure (+ dummy Var op) | high | FIXED | none |
| 075 | sdk-005 | TS `QueryStart` phase | high | FIXED | none |
| 077 | sdk-006 | branded `ParsedQuery` | high | FIXED | 005 |
| 076 | sdk-007 | `collectRec` casts | high | FIXED | none (file w/ 005) |
| 079 | sdk-008 | ABI `has_over` + marshal parse | high | FIXED | coord. 004, 006 |
| 082 | sdk-009 | wildcard as `absent` | med | FIXED | 011 |
| 080 | sdk-010 | interior polarity bool | med | FIXED | none |
| 081 | sdk-011 | tag-plus-all-payloads IR | med | FIXED | 001/002 |
| 083 | sdk-012 | sugar caps | med | FIXED | 001 |
| 084 | sdk-013 | condition trees | med | FIXED | 011 |
| 085 | sdk-014 | `ParsedRule` sum | med | FIXED | none |
| 086 | sdk-015 | param style two bools | med | FIXED | none |
| 090 | sdk-016 | `isQueryValue` forgets | med | FIXED | 006 |
| 091 | sdk-017 | `CmpData.mask` | med | FIXED | none |
| 095 | sdk-018 | compile-fail suite | med | FIXED | 001/004/005/012/013 — last |
| — | sdk-019 | `derived_tables` rec flag | med | DUPLICATE(sdk-001) | — |
| — | sdk-020 | dummy Var `op` | low | DUPLICATE(sdk-004) | — |
| 092 | sdk-021 | empty-interiors dummy array | low | FIXED | none |
| 093 | sdk-022 | SDK comment vocabulary | low | FIXED | after 005 |
| 106 | sdk-023 | C++ `relation_data` closed flag + leftover `closed_info` | high | FIXED | none (w/ 024) |
| 107 | sdk-024 | schema-lane tag-plus-all-payloads | med | FIXED | 023 |
| 108 | sdk-025 | schema sugar caps (`max_closed_handles = 8`) | med | FIXED | 023 |
| 109 | sdk-026 | `==` flattened to `bidirectional: bool` | med | FIXED | 024 |
| 087 | sdk-027 | `query!` `HeadTerm::Agg` `over: Option` | med | FIXED | coord. 004/008 |
| 094 | sdk-028 | violation / statement-slot optionals | med | FIXED | none |
| 088 | sdk-029 | `query!` interior-atom style `Option<bool>` | low | FIXED | with 014/015/027 |
| 089 | sdk-030 | `query!` diagnostics still say "predicate" | med | FIXED | with 014/015/027/029 |

### docs (30)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 002 | docs-001 | "multi-rule programs" (20-query-ir) | high | FIXED | none |
| 149 | docs-002 | main as anonymous predicate | high | FIXED | engine-041 |
| 003 | docs-003 | rec as SCC | high | FIXED | none |
| 150 | docs-004 | "today's query" embedding (IR) | med | FIXED | none |
| 004 | docs-005 | deleted cap names (IR) | med | FIXED | none |
| 151 | docs-006 | "not a Tarjan condensation" | med | FIXED | none |
| 005 | docs-007 | fuel hyphen ghost | med | FIXED | none |
| 006 | docs-008 | "no program renderer" | med | FIXED | none |
| 007 | docs-009 | "former named-head sneak" | med | FIXED | none |
| 008 | docs-010 | one-sink contradiction | med | FIXED | none |
| 009 | docs-011 | "program" in 40-execution | high | FIXED | none |
| 152 | docs-012 | "CQuery arm" | high | FIXED | lean-008 |
| 010 | docs-013 | "program whose every disjunct vanishes" | high | FIXED | none |
| 011 | docs-014 | "cte-list" emission | low | FIXED | none |
| 153 | docs-015 | "today's query" on prepare | med | FIXED | none |
| 012 | docs-016 | "data-modifying CTEs" (API) | med | FIXED | none |
| 154 | docs-017 | `predicate()` buffer authority | high | FIXED | engine-041 |
| 013 | docs-018 | ForeignPreparedQuery horizon | med | FIXED | none |
| 014 | docs-019 | cpp-lowering caps / today's query | med | FIXED | none |
| 015 | docs-020 | output-last denial | high | FIXED | none |
| 155 | docs-021 | README OPEN items in SCC coords | med | FIXED | none |
| 016 | docs-022 | cookbook CTE | med | FIXED | none |
| 017 | docs-023 | cookbook `Program` relation | low | FIXED | none |
| 018 | docs-024 | `AggregateInteriorPredicate` | high | FIXED | none |
| 156 | docs-025 | "zero stratification impact" | high | FIXED | none |
| 019 | docs-026 | "idb re-grounding tax" | high | FIXED | none |
| 157 | docs-027 | conformance two types | high | FIXED | lean-008 |
| 158 | docs-028 | "never idb" | med | FIXED | none |
| 020 | docs-029 | cookbook "not a second SCC" | low | FIXED | none |
| 159 | docs-030 | docs still teach `DeltaVariant` / `PreparedBody::Empty` | med | FIXED | engine-007, 023 |

### plan (8)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 125 | plan-001 | `FoldedMark` discards parsed σ | med | FIXED | none |
| 126 | plan-002 | two `pinned_fields`; they disagree | med | FIXED | none |
| 127 | plan-003 | `PlanOccurrence::relation()` panics on Interior | med | FIXED | 001, engine-017 |
| — | plan-004 | fj/validate "no Interior" claim | med | DUPLICATE(engine-030) | — |
| — | plan-005 | planning-floor alias | med | DUPLICATE(engine-018) | — |
| — | plan-006 | plan-side `edb() else` forest | med | DUPLICATE(engine-017) | — |
| — | plan-007 | `ground_program` name | low | DUPLICATE(engine-034) | — |
| — | plan-008 | Program vocabulary in plan | low | DUPLICATE(engine-011) | — |

### exec (17)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 112 | exec-001 | Agg Count as `over_slot: None` | high | FIXED | none (w/ 002) |
| 113 | exec-002 | `DedupRegime` flattened to four fields | high | FIXED | none (rides 001) |
| 114 | exec-003 | sink scan/skip as bools + `unreachable!` | med | FIXED | none |
| 115 | exec-004 | `Executor.pipe: Option` take/put | med | FIXED | none |
| 116 | exec-005 | `carried_col` Option-padded reverse index | med | FIXED | 004 |
| 117 | exec-006 | `KeyProbePlan.statement: Option` | med | FIXED | none |
| 118 | exec-007 | `batch_sources` / `scan_sources` Option holes | med | FIXED | 001 |
| 119 | exec-008 | `LeafPrecompute.single: bool` | med | FIXED | none |
| 120 | exec-009 | `SelectionLevel.set: bool` | med | FIXED | none |
| 121 | exec-010 | stop flags product (`all_cancelled` + `poison`) | med | FIXED | none |
| 122 | exec-011 | `row_fold_only: bool` | low | FIXED | 001 |
| 123 | exec-012 | `cover_choice(..., exact: bool)` | low | FIXED | none |
| — | exec-013 | Program vocabulary in exec | high | DUPLICATE(engine-011) | — |
| — | exec-014 | unit-labels mode bit | med | DUPLICATE(engine-029) | — |
| — | exec-015 | `predicate p{id}` strings | low | DUPLICATE(engine-033) | — |
| — | exec-016 | delta-variant comments | high | DUPLICATE(engine-007) | — |
| 124 | exec-017 | `PipeTables.absorb` Option is Root vs Node | med | FIXED | 004, 010 |

### schema (11)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 096 | schema-001 | sealed `Relation.extension: Option` | high | FIXED | none (foundation) |
| 097 | schema-002 | `KeyStatement` flag product → `KeyForm` | high | FIXED | none (w/ 006) |
| 099 | schema-003 | capacity tails as sidecar Options | high | FIXED | none (w/ 008) |
| 101 | schema-004 | capacity reuses containment `Enforcement` | med | FIXED | none |
| 102 | schema-005 | `IntervalTail.width: Option` | med | FIXED | none |
| 098 | schema-006 | dual `fresh_row` coordinates | med | FIXED | 002 |
| 103 | schema-007 | sealed `mirror` vs render Option holes | med | FIXED | none |
| 100 | schema-008 | sealed `hi: Option<Bound>` (`*` as absence) | med | FIXED | 003 |
| 104 | schema-009 | `SealedField.declared: Option` | med | FIXED | none |
| — | schema-010 | descriptor `extension: Option` is hostile spelling | low | WONTFIX | — |
| 105 | schema-011 | containment `source_tail` sidecar | med | FIXED | none (IntervalTail is 005) |

### store (5)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 128 | store-001 | `FactOp` one product for insert and delete | med | FIXED | none |
| 110 | store-002 | point-read path is `is_closed × fresh_row × U-tree` | med | FIXED | schema-001, 002 |
| 129 | store-003 | `Environment` modes as Option pair | med | FIXED | none |
| — | store-004 | `ForeignPreparedQuery` essential identity | med | WONTFIX | — |
| 130 | store-005 | `CommitReport { changed, new_generation }` | low | FIXED | none |

### image (4)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 111 | image-001 | `closed_slots: Box<[Option<u32>]>` | high | FIXED | schema-001 |
| 131 | image-002 | `Const` too wide; `ResolvedWordSource::Var` then `unreachable!` | med | FIXED | none |
| 132 | image-003 | `View::image` / `position_at` panic on `Unbound` | low | FIXED | none |
| 133 | image-004 | `TransientImage.image: Option` | low | FIXED | none |

### err (6)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 134 | err-001 | `RenderedViolation` tag-plus-payloads | med | FIXED | none |
| 135 | err-002 | `Violation::Functionality.incumbent: Option` | med | FIXED | none |
| 136 | err-003 | `Violations.cited` empty until `attach_cited` | med | FIXED | none |
| 137 | err-004 | `MalformedValue(&'static str)` catch-all | med | FIXED | none (proposed C10) |
| 138 | err-005 | `TraceEvent` `dur_ns == 0` ⇒ point event | low | FIXED | none |
| 139 | err-006 | "program" vocabulary in error/obs | low | FIXED | none |

### bench (12)

| Seq | Id | Title | Sev | Status | Depends on |
|---|---|---|---|---|---|
| 140 | bench-001 | two JSON emitters (CQuery vs Query) | high | FIXED | none (twin of lean-008) |
| 141 | bench-002 | irgen never draws interiors/rec | high | FIXED | none |
| 142 | bench-003 | stamp/fuzz/seeded/contradict call only `random_query` | high | FIXED | engine-020 |
| 143 | bench-004 | CQ-shaped consumers walk `query.rules` only | med | FIXED | engine-020, bench-003 |
| 144 | bench-005 | two sqlite expressibility gates | med | FIXED | engine-021 |
| 145 | bench-006 | derived CTEs still `p{id}` | med | FIXED | engine-021 |
| 146 | bench-007 | closure lane teaches delta-variants | med | FIXED | none |
| 147 | bench-008 | `exec_digest` is a CQ stats consumer | med | FIXED | engine-012 |
| 148 | bench-009 | "program" names a Query in the remaining crate | low | FIXED | none |
| — | bench-010 | `.predicate()` accessors | low | DUPLICATE(engine-041) | — |
| — | bench-011 | closure `exec: None` profile skip | low | DUPLICATE(engine-011) | — |
| — | bench-012 | querygen tests `RecursiveVariant` tag | low | DUPLICATE(engine-020) | — |

## Green (every FIXED fix commit)

`bash scripts/check.sh` and `bash scripts/lean.sh` (after lean-019), plus the tree-local suites the issue names. Corpus unchanged. Locked names unchanged.
