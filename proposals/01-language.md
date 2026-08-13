# 01 — Language: Query, interiors, one Rec

Normative surface after the cut. Semantics live in `02-lean.md` and are cited, not restated. This file fixes shape, mapping, and the roster of new refusals. Host-sugar tokens (`query!`, TS, C++) live in `04-bindings-docs.md`; they lower to this IR. They are not SQL, and this IR is not a CTE dialect.

The three knobs: `proposals/README.md`. This file states the **language**, then **this-cut narrowing**, then the roster. Mutual-linear is not “the language refuses”; it is this-cut scope (`Option<Rec>`, one name) so Tarjan dies with Program. `05-cutover.md` lists it under OPEN / other cuts — the same fact, not a contradiction.

## The language

A `Query` is three parts, in evaluation order:

1. **Named interiors** — conjunctive rule-lists, each a finite CQ (union of CQs), evaluated **once**, not an lfp. They form a DAG: declaration order is topological order. An interior may read EDB and any **already-finished** derived table (earlier interiors; after a rec SCC has closed, that rec — inlining-equivalent, OPEN this cut). Bodies may negate finished derived tables (a finished set is a set). Heads are bound variables only (creation quarantine at the interior boundary: no fold, no measure find — those live on main).
2. **Recursive SCCs** — least fixpoints over named derived relations. A recursive SCC's operator is monotone iff nothing negates or folds the SCC's **own** table (`odd_not_monotone` / `odd_no_fixpoint`); this cut's roster refuses all negation in the SCC regardless (`NegationInRec` — the monotone finished-table case is a this-cut refusal, OPEN, not this wall). Heads are range-restricted (bound variables). The denotation of one SCC is `lfpS` of its operator, not a fuel parameter. Recursion is never the answer. Sequential SCCs (A closes, B reads A) are stacked lfps, same class — OPEN this cut. Mutual-linear (several names, one SCC, each rule ≤1 rec atom) is one joint lfp — OPEN this cut.
3. **Main** — today's query: one head, ≥1 rule, folds, measures, negation, the full per-rule roster. Main reads EDB and every finished derived table. `evalQuery` is main `rulesAnswers` over that environment.

Params are **query-global**. Variables are **rule-scoped**. `AtomSource` names a stored relation or a derived table (an interior or a rec). `RelId` / `RelationId` never puns with `InteriorId`. Statements still quantify over stored relations only.

That is the object. Walls apply to every recursive SCC, in this cut and in any later one. The IR below is the **narrowing**: one linear SCC, interiors as a prefix that cannot see rec, main last.

## This cut (the IR we implement)

At most **one** recursive SCC, **one** name, **linear** arms. Interiors, then that rec, then main. No named interior of a finished rec (inline into main — equivalent). No second rec after the first. No mutual names in one SCC.

```rust
Query {
    interiors: Vec<Interior>,   // DAG, declaration order; no count cap
    rec:       Option<Rec>,     // at most one recursive SCC
    head:      Vec<HeadTerm>,   // MAIN answer shape
    rules:     Vec<Rule>,       // MAIN; ≥1 at validate, ≤ MAX_RULES
}

Interior {
    head:  Vec<HeadTerm>,       // bound-variable positions only (every HeadTerm::Var)
    rules: Vec<Rule>,           // ≥1, ≤ MAX_RULES; union; bodies: EDB ∪ earlier interiors
}

Rec {
    head: Vec<HeadTerm>,        // bound-variable positions only
    base: Vec<Rule>,            // ≥1; no self atom
    rec:  Vec<Rule>,            // ≥1; each arm: exactly one positive self-atom
                                // base.len() + rec.len() ≤ MAX_RULES  (one SCC, one cap)
}

AtomSource = Edb(RelationId) | Interior(InteriorId)

InteriorId(u32)  // index: interiors[i] has InteriorId(i);
                 // the Rec, if present, has InteriorId(interiors.len() as u32)
```

Lean spells the same list `interiors`, the same `Rec`, the same `InteriorId` numbering. There is no engine field named `with`.

