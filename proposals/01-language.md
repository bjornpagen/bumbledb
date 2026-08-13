# 01 — Language: Query, WITH, one RecCte

Normative surface after the cut. Semantics live in `02-lean.md` and are cited, not restated. This file fixes shape, grammar, mapping, and the roster of new refusals.

## The one execute target

`Query` is the only value `Db::prepare` accepts. There is no `Program`, no `PredicateDef`, no `PredId`, no `ProgramRef`, no `From<Query> for Program`.

```rust
Query {
    with: Vec<WithDef>,         // 0..=MAX_CTES, declaration order = DAG order
    rec:  Option<RecCte>,       // at most one; counts against MAX_CTES
    head: Vec<HeadTerm>,        // today's head — the MAIN query's answer shape
    rules: Vec<Rule>,           // today's rules — the MAIN query; ≥1 at validate, ≤ MAX_RULES
}

WithDef {
    head:  Vec<HeadTerm>,       // bound-variable positions only (every HeadTerm::Var)
    rules: Vec<Rule>,           // ≥1, ≤ MAX_RULES; union; bodies: EDB ∪ earlier WITH
}

RecCte {
    head: Vec<HeadTerm>,        // bound-variable positions only
    base: Vec<Rule>,            // ≥1; no self atom
    rec:  Vec<Rule>,            // ≥1; each arm: exactly one positive self-atom
                                // base.len() + rec.len() ≤ MAX_RULES  (one CTE, one cap)
}

AtomSource = Edb(RelationId) | Cte(CteId)

CteId(u16)  // index: with[i] has CteId(i);
            // the RecCte, if present, has CteId(with.len() as u16)
```

Lean spells `Query.with` as `views` (`with` is a keyword). Same list, same order, same `CteId` numbering.

`Rule`, `Atom`, `Term`, `FindTerm`, `ConditionTree`, `MAX_RULES` (16), `MAX_CONDITION_DEPTH` (64) are unchanged. `Atom.source` replaces `AtomSource::Idb(PredId)` with `AtomSource::Cte(CteId)`. A `Cte` atom's `FieldId(i)` addresses that CTE's head position `i` — positional, never nominal, today's Idb reading verbatim.

**No pun.** `RelId` / `RelationId` names stored relations. `CteId` names WITH/rec tables. `PredId` dies. Never encode a `CteId` as a `RelId` (the even/odd coding dies with `PAtom.code`).

Params remain **query-global**: one binding surface across every WITH rule, every rec arm, and the main query. `ParamIdGap` / scalar-set conflict are judged once across that whole surface. Variables remain **rule-scoped**.

**IR vs plan.** Validate refuses a Query with empty `Query.rules` (`EmptyRuleSet`). Prepare may still land `PreparedBody::Empty` when every **main** rule is statically dead — today's empty plan. That is not an empty IR. WITH preamble still runs when `with` is nonempty (`03-engine.md`).

## MAX_CTES = 16

`MAX_PREDICATES` (16) is **not** a rename of the same count. Today's cap counted **predicates including the output**. `MAX_CTES` counts `with.len() + rec.is_some() as usize` and **excludes** the main query. A query may therefore carry 16 CTE tables plus one answer sink — a one-slot relaxation.

**Why exclude main, and why 16 not 15.** The cap's reader is CTE materialization: one `TransientImage` per WITH/rec, pin-at-prepare floors, allocation high-water on those pools. The main query is today's `Query.rules` — it already exists, already has `MAX_RULES`, already has the head-owned sink, and does not get a CTE image. Shrinking to 15 to "keep 16 rule-lists" would be counting the wrong object. The product decision is unamended: queries stay query-shaped; the engine is never a rule-program runtime. Cookbook 24–25 use one rec and zero views. A DAG of sixteen named views is already past every sighted workload; past the cap is `TooManyCtes`. C++ / TS sugar caps stay sugar (`max_program_recs = 4` becomes `max_ctes = 4` in the C++ builder). The engine cap does not shrink to match sugar.

**`CteId` construction never panics.** Count with `usize`. `TooManyCtes` (and the implied `count > u16::MAX`) is judged **before** any `CteId(u16)`. After the cap, `i < MAX_CTES ≤ 16` so `CteId(i as u16)` is in range. Hostile `with.len() == 100_000` is a typed error, not `try_from(...).unwrap()`.

