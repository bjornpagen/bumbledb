# Bench representation audit (wave 2 — beyond the four oracles)

Brooks: the tables make the flowcharts obvious. Pike: data dominates; algorithms follow. Applied to `crates/bumbledb-bench/src/` **except** the four wave-1 oracles already filed as engine F19–F21 / F38:

- `naive/query.rs` (engine-019)
- `querygen.rs` + `querygen/shapes_recursive.rs` (engine-020)
- `translate/query.rs` + `translate/reach.rs` (engine-021)
- `conformance/reach.rs` (engine-038 → DUPLICATE(engine-012))

Priority hunt: `corpus_gen/`, `differential/`, `lawful/`, `sqlite_run/`, `scenarios/`, `closure/` (beyond the profile-skip comment), `families/`, `driver/`, `verify/`. Remaining `querygen/` (oracle, coverage, contradict, construct, builder, tests), remaining `translate/` (builder, goldens, types, `translate.rs` facade), remaining `conformance.rs`, remaining `naive/tests/`.

Program IR is gone. No generator in this crate still emits Program-shaped JSON (`predicates` / `output` / `strata` / `idb`). The leftover coordinate is **CQ-as-the-language**: one generator entry, one serializer, one expressibility gate, one param-anchor walk, and the stamp that gates timing all still describe a query as `rules` over EDB atoms. Interiors and rec are a second corpus, a second function, a second JSON spelling, a second SQL gate.

C1 freezes the 268 checked-in cases. Dual *spellings* (`relation` vs `edb`/`interior`; `{rules}` vs `{interiors,rec,rules}`) stay. Dual *code* that cannot share a Query does not.

---

## The shape that is wrong

The lever is the same missing sum as the engine, one layer out. After parse, a query is interiors then optional rec then main. The bench's trusted lanes still mint and consume the empty-prefix case as if it were the type:

```
random_query        → Query { interiors: [], rec: None, head, rules }   // the stamp, the fuzz, the seeded corpus
random_reach_query  → Query { interiors, rec, head, rules }             // a side entry (engine-020)
irgen::random_query → Query { interiors: [], rec: None, … }             // "structurally-free" and never draws the other two fields
render_case         → {"query":{"rules":[…]}}                            // CQuery JSON
reach::render       → {"query":{"interiors":[],"rec":{…},"arity":N,"rules":[…]}}
sqlite_expressible(Query)  vs  sqlite_reach_expressible(Query)
params_for / coverage / contradict  walk  query.rules  only
EdbAtom::relation()  panics on Interior
```

Every `random_query` caller that did not also remember `random_reach_query` is a branch the grammar still requires. Until the generator's top sum is one entry and the serializer/oracle/coverage walks are one walk over interiors-then-rec-then-main, the flowchart cannot get simpler than the table.

---

## Findings

### F1. Two JSON emitters, two query types — the producer of lean-008