`Rule`, `Atom`, `Term`, `FindTerm`, `ConditionTree`, `MAX_RULES` (16), `MAX_CONDITION_DEPTH` (64) are unchanged. `Atom.source` replaces `AtomSource::Idb(PredId)` with `AtomSource::Interior(InteriorId)`. An interior atom's `FieldId(i)` addresses that derived head position `i` — positional, never nominal, today's Idb reading verbatim.

**No pun.** `RelId` / `RelationId` names stored relations. `InteriorId` names derived tables (interiors and the rec). `PredId` dies. Never encode an `InteriorId` as a `RelId` (the even/odd coding dies with `PAtom.code`).

Params remain **query-global**: one binding surface across every interior rule, every rec arm, and the main query. `ParamIdGap` / scalar-set conflict are judged once across that whole surface. Variables remain **rule-scoped**.

**IR vs plan.** Validate refuses a Query with empty `Query.rules` (`EmptyRuleSet`). Prepare may still land `PreparedBody::Empty` when every **main** rule is statically dead — today's empty plan. That is not an empty IR. Interior preamble still runs when `interiors` is nonempty (`03-engine.md`).

`Query` is the only value `Db::prepare` accepts. There is no `Program`, no `PredicateDef`, no `PredId`, no `ProgramRef`, no `From<Query> for Program`.

## No `MAX_CTES`

`MAX_PREDICATES = 16` and `MAX_RULES = 16` were Program-era “keep programs query-shaped.” A **second** 16 on how many named interiors you wrote is not a complexity wall. Each interior is a finite CQ evaluated once (not an lfp). Work is bounded by the existing rule-size law plus result / tuple budgets, not by an interior counter.

**Do not invent `MAX_CTES`.** Do not export `MAX_INTERIORS`. Do not size an obs array to 16 interiors. `TooManyCtes` / `TooManyPredicates` have no successor that counts interiors.

If query-shaped-ness still needs a number, it is the existing `MAX_RULES` (16), applying uniformly to every rule-list in interiors ∪ rec ∪ main:

- Each `Interior.rules` independently ≤ `MAX_RULES` (structural `TooManyRules`, then `DnfExceedsRules` on that list's DNF).
- Main `Query.rules` independently ≤ `MAX_RULES` (today's query).
- The rec SCC is **one** SCC, one pool: `base.len() + rec.len() ≤ MAX_RULES` (`TooManyRules { count }` — `count` is the sum). A 17-rule base is this error; do not also run a redundant per-arm structural cap. DNF: `dnf_width(base) + dnf_width(rec) ≤ MAX_RULES` (`DnfExceedsRules { produced: sum, cap: MAX_RULES }`). Do **not** allow 16+16 of DNF.

Judged in declaration order: each interior, then the rec pool, then main. First failure wins. Payloads stay `{ count }` / `{ produced, cap }` — no `PredId`.

The expensive object is **rec answer size** (`DEFAULT_FIXPOINT_TUPLES` = 10⁷, result bytes), not how many named interiors you wrote. Complete-graph TC is n² whether the syntax is linear or not; seed the frontier (host demand). Rounds default `2¹⁶` is diameter headroom; size is the wall.

**`InteriorId` construction never panics.** Count with `usize`. `InteriorId` is `u32`, the same width as `RelationId`. If `interiors.len() + rec.is_some() as usize` does not fit in `u32`, that is a representation error (`InteriorIdOverflow`) — id-width, not a product cap of 16. Hostile `interiors.len() == 100_000` is a legal Query: each interior is still ≤ `MAX_RULES`, still eval-once. Judge overflow **before** any `InteriorId(u32)`. Never `u32::try_from(interiors.len()).unwrap()`.

C++ / TS sugar may still cap named interiors at 4 (`max_query_rules` precedent). That is sugar. The engine does not shrink to match it, and it is not `MAX_CTES`.

## Union

Unchanged at the rule-list: several conjunctive rules, one head, set semantics, **one sink per rule-list**, spanning seen-set. There is one sink per `Interior`, one sink for the `Rec`, one sink for the main query — not one sink for the whole `Query`. No merge node concatenates derived tables into the answer; later clauses read a finished derived table as an atom. `lean/Bumbledb/Query/Denotation.lean: mem_queryAnswers` retargeted as `mem_rulesAnswers` (`02-lean.md`). There is no `UNION` / `UNION ALL` keyword in the IR or the notation. Bags are unrepresentable. The SQLite translator may emit SQL `UNION` (which is ∪ under `SELECT DISTINCT`) — that is translator spelling, not an IR node (`04-bindings-docs.md`).

## Interiors — named, DAG, eval once

Non-recursive. Declaration order **is** the topological order. Interior `i` may read `Interior(j)` only for `j < i`. A self-read or a forward read is `InteriorNotPrior`.

**This cut:** interiors cannot read the rec. The rec's `InteriorId` is `interiors.len()`, which is never `< i`. That is the three-phase IR (`interiors` is a prefix list, `rec` is `Option`), not a wall. A named interior of a **finished** rec is inlining-equivalent and is OPEN (`05-cutover.md`). Hostile IR with an interior after rec is unrepresentable this cut — not an `InteriorNotPrior` on main.

Bodies: EDB atoms, earlier interiors, negation of either (a finished interior is a set — anti-join is ordinary), conditions, membership, params. **Measure comparisons in an interior body are legal** (filters; a ray raises `MeasureOfRay` after that interior, like any query). Rec bodies refuse every measure site. Heads: bound variables only — no `Aggregate`, no `Measure`, no `AggregateMeasure`. That is today's executable-class item (`AggregateInteriorPredicate` / `MeasureInteriorPredicate`) restated for interiors: an interior is a projection-shaped word-row table. Measure **finds** and folds live on the **main** query, over finished interiors/rec.

Eval once, in declaration order, into a transient image, before the rec if present, then the main query. A Query whose `rec` is `None` **never** enters the reach driver — interiors-only is the ordinary rule loop plus an interior preamble. No watermark, no round budget, no `FixpointBudgetExceeded`.

Empty `interiors` is legal. An individual `Interior` with zero rules is `EmptyInterior` (not `EmptyRuleSet` — that name is the main query).

## Rec — this cut: at most one linear SCC

One name. Two lists, so mixed arms are a roster item rather than a classification:

- **Base arms** (`Rec.base`): no atom — positive or negated — whose source is the rec's `InteriorId`. Extra EDB joins and earlier interiors are legal. **Negation is illegal in the whole rec SCC** (base and rec arms, EDB or interior or self): `NegationInRec`. Self-negation is the wall; EDB / earlier-interior negation is monotone and refused this cut — one roster item either way, OPEN in `05-cutover.md`. Measure terms (find, binding, comparison) are illegal in the whole rec SCC: heads → `MeasureInInterior`; rec **bodies** → `MeasureInRec`. Aggregates are already unwritable in a bound-var head; a fold find on an interior or rec head is `AggregateInInterior`. There is no `AggregationInRec` variant — the fold is on the head, so it is `AggregateInInterior`.
- **Rec arms** (`Rec.rec`): each arm has **exactly one** positive self-atom (`Interior` = rec's id). Zero is `RecArmMissingSelf`. Two or more is `NonlinearRecArm`. The self-atom is never negated (already `NegationInRec`). Extra EDB and earlier-interior joins on the same arm are legal — that is where Free Join earns its keep (primer's step rule is several EDB atoms plus one rec).

Both lists nonempty:

- `EmptyRecursiveBase` if `base` is empty. **Math:** a positive self-atom against an empty table derives nothing, so `rec(∅) = ∅`, so `T(∅) = ∅`, so the lfp is empty (`02-lean.md: reachOp_empty`). The roster refuses a constantly-empty rec (write no rec, or write a nonempty base). This is not “SQLite refuses the CTE.”
- `EmptyRecursiveStep` if `rec` is empty. That is an interior. Write an interior, not a rec.

The rec head is bound variables only — creation quarantine restated: `program_den_finite`'s premise, retargeted as `reach_den_finite`. The chain-window class (`w = w₁ ∩ w₂` in a rec head) stays outside, OPEN, unchanged.

The main query may join, anti-join, `Sum`, `Pack`, and measure the **finished** rec. It does not grow the recursive name: `reachOp` reads only `Rec.base` / `Rec.rec`. Unwritable from main.

Identity projection is never implicit. The rec is **never** the answer predicate (`02-lean.md: evalQuery_empty_rules` — empty main denotes `∅` even when `reachDen` is huge). `(c) | reach(c);` is a main rule, required, as today's bare output rule was required. A query of only interiors / rec and no bare rule is `EmptyRuleSet` — and in `query!`, a compile error. Today's programs whose `output` **was** the recursive predicate recut as rec plus an identity main of the same arity.

Linear is this cut because k=1 Δ-variant × Free Join at 10⁷ / 10 ms, not because SQLite allows one self-reference (`00-manifesto.md`). Several rec **arms** of one name (union) are legal. Nonlinear is OPEN (`05-cutover.md`), refused here as `NonlinearRecArm`.

## Main query

Today's query: one head, ≥1 rule, ≤ `MAX_RULES`, DNF, head alignment, folds, measures, negation, the full per-rule roster. Atoms may read EDB, any interior, and the rec if present. An out-of-range `InteriorId` is `UnknownInterior` (today's `UnknownPredicate` screen, spent as `wellFormed_reads_real` is spent — `02-lean.md`).

## Notation (host sugar, not SQL)

The notation stays the statement grammar's query side. Keywords are added so Program cannot sneak back through named heads. Tokens are host sugar (`query!` / `ir::render`); the IR fields are `interiors` / `rec`. Do not read this block as `WITH RECURSIVE`.

```text
query     := interior* recblock? main
interior  := 'interior' pred '(' head ')' '|' body ';'
recblock  := 'recursive' pred '(' head ')' '|' body ';'
main      := barerule+
barerule  := '(' head ')' '|' body ';'
pred      := lowercase ident
```

`body` / `head` / `atom` / `cond` otherwise unchanged from `20-query-ir.md` § the query notation, except: a body atom naming `pred` is an `Interior` atom (ordered-dense or indexed, same two spellings, never mixed). Relations remain UpperCamel. `and` / `or` / `interior` / `recursive` are reserved.

**Grouping — one law, every surface.** Derived names are unique. Multiple rules of one interior or one rec are several builders / several notation lines of **that one name**, not a second derived table.

| Surface | How one name gets several rules | Second declaration of the same name |
|---|---|---|
| `query!` | Consecutive `interior pred(...)` lines → one `Interior`. Consecutive `recursive pred(...)` lines → one `Rec`; a line whose body has an **atom** (positive or negated, either spelling) naming `pred` is a rec arm, else a base arm. The macro classifies, then emits `base` / `rec`. | Non-consecutive reuse is a compile error (write the rules together). |
| TS | `q.interior("mid", ...builders)` one `Interior`. `q.recursive("reach", { base: [...], rec: [...] })` — **arrays**. One callback is not "exactly one rec arm." | `q.interior("mid", ...)` a second time is a construction error. TS does not consecutive-union. |
| C++ | `bdb::interior<"mid">(rule_builders...)` one `Interior`. `bdb::recursive<"reach">(bdb::base{...}, bdb::rec{...})` — **two tagged packs**. | A second `interior<"mid">` is a consteval error. |

TS/C++ never scan bodies to classify base vs rec: the tagged lists **are** the IR. `query!` classifies because the text grammar has one production for both arms. A `query!` rec line that mentions `pred` only in a comment is a base arm; a line whose only self occurrence is negated is a rec arm, then `NegationInRec` / `RecArmMissingSelf` at validate.

TS/C++ also throw if `interior` / `recursive` is called after a main rule has been added, or `interior` after `recursive`. Declaration order is interiors, then rec, then main — same as `query!`.

**Compile errors (macro, spanned), exhaustive for this cut:**

- A named head **without** `interior` / `recursive` — the former Program sneak. Message names the keywords.
- Two different names under `recursive` — at most one rec SCC this cut.
- `interior` and `recursive` sharing a name.
- `interior` after `recursive`, or either after a bare rule — declaration order is interiors, then rec, then main.
- No bare rule (no main).
- `recursive` with zero base lines or zero rec lines (after classification).
- Duplicate interior names that are not consecutive-union of the same pred — names are unique; non-consecutive reuse is a compile error.

All-bare `query!` (no `interior`, no named heads) lowers to `Query { interiors: vec![], rec: None, head, rules }` — today's `ir::Query`, field for field plus two empty fields. Text-level backward compatibility for every existing non-recursive query.

Renderer: `interior p{id}(...) | ...;` then `recursive p{id}(...) | ...;` then bare main rules. Interior names stay synthesized `p{id}` — names remain a macro-local sidecar, never in the IR or the fingerprint. Round-trip goldens pin `render(lower(text))` as today.

## Mapping from today's reach programs

Cookbook 24, engine-native:

```text
reach(c) | Node(id: c), c == ?root;
reach(c) | Parent(child: c, parent: m), reach(m);
(c) | reach(c);
```

becomes:

```text
recursive reach(c) | Node(id: c), c == ?root;
recursive reach(c) | Parent(child: c, parent: m), reach(m);
(c) | reach(c);
```

Cookbook 25, fold over finished closure:

```text
sub(a) | Account(id: a), a == ?root;
sub(a) | AccountParent(child: a, parent: p), sub(p);
(total: Sum(minor)) | Posting(id, account: a, minor), sub(a);
```

becomes the same rewrite: `recursive sub(...)` on the first two lines; the `Sum` line stays bare (main). Aggregation is not in the rec SCC; it reads the finished rec. The strata-roster sentence "fold over a lower stratum" is now the ordinary sentence "main query over a finished interior."

**Primer cycle detector** (`requiresCycleQuery`): already the allowed shape — linear `reach(from, to)`, extra EDB on the step arm, output is **not** the rec. Recut 1:1. Empty output = the lattice is a DAG. Primer is a **downstream repo**: the recut is their P2.4 cutover, coordinated; the in-tree artifact is the primer-shaped `reach(x,x)` lock.

```text
recursive reach(from, to) | Produces(grp: from, capability: cap),
    Requires(consumer: to, capability: cap, state: Upheld), from != to;
recursive reach(from, to) | Produces(grp: from, capability: cap),
    Requires(consumer: mid, capability: cap, state: Upheld),
    Requires(consumer: to, state: Upheld), from != mid, reach(mid, to);
(node) | Grp(id: node), reach(node, node);
```

The step arm is several EDB atoms plus **one** rec atom — linear, Free Join's case. Main `reach(x,x)` is a join of the finished rec, not a second SCC. Do not invent a named interior of `reach` for the diagonal; the main rule *is* the diagonal.

Non-recursive interior predicate (today's fanout / view-as-Idb):

```text
mid(x) | Edge(src: x);
(x) | mid(x);
```

becomes:

```text
interior mid(x) | Edge(src: x);
(x) | mid(x);
```

This Query has `rec: None`. It prepares and executes as interiors-only. It must not touch the reach driver. That is the interior-as-eval-once law, operational.

**Interior that reads the rec.** Today's legal program `rec + non-recursive interior predicate that reads rec + output` has no this-cut interior image: interiors cannot read the rec (`InteriorId(interiors.len())` is never `< i`). Those interior rules **inline into the main query** (union of conjunctive bodies, DNF as ever). You cannot name a view of the rec this cut. Two different main-shaped queries over the same rec are two `Query` values (two prepares). OPEN: a named interior of a finished rec (inlining equivalent).

Worked example — today's `CLOSURE_ROOTS` (non-recursive `p1` anti-joins finished `p0`):

```text
-- today: p0 recursive, p1 reads p0, output = p1
-- after: rec p0, main is p1's body (anti-join of finished rec).
-- Absent field is the wildcard (no `_` term).
recursive reach(c, p) | OrgParent(child: c, parent: p);
recursive reach(c, p) | OrgParent(child: c, parent: m), reach(m, p);
(id) | Org(id: id), !reach(c: id);
```

Same answers as a named interior after rec would have given: the anti-join sees the **finished** lfp. The SQLite translator recuts from a second CTE after the rec to a main `SELECT` with `NOT EXISTS` / `LEFT JOIN ... IS NULL` (`04-bindings-docs.md`). That SQL is translator output, not the language.

**Dropped, not inlined — negation *inside* the rec SCC.** Today's recursive predicate that anti-joins a lower stratum (or EDB) during the walk is `NegationInRec`. Putting that anti-join on main is a **different** query (filter after closure ≠ filter during). Do not pretend they are the same rewrite. Two cases, named: anti-joining the rec table itself is the **wall** (`odd_not_monotone` / `odd_no_fixpoint`); anti-joining a **finished** table during the walk is monotone (`stratumOp_mono`'s stratified content today) and is dropped **this cut** — OPEN, trigger in `05-cutover.md`. The drop prices at zero today: no corpus case and no accepted test negates from inside a recursive rule. Hosts that need during-walk exclusion keep the host loop (cookbook 24's other dialect) or write the exclusion positively.

**Output was the rec predicate.** One-predicate recursive programs (`output = 0`, the rec IS the answer) become `rec` plus an identity main of the same arity. The rec table is not implicitly returned. `program-hand-closure.json` is this shape.

A today's degenerate `Program` (one predicate, no Idb) is a today's `Query`. After the cut it is `Query { interiors: [], rec: None, ... }`. There is no embedding type.

Mutual recursion, nonlinear rec, named-head-without-keyword, `Program` literals in tests — unwritable **this cut** or refused. Mutual-linear and nonlinear are OPEN (`05-cutover.md`), not walls. `04-bindings-docs.md` lists the test conversions.

## Validation roster (additions; the per-rule roster stays)

Judged in this order, after the existing query-shape checks (empty main, `MAX_RULES` / DNF per interior and per main and the rec **pool**, nesting, DNF, head-shape alignment). Roster names are `ValidationError`; `FixpointBudgetExceeded` / `MeasureOfRay` / `ResultBytesOverflow` are runtime `Error` and stay there:

1. `InteriorIdOverflow` — derived-table count does not fit `u32` (usize, before any `InteriorId`). There is **no** `TooManyCtes`.
2. Per `Interior` / `Rec` / main: existing empty-rule (`EmptyInterior` for an interior with zero rules; `EmptyRuleSet` only for empty **main**), DNF, head-alignment, per-rule roster, with `IdbSignatures` replaced by `InteriorSignatures` (sealed in declaration order — no chaotic iteration).
3. `InteriorNotPrior { interior, at }` — interior `i` reads `j ≥ i`, or any interior reads the rec id. `at` is the reading interior's id; `interior` is the illegal target.
4. `UnknownInterior { atom, interior }` / `InteriorColumnOutOfRange { atom, field }` — the well-formedness screen (`Query.WellFormed`, `02-lean.md`). Replaces `UnknownPredicate` / `PredicateColumnOutOfRange`.
5. Rec roster: `EmptyRecursiveBase`, `EmptyRecursiveStep`, `SelfInBase`, `RecArmMissingSelf`, `NonlinearRecArm`, `NegationInRec`, `MeasureInRec` (every measure site in a rec **body**: find is a head — that's item 6; binding and comparison are bodies).
6. `AggregateInInterior` / `MeasureInInterior` — fold or measure **find** on an interior or rec **head** (bound-var law). One error per shape: heads of interior or rec → `*InInterior`; rec bodies → `MeasureInRec`. Main heads keep today's aggregate/measure roster. Interior **bodies** may contain measure comparisons (not this item).
7. Query-global param unification (today's program-global pass, now the only pass).

**Canonical error names (this cut). Use these spellings everywhere — docs, tests, Bridge instruments.**

| New | Replaces / notes |
|---|---|
| *(none)* | `TooManyPredicates` dies. **No** `TooManyCtes`. Interior count is uncapped |
| `InteriorIdOverflow` | derived-table count > `u32::MAX` (id-width, not a product 16) |
| `UnknownInterior` | `UnknownPredicate` |
| `InteriorColumnOutOfRange` | `PredicateColumnOutOfRange` |
| `InteriorNotPrior` | forward/self interior read; interior reading rec (this cut) |
| `EmptyInterior` | zero-rule `Interior` |
| `EmptyRecursiveBase` / `EmptyRecursiveStep` | new. Empty base is empty lfp (math), refused as constantly-empty rec |
| `SelfInBase` / `RecArmMissingSelf` / `NonlinearRecArm` | new (k-variants die this cut) |
| `NegationInRec` | `NegationThroughCycle` for the rec SCC; main anti-join of finished rec is legal |
| `AggregateInInterior` | `AggregateInteriorPredicate` + `AggregationThroughCycle` when the fold is on an interior/rec **head** |
| `MeasureInInterior` | `MeasureInteriorPredicate` + `MeasureInRecursiveHead` when the measure **find** is on an interior/rec **head** |
| `MeasureInRec` | measure in a rec **body** (comparison / binding). Bindings may still hit `DurationInBinding` first (per-rule roster runs earlier) — both are correct; do not add a third name |
| `EmptyRuleSet` | empty **main** only |

**Deleted roster items (no replacement that preserves the shape):** `TooManyPredicates`, `TooManyCtes` (never ship it), `UnknownOutputPredicate`, `UnknownPredicate`, `PredicateColumnOutOfRange`, `NegationThroughCycle`, `AggregationThroughCycle`, `UnresolvedPredicateSignature`, `AggregateInteriorPredicate`, `MeasureInteriorPredicate`, `MeasureInRecursiveHead`. There is **no** `AggregationInRec` / `AggregationInRecCte`. Do not keep the old `*InCte` spellings as aliases.

Signature sealing: interior `i` seals from its rules against already-sealed `0..i`. Rec seals from **base** (EDB + sealed interiors — a stored column always names the type) then rec arms (self already sealed). `p(x) | p(x)` as a rec with empty base is `EmptyRecursiveBase`, not a sealing timeout. The signature fixpoint loop in `validate_program` dies.

An `Interior` atom on a Query with empty `interiors` and no rec is `UnknownInterior` — today's "Idb at the query boundary" refusal, without a Program to route through.

## What each knob refuses

**Walls (forever):**

- Negation or aggregation **through the cycle** — the rec SCC's own table under `!` or a fold. Main may anti-join / fold the finished rec. During-walk ≠ after-lfp.
- Created rec heads (chain-window stays OPEN).
- Fuel as meaning. The rec denotes an lfp; the budget is a resource abort (`03-engine.md`).
- Implicit output = last derived table. Bare rules are the output.
- A Datalog runtime: stored programs, magic sets, demand transformation, host-loop internalization. The host-loop idiom in cookbook 24 remains a **host** idiom, not an engine mode.
- Bags / `UNION ALL`.

**This cut (scope):**

- More than one recursive SCC (`rec: Option<Rec>`). Stacked sequential linear lfps are OPEN.
- Mutual names in one SCC. Mutual-linear is OPEN; refusing it here is how Tarjan / k-variants die with Program.
- Nonlinear rec arms (`NonlinearRecArm`). Trigger in `05-cutover.md`.
- A named interior of the rec (inline into main). OPEN, inlining-equivalent.
- A second eval path, a second Free Join, a `ReachProgram` type, Tarjan retained “for interior cycles” (interior cycles are `InteriorNotPrior`).
- During-walk anti-join of **finished** tables in rec arms (`NegationInRec` covers the whole SCC this cut; the case is monotone; OPEN).

**Not a refusal:** several linear rec arms of one name; extra EDB / earlier-interior joins on a linear arm; main over the finished rec (join, anti-join, fold) — cookbook 25, primer `reach(x,x)`; two independent reaches as two `Query` values (host owns composition); any number of named interiors, each a finite CQ.