**`MAX_RULES` (16), per list, with one pooled rec CTE.** Each `WithDef.rules` and the main `Query.rules` are capped independently at 16 — today's per-predicate cap (structural `TooManyRules`, then `DnfExceedsRules` on that list's DNF). The rec CTE is **one** CTE, one pool:

- Structural: `base.len() + rec.len() ≤ MAX_RULES` (`TooManyRules { count }` — `count` is the sum). A 17-rule base is this error; do not also run a redundant per-arm structural cap.
- DNF: `dnf_width(base) + dnf_width(rec) ≤ MAX_RULES` (`DnfExceedsRules { produced: sum, cap: MAX_RULES }`). Do **not** allow 16+16 of DNF.

Judged in declaration order: each WITH, then the rec pool, then main. First failure wins. Payloads stay `{ count }` / `{ produced, cap }` — no `PredId`.

## Union

Unchanged at the rule-list: several conjunctive rules, one head, set semantics, **one sink per rule-list**, spanning seen-set. There is one sink per `WithDef`, one sink for the `RecCte`, one sink for the main query — not one sink for the whole `Query`. No merge node concatenates CTE tables into the answer; later clauses read a finished CTE as an atom. `lean/Bumbledb/Query/Denotation.lean: mem_queryAnswers` retargeted as `mem_rulesAnswers` (`02-lean.md`). There is no `UNION` / `UNION ALL` keyword in the IR or the notation. Bags are unrepresentable. The SQLite oracle may emit SQL `UNION` (which is ∪ under `SELECT DISTINCT`) — that is translator spelling, not an IR node.

## WITH — named views, DAG, eval once

Non-recursive. Declaration order **is** the topological order. WithDef `i` may read `Cte(j)` only for `j < i`. A self-read or a forward read is `CteNotPrior`. WITH cannot read the rec CTE: the rec's `CteId` is `with.len()`, which is never `< i`. Unwritable, not a special case.

Bodies: EDB atoms, earlier WITH, negation of either (a finished view is a set — anti-join is ordinary), conditions, membership, params. **Measure comparisons in a WITH body are legal** (filters; a ray raises `MeasureOfRay` after that CTE, like any query). Rec CTE bodies refuse every measure site. Heads: bound variables only — no `Aggregate`, no `Measure`, no `AggregateMeasure`. That is today's executable-class item (`AggregateInteriorPredicate` / `MeasureInteriorPredicate`) restated for views: a WITH is a projection-shaped word-row table. Measure **finds** and folds live on the **main** query, over finished WITH/rec.

Eval once, in declaration order, into a transient image, before the rec CTE if present, then the main query. A Query whose `rec` is `None` **never** enters the reach driver — WITH-only is the ordinary rule loop plus a CTE preamble. No watermark, no round budget, no `FixpointBudgetExceeded`.

Empty `with` is legal. An individual `WithDef` with zero rules is `EmptyCte` (not `EmptyRuleSet` — that name is the main query).

## RecCte — at most one linear WITH RECURSIVE

SQL-shaped forest walk. One name. Two lists, so mixed arms are a roster item rather than a classification:

- **Base arms** (`RecCte.base`): no atom — positive or negated — whose source is the rec's `CteId`. Extra EDB joins and earlier WITH are legal. **Negation is illegal in the whole rec CTE** (base and rec arms, EDB or WITH or self): `NegationInRecCte`. Measure terms (find, binding, comparison) are illegal in the whole rec CTE: heads → `MeasureInCte`; rec **bodies** → `MeasureInRecCte`. Aggregates are already unwritable in a bound-var head; a fold find on a WITH or rec head is `AggregateInCte`. There is no `AggregationInRecCte` variant — the fold is on the head, so it is `AggregateInCte`.
- **Rec arms** (`RecCte.rec`): each arm has **exactly one** positive self-atom (`Cte` = rec's id). Zero is `RecArmMissingSelf`. Two or more is `NonlinearRecArm`. The self-atom is never negated (already `NegationInRecCte`). Extra EDB and earlier WITH joins on the same arm are legal.

Both lists nonempty: `EmptyRecursiveBase` if `base` is empty (lfp would be empty; SQLite refuses the CTE; write no query). `EmptyRecursiveStep` if `rec` is empty (that is a WITH; write `with`, not `with recursive`).

The rec CTE's head is bound variables only — creation quarantine restated: `program_den_finite`'s premise, retargeted as `reach_den_finite`. The chain-window class (`w = w₁ ∩ w₂` in a rec head) stays outside, OPEN, unchanged.

The main query may join, anti-join, `Sum`, `Pack`, and measure the **finished** rec CTE. It does not grow the recursive name: `reachOp` reads only `RecCte.base` / `RecCte.rec`. Unwritable from main.

Identity projection is never implicit. The rec CTE is **never** the answer predicate (`02-lean.md: evalQuery_empty_rules` — empty main denotes `∅` even when `reachDen` is huge). `(c) | reach(c);` is a main rule, required, as today's bare output rule was required. A query of only `with` / `with recursive` and no bare rule is `EmptyRuleSet` — and in `query!`, a compile error. Today's programs whose `output` **was** the recursive predicate recut as rec CTE plus an identity main of the same arity.

## Main query

Today's query: one head, ≥1 rule, ≤ `MAX_RULES`, DNF, head alignment, folds, measures, negation, the full per-rule roster. Atoms may read EDB, any WITH, and the rec CTE if present. An out-of-range `CteId` is `UnknownCte` (today's `UnknownPredicate` screen, spent as `wellFormed_reads_real` is spent — `02-lean.md`).

## Grammar (`query!` / `ir::render`)

The notation stays the statement grammar's query side. Keywords are added so Program cannot sneak back through named heads.

```text
query   := with* rec? main
with    := 'with' pred '(' head ')' '|' body ';'
rec     := 'with' 'recursive' pred '(' head ')' '|' body ';'
main    := barerule+
barerule:= '(' head ')' '|' body ';'
pred    := lowercase ident
```

`body` / `head` / `atom` / `cond` otherwise unchanged from `20-query-ir.md` § the query notation, except: a body atom naming `pred` is a `Cte` atom (ordered-dense or indexed, same two spellings, never mixed). Relations remain UpperCamel. `and` / `or` remain reserved.

**Grouping — one law, every surface.** CTE names are unique. Multiple rules of one CTE are several builders / several notation lines of **that one name**, not a second CTE.

| Surface | How one CTE gets several rules | Second declaration of the same name |
|---|---|---|
| `query!` | Consecutive `with pred(...)` lines → one `WithDef`. Consecutive `with recursive pred(...)` lines → one `RecCte`; a line whose body has an **atom** (positive or negated, either spelling) naming `pred` is a rec arm, else a base arm. The macro classifies, then emits `base` / `rec`. | Non-consecutive reuse is a compile error (write the rules together). |
| TS | `q.with("mid", ...builders)` one `WithDef`. `q.withRecursive("reach", { base: [...], rec: [...] })` — **arrays**. One callback is not "exactly one rec arm." | `q.with("mid", ...)` a second time is a construction error. TS does not consecutive-union. |
| C++ | `bdb::with<"mid">(rule_builders...)` one `WithDef`. `bdb::with_recursive<"reach">(bdb::base{...}, bdb::rec{...})` — **two tagged packs**. | A second `with<"mid">` is a consteval error. |

TS/C++ never scan bodies to classify base vs rec: the tagged lists **are** the IR. `query!` classifies because the text grammar has one production for both arms. A `query!` rec line that mentions `pred` only in a comment is a base arm; a line whose only self occurrence is negated is a rec arm, then `NegationInRecCte` / `RecArmMissingSelf` at validate.

TS/C++ also throw if `with` / `withRecursive` is called after a main rule has been added, or `with` after `withRecursive`. Declaration order is WITH, then rec, then main — same as `query!`.

**Compile errors (macro, spanned), exhaustive for this cut:**

- A named head **without** `with` / `with recursive` — the former Program sneak. Message names the keywords.
- Two different names under `with recursive` — at most one rec CTE.
- `with` and `with recursive` sharing a name.
- `with` after `with recursive`, or either after a bare rule — declaration order is WITH, then rec, then main.
- No bare rule (no main).
- `with recursive` with zero base lines or zero rec lines (after classification).
- Duplicate `with` names that are not consecutive-union of the same pred — names are unique; non-consecutive reuse is a compile error.

All-bare `query!` (no `with`, no named heads) lowers to `Query { with: vec![], rec: None, head, rules }` — today's `ir::Query`, field for field plus two empty fields. Text-level backward compatibility for every existing non-recursive query.

