# Docs audit — leftover coordinate system

Doctrine: `audit/00-representation-is-the-essence.md` (SPOVs: representation first; illegal states unrepresentable; special cases are coordinate artifacts).

Present-tense language: a `Query` is interiors + at most one linear `Rec` + main. `Program` is deleted. Fuel is not denotation (`reachDen = lfpS`; the budget is a resource abort). SQLite `WITH RECURSIVE` is a translator spelling, not the language. CTE / Idb / stratum / Tarjan condensation are gone coordinates.

Scope: `docs/architecture/`, `docs/cookbook.md`, `docs/architecture/00-product.md`, `docs/feature-register.md` (living product ledger), `lean/README.md`, `lean/conformance/README.md`.

Skipped as archival: `docs/research/*` (dated investigation reports, including ones that say “IR stands today” as of 2026-07-19), `docs/free-join-paper/`, `audit/` itself. Brooks/Pike English “program” in `docs/design/representation-first.md` is not the IR type. Deleted-vocabulary *obituaries* that map `rule program / stored rules` → host loop (`00-product.md`, `20-query-ir.md` non-goal) are not findings — they are the replacement sentence.

Not findings: `lean/README.md` “Fuel is not a Lean semantic parameter” / “not vs a fueled Lean evaluator (there isn’t one)” — that is the new wall, stated correctly. `60-validation.md` framing `WITH [RECURSIVE]` as a lossy SQLite translator (“not a grammar for the language and not a field in the IR”) is the required sentence. Least-fixpoint of *one* `Rec` (`reachDen = lfpS`) is the new meaning, not the old stratified-program meaning. Cookbook “two dialects” (host loop vs one linear rec) is the new product teaching.

`proposals/` citations: **none** in scope.

Severity: **high** = present-tense teaching of a deleted type or dual language as if it were the coordinate. **medium** = leftover names by negation, SCC/Tarjan as rec’s kind, empty-rec as a special embedding of “today’s query,” SPOV-2 runtime checks. **low** = SQL-idiom metaphor or name collision that will confuse a reader hunting the deleted type.

---

## Findings

### F1 — high — `docs/architecture/20-query-ir.md`

> **Hand-written multi-rule programs keep the head-projection law**

**Wrong.** `Program` is deleted. This is dual vocab: query vs program. A hand-written rule-list is still a `Query` (main, or one `Interior`, or the rec pool).

**Speak.** “Hand-written multi-rule **queries** keep the head-projection law” (or “hand-written main rule-lists”).

---

### F2 — high — `docs/architecture/20-query-ir.md`

