import Bumbledb.Dependencies

/-!
# Query syntax — the pure-data IR (Level 0, PRD 04)

A faithful abstraction of `crates/bumbledb/src/ir.rs` (the IR, not the
notation): terms, atoms with named-field bindings (absence of a field
IS the wildcard), the input condition grammar (leaf / and / or — the
one place a nested OR is writable), rules (atoms, negated, conditions,
finds), named interiors, at most one linear rec SCC, and the main
query. Syntax only — meaning lives in `Bumbledb.Query.Denotation` and
`Bumbledb.Exec.Reach`.

## Narrowings recorded (law 5: narrow and record)

* **Finds are projected variables.** `Rule.finds : List VarId`;
  aggregate and measure find positions are PRD 05's folds over the
  binding sets PRD 04 denotes, so the head degenerates to its arity
  (`Query.arity` — every `HeadTerm` is `Var` at this level; the
  var-free head-shape row arrives with the aggregate ops). Interior
  and rec heads are projection-shaped BY CONSTRUCTION here; the
  engine roster that refuses `Aggregate` / `Measure` finds on those
  heads discharges a class Lean cannot write.
* **The Allen mask is the admitted relation LIST.** `AllenMask` is a
  `List AllenRel` read as a set (membership); the engine's bitmask
  (`crate::allen::AllenMask`) is its encoding. The vacuity rules
  (∅ and full rejected) are validator shape checks, unspent here.
* **`WellTyped` keeps the SHAPE discipline only** — the measure and
  param-set placement rules (`DurationInBinding`,
  `DurationComparisonOperator`, `DurationBothSides`,
  `ParamSetComparison`). The positional TYPE rules (slot anchoring,
  bivalent resolution, the order-operand screen) are validator
  mechanism (`ir/validate/context.rs`); the denotation stays total
  without a typing premise — an ill-placed term selects nothing, and
  the validator's typed pass makes every ill-typed comparison
  unreachable on accepted rules (the honest reading is PRD 04's
  module doc: `ne` denotes plain disequality, not the empty arm).
