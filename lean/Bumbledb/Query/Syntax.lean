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
* Boundary caps (`MAX_RULES`, `MAX_CONDITION_DEPTH`) are hostile-input
  mechanism, not semantics — unmodeled.
* **Acceptance is strictly narrower than `Safe ∧ WellTyped` — the
  roster.** The engine rejects programs this model denotes exactly:
  the empty-edge refusals (`EmptyRuleSet`, `EmptyFinds`,
  `NoPositiveAtoms`, the all-vanished `Or([])` program,
  `DuplicateFindTerm`), the write-the-query-you-mean refusals
  (`SelfComparison`, `ConstantComparison`), the Allen ∅/full vacuity
  rejections, and the caps above. Benign in every case — never
  unsound: each theorem quantifies over arbitrary syntax or assumes
  only `Safe`/`WellTyped`, so a rejected-but-denotable program simply
  never reaches execution.
* **The unknown-PredId gap, recorded LOUDLY, with its screen.** A rule
  reading `idb k` with `k` outside `predicates` reads the EMPTY fact
  set: a positive phantom read kills its rule, but a NEGATED phantom
  read is vacuously satisfied — and `Program.StratifiedBy` never
  refuses the shape (a stratum witness may map the phantom low). The
  screen is `Program.WellFormed` (every `idb` source's `PredId` in
  range, positive and negated); the refusal itself is validation's
  roster item (`docs/reference/recursion-design.md` §1, the
  unknown-`PredId` row — queued with the engine discharge). The
  degenerate embedding carries the screen vacuously
  (`Query.toProgram_wellFormed`). The `Exec/Fixpoint.lean` agreement
  theorems are exact equalities with or without the screen — both the
  denotation and the evaluator read a phantom as empty (the record
  there, with `wellFormed_reads_real` as the spent form) — so the
  premise belongs to acceptance readings, not to the agreement.

## The creation-quarantine gravestones (law text; the full record is
`Txn/Fresh.lean`'s module doc)

`fresh` never appears in a rule head, and no arithmetic appears in a
rule head — both UNREPRESENTABLE in this IR, permanently: `Term` has
no mint constructor (the mint is the write path's, Level 2 —
`Txn/Fresh.lean`), heads are projected variables, and the measure is
the one arithmetic, its positions boundary-only (`Rule.WellTyped`).
The program cut below inherits both gravestones verbatim (`PRule`
heads are `List VarId` too), so recursion's safety roster
(`MeasureInRecursiveHead` and kin) is this same creation-quarantine
law restated for fixpoint topology, not a new rule
(`docs/architecture/20-query-ir.md` § the creation quarantine).

## The program cut (recursion-design §1, landed)

`Program`/`PredicateDef`/`AtomSource`/`PAtom`/`PRule` are the IR cut
of `docs/reference/recursion-design.md` §1: a query is a non-recursive
Datalog program one step short of the fixpoint, and the cut takes that
step and nothing else. The degenerate form is today's `Query` — a
one-predicate program with no `idb` atom — and the embedding is a
THEOREM, not a convention (`Exec/Fixpoint.lean:
degenerate_embedding`; `Query::single` is the Rust precedent).
Stratification lives here as a predicate over the SYNTAX (validation
models it): the dependency graph is data (`PRule.edges`, labeled
positive/negated), and `Program.Stratified` is the existence of a
stratum witness with positive edges non-decreasing and negated edges
strictly decreasing. Recorded shapes:

* **`PredId` never puns with `RelId`.** Statements quantify over
  stored relations permanently (`30-dependencies.md`, the
  stored-relations decision): no statement form carries a `PredId`
  position, so a statement about a predicate is UNWRITABLE, not
  rejected.