Renderer: `with p{id}(...) | ...;` then `with recursive p{id}(...) | ...;` then bare main rules. Interior names stay synthesized `p{id}` — names remain a macro-local sidecar, never in the IR or the fingerprint. Round-trip goldens pin `render(lower(text))` as today.

## Mapping from today's reach programs

Cookbook 24, engine-native:

```text
reach(c) | Node(id: c), c == ?root;
reach(c) | Parent(child: c, parent: m), reach(m);
(c) | reach(c);
```

becomes:

```text
with recursive reach(c) | Node(id: c), c == ?root;
with recursive reach(c) | Parent(child: c, parent: m), reach(m);
(c) | reach(c);
```

Cookbook 25, fold over finished closure:

```text
sub(a) | Account(id: a), a == ?root;
sub(a) | AccountParent(child: a, parent: p), sub(p);
(total: Sum(minor)) | Posting(id, account: a, minor), sub(a);
```

becomes the same rewrite: `with recursive sub(...)` on the first two lines; the `Sum` line stays bare (main). Aggregation is not in the rec CTE; it reads the finished CTE. The strata-roster sentence "fold over a lower stratum" is now the ordinary sentence "main query over a finished view."

Non-recursive interior predicate (today's fanout / view-as-Idb):

```text
mid(x) | Edge(src: x);
(x) | mid(x);
```

becomes:

```text
with mid(x) | Edge(src: x);
(x) | mid(x);
```

This Query has `rec: None`. It prepares and executes as WITH-only. It must not touch the reach driver. That is the interior-Idb-as-view law, operational.

**Interior that reads the rec.** Today's legal program `rec + non-recursive interior predicate that reads rec + output` has no WITH image: WITH cannot read the rec (`CteId(with.len())` is never `< i`). Those interior rules **inline into the main query** (union of conjunctive bodies, DNF as ever). You cannot name a view of the rec CTE. Two different main-shaped queries over the same rec are two `Query` values (two prepares). Hostile IR with a WITH after rec is unrepresentable (`rec` is `Option`, `with` is a prefix list) — not a `CteNotPrior` on main.

Worked example — today's `CLOSURE_ROOTS` (non-recursive `p1` anti-joins finished `p0`):

```text
-- today: p0 recursive, p1 reads p0, output = p1
-- after: rec p0, main is p1's body (anti-join of finished rec).
-- Absent field is the wildcard (no `_` term).
with recursive reach(c, p) | OrgParent(child: c, parent: p);
with recursive reach(c, p) | OrgParent(child: c, parent: m), reach(m, p);
(id) | Org(id: id), !reach(c: id);
```

Same answers as a WITH after rec would have given: the anti-join sees the **finished** lfp. SQLite golden recuts from `WITH RECURSIVE p0 AS (...), p1 AS (... FROM p0) SELECT FROM p1` to `WITH RECURSIVE r AS (...) SELECT ... FROM Org WHERE NOT EXISTS (SELECT 1 FROM r ...)`.

**Dropped, not inlined — negation *inside* the rec CTE.** Today's recursive predicate that anti-joins a lower stratum (or EDB) during the walk is `NegationInRecCte`. Putting that anti-join on main is a **different** query (filter after closure ≠ filter during). Do not pretend they are the same rewrite. Hosts that need during-walk exclusion keep the host loop (cookbook 24's other dialect) or wait for a later cut. This cut does not grow a second rec operator.

**Output was the rec predicate.** One-predicate recursive programs (`output = 0`, the rec IS the answer) become `rec` plus an identity main of the same arity. The rec table is not implicitly returned. `program-hand-closure.json` is this shape.

A today's degenerate `Program` (one predicate, no Idb) is a today's `Query`. After the cut it is `Query { with: [], rec: None, ... }`. There is no embedding type.

Mutual recursion, nonlinear rec, named-head-without-`with`, `Program` literals in tests — unwritable or refused. `04-bindings-docs.md` lists the test conversions.

## Validation roster (additions; the per-rule roster stays)

Judged in this order, after the existing query-shape checks (empty main, `MAX_RULES` / DNF per WITH and per main and the rec **pool**, nesting, DNF, head-shape alignment):

