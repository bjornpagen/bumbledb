# Issue index — the fix campaign ledger

Reconciliation against the wave-1 dumps: lean 18/18, engine 40/40,
sdks 21/21, docs 28/28 — every finding maps to exactly one file
below. Wave-2 additions: proc-01, eng-F41, lean-W1, sdk-22, sdk-23.
**Total: 112 files — 107 OPEN, 4 DUPLICATE, 1 REFUSED-BY-CONTRACT.**

Statuses flip to `FIXED(sha)` here AND in each issue file as fixes
land. The Gate (audit/README.md § Phase B) must be fully checked
before the first fix commit.

## Waves

- **Wave 0** — the sums + the red gate. `proc-01` lands first and
  alone (one line; un-reds `scripts/lean.sh` for everyone).
  Then two parallel workers: ENG-CORE (eng-F01…F07 as one coherent
  representation change per C2/C3) and LEAN-CORE (lean-H1 + lean-H2
  per C4).
- **Wave 1** — per-tree downstream, parallel within and across
  trees once that tree's wave 0 lands.
- **Wave 2** — SDKs. Independent of engine internals (C1 pins the
  boundary); may run in parallel with wave 1. Three workers:
  CPP (sdk-01,02,03,04,09,10,11,12,13,19,20,21,23 + sdk-08 with the
  bridge), TS (sdk-05,06,07,16,17,22), MACRO (sdk-14,15). sdk-18
  (compile-fail suite) closes the wave.
- **Wave 3** — docs, last (final names exist). One worker.

## Ledger