> **Main defines one anonymous predicate; rules derive it.** … sealed in the witness (`ir/validate`'s `Predicate`) … **The predicate is anonymous and engine-internal**

**Wrong.** Predicate-as-query-head is the old program coordinate (each predicate in a rule program). Main is the query’s answer shape: `head` + `rules`. The sealed object is the main signature, not a Datalog predicate.

**Speak.** Main owns the answer signature (arity, types, folds), sealed once at validation. Interiors and the rec are derived tables addressed by `InteriorId`. Do not call main a predicate.

---

### F3 — high — `docs/architecture/20-query-ir.md`

> `rec` is at most one linear SCC — `Rec { head, base, rec }`

and the IR shape:

> `rec: Option<Rec>,  // at most one linear SCC`

and later:

> the rec SCC as **one** pool (`base.len() + rec.len() ≤ MAX_RULES`)
>
> the rec roster refuses negation in the rec SCC (`NegationInRec`)

**Wrong.** An SCC is a Tarjan/stratum artifact. The representation is `Option<Rec>`: at most one linear rec. `Query.recLinear` is the judge. Calling rec an SCC keeps the condensation coordinate alive in the normative IR chapter.

**Speak.** `rec: Option<Rec>` — at most one linear rec. The rec pool is `base.len() + rec.len()`. `NegationInRec` refuses negation in that rec.

---

### F4 — medium — `docs/architecture/20-query-ir.md`

> a query with empty `interiors` and no rec is today's query plus two empty fields (`lean/Bumbledb/Exec/Reach.lean: evalQuery_plain`)

and:

> Main is today's query: one head, ≥1 rule, folds, measures, negation. … A query with empty `interiors` and no rec is today's query plus two empty fields … — not an embedding into another type.

**Wrong.** SPOV 3: the empty-interiors / `rec: None` case is treated as a special embedding of an older query type (“today’s query”) plus two fields. There is one `Query`. Empty interiors and no rec is not a second kind; `evalQuery_plain` is that case of `evalQuery`.

**Speak.** A `Query` with empty `interiors` and `rec: None` is still a `Query`. `evalQuery_plain` is that case, not an embedding of a prior type.

---

### F5 — medium — `docs/architecture/20-query-ir.md`

> There is no `MAX_CTES` / `MAX_INTERIORS` / `TooManyCtes`.

and:

> `InteriorIdOverflow` (derived-table count does not fit `u32` — id-width, not a product 16; there is no `TooManyCtes`)

**Wrong.** Architecture rule 6: no history, no retired names. CTE is a translator spelling. Teaching the deleted cap names by negation keeps the old coordinate in the roster.

**Speak.** Derived-table count is `u32` width (`InteriorIdOverflow`). There is no interior-count product cap. Do not name CTE errors.

---

### F6 — medium — `docs/architecture/20-query-ir.md`

> The rec roster (`lean/Bumbledb/Query/Syntax.lean: Query.recLinear`) is the judge, not a Tarjan condensation

**Wrong.** Same negation-of-retired-coordinate. The present-tense sentence is `recLinear` judges the one rec.

**Speak.** `Query.recLinear` is the well-formedness of the one linear rec (exactly one positive self-atom per rec arm, …). Drop Tarjan.

---

### F7 — medium — `docs/architecture/20-query-ir.md`

> fuel-is-not-denotation (`reachDen` is `lfpS`; the budget is a resource abort)

**Wrong.** The proposition is correct; the wall is still named after the deleted semantic parameter. Fuel-as-semantics should be dead, including as a hyphenated ghost.

**Speak.** Denotation is `reachDen = lfpS`. The derived-tuples / rounds budget is a resource abort (`DerivedBudgetExceeded`), incompleteness versus `evalQuery`, not a semantic parameter.

---

### F8 — medium — `docs/architecture/20-query-ir.md`

> There is no separate program renderer.

**Wrong.** `Program` is deleted. A present-tense renderer section should not mention a program renderer even to deny it.

**Speak.** `ir::render` prints a `Query`: interiors, optional rec, then bare main rules.

---

### F9 — medium — `docs/architecture/20-query-ir.md`

> A named head without either keyword is a compile error (the former named-head sneak).

**Wrong.** “Former sneak” is history. Present tense: a named head is `interior` or `recursive`; bare rules are main.

**Speak.** A named head requires `interior` or `recursive`. Bare rules are the main query.

---

### F10 — medium — `docs/architecture/20-query-ir.md`

Prepared-query section:

> the prepared query holds one validated plan per rule and **one** sink configuration, owned by the head — execution is the rule loop driving every rule's plan into that sink

**Wrong.** Earlier the same chapter says one sink per rule-list (one per `Interior`, one for the `Rec`, one for main). This section is still written in the old one-list coordinate (CQuery / program output). Interiors and rec are not “the head’s sink.”

**Speak.** Each rule-list has its own sink: interiors in declaration order, then the rec, then main. The prepared object holds one plan per rule of each list and one sink per list. Main’s sink is the answer.

---

### F11 — high — `docs/architecture/40-execution.md`

The execution chapter still calls a query a program:

> multi-rule program whose heads are provably pairwise disjoint
>
> spanning a multi-rule program, keyed by provenance
>
> each rule of a program executes its own plan
>
> `union_regime_head_projection` for hand-written programs
>
> A **hand-written multi-rule program** keys the **head projection**
>
> for a hand-written program the **head projection**
>
> the single-rule key-probe program
>
> a program shrunk to one rule sheds its union machinery like any single-rule program

**Wrong.** Dual vocab. `Program` is deleted. These are queries / rule-lists / main.

**Speak.** Multi-rule **query** (or main rule-list). Hand-written vs DNF-derived rule sets of one `Query`. Single-rule key-probe **query**.

---

### F12 — high — `docs/architecture/60-validation.md`

> the CQuery arm (`seeded-*.json`) is unchanged.

**Wrong.** Architecture is `Query`. `CQuery` is the old / parallel type. Seeded cases are queries with empty interiors and no rec, or they should be serialized as `Query`. Teaching a “CQuery arm” vs a “reach arm” is two languages.

**Speak.** Seeded cases are `Query` values (`interiors = []`, `rec = none`) in `seeded-*.json`. Reach cases are `Query` values with interiors / rec in `reach-*.json`. One type.

---

### F13 — high — `docs/architecture/60-validation.md`

> a program whose every disjunct vanishes is the empty union

**Wrong.** `Program` is deleted. This is the empty-main-rule-set case of a `Query`.

**Speak.** A query whose every main disjunct vanishes is the empty union (`EmptyRuleSet`).

---

### F14 — low — `docs/architecture/60-validation.md`

> it emits SQL `WITH [RECURSIVE]` then the whole cte-list because that is what SQLite speaks

**Wrong.** The paragraph correctly says this is not the language. Residual: “cte-list” as the shape being emitted still invites reading the IR as a CTE list. SQLite’s spelling can be named as SQL without importing CTE as a bumbledb noun.

**Speak.** The translator emits SQLite `WITH RECURSIVE` (SQLite’s spelling of interiors + rec + main). That SQL is not a field in the IR.

---

### F15 — medium — `docs/architecture/70-api.md`

> A query with empty interiors and no rec prepares as today's query plus two empty fields

**Wrong.** Same special-case embedding as F4, now on the embedding API.

**Speak.** `Db::prepare(&Query)` — empty interiors and `rec: None` is an ordinary `Query` (`evalQuery_plain`).

---

### F16 — medium — `docs/architecture/70-api.md`

> everything SQL spells with data-modifying CTEs — must read on a snapshot first

and:

> the data-modifying-CTE shapes with the premises witnessed instead of locked.

**Wrong.** CTE is not a bumbledb word. These are host write idioms (insert-select, update-where) over prepared queries. Importing CTE teaches the SQL coordinate as if it were ours.

**Speak.** Insert-select / update-where: query on a snapshot, then `write_from`. SQL’s data-modifying `WITH` is a translator/host analogy at most, and should not be the name of the idiom.

---

### F17 — high — `docs/architecture/70-api.md`

> column metadata via `PreparedQuery::predicate()` — the predicate the query defines (`20-query-ir.md` § the query shape) is the **buffer-typing authority**

**Wrong.** Dual vocab: the query’s sealed main signature is still spelled “predicate” on the embedding API and in the architecture sentence that teaches it.

**Speak.** The sealed main signature (answer columns + folds). If the Rust method is still `predicate()`, the architecture doc should call that a leftover name and speak “main signature” — or the method should be renamed in the same cut this doc is describing as current.

---

### F18 — medium — `docs/architecture/70-api.md`

> same-schema/different-environment confusion stays a runtime check (`ForeignPreparedQuery`).

**Wrong.** SPOV 2: the doc admits a guard instead of a representation. Cross-schema is already unrepresentable (`Db<S>`). Cross-environment is the same class of confusion left as a check.

**Speak.** Brand `PreparedQuery` with the preparing environment (or a generation/instance witness in the type) so a foreign snapshot is unrepresentable at the call, not detected at execute. If that cost is refused, say so as essential complexity with a horizon representation — do not leave “stays a runtime check” as the last word.

---

### F19 — medium — `docs/architecture/75-cpp-lowering.md`

> Empty `interiors` and `rec: None` is today's query plus two empty fields. Caps: `MAX_RULES = 16` per rule-list (rec pooled); **no `MAX_CTES` / `MAX_PREDICATES`**

**Wrong.** “Today’s query” special case + deleted cap names (`MAX_CTES`, `MAX_PREDICATES`) in a present-tense lowering contract.

**Speak.** `Query { interiors, rec: Option<Rec>, head, rules }`. `MAX_RULES = 16` per list (rec pooled). No interior-count cap.

---

### F20 — high — `docs/architecture/75-cpp-lowering.md`

> There is no output-last predicate slot and no `output = recs.length`.

and the checklist:

> interiors then rec then main — no output-last predicate slot

**Wrong.** That is the deleted `Program` layout (predicates, recs list, output index). A lowering contract should describe `Query` field-for-field, not deny the old slots.

**Speak.** Wire shape is `QueryIr { interiors, rec, head, rules }`. Evaluation order is interiors, optional rec, main. Main is `head` + `rules`, not an output index into a predicate table.

---

### F21 — medium — `docs/architecture/README.md`

> **Mutual-linear** (one SCC, several names, each rule ≤1 rec atom). … even/odd encodes as one linear predicate with a parity column. Refused this cut so Tarjan / k-variants / multi-pred scratch stay gone. … not a resurrection of a predicate table.
>
> still not a second SCC.
>
> Refused this cut so `NegationInRec` covers the whole SCC

**Wrong.** OPEN items are present-tense product. They still locate the design in SCC / Tarjan / predicate-table coordinates. The refused future is `List Rec` or several names, not “a second SCC.”

**Speak.** Mutual-linear: several names, each rule ≤1 rec atom — a new IR, not `Option<Rec>`. Named interior of a finished rec is not a second rec. `NegationInRec` covers the one rec.

---

### F22 — medium — `docs/cookbook.md`

> **Insert-select**: query source answers, insert the derived facts — the data-modifying CTE with its premises witnessed instead of locked.

**Wrong.** Same CTE import as F16, in the cookbook’s write-idiom teaching.

**Speak.** Insert-select: query source answers, insert the derived facts, `write_from` witnessing the snapshot.

---

### F23 — low — `docs/cookbook.md`

Recipe 30:

> The schema says `Program(grp) -> Program` — one program per group
>
> `relation Program { … }`
>
> `Program(grp) -> Program;    // one program per group — the callable law`

**Wrong.** After `Program` the IR type is deleted, a worked schema named `Program` collides with the deleted coordinate. Readers will not know which Program is meant.

**Speak.** Rename the example relation (`Course`, `Track`, `Offering`, …). The law is “one row per group,” not “one program.”

---

### F24 — high — `docs/feature-register.md`

> refused today by name (`AggregateInteriorPredicate`).

**Wrong.** Living product ledger. The current refusal is `AggregateInInterior` (interior/rec heads project bound variables). `AggregateInteriorPredicate` is the old predicate-table error.

**Speak.** `AggregateInInterior` / `MeasureInInterior` — folds and measure finds are legal only at the main head.

---

### F25 — high — `docs/feature-register.md`

> zero stratification impact, no new Lean axioms

**Wrong.** Stratum is deleted. Weak-form HAVING has no rec-roster impact, not “zero stratification impact.”

**Speak.** No change to `recLinear` / `NegationInRec` / the one linear rec. No new Lean axioms.

---

### F26 — high — `docs/feature-register.md`

> The idb re-grounding tax (an idb atom is a join position) — engine law, documented, ~6 recursive queries carry one extra `.match`.

**Wrong.** `Idb` is deleted. Derived-table atoms are `AtomSource::Interior`. Teaching “idb atom” as current engine law is the old program coordinate.

**Speak.** An `Interior` atom is a join position (re-grounding tax, if that law still holds). Derived tables are `InteriorId`, never `Idb`.

---

### F27 — high — `lean/conformance/README.md`

> A reach case carries a Query with `interiors` / `rec` / main `rules` (`Bumbledb/Query/Syntax.lean`) instead of a CQuery.

The interchange still has two query types: `CQuery` for `seeded-*.json` / query cases, `Query` for `reach-*.json`.

**Wrong.** Dual representation in the third oracle’s corpus. SPOV 1: complexity lives in the two types. Architecture is one `Query`. “Instead of a CQuery” teaches the split as current.

**Speak.** Every case carries a `Query`. Plain cases have empty `interiors` and `rec: null`. Reach cases fill those fields. Decode one type. (`plainQuery : CQuery → Query` is the leftover embedding to delete.)

---

### F28 — medium — `lean/conformance/README.md`

> Atoms on this arm are `edb` / `interior` (never `idb`, never a stored `relation` key).

**Wrong.** Correct atoms, leftover `idb` by negation. The JSON key is `edb` | `interior`.

**Speak.** Atoms are `edb` / `interior`. `FieldId` on an interior atom addresses a derived head position.

---

## Counts by severity

| Severity | Count | Findings |
|---|---|---|
| high | 12 | F1 F2 F3 F11 F12 F13 F17 F20 F24 F25 F26 F27 |
| medium | 14 | F4 F5 F6 F7 F8 F9 F10 F15 F16 F18 F19 F21 F22 F28 |
| low | 2 | F14 F23 |
| **total** | **28** | F1–F28 |

---

## Clean (in scope, no finding)

- `docs/architecture/00-product.md` — present-tense interiors + one linear rec; rule-program listed only as deleted vocabulary with a replacement.
- `docs/architecture/10-data-model.md`, `30-dependencies.md`, `50-storage.md`, `61-bench-lanes.md` — no Program/CTE/Idb/stratum/CQuery/fuel-as-semantics teaching.
- `lean/README.md` — fuel is not a semantic parameter; linear-reach model is interiors + one rec. History’s “stratification lemma” is labeled History.
- `docs/architecture/40-execution.md` § linear reach driver — interiors, then one linear reach, then main; budget as resource abort vs `evalQuery`. (The surrounding chapter still says “program”; that is F11.)
- `proposals/` — no citations in scope.
