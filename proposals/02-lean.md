# 02 — Lean refactor spec

This file is the working spec. Open `lean/Bumbledb/Query/Syntax.lean` next. Do not wait for Rust. Do not keep `Program` as a compatibility synonym.

Caps (`MAX_RULES`, `MAX_CTES`, `MAX_CONDITION_DEPTH`) stay unmodeled — hostile-input mechanism, same narrowing as today's `Syntax.lean` module doc.

**This file is enough to start.** Module filenames, inductive types, constructor rewrites, and keep/retarget/delete are below. Do not merge a half-tree (see README).

## Target module map

| Today | After |
|---|---|
| `Query/Syntax.lean` — `Query`, `Rule`, `Atom` (`relation : RelId`), then the program cut (`PredId`, `AtomSource`, `PAtom`, `PRule`, `PredicateDef`, `Program`, `StratifiedBy`, `WellFormed`, `Query.toProgram`) | `Query/Syntax.lean` — merge the cut into the query: `Atom.source : AtomSource`, `AtomSource = edb \| cte`, `CteId`, `WithDef`, `RecCte`, `Query` with `views` / `rec` (`views`, not `with`). Delete every `Program*` / `PAtom` / `PRule` / `PredicateDef` / `PredId` / `StratifiedBy` / `toProgram` declaration |
| `Query/Denotation.lean` — `Matches`, `derives` over `I a.relation`, `ruleAnswers`, `queryAnswers`, `evalList`, `eval_sound` | Same file. `Matches` unchanged (never read the source). `derives` / `ruleAnswers` / `rulesAnswers` take `F : AtomSource → Set Fact`. Today's `queryAnswers C q I ρ` becomes `rulesAnswers C q.rules (edbEnv I) ρ` under `Plain q`. New: `edbEnv`, `sourceDen`, `tupleFact` (move down from Fixpoint). `evalList` / `eval_sound` retarget to `rulesAnswers` |
| `Query/Membership.lean` | Keep. **Lie withdrawn:** this file is full of `a.relation`. After the cut, `decode`/`theorems` match `a.source = .edb R` and use `R`. CTE membership is engine-only (`CteSignatures` in normalize) — same as today's Idb, which Membership never modeled (`PAtom` lived elsewhere). Do **not** add `def Atom.relation` that returns `⟨0⟩` on `.cte` — that is the RelId pun |
| `Query/Aggregates.lean` | Keep. `bindingSet` / `Group` / `aggAnswers` take `F`. Recover `I` via `edbEnv I`. PRD 05 never enters `reachOp` |
| `Exec/Fixpoint.lean` — `programDen`, `stratumOp`, `finished`, `fueledLoop`, `evalProgram`, `degenerate_embedding`, `program_eval_sound`, `PAtom.code` | **Delete the file.** Replace with `Exec/Reach.lean`: `lfpS`, `evalWith`, `reachOp`, `reachDen`, `evalQuery`, `evalLinearReach`, `evalQueryList`, agreement theorems. `lfpP` / `PredSets` / `stratumSets` / `stratumOp` / `finished` / `programDen` / `programAnswers` / `evalProgram*` / `strataEval` / `degenerate_embedding` / the even-odd coding / `PRule.Safe` **do not move** — they die. `semi_naive_agrees` **moves** (already `{α} (T : Set α → Set α)`). `fueledLoop` **moves as a private proof device** for `evalLinearReach_eq_lfp`, not as a public denotation |
| `Exec/Plan.lean`, `Exec/Dedup.lean`, `Exec/Rewrites.lean`, `Exec/Sweep.lean` | Stay. They quantify over **rule lists**, not Programs. `a.relation` → match `.edb`. `⟨n, rs⟩` → `Query.plain n rs` or drop the Query wrapper. `queryAnswers C ⟨n, rs⟩ I ρ` → `rulesAnswers C rs (edbEnv I) ρ`. Comments that say `Program::Empty` mean today's **prepared** empty plan — retarget the comment to `PreparedBody::Empty`. Do not grow CTE-aware rewrites |
| `Bridge.lean` rows on `degenerate_embedding`, `wellFormed_reads_real`, `stratumOp_mono`, `program_den_finite`, `program_eval_sound` (×2), `semi_naive_agrees` (×2) | Retarget per the Bridge table below. `ledger_count` moves with the rows |
| `Countermodels.lean` — `oddProgram`, `odd_not_monotone`, `succ_prefixed_infinite` | Keep the **walls**. Rewrite `oddProgram` as a `RecCte` that `recLinear` refuses (self-negation). `succOp` stays an operator-level countermodel |
| `Main.lean` `ProgramCase` / `checkProgramCase` / `evalProgram` | `ReachCase` / `checkReachCase` / `evalQueryList`. Recut `program-*.json` → `reach-*.json` in the **same Lean commit** |
| `Conformance.lean` `decodeAtom` / `decodeQuery` (`CQuery`, aggregate finds, JSON `"relation"`) | **Keep this arm.** Seeded query cases stay here. `decodeAtom` maps `"relation"` → `.edb`. Do not require `views`/`rec` keys on seeded files |
| `Bumbledb.lean`, `lean/README.md` | Import `Exec/Reach` instead of `Exec/Fixpoint`. README Level-1 bullet: Reach, not stratified fueled `evalProgram` |
| `lean/conformance/README.md` | Dispatch: `judgment-*` unchanged; `reach-*` → `evalQueryList`; else CQuery. Drop `program-*` |

`Exec/Plan.lean`, `Exec/Dedup.lean`, `Exec/Rewrites.lean`, `Exec/Sweep.lean` stay. After the cut:

- `queryAnswers C q I ρ` → `rulesAnswers C rules (edbEnv I) ρ`.
- `Query` constructors `⟨n, rules⟩` → `Query.plain n rules` where a whole Query is still required; prefer quantifying `List Rule` when the theorem never reads `views` / `rec`.
- `evalList C W ρ q` → `evalList C W CteTables.empty ρ q.rules`.
- `a.relation` → `match a.source with | .edb R => ... | .cte _ => (skip / False)`. Grounding's "Idb refuses" becomes "Cte refuses" (CTE tables are not stored relations; statements do not mention them). Do not grow CTE-aware rewrite rules in this cut.

### Constructor grep (do this in the Syntax commit, or the tree will not elaborate)

Every `Atom` positional `{ relation := R, ... }` / `⟨R, bs⟩ : Atom` becomes `{ source := .edb R, bindings := bs }`. Sites: `Query/Membership.lean`, `Query/Denotation.lean` (if any), `Exec/{Plan,Dedup,Rewrites,Sweep}.lean`, `Countermodels.lean`, `Conformance.lean` `decodeAtom`, `Txn/*` if they build Atoms. `rg 'relation :=|a\.relation|⟨n, .*: Query'` is the queue.

