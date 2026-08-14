# Representation audit — `lean/` after the Program → Query + linear reach cut

Brooks/Pike (SPOV 1–3, Insight 16). Scope: `lean/` only. No fixes.
Hunt: leftover Program-shaped types, dual denotations, flags that
reconstruct illegal states, validation that throws away proofs, guards
a tighter inductive type would delete.

Not filed (this-cut scope / essential, not sloppy representation):
mutual rec; Level 0 `evalQuery` vs Level 1 `evalQueryList` (the tree's
refinement pattern, proved equal); `Safe` / `WellTyped`; product
encoding of `Query`/`Rec` forced by Lean's `T.rec` recursor name
(language); private `fueledLoop` as a termination metric; Plan staying
over `Rule` (recorded); CQuery aggregate finds (PRD 04/05 narrowing);
acceptance roster items that are benign vs denotation (`EmptyRuleSet`,
caps).

---

## HIGH

### H1. `Query` is still a Program product with an `Option Rec` hole

- **Where:** `Query/Syntax.lean:289–305` (`Query`, `Query.mk`,
  `Query.plain`, `Query.Plain`); consumers `Exec/Reach.lean:748–757`
  (`evalQuery`), `770–779` (`evalQueryList`), `994–1048`
  (`evalQuery_sound` cases `q.rec`).
- **What's wrong:** The cut's IR is supposed to be
  `{ interiors, rec, head/arity, rules }`. The type is still
  `List Interior × Option Rec × Nat × List Rule` — a Program (rule
  list + arity) with two extra fields, one of them `Option`. Illegal
  states still representable: rec without interiors numbering,
  interiors with a junk rec, `Plain` as a *predicate* reconstructing
  `interiors = [] ∧ rec = none`, `recLinear`'s `none => True` vacuity
  (`Syntax.lean:483–484`). Every evaluator independently `match`es
  `q.rec` and rebuilds `self := ⟨q.interiors.length⟩`. That is the
  three-boolean problem: optional rec × empty interiors × unused
  arity.
- **What representation collapses it:** A sum
  `inductive Query | plain (rules) | interiors (defs : Interiors) (rules) | rec (defs) (r : LinearRec) (rules)`.
  `Query.Plain`, `Query.plain`, `recLinear`'s none-arm, and both
  `match q.rec` sites in `evalQuery` / `evalQueryList` vanish.
  `evalQuery` becomes one function by cases, not a product plus
  flags.
- **Essential vs accidental:** Accidental. The language has three
  shapes (plain / interiors-only / interiors+linear rec). Those are
  essential *constructors*, currently encoded as a product plus
  `Option` plus a Prop.
- **Severity:** high

### H2. `Rec` is still a stratified-SCC record: arity + two untyped rule lists

- **Where:** `Query/Syntax.lean:271–279` (`Rec := Nat × List Rule × List Rule`);
  `recLinear` `Syntax.lean:482–490`; `Rule.selfCount` `459–460`;
  `Rule.hasNegatedSelf` `463–464`; `reachOp_mono`
  `Exec/Reach.lean:228–230`; `recLinear_arms` `965–972`;
  `Countermodels.lean:1406–1417` (`oddRec` is writable).
- **What's wrong:** A linear rec is: nonempty base (no self), nonempty
  rec arms (exactly one self atom, no negation). The type is two
  `List Rule`s and a `Nat`. Representable: empty base, empty rec,
  `selfCount = 0` rec arm (`p ← ¬p` — `oddRec`), `selfCount ≥ 2`
  (nonlinear), negation in rec, base that names self. `recLinear` is
  shotgun validation over that product. `hasNegatedSelf` is *strictly
  implied* by `r.negated = []` on the next line (`488–490`) — leftover
  from "no self-negation" (stratified) before `NegationInRec` banned
  all negation. `recLinear_arms` then *re-parses* the Prop and keeps
  only `selfCount = 1 ∧ negated = []`, throwing away nonempty-base,
  base-not-self, and `hasNegatedSelf`. Parse-don't-validate inverted:
  validate, discard, re-check at every lemma.
