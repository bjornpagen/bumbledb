import Bumbledb.Query.Syntax

/-!
# Query denotation — the matching equation (Level 0, PRD 04)

What a query means: THE matching equation, unification, positive range
restriction (`Safe`), the anti-join, condition lowering (DNF
preservation), rule union, answer identity — the normative denotation
the executor, the naive model, and the conformance lane (PRD 13) are
all measured against. Plus the EXECUTABLE half: `evalList`, a
`List`-backed evaluator over concrete finite instances, with
`eval_sound` — the refinement theorem PRD 13 stands on.

## Narrowings recorded (law 5: narrow and record)

* **Allen `classify` is an opaque parameter** (`Classify`): the
  denotation is parametric in the classification function — PRD 05
  refines it; nothing here inspects it, so every theorem holds for
  the real one by instantiation.
* **A ray's measure comparison is FALSE, not an error.** The model's
  `Value.measure?` is `none` on rays (`measure_ray_none`), so a
  measure term SELECTS nothing and any comparison over it fails. The
  engine instead raises the typed execution error
  `crate::Error::MeasureOfRay` — an effect this level does not carry;
  the conformance lane (PRD 13) compares answer sets on error-free
  executions only.
* **`Value.measure?` carries a domain-ceiling guard** (`measureOfNat`):
  the gap of an in-domain interval is always below `2^64`, but the
  `PointDomain.gap` payload is a bare `Nat`, so the embedding into a
  `U64` value checks the bound it cannot see — the `none` arm is
  unreachable on real intervals and exists for totality alone.
* **Ill-typed comparisons are total, and the validator makes them
  unreachable.** Order operators on non-scalars, mismatched element
  domains, and `pointIn` on non-intervals fall through to the empty
  reading (`False`); `ne` does NOT — `cmpDen .ne a b` is plain value
  disequality, TRUE on a type-mismatched pair (a `u64` and a `bool`
  differ as `Value`s). The model does not arbitrate ill-typed pairs:
  the validator REJECTS every such shape (`ir/validate/context.rs`,
  the typed pass — `ValidationError::IllegalComparison`), so no
  accepted rule reaches these arms; totality is for the model's own
  sake, not a semantic claim.
* **Finite instances are association lists** (`ListInstance`) for the
  executable half; the `Set`-valued denotation stays over arbitrary
  `Instance`s.
* **`PendingIntern` string literals are outside the modeled
  fragment.** Model literals carry `StrId`; the engine's raw-bytes
  string literal resolves per execution
  (`ir/normalize/lower_literal.rs::lower_literal`), a dictionary miss
  becoming the never-minted sentinel so `Eq` fails and `Ne` passes
  (`exec/dispatch/key_probe_fact.rs::const_operand`,
  `exec/dispatch/key_probe_fact.rs::const_bytes`). That coincides
  with this model's exclusion reading — an absent string equals no
  stored value — but no theorem covers the resolution step itself.

## The decidable instances

`DecidableEq Value` and the comparison deciders live here, not in
`Values.lean`: PRD 04's executable half is what spends them (the
technical direction: decidable instances added only where PRD 13
needs them).
-/

namespace Bumbledb

/-! ## Value observers — the readings the comparisons spend -/

