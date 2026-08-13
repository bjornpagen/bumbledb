# 04 — Bindings, docs, oracles, refusals

Engine IR is `Query` (`01`, `03`). Sugar lowers to that IR. Program vocabulary is deleted on every surface in the same Rust-phase commit as the IR, not "later in the SDK."

Grouping law, error names, `MAX_RULES` pooling, no interior-count cap: `01-language.md` is normative. This file does not invent a second spelling and does not specify the language by writing SQLite.

SQLite `WITH [RECURSIVE]` appears only in the **translator** section below. It is a lossy image of this cut's fragment.

## `query!` (`crates/bumbledb-query-macros`)

Module-doc grammar becomes `01-language.md`'s block (`interior` / `recursive`, not `with` / `with recursive`). Expansion:

- All-bare → `Query { interiors: vec![], rec: None, head, rules }` (today's lowering plus two empty fields).
- `interior` / `recursive` present → fill `interiors` / `rec`; bare rules are main. **Never emit `ir::Program`.**
- Named head without the keywords → `compile_error!` at the name, telling the author to write `interior` or `recursive`.

Classification (rec arms): a `recursive pred` line is a rec arm iff its body has an **atom** (positive or negated, either spelling) naming `pred`. Else base. Consecutive same-name lines union; non-consecutive reuse is a compile error (`01` table).

Delete the predicate-table path that buckets named heads into `PredicateDef` and returns `Program { predicates, output }` (`lib.rs` around the `emitter.predicates.is_empty()` split). Delete `PredId` from generated code.

Goldens (`bumbledb-query/tests/notation.rs`, `notation-corpus`): `program-recursion` recut to `recursive` text; `named_head_program_lowers_to_the_exact_ir` becomes a compile-fail fixture (named head, no keyword). Round-trip: `render(lower(text))` against the new renderer.

Cookbook tests (`bumbledb-query/tests/cookbook.rs` recipes 24–25): native forms use `recursive`. Host-loop forms are unchanged (they were already plain queries). Primer-shaped `reach(x,x)` is a cookbook or engine lock, not a third dialect.

## TypeScript (`ts/src/query/predicate.ts` and friends)

Delete `program()`, `ProgramScope` (`.rec` / `.output`), `p.rec("reach")`, `p.output(...)`. The sealed program-as-query-value trick goes with them. (There is no `recursiveProgram` in the tree — nothing to delete under that name; do not invent it to kill it.)

Add on the existing query builder (the `Query` value / scope that already builds rules):

```ts
q.interior("mid", ...builders)   // one Interior; builders are the union
q.recursive("reach", {
  base: [...builders],           // arrays — one callback is not "exactly one arm"
  rec:  [...builders],
})
```

Pick this shape, not `rec.rule` plus a later `output`. Base and rec are unmissable. Multiple `interior` calls append `Interior`s in call order (DAG order). A second `interior("mid", ...)` is a construction error (names unique; TS does not consecutive-union). A second `recursive` throws at construction. `interior` after `recursive` throws. `interior` / `recursive` after a main rule has been added throws. The builder returns the ordinary query value; `db.prepare(q)` lowers to `QueryIr` with `interiors` / `rec` fields.

**Grouping law** (`01-language.md` table): names unique; multiple rules = many builders on **one** `interior` / `recursive`. TS `base`/`rec` are arrays. C++ two tagged packs. `query!` classifies base vs rec by whether an atom names `pred`.

Lowering (`ts/src/query/lower.ts` §4.2 of `75-cpp-lowering.md`): recs-in-declaration-order + output-last **dies**. Emit `interiors[]`, optional `rec { head, base, rec }`, `head`, `rules`. Param registry: interiors in order, then rec base, then rec arms, then main — first use still mints `ParamId`. IDB-head-order bindings become interior-head-order.

Wire (`ts/src/native.ts`): `ProgramIr` / `PredicateDefIr` / `{kind:"idb",pred}` die. `QueryIr` grows `interiors`, `rec`. `{kind:"interior",interior}`. napi `dbPrepare(db, program)` → `dbPrepare(db, query)` only. `ts/crate/src/marshal.rs` `program_in` → `query_in` with the new fields. Rule builders: `.idb(rec, binds)` → `.interior(name, binds)`. Negation keeps its one spelling: today `r.not(rec, …)` lowers to wire cond `"notIdb"`; after, `r.not(name, …)` lowers to `"notInterior"`. There is no `.notIdb` method to rename and no `.notInterior` method to add.

Tests: `ts/test/query.test.ts` recursion fences, `destructure-kernel` rec ports, `answers-named-orderable-ban` rec-head plumb, `psi-query-atoms` `output: 0` — recut to `interior` / `recursive`. Construction errors for two recs. TS used `program()`; deleting it is the named-without-keyword fence.

**Primer cycle detector.** `requiresCycleQuery` recuts 1:1: `q.recursive("reach", { base: [edge], rec: [step with extra EDB + one reach atom] })` plus a main rule `grp ⋈ reach(x,x)`. Empty answers = DAG. Do not add a named interior of `reach` for the diagonal. Primer is a **downstream repo** and its recut is **out of this cut**: when steps 3+4 merge, file the Primer issue carrying this recipe verbatim; it lands on their cadence and gates nothing here. The in-tree artifacts are the primer-shaped `reach(x,x)` lock and `ts/test/expressibility-operand-views.test.ts` as the living evidence.

TS has no `max_program_recs` today. Do not add an engine-cap duplicate, do not add `max_ctes`, and do not add a sugar cap either: named interiors are uncapped at every layer — the builder is variadic, mirroring the engine.

## C++ (`cpp/src/query/program.cc`, `ir.cc`, `foreign/program.cc`)

Delete `bdb::program`, `bdb::rec<"name">`, `bdb::output`, `rec_def`, `output_def`, `is_rec_def_v`. Delete `foreign/program.cc`'s `bdb_program` graph as a separate Program IR — the static view is a `Query` with interiors/rec.

Add:

```cpp
q.interior<"mid">(rule_builders...)
q.recursive<"reach">(bdb::base{...}, bdb::rec{...})
```

Two **tagged packs**, not a flat variadic with no separator. Trailing `output(...)` is gone; main rules **are** the trailing builders, as a non-recursive query already is. `.idb(pred, ...)` / `.not_idb` become `.interior<"n">` / `.not_interior<"n">`. `interior` / `recursive` after a main rule, `interior` after `recursive`, a second `interior<"mid">`, a second `recursive` — consteval errors.

`max_program_recs = 4` dies as a Program-era name, and **no `max_interiors` replaces it**: named interiors are uncapped in C++ sugar too — a variadic pack of `interior<"n">` clauses, mirroring the engine. `max_query_rules = 4` stays what it is: a consteval cap on *rules* per recipe, with the existing `ir.cc` comment that the engine's `MAX_RULES` is the real law. Do not write `MAX_CTES = 16` next to it.

`75-cpp-lowering.md` §4 rewrite: drop Program shapes; Query with `interiors`/`rec`; `AtomSource` `interior`; lowering order interiors then rec then main; `output = recs.length` sentence deleted. Recipe parity: cookbook 24–25 C++ ports use `recursive`. Fingerprints will change (IR shape change); `00-product.md` compatibility is never a design input — regenerate the recipe fingerprint file in the same commit.

Bridge `cpp/bridge/src/query.rs` `program_in` dies. C ABI: `bdb_query { interiors, rec, head, rules }`, `bdb_atom_source_kind::INTERIOR` (`03-engine.md`). Delete `bdb_program`, `bdb_predicate`, `raii.cc prepare(bdb_program)`. Regenerated `bumbledb_c.h` via cbindgen.

## Architecture docs (present tense, same commit as Rust)

Zero-duplication law: cite `evalQuery`, `evalQuery_plain`, `reachDen`, `evalLinearReach_eq_lfp`, `reachOp_mono`, `reach_den_finite`, `wellFormed_interior_reads_real`. Do not restate denotations. Do not describe the language as `WITH RECURSIVE`. SQLite translator facts stay in `60-validation.md`.

| Doc | Edit |
|---|---|
| `20-query-ir.md` | Kill "a query is a program" / engine-recursion Program cut. Query shape: interiors + optional Rec + main. Union paragraph stays (set union, **one sink per rule-list**, no UNION ALL keyword). Caps: `MAX_RULES` per list; rec pools `MAX_RULES`; **no `MAX_CTES` / `MAX_PREDICATES`.** `degenerate_embedding` citations → `evalQuery_plain`. Strata judge → rec roster. Named-head notation → `interior` / `recursive`. Error names from `01` table. Three knobs in one paragraph (walls / this cut / OPEN), citing the chain-window OPEN already in the README |
| `40-execution.md` | Fixpoint driver section → linear reach driver: one DeltaVariant, round 0 = `reachOp_empty`, interiors-only never enters (`PreparedBody::Rules` or `Empty`). Drop "Lean evalProgram complete only under sufficient fuel" — the sentence is false **today** (fuel is internal; `missingCount_le` proves the bound); it does not survive to be retargeted. Do not replace it with Lean-fuel incompleteness. Budget = one derived-tuples ledger over interiors ∪ rec (`DerivedBudgetExceeded`), incompleteness vs `evalQuery`; rounds axis rec-only. Size (`DEFAULT_DERIVED_TUPLES`, né `DEFAULT_FIXPOINT_TUPLES`) is the wall, not interior count. Observability: `interiors:` then optional one `reach` then main; no strata, no `STRATUM`, no 16-slot interior span array |
| `00-product.md` | Deleted vocabulary: *rule program* stays deleted; engine recursion sentence becomes interiors + one linear rec, budgeted; drop `MAX_PREDICATES`. Deductive-database non-goal unamended |
| `70-api.md` | `prepare(&Query)` only. Drop `ProgramRef` / `From<Query> for Program` / degenerate embedding paragraph. `set_derived_budget(rounds, tuples)` (né `set_fixpoint_budget`): tuples axis judges every query, rounds axis rec-only |
| `75-cpp-lowering.md` | §4 as above |
| `60-validation.md` | `translate_program` → `translate_query`; `program-*.json` → `reach-*.json`; third oracle on that arm is `evalQueryList`. CQuery arm unchanged. SQLite is a lossy translator of this cut; inexpressible-set rows for mutual/nonlinear become unreachable (unwritable), not a denotation reason |
| `docs/architecture/README.md` | Recursion OPEN: chain-window unchanged. Add OPEN rows for stacked linear lfps, mutual-linear, named interior of finished rec, nonlinear-at-L — triggers from `05-cutover.md`. No new OPEN that re-litigates walls |
| `lean/README.md` | Level-1: `Exec/Reach.lean`, not fueled `evalProgram` |
| `lean/conformance/README.md` | `reach-*` dispatch; drop `program-*` |
| `docs/cookbook.md` 24–25 | Native forms use `recursive`. Guarantee citations: `evalLinearReach_eq_lfp` / `evalQuery_sound`, not `program_eval_sound`. Host-loop dialect stays. Primer-shaped `reach(x,x)` may be cited as the same family |

Census: every deleted theorem name in docs is a fail until the doc moves. Do the docs in the commit that lands the Lean rename, or immediately after Lean green and **before** Rust (`05`).

## Oracles

**SQLite — a lossy translator of this cut, never the denotation.** `translate_program` becomes `translate_query`. The translator emits SQL `WITH [RECURSIVE]` then the **whole** cte-list because that is what SQLite speaks — never `RECURSIVE` in the middle, never a second `WITH`. The SELECT is the **main** query. No CTE after the rec (this cut: interiors cannot read rec; interiors that read rec are already inlined in the IR). This SQL is not a grammar for `query!` and not a field name in the IR.

Interiors-only (translator output):

```sql
WITH w0(c0) AS (SELECT DISTINCT ...),
     w1(c0) AS (SELECT DISTINCT ... FROM ... JOIN w0 ...)
SELECT DISTINCT ... FROM ... JOIN w1 ...
```

Rec present (interior prefix optional). Identity main over the rec is `SELECT DISTINCT ... FROM r` — still a main query, not "the CTE is the answer":

```sql
WITH RECURSIVE
  w0(c0) AS (SELECT DISTINCT ...),
  r(c0) AS (
    SELECT ...                    -- base arms, UNION of them
    UNION
    SELECT ... FROM ... JOIN r    -- rec arms, one r reference each
  )
SELECT DISTINCT ... FROM ... JOIN r ...
```

`RECURSIVE` is required iff `rec` is `Some`, and it marks the entire cte-list — a SQLite syntactic constraint, not ours. Goldens today:

- `CLOSURE = "WITH RECURSIVE p0(...) AS (... UNION ...) SELECT DISTINCT ... FROM p0"` — recut: same placement; identity main stays `SELECT DISTINCT ... FROM r` (or `p0` synthesized).
- `CLOSURE_ROOTS` (p1 after p0 anti-joins finished rec) — recut: **one** rec, main SELECT is the inlined anti-join (`NOT EXISTS` / `LEFT JOIN ... WHERE r.col IS NULL`), not a second CTE after rec (`01-language.md` worked example).
- `CLOSURE_FROM_PARAM` — same as CLOSURE with a param in base.

Never `UNION ALL`. `sqlite_run/tests.rs` `UNION ALL` generator-bomb is unrelated SQLite, not a golden.

`sqlite_program_expressible`: mutual / nonlinear / `RecursiveFold` / `SelfNegation` become unreachable on accepted Queries **this cut**. **Delete the four Program gates**; validation is the screen; translator still errors on interval-typed derived columns (documented limit, generator-unreachable after the rec corpus is scalar-shaped as today). If a later cut admits mutual-linear or nonlinear, the translator grows an `Inexpressible` row again — that is a translator fact, not a reason to refuse the shape in Lean.

**Naive.** `NaiveDb::program` dies. Query eval: materialize interior tables (BTreeSet of tuples) in order; if rec, iterate `T(acc)=base ∪ rec(acc)` to fixpoint (no budget in the model); then eval main. Empty base ⇒ empty lfp is this iteration, not a SQLite special case. Three-way: engine (budgeted) vs naive (complete) vs Lean `evalQueryList` (complete) on `reach-*.json`. A budget abort is an engine-only miss — the differential must not demand engine completeness past the budget; today's tight-budget test stays a typed-error test, not a three-way case.

**Lean.** `02-lean.md` § conformance. Two arms: CQuery (`seeded-*.json`, `"relation"` → `.edb`) and Reach (`reach-*.json`, `evalQueryList`). Do not run `generate_program_corpus` until the Rust builder emits Reach JSON.

## Tests that become refusals or deletions

**Compile / construction refusals (keep as locks):**

- `query!` named head without `interior` / `recursive`
- `query!` two `recursive` names
- `query!` `interior` after `recursive`
- TS/C++ second `recursive` / `interior` after rec / `interior` after a main rule
- `NonlinearRecArm` (two self-atoms in one rec arm)
- `NegationInRec`
- `MeasureInRec` / `MeasureInInterior` / `AggregateInInterior`
- `EmptyRecursiveBase` / `EmptyRecursiveStep` / `SelfInBase` / `RecArmMissingSelf` / `EmptyInterior`
- `InteriorNotPrior`, `UnknownInterior`, `InteriorColumnOutOfRange`
- `an_interiors_only_query_does_not_enter_reach`
- more than 16 interiors still validate (no `TooManyCtes`)

**Delete (shape unwritable this cut; do not keep a Program-shaped fixture):**

- Mutual-recursion accepts (`a_mutual_pair_iterates_jointly`, condensation cycle tests as **acceptance** of mutual SCC). Mutual-linear is OPEN; do not keep a trophy fixture
- k-variant prepare tests (two same-stratum Idb atoms on one rule)
- `UnresolvedPredicateSignature` / `p(x) | p(x)` sealing (`EmptyRecursiveBase` covers the rec form)
- `UnknownOutputPredicate`
- `NegationThroughCycle` as an SCC item (replaced by `NegationInRec`)
- `From<Query> for Program` / `ProgramRef` tests
- `render_program` goldens (replaced by `render` of interiors/rec Query)
- `validate_program` entry tests
- `TooManyPredicates` / any `TooManyCtes` test
- C++ `bdb::program` / `rec<>` / `output` tests
- TS `program()` / `p.rec` / `p.output` tests
- C ABI `bdb_program` / `BDB_ATOM_SOURCE_KIND_IDB` tests
- `querygen/shapes_recursive.rs` mutual and nonlinear variants (rewrite the file to Query; do not keep those two coverage rows)
- `conformance/program.rs` Program JSON builder (rewrite to Reach JSON; Lean files already recut)
- `AggregationInRecCte` / `AggregationInRec` as a name (does not exist; use `AggregateInInterior`)
- `MAX_CTES` / `MAX_PREDICATES` as exported constants

**Retarget (same lock, new names — `01` table):**

- `rejects_a_negated_phantom_read` → `UnknownInterior`
- `rejects_a_measure_in_a_recursive_head` → `MeasureInInterior` (head, not `MeasureInRec`)
- `rejects_aggregation_through_a_cycle` → `AggregateInInterior`
- `a_measure_head_over_a_lower_stratum_is_legal` → measure/Sum on **main** over finished rec (cookbook 25)
- `negation_of_a_lower_stratum_passes` → negation of an interior or of finished rec **in main** (legal); negation **in rec** refuses (`NegationInRec` — self is the wall, finished-table is this-cut/OPEN) and is **not** the same query as the main anti-join
- `a_degenerate_program_executes_as_its_query` → `evalQuery_plain` lock
- tree/cyclic closure goldens — cyclic **graph** (data), still linear **rules**. Keep. Mutual **predicates** go.
- primer cycle detector (the **in-tree** lock / TS expressibility test) — recut 1:1; empty on a DAG. The Primer repo itself is the filed issue, not this list

Iterative Tarjan unit tests in `strata.rs` die with the file. Do not reimplement SCC "in case."