`Conformance.decodeAtom` JSON key stays `"relation"` (seeded corpus). Lean value is `.edb`.

## Types — `Query/Syntax.lean`

Delete the entire "program cut" section from `PredId` through `Query.toProgram_wellFormed`. Replace with the following, in this style (names, `deriving`, comment shape).

```lean
structure CteId where
  id : Nat
deriving DecidableEq

inductive AtomSource where
  | edb (R : RelId)
  | cte (C : CteId)
deriving DecidableEq

def AtomSource.cte? : AtomSource → Option CteId
  | .cte C => some C
  | .edb _ => none

def AtomSource.edb? : AtomSource → Option RelId
  | .edb R => some R
  | .cte _ => none

structure Atom where
  source : AtomSource
  bindings : List (FieldId × Term)

-- Rule, Condition, Term, Safe, WellTyped: unchanged, except Atom.vars
-- already reads bindings only.

structure WithDef where
  arity : Nat
  rules : List Rule

structure RecCte where
  arity : Nat
  base : List Rule
  rec : List Rule

/-- Engine field is `with`. Lean cannot use that name (`with` is a
keyword: `match … with`, `{s with field := …}`). The list is `views`. -/
structure Query where
  views : List WithDef
  rec : Option RecCte
  arity : Nat
  rules : List Rule

def Query.plain (arity : Nat) (rules : List Rule) : Query :=
  { views := [], rec := none, arity, rules }

def Query.Plain (q : Query) : Prop :=
  q.views = [] ∧ q.rec = none

def Query.allRules (q : Query) : List Rule :=
  q.views.flatMap (·.rules) ++
    (match q.rec with
     | none => []
     | some r => r.base ++ r.rec) ++
    q.rules

def Rule.edbOnly (r : Rule) : Prop :=
  ∀ a, (a ∈ r.atoms ∨ a ∈ r.negated) → ∃ R, a.source = .edb R
```

`Atom.toPAtom` / `Rule.toPRule` / `Query.toProgram` / `Query.toProgram_wellFormed` are gone. There is no `degenerate_embedding` because there is no embedding type.

**Do not** define `Atom.relation : RelId` as a total function. `PredId` is gone; `CteId` is not a `RelId`.

`Rule.finds : List VarId` stays (PRD 04 narrowing: aggregates/measures are PRD 05). Consequently WITH/rec heads are projection-shaped **by construction** in Lean. The engine roster that refuses `Aggregate` / `Measure` finds on those heads is discharging a class Lean cannot write — same relationship today's interior-predicate roster had to `PRule.finds`.

### Identifiers

`CteId` is a separate identity from `RelId`. No statement form carries a `CteId`. A statement about a CTE is unwritable — today's `PredId` law, restated.

Numbering, normative in Lean and in the engine:

- `WithDef` at list index `i` has `CteId` `⟨i⟩`.
- If `q.rec = some rec`, the rec CTE has `CteId` `⟨q.views.length⟩`.
- Nothing else is a `CteId`.

```lean
def Query.recId (q : Query) : Option CteId :=
  match q.rec with
  | none => none
  | some _ => some ⟨q.views.length⟩

def Query.cteCount (q : Query) : Nat :=
  q.views.length + (if q.rec.isSome then 1 else 0)
```

## Well-formedness — one predicate, no SCC

Delete `Program.StratifiedBy`, `Program.Stratified`, `Edge`, `EdgeKind`, `PRule.edges`. Stratification of a general IDB is not a judgment this language makes. Linearity + no-negation-in-rec **are**.

```lean
def Rule.cteReads (r : Rule) : List CteId :=
  (r.atoms ++ r.negated).filterMap fun a => a.source.cte?

def Rule.positiveCteReads (r : Rule) : List CteId :=
  r.atoms.filterMap fun a => a.source.cte?

def Rule.selfCount (r : Rule) (self : CteId) : Nat :=
  (r.atoms.filter fun a => decide (a.source = .cte self)).length

def Rule.hasNegatedSelf (r : Rule) (self : CteId) : Prop :=
  ∃ a, a ∈ r.negated ∧ a.source = .cte self

/-- Every cte source names a real WITH or the rec. -/
def Query.sourcesInRange (q : Query) : Prop :=
  ∀ r, r ∈ q.allRules → ∀ a, (a ∈ r.atoms ∨ a ∈ r.negated) →
    ∀ C, a.source = .cte C → C.id < q.cteCount

/-- WITH i reads only strictly earlier WITH. The rec id is never < i. -/
def Query.withDag (q : Query) : Prop :=
  ∀ i d, q.views[i]? = some d → ∀ r, r ∈ d.rules →
    ∀ C, C ∈ r.cteReads → C.id < i

/-- Match `q.rec` only. `self` is `⟨q.views.length⟩` — do not match
`recId` beside it (the catch-all is not unreachable to the elaborator). -/
def Query.recLinear (q : Query) : Prop :=
  match q.rec with
  | none => True
  | some rec =>
      let self : CteId := ⟨q.views.length⟩
      rec.base ≠ [] ∧ rec.rec ≠ [] ∧
      (∀ r, r ∈ rec.base → r.selfCount self = 0 ∧ ¬ r.hasNegatedSelf self) ∧
      (∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ ¬ r.hasNegatedSelf self) ∧
      (∀ r, r ∈ rec.base ++ rec.rec → r.negated = [])

def Query.WellFormed (q : Query) : Prop :=
  q.sourcesInRange ∧ q.withDag ∧ q.recLinear
```

`Query.allRules` is the quantification surface `Program.rulesList` was.

`recLinear` bans **all** negation in the rec CTE (`negated = []`), not only self-negation — matches `NegationInRecCte`.

**Recorded narrowing, measure-in-rec.** Lean `Term.measure` is still writable in a rec-arm comparison. The engine refuses it (`MeasureInRecCte`). Lean WF as stated does not mention measure: `Rule.WellTyped` already bans measure in **bindings**; comparison-side measure in a rec CTE is engine-only, like today's `MeasureInRecursiveHead` sitting beside `PRule.finds : List VarId`. Do not grow Lean WF into the positional type roster. The denotation of a measure comparison on a ray is `False` here (`Value.measure? = none`); the engine raises `MeasureOfRay`. Same narrowing as `Denotation.lean`'s module doc. Rec CTE plus measure is refused before denotation on accepted programs.

**Head arity.** Each `WithDef.arity` / `RecCte.arity` / `Query.arity` agrees with every of its rules' `finds.length`. Unmodeled in Lean (today's Query similarly carries `arity` without a WF clause forcing it); the engine's head-alignment roster discharges it. Theorems that project through `finds` do not need the clause.

**Empty main.** Lean `evalQuery_empty_rules`: `q.rules = []` denotes `∅`. The engine refuses `EmptyRuleSet`. Same narrowing as today's empty query — recorded, not a WF clause.