- **What representation collapses it:**
  ```
  structure RecBase where finds : List VarId; atoms : List NonSelfAtom; conditions : List Condition
  structure RecStep where finds : List VarId; self : Atom; atoms : List NonSelfAtom; conditions : List Condition
  -- RecStep.self.source is definitionally the rec; negated unrepresentable
  structure LinearRec where base : NonEmpty RecBase; rec : NonEmpty RecStep
  ```
  `selfCount`, `hasNegatedSelf`, `recLinear`, `odd_not_stratified` as a
  *syntax* inhabitant, and `reachOp_mono`'s `hlin` premise all die.
  `odd_not_monotone` stays as an operator-level wall (essential).
- **Essential vs accidental:** Accidental leftover of Program/SCC
  (one rule type, count recursive occurrences, stratify). Linearity
  and no-negation-in-rec are essential *language* facts; storing them
  as `Nat` counts over a shared `Rule` is accidental.
- **Severity:** high

### H3. `InteriorId` is a free `Nat`; rec identity is a numbering coincidence

- **Where:** `InteriorId` `Syntax.lean:144–146`; `AtomSource`
  `179–182`; `Query.recId` `318–321` (**dead** — defined, then every
  comment says do not match it: `Syntax.lean:477–478`,
  `Reach.lean:747`); `derivedCount` `324–325`; `sourcesInRange`
  `467–469`; `interiorsDag` `473–475`; `reachOp` / `reachDen` /
  `evalLinearReach` take `rec : Rec` and `self : InteriorId` as
  *independent* arguments (`Reach.lean:196–205`, `489–493`);
  `evalQuery` rebuilds `self := ⟨q.interiors.length⟩` (`755–756`,
  `777–778`, `1010`).
- **What's wrong:** Derived-table identity is a global `Nat`. An atom
  can name `⟨999⟩`. Rec is not a constructor; it is "whatever id
  equals `interiors.length` today." `recId : Option InteriorId` is a
  second coordinate for the same fact, unused because matching it
  beside `q.rec` leaves a catch-all the elaborator cannot see
  unreachable — they added a dual path and then posted guards against
  using it. `reachOp C rec ⟨999⟩` is well-typed and denotes a
  different operator than `reachOp C rec ⟨q.interiors.length⟩`.
  `sourcesInRange` / `interiorsDag` are the shotgun that tries to glue
  the numbering back together. `wellFormed_interior_reads_real`
  (`Reach.lean:185–190`) is `hwf.1` — a WellFormed proof that does
  not refine `InteriorId`.
- **What representation collapses it:** `AtomSource` as
  `| edb RelId | interior (Fin n) | recSelf`, with interiors a
  telescope so interior `i` can only name `Fin i`, and `recSelf`
  legal only in `RecStep`. Rec identity is the constructor, not
  `length`. `recId`, `derivedCount`, `sourcesInRange`,
  `interiorsDag`'s rec clause, the "do not match recId" comments, and
  the floating `self` parameter all vanish. `reachOp` takes
  `LinearRec` only.
- **Essential vs accidental:** Accidental. Separate EDB vs derived
  identity is essential (`InteriorId` must not pun `RelId`). Free
  `Nat` plus a length convention is the leftover Program/IDB
  numbering.
- **Severity:** high

### H4. `WellFormed` is bundled validation nobody spends; `interiorsDag` is dead

