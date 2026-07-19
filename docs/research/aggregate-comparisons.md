# Aggregate comparisons (the HAVING shape) — feature investigation

Date: 2026-07-19. Read-only investigation; no repo edits. Repos read:
`/Users/bjorn/Documents/bumbledb` (worktree `.claude/worktrees/host-idiom-040`, clean on main @ 8e30387e)
and `/Users/bjorn/Documents/primer`.

The candidate: let a rule's judgment consume aggregate outputs — compare an
aggregate to a literal/param/another aggregate inside the engine — instead of the
host folding after execution.

STATUS: docs/goal-alignment layer complete; Lean-model, engine, and workload
sections pending subagent reports (placeholders marked TBD).

---

## 0. The two distinct shapes hiding under "HAVING"

Everything below keeps these apart, because the repo's decision records treat
them completely differently:

- **Shape A — boundary HAVING (weak form):** a comparison over the OUTPUT
  predicate's aggregate head positions, filtering answers before they exit to
  the host. `(a, n: Count()) … having n >= ?floor`. The created value is
  compared at the answer boundary and discarded; it never feeds an atom.
- **Shape B — interior HAVING (strong form):** an interior predicate with an
  aggregate head whose output an `Idb` atom of a downstream rule consumes
  (aggregate → compare → feed another rule). This is a new stratum boundary in
  the full sense.

## 1. FEASIBILITY — what exactly would change

### 1.1 Where the IR stands today (docs, normative)

- Aggregates exist only as head positions: `HeadTerm = Var | Aggregate(HeadOp)`,
  `FindTerm::Aggregate/AggregateMeasure` — `docs/architecture/20-query-ir.md:386-422`.
  Comparisons (`Comparison { op, lhs: Term, rhs: Term }`, 20-query-ir.md:423)
  range over rule-scoped `Term`s; an aggregate output is not a `Term` and is
  therefore not comparable — unwritable, not rejected.