| id | title | sev | status | blocked-by | blocks | wave |
|---|---|---|---|---|---|---|
| proc-01 | Bridge census token → deleted translate/program.rs; lean gate RED | high | OPEN | — | all (gate) | 0 |
| lean-H1 | Query sum (cq/reach) | high | OPEN | — | H3 H4 H5 M1 M2 M3 M6 M8 L4 | 0 |
| lean-H2 | typed LinearRec | high | OPEN | — | H3 H6 M5 L1 L3 | 0 |
| lean-H3 | rec-identity dual coords (C5 split) | high | OPEN | H1 H2 | — | 1 |
| lean-H4 | WellFormed bundle death (C5 split) | high | OPEN | H1 M1 | — | 1 |
| lean-H5 | one rule-list theory | high | OPEN | H1 | L2 | 1 |
| lean-H6 | orphan arity (C5 split) | high | OPEN | H1 H2 | — | 1 |
| lean-M1 | interiors fold, no Nat stage | med | OPEN | H1 | H4 | 1 |
| lean-M2 | one decoder / one atom grammar | med | OPEN | H1 | — | 1 |
| lean-M3 | allRules + inversions die | med | OPEN | H1 | — | 1 |
| lean-M4 | iterators leave meaning module | med | OPEN | — | — | 1 |
| lean-M5 | recDom drops idb coordinate | med | OPEN | H2 | — | 1 |
| lean-M6 | Option-rec flag-site cleanup | med | OPEN | H1 | — | 1 |
| lean-M7 | total InteriorEnv | med | REFUSED-BY-CONTRACT(C5) | — | — | — |
| lean-M8 | edbOnly / hostile-plain prose | med | OPEN | H1 | — | 1 |
| lean-L1 | odd_not_stratified name | low | OPEN | H2 | — | 1 |
| lean-L2 | RewriteStep orphan Nat | low | OPEN | H5 | — | 1 |
| lean-L3 | selfCount guards | low | DUPLICATE(lean-H2) | H2 | — | — |
| lean-L4 | empty-rules theorem restated | low | OPEN | H1 | — | 1 |
| lean-W1 | lean prose "program" sweep | low | OPEN | H1 H5 | — | 1 |
| eng-F01 | PreparedPipeline sum | high | OPEN | — | F08 F09 F12 F14 F15 F23 F24 F26 F29 F31 | 0 |
| eng-F02 | RecArm type; Recursive variant dies | high | OPEN | — | F07 F25 F26 | 0 |
| eng-F03 | rec id / derived count stored once (+F28) | high | OPEN | — | F17 | 0 |
| eng-F04 | witness rec arms nonempty | high | OPEN | — | F22 | 0 |
| eng-F05 | witness sum + self_occ | high | OPEN | — | F16 F22 | 0 |
| eng-F06 | sealing slices, no Option holes | high | OPEN | — | — | 0 |
| eng-F07 | DeltaVariant death | high | OPEN | F02 | F18 F39 | 0 |
| eng-F08 | one execute/profile protocol | high | OPEN | F01 | F29 F31 | 1 |
| eng-F09 | run_reach single match | high | OPEN | F01 | — | 1 |
| eng-F10 | DerivedBind sum; idb_* renames | high | OPEN | F13 | — | 1 |
| eng-F11 | load-bearing zombie vocab / false invariants | high | OPEN | F01 | — | 1 |
| eng-F12 | ExecutionStats sum (+F38) | high | OPEN | F01 | F29 | 1 |
| eng-F13 | one DerivedImages layout | med | OPEN | F01 | F10 F32 F40 | 1 |
| eng-F14 | rounds_budget on Reach arm | med | OPEN | F01 | — | 1 |
| eng-F15 | main out of ReachDriver | med | OPEN | F01 | — | 1 |
| eng-F16 | prepare matches witness sum | med | OPEN | F05 | — | 1 |
| eng-F17 | bind roles, not edb().is_none() | med | OPEN | F03 | — | 1 |
| eng-F18 | selectivity floors on occurrence | med | OPEN | F07 | — | 1 |
| eng-F19 | naive DerivedWorld | med | OPEN | — | — | 1 |
| eng-F20 | querygen shape sum | med | OPEN | — | — | 1 |
| eng-F21 | translator one path | med | OPEN | — | — | 1 |
| eng-F22 | one rec parser (3 walks → 1) | med | OPEN | F04 F05 | — | 1 |
| eng-F23 | Empty is not a variant | med | OPEN | F01 | — | 1 |
| eng-F24 | one ray-probe loop | med | OPEN | F01 | — | 1 |
| eng-F25 | accessor forest deletes | med | OPEN | F02 | — | 1 |
| eng-F26 | rule enum per sink discipline | med | OPEN | F01 F02 | F36 | 1 |
| eng-F27 | nonempty witness lists | med | OPEN | — | — | 1 |
| eng-F28 | derived-count restated | med | DUPLICATE(eng-F03) | F03 | — | — |
| eng-F29 | ReportBody sum | med | OPEN | F01 F08 F12 | — | 1 |
| eng-F30 | dead normalize() + false claim | med | OPEN | — | — | 1 |
| eng-F31 | key-probe lane parsed once | med | OPEN | F01 F08 | — | 1 |
| eng-F32 | occ_images dense, no Options | med | OPEN | F13 | — | 1 |
| eng-F33 | render/display vocab | low | OPEN | F03 F11 | docs | 1 |
| eng-F34 | ground_main rename | low | OPEN | — | — | 1 |
| eng-F35 | engine prose sweep (wave-2 expanded) | low | OPEN | F01 F07 F10 F11 F34 F41 | — | 1 |
| eng-F36 | either-sink marker deletion | low | OPEN | F26 | — | 1 |
| eng-F37 | Query::single recorded non-violation | low | DUPLICATE(eng-F01, eng-F08) | — | — | — |
| eng-F38 | stats/JSON drift | low | DUPLICATE(eng-F12) | F12 | — | — |
| eng-F39 | prepare_rec_arm entry | low | OPEN | F07 | — | 1 |
| eng-F40 | PingPong layout | low | OPEN | F13 | — | 1 |
| eng-F41 | Predicate → Signature rename (wave-2) | low | OPEN | F05 | docs-F02 docs-F17 | 1 |
| sdk-01 | C++ phase machine | high | OPEN | — | 02 18 19 | 2 |
| sdk-02 | one C++ IR | high | OPEN | 01 | — | 2 |
| sdk-03 | wire_atom sum | high | OPEN | — | — | 2 |
| sdk-04 | find_form Measure | high | OPEN | — | 08 20 | 2 |
| sdk-05 | TS phase in the type | high | OPEN | — | 18 | 2 |
| sdk-06 | branded ParsedQuery / find sum | high | OPEN | — | — | 2 |
| sdk-07 | collectRec one assignment | high | OPEN | — | — | 2 |
| sdk-08 | ABI has_over death + marshal parse | high | OPEN | 04 | — | 2 |
| sdk-09 | wildcard = absence | med | OPEN | — | — | 2 |
| sdk-10 | interior polarity sum | med | OPEN | — | — | 2 |
| sdk-11 | variant builder IR | med | OPEN | — | — | 2 |
| sdk-12 | sugar caps die | med | OPEN | — | — | 2 |
| sdk-13 | condition trees in dialect | med | OPEN | — | — | 2 |
| sdk-14 | ParsedRule sum | med | OPEN | — | — | 2 |
| sdk-15 | param style sum | med | OPEN | — | — | 2 |
| sdk-16 | isQueryValue honest | med | OPEN | — | — | 2 |
| sdk-17 | CmpData op sum | med | OPEN | — | — | 2 |
| sdk-18 | compile-fail suite closes wave | med | OPEN | 01 04 05 13 | — | 2 |
| sdk-19 | derived_tables if constexpr | med | OPEN | 01 | — | 2 |
| sdk-20 | dummy op filler dies | low | OPEN | 04 | — | 2 |
| sdk-21 | array<T,0> | low | OPEN | — | — | 2 |
| sdk-22 | TS prose sweep (wave-2) | low | OPEN | — | — | 2 |
| sdk-23 | C++ prose sweep (wave-2) | low | OPEN | — | — | 2 |
| docs-F01 | multi-rule programs → queries | high | OPEN | — | — | 3 |
| docs-F02 | main ≠ predicate | high | OPEN | eng-F41 | — | 3 |
| docs-F03 | rec ≠ SCC | high | OPEN | — | — | 3 |
| docs-F04 | today's-query embedding (IR) | med | OPEN | — | — | 3 |
| docs-F05 | deleted cap names | med | OPEN | — | — | 3 |
| docs-F06 | Tarjan denial | med | OPEN | lean-H2 | — | 3 |
| docs-F07 | fuel hyphen ghost | med | OPEN | — | — | 3 |
| docs-F08 | program renderer denial | med | OPEN | — | — | 3 |
| docs-F09 | former-sneak history | med | OPEN | — | — | 3 |
| docs-F10 | one sink per list | med | OPEN | — | — | 3 |
| docs-F11 | execution chapter program ×8 | high | OPEN | — | — | 3 |
| docs-F12 | CQuery arm | high | OPEN | lean-M2 | — | 3 |
| docs-F13 | empty-union program | high | OPEN | — | — | 3 |
| docs-F14 | cte-list residue | low | OPEN | — | — | 3 |
| docs-F15 | today's-query (API) | med | OPEN | — | — | 3 |
| docs-F16 | data-modifying CTE (API) | med | OPEN | — | — | 3 |
| docs-F17 | predicate() buffer authority | high | OPEN | eng-F41 | — | 3 |
| docs-F18 | ForeignPreparedQuery horizon | med | OPEN | — | — | 3 |
| docs-F19 | cpp-lowering caps/today's-query | med | OPEN | — | — | 3 |
| docs-F20 | output-last denial | high | OPEN | — | — | 3 |
| docs-F21 | OPEN items in SCC coords | med | OPEN | — | — | 3 |
| docs-F22 | cookbook CTE import | med | OPEN | — | — | 3 |
| docs-F23 | cookbook Program relation (+ts/cpp fixtures) | low | OPEN | — | — | 3 |
| docs-F24 | stale AggregateInteriorPredicate | high | OPEN | — | — | 3 |
| docs-F25 | zero stratification impact | high | OPEN | lean-H2 | — | 3 |
| docs-F26 | idb re-grounding tax | high | OPEN | — | — | 3 |
| docs-F27 | conformance two types | high | OPEN | lean-M2 | — | 3 |
| docs-F28 | never-idb negation | med | OPEN | — | — | 3 |

## Definition of green (every fix commit)

`bash scripts/check.sh` AND `bash scripts/lean.sh` (after proc-01
un-reds it), plus the tree-local suites each issue names. Assertions
are never weakened; corpus JSON never regenerates; locked names
(`DerivedBudgetExceeded`, `set_derived_budget`,
`DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`) never move.