- **Where:** `Query.WellFormed` `Syntax.lean:492–493`;
  `plain_wellFormed` `498–510`; `evalQuery_sound`
  `Reach.lean:974–981` ("Premises: `Safe` / `WellTyped` / `recLinear`,
  **not full `WellFormed`**"). `interiorsDag` is *never* a hypothesis
  of a denotation or agreement theorem (grep: definition + WellFormed
  conjunct only).
- **What's wrong:** Three independent Props glued with `∧`. Callers
  pick `recLinear` and ignore the rest. `interiorsDag` is the
  topological-order invariant of interiors — the cut's DAG — and it
  is thrown away. `evalInteriorsAt` still "works" on a cyclic list:
  later interiors simply do not exist yet, so a back-edge reads
  empty. That is a semantic landmine a telescope would refuse to
  write. This is King's "validate, discard the proof" exactly.
- **What representation collapses it:** Interiors as a snoc-telescope
  / `Vector` indexed by declaration order; each body's `AtomSource`
  can only name strictly earlier `Fin i`. `interiorsDag` and
  `WellFormed` as a bundle disappear. Remaining acceptance
  (`Safe`/`WellTyped`) stays as named premises of `eval_sound` —
  those are essential, and already spent.
- **Essential vs accidental:** Accidental packaging. A DAG of
  interiors is essential; a `List` plus an unspent Prop is leftover
  Program-strata thinking (order the SCCs, then check).
- **Severity:** high

### H5. Dual denotation: `evalQuery` vs `rulesAnswers ∘ edbEnv`

- **Where:** THE denotation is `evalQuery` (`Syntax.lean:282–283`,
  `Reach.lean:6–7`, `745–757`). The rest of the tree still denotes a
  Query as `rulesAnswers C q.rules (edbEnv I)`:
  - `snapshot_single` / `Query.relations` `Denotation.lean:1008–1026`
    — Theorem 9 claims "the denotation reads ONE instance" and then
    only walks **main** rules under `edbEnv`. Interior/rec EDB reads
    are invisible. After the cut this is not the denotation.
  - `evalQuery_plain` `Reach.lean:1050–1056` — shim recovering the
    old Program equation.
  - `seenfold_is_set_semantics` `Exec/Dedup.lean:329–331`;
    `union_regime_head_projection` `611–615`;
    `disjoint_witness_licence` uses `q.rules` + `edbEnv`.
  - `RewriteStep` `Exec/Rewrites.lean:2299–2359` — every constructor
    wraps `Query.plain n (pre ++ …)`; comment still says "rewrite
    step on a **program**." `step_preserves` `2456–2459` proves
    equality of `rulesAnswers q.rules (edbEnv I)`, not `evalQuery`.
- **What's wrong:** Two denotations. `evalQuery` is the cut.
  `rulesAnswers ∘ edbEnv` is Program. `Query.Plain` exists to paper
  over the gap. A Query with interiors/rec can inhabit Dedup/Rewrites
  theorems that silently ignore derived tables — the type does not
  stop you. Dual path, leftover Program coordinate.
- **What representation collapses it:** Theorems that are about a
  rule list take `List Rule`, not `Query`. Theorems about a query
  take `evalQuery`. `RewriteStep : List Rule → List Rule`. Delete
  `Query.Plain` / `evalQuery_plain` as a semantic API (plain is a
  constructor, see H1). Theorem 9 restated over `evalQuery` and
  `q.allRules`'s EDB sources (or, with H3, the typed EDB set).
- **Essential vs accidental:** Accidental leftover of the cut.
  Per-rule facts (`ruleAnswers`, `derives`) are essential and should
  stay over `AtomSource → Set Fact`. Using `Query` as a Program
  wrapper is the accident.
- **Severity:** high

### H6. Orphan `arity` fields; candidate space refuses to believe them

- **Where:** `Interior.arity` `Syntax.lean:267–269` (**never read** by
  any denotation); `Rec.arity` `276`; `Query.arity` `293`; decoder
  still fills them (`Main.lean:379–408`). Recorded dual path
  `Exec/Reach.lean:20–24`: spec names `allTuples recDom rec.arity`;
  the proved evaluator uses `r.finds.length` (`recCands` `372–375`)
  "so agreement does not assume head-arity equals `Rec.arity`."
  `fillerValue` / `tupleFact` `Denotation.lean:669–677` — out-of-arity
  fields are a total dummy.
- **What's wrong:** Head shape lives twice: a floating `Nat` and
  `finds.length` per rule. They can disagree. The evaluator *worked
  around* the disagreement instead of deleting the `Nat`. `tupleFact`
  then needs a filler because `AnswerTuple` is an untyped `List Value`
  and `FieldId` is a free `Nat`. Illegal state: arity 3, finds of
  length 1, probe of field 7 → `false`.
- **What representation collapses it:** Drop every `arity : Nat`.
  Head width *is* `finds.length` (and interiors/rec require uniform
  width in the type: `LinearRec` carries one `Vector Value n`, or
  `n` is an index of the structure). `FieldId` at a derived table is
  `Fin n`. `fillerValue`, `tupleFact`'s `getD`, and the recCands
  narrowing disappear.
- **Essential vs accidental:** Accidental. Projection-shaped heads
  are essential (creation quarantine). A parallel `Nat` that the
  proofs already refuse to trust is leftover Program/head-arity.
- **Severity:** high

---

## MED

### M1. `evalInteriorsAt` is a fueled write with an illegal `none` arm

- **Where:** `Exec/Reach.lean:128–148`, especially `141–144`
  `match defs[n]? | some d => … | none => False`;
  `evalInteriorTables_step` `840–888` is a trichotomy on `c.id` vs
  `i` reconstructing "already written / writing now / not yet."
- **What's wrong:** Stage is a free `Nat`. Overshoot (`n > length`)
  and holes (`defs[n]? = none` mid-eval) are representable; the doc
  says "call with `defs.length`." Control flow is guarding a
  length-indexed fold that should have been the type. Same family as
  H4's unspent DAG.
- **What representation collapses it:** `evalInteriors` as a fold over
  a telescope / `Vector Interior n`, writing `Fin n` slots in order.
  The `none => False` arm and the trichotomy lemma go away.
- **Essential vs accidental:** Accidental. Declaration-order eval is
  essential; a `Nat` stage parameter is leftover fuel thinking
  (the cut claimed "No fuel. No strata").
- **Severity:** med

### M2. Dual atom JSON / dual `evalQuery` at the driver

- **Where:** `Conformance.lean:383–390` `decodeAtom` always
  `source := .edb ⟨relation⟩` (Program atom: a stored `relation`
  key); `Main.lean:352–364` `decodeReachAtom` is a second grammar
  (`edb` | `interior`). `Conformance.evalQuery` `946–973` vs
  `Query.evalQuery` / `evalQueryList`. `plainQuery`
  `Conformance.lean:927–930` rebuilds `Query.plain`.
- **What's wrong:** Two query IRs, two atom encodings, two functions
  named `evalQuery`. The cut's `AtomSource` never reached the
  non-reach conformance lane. Leftover Program coordinate at the
  boundary.
- **What representation collapses it:** One decoder into `Atom` /
  `Query`. The query lane is `evalQueryList` (or the aggregate glue
  over that environment). `relation` vs `edb` is a JSON spelling,
  not a second type.
- **Essential vs accidental:** Accidental at the IR/driver layer.
  Aggregate head shapes vs projection finds are essential (PRD 05)
  and may still need `CFind` — that is not an excuse for a second
  atom source encoding.
- **Severity:** med

### M3. `Query.allRules` flattens a Program, then theorems re-split it

- **Where:** `Syntax.lean:309–314`; `mem_allRules_interior` /
  `mem_allRules_rec` / `mem_allRules_main` `Reach.lean:946–963`;
  `evalQuery_sound` immediately unpacks `allRules` back into
  interiors / rec / main (`984–1016`).
- **What's wrong:** Quantification surface is a concatenated Program
  rule list. Every theorem that needs the cut's structure has to
  invert the concat. Leftover "range over the program's rules."
- **What representation collapses it:** H1's sum type. Premises are
  `∀ r, r ∈ q.rules → Safe r` on each constructor's lists. `allRules`
  and the three `mem_allRules_*` lemmas disappear.
- **Essential vs accidental:** Accidental.
- **Severity:** med

### M4. `naiveIter` / `semiNaiveIter` sit beside `lfpS` and are not on the `evalQuery` path

- **Where:** `Reach.lean:266–318`; Bridge rows on
  `semi_naive_agrees` (`Bridge.lean:588–596`). `evalLinearReach`
  uses `fueledLoop` ∘ `reachStep`, proved equal to `lfpS` — it never
  calls `naiveIter` or `semiNaiveIter`.
- **What's wrong:** Stratified-Program evaluation theory (naive vs
  semi-naive rounds) kept as a second/third iterator after the
  denotation became `reachDen = lfpS`. Dual evaluation coordinate,
  disconnected from the query denotation. (Engine delta agreement is
  a real claim — it does not need a second iterator *family* living
  in the spec's Reach module as if it were the meaning.)
- **What representation collapses it:** Keep `lfpS` as the meaning.
  If semi-naive agreement is ledgered, instantiate it at `reachOp`
  as a corollary, or move it to Countermodels/Bridge prose. Do not
  keep a parallel `semiNaiveIter` denotation.
- **Essential vs accidental:** Accidental leftover of Program/stratum
  eval. Relating engine deltas to the lfp is essential *as a
  theorem*, not as a second meaning of Query.
- **Severity:** med

### M5. `recDom` still talks `idb` / "old program domain"

- **Where:** `Exec/Reach.lean:359–369`.
- **What's wrong:** Comment: "Ignores the accumulating self (same as
  ignoring `idb` on the old program domain)." The function walks
  `base ++ rec` atoms as a Program EDB/IDB split, then special-cases
  `interior C` vs `edb`. `self` is not in the type of the domain, so
  a self-atom under `V.update self acc` is handled later with
  extra `by_cases Q = self` in `evalRule_in_cands` (`559–579`).
