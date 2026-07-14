import Bumbledb.Dependencies

/-!
# Query syntax — the pure-data IR (Level 0, PRD 04)

A faithful abstraction of `crates/bumbledb/src/ir.rs` (the IR, not the
notation): terms, atoms with named-field bindings (absence of a field
IS the wildcard), the input condition grammar (leaf / and / or — the
one place a nested OR is writable), rules (atoms, negated, conditions,
finds), and the query as a program of rules. Syntax only — meaning
lives in `Bumbledb.Query.Denotation`.

## Narrowings recorded (law 5: narrow and record)

* **Finds are projected variables.** `Rule.finds : List VarId`;
  aggregate and measure find positions are PRD 05's folds over the
  binding sets PRD 04 denotes, so the head degenerates to its arity
  (`Query.arity` — every `HeadTerm` is `Var` at this level; the
  var-free head-shape row arrives with the aggregate ops).
* **The Allen mask is the admitted relation LIST.** `AllenMask` is a
  `List AllenRel` read as a set (membership); the engine's bitmask
  (`crate::allen::AllenMask`) is its encoding. The vacuity rules
  (∅ and full rejected) are validator shape checks, unspent here.
* **`WellTyped` keeps the SHAPE discipline only** — the measure and
  param-set placement rules (`DurationInBinding`,
  `DurationComparisonOperator`, `DurationBothSides`,
  `ParamSetComparison`). The positional TYPE rules (slot anchoring,
  bivalent resolution, the order-operand screen) are validator
  mechanism (`ir/validate/context.rs`); their semantic content is
  carried by the denotation's degenerate arms — an ill-typed
  comparison denotes `False`, an ill-placed term selects nothing —
  so the model is total and exact without a typing premise.
* **The membership BINDING reading is not a syntax node.** "Membership
  is a typing rule, not a node" (`ir.rs::Atom::bindings`): the matching
  equation's atom bindings select VALUES (PRD 04's decided shape), and
  point membership reaches this level as the `PointIn` comparison —
  exactly the predicate form the validator's typing rule licenses. The
  bivalent-position resolution that turns a surface membership binding
  into an element-typed check is `ir/validate/context.rs::
  resolve_bivalents` + `ir/normalize/lower_literal.rs` mechanism.
* Boundary caps (`MAX_RULES`, `MAX_CONDITION_DEPTH`) are hostile-input
  mechanism, not semantics — unmodeled.
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

/-- The `Allen` comparison's mask position: a literal mask, or a param
resolved at bind — a two-variant sum, not a `Term`: a variable or set
mask is UNREPRESENTABLE, not rejected (`crate::ir::MaskTerm`). -/
inductive MaskTerm where
  | lit (mask : AllenMask)
  | param (p : ParamId)

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

/-- One atom: a relation with named-field bindings. Absence of a field
IS the wildcard — "wildcard bound to something" is unwritable. An atom
with zero bindings is legal and means a nonemptiness gate on the
relation (`crate::ir::Atom`). -/
structure Atom where
  relation : RelId
  bindings : List (FieldId × Term)

/-! ## Comparisons and the input condition grammar -/

/-- Comparison operators (`crate::ir::CmpOp`): `eq`/`ne` for all six
types, order operators for the two orderable scalars, `allen` as THE
interval-pair comparison (interval `Eq`/`Ne` are its derived facts —
normalization canonicalizes them to `EQUALS`/`¬EQUALS`), and `pointIn`
as point membership in predicate form (interval left, point right). -/
inductive CmpOp where
  | eq | ne | lt | le | gt | ge
  | allen (mask : MaskTerm)
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

/-! ## Rules and queries -/

/-- One rule: a conjunctive body projecting its finds. A rule is its
OWN variable scope — `VarId`s never cross rules; params, by contrast,
are query-global (`crate::ir::Rule`). `negated` are anti-join atoms:
a binding satisfies one iff NO fact of its relation matches — plain
anti-join over sets, no null trick, no three-valued logic; negation is
a POSITION in the rule, not a kind of atom, so the list reuses `Atom`
unchanged. `conditions` are conjoined. `finds : List VarId` — the
recorded narrowing: aggregate/measure finds are PRD 05's. -/
structure Rule where
  finds : List VarId
  atoms : List Atom
  negated : List Atom
  conditions : List Condition

/-- A query: a non-recursive Datalog program — one head, rules.
**Denotation: the set union of the rules' denotations**
(`Bumbledb.Query.queryAnswers`); set semantics means there is exactly
one union — no bag distinction exists or is representable. The head
is its arity at this level (every head position is a projected
variable — recorded narrowing; PRD 05 restores the shape row). -/
structure Query where
  arity : Nat
  rules : List Rule

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
positional TYPE rules are validator mechanism whose semantic content
the denotation carries totally (ill-typed comparisons denote `False`)
— the recorded narrowing in the module doc.
Bridge: `ir/validate/context.rs::check_atoms` / `comparison_shape`. -/
def Rule.WellTyped (r : Rule) : Prop :=
  (∀ a, (a ∈ r.atoms ∨ a ∈ r.negated) →
    ∀ b, b ∈ a.bindings → ¬ b.2.isMeasure) ∧
  (∀ t, t ∈ r.conditions → t.wellShaped)

end Bumbledb.Query