- The one precedent for a computed comparable: `Term::Measure(VarId)` —
  "comparison side only … a binding position is a typed rejection"
  (20-query-ir.md:400-403, 555-618). The measure section enumerates legal
  positions exhaustively and is explicitly the template for admitting any new
  computation ("individual named computations may be admitted one at a time on
  the measure's precedent — typed positions, boundary-only, each a recorded
  decision", 20-query-ir.md:679-683).
- In a `Program`, folds and measures are legal ONLY at the output predicate's
  head: typed refusals `AggregateInteriorPredicate` / `MeasureInteriorPredicate`
  ("interior heads, recursive or not, project bound variables, the Lean cut's
  own class: `PRule.finds : List VarId`") — 20-query-ir.md:856-864. So Shape B
  is currently refused at validation by name.
- The other half of the composition already exists: an OUTPUT-head fold over a
  finished lower stratum is legal and shipped (`AggregationThroughCycle` refuses
  only folds through a cycle; "a fold reads finished sets only",
  20-query-ir.md:131-149; cookbook recipe 25, docs/cookbook.md:1171-1241, shows
  `(total: Sum(minor)) | Posting(…), sub(a);` over a converged closure stratum).
  HAVING is exactly the missing reverse direction: consuming the fold's output.

### 1.2 What each shape would require

**Shape A (boundary HAVING):**
- IR: a new comparison position over output-head aggregate positions — most
  naturally a per-QUERY (not per-rule) condition list whose terms may reference
  head positions by index, or a `Term::AggOut(HeadPos)` legal on comparison
  sides only, exactly the measure's pattern. Aggregates fold across rules (the
  fold domain is the union of the rules' binding sets projected to the head,
  20-query-ir.md:283-288), so the filter is semantically per-GROUP after the
  union fold — it cannot live in any single rule's `conditions`, which is a
  genuinely new slot in `Query`, not a new leaf in an existing list.
- Validation: type the comparison against the aggregate's result type
  (Count/CountDistinct → U64; Sum/Min/Max → input type; Pack/Arg produce
  relation-shaped/selected values — comparisons over Pack segments or Arg
  carries would need their own rulings or refusals). Aggregate-vs-aggregate
  comparison (same head, e.g. `Min(x) < Max(x)` per group... or two different
  folds) is representable in the same slot; params and literals type as usual.
- Exec: a post-fold filter at the sink's finalize/emit point — after the
  spanning seen-set and the fold, before the answer copies out. (Engine agent:
  confirm where finalize lives and whether a filter there breaks the
  zero-allocation contract — it should not; it's a keep/drop test per group.)
- Notation/`query!`: a new clause after the rule list or attached to the head
  (`ir::render` and the render grammar, 20-query-ir.md:917-1083, must both grow);
  TS builder: a `.having(...)` on the select/output scope (ts/src/query/,
  prd-S3 surface). M4/conformance corpus: new cases; SQLite CAN express HAVING,
  so the external oracle covers it directly (60-validation.md:20-23) — unusual
  luxury for this repo's features.

**Shape B (interior HAVING):**
- Everything in Shape A's validation/typing PLUS: delete or narrow
  `AggregateInteriorPredicate`; extend predicate signature sealing so an
  aggregate head seals a typed column readable by `Idb` atoms; the downstream
  comparison is then just today's scalar comparison over a bound variable —
  no new comparison construct at all. The stratification is already correct
  machinery-wise: the strata judge would admit "fold at a stratum boundary,
  never through a cycle" with `AggregationThroughCycle` unchanged.
- But it reverses a Closed-by-ruling item verbatim: "A created value never
  re-enters a derivation — heads bind, filters compare, folds create at the
  answer boundary only" (docs/architecture/README.md:178-180; the full decision
  record with alternative and reversal condition is 20-query-ir.md:654-683).
  Sum/Count outputs are values outside the active domain; letting an `Idb` atom
  bind them breaks the "atoms select — every joined value exists in a stored
  column" invariant that planning/statistics and the finiteness theorem's
  premise lean on. (Lean agent: confirm what `program_den_finite`'s premise
  actually requires and whether finite-per-stratum fold outputs could re-prove
  it; engine agent: confirm what the selectivity ladder and transient-image
  bind assume about Idb columns.)

### 1.3 Is it a new stratum boundary?

Shape A: no — one extra evaluation step at the answer boundary of the last
stratum. Shape B: yes — the aggregate predicate must be a finished stratum
below its consumers; the existing condensation/topological witness
(20-query-ir.md:131-135) already expresses that, so no new graph theory, but a
new refusal-roster arm ("aggregate predicate inside an SCC" =
AggregationThroughCycle generalized from body-position to head-position).

## 2. DECIDABILITY

Docs-layer analysis (Lean agent findings TBD):

- Totality/decidability posture today: validation is a total boundary judgment
  (the trust-boundary law, 20-query-ir.md:814-828, adversarially swept);
  evaluation of programs is finite by `lean/Bumbledb/Exec/Fixpoint.lean:
  program_den_finite`, whose PREMISE is "recursive heads project bound
  variables only" (20-query-ir.md:143-148) — the countermodel when a head
  creates values is `Countermodels.lean: succ_prefixed_infinite`.
- Shape A adds no fixpoint interaction at all: the filter runs once, after
  every fixpoint has converged and the output fold is finished. Groups are
  finite, the comparison is the existing decidable typed comparison, semantics
  stay total. No new axiom class is plausible; the proof obligation is a
  filtered-denotation lemma over the existing aggregate denotation
  (`Query/Aggregates.lean`), not a new termination story.
- Shape B also does NOT threaten termination in the Datalog-theoretic sense —
  stratified aggregation (fold at stratum boundaries only) is the classical
  decidable regime, and `AggregationThroughCycle` already fences the one
  undecidable/non-monotone door (aggregation through a cycle stays unwritable).
  The hazard is not decidability but the PROOF ARCHITECTURE: `program_den_finite`
  is stated with var-only interior heads (`PRule.finds : List VarId` is the
  Lean cut's own class, 20-query-ir.md:863-864), so Shape B needs the theorem
  re-proved with a per-stratum "finite input ⇒ finite fold output" lemma and a
  new PRule head grammar — real Lean surgery at the model's load-bearing
  definition, not an add-on lemma. (Lean agent: size this.)
- Conformance: three-way lane (engine / naive model / `lake exe conformance`
  over `lean/conformance/cases/`, 60-validation.md:46-61) extends mechanically
  for Shape A (same case format, new construct in the serialized IR — requires
  the Lean evaluator and naive model to grow the same arm). SQLite oracle
  covers HAVING natively (60-validation.md:20-23).

## 3. GOAL ALIGNMENT

Philosophy anchors, verbatim sources:

- "Representation over control flow… Illegal states unrepresentable"
  (00-product.md:27-30). Non-goals: "A deductive database / logic-programming
  runtime… Turing-completeness lives in the host" (00-product.md:271-278).
  Deleted vocabulary maps *rule program* → "the host loop over prepared
  queries" (00-product.md:307-308).
- The census discipline: "The census drove the feature set; nothing shipped
  without a sighting in it" (00-product.md:68-70).
- The house posture for post-processing: OPEN item "Ordering/limit conveniences
  and top-k pushdown: presentation-layer; results are sets, the host sorts.
  Trigger: owner pain, or a measured materialize-then-sort latency-budget
  violation" (docs/architecture/README.md:70-72). Host-fold-then-filter is the
  same class as host-sort.
- The sanctioned v0 idiom is documented, twice: "aggregate in one query, join
  its result in the host" (20-query-ir.md:293-294); coalesced totals "two
  prepared queries or a host fold over packed answers… refusals recorded in the
  ledger" (20-query-ir.md:344-349, cookbook.md:867-869).
- The admission path exists and is recorded: the creation quarantine's reversal
  clause — "never as a general mechanism; individual named computations may be
  admitted one at a time on the measure's precedent — typed positions,
  boundary-only, each a recorded decision" (20-query-ir.md:681-683).

**The case FOR (natural completion):**
- The aggregate story is asymmetric today: the engine can fold over anything
  (including finished recursive strata, recipe 25) but cannot state the
  simplest judgment about its own fold — "count ≥ k" — which SQL, SQLite (the
  oracle!), and every Datalog-with-aggregates dialect can. The engine's thesis
  is "invariants are judgments about queries"; a threshold over a fold is the
  most judgment-shaped query feature imaginable, and the dependency language
  already has the cardinality-window form (00-product.md:9-11) — the WINDOW
  judgment exists on the write side but is unstatable on the read side.
- Shape A fits the recorded admission path exactly: typed position,
  boundary-only, one named computation (a comparison over a fold output),
  precedented by the measure — which went from "refused arithmetic" to a
  shipped boundary-only construct under the same clause.
- SQLite expressibility means the strongest oracle covers it from day one; the
  semantics (filter groups after the fold, empty-input yields empty set — no
  SQL zero-row trap, already litigated at 20-query-ir.md:350-355) compose with
  no open questions.

**The case AGAINST (scope creep):**
- Zero latency-budget pain is on record. The host fold is O(answer groups) of
  scalar comparisons; the OPEN-items posture says post-processing moves
  engine-side only on a MEASURED budget violation, and none exists (workload
  agent: confirm at the 2-3 primer sites). The pruning win (fewer rows copied
  out) only matters when groups are numerous and survivors few — TBD whether
  any real site has that shape.
- The composition asymmetry cuts the other way too: Shape A's HAVING output
  still can't feed a join (creation quarantine), so "counts ≥ k, joined to
  their accounts" remains a host composition anyway — HAVING alone doesn't
  delete the host loop at sites that filter AND then use the group key
  downstream; it only trims the transfer.
- Every IR arm here is forever and pays the full oracle tax by explicit ruling:
  "every accreted feature pays this project's full oracle + differential +
  fuzz cost — cheap in a Datalog engine, ruinous here" (20-query-ir.md:212-216)
  — written about exactly this kind of accretion.
- Shape B re-litigates a Closed-by-ruling item (README.md:118-121: "listed here
  so nothing is re-litigated by accident"; README.md:178-180). Under this
  repo's constitution Shape B is not a feature proposal, it is a
  constitutional amendment.

## 4. COST

(Effort estimates to be reconciled with engine-agent findings — TBD.)

Maintenance surface for Shape A, enumerated:
new IR slot in `Query` (+ fingerprint, + serialization in the conformance
lane, + `ir::render` read-side syntax), validator arms (result-type rules,
Pack/Arg refusals, program roster interaction), sink finalize filter in exec,
naive-model arm, Lean: syntax + denotation + evaluator + soundness lemma +
conformance evaluator, corpus cases (three-way + SQLite golden), fuzz
generators (the adversarial sweep + the five fuzz targets' IR generator),
`query!` grammar + renderer round-trip, TS builder + type-level result typing
(prd-S3 invariants 3/5: IR bijection), cookbook recipe + refusals-ledger
updates. Two hosts' surfaces by standing law (Rust `bumbledb-query`, TS
`@bjornpagen/bumbledb`).

Do-nothing cost: the documented idiom (host fold) at the known sites —
detailed per-site in § workload (TBD).

## 5. Workload evidence (primer) — subagent findings

Repo swept: every aggregate call of the TS SDK surface (`r.count/countDistinct/
sum/min/max/argMax/argMin/pack`, defined at bumbledb `ts/src/query/select.ts:57-102`)
across all of `/Users/bjorn/Documents/primer/src`. **All aggregate call sites in
the entire repo live in two files**: `store/gates.ts` (5 queries) and
`store/derive.ts` (3 queries). (Drizzle/Postgres HAVING hits under
`src/db/queries/**` are a different database.)

### 5.1 The complete census of host-fold sites — exactly four, one module

| # | Site | Query + fold | HAVING shape needed |
|---|------|--------------|---------------------|
| 1 | `positionGapGate` — gates.ts:663 + 681 | `select("program", r.max("maxPos"), r.count())` then `.filter(row => row.maxPos !== row.count)` | **agg vs agg, Ne** (same group) |
| 2 | `riUnderCoveredGate` — gates.ts:598 + 624 | `select("holder", r.count())` then `.filter(row => row.count < 2n)` | **agg vs literal, Lt** (floor is doctrinal `2`, gates.ts:612-613; a param would serve if configurable) |
| 3 | `terminalFormGate` — gates.ts:693,701 + 724,737 | `argMax(terminalToi, pos)` per program then `row.terminalToi !== "FactSystem"` / `!== "CognitiveRoutine"` | **argMax-carried payload vs literal, Ne** — a distinct shape (filter on the value the Arg restriction carries, not a fold result) |
| 4 | `courseMetaCardinalityGate` — gates.ts:1375 + 1390-1398 | whole-relation `count()` then host maps the absent row to `{count: 0n}` and passes iff `count === 1n` | **agg vs literal, Ne**, plus irreducible host handling of the empty-input no-row case |

The three `derive.ts` aggregate sites are NOT host folds (derive.ts:139, 292:
Arg outputs feed set-diff derivations with no comparison; derive.ts:785 +
1077-1083: a count feeds a host solver as a tie-break weight). HAVING buys
nothing there.

Empty-group semantics are load-bearing at two sites and already handled the
house way: `riZeroCoverageQuery` (gates.ts:574-583) is a separate negation
query because "an empty group yields no row, never a zero" (gates.ts:608-610);
site 4's zero case is a host witness. **HAVING does not subsume either** —
groups with no bindings produce no row to filter under any coherent design.

### 5.2 Pain assessment

All four folds are one-liner `.filter()`s over tiny result sets — one row per
program (group comments run to ~g238, so low hundreds), per RI holder, or one
row total; underlying ledgers are hundreds of facts per generation
(gates.ts:544-545). The gates re-run on every witnessed write, but O(hundreds)
of host comparisons per settle is noise. **No performance or correctness pain
at any site; the case is purely vocabulary/doctrine.** The consumer runs an
explicit purity regime: host post-processing is "legal ONLY for comparisons the
query IR fences out (aggregate-vs-aggregate comparison has no IR spelling);
each instance carries the citation" (gates.ts:23-27; per-site citations at
gates.ts:606-608 and 671-673 name aggregate comparison specifically — a
standing register of exactly this gap, maintained as doc-comment discipline
under PRD-09's audit rule, gates.ts:8-10). No TODO/wish comments exist.

Shapes by evidence strength: agg-vs-literal Lt/Ne (2 sites), agg-vs-agg Ne
(1 site), argMax-payload-vs-literal Ne (1 site / 2 queries). **Agg-vs-param
has no current witness.** Note gates.ts:712-713 records a second, independent
limitation at site 3 (Arg restriction is single-rule ⇒ host merge across
disjuncts) that HAVING alone would not lift.

## 6. Lean model detail — subagent findings

(All paths under `lean/`; line numbers per main @ 8e30387e.)

### 6.1 How aggregates are modeled

- The rule IR has no aggregate node: `Term = var | param | paramSet | lit |
  measure` (`Query/Syntax.lean:176-181`); `Rule.finds : List VarId`
  (232-236). Head shape degenerates to arity; aggregates are PRD-05 folds over
  the binding sets the denotation defines (recorded narrowing, Syntax.lean:15-19).
- `AggOp` lives in `Query/Aggregates.lean:1741-1751` (count, countDistinct,
  sum, min, max, pack, argMax, argMin, measureFold); theorems are stated over
  the underlying folds and sets, not by recursion over `AggOp` (66-72).
- The core: `aggAnswers` (Aggregates.lean:1569-1574) — an ∃-witnessed set of
  answer tuples, `fold (keyTuple keys σ) (Group …)`; groups fiber over
  evaluated head values. `empty_global_no_answer` (1586-1592): empty binding
  set ⇒ empty answer set, never a zero row (refused SQL reading:
  `Countermodels.lean:543-591`).

### 6.2 Stratification and the fold fence

- `Program.StratifiedBy` (`Query/Syntax.lean:506-514`) has TWO edge kinds only
  (`EdgeKind = positive | negated`, 476-479) — and the doc line beside it:
  "**Fold-input is UNREPRESENTABLE at this level** … folds read a finished
  output fixpoint." Aggregation-through-cycle is forbidden by
  unrepresentability, not by a check (Syntax.lean:106-115: a fold-input edge
  "has no writable syntax"; the Rust validator's `AggregationThroughCycle` is
  the engine-side mirror).
- Stratification is spent as a theorem premise: `program_eval_sound`
  (`Exec/Fixpoint.lean:1518-1525`) takes `hstrat : p.StratifiedBy strat`;
  `stratumOp_mono` (401) cashes the strict `<` on negated edges.

### 6.3 Totality/decidability status — clean, and what HAVING costs

- Zero `partial def`, zero `sorry`, zero `axiom` anywhere in `lean/`
  (grep-verified; enforced by `scripts/lean.sh` batteries and `lean/README.md:104-112`).
  The fixpoint is fueled with a PROVED fuel bound (`fueledLoop`,
  Exec/Fixpoint.lean:1057-1061; `missingCount_le` 1070; `fueledLoop_fixpoint`
  1079); `evalProgram` 1490; `program_den_finite` 1535.
- Scalar comparison semantics are directly reusable for HAVING: denotational
  `cmpDen` (`Query/Denotation.lean:408-416`), executable `compHoldsB` with
  `compHoldsB_iff` (1116-1121), `Decidable` order instances (918-957).
- **Shape A is semantically cheap in the model**: aggregate outputs are
  ordinary `Value`s in an `AnswerTuple`, so boundary HAVING is
  `fun t => t ∈ aggAnswers … ∧ φ t` with `φ` built from `cmpDen` — a new
  definition reusing comparison semantics wholesale, **no new stratum** (the
  fold already sits strictly after the finished fixpoint), no NULL story
  (empty groups never produce a row), no new axioms, totality preserved. One
  design note: body comparisons are ∃-witnessed through `Term.selects`; a
  HAVING comparison runs over finalized scalars — `cmpDen` applied directly.
- **Shape B crosses the recorded fence**: it is exactly the fold-input edge
  with "no writable syntax" (Syntax.lean:472-475). It needs a third `EdgeKind`
  with the strict-`<` discipline (the negation pattern exists to copy), plus a
  `PRule` head grammar beyond `finds : List VarId` — the type that carries the
  creation-quarantine gravestones verbatim (Syntax.lean:72-84, 399-403) — and
  a re-proof of the finiteness/soundness stack over the new head shape. Major
  surgery at the model's load-bearing definitions.

### 6.4 Three-way conformance extension

The three oracles: Rust engine, Rust naive model, Lean executable denotation
(`conformance/README.md:313`; SQLite is a fourth, partial attestor on
`program-*` cases). Corpus: `lean/conformance/cases/*.json` — 200 seeded + 19
hand query cases, 24 judgment, 3 program cases. Aggregates serialize as
`{"agg":{"op":…,"over":n}}` finds (decoder `Conformance.lean:352-375`,
serializer `crates/bumbledb-bench/src/conformance.rs:590-635`); seven aggregate
hand cases exist today. Extending for Shape A is mechanically small: one new
JSON key per rule, a decoder arm, a filter after `projectGroup`/
`projectUnionGroup` (Conformance.lean:655, 739), the mirrored serializer arm,
new hand/seeded cases — but the gate law (`lean/README.md:80-83`) requires the
`aggAnswers`-level filter definition AND its theorems to land in the same
commit as the engine arm.

No occurrence of "HAVING"/"post-aggregate" anywhere in `lean/`; the closest
recorded statements of WHY aggregates are terminal are the creation quarantine
(`Query/Aggregates.lean:34-49`) and the fold-input unrepresentability notes.

## 7. Engine detail — subagent findings

(All paths under the bumbledb worktree; line numbers per main @ 8e30387e.)

### 7.1 IR and the closest precedent

- `AggOp` ir.rs:192-227; `FindTerm` ir.rs:234-257; `HeadOp`/`HeadTerm`
  ir.rs:280-318. Aggregates appear in exactly one place: `Rule::finds`
  (ir.rs:422). No body position for an aggregate exists anywhere.
- `Term` (ir.rs:133-157) has no arm that could reference an aggregate output;
  `Term::Measure(VarId)` (ir.rs:144-157) is the precedent for a computed value
  legal on a comparison side only, never bindable.

### 7.2 Validation — HAVING is NOT expressible today, confirmed at the code

- `AggregateInteriorPredicate` — `ir/validate/validate.rs:180-191`: any
  non-output predicate with a `HeadTerm::Aggregate` is a typed refusal
  (`MeasureInteriorPredicate` at 198-204; rationale in
  `crates/bumbledb/src/error.rs:877-911` — interior predicates are transient
  word-row tables; folds materialize only at the output finalize; the Lean cut
  `PRule.finds : List VarId` cannot represent an interior fold head). So
  "aggregate rule feeds a comparison rule" is refused by name — the two-rule
  route (Shape B) is the expensive road, requiring a new executable class
  (fold finalize into a `TransientImage`) plus the Lean head-grammar surgery
  of § 6.3.
- Strata judge: `stratify()` `ir/validate/strata.rs:69-166`;
  `AggregationThroughCycle` strata.rs:136-150; `MeasureInRecursiveHead`
  151-158. Boundary HAVING (Shape A) touches none of this — no stratification
  interaction; the filter runs at the output predicate's finalize.
- Aggregate roster: `check_finds()` `ir/validate/finds.rs:84-261`; result
  types come from `Predicate::derive` (finds.rs:18-59). Comparison legality:
  `check_comparisons()` `ir/validate/context.rs:645-662`,
  `comparison_shape()` context.rs:690+.
- A HAVING slot must be **head-level, not rule-level** (aggregates fold across
  rules — the union regime — so a per-rule HAVING is semantically incoherent);
  natural home beside `head` on `Query`/`PredicateDef`. LHS restricted to fold
  outputs (Sum/Min/Max/Count/CountDistinct/AggregateMeasure; Pack and Arg
  positions excluded — no single per-group scalar without a further ruling);
  RHS literal/param/another fold position, type-equal; all fold outputs are
  U64/I64 so order operators apply cleanly; the usual self/constant refusals.

### 7.3 Exec — the insertion point is nearly free

Rule bodies plan head-agnostically; the aggregate difference is the sink
(`EitherSink::Aggregate`, `api/prepared/build.rs:1048-1068`; `AggregateSink`
`exec/sink.rs:303-424`). Output materializes at
`AggregateSink::finalize_into` — `exec/sink/aggregate/finalize.rs:25-73`,
group row assembled then emitted at finalize.rs:70. HAVING = one word
comparison per group row before that `emit` (encodings are order-preserving
u64/biased-i64 words, so Lt/Gt are raw word compares; params resolve to words
at bind). Zero planner change; no allocation; no fingerprint ripple (the TS
cross-host lock pins the schema hash only, `ts/crate/src/fingerprint_lock.rs`).

### 7.4 Notation, corpus, fuzz

- `query!`: grammar at `crates/bumbledb-query/src/lib.rs:10-56`; named head
  positions (`total: Sum(x)`, lib.rs:594-630) already give the vocabulary a
  HAVING clause would reference; renderer `ir/render.rs:209-232` and round-trip
  goldens move together.
- TS: aggregate constructors `ts/src/query/select.ts:57-114`; comparisons
  `ts/src/query/atom.ts` (OrderSide atom.ts:356); a `.having(...)` typed from
  select-entry result types + lowering in `ts/src/query/lower.ts` (agg → IR at
  1900-1941).
- Corpus: 270 cases in `lean/conformance/cases/`; seven aggregate hand cases;
  IR-verbatim JSON — HAVING = a new query field + Lean decoder arm + a group
  filter in the glue + boundary hand cases (all-groups-filtered → empty set;
  param RHS; agg-vs-agg; multi-rule union head).
- Fuzz: `fuzz/src/query.rs` runs three-way parity + a hostile-IR arm with a
  TOTAL `ValidationError` census match (new variants are compile errors there
  — mechanical but mandatory); hostile gen `fuzz/src/irgen.rs:321-407,488-493`;
  valid gen `crates/bumbledb-bench/src/querygen.rs:277,309,344` + naive-model
  arm + SQLite `HAVING` translation (natively expressible — three-way parity
  holds, a real asset).

### 7.5 Effort by layer (engine agent's estimates, reconciled)

| Layer | Effort | Note |
|---|---|---|
| IR arm | S | head-level `having` list; pure data |
| Validator | M | position roster, type rules, Pack/Arg exclusions, ~4-6 typed errors |
| Planner/exec | S | filter before finalize.rs:70 + bind-time param words |
| `query!` + renderer | M | grammar clause, goldens, compile-fail tests |
| TS builder | M | `.having` + type-level result typing + lowering + marshal |
| Naive model | S | group filter after its fold |
| Lean (Shape A) | M | decoder + glue filter S; the same-commit theorem law (filtered-`aggAnswers` definition + lemmas) makes it M |
| Corpus + fuzz | S/M | JSON field, hand cases, irgen/querygen knobs, SQLite translate, census lines |
| Docs/cookbook | S | 20-query-ir §, refusals-ledger edits, a recipe |

Total for Shape A: a solid **M** spread across ~9 permanent surfaces. Shape B:
**L/XL** (new executable class + Lean head-grammar re-proof) and
constitutionally blocked (§ 3).

## 8. VERDICT

**Reject Shape B; defer Shape A with a recorded trigger.** The reasoning, laid
bare:

1. **Shape B is not a feature request, it is a constitutional amendment.** It
   reverses the Closed-by-ruling creation quarantine
   (docs/architecture/README.md:178-180, "listed here so nothing is
   re-litigated by accident"), requires a third `EdgeKind` and a new `PRule`
   head grammar in Lean (§ 6.3), a new executable class in the engine
   (§ 7.2), and — decisively — **no workload site needs it**: all four primer
   sites filter final answers; none feeds a filtered aggregate back into a
   join (§ 5.1). Reject.

2. **Shape A is genuinely cheap and genuinely aligned in mechanism** — the
   measure precedent's admission clause fits it exactly (typed positions,
   boundary-only, one recorded decision, 20-query-ir.md:681-683); the exec
   insertion is one word-compare before an existing emit; SQLite covers it as
   an oracle natively; no stratification, totality, or axiom questions arise
   (§ 2, § 6.3). If any aggregate-comparison feature ever lands, this is the
   correct and complete design, and this report is the PRD skeleton for it.

3. **But the evidence does not clear the house's own bar for shipping it
   now.** The repo's posture for post-processing is explicit: host does it
   until a MEASURED budget violation (README.md:70-72, the top-k OPEN item —
   the exact same class). The measured pain is zero: four one-liner
   `.filter()`s over row counts in the low hundreds, re-run per settle, with
   maintained doctrinal citations rather than TODOs (§ 5.2). Worse for the
   feature's ROI: the fold-output-only cut covers just 3 of the 4 sites
   (site 3 filters an Arg-carried payload — its own ruling; and its real
   blocker is Arg's single-rule restriction, which HAVING doesn't lift),
   site 4 keeps its host step for the empty-input row regardless, and
   agg-vs-param — the shape parameterized gates would actually want — has no
   witness at all. The census law ("nothing shipped without a sighting",
   00-product.md:68-70) is technically satisfied but thinly: one agg-vs-agg
   sighting, two agg-vs-literal.

4. **The do-nothing cost is a citation register, not a budget line.** The
   consumer's purity regime (gates.ts:23-27) already names
   aggregate-comparison as a fenced-out class and audits each instance —
   that is the system working as designed, the same way recipe 24's closure
   idiom carries the chain-window fence.

**Recorded-trigger recommendation** (mirroring the OPEN-item convention): admit
Shape A when EITHER (a) a measured materialize-then-filter latency- or
transfer-budget violation appears at a real site (groups numerous, survivors
few), OR (b) the host-fold register grows past its current one-module footprint
(new modules/repos accumulating cited HAVING-shaped folds), OR (c) an
agg-vs-param sighting lands (a configurable threshold gate) — that shape is
where engine-side judgment starts paying for real (bind-time thresholds over
prepared queries, not re-authored host lambdas). Until then, the host fold
remains the documented idiom (20-query-ir.md:293-294), and the four citations
in gates.ts are its ledger.