**Plain queries are well-formed only when they are all-EDB.** `Query.plain` does **not** rewrite atoms to `.edb` (that was `toPRule`'s job, and it dies). Hostile `Cte` atoms on a plain query fail `sourcesInRange` (`cteCount = 0`). This **replaces** `Query.toProgram_wellFormed`, which was vacuously true because the embedding forced edb:

```lean
theorem Query.plain_wellFormed (arity : Nat) (rules : List Rule)
    (hedb : ∀ r, r ∈ rules → r.edbOnly) :
    (Query.plain arity rules).WellFormed := by
  -- sourcesInRange: edbOnly ⇒ no cte sources.
  -- withDag: views = []. recLinear: rec = none.
```

All existing Plan/Rewrites constructors that build `⟨n, rs⟩` from stored-relation atoms satisfy `edbOnly`.

## Denotation — `Query/Denotation.lean`

### Step 1 — environment, not instance, in the body judgment

Today:

```lean
def derives (C : Classify) (r : Rule) (I : Instance) (ρ : ParamEnv)
    (σ : Assignment) : Prop :=
  (∀ a, a ∈ r.atoms → ∃ f, f ∈ I a.relation ∧ Matches f a σ ρ) ∧
  (∀ a, a ∈ r.negated → ¬ ∃ f, f ∈ I a.relation ∧ Matches f a σ ρ) ∧
  (∀ t, t ∈ r.conditions → Condition.holds C ρ σ t)

def ruleAnswers (C : Classify) (r : Rule) (I : Instance)
    (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ σ, derives C r I ρ σ ∧ t = r.finds.map σ

def queryAnswers (C : Classify) (q : Query) (I : Instance)
    (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ r, r ∈ q.rules ∧ t ∈ ruleAnswers C r I ρ
```

After. `Matches` is unchanged (`Term.selects` on `bindings` — it never read `relation`). Move `tupleFact` / `fillerValue` from `Fixpoint.lean` into this file (they are the CTE fact reading, not a fixpoint device):

```lean
def fillerValue : Value := ⟨.bool, false⟩

def tupleFact (t : AnswerTuple) : Fact :=
  fun i => (t[i.id]?).getD fillerValue

abbrev CteEnv : Type := CteId → Set AnswerTuple

def CteEnv.empty : CteEnv := fun _ _ => False

def CteEnv.update (W : CteEnv) (c : CteId) (X : Set AnswerTuple) : CteEnv :=
  fun d => if d = c then X else W d

def sourceDen (I : Instance) (W : CteEnv) : AtomSource → Set Fact
  | .edb R => I R
  | .cte C => fun f => ∃ t, t ∈ W C ∧ f = tupleFact t

def edbEnv (I : Instance) : AtomSource → Set Fact :=
  sourceDen I CteEnv.empty

def derives (C : Classify) (r : Rule) (F : AtomSource → Set Fact)
    (ρ : ParamEnv) (σ : Assignment) : Prop :=
  (∀ a, a ∈ r.atoms → ∃ f, f ∈ F a.source ∧ Matches f a σ ρ) ∧
  (∀ a, a ∈ r.negated → ¬ ∃ f, f ∈ F a.source ∧ Matches f a σ ρ) ∧
  (∀ t, t ∈ r.conditions → Condition.holds C ρ σ t)

def ruleAnswers (C : Classify) (r : Rule) (F : AtomSource → Set Fact)
    (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ σ, derives C r F ρ σ ∧ t = r.finds.map σ

/-- The union of a rule list — today's `queryAnswers` body. -/
def rulesAnswers (C : Classify) (rules : List Rule) (F : AtomSource → Set Fact)
    (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ r, r ∈ rules ∧ t ∈ ruleAnswers C r F ρ

theorem mem_rulesAnswers {C : Classify} {rules : List Rule}
    {F : AtomSource → Set Fact} {ρ : ParamEnv} {t : AnswerTuple} :
    t ∈ rulesAnswers C rules F ρ ↔
      ∃ r, r ∈ rules ∧ t ∈ ruleAnswers C r F ρ :=
  Iff.rfl
```

**Both polarities** go through `F a.source`. There is no second path for negated atoms. `sourceDen` of an unread CTE id is the empty fact set: a positive phantom kills the rule; a negated phantom vacuously passes — the unknown-PredId gap, restated. `sourcesInRange` + `withDag` are the screen.

Recover the EDB-only reading so existing proofs retarget by a one-line wrapper, not a rewrite of every lemma:

```lean
theorem derives_edb {C : Classify} {r : Rule} {I : Instance}
    {ρ : ParamEnv} {σ : Assignment}
    (hedb : r.edbOnly) :
    derives C r (edbEnv I) ρ σ ↔
      (∀ a, a ∈ r.atoms → ∃ R f, a.source = .edb R ∧ f ∈ I R ∧ Matches f a σ ρ) ∧
      (∀ a, a ∈ r.negated → ¬ ∃ R f, a.source = .edb R ∧ f ∈ I R ∧ Matches f a σ ρ) ∧
      (∀ t, t ∈ r.conditions → Condition.holds C ρ σ t)
```

Do not keep a second `derives` overloaded on `Instance`. One judgment.

`pderives` / `pruleAnswers` / `PMatches` in Fixpoint.lean **are** this `derives` / `ruleAnswers` / `Matches`. Do not duplicate them in Reach.lean. Delete them with Fixpoint.lean.

### `queryAnswers` — the name

**Decision:** retire `queryAnswers` as a definition. Every current theorem that says `t ∈ queryAnswers C q I ρ` retargets to `t ∈ rulesAnswers C q.rules (edbEnv I) ρ`, and gains `q.Plain` if it mentions `q.views` / `q.rec` (almost none do — today's `Query` has only `arity`/`rules`; after widening, those theorems are about `q.rules` and should not mention `Query` at all where a `List Rule` suffices).

Rename in place (same job):

| Today | After |
|---|---|
| `mem_queryAnswers` | `mem_rulesAnswers` |
| `queryAnswers_congr_at` / `_drop_at` / `_drop_covered` (`Rewrites.lean`) | `rulesAnswers_congr_at` / `_drop_at` / `_drop_covered` over `List Rule` |
| `dnf_preserves_denotation` | retarget the `queryAnswers` mention |
| `answers_finite_of_safe` | same retarget |
| `eval_sound` | `evalList` ↔ `rulesAnswers` over `sourceDen W.den T.toEnv` |

### Executable join — `factsOf` is the only path

Today `evalRule` / `joinAtoms` read `W.facts a.relation` on both the join and the negated filter. After the `Atom.source` cut they take a CTE table map `T`. **Do not** bind CTE tables into a `ListInstance` by allocating fresh `RelId`s — that is `AtomSource.code` under a new name. Delete the even/odd coding. `factsOf` is the only executable join.

```lean
abbrev CteTables : Type := CteId → List AnswerTuple

def CteTables.empty : CteTables := fun _ => []

def factsOf (W : ListInstance) (T : CteTables) : AtomSource → List Fact
  | .edb R => W.facts R
  | .cte C => (T C).map tupleFact

def CteTables.update (T : CteTables) (c : CteId) (rows : List AnswerTuple) :
    CteTables :=
  fun d => if d = c then rows else T d

def CteTables.toEnv (T : CteTables) : CteEnv :=
  fun c t => t ∈ T c

/-- Today's `joinAtoms`, fact source `factsOf` on **both** polarities. -/
def joinAtoms (W : ListInstance) (T : CteTables) (ρ : ParamEnv) :
    List Atom → List PartialAssign → List PartialAssign
  | [], σs => σs
  | a :: rest, σs =>
    joinAtoms W T ρ rest
      (σs.flatMap fun σ =>
        (factsOf W T a.source).filterMap fun f =>
          bindAtom ρ f a.bindings σ)

/-- Today's `evalRule`. Negated filter iterates `factsOf`, never
`W.facts a.relation`. -/
def evalRule (C : Classify) (W : ListInstance) (T : CteTables)
    (ρ : ParamEnv) (r : Rule) : List AnswerTuple :=
  ((joinAtoms W T ρ r.atoms [[]]).filter fun σp =>
    (r.negated.all fun a =>
      (factsOf W T a.source).all fun f =>
        ! matchesB ρ (totalize σp) a f) &&
    (r.conditions.all fun t => condHoldsB C ρ (totalize σp) t)).map
    fun σp => r.finds.map (totalize σp)

/-- Today's `evalList` took a `Query` and read `q.rules`. After: a
rule list plus `T`. Plain callers pass `CteTables.empty`. -/
def evalList (C : Classify) (W : ListInstance) (T : CteTables)
    (ρ : ParamEnv) (rules : List Rule) : List AnswerTuple :=
  rules.flatMap (evalRule C W T ρ)
```

`eval_sound` (retargeted; no longer quantified over `Query`):

```lean
theorem eval_sound {C W T ρ rules}
    (hsafe : ∀ r, r ∈ rules → Safe r)
    (hwt : ∀ r, r ∈ rules → r.WellTyped) :
    ∀ t, t ∈ evalList C W T ρ rules ↔
      t ∈ rulesAnswers C rules (sourceDen W.den T.toEnv) ρ
```

Plain queries: `T = CteTables.empty` ⇒ `T.toEnv = CteEnv.empty` ⇒ `sourceDen W.den empty = edbEnv W.den`. CTE evaluation is `evalQueryList` composing the same `evalList` with a growing `T`.

`eval_sound` does **not** need `WellFormed` — phantom CTE reads are empty on both sides, same as today's `program_eval_sound` without `Program.WellFormed`.

## WITH — `evalWith`

In `Exec/Reach.lean`. File header (this is the module doc; write it first):

```lean
/-!
# Exec/Reach — WITH DAG, one linear reach, the query denotation

Level 0: `evalWith`, `reachOp`, `reachDen = lfpS`, `evalQuery`.
No fuel. No strata. No Program.

Level 1: `evalLinearReach`, `evalQueryList`, proved equal to Level 0.
`fueledLoop` is a **private** termination metric (`missingCount_le` is
why `cands.length + 1` always suffices). It is not a parameter of any
public def and not a Bridge incompleteness caveat.

Engine `FixpointBudgetExceeded` is incompleteness vs `reachDen`, the
same class as `ResultBytesOverflow` vs `rulesAnswers`.
-/
```

Level 0 denotation of WITH (no fuel, no rounds):

```lean
/-- Finished WITH tables after the first `n` defs (declaration order).
`evalWithAt … 0` is empty. `evalWithAt … (k+1)` writes `CteId ⟨k⟩`
from `defs[k]?` against the prefix env. Call `evalWithAt … defs.length`
to finish every view — not `defs.length - 1`. -/
def evalWithAt (C : Classify) (defs : List WithDef) (I : Instance)
    (ρ : ParamEnv) : Nat → CteEnv
  | 0 => CteEnv.empty
  | n + 1 =>
    let prev := evalWithAt C defs I ρ n
    fun c t =>
      if h : c.id < n then
        prev c t
      else if c.id = n then
        match defs[n]? with
        | some d => t ∈ rulesAnswers C d.rules (sourceDen I prev) ρ
        | none => False
      else False

def evalWith (C : Classify) (q : Query) (I : Instance) (ρ : ParamEnv) :
    CteEnv :=
  evalWithAt C q.views I ρ q.views.length
```

**Specified cases:**

| Input | `evalWith` |
|---|---|
| `q.views = []` | `CteEnv.empty` (plain and rec-only) |
| WITH `i`, both polarities | `rulesAnswers` of `d.rules` over `sourceDen I prev` — negated earlier WITH is an anti-join of a finished set |
| Forward / rec id in a WITH body | `prev` has not written that id → empty facts. Positive phantom kills; negated phantom vacuously passes. `withDag` refuses this on accepted queries |
| Empty `WithDef.rules` | that id denotes `∅`. Engine `EmptyCte` |

Each WITH is the union of its rules over EDB plus **already-finished** earlier WITH. Evaluated once. Not a fixpoint.

```lean
theorem evalWithAt_stable {C defs I ρ n}
    (h : n ≥ defs.length) :
    ∀ c, c.id < defs.length →
      evalWithAt C defs I ρ n c = evalWithAt C defs I ρ defs.length c
```

Under `q.withDag`, a WITH body that names `Cte ⟨j⟩` with `j ≥ i` is excluded by WF. `sourcesInRange` plus `withDag` are the screen. Spend them:

```lean
theorem wellFormed_cte_reads_real {q : Query} (hwf : q.WellFormed)
    {r : Rule} (hr : r ∈ q.allRules) {a : Atom}
    (ha : a ∈ r.atoms ∨ a ∈ r.negated) {C : CteId}
    (hsrc : a.source = .cte C) :
    C.id < q.cteCount
```

This **replaces** `wellFormed_reads_real`. Bridge retargets onto it. It does **not** by itself prove WITH doesn't read rec — `withDag` does (`recId` is never `< i`).

## Reach — `reachOp`, `reachDen`, no fuel

Knaster–Tarski over a **single** set, not `PredSets`. `lfpP` dies with Program. Do not encode the rec CTE as "a one-predicate program."

```lean
def lfpS {α : Type u} (T : Set α → Set α) : Set α :=
  fun a => ∀ X, (∀ x, x ∈ T X → x ∈ X) → a ∈ X

def MonoS {α : Type u} (T : Set α → Set α) : Prop :=
  ∀ X Y, (∀ a, a ∈ X → a ∈ Y) → ∀ a, a ∈ T X → a ∈ T Y

theorem lfpS_fixed {α} {T : Set α → Set α} (hm : MonoS T) :
    ∀ a, a ∈ T (lfpS T) ↔ a ∈ lfpS T :=
  -- port lfpP_fixed; PredSets.le becomes pointwise ∈ on one set
```

The operator. `W` is the finished WITH environment (rec id unread — `evalWith` never writes it). `self = ⟨q.views.length⟩`.

```lean
def reachOp (C : Classify) (rec : RecCte) (self : CteId)
    (I : Instance) (W : CteEnv) (ρ : ParamEnv)
    (X : Set AnswerTuple) : Set AnswerTuple :=
  fun t =>
    t ∈ rulesAnswers C rec.base (sourceDen I W) ρ ∨
    t ∈ rulesAnswers C rec.rec (sourceDen I (W.update self X)) ρ

def reachDen (C : Classify) (rec : RecCte) (self : CteId)
    (I : Instance) (W : CteEnv) (ρ : ParamEnv) : Set AnswerTuple :=
  lfpS (reachOp C rec self I W ρ)
```

**Base does not see `X`.** Base is evaluated against `W`, not `W.update self X`. A `SelfInBase` arm (illegal) therefore reads the unfinished rec id = empty, not the recursive argument. Do not evaluate illegal programs; `recLinear` refuses the shape.

**Rec arms see `X` at `self` and finished WITH in `W`.** Extra WITH joins on a rec arm are legal (`01-language.md`).

**Monotonicity.** Rec arms read `X` only through a **positive** self-atom (`recLinear`). Base does not read `self`. The rec CTE has `negated = []`, so no anti-join of `X`. Therefore:

```lean
theorem reachOp_mono {C : Classify} {rec : RecCte} {self : CteId}
    {I : Instance} {W : CteEnv} {ρ : ParamEnv}
    (hlin : ∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ r.negated = []) :
    MonoS (reachOp C rec self I W ρ)
```

This **replaces** `stratumOp_mono`. The stratification premise is gone; linearity + no-negation-in-rec are the premise. `Countermodels.odd_not_monotone` remains the wall: a rec CTE with a negated self-atom is not `recLinear`, and the operator is not monotone. Do not delete the countermodel; retarget its syntax to a `Rule` with `negated = [⟨.cte self, []⟩]` and show `¬ MonoS (reachOp ...)` on that illegal CTE. WF is what keeps accepted programs off the wall.

**Round 0 is base. Empty Δ is a closed step.**

```lean
theorem reachOp_empty {C rec self I W ρ}
    (hpos : ∀ r, r ∈ rec.rec → r.selfCount self = 1) :
    ∀ t, t ∈ reachOp C rec self I W ρ (fun _ => False) ↔
         t ∈ rulesAnswers C rec.base (sourceDen I W) ρ
```

A positive self-atom against an empty table derives nothing, so `rec(∅) = ∅`, so `T(∅) = base`. The engine's "round 0 runs base arms" is this equation, not a driver quirk.

Empty rec list (illegal `EmptyRecursiveStep`): `T(X) = base`, lfp = base — that is a WITH. Do not evaluate it as rec.

Empty base (illegal `EmptyRecursiveBase`): `T(∅) = rec(∅) = ∅`, lfp empty.

**Finiteness.** Port `progDom` / `allTuples` / `pevalRule_dom` off `Program` onto `RecCte` plus the EDB/WITH facts in `W`. Heads are bound vars, so derived tuples live on the active domain of base (stored columns and finished WITH columns) plus what rec arms copy through the self-atom (inductively the same domain).

Today's `progDom` ignored `idb` (`| idb _ => []`) and recovered IDB values inductively from the table. `recDom` **includes finished WITH tables** `V` — rec arms legally join them, and those values must sit in the candidate domain on round 0. Self tuples stay on that domain inductively. Still ignore the accumulating self in `recDom` (same as ignoring `idb`).

```lean
theorem reach_den_finite (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (rec : RecCte) (self : CteId) (V : CteTables)
    (hsafe : ∀ r, r ∈ rec.base ++ rec.rec → Safe r)
    (hwt : ∀ r, r ∈ rec.base ++ rec.rec → r.WellTyped)
    (hlin : ∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ r.negated = []) :
    (reachDen C rec self W.den V.toEnv ρ).Finite
```

This **replaces** `program_den_finite`. Same wall: `succ_prefixed_infinite`.

## `evalLinearReach` — Level 1, complete, no fuel in the signature

```lean
def recDom (rec : RecCte) (W : ListInstance) (V : CteTables) : List Value :=
  fillerValue ::
    (rec.base ++ rec.rec).flatMap fun r =>
      r.atoms.flatMap fun a =>
        match a.source with
        | .edb R => a.bindings.flatMap fun b => (W.facts R).map (· b.1)
        | .cte C => a.bindings.flatMap fun b =>
            (V C).flatMap fun t => (t[b.1.id]?).toList

/-- Naive step: `T(acc) = base ∪ rec(acc)`. List concat is the union;
membership is the set. This is **not** the engine's delta loop. -/
def reachStep (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (rec : RecCte) (self : CteId) (V : CteTables)
    (base : List AnswerTuple) (acc : List AnswerTuple) :
    List AnswerTuple :=
  let T := V.update self acc
  base ++ evalList C W T ρ rec.rec

def evalLinearReach (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (rec : RecCte) (self : CteId) (V : CteTables) : List AnswerTuple :=
  let base := evalList C W V ρ rec.base
  let cands := allTuples (recDom rec W V) rec.arity
  fueledLoop (reachStep C W ρ rec self V base) (cands.length + 1) []
```

`fueledLoop` is **private** in this file (today's definition, moved, not public). `fueledLoop_fixpoint` / `missingCount` / `missingCount_le` stay private beside it. Callers cannot under-fuel the spec evaluator. The fuel argument is not a parameter of `evalLinearReach`.

Today's stop: `(step acc).all (· ∈ acc)` i.e. `step acc ⊆ acc`. That **is** engine empty Δ after identifying `new = T(acc) \ acc`:

- Start `acc = []`. First growing step (if any) is `T(∅) = base` (`reachOp_empty`).
- Later, `T(acc) ⊆ acc` iff no new rec tuples — the engine's empty frontier.
- `cands.length + 1` fuel is `missingCount_le`, not a semantic parameter.

```lean
theorem evalLinearReach_eq_lfp
    (hsafe : ∀ r, r ∈ rec.base ++ rec.rec → Safe r)
    (hwt : ∀ r, r ∈ rec.base ++ rec.rec → r.WellTyped)
    (hlin : ∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ r.negated = []) :
    ∀ t, t ∈ evalLinearReach C W ρ rec self V ↔
         t ∈ reachDen C rec self W.den V.toEnv ρ
```

This **replaces** `program_eval_sound` as the rec agreement. The public theorem has no fuel hypothesis. The engine may still abort with `FixpointBudgetExceeded` before the lfp — engine-only incompleteness vs `reachDen`. Document that in the Bridge **engine** row, in `40-execution.md`, and in `fixpoint.rs`'s successor module. Withdraw the sentence in `40-execution.md` that "Lean `evalProgram` is complete only under sufficient fuel." Do not replace it with a sentence that Lean `evalLinearReach` is incomplete under insufficient fuel — there is no such parameter.

`evalLinearReach` is the **naive** chain. The engine is the **semi-naive** realization (`new = T(acc) \ acc` after round 0, one `DeltaVariant` per rec arm). Do not implement `evalLinearReach` as delta-variants. `semi_naive_agrees` instantiates at `T := reachOp C rec self I W ρ`. Keep the theorem; delete the k-variant reading from its Bridge engine row. One self-atom ⇒ one delta occurrence ⇒ `new = T(acc) \ acc = rec(acc) \ acc` after round 0 (`base ⊆ acc`).

## `evalQuery` — the denotation of a Query

Match `q.rec` only. Do not match `recId` beside it (the catch-all would drop reach).

```lean
def evalQuery (C : Classify) (q : Query) (I : Instance) (ρ : ParamEnv) :
    Set AnswerTuple :=
  let V := evalWith C q I ρ
  let V' :=
    match q.rec with
    | none => V
    | some rec =>
        let self : CteId := ⟨q.views.length⟩
        V.update self (reachDen C rec self I V ρ)
  rulesAnswers C q.rules (sourceDen I V') ρ
```

Executable. WITH fold in declaration order; optional reach; then main. Same `evalList`.

```lean
def evalWithTables.go (C : Classify) (W : ListInstance) (ρ : ParamEnv) :
    Nat → List WithDef → CteTables → CteTables
  | _, [], T => T
  | i, d :: ds, T =>
      evalWithTables.go C W ρ (i + 1) ds
        (T.update ⟨i⟩ (evalList C W T ρ d.rules))

def evalWithTables (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (defs : List WithDef) : CteTables :=
  evalWithTables.go C W ρ 0 defs CteTables.empty

def evalQueryList (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (q : Query) : List AnswerTuple :=
  let T₀ := evalWithTables C W ρ q.views
  let T :=
    match q.rec with
    | none => T₀
    | some rec =>
        let self : CteId := ⟨q.views.length⟩
        T₀.update self (evalLinearReach C W ρ rec self T₀)
  evalList C W T ρ q.rules
```

```lean
theorem evalQuery_sound {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {q : Query}
    (hsafe : ∀ r, r ∈ q.allRules → Safe r)
    (hwt : ∀ r, r ∈ q.allRules → r.WellTyped)
    (hlin : q.recLinear) :
    ∀ t, t ∈ evalQueryList C W ρ q ↔ t ∈ evalQuery C q W.den ρ
```

**Premises.** `hsafe` / `hwt` / `recLinear` — enough for `evalLinearReach_eq_lfp` on the rec branch and `eval_sound` everywhere. **Not** full `WellFormed`: phantom CTE reads agree (empty) without `sourcesInRange`, same as today's `program_eval_sound`. `recLinear` is `True` when `rec = none`.

Conformance `reach-*.json` runs `evalQueryList`. This **replaces** `checkProgramCase`'s use of `evalProgram`.

### The degenerate theorem (replaces `degenerate_embedding`)

```lean
theorem evalQuery_plain (C : Classify) (q : Query) (I : Instance)
    (ρ : ParamEnv) (hp : q.Plain) :
    ∀ t, t ∈ evalQuery C q I ρ ↔
         t ∈ rulesAnswers C q.rules (edbEnv I) ρ
```

Proof: `evalWith` of `[]` is `CteEnv.empty`; `rec = none` skips `reachDen`; `sourceDen I empty = edbEnv I`. No `toProgram`. No stratum-0 `lfpP_const`. A one-line unfolding. Hostile `Cte` atoms on a Plain query read empty (same phantom); acceptance is `UnknownCte`.

```lean
theorem evalQueryList_plain {C W ρ q} (hp : q.Plain) :
    evalQueryList C W ρ q = evalList C W CteTables.empty ρ q.rules
```

```lean
theorem evalQuery_empty_rules {C q I ρ} (hr : q.rules = []) :
    ∀ t, t ∉ evalQuery C q I ρ
```

The rec CTE is never the answer. Empty main denotes `∅` even when `reachDen` is huge. Identity main is required (`01-language.md`).

A WITH-only query (`rec = none`, `q.views ≠ []`) is **not** plain and **not** a reach. `evalQuery` is `rulesAnswers` of the main over `sourceDen I (evalWith ...)`. No `lfpS`. The engine must match that: WITH preamble, then the ordinary rule loop (`03-engine.md`). `PreparedBody` is `Rules` or `Empty`, never `Reach`.

A rec query is `evalWith`, then `reachDen`, then main `rulesAnswers`. Main may anti-join the rec CTE: `V'` already holds the finished lfp, so a negated `Cte self` in a **main** rule is an anti-join of a finished set — legal, monotone, not "negation through a cycle." Rec CTE itself still has `negated = []`.

## Theorem table

### Keep (names stay; file may move)

| Theorem | Note |
|---|---|
| `Matches`, `matches_def`, `repeated_var_unifies`, `param_selects_not_binds` | Source-blind |
| `Safe`, `safe_negated_bound`, `membership_only_unsafe`, `antijoin_over_active_domain` | On `Rule` |
| `Rule.WellTyped`, comparison shape | Unchanged. `PRule.BindingsMeasureFree` **is** the measure-in-binding conjunct of `WellTyped` — do not keep a second name |
| `dnf_preserves_denotation` | Retarget the `queryAnswers` mention to `rulesAnswers` on the lowered rule list |
| `eval_sound` | `evalList` ↔ `rulesAnswers` over `sourceDen W.den T.toEnv` (`CteTables.empty` recovers `edbEnv`) |
| `answers_finite_of_safe` | Same retarget |
| `membership_lowering_preserves`, `_negated` | `a.relation` → `.edb R` under `a.source = .edb R`. JSON `"relation"` stays on the CQuery arm |
| Aggregate laws in `Query/Aggregates.lean` | `bindingSet` / `Group` / `aggAnswers` take `F`. Recover `I` via `edbEnv I`. A main `Sum` over a finished CTE is `aggAnswers` at `sourceDen I V'` — PRD 05 never enters `reachOp`. Cookbook 25 is an **engine** instrument of `evalQuery_sound`, not an Aggregates.lean Bridge row |
| `semi_naive_agrees` | Move to Reach.lean; instantiate at `reachOp` |
| `fueledLoop_fixpoint`, `missingCount_le` | Private in Reach.lean; not public semantics |
| `succ_prefixed_infinite`, `odd_not_monotone` | Walls. Retarget odd's syntax off `Program` |
| Plan / Dedup / Rewrites / Sweep theorems | `queryAnswers C ⟨n, rs⟩ I ρ` → `rulesAnswers C rs (edbEnv I) ρ`. `Query.plain` where a Query is still required. `evalList` takes `CteTables.empty`. `a.relation` match `.edb` |
| `lfpP_fixed` | **Delete** with `lfpP`. Replaced by `lfpS_fixed` |

### Retarget (new name, old job)

| Today | After |
|---|---|
| `mem_queryAnswers` | `mem_rulesAnswers` |
| `queryAnswers` | `rulesAnswers` (union of a rule list over `F`); `evalQuery` (denotation of a `Query`) |
| `Query.toProgram` / `degenerate_embedding` | `Query.plain` / `evalQuery_plain` / `evalQueryList_plain` |
| `queryAnswers` of empty rules | `evalQuery_empty_rules` (empty main denotes `∅`; rec is never the answer) |
| `Query.toProgram_wellFormed` | `Query.plain_wellFormed` (**needs `edbOnly`**) |
| `Program.WellFormed` / `wellFormed_reads_real` | `Query.WellFormed` / `wellFormed_cte_reads_real` |
| `Program.StratifiedBy` / `stratumOp_mono` | `Query.recLinear` / `reachOp_mono` |
| `programDen` / `programAnswers` | `reachDen` (rec table) + `evalQuery` (answers) |
| `evalProgram` / `evalProgramAt` / `strataEval` | `evalLinearReach` (rec) + `evalQueryList` (whole Query) |
| `program_eval_sound` | `evalLinearReach_eq_lfp` + `evalQuery_sound` |
| `program_den_finite` | `reach_den_finite` |
| `PRule.Safe` / `PRule.BindingsMeasureFree` | `Safe` / `Rule.WellTyped` (already) |
| `sourceDen` on `PredSets` | `sourceDen` on `CteEnv` |
| `AtomSource.idb?` / `idb?_eq_some` | `AtomSource.cte?` / `cte?_eq_some` (and `edb?`) |
| `oddProgram` / `odd_not_stratified` | illegal `RecCte` / `¬ recLinear` |
| Bridge `Program::Empty` mechanism string | `PreparedBody::Empty` |

### Delete (no residue, no synonym)

`Program`, `PredicateDef`, `PredId`, `PAtom`, `PRule`, `Edge`, `EdgeKind`, `Program.rulesList`, `Program.Stratified`, `Atom.toPAtom`, `Rule.toPRule`, `Query.toProgram`, `stratumOp`, `stratumSets`, `stratumEnv`, `finished`, `programDen`, `programAnswers`, `PredSets`, `lfpP`, `MonoP`, `evalProgram`, `evalProgramAt`, `strataEval`, `stratumStep`, `stratumCands`, `StratumInv`, `degenerate_embedding`, `pderives`, `pruleAnswers`, `PMatches`, `PAtom.code`, `AtomSource.code`, `pderives_code`, `codeWorld`, `pevalRule`, `progEnv`, `tabAt`, `evalProgramAt_den`, `program_eval_sound`, `wellFormed_reads_real` (old name), `oddProgram` as a `Program` value (rebuild as an illegal `RecCte`), `Atom.relation` as a field or total accessor.

Do not leave `abbrev Program := Query`. Do not leave `evalProgram` as an alias of `evalQueryList`. Grep-clean the Lean tree.

## Bridge

Delete rows: `@Query.degenerate_embedding`, `@Query.wellFormed_reads_real`, `@Query.stratumOp_mono`, `@Query.program_den_finite`, both `@Query.program_eval_sound`.

Keep / retarget `semi_naive_agrees` (two rows today: naive oracle, engine delta). Engine row: one `DeltaVariant` per rec arm, `WordMap::iter_since`, `TransientImage`, **not** k-variants, **not** `run_fixpoint` over strata.

Add:

| Theorem | Premise (one sentence) | Mechanism | Instrument |
|---|---|---|---|
| `evalQuery_plain` | A query with empty WITH and no rec denotes the union of its main rules over the instance | `validate` / `prepare` on `Query` (no reach driver) | a plain query executes as today (`tests/api.rs` retarget of `a_degenerate_program_executes_as_its_query`) |
| `wellFormed_cte_reads_real` | Every CTE source an accepted query reads names a real WITH or the rec | `validate` CTE screen; `ValidationError::UnknownCte` | `rejects_a_negated_phantom_cte` (retarget of `rejects_a_negated_phantom_read`) |
| `reachOp_mono` | Linearity and no negation in the rec CTE make the reach operator monotone | rec roster (`NegationInRecCte`, `NonlinearRecArm`) | `rejects_negation_in_rec_cte`; `odd_not_monotone` |
| `reach_den_finite` | Rec heads project bound variables, so the lfp is a finite subset of the active domain | `MeasureInCte`, `AggregateInCte` on heads | `rejects_a_measure_in_a_rec_head` (`MeasureInCte`); `succ_prefixed_infinite` |
| `evalLinearReach_eq_lfp` | The executable reach lists exactly `reachDen` | conformance `evalQueryList` on `reach-*.json`; `translate_query` | rec corpus three-way |
| `evalLinearReach_eq_lfp` (engine) | The reach driver computes those answers when it terminates. `FixpointBudgetExceeded` is incompleteness vs `reachDen` — not vs a fueled Lean evaluator (there isn't one) | `run_reach` (`api/prepared/reach.rs`); `Error::FixpointBudgetExceeded` | recursive goldens; `a_tight_fixpoint_budget_trips_with_the_typed_error` |
| `evalQuery_sound` | WITH DAG once, optional `reachDen`, then main `rulesAnswers` — listed by `evalQueryList` | `run_with` + `run_reach` + rule loop | cookbook 24 goldens; cookbook 25 is main `Sum` over finished rec (engine, not an Aggregates theorem); WITH-only does not enter reach |
| `semi_naive_agrees` (engine) | One delta occurrence per rec arm walks the naive chain; the spanning seen-set absorbs re-derivation | `DeltaVariant` (exactly one per rec arm); `answers_since` | recursive goldens; delete k-variant tests |
| `evalQuery_empty_rules` | Empty main denotes `∅`; the rec CTE is never the answer | `EmptyRuleSet` at validate; identity main required | recut of `output = 0` programs |

`ledger_count` is whatever the list length is after the edit. Do not preserve 96.

## Conformance

**Two arms. Do not smash them.**

1. **CQuery arm (unchanged files).** `seeded-*.json` and other non-`judgment-*` / non-`reach-*` cases. `Conformance.decodeQuery` / aggregate glue / `evalList` + surface anti-join. Atoms keep JSON `"relation"`; after `Atom.source`, `decodeAtom` writes `.edb ⟨id⟩`. **Do not add `views`/`rec` keys. Do not recut 200 seeded files.** Missing keys are not a defaulting story — this arm never looks for them.

2. **Reach arm (this commit).** Recut `lean/conformance/cases/program-*.json` → `reach-*.json` in the **same Lean commit** as the Syntax deletion. No Program decoder. No `strata` / `output` / `predicates` / `idb` keys. `Main.lean`: `ReachCase` / `decodeReachQuery` / `checkReachCase` runs `evalQueryList`. Delete `ProgramCase`, `decodePAtom`, `decodePRule`, `decodePredicate`, `PCase`.

JSON shape for the reach arm (Lean field names — this is the Lean oracle's interchange, not the engine IR):

```json
"query": {
  "views": [],
  "rec": {
    "arity": 2,
    "base": [{ "finds": [0, 1], "atoms": [{ "edb": 7, "bindings": ... }],
               "negated": [], "conditions": [] }],
    "rec":  [{ "finds": [0, 2], "atoms": [
                 { "edb": 7, "bindings": ... },
                 { "cte": 0, "bindings": ... }
               ], "negated": [], "conditions": [] }]
  },
  "arity": 2,
  "rules": [{ "finds": [0, 1], "atoms": [{ "cte": 0, "bindings": ... }],
              "negated": [], "conditions": [] }]
}
```

Rules: Lean `finds : List VarId`, not engine `HeadTerm`. Atoms `edb` / `cte` (never `idb`, never `relation` on this arm). `rec: null` (or omit) for WITH-only. One-predicate rec programs (`output = 0`) become `views = []`, `rec = some`, **identity main** of the same arity — empty `rules` denotes `∅` (`evalQuery_empty_rules`). WITH-only cases have `rec: null` and nonempty `views`. Rec id is `views.length` (0 when `views` is empty, as in the example).

Filename: `reach-hand-closure.json`, `reach-seeded-0000.json`, … Dispatch in `Main.lean` / `conformance/README.md` by prefix `reach-`. Do **not** name them `query-*.json` — that is the CQuery glob's leftover bucket and would run the wrong decoder.

**Drop** `program-hand-mutual.json`. Mutual recursion is unwritable. Do not keep it as a Program-shaped trophy.

Until the Rust builder is recut (step 4 of `05-cutover.md`), the Lean corpus is the checked-in recut files — not a heuristic that still parses `predicates`/`output`. **Do not run `generate_program_corpus` between Lean green and the Rust rewrite** — it would overwrite `reach-*.json` with Program JSON.

`lean/conformance/README.md` in the same commit: `program-*.json` sentence dies.

## Countermodels

- `odd_not_monotone` / `odd_rounds_oscillate` / `odd_not_stratified`: rebuild on `reachOp` of a RecCte whose rec arm is `p ← ¬p` (`negated = [⟨.cte self, []⟩]`, `selfCount = 0`). Base may be empty — the countermodel is illegal (`¬ recLinear`, `EmptyRecursiveBase` would also fire). Show `¬ Query.recLinear` (on a Query wrapping that RecCte) and `¬ MonoS`. Delete `oddProgram : Query.Program`. Rename `odd_not_stratified` → keep the name as a wall or retarget the statement to `¬ recLinear`; do not keep a `Stratified` predicate.
- `succ_prefixed_infinite`: keep as operator-level wall. Cite from `reach_den_finite`, not `program_den_finite`.
- `unsafe_rule_infinite`: unchanged (`Safe`). Atom constructor `.edb`.

## Aggregates

PRD 05 stays a composition over the **main** query's binding set. Generalize:

```lean
def bindingSet (C : Classify) (r : Rule) (F : AtomSource → Set Fact)
    (ρ : ParamEnv) : Set Assignment :=
  fun σ => derives C r F ρ σ

def Group (C : Classify) (r : Rule) (F : AtomSource → Set Fact)
    (ρ : ParamEnv) (keys : List KeyTerm) (g : List (Option Value)) :
    Set Assignment :=
  fun σ => derives C r F ρ σ ∧ keyTuple keys σ = g

def aggAnswers (C : Classify) (r : Rule) (F : AtomSource → Set Fact)
    (ρ : ParamEnv) (keys : List KeyTerm)
    (fold : List (Option Value) → Set Assignment → AnswerTuple) :
    Set AnswerTuple :=
  fun t => ∃ σ, derives C r F ρ σ ∧
    t = fold (keyTuple keys σ) (Group C r F ρ keys (keyTuple keys σ))
```

Theorems that took `I : Instance` recover via `edbEnv I`. A `Sum` over `reach(a)` is a main-rule `aggAnswers` at `sourceDen I V'` after `evalQuery` has closed WITH/rec. Do not define `reachOp` over aggregate heads — unrepresentable (`finds : List VarId`) and refused in the engine. `AggregationThroughCycle` is not a theorem to retarget; the shape is unwritable.

Do not add a Bridge row that cites cookbook 25 as an Aggregates.lean theorem. Cookbook 25 is the engine instrument of `evalQuery_sound` (main fold, finished CTE).

## What Lean will not take on in this cut

`ArgKey::Measure`, C20, FFI UAF, Q-mark abort, chain-window (`w = w₁ ∩ w₂` in a rec head), the negated-closed complement fold (still unmodeled in Rewrites), Plan/COLT, CTE-aware rewrites, CTE membership in `Membership.lean`, `FixpointBudgetExceeded` as a Lean error (it is not an error in `reachDen`; it is engine incompleteness). Smashing CQuery into `Syntax.Query` (aggregate finds). `05-cutover.md` § not in the cut.

## First file to open

`lean/Bumbledb/Query/Syntax.lean`: delete from `/-! ## The program cut` through `Query.toProgram_wellFormed`. Insert `CteId` / `AtomSource` / widened `Atom` / `WithDef` / `RecCte` / widened `Query` (`views`, `rec`, `arity`, `rules`) / `WellFormed` / `Rule.edbOnly` / `Query.plain`. Then the constructor grep (`a.relation`, `⟨n, rs⟩`). `lake build` will redden Denotation, Membership, Fixpoint, Plan, Rewrites, Dedup, Sweep, Aggregates, Conformance, Countermodels, Main, Bridge — that is the queue, not a reason to keep Program for another day.

Work queue after Syntax elaborates:

1. `Denotation.lean` — `F`, `rulesAnswers`, `factsOf`, `eval_sound`.
2. `Membership.lean` / `Aggregates.lean` — `.edb` match / `F`.
3. `Exec/{Plan,Dedup,Rewrites,Sweep}.lean` — `rulesAnswers`, `Query.plain`, `.edb`.
4. Add `Exec/Reach.lean`. Delete `Exec/Fixpoint.lean`.
5. Countermodels, Bridge, `Bumbledb.lean`, `Main.lean`, `reach-*.json`, `conformance/README.md`, `lean/README.md`.
6. `lake build` / `scripts/lean.sh` / census (docs next, same PR).