- **What representation collapses it:** H2+H3: rec domain is EDB
  columns plus finished-interior columns; `recSelf` is not a source
  that contributes to the *input* domain (heads project bound vars —
  already the creation quarantine). The `idb` coordinate and the
  `Q = self` branch go away.
- **Essential vs accidental:** Accidental leftover Program/IDB.
  Active-domain finiteness is essential (`reach_den_finite`).
- **Severity:** med

### M6. `Option Rec` / `rec.isSome` as null in the type

- **Where:** `derivedCount` `Syntax.lean:324–325`
  `interiors.length + (if q.rec.isSome then 1 else 0)`;
  `recLinear` none-arm `483–484`; `evalQuery`/`evalQueryList` match
  (`Reach.lean:752–756`, `773–778`); `decodeRecOpt`
  `Main.lean:393–398` (`none` and JSON `null` both mean absent).
- **What's wrong:** Hoare's mechanism. Absence of rec is a value in
  the product, so every consumer branches. `derivedCount` is a flag
  reconstructing H1's sum.
- **What representation collapses it:** H1. `derivedCount` is
  `interiors.length` on the interiors constructors and
  `interiors.length + 1` on the rec constructor — or just "the
  `Vector`'s length."
- **Essential vs accidental:** Accidental.
- **Severity:** med

