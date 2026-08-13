# 04 — Bindings, docs, oracles, refusals

Engine IR is `Query` (`01`, `03`). Sugar lowers to that IR. Program vocabulary is deleted on every surface in the same Rust-phase commit as the IR, not "later in the SDK."

Grouping law, error names, MAX pooling: `01-language.md` is normative. This file does not invent a second spelling.

## `query!` (`crates/bumbledb-query-macros`)

Module-doc grammar becomes `01-language.md`'s block. Expansion:

- All-bare → `Query { with: vec![], rec: None, head, rules }` (today's lowering plus two empty fields).
- `with` / `with recursive` present → fill `with` / `rec`; bare rules are main. **Never emit `ir::Program`.**
- Named head without the keywords → `compile_error!` at the name, telling the author to write `with` or `with recursive`.

Classification (rec arms): a `with recursive pred` line is a rec arm iff its body has an **atom** (positive or negated, either spelling) naming `pred`. Else base. Consecutive same-name lines union; non-consecutive reuse is a compile error (`01` table).

Delete the predicate-table path that buckets named heads into `PredicateDef` and returns `Program { predicates, output }` (`lib.rs` around the `emitter.predicates.is_empty()` split). Delete `PredId` from generated code.

Goldens (`bumbledb-query/tests/notation.rs`, `notation-corpus`): `program-recursion` recut to `with recursive` text; `named_head_program_lowers_to_the_exact_ir` becomes a compile-fail fixture (named head, no keyword). Round-trip: `render(lower(text))` against the new renderer.

Cookbook tests (`bumbledb-query/tests/cookbook.rs` recipes 24–25): native forms use the keywords. Host-loop forms are unchanged (they were already plain queries).

## TypeScript (`ts/src/query/predicate.ts` and friends)

Delete `program()`, `ProgramScope.rec`, `ProgramScope.output`, `Rec`, `p.rec("reach")`, `p.output(...)`. The sealed program-as-query-value trick goes with them.

Add on the existing query builder (the `Query` value / scope that already builds rules):

```ts
q.with("mid", ...builders)   // one WithDef; builders are the union
q.withRecursive("reach", {
  base: [...builders],       // arrays — one callback is not "exactly one arm"
  rec:  [...builders],
})
```

Pick this shape, not `rec.rule` plus a later `output`. Base and rec are unmissable. Multiple `with` calls append `WithDef`s in call order (DAG order). A second `with("mid", ...)` is a construction error (names unique; TS does not consecutive-union). A second `withRecursive` throws at construction. `with` after `withRecursive` throws. `with` / `withRecursive` after a main rule has been added throws. The builder returns the ordinary query value; `db.prepare(q)` lowers to `QueryIr` with `with` / `rec` fields.

**Grouping law** (`01-language.md` table): names unique; multiple rules = many builders on **one** `with` / `withRecursive`. TS `base`/`rec` are arrays. C++ two tagged packs. `query!` classifies base vs rec by whether an atom names `pred`.

Lowering (`ts/src/query/lower.ts` §4.2 of `75-cpp-lowering.md`): recs-in-declaration-order + output-last **dies**. Emit `with[]`, optional `rec { head, base, rec }`, `head`, `rules`. Param registry: WITH in order, then rec base, then rec arms, then main — first use still mints `ParamId`. IDB-head-order bindings become CTE-head-order.

Wire (`ts/src/native.ts`): `ProgramIr` / `PredicateDefIr` / `{kind:"idb",pred}` die. `QueryIr` grows `with`, `rec`. `{kind:"cte",cte}`. napi `dbPrepare(db, program)` → `dbPrepare(db, query)` only. `ts/crate/src/marshal.rs` `program_in` → `query_in` with the new fields. Rule builders: `.idb(rec, binds)` → `.cte(name, binds)` / `.notCte`.

Tests: `ts/test/query.test.ts` recursion fences, `destructure-kernel` rec ports, `answers-named-orderable-ban` rec-head plumb, `psi-query-atoms` `output: 0` — recut to `with` / `withRecursive`. Construction errors for two recs. TS used `program()`; deleting it is the named-without-with fence.

TS has no `max_program_recs` today. Do not add an engine-cap duplicate; the engine validates `MAX_CTES = 16`. If a sugar cap is added later, it is 4, like C++, and stays sugar.

## C++ (`cpp/src/query/program.cc`, `ir.cc`, `foreign/program.cc`)

Delete `bdb::program`, `bdb::rec<"name">`, `bdb::output`, `rec_def`, `output_def`, `is_rec_def_v`. Delete `foreign/program.cc`'s `bdb_program` graph as a separate Program IR — the static view is a `Query` with WITH/rec.

Add:

```cpp
q.with<"mid">(rule_builders...)
q.with_recursive<"reach">(bdb::base{...}, bdb::rec{...})
```

Two **tagged packs**, not a flat variadic with no separator. Trailing `output(...)` is gone; main rules **are** the trailing builders, as a non-recursive query already is. `.idb(pred, ...)` / `.not_idb` become `.cte<"n">` / `.not_cte<"n">`. `with` / `with_recursive` after a main rule, `with` after `with_recursive`, a second `with<"mid">`, a second `with_recursive` — consteval errors.

`max_program_recs = 4` → `max_ctes = 4` (sugar). Engine `MAX_CTES = 16`. A builder that exceeds 4 fails at consteval; a raw IR that exceeds 16 fails at validate. Comment in `ir.cc` already says this about `max_query_rules` vs `MAX_RULES` — same sentence, CTE edition.

`75-cpp-lowering.md` §4 rewrite: drop Program shapes; Query with `with`/`rec`; `AtomSource` `cte`; lowering order WITH then rec then main; `output = recs.length` sentence deleted. Recipe parity: cookbook 24–25 C++ ports use `with_recursive`. Fingerprints will change (IR shape change); `00-product.md` compatibility is never a design input — regenerate the recipe fingerprint file in the same commit.

Bridge `cpp/bridge/src/query.rs` `program_in` dies. C ABI: `bdb_query { with, rec, head, rules }`, `bdb_atom_source_kind::CTE` (`03-engine.md`). Delete `bdb_program`, `bdb_predicate`, `raii.cc prepare(bdb_program)`. Regenerated `bumbledb_c.h` via cbindgen.

## Architecture docs (present tense, same commit as Rust)

Zero-duplication law: cite `evalQuery`, `evalQuery_plain`, `reachDen`, `evalLinearReach_eq_lfp`, `reachOp_mono`, `reach_den_finite`, `wellFormed_cte_reads_real`. Do not restate denotations.

| Doc | Edit |
|---|---|
| `20-query-ir.md` | Kill "a query is a program" / engine-recursion Program cut. Query shape: WITH + optional RecCte + main. Union paragraph stays (set union, **one sink per rule-list**, no UNION ALL keyword). Caps: `MAX_CTES` (excludes main; not a rename of `MAX_PREDICATES`). Rec CTE pools `MAX_RULES`. `degenerate_embedding` citations → `evalQuery_plain`. Strata judge → rec roster. Named-head notation → `with` / `with recursive`. Error names from `01` table |
| `40-execution.md` | Fixpoint driver section → linear reach driver: one DeltaVariant, round 0 = `reachOp_empty`, WITH-only never enters (`PreparedBody::Rules` or `Empty`). Drop "Lean evalProgram complete only under sufficient fuel." Do not replace it with Lean-fuel incompleteness. Budget = incompleteness vs `reachDen`. Observability: `ctes:` then optional one `reach` then main; no strata, no `STRATUM` |
| `00-product.md` | Deleted vocabulary: *rule program* stays deleted; engine recursion sentence becomes WITH + one linear rec, capped `MAX_CTES`, budgeted. Deductive-database non-goal unamended |
| `70-api.md` | `prepare(&Query)` only. Drop `ProgramRef` / `From<Query> for Program` / degenerate embedding paragraph. `set_fixpoint_budget` stays, rec-only effect |
| `75-cpp-lowering.md` | §4 as above |
| `60-validation.md` | `translate_program` → `translate_query`; `program-*.json` → `reach-*.json`; third oracle on that arm is `evalQueryList`. CQuery arm unchanged |
| `docs/architecture/README.md` | Recursion OPEN: chain-window unchanged. No new OPEN for this cut |
| `lean/README.md` | Level-1: `Exec/Reach.lean`, not fueled `evalProgram` |
| `lean/conformance/README.md` | `reach-*` dispatch; drop `program-*` |
| `docs/cookbook.md` 24–25 | Native forms use `with recursive`. Guarantee citations: `evalLinearReach_eq_lfp` / `evalQuery_sound`, not `program_eval_sound`. Host-loop dialect stays |

Census: every deleted theorem name in docs is a fail until the doc moves. Do the docs in the commit that lands the Lean rename, or immediately after Lean green and **before** Rust (`05`).

## Oracles

**SQLite.** `translate_program` becomes `translate_query`. SQL keyword is `WITH [RECURSIVE]` then the **whole** cte-list — never `RECURSIVE` in the middle, never a second `WITH`. The SELECT is the **main** query. No CTE after the rec (WITH cannot read rec; interiors that read rec are already inlined in the IR).

WITH-only:

```sql
WITH w0(c0) AS (SELECT DISTINCT ...),
     w1(c0) AS (SELECT DISTINCT ... FROM ... JOIN w0 ...)
SELECT DISTINCT ... FROM ... JOIN w1 ...
```

Rec present (WITH prefix optional). Identity main over the rec CTE is `SELECT DISTINCT ... FROM r` — still a main query, not "the CTE is the answer":

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

`RECURSIVE` is required iff `rec` is `Some`, and it marks the entire cte-list. Goldens today:

- `CLOSURE = "WITH RECURSIVE p0(...) AS (... UNION ...) SELECT DISTINCT ... FROM p0"` — recut: same placement; identity main stays `SELECT DISTINCT ... FROM r` (or `p0` synthesized).
- `CLOSURE_ROOTS` (p1 after p0 anti-joins finished rec) — recut: **one** rec CTE, main SELECT is the inlined anti-join (`NOT EXISTS` / `LEFT JOIN ... WHERE r.col IS NULL`), not a second CTE after rec (`01-language.md` worked example).
- `CLOSURE_FROM_PARAM` — same as CLOSURE with a param in base.

Never `UNION ALL`. `sqlite_run/tests.rs` `UNION ALL` generator-bomb is unrelated SQLite, not a golden.

`sqlite_program_expressible`: mutual / nonlinear / `RecursiveFold` / `SelfNegation` become unreachable on accepted Queries. **Delete the four Program gates**; validation is the screen; translator still errors on interval-typed CTE columns (documented limit, generator-unreachable after the rec corpus is scalar-shaped as today).

**Naive.** `NaiveDb::program` dies. Query eval: materialize WITH tables (BTreeSet of tuples) in order; if rec, iterate `T(acc)=base ∪ rec(acc)` to fixpoint (no budget in the model); then eval main. Three-way: engine (budgeted) vs naive (complete) vs Lean `evalQueryList` (complete) on `reach-*.json`. A budget abort is an engine-only miss — the differential must not demand engine completeness past the budget; today's tight-budget test stays a typed-error test, not a three-way case.

**Lean.** `02-lean.md` § conformance. Two arms: CQuery (`seeded-*.json`, `"relation"` → `.edb`) and Reach (`reach-*.json`, `evalQueryList`). Do not run `generate_program_corpus` until the Rust builder emits Reach JSON.

## Tests that become refusals or deletions

**Compile / construction refusals (keep as locks):**

- `query!` named head without `with` / `with recursive`
- `query!` two `with recursive` names
- `query!` `with` after `with recursive`
- TS/C++ second `withRecursive` / `with` after rec / `with` after a main rule
- `NonlinearRecArm` (two self-atoms in one rec arm)
- `NegationInRecCte`
- `MeasureInRecCte` / `MeasureInCte` / `AggregateInCte`
- `EmptyRecursiveBase` / `EmptyRecursiveStep` / `SelfInBase` / `RecArmMissingSelf` / `EmptyCte`
- `CteNotPrior`, `UnknownCte`, `CteColumnOutOfRange`, `TooManyCtes`
- `a_with_only_query_does_not_enter_reach`

**Delete (shape unwritable; do not keep a Program-shaped fixture):**

- Mutual-recursion accepts (`a_mutual_pair_iterates_jointly`, condensation cycle tests as **acceptance** of mutual SCC)
- k-variant prepare tests (two same-stratum Idb atoms on one rule)
- `UnresolvedPredicateSignature` / `p(x) | p(x)` sealing (`EmptyRecursiveBase` covers the rec form)
- `UnknownOutputPredicate`
- `NegationThroughCycle` as an SCC item (replaced by `NegationInRecCte`)
- `From<Query> for Program` / `ProgramRef` tests
- `render_program` goldens (replaced by `render` of WITH/rec Query)
- `validate_program` entry tests
- C++ `bdb::program` / `rec<>` / `output` tests
- TS `program()` / `p.rec` / `p.output` tests
- C ABI `bdb_program` / `BDB_ATOM_SOURCE_KIND_IDB` tests
- `querygen/shapes_recursive.rs` mutual and nonlinear variants (rewrite the file to Query; do not keep those two coverage rows)
- `conformance/program.rs` Program JSON builder (rewrite to Reach JSON; Lean files already recut)
- `AggregationInRecCte` as a name (does not exist; use `AggregateInCte`)

**Retarget (same lock, new names — `01` table):**

- `rejects_a_negated_phantom_read` → `UnknownCte`
- `rejects_a_measure_in_a_recursive_head` → `MeasureInCte` (head, not `MeasureInRecCte`)
- `rejects_aggregation_through_a_cycle` → `AggregateInCte`
- `a_measure_head_over_a_lower_stratum_is_legal` → measure/Sum on **main** over finished rec (cookbook 25)
- `negation_of_a_lower_stratum_passes` → negation of a WITH or of finished rec **in main** (legal); negation **in rec CTE** refuses (`NegationInRecCte`) and is **not** the same query as the main anti-join
- `a_degenerate_program_executes_as_its_query` → `evalQuery_plain` lock
- tree/cyclic closure goldens — cyclic **graph** (data), still linear **rules**. Keep. Mutual **predicates** go.

Iterative Tarjan unit tests in `strata.rs` die with the file. Do not reimplement SCC "in case."