/-- The point a scalar value denotes at an interval position — the
predicate form of the membership typing rule reads the point side
through this (`Point` is PRD 03's tagged sum). Scalars of the two
element domains carry a point; everything else carries none. -/
def Value.point : Value → Option Point
  | { type := .u64, val := x } => some (.u64 x)
  | { type := .i64, val := x } => some (.i64 x)
  | _ => none

/-- The encoded order word of an orderable scalar, tagged by its
element domain — order comparisons read THIS, so the order the model
compares is the order the storage encodings sort
(`encode_u64_order_embedding` / `encode_i64_order_embedding` carry it
to the numeric order). Cross-domain comparison is unrepresentable:
the tags must agree. -/
def Value.orderWord : Value → Option (Elem × Word)
  | { type := .u64, val := x } => some (.u64, encodeU64 x)
  | { type := .i64, val := x } => some (.i64, encodeI64 x)
  | _ => none

/-- Strict value order: same element domain, strictly smaller encoded
word. `False` on every non-scalar operand — the order-operand screen's
denotation (`screen_order_operand` rejects what this empties). -/
def Value.vlt (a b : Value) : Prop :=
  match a.orderWord, b.orderWord with
  | some (e₁, w₁), some (e₂, w₂) => e₁ = e₂ ∧ w₁ < w₂
  | _, _ => False

/-- Non-strict value order, as `Value.vlt`. -/
def Value.vle (a b : Value) : Prop :=
  match a.orderWord, b.orderWord with
  | some (e₁, w₁), some (e₂, w₂) => e₁ = e₂ ∧ w₁ ≤ w₂
  | _, _ => False

/-- The `U64` value a gap denotes — total via the domain-ceiling guard
(the recorded narrowing: the `none` arm is unreachable on gaps of
in-domain intervals). -/
def measureOfNat (n : Nat) : Option Value :=
  if h : n < 2 ^ 64 then some ⟨.u64, ⟨n, h⟩⟩ else none

/-- The measure of an interval value: `none` on rays (the MeasureOfRay
law, `measure_ray_none`) and on non-interval values; else the gap as a
`U64` value. Bridge: `crate::ir::Term::Measure` evaluation — the
two-slot read + subtraction; the ray raises
`crate::Error::MeasureOfRay` where this model reads `none`. -/
def Value.measure? : Value → Option Value
  | { type := .interval .u64, val := iv } => iv.measure.bind measureOfNat
  | { type := .interval .i64, val := iv } => iv.measure.bind measureOfNat
  -- A fixed-width value measures through its derived interval —
  -- always `some w` (`fixed_measure_const_u64`/`_i64`): never a ray,
  -- so the measure position accepts it trivially (the recorded
  -- choice, `Values.lean`).
  | { type := .intervalFixed .u64 _, val := v } =>
    v.toInterval.measure.bind measureOfNat
  | { type := .intervalFixed .i64 _, val := v } =>
    v.toInterval.measure.bind measureOfNat
  | _ => none

/-- The `Interval U64` a value carries, if any — a fixed-width value
carries its DERIVED `[s, s + w)`: Allen and `pointIn` classify over
derived bounds, so the fixed family participates in every
interval-pair reading through this one observer. -/
def Value.intervalU64 : Value → Option (Interval U64)
  | { type := .interval .u64, val := iv } => some iv
  | { type := .intervalFixed .u64 _, val := v } => some v.toInterval
  | _ => none

/-- The `Interval I64` a value carries, if any (fixed-width values as
`Value.intervalU64`). -/
def Value.intervalI64 : Value → Option (Interval I64)
  | { type := .interval .i64, val := iv } => some iv
  | { type := .intervalFixed .i64 _, val := v } => some v.toInterval
  | _ => none

namespace Query

/-! ## Environments -/

/-- An assignment: a total valuation of the rule's variable scope.
Rules quantify over assignments — a variable is "unbound" only in the
sense that no binding constrains it; the matching equation then binds
it by constraining `σ v`. -/
abbrev Assignment : Type := VarId → Value

/-- The parameter environment: scalar values, set slices, and Allen
masks, each by param id. A `ParamId` is scalar or set, never both
(`ValidationError::ParamScalarAndSet`); the three faces model the
three bind-time payload kinds (`crate::BindValue`), and an id's unused
faces are simply never read. -/
structure ParamEnv where
  scalar : ParamId → Value
  set : ParamId → List Value
  mask : ParamId → AllenMask

/-- One answer: the projected find tuple. -/
abbrev AnswerTuple : Type := List Value

/-! ## THE matching equation -/

/-- What a term SELECTS at a fact position — the matching equation,
term by term (the port of the artifact's `MatchTerms`, reshaped to
named-field bindings): an unbound variable binds (the equation
constrains `σ v`), a bound variable demands equality (same `σ v`,
one value), a param or literal selects its value, a param set selects
membership of the slice. A measure selects its finite measure value —
legal only in order comparisons; in an atom binding the validator
rejects it (`ValidationError::DurationInBinding`), so that arm is
unreachable on accepted rules (the shape discipline `WellTyped`). -/
def Term.selects (ρ : ParamEnv) (σ : Assignment) : Term → Value → Prop
  | .var v, w => σ v = w
  | .param p, w => ρ.scalar p = w
  | .paramSet p, w => w ∈ ρ.set p
  | .lit c, w => c = w
  | .measure v, w => (σ v).measure? = some w

/-- **THE matching equation**: a fact matches an atom under an
assignment and a parameter environment iff every binding's term
selects the fact's value at that field. Absence of a field IS the
wildcard — unlisted fields are unconstrained, and the zero-binding
atom matches every fact (the nonemptiness gate).
Named `Matches`, capitalized: `matches` is a Lean keyword (the
term-level pattern test), so the judgment joins the tree's
capitalized-Prop convention (`Functionality`, `Coverage`).
Bridge: `crate::ir::Atom::bindings`; the executor's probe/scan
kernels realize exactly this per-binding conjunction. -/
def Matches (f : Fact) (a : Atom) (σ : Assignment) (ρ : ParamEnv) :
    Prop :=
  ∀ b, b ∈ a.bindings → Term.selects ρ σ b.2 (f b.1)

/-- Membership in the matching equation, unfolded. -/
theorem matches_def {f : Fact} {a : Atom} {σ : Assignment}
    {ρ : ParamEnv} :
    Matches f a σ ρ ↔ ∀ b, b ∈ a.bindings → Term.selects ρ σ b.2 (f b.1) :=
  Iff.rfl

/-! ## Theorem 1 — repeated variables unify -/

/-- **Theorem 1a (port, intra-atom).** A variable repeated within one
atom forces same-fact equality of the two field values: both bindings
select the ONE value `σ v`.
Bridge: repeated `VarId`s in one atom's bindings lower to one binding
slot; the kernels compare both field words against the same slot. -/
theorem repeated_var_unifies {f : Fact} {a : Atom} {σ : Assignment}
    {ρ : ParamEnv} {v : VarId} {i j : FieldId}
    (h : Matches f a σ ρ)
    (hi : (i, Term.var v) ∈ a.bindings)
    (hj : (j, Term.var v) ∈ a.bindings) :
    f i = f j :=
  (h _ hi).symm.trans (h _ hj)

/-- **Theorem 1b (port, cross-atom).** A variable repeated across two
atoms of one rule denotes a JOIN: any two facts matching the two atoms
under one assignment agree on the shared variable's positions — the
equijoin is the matching equation quantified twice over one `σ`.
Bridge: shared `VarId`s across atoms become join constraints in the
plan; the Free Join realization enumerates exactly the shared-slot
agreements. -/
theorem repeated_var_unifies_cross_atom {f g : Fact} {a b : Atom}
    {σ : Assignment} {ρ : ParamEnv} {v : VarId} {i j : FieldId}
    (ha : Matches f a σ ρ) (hb : Matches g b σ ρ)
    (hi : (i, Term.var v) ∈ a.bindings)
    (hj : (j, Term.var v) ∈ b.bindings) :
    f i = g j :=
  (ha _ hi).symm.trans (hb _ hj)

/-! ## Theorem 2 — params select, never bind -/

/-- **Theorem 2 (port).** A parameter position SELECTS and never
binds: its satisfaction is independent of the assignment (left), and
it forces the fact's field to the environment's value (right) — a
param is read from `ρ`, a variable is bound in `σ`, and the two never
trade roles. Bridge: `crate::ir::Term::Param`; the hostile lock
`a_param_position_does_not_bind_a_negated_variable_even_when_written_after_it`
pins the never-binds half at the validator. -/
theorem param_selects_not_binds {f : Fact} {a : Atom}
    {σ σ' : Assignment} {ρ : ParamEnv} {p : ParamId} {i : FieldId}
    (h : Matches f a σ ρ) (hi : (i, Term.param p) ∈ a.bindings) :
    (Term.selects ρ σ (Term.param p) (f i) ↔
      Term.selects ρ σ' (Term.param p) (f i)) ∧
    f i = ρ.scalar p :=
  ⟨Iff.rfl, (h _ hi).symm⟩

/-- **Theorem 2 companion.** A param-set position selects MEMBERSHIP
of the bind-time slice — the field value is some element of the set,
never a fresh binding. Bridge: `crate::ir::Term::ParamSet` (a binding
position matches iff the field value is in the set). -/
theorem paramSet_selects_membership {f : Fact} {a : Atom}
    {σ : Assignment} {ρ : ParamEnv} {p : ParamId} {i : FieldId}
    (h : Matches f a σ ρ) (hi : (i, Term.paramSet p) ∈ a.bindings) :
    f i ∈ ρ.set p :=
  h _ hi

/-! ## `Safe` — positive range restriction -/

/-- **`Safe`** — positive range restriction (port of the artifact's
`PositivelyRangeRestricted`, intentionally stronger than lexical
well-scopedness): every variable the rule mentions ANYWHERE — head
finds, negated atoms, conditions — is bound by a positive atom
binding. Positive atoms are the language's one binding site; a
negated atom binds nothing, only rejects, and a comparison binds
nothing, only filters.

This is the validator's spec in three diagnostics:
`NegatedVariableUnbound` (negated-atom variables),
`ComparisonOnlyVariable` (condition variables), and the find-side
check — plus `MembershipOnlyVariable` as the point-variable face
(`membership_only_unsafe`). Bridge:
`ir/validate/context.rs::check_atoms` (the negation safety walk,
positives first by construction) and `comparison_var`. -/
def Safe (r : Rule) : Prop :=
  ∀ v, v ∈ r.allVars → v ∈ r.positiveVars

/-- `Safe` spends on negated atoms: every negated-atom variable is
positively bound — the exact spec of `NegatedVariableUnbound`, pinned
by the lock `rejects_a_negated_atom_variable_unbound_by_positive_atoms`. -/
theorem safe_negated_bound {r : Rule} (h : Safe r) {a : Atom}
    (ha : a ∈ r.negated) {v : VarId} (hv : v ∈ a.vars) :
    v ∈ r.positiveVars :=
  h v (by
    unfold Rule.allVars
    simp only [List.mem_append, List.mem_flatMap]
    exact Or.inl (Or.inr ⟨a, ha, hv⟩))

/-- **The membership-only refusal as a Safety consequence.** A point
variable bound ONLY by membership reaches this level as a variable
occurring in conditions (the `pointIn` predicate form — the membership
binding's typing rule) with no positive value-binding occurrence: such
a rule is UNSAFE, exactly the no-enumerable-domain refusal.
Bridge: `ValidationError::MembershipOnlyVariable`
(`check_membership_domains`; lock `rejects_a_membership_only_variable`). -/
theorem membership_only_unsafe {r : Rule} {v : VarId}
    (hmem : v ∈ r.allVars) (hnopos : v ∉ r.positiveVars) :
    ¬ Safe r :=
  fun h => hnopos (h v hmem)

/-! ## Theorem 4 — safety is order-independent -/

/-- Membership in `Rule.allVars`, unfolded to the four sites. -/
theorem mem_allVars {r : Rule} {v : VarId} :
    v ∈ r.allVars ↔
      v ∈ r.finds ∨ (∃ a, a ∈ r.atoms ∧ v ∈ a.vars) ∨
      (∃ a, a ∈ r.negated ∧ v ∈ a.vars) ∨
      (∃ t, t ∈ r.conditions ∧ v ∈ t.vars) := by
  unfold Rule.allVars
  simp only [List.mem_append, List.mem_flatMap, or_assoc]

/-- Membership in `Rule.positiveVars`, unfolded. -/
theorem mem_positiveVars {r : Rule} {v : VarId} :
    v ∈ r.positiveVars ↔ ∃ a, a ∈ r.atoms ∧ v ∈ a.boundVars := by
  unfold Rule.positiveVars
  exact List.mem_flatMap

/-- **Theorem 4.** `Safe` is invariant under ANY permutation of a
rule's items — order carries no meaning anywhere in a rule; a negated
atom is safe or unsafe regardless of where it is written relative to
the positive atom that binds its variables. The spec of the validator's
order-independence: `check_atoms` walks positives first BY CONSTRUCTION
(never by input order), pinned by the hostile lock
`a_param_position_does_not_bind_a_negated_variable_even_when_written_after_it`. -/
theorem safety_order_independent {r r' : Rule}
    (hf : ∀ v : VarId, v ∈ r.finds ↔ v ∈ r'.finds)
    (ha : ∀ a : Atom, a ∈ r.atoms ↔ a ∈ r'.atoms)
    (hn : ∀ a : Atom, a ∈ r.negated ↔ a ∈ r'.negated)
    (hc : ∀ t : Condition, t ∈ r.conditions ↔ t ∈ r'.conditions) :
    Safe r ↔ Safe r' := by
  unfold Safe
  constructor
  · intro h v hv
    have := h v (mem_allVars.mpr (by
      rcases mem_allVars.mp hv with h1 | ⟨a, haa, hva⟩ | ⟨a, haa, hva⟩ | ⟨t, htt, hvt⟩
      · exact Or.inl ((hf v).mpr h1)
      · exact Or.inr (Or.inl ⟨a, (ha a).mpr haa, hva⟩)
      · exact Or.inr (Or.inr (Or.inl ⟨a, (hn a).mpr haa, hva⟩))
      · exact Or.inr (Or.inr (Or.inr ⟨t, (hc t).mpr htt, hvt⟩))))
    rcases mem_positiveVars.mp this with ⟨a, haa, hva⟩
    exact mem_positiveVars.mpr ⟨a, (ha a).mp haa, hva⟩
  · intro h v hv
    have := h v (mem_allVars.mpr (by
      rcases mem_allVars.mp hv with h1 | ⟨a, haa, hva⟩ | ⟨a, haa, hva⟩ | ⟨t, htt, hvt⟩
      · exact Or.inl ((hf v).mp h1)
      · exact Or.inr (Or.inl ⟨a, (ha a).mp haa, hva⟩)
      · exact Or.inr (Or.inr (Or.inl ⟨a, (hn a).mp haa, hva⟩))
      · exact Or.inr (Or.inr (Or.inr ⟨t, (hc t).mp htt, hvt⟩))))
    rcases mem_positiveVars.mp this with ⟨a, haa, hva⟩
    exact mem_positiveVars.mpr ⟨a, (ha a).mpr haa, hva⟩

/-- Permutations are membership-preserving, so `safety_order_independent`
covers them: the `List.Perm` corollary, stated for the criterion's
plain reading. -/
theorem safety_perm_invariant {r r' : Rule}
    (hf : r.finds.Perm r'.finds) (ha : r.atoms.Perm r'.atoms)
    (hn : r.negated.Perm r'.negated)
    (hc : r.conditions.Perm r'.conditions) :
    Safe r ↔ Safe r' :=
  safety_order_independent (fun _ => hf.mem_iff) (fun _ => ha.mem_iff)
    (fun _ => hn.mem_iff) (fun _ => hc.mem_iff)

/-! ## Conditions — comparison denotation -/

/-- The Allen classification, ABSTRACT at this level (the recorded
narrowing): one classifier per element domain, mapping an interval
pair to its Allen relation. PRD 05 refines it to the concrete
thirteen-way case split (`crate::allen::classify`); nothing in PRD 04
inspects it, so every theorem holds for the refinement by
instantiation. -/
structure Classify where
  u64 : Interval U64 → Interval U64 → AllenRel
  i64 : Interval I64 → Interval I64 → AllenRel

/-- The classification of a VALUE pair: defined exactly on interval
pairs of one element domain — the typed legality of the `allen`
comparison, as a partiality. -/
def classifyValue (C : Classify) (a b : Value) : Option AllenRel :=
  match a.intervalU64, b.intervalU64 with
  | some iv, some jv => some (C.u64 iv jv)
  | _, _ =>
    match a.intervalI64, b.intervalI64 with
    | some iv, some jv => some (C.i64 iv jv)
    | _, _ => none

/-- The mask a mask term denotes: a literal mask is itself; a param
mask reads the environment (`crate::BindValue::AllenMask`, resolved at
bind). -/
def MaskTerm.den (ρ : ParamEnv) : MaskTerm → AllenMask
  | .lit m => m
  | .param p => ρ.mask p

/-- One operator's denotation over a value pair. Equality is value
identity (the canonical-bytes law `value_eq_iff_encode_eq` carries it
to the encoding); order reads the encoded word order (`Value.vlt`,
with `gt`/`ge` the mirrored reads — validation seals the mirror,
`OpClass::Order`); `allen` is mask membership of the classification;
`pointIn` is point membership, interval left, point right
(`crate::ir::CmpOp`). Ill-typed operand pairs: the order, `allen`,
and `pointIn` arms fall through to `False`, while `eq`/`ne` stay
plain value identity/disequality (`ne` holds on a mismatched pair) —
the validator's `IllegalComparison` makes every ill-typed pair
unreachable on accepted rules (module doc). Note interval `eq` here
is plain value identity, whereas the engine canonicalizes it to
`Allen(EQUALS)` — PRD 05's `classify` refinement makes the two
readings provably equal. -/
def cmpDen (C : Classify) (ρ : ParamEnv) : CmpOp → Value → Value → Prop
  | .eq, a, b => a = b
  | .ne, a, b => a ≠ b
  | .lt, a, b => a.vlt b
  | .le, a, b => a.vle b
  | .gt, a, b => b.vlt a
  | .ge, a, b => b.vle a
  | .allen m, a, b => ∃ rel, classifyValue C a b = some rel ∧ rel ∈ m.den ρ
  | .pointIn, a, b => ∃ p, b.point = some p ∧ p ∈ a.points

/-- One comparison holds when each side selects a value and the
operator's denotation relates them — the `∃` reading makes the param
set's membership semantics (`EqVarSet`) and the measure's
finite-measure semantics fall out of `Term.selects` with no extra
cases. -/
def Comparison.holds (C : Classify) (ρ : ParamEnv) (σ : Assignment)
    (c : Comparison) : Prop :=
  ∃ a b, Term.selects ρ σ c.lhs a ∧ Term.selects ρ σ c.rhs b ∧
    cmpDen C ρ c.op a b

mutual
  /-- A condition tree's denotation: leaves are comparisons, `and` is
  conjunction over the children, `or` is disjunction — the empty
  combinations keep their algebraic readings (`and []` true, `or []`
  false), exactly as `crate::ir::ConditionTree` documents them. -/
  def Condition.holds (C : Classify) (ρ : ParamEnv) (σ : Assignment) :
      Condition → Prop
    | .leaf c => c.holds C ρ σ
    | .and cs => Condition.allHold C ρ σ cs
    | .or cs => Condition.anyHold C ρ σ cs

  /-- Every condition of the list holds. -/
  def Condition.allHold (C : Classify) (ρ : ParamEnv) (σ : Assignment) :
      List Condition → Prop
    | [] => True
    | t :: ts => Condition.holds C ρ σ t ∧ Condition.allHold C ρ σ ts

  /-- Some condition of the list holds. -/
  def Condition.anyHold (C : Classify) (ρ : ParamEnv) (σ : Assignment) :
      List Condition → Prop
    | [] => False
    | t :: ts => Condition.holds C ρ σ t ∨ Condition.anyHold C ρ σ ts
end

/-- `allHold` is the forall-membership conjunction. -/
theorem Condition.allHold_iff {C : Classify} {ρ : ParamEnv}
    {σ : Assignment} :
    ∀ cs : List Condition,
      Condition.allHold C ρ σ cs ↔ ∀ t, t ∈ cs → Condition.holds C ρ σ t
  | [] => by simp [Condition.allHold]
  | t :: ts => by
    simp [Condition.allHold, Condition.allHold_iff ts]

/-- `anyHold` is the exists-membership disjunction. -/
theorem Condition.anyHold_iff {C : Classify} {ρ : ParamEnv}
    {σ : Assignment} :
    ∀ cs : List Condition,
      Condition.anyHold C ρ σ cs ↔ ∃ t, t ∈ cs ∧ Condition.holds C ρ σ t
  | [] => by simp [Condition.anyHold]
  | t :: ts => by
    simp [Condition.anyHold, Condition.anyHold_iff ts]

/-! ## Theorem 8 — pointIn and Allen unfold -/

/-- **Theorem 8a (port).** `pointIn` unfolds to the half-open reading:
`x ∈ [start, end)` is `start ≤ x ∧ x < end` — inclusive at `start`,
exclusive at `end` (`points_halfopen` carries the interval side).
Stated over the `u64` element domain; `pointIn_unfold_i64` is the
companion. Bridge: `crate::ir::CmpOp::PointIn` — "`x PointIn iv` iff
`iv.start ≤ x < iv.end`"; normalization lowers it to the two endpoint
comparisons this reading is. -/
theorem pointIn_unfold (C : Classify) (ρ : ParamEnv)
    (iv : Interval U64) (x : U64) :
    cmpDen C ρ .pointIn ⟨.interval .u64, iv⟩ ⟨.u64, x⟩ ↔
      iv.start ≤ x ∧ x < iv.«end» := by
  constructor
  · rintro ⟨p, hp, hmem⟩
    cases hp
    exact hmem
  · intro h
    exact ⟨.u64 x, rfl, h⟩

/-- **Theorem 8a (i64 companion).** -/
theorem pointIn_unfold_i64 (C : Classify) (ρ : ParamEnv)
    (iv : Interval I64) (x : I64) :
    cmpDen C ρ .pointIn ⟨.interval .i64, iv⟩ ⟨.i64, x⟩ ↔
      iv.start ≤ x ∧ x < iv.«end» := by
  constructor
  · rintro ⟨p, hp, hmem⟩
    cases hp
    exact hmem
  · intro h
    exact ⟨.i64 x, rfl, h⟩

/-- **Theorem 8b (port shape).** The Allen comparison denotes mask
membership of the classification: `Allen iv mask jv ↔ classify iv jv ∈
mask`, with `classify` abstract here (PRD 05 refines it). Stated over
the `u64` element domain; `allen_mask_denotation_i64` is the
companion. Bridge: `crate::allen::AllenMask::contains(classify(lhs,
rhs))` — the mask-carrying filter shapes of `ir/normalize`. -/
theorem allen_mask_denotation (C : Classify) (ρ : ParamEnv)
    (m : MaskTerm) (iv jv : Interval U64) :
    cmpDen C ρ (.allen m) ⟨.interval .u64, iv⟩ ⟨.interval .u64, jv⟩ ↔
      C.u64 iv jv ∈ m.den ρ := by
  constructor
  · rintro ⟨rel, hrel, hmem⟩
    cases hrel
    exact hmem
  · intro h
    exact ⟨C.u64 iv jv, rfl, h⟩

/-- **Theorem 8b (i64 companion).** -/
theorem allen_mask_denotation_i64 (C : Classify) (ρ : ParamEnv)
    (m : MaskTerm) (iv jv : Interval I64) :
    cmpDen C ρ (.allen m) ⟨.interval .i64, iv⟩ ⟨.interval .i64, jv⟩ ↔
      C.i64 iv jv ∈ m.den ρ := by
  constructor
  · rintro ⟨rel, hrel, hmem⟩
    cases hrel
    exact hmem
  · intro h
    exact ⟨C.i64 iv jv, rfl, h⟩

/-! ## `lower` — the DNF lowering -/

mutual
  /-- The DNF of one condition tree: a list of disjuncts, each a flat
  comparison conjunction — a leaf is one one-leaf disjunct, `and`
  distributes (the cross product), `or` unions (concatenation).
  Mirror of `ir/normalize/dnf.rs::tree_terms`. -/
  def Condition.lower : Condition → List (List Comparison)
    | .leaf c => [[c]]
    | .and cs => Condition.lowerAll cs
    | .or cs => Condition.lowerAny cs

  /-- DNF of a conjunction of trees: the cross product of the
  children's disjunct sets — one empty disjunct for the empty
  conjunction (mirror of `conjunction_terms`). -/
  def Condition.lowerAll : List Condition → List (List Comparison)
    | [] => [[]]
    | t :: ts =>
      (Condition.lower t).flatMap fun d =>
        (Condition.lowerAll ts).map (d ++ ·)

  /-- DNF of a disjunction of trees: concatenation — the empty
  disjunction has NO disjuncts (`or []` lowers to zero rules). -/
  def Condition.lowerAny : List Condition → List (List Comparison)
    | [] => []
    | t :: ts => Condition.lower t ++ Condition.lowerAny ts
end

/-- Every comparison of a disjunct holds — the flat conjunction a
lowered rule carries. -/
def disjunctHolds (C : Classify) (ρ : ParamEnv) (σ : Assignment)
    (d : List Comparison) : Prop :=
  ∀ c, c ∈ d → Comparison.holds C ρ σ c

mutual
  /-- The tree-level DNF preservation: a tree holds iff some lowered
  disjunct holds entirely — lowering-then-evaluating equals evaluating
  the tree naively, at one assignment. -/
  theorem Condition.lower_holds (C : Classify) (ρ : ParamEnv)
      (σ : Assignment) :
      ∀ t : Condition,
        Condition.holds C ρ σ t ↔
          ∃ d, d ∈ t.lower ∧ disjunctHolds C ρ σ d
    | .leaf c => by
      simp [Condition.holds, Condition.lower, disjunctHolds]
    | .and cs => by
      simp only [Condition.holds, Condition.lower]
      exact Condition.lowerAll_holds C ρ σ cs
    | .or cs => by
      simp only [Condition.holds, Condition.lower]
      exact Condition.lowerAny_holds C ρ σ cs

  /-- The conjunction-list half: the cross product's disjuncts are
  exactly the pointwise concatenations. -/
  theorem Condition.lowerAll_holds (C : Classify) (ρ : ParamEnv)
      (σ : Assignment) :
      ∀ cs : List Condition,
        Condition.allHold C ρ σ cs ↔
          ∃ d, d ∈ Condition.lowerAll cs ∧ disjunctHolds C ρ σ d
    | [] => by
      simp [Condition.allHold, Condition.lowerAll, disjunctHolds]
    | t :: ts => by
      simp only [Condition.allHold, Condition.lowerAll]
      rw [Condition.lower_holds C ρ σ t, Condition.lowerAll_holds C ρ σ ts]
      constructor
      · rintro ⟨⟨d₁, hd₁, h₁⟩, ⟨d₂, hd₂, h₂⟩⟩
        refine ⟨d₁ ++ d₂, ?_, ?_⟩
        · exact List.mem_flatMap.mpr
            ⟨d₁, hd₁, List.mem_map.mpr ⟨d₂, hd₂, rfl⟩⟩
        · intro c hc
          rcases List.mem_append.mp hc with h | h
          · exact h₁ c h
          · exact h₂ c h
      · rintro ⟨d, hd, hall⟩
        rcases List.mem_flatMap.mp hd with ⟨d₁, hd₁, hmap⟩
        rcases List.mem_map.mp hmap with ⟨d₂, hd₂, rfl⟩
        exact ⟨⟨d₁, hd₁, fun c hc => hall c (List.mem_append.mpr (.inl hc))⟩,
          ⟨d₂, hd₂, fun c hc => hall c (List.mem_append.mpr (.inr hc))⟩⟩

  /-- The disjunction-list half: concatenation unions the disjunct
  sets. -/
  theorem Condition.lowerAny_holds (C : Classify) (ρ : ParamEnv)
      (σ : Assignment) :
      ∀ cs : List Condition,
        Condition.anyHold C ρ σ cs ↔
          ∃ d, d ∈ Condition.lowerAny cs ∧ disjunctHolds C ρ σ d
    | [] => by
      simp [Condition.anyHold, Condition.lowerAny]
    | t :: ts => by
      simp only [Condition.anyHold, Condition.lowerAny]
      rw [Condition.lower_holds C ρ σ t, Condition.lowerAny_holds C ρ σ ts]
      constructor
      · rintro (⟨d, hd, h⟩ | ⟨d, hd, h⟩)
        · exact ⟨d, List.mem_append.mpr (.inl hd), h⟩
        · exact ⟨d, List.mem_append.mpr (.inr hd), h⟩
      · rintro ⟨d, hd, h⟩
        rcases List.mem_append.mp hd with hm | hm
        · exact .inl ⟨d, hm, h⟩
        · exact .inr ⟨d, hm, h⟩
end

/-! ## Answers — body environments, filtered and projected -/

/-- The body judgment: an assignment derives when every positive atom
has a matching fact in the instance, NO fact matches any negated atom
— **the anti-join**: negation denotes `¬∃` over the relation's finite
extension, NEVER membership in an infinite complement (there is no
complement anywhere in this definition to take) — and every condition
tree holds. Bridge: `docs` § negation — plain anti-join over sets, no
null trick, no three-valued logic; `exec`'s anti-probe descriptors
(`ir/normalize`'s `AntiProbe`) realize the `¬∃` by probe. -/
def derives (C : Classify) (r : Rule) (I : Instance) (ρ : ParamEnv)
    (σ : Assignment) : Prop :=
  (∀ a, a ∈ r.atoms → ∃ f, f ∈ I a.relation ∧ Matches f a σ ρ) ∧
  (∀ a, a ∈ r.negated → ¬ ∃ f, f ∈ I a.relation ∧ Matches f a σ ρ) ∧
  (∀ t, t ∈ r.conditions → Condition.holds C ρ σ t)

/-- One rule's answers: deriving body environments projected through
the finds. A `Set` — multiplicity is UNREPRESENTABLE, which is the
set-semantics law at the representation level. -/
def ruleAnswers (C : Classify) (r : Rule) (I : Instance)
    (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ σ, derives C r I ρ σ ∧ t = r.finds.map σ

/-- Membership in a rule's answers, unfolded. -/
theorem mem_ruleAnswers {C : Classify} {r : Rule} {I : Instance}
    {ρ : ParamEnv} {t : AnswerTuple} :
    t ∈ ruleAnswers C r I ρ ↔
      ∃ σ, derives C r I ρ σ ∧ t = r.finds.map σ :=
  Iff.rfl

/-- **A query's answers: the set UNION of its rules' answers** — the
one union; no bag distinction exists or is representable
(`crate::ir::Query`, the denotation block). -/
def queryAnswers (C : Classify) (q : Query) (I : Instance)
    (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ r, r ∈ q.rules ∧ t ∈ ruleAnswers C r I ρ

/-- Membership in a query's answers, unfolded. -/
theorem mem_queryAnswers {C : Classify} {q : Query} {I : Instance}
    {ρ : ParamEnv} {t : AnswerTuple} :
    t ∈ queryAnswers C q I ρ ↔
      ∃ r, r ∈ q.rules ∧ t ∈ ruleAnswers C r I ρ :=
  Iff.rfl

/-! ## Theorem 3 — the anti-join stays on the active domain -/

/-- A term binds `v` exactly when it IS the variable `v`. -/
theorem Term.mem_bindingVars {t : Term} {v : VarId} :
    v ∈ t.bindingVars ↔ t = .var v := by
  cases t <;> simp [Term.bindingVars]
  exact eq_comm

/-- The active domain a rule sees: every value some positive atom's
fact carries at a bound field. Finite whenever the instance is — the
answers never leave it. -/
def activeDomain (I : Instance) (r : Rule) : Set Value :=
  fun w => ∃ a, a ∈ r.atoms ∧ ∃ f, f ∈ I a.relation ∧
    ∃ b, b ∈ a.bindings ∧ f b.1 = w

/-- **Theorem 3.** Under `Safe` — the hypothesis that makes the safety
rule load-bearing — every answer value lives in the rule's ACTIVE
DOMAIN: the anti-join semantics of `derives` quantifies negation over
the finite relation extension only, and positive range restriction
pins every projected variable to a positive fact's field, so no answer
ever mentions a value the instance does not carry. Without `Safe` the
"denotation" escapes to the infinite value universe —
`Countermodels.unsafe_rule_infinite` is the countermodel.
Bridge: `ValidationError::NegatedVariableUnbound` /
`ComparisonOnlyVariable` are the acceptance forms of the hypothesis;
`answers_finite_of_safe` cashes it into finiteness. -/
theorem antijoin_over_active_domain {C : Classify} {r : Rule}
    {I : Instance} {ρ : ParamEnv} (hsafe : Safe r) :
    ∀ t, t ∈ ruleAnswers C r I ρ → ∀ w, w ∈ t → w ∈ activeDomain I r := by
  intro t ht w hw
  obtain ⟨σ, hder, rfl⟩ := mem_ruleAnswers.mp ht
  obtain ⟨v, hv, rfl⟩ := List.mem_map.mp hw
  have hvall : v ∈ r.allVars := mem_allVars.mpr (Or.inl hv)
  obtain ⟨a, ha, hbound⟩ := mem_positiveVars.mp (hsafe v hvall)
  obtain ⟨b, hb, hvb⟩ := List.mem_flatMap.mp hbound
  have hvar : b.2 = Term.var v := Term.mem_bindingVars.mp hvb
  obtain ⟨f, hf, hm⟩ := hder.1 a ha
  have hsel := hm b hb
  rw [hvar] at hsel
  exact ⟨a, ha, f, hf, b, hb, hsel.symm⟩

/-! ## Theorem 5 — DNF preserves the denotation -/

/-- One rule's DNF: each disjunct of the lowered condition
cross-product becomes a rule — atoms, negated atoms, and finds cloned,
conditions the disjunct's flat leaves. Mirror of
`ir/normalize/dnf.rs::distribute` (`LoweredRule`). -/
def Rule.lower (r : Rule) : List Rule :=
  (Condition.lowerAll r.conditions).map fun d =>
    { finds := r.finds, atoms := r.atoms, negated := r.negated,
      conditions := d.map Condition.leaf }

/-- A flat-leaf condition list holds exactly when its disjunct
does. -/
theorem holds_map_leaf {C : Classify} {ρ : ParamEnv} {σ : Assignment}
    {d : List Comparison} :
    (∀ t, t ∈ d.map Condition.leaf → Condition.holds C ρ σ t) ↔
      disjunctHolds C ρ σ d := by
  simp [disjunctHolds, Condition.holds]

/-- **Theorem 5.** `body (A ∧ (B ∨ C)) = body (A∧B) ∪ body (A∧C)`,
generalized: lowering a rule's condition trees to DNF preserves its
answers — the union of the lowered rules' answers is the rule's. The
normalize pass's contract: lowering-then-evaluating ≡ evaluating the
tree naively; the engine never sees an `Or`.
Bridge: `ir/normalize/dnf.rs::distribute` (each disjunct becomes a
rule; `collapse` then dedups — sound by `union_idempotent`); the
differential suite proves the same property against the naive
model. -/
theorem dnf_preserves_denotation (C : Classify) (r : Rule)
    (I : Instance) (ρ : ParamEnv) :
    ∀ t, t ∈ ruleAnswers C r I ρ ↔
      ∃ r', r' ∈ r.lower ∧ t ∈ ruleAnswers C r' I ρ := by
  intro t
  constructor
  · intro ht
    obtain ⟨σ, ⟨hatoms, hneg, hconds⟩, rfl⟩ := mem_ruleAnswers.mp ht
    have hall : Condition.allHold C ρ σ r.conditions :=
      (Condition.allHold_iff r.conditions).mpr hconds
    obtain ⟨d, hd, hdis⟩ :=
      (Condition.lowerAll_holds C ρ σ r.conditions).mp hall
    refine ⟨_, List.mem_map.mpr ⟨d, hd, rfl⟩,
      mem_ruleAnswers.mpr ⟨σ, ⟨hatoms, hneg, ?_⟩, rfl⟩⟩
    exact holds_map_leaf.mpr hdis
  · rintro ⟨r', hr', ht⟩
    obtain ⟨d, hd, rfl⟩ := List.mem_map.mp hr'
    obtain ⟨σ, ⟨hatoms, hneg, hconds⟩, rfl⟩ := mem_ruleAnswers.mp ht
    refine mem_ruleAnswers.mpr ⟨σ, ⟨hatoms, hneg, ?_⟩, rfl⟩
    exact (Condition.allHold_iff r.conditions).mp
      ((Condition.lowerAll_holds C ρ σ r.conditions).mpr
        ⟨d, hd, holds_map_leaf.mp hconds⟩)

/-! ## Theorem 6 — union is idempotent -/

/-- **Theorem 6 (port of `ruleUnion_set_idempotent`).** A duplicated
rule adds nothing: duplicate rules, duplicate derivations, ONE answer
— set semantics at the program level. Bridge: the sinks are where
union lives (`exec/sink.rs`): one seen-set spans every rule of a
program, reset once per execution, so a later rule re-deriving a head
fact is absorbed exactly like a within-rule duplicate; `dnf.rs::
collapse` spends the same fact at the representation level. -/
theorem union_idempotent (C : Classify) (n : Nat) (r : Rule)
    (rs : List Rule) (I : Instance) (ρ : ParamEnv) :
    ∀ t, t ∈ queryAnswers C ⟨n, r :: r :: rs⟩ I ρ ↔
      t ∈ queryAnswers C ⟨n, r :: rs⟩ I ρ := by
  intro t
  constructor
  · intro ht
    obtain ⟨r', hmem, h⟩ := mem_queryAnswers.mp ht
    rcases List.mem_cons.mp hmem with rfl | hmem'
    · exact mem_queryAnswers.mpr ⟨r', List.mem_cons.mpr (Or.inl rfl), h⟩
    · exact mem_queryAnswers.mpr ⟨r', hmem', h⟩
  · intro ht
    obtain ⟨r', hmem, h⟩ := mem_queryAnswers.mp ht
    exact mem_queryAnswers.mpr ⟨r', List.mem_cons.mpr (Or.inr hmem), h⟩

/-! ## Theorem 7 — answer identity is the projected tuple -/

/-- **Theorem 7.** Two body environments producing the same projected
head tuple are ONE answer: membership in `ruleAnswers` is decided by
the tuple alone, so the second environment's tuple is already the
first's element — the `Set` carrier makes answer multiplicity
unrepresentable, and this is the seen-set's spec.
Bridge: `exec/sink.rs` — the projection sink keys the projected find
tuple (head-shaped keys, rule-independent), never the rule's full slot
array; `answer_identity` is why that key is complete. -/
theorem answer_identity_canonical {C : Classify} {r : Rule}
    {I : Instance} {ρ : ParamEnv} {σ σ' : Assignment}
    (h : derives C r I ρ σ)
    (hproj : r.finds.map σ = r.finds.map σ') :
    r.finds.map σ' ∈ ruleAnswers C r I ρ ∧
      r.finds.map σ ∈ ruleAnswers C r I ρ ∧
      (r.finds.map σ : AnswerTuple) = r.finds.map σ' :=
  ⟨⟨σ, h, hproj.symm⟩, ⟨σ, h, rfl⟩, hproj⟩

/-! ## Theorem 9 — the denotation reads ONE instance -/

/-- The relations a query mentions, positive and negated. -/
def Query.relations (q : Query) : List RelId :=
  q.rules.flatMap fun r =>
    r.atoms.map Atom.relation ++ r.negated.map Atom.relation

/-- **Theorem 9.** The denotation is a function of ONE `Instance` —
the signature of `queryAnswers` IS the structural note (no
mixed-instance evaluation is writable), and this theorem makes it
checkable: two instances agreeing on every mentioned relation yield
identical answers, i.e. the denotation reads nothing else.
Bridge: snapshot isolation — an execution runs against one storage
snapshot (`crate::Db::query` pins one read transaction); PRD 09 owns
the transaction side. -/
theorem snapshot_single {q : Query} {I J : Instance} (C : Classify)
    (ρ : ParamEnv) (h : ∀ R, R ∈ q.relations → I R = J R) :
    ∀ t, t ∈ queryAnswers C q I ρ ↔ t ∈ queryAnswers C q J ρ := by
  have hrel : ∀ r, r ∈ q.rules → ∀ a,
      (a ∈ r.atoms ∨ a ∈ r.negated) → I a.relation = J a.relation := by
    intro r hr a ha
    refine h a.relation (List.mem_flatMap.mpr ⟨r, hr, ?_⟩)
    rcases ha with ha | ha
    · exact List.mem_append.mpr (Or.inl (List.mem_map.mpr ⟨a, ha, rfl⟩))
    · exact List.mem_append.mpr (Or.inr (List.mem_map.mpr ⟨a, ha, rfl⟩))
  have hder : ∀ r, r ∈ q.rules → ∀ σ,
      derives C r I ρ σ ↔ derives C r J ρ σ := by
    intro r hr σ
    unfold derives
    constructor
    · rintro ⟨hatoms, hneg, hconds⟩
      refine ⟨fun a ha => ?_, fun a ha => ?_, hconds⟩
      · exact hrel r hr a (Or.inl ha) ▸ hatoms a ha
      · exact hrel r hr a (Or.inr ha) ▸ hneg a ha
    · rintro ⟨hatoms, hneg, hconds⟩
      refine ⟨fun a ha => ?_, fun a ha => ?_, hconds⟩
      · exact (hrel r hr a (Or.inl ha)).symm ▸ hatoms a ha
      · exact (hrel r hr a (Or.inr ha)).symm ▸ hneg a ha
  intro t
  constructor
  · intro ht
    obtain ⟨r, hr, σ, hd, hproj⟩ := mem_queryAnswers.mp ht
    exact mem_queryAnswers.mpr ⟨r, hr, σ, (hder r hr σ).mp hd, hproj⟩
  · intro ht
    obtain ⟨r, hr, σ, hd, hproj⟩ := mem_queryAnswers.mp ht
    exact mem_queryAnswers.mpr ⟨r, hr, σ, (hder r hr σ).mpr hd, hproj⟩

/-! ## Decidable instances — the executable half's toolkit

Added HERE, not in `Values.lean`: PRD 13's conformance lane is what
spends them (the technical direction's rule). -/

instance {α : Type} [LT α] [DecidableEq α] : DecidableEq (Interval α) :=
  fun iv jv =>
    if h1 : iv.start = jv.start then
      if h2 : iv.«end» = jv.«end» then isTrue (Interval.ext h1 h2)
      else isFalse fun h => h2 (congrArg Interval.«end» h)
    else isFalse fun h => h1 (congrArg Interval.start h)

/-- Decidable equality at each carrier — the per-type payload of
`DecidableEq Value`. -/
def ValueType.carrierDecEq : (t : ValueType) → DecidableEq t.carrier
  | .bool => inferInstanceAs (DecidableEq Bool)
  | .u64 => inferInstanceAs (DecidableEq U64)
  | .i64 => inferInstanceAs (DecidableEq I64)
  | .str => inferInstanceAs (DecidableEq StrId)
  | .fixedBytes n => inferInstanceAs (DecidableEq (FixedBytes n))
  | .interval .u64 => inferInstanceAs (DecidableEq (Interval U64))
  | .interval .i64 => inferInstanceAs (DecidableEq (Interval I64))
  | .intervalFixed .u64 w => inferInstanceAs (DecidableEq (FixedU64 w))
  | .intervalFixed .i64 w => inferInstanceAs (DecidableEq (FixedI64 w))

instance : DecidableEq Value := fun a b =>
  match a, b with
  | ⟨ta, va⟩, ⟨tb, vb⟩ =>
    if h : ta = tb then by
      subst h
      exact match ValueType.carrierDecEq ta va vb with
        | isTrue hv => isTrue (by rw [hv])
        | isFalse hv => isFalse fun heq =>
            hv (eq_of_heq ((Value.mk.injEq _ _ _ _).mp heq).2)
    else isFalse fun heq => h ((Value.mk.injEq _ _ _ _).mp heq).1

instance : (a b : Value) → Decidable (a.vlt b) := fun a b => by
  unfold Value.vlt
  cases a.orderWord with
  | none =>
    cases b.orderWord with
    | none => exact isFalse fun h => h
    | some q => exact isFalse fun h => h
  | some p =>
    obtain ⟨e₁, w₁⟩ := p
    cases b.orderWord with
    | none => exact isFalse fun h => h
    | some q =>
      obtain ⟨e₂, w₂⟩ := q
      exact inferInstanceAs (Decidable (e₁ = e₂ ∧ w₁ < w₂))

instance : (a b : Value) → Decidable (a.vle b) := fun a b => by
  unfold Value.vle
  cases a.orderWord with
  | none =>
    cases b.orderWord with
    | none => exact isFalse fun h => h
    | some q => exact isFalse fun h => h
  | some p =>
    obtain ⟨e₁, w₁⟩ := p
    cases b.orderWord with
    | none => exact isFalse fun h => h
    | some q =>
      obtain ⟨e₂, w₂⟩ := q
      exact inferInstanceAs (Decidable (e₁ = e₂ ∧ w₁ ≤ w₂))

instance : (a : Value) → (p : Point) → Decidable (p ∈ a.points)
  | ⟨.interval .u64, iv⟩, p =>
    match p with
    | .u64 x => inferInstanceAs (Decidable (x ∈ iv.points))
    | .i64 _ => isFalse fun h => h
  | ⟨.interval .i64, iv⟩, p =>
    match p with
    | .i64 x => inferInstanceAs (Decidable (x ∈ iv.points))
    | .u64 _ => isFalse fun h => h
  | ⟨.intervalFixed .u64 _, v⟩, p =>
    match p with
    | .u64 x => inferInstanceAs (Decidable (x ∈ v.toInterval.points))
    | .i64 _ => isFalse fun h => h
  | ⟨.intervalFixed .i64 _, v⟩, p =>
    match p with
    | .i64 x => inferInstanceAs (Decidable (x ∈ v.toInterval.points))
    | .u64 _ => isFalse fun h => h
  | ⟨.bool, _⟩, _ => isFalse fun h => h
  | ⟨.u64, _⟩, _ => isFalse fun h => h
  | ⟨.i64, _⟩, _ => isFalse fun h => h
  | ⟨.str, _⟩, _ => isFalse fun h => h
  | ⟨.fixedBytes _, _⟩, _ => isFalse fun h => h

/-- Deciding an existential over an `Option` scrutinee — the shape
`cmpDen` gives `allen` and `pointIn`. -/
instance {α : Type} {P : α → Prop} [DecidablePred P] :
    (o : Option α) → Decidable (∃ x, o = some x ∧ P x)
  | some v =>
    if h : P v then isTrue ⟨v, rfl, h⟩
    else isFalse fun ⟨_, hx, hp⟩ => h (by cases hx; exact hp)
  | none => isFalse fun ⟨_, hx, _⟩ => by cases hx

instance (C : Classify) (ρ : ParamEnv) :
    (op : CmpOp) → (a b : Value) → Decidable (cmpDen C ρ op a b)
  | .eq, a, b => inferInstanceAs (Decidable (a = b))
  | .ne, a, b => inferInstanceAs (Decidable (a ≠ b))
  | .lt, a, b => inferInstanceAs (Decidable (a.vlt b))
  | .le, a, b => inferInstanceAs (Decidable (a.vle b))
  | .gt, a, b => inferInstanceAs (Decidable (b.vlt a))
  | .ge, a, b => inferInstanceAs (Decidable (b.vle a))
  | .allen m, a, b =>
    inferInstanceAs
      (Decidable (∃ rel, classifyValue C a b = some rel ∧ rel ∈ m.den ρ))
  | .pointIn, a, b =>
    inferInstanceAs (Decidable (∃ p, b.point = some p ∧ p ∈ a.points))

instance (ρ : ParamEnv) (σ : Assignment) :
    (t : Term) → (w : Value) → Decidable (Term.selects ρ σ t w)
  | .var v, w => inferInstanceAs (Decidable (σ v = w))
  | .param p, w => inferInstanceAs (Decidable (ρ.scalar p = w))
  | .paramSet p, w => inferInstanceAs (Decidable (w ∈ ρ.set p))
  | .lit c, w => inferInstanceAs (Decidable (c = w))
  | .measure v, w => inferInstanceAs (Decidable ((σ v).measure? = some w))

/-! ## The executable half — `evalList` (PRD 13's foundation)

The compositional pipeline: per-atom extension of partial assignments
(the join), the negation filter, the condition filter, the projection
— each stage's soundness is proved against its denotational
counterpart, and `eval_sound` composes them. -/

/-- A concrete finite instance: relation extensions as an association
list — the executable counterpart of `Instance` (the Tiny worlds the
conformance lane evaluates). -/
structure ListInstance where
  rels : List (RelId × List Fact)

/-- The fact list one relation carries (first entry wins; a missing
relation is empty). -/
def ListInstance.facts (W : ListInstance) (R : RelId) : List Fact :=
  match W.rels.find? fun e => e.1 == R with
  | some e => e.2
  | none => []

/-- The instance a concrete world denotes: list membership per
relation. -/
def ListInstance.den (W : ListInstance) : Instance :=
  fun R => fun f => f ∈ W.facts R

/-- The join's state: a partial assignment as an association list. -/
abbrev PartialAssign : Type := List (VarId × Value)

/-- Association-list lookup (first binding wins; the join never
shadows — it binds a variable only when unbound). -/
def lookupVar : PartialAssign → VarId → Option Value
  | [], _ => none
  | e :: σ, v => if e.1 = v then some e.2 else lookupVar σ v

/-- Totalization: bound variables read their binding; unbound ones a
dummy no `Safe` rule ever consults (every spent variable is
join-bound). -/
def totalize (σ : PartialAssign) : Assignment :=
  fun v => (lookupVar σ v).getD ⟨.bool, false⟩

/-- One binding's extension step: a bound variable CHECKS (bound var
demands equality), an unbound variable BINDS (the equality reading —
the matching equation's `σ v = w` solved for `σ v`), params, sets and
literals check their selection, and a measure binds nothing (the
validator rejects it; `eval_sound` spends `WellTyped` exactly here). -/
def bindTerm (ρ : ParamEnv) (σ : PartialAssign) (t : Term) (w : Value) :
    Option PartialAssign :=
  match t with
  | .var v =>
    match lookupVar σ v with
    | some x => if x = w then some σ else none
    | none => some ((v, w) :: σ)
  | .param p => if ρ.scalar p = w then some σ else none
  | .paramSet p => if w ∈ ρ.set p then some σ else none
  | .lit c => if c = w then some σ else none
  | .measure _ => none

/-- One atom's extension against one fact: fold `bindTerm` over the
bindings. -/
def bindAtom (ρ : ParamEnv) (f : Fact) :
    List (FieldId × Term) → PartialAssign → Option PartialAssign
  | [], σ => some σ
  | b :: bs, σ =>
    match bindTerm ρ σ b.2 (f b.1) with
    | some σ' => bindAtom ρ f bs σ'
    | none => none

/-- The join: extend every open assignment through every fact of every
positive atom, in atom order. -/
def joinAtoms (W : ListInstance) (ρ : ParamEnv) :
    List Atom → List PartialAssign → List PartialAssign
  | [], σs => σs
  | a :: rest, σs =>
    joinAtoms W ρ rest
      (σs.flatMap fun σ =>
        (W.facts a.relation).filterMap fun f => bindAtom ρ f a.bindings σ)

/-- The matching equation, decided (over a total assignment). -/
def matchesB (ρ : ParamEnv) (σ : Assignment) (a : Atom) (f : Fact) :
    Bool :=
  a.bindings.all fun b => decide (Term.selects ρ σ b.2 (f b.1))

theorem matchesB_iff {ρ : ParamEnv} {σ : Assignment} {a : Atom}
    {f : Fact} : matchesB ρ σ a f = true ↔ Matches f a σ ρ := by
  simp [matchesB, Matches, List.all_eq_true]

/-- The value list a term selects from — finite by construction: one
value for a variable, param or literal, the slice for a set, the
finite measure (if any) for a measure. -/
def Term.values (ρ : ParamEnv) (σ : Assignment) : Term → List Value
  | .var v => [σ v]
  | .param p => [ρ.scalar p]
  | .paramSet p => ρ.set p
  | .lit c => [c]
  | .measure v => ((σ v).measure?).toList

theorem mem_values_iff {ρ : ParamEnv} {σ : Assignment} {t : Term}
    {w : Value} : w ∈ Term.values ρ σ t ↔ Term.selects ρ σ t w := by
  cases t <;>
    simp [Term.values, Term.selects, eq_comm, Option.mem_toList]

/-- One comparison, decided: some candidate pair satisfies the
operator. -/
def compHoldsB (C : Classify) (ρ : ParamEnv) (σ : Assignment)
    (c : Comparison) : Bool :=
  (Term.values ρ σ c.lhs).any fun a =>
    (Term.values ρ σ c.rhs).any fun b => decide (cmpDen C ρ c.op a b)

theorem compHoldsB_iff {C : Classify} {ρ : ParamEnv} {σ : Assignment}
    {c : Comparison} :
    compHoldsB C ρ σ c = true ↔ c.holds C ρ σ := by
  simp only [compHoldsB, List.any_eq_true, decide_eq_true_eq,
    Comparison.holds]
  constructor
  · rintro ⟨a, ha, b, hb, hden⟩
    exact ⟨a, b, mem_values_iff.mp ha, mem_values_iff.mp hb, hden⟩
  · rintro ⟨a, b, ha, hb, hden⟩
    exact ⟨a, mem_values_iff.mpr ha, b, mem_values_iff.mpr hb, hden⟩

mutual
  /-- One condition tree, decided. -/
  def condHoldsB (C : Classify) (ρ : ParamEnv) (σ : Assignment) :
      Condition → Bool
    | .leaf c => compHoldsB C ρ σ c
    | .and cs => condAllB C ρ σ cs
    | .or cs => condAnyB C ρ σ cs

  /-- A condition list, decided conjunctively. -/
  def condAllB (C : Classify) (ρ : ParamEnv) (σ : Assignment) :
      List Condition → Bool
    | [] => true
    | t :: ts => condHoldsB C ρ σ t && condAllB C ρ σ ts

  /-- A condition list, decided disjunctively. -/
  def condAnyB (C : Classify) (ρ : ParamEnv) (σ : Assignment) :
      List Condition → Bool
    | [] => false
    | t :: ts => condHoldsB C ρ σ t || condAnyB C ρ σ ts
end

mutual
  theorem condHoldsB_iff (C : Classify) (ρ : ParamEnv) (σ : Assignment) :
      ∀ t : Condition, condHoldsB C ρ σ t = true ↔ Condition.holds C ρ σ t
    | .leaf c => by
      simp only [condHoldsB, Condition.holds]
      exact compHoldsB_iff
    | .and cs => by
      simp only [condHoldsB, Condition.holds]
      exact condAllB_iff C ρ σ cs
    | .or cs => by
      simp only [condHoldsB, Condition.holds]
      exact condAnyB_iff C ρ σ cs

  theorem condAllB_iff (C : Classify) (ρ : ParamEnv) (σ : Assignment) :
      ∀ cs : List Condition,
        condAllB C ρ σ cs = true ↔ Condition.allHold C ρ σ cs
    | [] => by simp [condAllB, Condition.allHold]
    | t :: ts => by
      simp [condAllB, Condition.allHold, Bool.and_eq_true,
        condHoldsB_iff C ρ σ t, condAllB_iff C ρ σ ts]

  theorem condAnyB_iff (C : Classify) (ρ : ParamEnv) (σ : Assignment) :
      ∀ cs : List Condition,
        condAnyB C ρ σ cs = true ↔ Condition.anyHold C ρ σ cs
    | [] => by simp [condAnyB, Condition.anyHold]
    | t :: ts => by
      simp [condAnyB, Condition.anyHold, Bool.or_eq_true,
        condHoldsB_iff C ρ σ t, condAnyB_iff C ρ σ ts]
end

/-- **`evalList`'s rule stage**: join, negation filter, condition
filter, projection — the naive evaluator whose stages mirror the
denotation clause for clause. -/
def evalRule (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (r : Rule) : List AnswerTuple :=
  ((joinAtoms W ρ r.atoms [[]]).filter fun σp =>
    (r.negated.all fun a =>
      (W.facts a.relation).all fun f => ! matchesB ρ (totalize σp) a f) &&
    (r.conditions.all fun t => condHoldsB C ρ (totalize σp) t)).map
    fun σp => r.finds.map (totalize σp)

/-- **`evalList`** — the executable denotation: evaluate every rule
over a concrete finite world and concatenate (list-level union;
`eval_sound` says membership agrees with `queryAnswers`, so the
concatenation IS the set union). PRD 13 runs THIS against the engine
and the naive model as the third differential oracle. -/
def evalList (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (q : Query) : List AnswerTuple :=
  q.rules.flatMap (evalRule C W ρ)

/-! ## `eval_sound` — the refinement, stage by stage -/

/-- `σ'` extends `σ`: every bound variable keeps its value (the join
only ever CONSES fresh bindings — it never shadows). -/
def ExtendsPA (σ' σ : PartialAssign) : Prop :=
  ∀ v x, lookupVar σ v = some x → lookupVar σ' v = some x

theorem ExtendsPA.rfl {σ : PartialAssign} : ExtendsPA σ σ :=
  fun _ _ h => h

theorem ExtendsPA.trans {σ₃ σ₂ σ₁ : PartialAssign}
    (h₂₁ : ExtendsPA σ₂ σ₁) (h₃₂ : ExtendsPA σ₃ σ₂) :
    ExtendsPA σ₃ σ₁ :=
  fun v x h => h₃₂ v x (h₂₁ v x h)

/-- What one successful `bindTerm` step pins into the state: the
partial-assignment form of `Term.selects`. -/
def TermPin (ρ : ParamEnv) (σ : PartialAssign) (t : Term) (w : Value) :
    Prop :=
  match t with
  | .var v => lookupVar σ v = some w
  | .param p => ρ.scalar p = w
  | .paramSet p => w ∈ ρ.set p
  | .lit c => c = w
  | .measure _ => False

theorem TermPin.mono {ρ : ParamEnv} {σ σ' : PartialAssign} {t : Term}
    {w : Value} (hext : ExtendsPA σ' σ) (h : TermPin ρ σ t w) :
    TermPin ρ σ' t w := by
  cases t with
  | var v => exact hext v w h
  | _ => exact h

/-- A pinned binding selects under the totalization — the per-term
soundness of the join. -/
theorem TermPin.selects {ρ : ParamEnv} {σ : PartialAssign} {t : Term}
    {w : Value} (h : TermPin ρ σ t w) :
    Term.selects ρ (totalize σ) t w := by
  cases t with
  | var v =>
    show totalize σ v = w
    unfold totalize
    rw [show lookupVar σ v = some w from h]
    rfl
  | measure v => exact absurd h (fun hf => hf)
  | _ => exact h

/-- `bindTerm` soundness: a successful step extends the state and pins
the binding. -/
theorem bindTerm_sound {ρ : ParamEnv} {σ σ' : PartialAssign} {t : Term}
    {w : Value} (h : bindTerm ρ σ t w = some σ') :
    ExtendsPA σ' σ ∧ TermPin ρ σ' t w := by
  cases t with
  | var v =>
    cases hlk : lookupVar σ v with
    | some x =>
      simp only [bindTerm, hlk] at h
      by_cases hxw : x = w
      · rw [if_pos hxw] at h
        cases h
        exact ⟨ExtendsPA.rfl, by rw [← hxw]; exact hlk⟩
      · rw [if_neg hxw] at h
        cases h
    | none =>
      simp only [bindTerm, hlk] at h
      cases h
      constructor
      · intro u y hy
        show (if v = u then some w else lookupVar σ u) = some y
        by_cases hvu : v = u
        · subst hvu
          rw [hlk] at hy
          cases hy
        · rw [if_neg hvu]
          exact hy
      · show (if v = v then some w else lookupVar σ v) = some w
        rw [if_pos rfl]
  | param p =>
    simp only [bindTerm] at h
    by_cases hc : ρ.scalar p = w
    · rw [if_pos hc] at h; cases h; exact ⟨ExtendsPA.rfl, hc⟩
    · rw [if_neg hc] at h; cases h
  | paramSet p =>
    simp only [bindTerm] at h
    by_cases hc : w ∈ ρ.set p
    · rw [if_pos hc] at h; cases h; exact ⟨ExtendsPA.rfl, hc⟩
    · rw [if_neg hc] at h; cases h
  | lit c =>
    simp only [bindTerm] at h
    by_cases hc : c = w
    · rw [if_pos hc] at h; cases h; exact ⟨ExtendsPA.rfl, hc⟩
    · rw [if_neg hc] at h; cases h
  | measure v =>
    simp only [bindTerm] at h
    cases h

/-- `bindAtom` soundness: a successful atom extension extends the
state and pins every binding. -/
theorem bindAtom_sound {ρ : ParamEnv} {f : Fact} :
    ∀ (bs : List (FieldId × Term)) (σ σ' : PartialAssign),
      bindAtom ρ f bs σ = some σ' →
      ExtendsPA σ' σ ∧ ∀ b, b ∈ bs → TermPin ρ σ' b.2 (f b.1)
  | [], σ, σ', h => by
    cases h
    exact ⟨ExtendsPA.rfl, fun b hb => absurd hb (by simp)⟩
  | b :: bs, σ, σ', h => by
    simp only [bindAtom] at h
    cases hbt : bindTerm ρ σ b.2 (f b.1) with
    | none => rw [hbt] at h; cases h
    | some σ₁ =>
      rw [hbt] at h
      obtain ⟨hext₁, hpin₁⟩ := bindTerm_sound hbt
      obtain ⟨hext₂, hpins⟩ := bindAtom_sound bs σ₁ σ' h
      refine ⟨hext₁.trans hext₂, fun b' hb' => ?_⟩
      rcases List.mem_cons.mp hb' with rfl | hmem
      · exact hpin₁.mono hext₂
      · exact hpins b' hmem

/-- The join's soundness: every produced state came from an input
state, extends it, and pins a matching fact for every atom — under
the FINAL state, because extension preserves pins. -/
theorem joinAtoms_sound {W : ListInstance} {ρ : ParamEnv} :
    ∀ (atoms : List Atom) (σs : List PartialAssign)
      (σp : PartialAssign), σp ∈ joinAtoms W ρ atoms σs →
      ∃ σ₀, σ₀ ∈ σs ∧ ExtendsPA σp σ₀ ∧
        ∀ a, a ∈ atoms → ∃ f, f ∈ W.facts a.relation ∧
          ∀ b, b ∈ a.bindings → TermPin ρ σp b.2 (f b.1)
  | [], σs, σp, h => by
    refine ⟨σp, h, ExtendsPA.rfl, fun a ha => absurd ha (by simp)⟩
  | a :: rest, σs, σp, h => by
    simp only [joinAtoms] at h
    obtain ⟨σ₁, hσ₁, hext, hrest⟩ := joinAtoms_sound rest _ σp h
    obtain ⟨σ₀, hσ₀, hfm⟩ := List.mem_flatMap.mp hσ₁
    obtain ⟨f, hf, hbind⟩ := List.mem_filterMap.mp hfm
    obtain ⟨hext₀, hpins⟩ := bindAtom_sound a.bindings σ₀ σ₁ hbind
    refine ⟨σ₀, hσ₀, hext₀.trans hext, fun a' ha' => ?_⟩
    rcases List.mem_cons.mp ha' with rfl | hmem
    · exact ⟨f, hf, fun b hb => (hpins b hb).mono hext⟩
    · exact hrest a' hmem

/-- Rule-stage soundness: everything `evalRule` emits is a
denotational answer — unconditionally (the join binds only what facts
carry; the filters decide exactly the denotation's clauses). -/
theorem evalRule_sound {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {r : Rule} {t : AnswerTuple}
    (h : t ∈ evalRule C W ρ r) : t ∈ ruleAnswers C r W.den ρ := by
  obtain ⟨σp, hσp, rfl⟩ := List.mem_map.mp h
  obtain ⟨hjoin, hchecks⟩ := List.mem_filter.mp hσp
  rw [Bool.and_eq_true] at hchecks
  obtain ⟨hnegB, hcondB⟩ := hchecks
  obtain ⟨-, -, -, hpins⟩ := joinAtoms_sound r.atoms [[]] σp hjoin
  refine mem_ruleAnswers.mpr ⟨totalize σp, ⟨?_, ?_, ?_⟩, rfl⟩
  · intro a ha
    obtain ⟨f, hf, hpin⟩ := hpins a ha
    exact ⟨f, hf, fun b hb => (hpin b hb).selects⟩
  · intro a ha hex
    obtain ⟨f, hf, hm⟩ := hex
    have hall := List.all_eq_true.mp hnegB a ha
    have hfall := List.all_eq_true.mp hall f hf
    rw [Bool.not_eq_true'] at hfall
    exact absurd (matchesB_iff.mpr hm) (by rw [hfall]; simp)
  · intro c hc
    exact (condHoldsB_iff C ρ (totalize σp) c).mp
      (List.all_eq_true.mp hcondB c hc)

/-- `σp` agrees with the total `σ`: every bound variable carries `σ`'s
value — the completeness invariant. -/
def AgreesPA (σp : PartialAssign) (σ : Assignment) : Prop :=
  ∀ v x, lookupVar σp v = some x → σ v = x

/-- `bindTerm` completeness: a selecting, measure-free binding always
extends an agreeing state to an agreeing state. -/
theorem bindTerm_complete {ρ : ParamEnv} {σp : PartialAssign}
    {σ : Assignment} {t : Term} {w : Value} (hag : AgreesPA σp σ)
    (hsel : Term.selects ρ σ t w) (hnm : ¬ t.isMeasure) :
    ∃ σ', bindTerm ρ σp t w = some σ' ∧ AgreesPA σ' σ ∧
      ExtendsPA σ' σp := by
  cases t with
  | var v =>
    have hσv : σ v = w := hsel
    cases hlk : lookupVar σp v with
    | some x =>
      have hxw : x = w := (hag v x hlk).symm.trans hσv
      refine ⟨σp, ?_, hag, ExtendsPA.rfl⟩
      simp only [bindTerm, hlk]
      rw [if_pos hxw]
    | none =>
      refine ⟨(v, w) :: σp, ?_, ?_, ?_⟩
      · simp only [bindTerm, hlk]
      · intro u y hy
        have : (if v = u then some w else lookupVar σp u) = some y := hy
        by_cases hvu : v = u
        · rw [if_pos hvu] at this
          cases this
          exact hvu ▸ hσv
        · rw [if_neg hvu] at this
          exact hag u y this
      · intro u y hy
        show (if v = u then some w else lookupVar σp u) = some y
        by_cases hvu : v = u
        · subst hvu
          rw [hlk] at hy
          cases hy
        · rw [if_neg hvu]
          exact hy
  | param p =>
    exact ⟨σp, by
      simp only [bindTerm]
      rw [if_pos (show ρ.scalar p = w from hsel)], hag, ExtendsPA.rfl⟩
  | paramSet p =>
    exact ⟨σp, by
      simp only [bindTerm]
      rw [if_pos (show w ∈ ρ.set p from hsel)], hag, ExtendsPA.rfl⟩
  | lit c =>
    exact ⟨σp, by
      simp only [bindTerm]
      rw [if_pos (show c = w from hsel)], hag, ExtendsPA.rfl⟩
  | measure v => exact absurd trivial hnm

/-- `bindAtom` completeness. -/
theorem bindAtom_complete {ρ : ParamEnv} {f : Fact} {σ : Assignment} :
    ∀ (bs : List (FieldId × Term)) (σp : PartialAssign),
      AgreesPA σp σ →
      (∀ b, b ∈ bs → Term.selects ρ σ b.2 (f b.1)) →
      (∀ b, b ∈ bs → ¬ b.2.isMeasure) →
      ∃ σ', bindAtom ρ f bs σp = some σ' ∧ AgreesPA σ' σ ∧
        ExtendsPA σ' σp
  | [], σp, hag, _, _ => ⟨σp, rfl, hag, ExtendsPA.rfl⟩
  | b :: bs, σp, hag, hsel, hnm => by
    obtain ⟨σ₁, hb₁, hag₁, hext₁⟩ :=
      bindTerm_complete hag (hsel b (List.mem_cons_self ..))
        (hnm b (List.mem_cons_self ..))
    obtain ⟨σ', hb', hag', hext'⟩ :=
      bindAtom_complete bs σ₁ hag₁
        (fun b' hb => hsel b' (List.mem_cons_of_mem _ hb))
        (fun b' hb => hnm b' (List.mem_cons_of_mem _ hb))
    refine ⟨σ', ?_, hag', hext₁.trans hext'⟩
    simp only [bindAtom]
    rw [hb₁]
    exact hb'

/-- The join's completeness: a deriving assignment is realized by some
produced state agreeing with it. -/
theorem joinAtoms_complete {W : ListInstance} {ρ : ParamEnv}
    {σ : Assignment} :
    ∀ (atoms : List Atom) (σs : List PartialAssign)
      (σ₀ : PartialAssign), σ₀ ∈ σs → AgreesPA σ₀ σ →
      (∀ a, a ∈ atoms → ∃ f, f ∈ W.facts a.relation ∧ Matches f a σ ρ) →
      (∀ a, a ∈ atoms → ∀ b, b ∈ a.bindings → ¬ b.2.isMeasure) →
      ∃ σp, σp ∈ joinAtoms W ρ atoms σs ∧ AgreesPA σp σ
  | [], σs, σ₀, hmem, hag, _, _ => ⟨σ₀, hmem, hag⟩
  | a :: rest, σs, σ₀, hmem, hag, hatoms, hnm => by
    obtain ⟨f, hf, hm⟩ := hatoms a (List.mem_cons_self ..)
    obtain ⟨σ₁, hb₁, hag₁, -⟩ :=
      bindAtom_complete a.bindings σ₀ hag (fun b hb => hm b hb)
        (hnm a (List.mem_cons_self ..) )
    have hσ₁ : σ₁ ∈ σs.flatMap fun σ' =>
        (W.facts a.relation).filterMap fun f' => bindAtom ρ f' a.bindings σ' :=
      List.mem_flatMap.mpr ⟨σ₀, hmem,
        List.mem_filterMap.mpr ⟨f, hf, hb₁⟩⟩
    obtain ⟨σp, hσp, hagp⟩ :=
      joinAtoms_complete rest _ σ₁ hσ₁ hag₁
        (fun a' ha' => hatoms a' (List.mem_cons_of_mem _ ha'))
        (fun a' ha' => hnm a' (List.mem_cons_of_mem _ ha'))
    exact ⟨σp, by simpa only [joinAtoms] using hσp, hagp⟩

/-- Every produced state covers the rule's positively bound variables
— read off the join's own soundness. -/
theorem joinAtoms_covers {W : ListInstance} {ρ : ParamEnv}
    {atoms : List Atom} {σs : List PartialAssign}
    {σp : PartialAssign} (h : σp ∈ joinAtoms W ρ atoms σs) :
    ∀ v, (∃ a, a ∈ atoms ∧ v ∈ a.boundVars) →
      (lookupVar σp v).isSome := by
  rintro v ⟨a, ha, hv⟩
  obtain ⟨-, -, -, hpins⟩ := joinAtoms_sound atoms σs σp h
  obtain ⟨b, hb, hvb⟩ := List.mem_flatMap.mp hv
  obtain ⟨f, -, hpin⟩ := hpins a ha
  have := hpin b hb
  rw [Term.mem_bindingVars.mp hvb] at this
  rw [show lookupVar σp v = some (f b.1) from this]
  rfl

/-- Agreement cashes into the totalization on covered variables. -/
theorem totalize_agrees {σp : PartialAssign} {σ : Assignment}
    (hag : AgreesPA σp σ) {v : VarId}
    (hsome : (lookupVar σp v).isSome) : totalize σp v = σ v := by
  cases h : lookupVar σp v with
  | none => rw [h] at hsome; cases hsome
  | some x =>
    show (lookupVar σp v).getD ⟨.bool, false⟩ = σ v
    rw [h]
    exact (hag v x h).symm

/-! ### Congruence — satisfaction reads only a construct's variables -/

theorem selects_congr {ρ : ParamEnv} {σ σ' : Assignment} {t : Term}
    {w : Value} (h : ∀ v, v ∈ t.vars → σ v = σ' v) :
    Term.selects ρ σ t w ↔ Term.selects ρ σ' t w := by
  cases t with
  | var v =>
    have := h v (by simp [Term.vars])
    show σ v = w ↔ σ' v = w
    rw [this]
  | measure v =>
    have := h v (by simp [Term.vars])
    show (σ v).measure? = some w ↔ (σ' v).measure? = some w
    rw [this]
  | _ => exact Iff.rfl

theorem matches_congr {f : Fact} {a : Atom} {σ σ' : Assignment}
    {ρ : ParamEnv} (h : ∀ v, v ∈ a.vars → σ v = σ' v) :
    Matches f a σ ρ ↔ Matches f a σ' ρ := by
  unfold Matches
  constructor <;> intro hm b hb <;>
    refine (selects_congr fun v hv => ?_).mp (hm b hb)
  · exact h v (List.mem_flatMap.mpr ⟨b, hb, hv⟩)
  · exact (h v (List.mem_flatMap.mpr ⟨b, hb, hv⟩)).symm

theorem compHolds_congr {C : Classify} {ρ : ParamEnv}
    {σ σ' : Assignment} {c : Comparison}
    (h : ∀ v, v ∈ c.vars → σ v = σ' v) :
    c.holds C ρ σ ↔ c.holds C ρ σ' := by
  unfold Comparison.holds
  have hl : ∀ v, v ∈ c.lhs.vars → σ v = σ' v := fun v hv =>
    h v (List.mem_append.mpr (Or.inl hv))
  have hr : ∀ v, v ∈ c.rhs.vars → σ v = σ' v := fun v hv =>
    h v (List.mem_append.mpr (Or.inr hv))
  constructor
  · rintro ⟨a, b, ha, hb, hden⟩
    exact ⟨a, b, (selects_congr hl).mp ha, (selects_congr hr).mp hb, hden⟩
  · rintro ⟨a, b, ha, hb, hden⟩
    exact ⟨a, b, (selects_congr hl).mpr ha, (selects_congr hr).mpr hb,
      hden⟩

mutual
  theorem condHolds_congr (C : Classify) (ρ : ParamEnv)
      (σ σ' : Assignment) :
      ∀ t : Condition, (∀ v, v ∈ t.vars → σ v = σ' v) →
        (Condition.holds C ρ σ t ↔ Condition.holds C ρ σ' t)
    | .leaf c, h => by
      simp only [Condition.holds]
      exact compHolds_congr fun v hv => h v (by
        simpa [Condition.vars] using hv)
    | .and cs, h => by
      simp only [Condition.holds]
      exact condAllHold_congr C ρ σ σ' cs fun v hv => h v (by
        simpa [Condition.vars] using hv)
    | .or cs, h => by
      simp only [Condition.holds]
      exact condAnyHold_congr C ρ σ σ' cs fun v hv => h v (by
        simpa [Condition.vars] using hv)

  theorem condAllHold_congr (C : Classify) (ρ : ParamEnv)
      (σ σ' : Assignment) :
      ∀ cs : List Condition, (∀ v, v ∈ Condition.varsList cs → σ v = σ' v) →
        (Condition.allHold C ρ σ cs ↔ Condition.allHold C ρ σ' cs)
    | [], _ => Iff.rfl
    | t :: ts, h => by
      simp only [Condition.allHold]
      rw [condHolds_congr C ρ σ σ' t fun v hv => h v (by
            simp only [Condition.varsList, List.mem_append]
            exact Or.inl hv),
          condAllHold_congr C ρ σ σ' ts fun v hv => h v (by
            simp only [Condition.varsList, List.mem_append]
            exact Or.inr hv)]

  theorem condAnyHold_congr (C : Classify) (ρ : ParamEnv)
      (σ σ' : Assignment) :
      ∀ cs : List Condition, (∀ v, v ∈ Condition.varsList cs → σ v = σ' v) →
        (Condition.anyHold C ρ σ cs ↔ Condition.anyHold C ρ σ' cs)
    | [], _ => Iff.rfl
    | t :: ts, h => by
      simp only [Condition.anyHold]
      rw [condHolds_congr C ρ σ σ' t fun v hv => h v (by
            simp only [Condition.varsList, List.mem_append]
            exact Or.inl hv),
          condAnyHold_congr C ρ σ σ' ts fun v hv => h v (by
            simp only [Condition.varsList, List.mem_append]
            exact Or.inr hv)]
end

/-- Rule-stage completeness: under `Safe` (every spent variable is
join-bound) and the binding shape discipline (`WellTyped`'s
measure-free bindings — the join cannot bind through a computation),
every denotational answer is emitted. -/
theorem evalRule_complete {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {r : Rule} {t : AnswerTuple} (hsafe : Safe r)
    (hnm : ∀ a, a ∈ r.atoms → ∀ b, b ∈ a.bindings → ¬ b.2.isMeasure)
    (h : t ∈ ruleAnswers C r W.den ρ) : t ∈ evalRule C W ρ r := by
  obtain ⟨σ, ⟨hatoms, hneg, hconds⟩, rfl⟩ := mem_ruleAnswers.mp h
  obtain ⟨σp, hjoin, hag⟩ :=
    joinAtoms_complete r.atoms [[]] [] (by simp)
      (fun v x hx => by cases hx) hatoms hnm
  have hcov := joinAtoms_covers hjoin
  have hagree : ∀ v, v ∈ r.positiveVars → totalize σp v = σ v :=
    fun v hv => totalize_agrees hag (hcov v (mem_positiveVars.mp hv))
  refine List.mem_map.mpr ⟨σp, List.mem_filter.mpr ⟨hjoin, ?_⟩, ?_⟩
  · rw [Bool.and_eq_true]
    constructor
    · refine List.all_eq_true.mpr fun a ha => ?_
      refine List.all_eq_true.mpr fun f hf => ?_
      rw [Bool.not_eq_true']
      cases hb : matchesB ρ (totalize σp) a f with
      | false => rfl
      | true =>
        have hm : Matches f a (totalize σp) ρ := matchesB_iff.mp hb
        have hm' : Matches f a σ ρ :=
          (matches_congr fun v hv => hagree v (hsafe v
            (mem_allVars.mpr (Or.inr (Or.inr (Or.inl ⟨a, ha, hv⟩)))))).mp
            hm
        exact absurd ⟨f, hf, hm'⟩ (hneg a ha)
    · refine List.all_eq_true.mpr fun c hc => ?_
      exact (condHoldsB_iff C ρ (totalize σp) c).mpr
        ((condHolds_congr C ρ (totalize σp) σ c fun v hv =>
          hagree v (hsafe v (mem_allVars.mpr
            (Or.inr (Or.inr (Or.inr ⟨c, hc, hv⟩)))))).mpr (hconds c hc))
  · refine List.map_congr_left fun v hv => ?_
    exact hagree v (hsafe v (mem_allVars.mpr (Or.inl hv)))

/-- **`eval_sound` — the refinement theorem (PRD 13's foundation).**
List-backed evaluation over a concrete finite world agrees with the
`Set` denotation, membership for membership: `evalRule`'s stages are
sound unconditionally, and complete under `Safe` (positive range
restriction — the join can only enumerate what positive atoms bind)
plus the binding shape discipline the validator enforces
(`WellTyped`: no measure in a binding). These two hypotheses are
exactly the acceptance rules — the theorem names the premises the
engine's validator discharges, which is the covenant's Level-1
pattern.
Bridge: PRD 13 runs `evalList` on Tiny worlds as the third
differential oracle against `crate::exec` and the naive model. -/
theorem eval_sound {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {q : Query} (hsafe : ∀ r, r ∈ q.rules → Safe r)
    (hwt : ∀ r, r ∈ q.rules → r.WellTyped) :
    ∀ t, t ∈ evalList C W ρ q ↔ t ∈ queryAnswers C q W.den ρ := by
  intro t
  simp only [evalList, List.mem_flatMap]
  constructor
  · rintro ⟨r, hr, ht⟩
    exact mem_queryAnswers.mpr ⟨r, hr, evalRule_sound ht⟩
  · intro ht
    obtain ⟨r, hr, hta⟩ := mem_queryAnswers.mp ht
    exact ⟨r, hr, evalRule_complete (hsafe r hr)
      (fun a ha b hb => (hwt r hr).1 a (Or.inl ha) b hb) hta⟩

/-- The finiteness the safety rule buys, cashed through the executable
half: over a concrete finite world, a safe well-shaped query's answer
set carries the listability token — `evalList` itself is the witness
list. The unsafe converse is `Countermodels.unsafe_rule_infinite`. -/
theorem answers_finite_of_safe {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {q : Query} (hsafe : ∀ r, r ∈ q.rules → Safe r)
    (hwt : ∀ r, r ∈ q.rules → r.WellTyped) :
    (queryAnswers C q W.den ρ).Finite :=
  ⟨evalList C W ρ q, fun t => (eval_sound hsafe hwt t).symm⟩

end Query
end Bumbledb