### M7. `InteriorEnv` is a total `InteriorId → Set`; phantom = unread = empty

- **Where:** `Denotation.lean:689–704` (`InteriorEnv`, `sourceDen`
  unread interior is empty); `InteriorEnv.empty`; module doc
  `Syntax.lean:67–76` (phantom positive kill / negated phantom
  vacuous — recorded, then left in the type).
- **What's wrong:** A free `InteriorId` is in every environment
  (null-in-every-type). `sourceDen` cannot distinguish "not a table
  of this query" from "this table is empty," which is why negated
  phantoms are vacuously true and why `sourcesInRange` exists.
  Acceptance screen instead of a typed environment.
- **What representation collapses it:** Environment is
  `Vector (Set AnswerTuple) n` (or a telescope). Out-of-range reads
  do not type. The phantom reading and its screen become
  unrepresentable; the engine's `UnknownInterior` is the parse at
  the boundary.
- **Essential vs accidental:** Accidental given H3. Empty tables as
  empty sets are essential set semantics.
- **Severity:** med

### M8. `Rule.edbOnly` / hostile interiors on `Query.plain`

- **Where:** `Syntax.lean:327–332`, `71–73`, `plain_wellFormed`
  `498–507` (hostile `Interior` atoms fail `sourcesInRange` because
  `derivedCount = 0`).