1. `TooManyCtes { count }` — `with.len() + rec.is_some() as usize > MAX_CTES` (usize, before any `CteId`).
2. Per `WithDef` / `RecCte` / main: existing empty-rule (`EmptyCte` for a WITH with zero rules; `EmptyRuleSet` only for empty **main**), DNF, head-alignment, per-rule roster, with `IdbSignatures` replaced by `CteSignatures` (sealed in declaration order — no chaotic iteration).
3. `CteNotPrior { cte, at }` — WITH `i` reads `j ≥ i`, or any WITH reads the rec id. `at` is the reading CTE's id; `cte` is the illegal target.
4. `UnknownCte { atom, cte }` / `CteColumnOutOfRange { atom, field }` — the well-formedness screen (`Query.WellFormed`, `02-lean.md`). Replaces `UnknownPredicate` / `PredicateColumnOutOfRange`.
5. Rec roster: `EmptyRecursiveBase`, `EmptyRecursiveStep`, `SelfInBase`, `RecArmMissingSelf`, `NonlinearRecArm`, `NegationInRecCte`, `MeasureInRecCte` (every measure site in a rec **body**: find is a head — that's item 6; binding and comparison are bodies).
6. `AggregateInCte` / `MeasureInCte` — fold or measure **find** on a WITH or rec **head** (bound-var law). One error per shape: heads of WITH or rec → `*InCte`; rec bodies → `MeasureInRecCte`. Main heads keep today's aggregate/measure roster. WITH **bodies** may contain measure comparisons (not this item).
7. Query-global param unification (today's program-global pass, now the only pass).

**Canonical error names (this cut). Use these spellings everywhere — docs, tests, Bridge instruments.**

| New | Replaces / notes |
|---|---|
| `TooManyCtes` | `TooManyPredicates` |
| `UnknownCte` | `UnknownPredicate` |
| `CteColumnOutOfRange` | `PredicateColumnOutOfRange` |
| `CteNotPrior` | forward/self WITH read; WITH reading rec |
| `EmptyCte` | zero-rule `WithDef` |
| `EmptyRecursiveBase` / `EmptyRecursiveStep` | new |
| `SelfInBase` / `RecArmMissingSelf` / `NonlinearRecArm` | new (k-variants die) |
| `NegationInRecCte` | `NegationThroughCycle` for the rec CTE; main anti-join of finished CTE is legal |
| `AggregateInCte` | `AggregateInteriorPredicate` + `AggregationThroughCycle` when the fold is on a WITH/rec **head** |
| `MeasureInCte` | `MeasureInteriorPredicate` + `MeasureInRecursiveHead` when the measure **find** is on a WITH/rec **head** |
| `MeasureInRecCte` | measure in a rec **body** (comparison / binding). Bindings may still hit `DurationInBinding` first (per-rule roster runs earlier) — both are correct; do not add a third name |
| `EmptyRuleSet` | empty **main** only |

**Deleted roster items (no replacement that preserves the shape):** `TooManyPredicates`, `UnknownOutputPredicate`, `UnknownPredicate`, `PredicateColumnOutOfRange`, `NegationThroughCycle`, `AggregationThroughCycle`, `UnresolvedPredicateSignature`, `AggregateInteriorPredicate`, `MeasureInteriorPredicate`, `MeasureInRecursiveHead`. There is **no** `AggregationInRecCte`.

Signature sealing: WITH `i` seals from its rules against already-sealed `0..i`. Rec seals from **base** (EDB + sealed WITH — a stored column always names the type) then rec arms (self already sealed). `p(x) | p(x)` as a rec with empty base is `EmptyRecursiveBase`, not a sealing timeout. The signature fixpoint loop in `validate_program` dies.

A `Cte` atom on a Query with empty `with` and no rec is `UnknownCte` — today's "Idb at the query boundary" refusal, without a Program to route through.

## What this language refuses to be

- A stratified Datalog program. One rec CTE, linear, no not-in-rec, no mutual, no SCC.
- Bags / `UNION ALL`.
- Implicit output = last CTE. Bare rules are the output.
- Fuel as meaning. The rec CTE denotes an lfp; the budget is a resource abort (`03-engine.md`).
- A second eval path (host-loop internalization, ParamSet-per-round). The host-loop idiom in cookbook 24 remains a **host** idiom, not an engine mode.
- During-walk negation in the rec CTE. Main may anti-join the finished rec.