* **Fold-input edges are unrepresentable at this level.** `PRule`
  heads are projected variables (`finds : List VarId`), so program
  predicates are projection-shaped BY CONSTRUCTION (the design's §5);
  aggregation is the `Query/Aggregates.lean` composition over a
  program's OUTPUT, which reads a finished fixpoint — strictly lower
  by construction. The edge label sum therefore carries
  positive/negated only; a fold-input edge has no writable syntax.
* **`Query` and `Atom` stay as they are.** The engine's IR today is
  `Atom.relation : RelationId`; the modeled `Atom` matches it, and the
  program shapes model the design's post-trigger cut. When the Rust
  cut lands (`Atom.relation → Atom.source`), the two atom shapes merge
  here in the same change (the gate law).
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
positional TYPE rules are validator mechanism the denotation never
needs: it is total on ill-typed pairs, and the validator makes those
pairs unreachable on accepted rules — the recorded narrowing in the
module doc (and PRD 04's honest `ne` note).
Bridge: `ir/validate/context.rs::check_atoms` / `comparison_shape`. -/
def Rule.WellTyped (r : Rule) : Prop :=
  (∀ a, (a ∈ r.atoms ∨ a ∈ r.negated) →
    ∀ b, b ∈ a.bindings → ¬ b.2.isMeasure) ∧
  (∀ t, t ∈ r.conditions → t.wellShaped)

/-! ## The program cut — recursion's IR (recursion-design §1) -/

/-- Dense predicate id — the index into a program's predicate list
(the design's `PredId`; `Program.predicates` is the address space).
A SEPARATE identity from `RelId`, deliberately: statements quantify
over stored relations permanently, and no statement form carries a
`PredId` position — a statement about a predicate is unwritable
(module doc). -/
structure PredId where
  id : Nat
deriving DecidableEq

/-- Where an atom draws its facts: a stored (EDB) relation or a
program predicate (IDB) — the design's one-line sum, landing
INHABITED (the one-inhabitant refusal is why it waited for the
fixpoint semantics beside it). -/
inductive AtomSource where
  | edb (R : RelId)
  | idb (P : PredId)
deriving DecidableEq

/-- The predicate an atom source reads, if any — the dependency
graph's projection. -/
def AtomSource.idb? : AtomSource → Option PredId
  | .idb P => some P
  | .edb _ => none

/-- A program-level atom: `Atom` with the relation position widened
to `AtomSource` (the design's `Atom.relation → Atom.source` cut). An
`idb` atom's bindings address HEAD POSITIONS positionally — `FieldId
i` is the target predicate's column `i` — through the same `FieldId`
reading (`FieldId` is already positional, never nominal). -/
structure PAtom where
  source : AtomSource
  bindings : List (FieldId × Term)

/-- A program-level rule: `Rule` with `PAtom` occurrences, everything
else verbatim — same scope law (a rule is its own variable scope),
same negation reading (a position, not a kind of atom), same
projected-variable head. The head gravestones carry over unchanged:
no mint, no arithmetic, no measure is writable in `finds`. -/
structure PRule where
  finds : List VarId
  atoms : List PAtom
  negated : List PAtom
  conditions : List Condition

/-- One predicate: head arity plus deriving rules — today's `Query`
verbatim as the predicate body (the design's `PredicateDef`). -/
structure PredicateDef where
  arity : Nat
  rules : List PRule

/-- A program: a predicate list (`PredId` = index) and the answer
predicate (the design's `Program`). Boundary caps (`MAX_PREDICATES`,
per-predicate `MAX_RULES`) are hostile-input mechanism, not
semantics — unmodeled, like `MAX_RULES` on `Query`. -/
structure Program where
  predicates : List PredicateDef
  output : PredId

/-- Every rule of every predicate — the quantification surface the
fixpoint theorems range over. -/
def Program.rulesList (p : Program) : List PRule :=
  p.predicates.flatMap PredicateDef.rules

/-! ## The degenerate embedding (syntax half; the theorem is
`Exec/Fixpoint.lean: degenerate_embedding`) -/

/-- An atom is a program atom over its stored relation. -/
def Atom.toPAtom (a : Atom) : PAtom :=
  { source := .edb a.relation, bindings := a.bindings }

/-- A rule embeds field-for-field; every occurrence is `edb`. -/
def Rule.toPRule (r : Rule) : PRule :=
  { finds := r.finds, atoms := r.atoms.map Atom.toPAtom,
    negated := r.negated.map Atom.toPAtom, conditions := r.conditions }

/-- The degenerate program: ONE predicate, no `idb` atom — today's
`Query`, field for field (the `Query::single` precedent). -/
def Query.toProgram (q : Query) : Program :=
  { predicates := [{ arity := q.arity, rules := q.rules.map Rule.toPRule }],
    output := ⟨0⟩ }

/-! ## Variable occurrence over program rules — `PRule.Safe`'s raw
material (the `Rule` functions, verbatim over `PAtom`) -/

/-- The variables a program atom's bindings mention. -/
def PAtom.vars (a : PAtom) : List VarId :=
  a.bindings.flatMap fun b => b.2.vars

/-- The variables a program atom BINDS (positive positions only bind
`var` terms — `Term.bindingVars`). -/
def PAtom.boundVars (a : PAtom) : List VarId :=
  a.bindings.flatMap fun b => b.2.bindingVars

/-- The variables bound by a program rule's positive atoms. -/
def PRule.positiveVars (r : PRule) : List VarId :=
  r.atoms.flatMap PAtom.boundVars

/-- Every variable a program rule mentions anywhere. -/
def PRule.allVars (r : PRule) : List VarId :=
  r.finds ++ r.atoms.flatMap PAtom.vars ++ r.negated.flatMap PAtom.vars
    ++ r.conditions.flatMap Condition.vars

/-! ## Stratification — the dependency graph as data, the witness as
a predicate (recursion-design §2; validation models it) -/

/-- An edge label: how a rule reads its target predicate. Fold-input
is UNREPRESENTABLE at this level (module doc: heads are projected
variables, so program predicates are projection-shaped by
construction; folds read a finished output fixpoint). -/
inductive EdgeKind where
  | positive
  | negated
deriving DecidableEq

/-- One dependency edge: the predicate a rule reads and how. -/
structure Edge where
  target : PredId
  kind : EdgeKind
deriving DecidableEq

/-- A rule's positively read predicates. -/
def PRule.idbPositive (r : PRule) : List PredId :=
  r.atoms.filterMap fun a => a.source.idb?

/-- A rule's negatively read predicates. -/
def PRule.idbNegated (r : PRule) : List PredId :=
  r.negated.filterMap fun a => a.source.idb?

/-- A rule's dependency edges — the graph, one rule at a time. -/
def PRule.edges (r : PRule) : List Edge :=
  r.idbPositive.map (fun Q => ⟨Q, .positive⟩)
    ++ r.idbNegated.map (fun Q => ⟨Q, .negated⟩)

/-- `strat` witnesses stratification: along every edge of every rule,
positive targets sit no higher and negated targets sit STRICTLY
lower. Negation of a lower stratum is legal — a lower stratum is a
finished set before this stratum's operator runs, which is exactly
what keeps the operator monotone (`Exec/Fixpoint.lean:
stratumOp_mono` spends the negated half). -/
def Program.StratifiedBy (p : Program) (strat : PredId → Nat) : Prop :=
  ∀ i d, p.predicates[i]? = some d → ∀ r, r ∈ d.rules → ∀ e, e ∈ r.edges →
    (e.kind = .positive → strat e.target ≤ strat ⟨i⟩) ∧
    (e.kind = .negated → strat e.target < strat ⟨i⟩)

/-- Stratified: some stratum witness exists. Validation computes one
(SCC condensation — mechanism); the semantics carries the witness. -/
def Program.Stratified (p : Program) : Prop :=
  ∃ strat, p.StratifiedBy strat

/-! ## Well-formed sources — the unknown-PredId screen -/

/-- Every `idb` source of every rule — positive and negated — names a
real predicate: the index sits inside `predicates`. This is the
unknown-PredId roster item of `docs/reference/recursion-design.md` §1
as a predicate over the syntax (the module-doc gap record): WITHOUT
it, a phantom `idb` read denotes the empty fact set — a positive
phantom read kills its rule, a NEGATED phantom read is vacuously
satisfied — and `StratifiedBy` never screens it (map the phantom
low). Accepted programs carry this predicate; the degenerate
embedding carries it vacuously (`Query.toProgram_wellFormed`). -/
def Program.WellFormed (p : Program) : Prop :=
  ∀ r, r ∈ p.rulesList → ∀ a, (a ∈ r.atoms ∨ a ∈ r.negated) →
    ∀ Q, a.source = .idb Q → Q.id < p.predicates.length

/-- The degenerate program is well-formed: no rule of an embedded
query reads any `idb` source at all. -/
theorem Query.toProgram_wellFormed (q : Query) :
    q.toProgram.WellFormed := by
  intro r hr a ha Q hsrc
  obtain ⟨d, hd, hrd⟩ := List.mem_flatMap.mp hr
  rw [show q.toProgram.predicates
      = [{ arity := q.arity, rules := q.rules.map Rule.toPRule }] from rfl,
    List.mem_singleton] at hd
  subst hd
  obtain ⟨r₀, -, hr₀⟩ := List.mem_map.mp hrd
  subst hr₀
  have hedb : ∃ b : Atom, a = b.toPAtom := by
    rcases ha with ha | ha
    · obtain ⟨b, -, hb⟩ := List.mem_map.mp ha
      exact ⟨b, hb.symm⟩
    · obtain ⟨b, -, hb⟩ := List.mem_map.mp ha
      exact ⟨b, hb.symm⟩
  obtain ⟨b, rfl⟩ := hedb
  exact nomatch hsrc

/-- `idb?` reads back the source — the membership bridge for the
occurrence lists. -/
theorem AtomSource.idb?_eq_some {s : AtomSource} {Q : PredId} :
    s.idb? = some Q ↔ s = .idb Q := by
  cases s with
  | edb R => simp [AtomSource.idb?]
  | idb P => simp [AtomSource.idb?]

/-- A negated `idb` occurrence is a negated edge. -/
theorem PRule.negated_edge {r : PRule} {a : PAtom} {Q : PredId}
    (ha : a ∈ r.negated) (hsrc : a.source = .idb Q) :
    (⟨Q, .negated⟩ : Edge) ∈ r.edges :=
  List.mem_append.mpr (Or.inr (List.mem_map.mpr
    ⟨Q, List.mem_filterMap.mpr ⟨a, ha, AtomSource.idb?_eq_some.mpr hsrc⟩,
      rfl⟩))

/-- A positive `idb` occurrence is a positive edge. -/
theorem PRule.positive_edge {r : PRule} {a : PAtom} {Q : PredId}
    (ha : a ∈ r.atoms) (hsrc : a.source = .idb Q) :
    (⟨Q, .positive⟩ : Edge) ∈ r.edges :=
  List.mem_append.mpr (Or.inl (List.mem_map.mpr
    ⟨Q, List.mem_filterMap.mpr ⟨a, ha, AtomSource.idb?_eq_some.mpr hsrc⟩,
      rfl⟩))

/-- The stratification witness, spent at a negated occurrence: the
target is strictly below — the premise `stratumOp_mono` cashes. -/
theorem Program.StratifiedBy.negated_lt {p : Program}
    {strat : PredId → Nat} (h : p.StratifiedBy strat) {i : Nat}
    {d : PredicateDef} (hd : p.predicates[i]? = some d) {r : PRule}
    (hr : r ∈ d.rules) {a : PAtom} (ha : a ∈ r.negated) {Q : PredId}
    (hsrc : a.source = .idb Q) : strat Q < strat ⟨i⟩ :=
  (h i d hd r hr _ (PRule.negated_edge ha hsrc)).2 rfl

/-- The stratification witness, spent at a positive occurrence. -/
theorem Program.StratifiedBy.positive_le {p : Program}
    {strat : PredId → Nat} (h : p.StratifiedBy strat) {i : Nat}
    {d : PredicateDef} (hd : p.predicates[i]? = some d) {r : PRule}
    (hr : r ∈ d.rules) {a : PAtom} (ha : a ∈ r.atoms) {Q : PredId}
    (hsrc : a.source = .idb Q) : strat Q ≤ strat ⟨i⟩ :=
  (h i d hd r hr _ (PRule.positive_edge ha hsrc)).1 rfl

end Bumbledb.Query