- **Where:** `conformance.rs:810-864` (`render_case`); atoms at `1025` (`{"relation":…}` via `EdbAtom::relation`). The reach twin (already filed, out of this dump's edit scope) is `conformance/reach.rs`.
- **What's wrong:** The seeded/hand emitter writes `{"query":{"rules":[…]}}` — no `interiors`, no `rec`, no `head`. Atoms are `"relation"`. That is `CQuery`. Reach cases are a different document shape (`interiors` / `rec` / `arity` / `rules`; atoms `edb` / `interior`). One language, two serializers that do not share a Query renderer. lean-008 is the decoder; this is the mint. docs-027 teaches the split because this code produces it.
- **Collapsing representation:** one Query renderer. Two frozen *spellings* (C1): omit empty interiors/rec/`head` for the 246 seeded/hand files so they stay byte-identical; emit the reach spelling for `reach-*.json`. `relation` and `edb` are two keys for `AtomSource::Edb`. Do not regenerate the 268 cases.
- **Essential vs accidental:** two JSON spellings at a sum-less boundary are essential (C1). Two functions that cannot spell the other arm are accidental.
- **Severity:** high

### F2. `irgen` claims a structurally-free Query and never draws interiors or rec

- **Where:** `corpus_gen/irgen.rs:37-71,79-126`
- **What's wrong:** The fuzz arm's contract: "every shape the IR type can spell is reachable by some byte string." Both constructors write `interiors: vec![], rec: None` (or `Query::single`). Empty interiors, empty rec, dangling `InteriorId`, empty `Rec.base`/`Rec.rec`, self-in-base, nonlinear self, negation-in-rec — the roster the engine exists to refuse — are unemittable. A generator that cannot mint the hostile states of the new IR can only confirm the old CQ roster. Vocabulary: "Valid and invalid programs both arise."
- **Collapsing representation:** the free draw reaches `Interior` / `Rec` the same way it reaches empty rule lists and dangling relation ids — as hostile data, judged by the engine. `Query::single` stays the coherent-core path (engine-037). The free arm is a Query, not a CQ with two frozen empty fields.
- **Essential vs accidental:** a hostile generator that types nothing is essential (the engine is the judge). Pinning two of four Query fields to empty is accidental CQ leftover.
- **Severity:** high

### F3. The stamp, the fuzz, the seeded corpus, and the contradiction knob consume only `random_query`

- **Where:** `verify/run.rs:134-177` (`random_lane`); `corpus_gen/opgen.rs:90`; `conformance.rs:1527,1663`; `querygen/contradict.rs:19-24`; also `differential/tests/{closed,fold}.rs`, `corpus_gen/rng.rs:130`
- **What's wrong:** engine-020's side entry is not a comment. Every trusted consumer that "draws a random Query" calls `random_query` and never `random_reach_query`. The verify stamp that gates timing, the lifecycle fuzz pool, the 246 seeded conformance cases, and the contradiction-fold differential are CQ-only. Rec coverage lives in a parallel world (closure inline gate, `differential/tests/recursive.rs`, the reach conformance arm). A shared misreading of interiors/rec can pass the stamp forever — the dual-oracle blind spot the Lean lane exists to close, reopened by the grammar's product.
- **Collapsing representation:** one `random_query` (engine-020). These callers update mechanically. Until that lands, they are the proof the side entry is not equivalent to a `Shape` row: a row would have been drawn here.
- **Essential vs accidental:** a dedicated closure measurement world is essential (different corpus shape). Making the *randomized Query grammar* CQ-only is accidental.
- **Severity:** high

### F4. Param anchors, coverage, contradiction, and `EdbAtom` walk `query.rules` over EDB atoms only

- **Where:** `querygen/oracle.rs:64-79,102-134`; `querygen/coverage.rs:532`; `querygen/contradict.rs:22`; `edb.rs:9-20`; also `conformance.rs:659,715,729,749,1025` (same `EdbAtom` on the seeded serializer)
- **What's wrong:** `params_for` discovers params by walking `query.rules`. Coverage tallies walk `query.rules`. Contradiction plants on `query.rules`. `EdbAtom::relation` panics on `Interior` — "harness atoms are stored-relation by construction." That premise is the CQ grammar. A rec query whose only param lives on a base arm is invisible to `params_for`; an Interior atom panics `coverage` / `contradict` / `render_case`. Hidden today only because F3 never feeds them such a Query. King: the walk validated "main rules, EDB sources" and threw the rest of the type away.
- **Collapsing representation:** one walk over interiors then rec (base+step) then main. Atom source is a match (`Edb` → schema field; `Interior` → derived column), not a panicking `relation()`. `EdbAtom` stays legal on the CQ *builder* (it only constructs EDB atoms); it dies as a trait on `Atom`.
- **Essential vs accidental:** EDB-only construction inside `Builder` is essential for that assembler. Treating `Atom` as EDB-only across the crate is accidental.
- **Severity:** med

### F5. Two expressibility gates for one translator

- **Where:** `translate.rs:39-44,58,229-261`; callers `verify/run.rs:152`, `differential/tests/recursive.rs:256,529`, `querygen/tests.rs:506,582`
- **What's wrong:** `sqlite_expressible` on a `Query` checks Pack (and nothing about derived tables) and returns `Ok`. `sqlite_reach_expressible` (engine-021) screens interval-typed derived columns and is named for rec while screening interiors. Verify's randomized lane uses the first; reach tests use the second. One translator, two gates, selected by which generator entry produced the Query — the same two-flag product as engine-021's front door, one layer up. `translate.rs` still documents "Interiors + rec = `WITH [RECURSIVE]` ([`reach`])."
- **Collapsing representation:** one `sqlite_expressible(&Query)`: Pack, then interval-derived-column. `sqlite_reach_expressible` dies into it (engine-021 rename). Callers do not choose a gate by shape.
- **Essential vs accidental:** Pack vs interval-derived-column vs judgments are essential inexpressible classes. Two functions for one Query are accidental.
- **Severity:** med

### F6. Derived CTEs are still Datalog `p{id}` — goldens lock the name

- **Where:** `translate/builder.rs:121-126`; goldens `translate/goldens.rs:147-160` (`p0(c0, c1)`, `FROM "p0"`, `¬p0`)
- **What's wrong:** engine-033 is the engine's diagnostic `predicate p{id}`. The translator's SQL identifiers are the same coordinate on the oracle the engine is checked against: an interior/rec atom becomes table `p{id}`. The three closure goldens pin `p0`. Every reader of the 3-way arbitration anchor learns rec is predicate p0. C3: `interior {id}` / `rec` — no `predicate p{}`.
- **Collapsing representation:** CTE name `interior{id}` for interiors, `rec` for the rec (one rec, no number). Goldens rewrite mechanically. Answers stay byte-identical; SQL strings do not (this is the point). Coordinate with engine-021 so the one WITH path is renamed once.
- **Essential vs accidental:** positional CTE columns (`c{i}`) are the translator's derived-table spelling — essential given SQLite. Calling the table `p0` is accidental Program residue.
- **Severity:** med

### F7. Closure lane still teaches delta-variants and "one program"

- **Where:** `closure.rs:1-9,223-266` (not line 502 — that skip is engine-011)
- **What's wrong:** Module doc: driven through `Db::prepare` (`AtomSource::Interior`, **the delta-variant plans**, the finished-image slot). Registry: "two families, **one program**, two corpus shapes." The query itself is a correct boundary `Query` (empty interiors, one rec, identity main — C1 numbering `InteriorId(0)` is the rec). The teaching around it is k-variant Program. The measured object is one Query; the comment says a program.
- **Collapsing representation:** present-tense: one Query, two corpus shapes selected by anchor. Prepare through the reach pipeline (no "delta-variant"). `InteriorId(0)` stays — that is C2 at the untrusted layer when interiors is empty.
- **Essential vs accidental:** two measurement axes (depth vs fanout) on one query are essential. "Program" / "delta-variant" are accidental.
- **Severity:** med

### F8. `exec_digest` is a CQ stats consumer

- **Where:** `driver/read_family.rs:12-44,175-186`
- **What's wrong:** The profile digest walks `stats.rules` (nodes, absorbed) and `stats.emits`. It does not read `stats.interiors` or `stats.reach`. Ledger/calendar families are CQ so it is true today; it is the old per-stratum rule table as the counted surface (engine-012). Closure skips profile rather than digest a Reach stats arm (engine-011). Two encodings of "rec is not a stats shape we have": skip, or walk the CQ field and miss.
- **Collapsing representation:** digest matches the pipeline sum (engine-012). CQ: main-rule covers. Reach: interior emits + reach rounds. No `stats.rules` as the universal table.
- **Essential vs accidental:** a plan-quality digest is essential. Shaping it as `stats.rules` only is accidental.
- **Severity:** med

### F9. "Program" still names a Query across the remaining crate

- **Where:** `querygen/shapes_rules.rs:1,60`; `querygen/construct.rs:60,83`; `querygen.rs:106,367` (module file is engine-020; the `Shape::Rules` comments remain in remaining construct); `querygen/contradict.rs:15`; `querygen/tests.rs:127`; `verify/run_algebra.rs:4,26,149,602-603,654,680`; `calendar/families.rs:169`; `corpus_gen/irgen.rs:39,49,58`; `conformance.rs:1247`
- **What's wrong:** Multi-rule Queries are "programs." A three-rule calendar family is "a three-rule program." The empty-Or algebra row is "the vanished program." Union idempotence is "at the program level." irgen's free draws are "small programs." C7: no `program` as our noun. The denotation did not keep Program. The bench's coverage names did.
- **Collapsing representation:** "multi-rule query," "vanished query (empty Or)," "union idempotent at the query," "three-rule family." Keep "programmer error" (English). Keep SQL `WITH RECURSIVE` as translator spelling.
- **Essential vs accidental:** accidental naming. Harmless in isolation; it trains every coverage report that a rule-list is a Program.
- **Severity:** low

### F10. `.predicate()` on every prepared-query consumer — DUPLICATE(engine-041)

- **Where:** `driver/read_family.rs:116`; `verify/check.rs:46`; `closure.rs:371`; `closure/tests.rs:64`; `scenarios/run_query.rs:102`; `lanes/curves.rs:587,765,1555`; `displaced.rs:530,661`; `churn/probes.rs:230`; `crud/run.rs:449`; `calendar/tests.rs:242`; `sqlite_run.rs:46` (comment); `sqlite_run/tests.rs:44,85`; `compare.rs:28`
- **What's wrong:** engine-041 owns the mechanical rename `Predicate` → `Signature`, `predicate()` → `signature()`, "across `crates/bumbledb` and `crates/bumbledb-bench` (~20 bench call sites)." These are those sites. No residual edit under a bench id.
- **Severity:** low
- **Status:** DUPLICATE(engine-041)

### F11. Closure `exec: None` because "the profile path is query-shaped" — DUPLICATE(engine-011)

- **Where:** `closure.rs:502`
- **What's wrong:** engine-011 already names this site; engine-008 owns the profile-path CODE. The comment is the skip's excuse, not a second defect.
- **Severity:** low
- **Status:** DUPLICATE(engine-011)

### F12. `querygen/tests.rs` still asserts interiors-only rows under `RecursiveVariant` — DUPLICATE(engine-020)

- **Where:** `querygen/tests.rs:490-491,649-678`
- **What's wrong:** engine-020's acceptance already requires `InteriorsDag` / `InteriorsAntiJoin` / `ManyInteriors` to leave `RecursiveVariant`. The tests are that issue's tree, not a second finding.
- **Severity:** low
- **Status:** DUPLICATE(engine-020)

---

## Not counted as bugs

- **Boundary `Query { interiors, rec: Option<Rec>, head, rules }` construction** in tests and families, including `InteriorId(0)` when interiors is empty (that is C2 at the untrusted layer). `Query::single` for one-rule CQ is the right constructor (engine-037); multi-rule CQ must spell the product. Do not invent a third type.
- **`families/read.rs` using `Query::single`.** Correct coordinate.
- **`scenarios::Surface` as a sum (`Query` | `KeyedGet`).** Correct — gate/time fold over data, not a rec/CQ flag.
- **`differential::Op::Query` as a first-class op.** Correct (R22). `differential/tests/recursive.rs` is dedicated goldens, not a second generator entry.
- **`lawful/`.** No Query IR. Admission/laws world.
- **SQL "predicate columns"** in `scenarios.rs` (WHERE-clause indexes) and **condition "predicates"** in DNF tests / dress comments — English for filters, not Datalog `Predicate`.
- **`calendar/corpus_gen.rs` "density strata"** — rank buckets, not IR strata.
- **"programmer error" / `Command::new(program)`.** English / OS.
- **Naive full-lfp vs engine semi-naive.** Essential oracle contrast (engine "Not counted").
- **Frozen reach JSON `arity` fields.** Produced by `conformance/reach.rs` (out of scope); C1 forbids regenerating. Lean H6 / C4 delete arity *in Lean*; the corpus spelling stays.
- **Wave-1 oracles.** engine-019/020/021/038. Not re-filed.

---

## Counts

| Severity | Count |
|----------|------:|
| high     |     3 |
| med      |     5 |
| low      |     4 |
| **total**| **12** |

High: F1–F3. Med: F4–F8. Low: F9 + three DUPLICATE stubs (F10–F12).

Unique OPEN issues: 9 (F1–F9). Duplicates: 3.

---

## Program-shaped JSON?

**No.** `rg '"predicates"|"strata"|"idb"'` over `crates/bumbledb-bench/src` is empty. Seeded/hand cases emit CQuery JSON (`{"query":{"rules":[…]}}`, atoms `"relation"`). Reach cases emit Query JSON (`interiors` / `rec` / `arity` / `rules`, atoms `edb` / `interior`) — no `predicates` / `output` / `strata` / `idb`. The leftover is the *dual Query/CQuery spelling*, not a revival of Program fields.

---

## The one table that would delete the flowchart

```
enum QueryClass { Cq(Shape), Derived(DerivedShape) }   // one random_query  (engine-020)

fn render_query(q: &Query, spelling: Seeded | Reach) -> String
    // Seeded: frozen {rules} + "relation"   (C1, 246 files byte-identical)
    // Reach:  frozen interiors/rec/rules + edb/interior

fn sqlite_expressible(q: &Query) -> Result<(), Inexpressible>
    // Pack | IntervalDerivedColumn | Judgment — one gate

fn walk_rules(q: &Query) -> impl Iterator<Item = &Rule>
    // interiors, then rec.base, rec.rec, then main
```

`random_reach_query` as a public entry, `render_case` vs reach render, `sqlite_reach_expressible`, `EdbAtom` on `Atom`, and `params_for` walking only `query.rules` have nowhere to stand.

Brooks: show the tables. This is the table. The `if we remembered the reach entry` forest is the flowchart it makes obsolete.