* **The membership BINDING reading is not a syntax node.** "Membership
  is a typing rule, not a node" (`ir.rs::Atom::bindings`): the matching
  equation's atom bindings select VALUES (PRD 04's decided shape), and
  point membership reaches this level as the `PointIn` comparison.
  This is no longer narrative: `Bumbledb.Query.SurfaceMatches`
  (`Query/Membership.lean`) judges the written bivalent binding on the
  types `ir/validate/context.rs::resolve_bivalents` resolves, and
  `membership_lowering_preserves` PROVES the lowering to this level's
  form answer-preserving — the arbiter the engine's `ir/normalize/
  normalize.rs::lower_atom` and the naive model are measured against.
  Membership stays EDB: theorems match `a.source = .edb R`. Interior
  membership is engine-only.
* Boundary caps (`MAX_RULES`, `MAX_CONDITION_DEPTH`) are hostile-input
  mechanism, not semantics — unmodeled. **There is no interior-count
  cap to model.** `InteriorId` is `Nat` here; the engine's `u32` width
  is representation, not denotation.
* **Acceptance is strictly narrower than `Safe ∧ WellTyped` — the
  roster.** The engine rejects queries this model denotes exactly:
  the empty-edge refusals (`EmptyRuleSet` for empty **main**,
  `EmptyInterior`, `EmptyFinds`, `NoPositiveAtoms`, the all-vanished
  `Or([])` query, `DuplicateFindTerm`), the write-the-query-you-mean
  refusals (`SelfComparison`, `ConstantComparison`), the Allen ∅/full
  vacuity rejections, the rec roster (`EmptyRecursiveBase`,
  `EmptyRecursiveStep`, `SelfInBase`, `RecArmMissingSelf`,
  `NonlinearRecArm`, `NegationInRec`), and the caps above. Benign in
  every case — never unsound: each theorem quantifies over arbitrary
  syntax or assumes only `Safe`/`WellTyped`/`recLinear`, so a
  rejected-but-denotable query simply never reaches execution.
* **The unknown-interior gap, recorded LOUDLY, with its screen.** A
  rule reading `interior k` with `k` outside `derivedCount` reads the
  EMPTY fact set: a positive phantom read kills its rule, but a
  NEGATED phantom read is vacuously satisfied. The screen is
  `Query.WellFormed` (`sourcesInRange`); the engine's refusal is
  `ValidationError::UnknownInterior`. `Query.plain` does **not**
  rewrite atoms to `.edb` — hostile `Interior` atoms on a plain query
  fail `sourcesInRange`. The `Exec/Reach.lean` agreement theorems are
  exact equalities with or without the screen — both the denotation
  and the evaluator read a phantom as empty — so the premise belongs
  to acceptance readings, not to the agreement.

## The creation-quarantine gravestones (law text; the full record is
`Txn/Fresh.lean`'s module doc)

`fresh` never appears in a rule head, and no arithmetic appears in a
rule head — both UNREPRESENTABLE in this IR, permanently: `Term` has
no mint constructor (the mint is the write path's, Level 2 —
`Txn/Fresh.lean`), heads are projected variables, and the measure is
the one arithmetic, its positions boundary-only (`Rule.WellTyped`).
Interior and rec heads are `List VarId` too, so recursion's safety
roster (`MeasureInInterior` and kin) is this same creation-quarantine
law restated for the reach operator, not a new rule
(`docs/architecture/20-query-ir.md` § the creation quarantine).

## Interiors and one linear rec (this cut's IR)

`InteriorId` / `AtomSource` / `Interior` / `Rec` / widened `Query`
(`interiors`, `rec`, `arity`, `rules`) are the IR. A query with empty
interiors and no rec is today's query plus two empty fields
(`Query.plain` / `evalQuery_plain`). Linearity and no-negation-in-rec
are `Query.recLinear` — not a Tarjan witness. Recorded shapes:

* **`InteriorId` never puns with `RelId`.** Statements quantify over
  stored relations permanently (`30-dependencies.md`, the
  stored-relations decision): no statement form carries an
  `InteriorId` position, so a statement about a derived table is
  UNWRITABLE, not rejected. Do not define `Atom.relation : RelId` as
  a total accessor.
* **Fold-input edges are unrepresentable at this level.** `Rule`
  heads are projected variables (`finds : List VarId`), so interiors
  and the rec are projection-shaped BY CONSTRUCTION. Aggregation is
  the `Query/Aggregates.lean` composition over **main**, which reads
  a finished environment — strictly after interiors/rec by
  construction of `evalQuery`.
* **One atom type.** `Atom.source : AtomSource` (`edb | interior`).
  An `interior` atom's bindings address HEAD POSITIONS positionally —
  `FieldId i` is the target derived table's column `i`. Numbering:
  interior `i` has `InteriorId ⟨i⟩`; the rec, if present, has
  `InteriorId ⟨interiors.length⟩`.
* **`Rec` / `Query` are products, not structures.** A structure field
  named `rec` collides with Lean's recursor (`T.rec`). Accessors
  `Rec.rec` / `Query.rec` keep the spec names; constructors are
  `Rec.mk` / `Query.mk`. Recorded so the IR names stay locked.
-/

namespace Bumbledb.Query

/-! ## Identities -/

/-- Dense query-variable id — **rule-scoped**: the same `VarId` in two
rules names two unrelated variables (`crate::ir::VarId`). -/
structure VarId where
  id : Nat
deriving DecidableEq

/-- Dense parameter id; values are supplied positionally at execution.
Params are query-global (`crate::ir::ParamId`). -/
structure ParamId where
  id : Nat
deriving DecidableEq

/-- Dense derived-table id — the index into a query's interior list,
or the rec's id `⟨interiors.length⟩` when rec is present. A SEPARATE
identity from `RelId`, deliberately: statements quantify over stored
relations permanently, and no statement form carries an `InteriorId`
position — a statement about a derived table is unwritable
(module doc). -/
structure InteriorId where
  id : Nat
deriving DecidableEq

/-! ## The Allen mask position -/

/-- The thirteen Allen interval relations — the classification's
codomain. Abstract at this level: PRD 05 refines `classify`; here the
mask position only needs the relations as a decidable-equality sum. -/
inductive AllenRel where
  | before | meets | overlaps | finishedBy | contains | starts
  | equals | startedBy | during | finishes | overlappedBy | metBy
  | after
deriving DecidableEq

/-- An Allen mask: the admitted relation list, read as a set — the
engine's bitmask (`crate::allen::AllenMask`) is its encoding. -/
abbrev AllenMask : Type := List AllenRel

/-! ## Terms and atoms -/

/-- One term of an atom binding or comparison (`crate::ir::Term`).
`paramSet` is a param id used as a SET — the term denotes any element.
`measure` is the one arithmetic the point-set denotation defines:
`|[s, e)| = e − s`, legal only on one side of an order comparison
(the shape discipline `WellTyped` keeps). -/
inductive Term where
  | var (v : VarId)
  | param (p : ParamId)
  | paramSet (p : ParamId)
  | lit (value : Value)
  | measure (v : VarId)

/-- Where an atom draws its facts: a stored (EDB) relation or a
derived table (a named interior, or the rec). -/
inductive AtomSource where
  | edb (R : RelId)
  | interior (C : InteriorId)
deriving DecidableEq

/-- The derived table an atom source reads, if any. -/
def AtomSource.interior? : AtomSource → Option InteriorId
  | .interior C => some C
  | .edb _ => none

/-- The stored relation an atom source reads, if any. -/
def AtomSource.edb? : AtomSource → Option RelId
  | .edb R => some R
  | .interior _ => none

/-- `interior?` reads back the source. -/
theorem AtomSource.interior?_eq_some {s : AtomSource} {C : InteriorId} :
    s.interior? = some C ↔ s = .interior C := by
  cases s with
  | edb R => simp [AtomSource.interior?]
  | interior D => simp [AtomSource.interior?]

/-- `edb?` reads back the source. -/
theorem AtomSource.edb?_eq_some {s : AtomSource} {R : RelId} :
    s.edb? = some R ↔ s = .edb R := by
  cases s with
  | edb S => simp [AtomSource.edb?]
  | interior C => simp [AtomSource.edb?]

/-- One atom: a source with named-field bindings. Absence of a field
IS the wildcard — "wildcard bound to something" is unwritable. An atom
with zero bindings is legal and means a nonemptiness gate on the
source (`crate::ir::Atom`). An `interior` atom's `FieldId i` addresses
that derived head position `i` — positional, never nominal. -/
structure Atom where
  source : AtomSource
  bindings : List (FieldId × Term)

/-! ## Comparisons and the input condition grammar -/

/-- Comparison operators (`crate::ir::CmpOp`): `eq`/`ne` for all six
types, order operators for the two orderable scalars, `allen` as THE
interval-pair comparison (interval `Eq`/`Ne` are its derived facts —
normalization canonicalizes them to `EQUALS`/`¬EQUALS`), and `pointIn`
as point membership in predicate form (interval left, point right).
The `allen` mask is a literal — bind-time mask params are unrepresentable. -/
inductive CmpOp where
  | eq | ne | lt | le | gt | ge
  | allen (mask : AllenMask)
  | pointIn

/-- One comparison condition (`crate::ir::Comparison`). `eq` between
two variables is unification and obeys identical type rules. -/
structure Comparison where
  op : CmpOp
  lhs : Term
  rhs : Term

/-- The input condition grammar (`crate::ir::ConditionTree`): any
boolean combination of positive comparisons — the one place the
surface admits a nested OR, and the engine never sees it (validation
distributes to DNF; `Bumbledb.Query.dnf_preserves_denotation` is the
contract). The empty combinations keep their algebraic readings:
`and []` is true, `or []` is false (the rule denotes nothing and
lowers to zero disjuncts). -/
inductive Condition where
  | leaf (c : Comparison)
  | and (children : List Condition)
  | or (children : List Condition)

/-! ## Rules, interiors, rec, queries -/

/-- One rule: a conjunctive body projecting its finds. A rule is its
OWN variable scope — `VarId`s never cross rules; params, by contrast,
are query-global (`crate::ir::Rule`). `negated` are anti-join atoms:
a binding satisfies one iff NO fact of its source matches — plain
anti-join over sets, no null trick, no three-valued logic; negation is
a POSITION in the rule, not a kind of atom, so the list reuses `Atom`
unchanged. `conditions` are conjoined. `finds : List VarId` — the
recorded narrowing: aggregate/measure finds are PRD 05's. -/
structure Rule where
  finds : List VarId
  atoms : List Atom
  negated : List Atom
  conditions : List Condition

/-- A named interior: a finite CQ (union of CQs), evaluated once.
Declaration order is topological order. -/
structure Interior where
  arity : Nat
  rules : List Rule

/-- One recursive SCC (this cut: one name, linear arms).
Product encoding: a structure field named `rec` collides with the
recursor. `Rec.rec` is the spec accessor for the rec arms. -/
def Rec : Type := Nat × List Rule × List Rule

def Rec.arity (r : Rec) : Nat := r.1
def Rec.base (r : Rec) : List Rule := r.2.1
def Rec.rec (r : Rec) : List Rule := r.2.2
def Rec.mk (arity : Nat) (base rec : List Rule) : Rec := ⟨arity, base, rec⟩

/-- A query: named interiors (a DAG, eval once), at most one linear
rec SCC, then the main query. **Denotation of a Query is `evalQuery`**
(`Bumbledb.Exec.Reach`); the union of a rule list is `rulesAnswers`.
Set semantics means there is exactly one union per rule-list — no bag
distinction exists or is representable. The main head is its arity at
this level (every head position is a projected variable — recorded
narrowing; PRD 05 restores the shape row). Product encoding so
`Query.rec` can be the spec accessor (not a recursor). -/
def Query : Type := List Interior × Option Rec × Nat × List Rule

def Query.interiors (q : Query) : List Interior := q.1
def Query.rec (q : Query) : Option Rec := q.2.1
def Query.arity (q : Query) : Nat := q.2.2.1
def Query.rules (q : Query) : List Rule := q.2.2.2
def Query.mk (interiors : List Interior) (rec : Option Rec)
    (arity : Nat) (rules : List Rule) : Query :=
  ⟨interiors, rec, arity, rules⟩

/-- Today's query: empty interiors, no rec. -/
def Query.plain (arity : Nat) (rules : List Rule) : Query :=
  Query.mk [] none arity rules

/-- A query with empty interiors and no rec. -/
def Query.Plain (q : Query) : Prop :=
  q.interiors = [] ∧ q.rec = none

/-- Every rule of every interior, the rec, and main — the
quantification surface the theorems range over. -/
def Query.allRules (q : Query) : List Rule :=
  q.interiors.flatMap (·.rules) ++
    (match q.rec with
     | none => []
     | some r => r.base ++ r.rec) ++
    q.rules

/-- The rec's derived-table id, when rec is present:
`⟨interiors.length⟩`. -/
def Query.recId (q : Query) : Option InteriorId :=
  match q.rec with
  | none => none
  | some _ => some ⟨q.interiors.length⟩

/-- How many derived tables the query names (interiors plus rec). -/
def Query.derivedCount (q : Query) : Nat :=
  q.interiors.length + (if q.rec.isSome then 1 else 0)

/-- Every atom of the rule — positive or negated — reads a stored
relation. Hostile `Interior` atoms on a `Query.plain` fail
`sourcesInRange`; this is the acceptance screen `plain_wellFormed`
spends. -/
def Rule.edbOnly (r : Rule) : Prop :=
  ∀ a, (a ∈ r.atoms ∨ a ∈ r.negated) → ∃ R, a.source = .edb R

/-! ## Variable occurrence — the raw material of `Safe` -/

/-- The variables a term mentions. A measure term mentions its
interval variable: the measure is a COMPUTATION over a bound variable,
never a binder itself. -/
def Term.vars : Term → List VarId
  | .var v => [v]
  | .measure v => [v]
  | .param _ | .paramSet _ | .lit _ => []

/-- The variables an atom's bindings mention. -/
def Atom.vars (a : Atom) : List VarId :=
  a.bindings.flatMap fun b => b.2.vars

/-- The variables a term BINDS at a positive binding position: a
`var` term and nothing else — a measure occurrence mentions its
variable but never binds it (the measure is a computation; Rust's
`atom_vars` records `Term::Var` alone). -/
def Term.bindingVars : Term → List VarId
  | .var v => [v]
  | _ => []

/-- The variables an atom BINDS. -/
def Atom.boundVars (a : Atom) : List VarId :=
  a.bindings.flatMap fun b => b.2.bindingVars

/-- The variables a comparison mentions. -/
def Comparison.vars (c : Comparison) : List VarId :=
  c.lhs.vars ++ c.rhs.vars

mutual
  /-- The variables a condition tree mentions. -/
  def Condition.vars : Condition → List VarId
    | .leaf c => c.vars
    | .and cs => Condition.varsList cs
    | .or cs => Condition.varsList cs

  /-- The variables a condition list mentions. -/
  def Condition.varsList : List Condition → List VarId
    | [] => []
    | t :: ts => t.vars ++ Condition.varsList ts
end

/-- The variables bound by the rule's POSITIVE atoms — the one binding
site the language has: positive atoms bind, everything else selects or
rejects (Rust's `atom_vars`, positive-only by construction:
`ir/validate/context.rs::check_atoms` inserts into `negated_vars` for
negated occurrences). -/
def Rule.positiveVars (r : Rule) : List VarId :=
  r.atoms.flatMap Atom.boundVars

/-- Every variable the rule mentions anywhere: finds, positive atoms,
negated atoms, conditions. -/
def Rule.allVars (r : Rule) : List VarId :=
  r.finds ++ r.atoms.flatMap Atom.vars ++ r.negated.flatMap Atom.vars
    ++ r.conditions.flatMap Condition.vars

/-! ## The shape discipline — `WellTyped` -/

/-- Whether a term is a measure — the placement rules single it out. -/
def Term.isMeasure : Term → Prop
  | .measure _ => True
  | _ => False

/-- Whether a term is a param set — legal in bindings and under `eq`
alone. -/
def Term.isSet : Term → Prop
  | .paramSet _ => True
  | _ => False

/-- Whether the operator is an order comparison — the only home the
measure has. -/
def CmpOp.isOrder : CmpOp → Prop
  | .lt | .le | .gt | .ge => True
  | _ => False

/-- One comparison's shape legality (the validator's shape pass,
`ir/validate/context.rs::comparison_shape`): a measure side only under
an order operator and never on both sides (`DurationComparisonOperator`,
`DurationBothSides`); a set side only under `eq` and never on both
sides (`ParamSetComparison`, `ConstantComparison`). -/
def Comparison.wellShaped (c : Comparison) : Prop :=
  ((c.lhs.isMeasure ∨ c.rhs.isMeasure) →
    c.op.isOrder ∧ ¬(c.lhs.isMeasure ∧ c.rhs.isMeasure)) ∧
  ((c.lhs.isSet ∨ c.rhs.isSet) →
    c.op = .eq ∧ ¬(c.lhs.isSet ∧ c.rhs.isSet))

mutual
  /-- Every leaf of a condition tree is well-shaped. -/
  def Condition.wellShaped : Condition → Prop
    | .leaf c => c.wellShaped
    | .and cs => Condition.wellShapedList cs
    | .or cs => Condition.wellShapedList cs

  /-- Every leaf of a condition list is well-shaped. -/
  def Condition.wellShapedList : List Condition → Prop
    | [] => True
    | t :: ts => t.wellShaped ∧ Condition.wellShapedList ts
end

/-- `WellTyped` — the validator's spec, kept minimal (only what the
theorems and the denotation's degenerate arms spend): no measure in
any atom binding (`DurationInBinding` — the measure is a computation,
not a bindable value), and every comparison well-shaped. The
positional TYPE rules are validator mechanism the denotation never
needs: it is total on ill-typed pairs, and the validator makes those
pairs unreachable on accepted rules — the recorded narrowing in the
module doc (and PRD 04's honest `ne` note).
Bridge: `ir/validate/context.rs::check_atoms` / `comparison_shape`. -/
def Rule.WellTyped (r : Rule) : Prop :=
  (∀ a, (a ∈ r.atoms ∨ a ∈ r.negated) →
    ∀ b, b ∈ a.bindings → ¬ b.2.isMeasure) ∧
  (∀ t, t ∈ r.conditions → t.wellShaped)

/-! ## Well-formedness — one recursive SCC, no Tarjan -/

/-- Interior sources a rule reads (both polarities). -/
def Rule.interiorReads (r : Rule) : List InteriorId :=
  (r.atoms ++ r.negated).filterMap fun a => a.source.interior?

/-- Interior sources a rule reads positively. -/
def Rule.positiveInteriorReads (r : Rule) : List InteriorId :=
  r.atoms.filterMap fun a => a.source.interior?

/-- How many positive atoms name `self`. -/
def Rule.selfCount (r : Rule) (self : InteriorId) : Nat :=
  (r.atoms.filter fun a => decide (a.source = .interior self)).length

/-- Whether a negated atom names `self`. -/
def Rule.hasNegatedSelf (r : Rule) (self : InteriorId) : Prop :=
  ∃ a, a ∈ r.negated ∧ a.source = .interior self

/-- Every interior source names a real named interior or the rec. -/
def Query.sourcesInRange (q : Query) : Prop :=
  ∀ r, r ∈ q.allRules → ∀ a, (a ∈ r.atoms ∨ a ∈ r.negated) →
    ∀ C, a.source = .interior C → C.id < q.derivedCount

/-- Interior `i` reads only strictly earlier interiors. The rec id is
never `< i`. -/
def Query.interiorsDag (q : Query) : Prop :=
  ∀ (i : Nat) (d : Interior), q.interiors[i]? = some d → ∀ r, r ∈ d.rules →
    ∀ (C : InteriorId), C ∈ r.interiorReads → C.id < i

/-- Match `q.rec` only. `self` is `⟨q.interiors.length⟩` — do not match
`recId` beside it (the catch-all is not unreachable to the elaborator).
Bans **all** negation in the rec SCC (`negated = []`), not only
self-negation — matches `NegationInRec`. Empty `base` / empty `rec`
fail `recLinear`. -/
def Query.recLinear (q : Query) : Prop :=
  match q.rec with
  | none => True
  | some rec =>
      let self : InteriorId := ⟨q.interiors.length⟩
      rec.base ≠ [] ∧ rec.rec ≠ [] ∧
      (∀ r, r ∈ rec.base → r.selfCount self = 0 ∧ ¬ r.hasNegatedSelf self) ∧
      (∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ ¬ r.hasNegatedSelf self) ∧
      (∀ r, r ∈ rec.base ++ rec.rec → r.negated = [])

def Query.WellFormed (q : Query) : Prop :=
  q.sourcesInRange ∧ q.interiorsDag ∧ q.recLinear

/-- A plain query of all-EDB rules is well-formed: no interior
sources, empty interiors, no rec. Hostile `Interior` atoms fail
`sourcesInRange` (`derivedCount = 0`). -/
theorem Query.plain_wellFormed (arity : Nat) (rules : List Rule)
    (hedb : ∀ r, r ∈ rules → r.edbOnly) :
    (Query.plain arity rules).WellFormed := by
  refine ⟨?src, ?dag, trivial⟩
  · intro r hr a ha C hsrc
    have hr' : r ∈ rules := by
      simpa [Query.plain, Query.mk, Query.allRules, Query.interiors,
        Query.rec, Query.rules] using hr
    obtain ⟨R, hR⟩ := hedb r hr' a ha
    exact nomatch (hR.symm.trans hsrc)
  · intro i d hi
    simp [Query.plain, Query.mk, Query.interiors] at hi

end Bumbledb.Query