- **What's wrong:** A "plain" query can still carry `.interior`
  atoms. `edbOnly` is a Prop flag reconstructing "this Rule cannot
  mention interiors." Dual with `Query.Plain`.
- **What representation collapses it:** H1 `Query.plain`'s atoms are
  `edb`-only in the type (`AtomSource` without `interior` on that
  constructor, or a `PlainAtom`). `edbOnly` and the hostile-phantom
  paragraph go away.
- **Essential vs accidental:** Accidental.
- **Severity:** med

---

## LOW

### L1. `odd_not_stratified` is a leftover stratum name

- **Where:** `Countermodels.lean:1374`, `1411–1417`. Statement is
  `¬ oddQuery.recLinear`.
- **What's wrong:** Stratification coordinate in the name after the
  cut replaced strata with `recLinear`. The wall is real (keep
  `odd_not_monotone`); the name reconstructs Program/stratum.
- **What representation collapses it:** Rename to `odd_not_recLinear`
  (and, with H2, the syntax inhabitant disappears entirely).
- **Essential vs accidental:** Accidental naming leftover.
- **Severity:** low

### L2. `RewriteStep` threads unused `n : Nat` (the orphan arity)

- **Where:** `Exec/Rewrites.lean:2310–2359` — every constructor
  `{n : Nat} … Query.plain n …`.
- **What's wrong:** Arity is a dummy to inhabit `Query.plain`. Special
  case of H5/H6 at the rewrite relation. Low only because constructors
  already force plain (the Program wrap is the high finding).
- **What representation collapses it:** `RewriteStep` over `List Rule`.
- **Essential vs accidental:** Accidental.
- **Severity:** low

### L3. `reachOp_empty` / `selfCount_eq_one_mem` are guards on numeric linearity

- **Where:** `Reach.lean:207–222`, `239–258`.
- **What's wrong:** `selfCount = 1` is unpacked to `∃ atom named self`
  by filtering `decide (a.source = .interior self)` and casing on
  list length. Control flow recovering a field H2 would have as
  `RecStep.self`.
- **What representation collapses it:** H2. `selfCount_eq_one_mem`
  becomes `r.self` definitional.
- **Essential vs accidental:** Accidental (corollary of H2).
- **Severity:** low

### L4. `evalQuery_empty_rules` special-cases "rec is never the answer"

- **Where:** `Reach.lean:1065–1069`; Bridge `583–586`.
- **What's wrong:** True because main `rules` and rec are peer fields
  of a product; empty main with a live rec still denotes `∅`. A rec
  constructor whose *answer* is the rec table would be a different
  language. Here the special case is: two result-shaped fields, only
  one is the answer. Mild leftover Program (IDB vs query head).
- **What representation collapses it:** H1 — the rec constructor still
  has a main `rules` as the query head (essential: rec is a derived
  table, not the result). The theorem stays; it just is not a
  product-field surprise. Borderline; filed because the Bridge still
  has to say "the rec is never the answer."
- **Essential vs accidental:** Mostly essential language (query
  result ≠ rec table). The surprise is accidental product layout.
- **Severity:** low

---

## Counts

| Severity | Count |
|----------|------:|
| high     |     6 |
| med      |     8 |
| low      |     4 |
| **total**|  **18** |

The cut renamed Program to Query and replaced strata with `lfpS`, then
left the Program *coordinate system* in place: optional rec, free
`InteriorId`, floating arity, one `Rule` type, and a second denotation
(`rulesAnswers ∘ edbEnv`) that the rest of the tree still inhabits.
Control flow (`recLinear`, `match q.rec`, `selfCount`, `fillerValue`,
`do not match recId`) is downstream of that representation.
