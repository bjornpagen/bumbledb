import Bumbledb.Query.Denotation

/-!
# Membership lowering — the bivalent surface binding (Level 0)

The surface rule (`docs/architecture/20-query-ir.md`, § membership is
a typing rule): an element-typed term at an interval-typed field
position means POINT MEMBERSHIP (half-open — `points_halfopen` carries
the reading); an interval-typed term at the same position is value
equality. This module makes that bivalent reading a JUDGMENT
(`SurfaceMatches`, over `Term.selectsAt`) and PROVES the lowering to
the pre-lowered form `Matches` reads — an interval-variable binding
plus a `PointIn` condition — answer-preserving
(`membership_lowering_preserves`). The engine's normalize
(`ir/normalize/normalize.rs::lower_atom` over `is_membership`, on the
types `ir/validate/context.rs::resolve_bivalents` resolved) and the
naive model re-derive this lowering independently; this theorem is the
arbiter both are measured against.

The roster covered: `var` / `param` / `lit` terms and `paramSet`
(any element satisfying both — the `AnyPointIn` reading;
`paramSet_selects_membership` composes), positive AND negated atoms
(the surface judgment reads both; see the narrowing below for which
half is syntactic), and repeated variables. Repeated variables need no
special case at this level: the surface reading is per-binding over
ONE total assignment, so the engine's slot discipline — the variable
position is the first DOMAIN binding; membership occurrences bind
nothing (`normalize.rs` pass 1) — is answer-invisible, and
`repeated_membership_same_fact` is exactly the same-fact composition
(`FieldsPointIn`) the engine lowers a scalar-anchored repeat to.

## Narrowings recorded (law 5: narrow and record)

* **The typing witness is a parameter** (`Typing`): the resolved
  var/param types the membership rule consults are taken as given —
  the validator's typed pass computes them
  (`ir/validate/context.rs::resolve_bivalents`); nothing here
  re-derives resolution order. On accepted rules the witness is
  exactly that resolution.
* **The measure arm is parked on the equality reading**
  (`Typing.termInterval` answers `true` for a measure): a measure
  never appears in a binding (`ValidationError::DurationInBinding`),
  so the arm is unreachable on accepted rules and the parking only
  keeps the definition total.
* **Negated membership atoms have no pre-lowered RULE form.** The
  positive lowering names the interval field's value with a fresh
  variable; a fresh variable under negation is unsafe by construction
  (`NegatedVariableUnbound` — `Safe` forbids it), so no rule of the
  modeled syntax expresses a negated membership binding after
  lowering. Its lowered home is therefore NOT a rule but the
  anti-probe OCCURRENCE the engine actually executes
  (`ir/normalize/normalize.rs::lower_atom`, "positive or negated —
  the rules are identical"; the `AntiProbe` descriptor carries the
  membership filters, evaluated inside the probe). **The formerly
  recorded remaining gap, closed (2026-07-14, the admission-calculus
  docket):** `AntiOccurrence` is the filter-carrying negated
  judgment, `Atom.lowerNegated` the role-blind lowering onto it, and
  `membership_lowering_preserves_negated` states answer preservation
  for the FULL roster — negated membership bindings included — with
  no membership-free hypothesis: the lowered rule's positive atoms
  read the pre-lowered `Matches`/`PointIn` form and its negated atoms
  reject by anti-probe (`antiProbeRuleAnswers`).
  `antiprobe_eq_antijoin_of_negFree` composes the anti-probe
  denotation back to the plain anti-join on membership-free negated
  atoms, so `membership_lowering_preserves` — the pre-lowered-rule
  fragment, which keeps its hypothesis because it speaks the rule
  syntax — is the composition of the two. No safety subtlety
  appeared: assignments are total at this level and `Safe` pins every
  negated-atom variable positively, so the anti-probe's `PointIn`
  filters only ever read bound values — exactly the engine's
  point-membership scan.
* **The fresh mint is the canonical ceiling supply**
  (`Rule.freshVar`). Answers are projection-determined, so the
  variable names the lowering introduces are answer-invisible; the
  engine introduces none at all (its filters read fields in place),
  which is exactly why the modeled IR must mint one — a condition can
  only read the assignment.
* **The lowering is per-rule.** `VarId`s are rule-scoped, so the
  typing witness and the mint are too; `queryAnswers` is the rules'
  union (`mem_queryAnswers`), and the query-level statement is the
  pointwise lift of the rule-level theorem.
* **The fold-level companion, DISCHARGED (2026-07-23 audit, finding
  087; the R2 fold-domain docket).** A membership term SELECTS, it
  never binds: the aggregate fold domain of a lowered membership rule
  is the SURFACE rule's distinct binding set — the minted interval
  variable is no fold slot (the aggregation contract,
  `20-query-ir.md`: every aggregate folds the query's distinct full
  bindings; the mint is answer-invisible, and it is fold-invisible on
  the same ground). The lowering is now proved at the BINDING level
  (`stepLower_derives_forward` / `stepLower_derives_backward`,
  iterated by `lowerFuel_derives_forward` /
  `lowerFuel_derives_backward` — forward extends at the mints alone,
  agreeing on every written variable; backward reuses the assignment
  unchanged), and `membership_lowering_preserves_fold`
  (`Exec/Dedup.lean` — where the fold domains live) composes them:
  aggregates over the lowered rule with the mint projected away equal
  aggregates over the surface reading, fiber for fiber, uniformly in
  the fold. The conformance glue folds the surface width (the
  serializer's `"width"` key, `Conformance.lean`) and the corpus
  fence (`Exclusion::AggregateMembership`) lifts, so the third oracle
  adjudicates membership-under-additive-fold instead of excluding it.
-/

namespace Bumbledb

/-! ## Type observers — the static half of the membership rule -/

/-- Whether a value type is interval-shaped — the discriminator the
membership typing rule reads on both the field side and the term side
(`ir/normalize/normalize.rs::is_membership`). -/
def ValueType.isInterval : ValueType → Bool
  | .interval _ => true
  | .intervalFixed _ _ => true
  | _ => false

/-- The declared type of field `i` of relation `R` — total via a
scalar default: an out-of-signature position reads `.bool`, never an
interval, so it can never trigger the membership arm (acceptance makes
such positions unreachable anyway). -/
def Header.fieldType (h : Header) (R : RelId) (i : FieldId) : ValueType :=
  ((h.sig R)[i.id]?).getD .bool

/-- `Header.isInterval` and the interval-shape of `Header.fieldType`
agree — the two reads of one signature entry. -/
theorem Header.fieldType_isInterval {h : Header} {R : RelId}
    {i : FieldId} :
    h.isInterval R i = true ↔ (h.fieldType R i).isInterval = true := by
  unfold Header.isInterval Header.fieldType
  cases hx : (h.sig R)[i.id]? with
  | none => simp [ValueType.isInterval]
  | some ty => cases ty <;> simp [ValueType.isInterval]

/-- Point membership of a value pair: `x`'s point lies in `w`'s
point-family — the value-level reading of the membership binding, and
definitionally the `pointIn` comparison with `w` on the interval side
(`pointMem_iff_pointIn`). -/
def Value.pointMem (x w : Value) : Prop :=
  ∃ p, x.point = some p ∧ p ∈ w.points

/-- **The fixed-width membership lowering (u64):** an element-typed
value at an `interval<u64, w>` position reads point membership in
the DERIVED half-open `[s, s + w)` — the same `pointMem` reading
every interval position gets (`Header.isInterval` answers `true` for
the family, so `Typing.membership` fires unchanged); the width is
the type's, never the bytes'. -/
theorem pointMem_fixed_u64 {w : Nat} (x : U64) (v : FixedU64 w) :
    Value.pointMem ⟨.u64, x⟩ ⟨.intervalFixed .u64 w, v⟩ ↔
      v.val ≤ x ∧ x.val < v.val.val + w := by
  constructor
  · rintro ⟨p, hp, hmem⟩
    cases hp
    exact hmem
  · intro h
    exact ⟨.u64 x, rfl, h⟩

/-- **The fixed-width membership lowering (i64 companion).** -/
theorem pointMem_fixed_i64 {w : Nat} (x : I64) (v : FixedI64 w) :
    Value.pointMem ⟨.i64, x⟩ ⟨.intervalFixed .i64 w, v⟩ ↔
      v.val ≤ x ∧ x.val < v.val.val + (w : Int) := by
  constructor
  · rintro ⟨p, hp, hmem⟩
    cases hp
    exact hmem
  · intro h
    exact ⟨.i64 x, rfl, h⟩

namespace Query

/-! ## The resolved typing — the validator's witness -/

/-- The typed pass's resolution, as data: the schema's field
signature plus the resolved variable and parameter types — what
`ir/validate/context.rs::resolve_bivalents` computes and
`ir/normalize/normalize.rs::lower_atom` consumes. The model takes it
as a parameter (recorded narrowing). -/
structure Typing where
  header : Header
  var : VarId → ValueType
  param : ParamId → ValueType

/-- Whether a term's resolved type is interval-shaped — the term side
of `is_membership`. A `paramSet` holds points (the validator anchors
the set's element type), never an interval; a measure answers `true`
so its arm keeps the equality (degenerate) reading — a measure never
appears in a binding (recorded narrowing, `DurationInBinding`). -/
def Typing.termInterval (Γ : Typing) : Term → Bool
  | .var v => (Γ.var v).isInterval
  | .param p => (Γ.param p).isInterval
  | .paramSet _ => false
  | .lit c => c.type.isInterval
  | .measure _ => true

/-- THE membership typing rule (`is_membership`): an interval-typed
field position read by an element-typed term. -/
def Typing.membership (Γ : Typing) (R : RelId) (i : FieldId)
    (t : Term) : Bool :=
  Γ.header.isInterval R i && !(Γ.termInterval t)

/-- Rewriting the typing at one variable — the mint's footprint: the
lowering types its fresh variable at the lowered field. -/
def Typing.updateVar (Γ : Typing) (u : VarId) (ty : ValueType) :
    Typing :=
  { Γ with var := fun v => if v = u then ty else Γ.var v }

/-- The term side of the membership rule reads only the term's own
variables: updating the typing at an absent variable changes
nothing. -/
theorem Typing.termInterval_updateVar {Γ : Typing} {u : VarId}
    {ty : ValueType} {t : Term} (hne : ∀ v, v ∈ t.vars → v ≠ u) :
    (Γ.updateVar u ty).termInterval t = Γ.termInterval t := by
  cases t with
  | var v =>
    have hv : v ≠ u := hne v (List.mem_cons_self ..)
    simp [Typing.termInterval, Typing.updateVar, hv]
  | param p => rfl
  | paramSet p => rfl
  | lit c => rfl
  | measure v => rfl

/-- Membership status is stable under a typing update at an absent
variable. -/
theorem Typing.membership_updateVar {Γ : Typing} {u : VarId}
    {ty : ValueType} {R : RelId} {i : FieldId} {t : Term}
    (hne : ∀ v, v ∈ t.vars → v ≠ u) :
    (Γ.updateVar u ty).membership R i t = Γ.membership R i t := by
  unfold Typing.membership
  rw [Typing.termInterval_updateVar hne]
  rfl

/-! ## The bivalent surface selection -/

/-- What a term SELECTS at a field position under the SURFACE reading:
a membership position (interval field, element-typed term) selects
point membership of the term's value — any selected value whose point
lies in the fact's interval — and every other position selects exactly
`Term.selects` (value reading). This is the binding rule of
`docs/architecture/20-query-ir.md` § membership, as a judgment.
Bridge: `crate::ir::Atom::bindings` under the resolved types;
`ir/normalize/normalize.rs::lower_atom` pass 2. -/
def Term.selectsAt (Γ : Typing) (ρ : ParamEnv) (σ : Assignment)
    (R : RelId) (i : FieldId) (t : Term) (w : Value) : Prop :=
  if Γ.membership R i t = true then
    ∃ x, Term.selects ρ σ t x ∧ x.pointMem w
  else Term.selects ρ σ t w

/-- A non-membership position keeps the matching equation's value
reading. -/
theorem selectsAt_of_not_membership {Γ : Typing} {ρ : ParamEnv}
    {σ : Assignment} {R : RelId} {i : FieldId} {t : Term} {w : Value}
    (hm : Γ.membership R i t = false) :
    Term.selectsAt Γ ρ σ R i t w ↔ Term.selects ρ σ t w := by
  unfold Term.selectsAt
  rw [if_neg (by simp [hm])]

/-- A membership position selects point membership of any selected
value. -/
theorem selectsAt_of_membership {Γ : Typing} {ρ : ParamEnv}
    {σ : Assignment} {R : RelId} {i : FieldId} {t : Term} {w : Value}
    (hm : Γ.membership R i t = true) :
    Term.selectsAt Γ ρ σ R i t w ↔
      ∃ x, Term.selects ρ σ t x ∧ x.pointMem w := by
  unfold Term.selectsAt
  rw [if_pos hm]

/-- `Value.pointMem` IS the `pointIn` comparison's denotation, interval
side on the right — the definitional seam between the binding form and
the predicate form. -/
theorem pointMem_iff_pointIn (C : Classify) (ρ : ParamEnv)
    {x w : Value} : x.pointMem w ↔ cmpDen C ρ .pointIn w x :=
  Iff.rfl

/-! ### The roster, term by term -/

/-- An element-typed VARIABLE at an interval field reads point
membership of its assigned value. -/
theorem selectsAt_var_membership {Γ : Typing} {ρ : ParamEnv}
    {σ : Assignment} {R : RelId} {i : FieldId} {v : VarId} {w : Value}
    (hm : Γ.membership R i (.var v) = true) :
    Term.selectsAt Γ ρ σ R i (.var v) w ↔ (σ v).pointMem w := by
  rw [selectsAt_of_membership hm]
  constructor
  · rintro ⟨x, hx, hmem⟩
    exact (show σ v = x from hx) ▸ hmem
  · intro hmem
    exact ⟨σ v, rfl, hmem⟩

/-- An element-typed PARAM at an interval field reads point membership
of the bind-time value. -/
theorem selectsAt_param_membership {Γ : Typing} {ρ : ParamEnv}
    {σ : Assignment} {R : RelId} {i : FieldId} {p : ParamId} {w : Value}
    (hm : Γ.membership R i (.param p) = true) :
    Term.selectsAt Γ ρ σ R i (.param p) w ↔ (ρ.scalar p).pointMem w := by
  rw [selectsAt_of_membership hm]
  constructor
  · rintro ⟨x, hx, hmem⟩
    exact (show ρ.scalar p = x from hx) ▸ hmem
  · intro hmem
    exact ⟨ρ.scalar p, rfl, hmem⟩

/-- An element-typed LITERAL at an interval field reads point
membership of the literal. -/
theorem selectsAt_lit_membership {Γ : Typing} {ρ : ParamEnv}
    {σ : Assignment} {R : RelId} {i : FieldId} {c : Value} {w : Value}
    (hm : Γ.membership R i (.lit c) = true) :
    Term.selectsAt Γ ρ σ R i (.lit c) w ↔ c.pointMem w := by
  rw [selectsAt_of_membership hm]
  constructor
  · rintro ⟨x, hx, hmem⟩
    exact (show c = x from hx) ▸ hmem
  · intro hmem
    exact ⟨c, rfl, hmem⟩

/-- A PARAM SET at an interval field reads "any element's point lies
in the field's interval" — the `AnyPointIn` reading;
`paramSet_selects_membership` is the value-reading companion this
composes with. -/
theorem selectsAt_paramSet_membership {Γ : Typing} {ρ : ParamEnv}
    {σ : Assignment} {R : RelId} {i : FieldId} {p : ParamId} {w : Value}
    (hm : Γ.membership R i (.paramSet p) = true) :
    Term.selectsAt Γ ρ σ R i (.paramSet p) w ↔
      ∃ x, x ∈ ρ.set p ∧ x.pointMem w := by
  rw [selectsAt_of_membership hm]
  exact Iff.rfl

/-! ## The surface matching equation and rule denotation -/

/-- The SURFACE matching equation: `Matches` with every binding read
bivalently. This is what a written atom means BEFORE lowering — the
judgment the engine's normalize and the naive model each re-derive;
`membership_lowering_preserves` is their shared arbiter. -/
def SurfaceMatches (Γ : Typing) (f : Fact) (a : Atom) (σ : Assignment)
    (ρ : ParamEnv) : Prop :=
  ∀ b, b ∈ a.bindings → Term.selectsAt Γ ρ σ a.relation b.1 b.2 (f b.1)

/-- The surface body judgment: `derives` with the surface matching
equation at both atom polarities — negation is the same anti-join,
read bivalently. -/
def surfaceDerives (Γ : Typing) (C : Classify) (r : Rule)
    (I : Instance) (ρ : ParamEnv) (σ : Assignment) : Prop :=
  (∀ a, a ∈ r.atoms → ∃ f, f ∈ I a.relation ∧ SurfaceMatches Γ f a σ ρ) ∧
  (∀ a, a ∈ r.negated → ¬ ∃ f, f ∈ I a.relation ∧ SurfaceMatches Γ f a σ ρ) ∧
  (∀ t, t ∈ r.conditions → Condition.holds C ρ σ t)

/-- One rule's SURFACE answers — the denotation of the written rule. -/
def surfaceRuleAnswers (Γ : Typing) (C : Classify) (r : Rule)
    (I : Instance) (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ σ, surfaceDerives Γ C r I ρ σ ∧ t = r.finds.map σ

/-- Membership in a rule's surface answers, unfolded. -/
theorem mem_surfaceRuleAnswers {Γ : Typing} {C : Classify} {r : Rule}
    {I : Instance} {ρ : ParamEnv} {t : AnswerTuple} :
    t ∈ surfaceRuleAnswers Γ C r I ρ ↔
      ∃ σ, surfaceDerives Γ C r I ρ σ ∧ t = r.finds.map σ :=
  Iff.rfl

/-! ## The occurrence form — vars plus filters, both polarities -/

/-- The surface equation split the way the engine's occurrence is: the
value-read bindings are a plain `Matches` (the variable positions and
value filters) and every membership binding is a same-fact `PointIn`
filter. Bridge: `ir/normalize/normalize.rs::lower_atom` — pass 1 keeps
the domain bindings, pass 2 emits `PointIn`/`FieldsPointIn`/
`AnyPointIn` for exactly the membership positions. -/
theorem surfaceMatches_iff_occurrence {Γ : Typing} {f : Fact} {a : Atom}
    {σ : Assignment} {ρ : ParamEnv} :
    SurfaceMatches Γ f a σ ρ ↔
      (Matches f
        ⟨a.relation,
         a.bindings.filter fun b => !(Γ.membership a.relation b.1 b.2)⟩
        σ ρ ∧
       ∀ b, b ∈ a.bindings → Γ.membership a.relation b.1 b.2 = true →
         ∃ x, Term.selects ρ σ b.2 x ∧ x.pointMem (f b.1)) := by
  constructor
  · intro h
    constructor
    · intro b hb
      obtain ⟨hmem, hflt⟩ := List.mem_filter.mp hb
      have hfalse : Γ.membership a.relation b.1 b.2 = false := by
        simpa using hflt
      exact (selectsAt_of_not_membership hfalse).mp (h b hmem)
    · intro b hb hm
      exact (selectsAt_of_membership hm).mp (h b hb)
  · rintro ⟨hdom, hflt⟩ b hb
    cases hm : Γ.membership a.relation b.1 b.2 with
    | false =>
      refine (selectsAt_of_not_membership hm).mpr ?_
      exact hdom b (List.mem_filter.mpr ⟨hb, by simp [hm]⟩)
    | true =>
      exact (selectsAt_of_membership hm).mpr (hflt b hb hm)

/-- The anti-probe filter form: a negated membership atom rejects
exactly when no fact passes the occurrence's domain bindings AND its
membership filters — the engine lowers negated atoms identically to
positive ones, and this is why that is sound. Bridge:
`ir/normalize/normalize.rs::lower_atom` (role-blind); the anti-probe
descriptors carry their filters. -/
theorem surface_antiprobe_filters {Γ : Typing} {a : Atom}
    {σ : Assignment} {ρ : ParamEnv} {I : Instance} :
    (¬ ∃ f, f ∈ I a.relation ∧ SurfaceMatches Γ f a σ ρ) ↔
      ¬ ∃ f, f ∈ I a.relation ∧
        (Matches f
          ⟨a.relation,
           a.bindings.filter fun b => !(Γ.membership a.relation b.1 b.2)⟩
          σ ρ ∧
         ∀ b, b ∈ a.bindings → Γ.membership a.relation b.1 b.2 = true →
           ∃ x, Term.selects ρ σ b.2 x ∧ x.pointMem (f b.1)) :=
  ⟨fun hn ⟨f, hf, h⟩ => hn ⟨f, hf, surfaceMatches_iff_occurrence.mpr h⟩,
   fun hn ⟨f, hf, h⟩ => hn ⟨f, hf, surfaceMatches_iff_occurrence.mp h⟩⟩

/-- The repeated-variable same-fact composition: a variable with a
domain binding at `j` and a membership binding at `i` in ONE atom pins
`σ v` to the fact's `j` value and reads that value's point membership
in the fact's `i` interval — the `FieldsPointIn` filter. The slot
discipline (the variable position is the first DOMAIN binding,
`normalize.rs` pass 1) is answer-invisible: the reading is per-binding
over one total assignment. -/
theorem repeated_membership_same_fact {Γ : Typing} {f : Fact} {a : Atom}
    {σ : Assignment} {ρ : ParamEnv} {v : VarId} {i j : FieldId}
    (h : SurfaceMatches Γ f a σ ρ)
    (hi : (i, Term.var v) ∈ a.bindings)
    (hm : Γ.membership a.relation i (.var v) = true)
    (hj : (j, Term.var v) ∈ a.bindings)
    (hd : Γ.membership a.relation j (.var v) = false) :
    f j = σ v ∧ (σ v).pointMem (f i) :=
  ⟨((selectsAt_of_not_membership hd).mp (h _ hj)).symm,
   (selectsAt_var_membership hm).mp (h _ hi)⟩

/-! ## Membership-free collapse — the surface reading IS `Matches` -/

/-- An atom with no membership bindings under the typing. -/
def Atom.membershipFree (Γ : Typing) (a : Atom) : Prop :=
  ∀ b, b ∈ a.bindings → Γ.membership a.relation b.1 b.2 = false

/-- On a membership-free atom the surface equation IS the matching
equation. -/
theorem surfaceMatches_of_membershipFree {Γ : Typing} {f : Fact}
    {a : Atom} {σ : Assignment} {ρ : ParamEnv}
    (hfree : a.membershipFree Γ) :
    SurfaceMatches Γ f a σ ρ ↔ Matches f a σ ρ := by
  constructor <;> intro h b hb
  · exact (selectsAt_of_not_membership (hfree b hb)).mp (h b hb)
  · exact (selectsAt_of_not_membership (hfree b hb)).mpr (h b hb)

/-- On a rule whose atoms (both polarities) are membership-free, the
surface denotation IS the denotation — the collapse the lowering
composes with. -/
theorem surface_eq_denotation_of_free {Γ : Typing} {C : Classify}
    {r : Rule} {I : Instance} {ρ : ParamEnv}
    (hpos : ∀ a, a ∈ r.atoms → Atom.membershipFree Γ a)
    (hneg : ∀ a, a ∈ r.negated → Atom.membershipFree Γ a) :
    ∀ t, t ∈ surfaceRuleAnswers Γ C r I ρ ↔ t ∈ ruleAnswers C r I ρ := by
  intro t
  constructor
  · rintro ⟨σ, ⟨hp, hn, hc⟩, rfl⟩
    refine mem_ruleAnswers.mpr ⟨σ, ⟨?_, ?_, hc⟩, rfl⟩
    · intro a ha
      obtain ⟨f, hf, hm⟩ := hp a ha
      exact ⟨f, hf, (surfaceMatches_of_membershipFree (hpos a ha)).mp hm⟩
    · rintro a ha ⟨f, hf, hm⟩
      exact hn a ha
        ⟨f, hf, (surfaceMatches_of_membershipFree (hneg a ha)).mpr hm⟩
  · rintro ⟨σ, ⟨hp, hn, hc⟩, rfl⟩
    refine ⟨σ, ⟨?_, ?_, hc⟩, rfl⟩
    · intro a ha
      obtain ⟨f, hf, hm⟩ := hp a ha
      exact ⟨f, hf, (surfaceMatches_of_membershipFree (hpos a ha)).mpr hm⟩
    · rintro a ha ⟨f, hf, hm⟩
      exact hn a ha
        ⟨f, hf, (surfaceMatches_of_membershipFree (hneg a ha)).mp hm⟩

/-! ## Stability — the mint touches nothing that exists -/

/-- Surface selection is stable under updating the typing AND the
assignment at a variable the term does not mention. -/
theorem selectsAt_stable {Γ : Typing} {u : VarId} {ty : ValueType}
    {ρ : ParamEnv} {σ σ' : Assignment} {R : RelId} {i : FieldId}
    {t : Term} {w : Value} (hne : ∀ v, v ∈ t.vars → v ≠ u)
    (hσ : ∀ v, v ∈ t.vars → σ v = σ' v) :
    Term.selectsAt Γ ρ σ R i t w ↔
      Term.selectsAt (Γ.updateVar u ty) ρ σ' R i t w := by
  unfold Term.selectsAt
  rw [Typing.membership_updateVar hne]
  by_cases hm : Γ.membership R i t = true
  · rw [if_pos hm, if_pos hm]
    constructor
    · rintro ⟨x, hx, hmem⟩
      exact ⟨x, (selects_congr hσ).mp hx, hmem⟩
    · rintro ⟨x, hx, hmem⟩
      exact ⟨x, (selects_congr hσ).mpr hx, hmem⟩
  · rw [if_neg hm, if_neg hm]
    exact selects_congr hσ

/-- The surface matching equation is stable under the same update, at
an atom that does not mention the minted variable. -/
theorem surfaceMatches_stable {Γ : Typing} {u : VarId} {ty : ValueType}
    {f : Fact} {a : Atom} {σ σ' : Assignment} {ρ : ParamEnv}
    (hne : ∀ v, v ∈ a.vars → v ≠ u)
    (hσ : ∀ v, v ∈ a.vars → σ v = σ' v) :
    SurfaceMatches Γ f a σ ρ ↔
      SurfaceMatches (Γ.updateVar u ty) f a σ' ρ := by
  constructor <;> intro h b hb
  · exact (selectsAt_stable
      (fun v hv => hne v (List.mem_flatMap.mpr ⟨b, hb, hv⟩))
      (fun v hv => hσ v (List.mem_flatMap.mpr ⟨b, hb, hv⟩))).mp (h b hb)
  · exact (selectsAt_stable
      (fun v hv => hne v (List.mem_flatMap.mpr ⟨b, hb, hv⟩))
      (fun v hv => hσ v (List.mem_flatMap.mpr ⟨b, hb, hv⟩))).mpr (h b hb)

/-! ## The fresh mint -/

/-- One past the largest id in the list — the canonical fresh
ceiling. -/
def varCeiling : List VarId → Nat
  | [] => 0
  | v :: vs => max (v.id + 1) (varCeiling vs)

/-- Every listed id is below the ceiling. -/
theorem lt_varCeiling {v : VarId} :
    ∀ {l : List VarId}, v ∈ l → v.id < varCeiling l
  | w :: ws, h => by
    rcases List.mem_cons.mp h with rfl | hm
    · exact Nat.lt_of_lt_of_le (Nat.lt_succ_self _) (Nat.le_max_left _ _)
    · exact Nat.lt_of_lt_of_le (lt_varCeiling hm) (Nat.le_max_right _ _)

/-- The rule's canonical fresh variable — the ceiling mint. -/
def Rule.freshVar (r : Rule) : VarId :=
  ⟨varCeiling r.allVars⟩

/-- The mint is fresh: it occurs nowhere in the rule. -/
theorem Rule.freshVar_not_mem (r : Rule) : r.freshVar ∉ r.allVars :=
  fun h => Nat.lt_irrefl _ (lt_varCeiling h)

/-! ## The lowering — one membership binding becomes one condition -/

/-- Lower the FIRST membership binding of a binding list: replace it
with a value read of the minted variable and return the lowered field
and the displaced term. `none` exactly when no binding is a
membership position. -/
def lowerBindings (isM : FieldId → Term → Bool) (u : VarId) :
    List (FieldId × Term) →
      Option (List (FieldId × Term) × FieldId × Term)
  | [] => none
  | b :: bs =>
    if isM b.1 b.2 then some ((b.1, Term.var u) :: bs, b.1, b.2)
    else (lowerBindings isM u bs).map fun out => (b :: out.1, out.2)

/-- `lowerBindings` finds nothing exactly on membership-free
bindings. -/
theorem lowerBindings_none {isM : FieldId → Term → Bool} {u : VarId} :
    ∀ {bs : List (FieldId × Term)},
      lowerBindings isM u bs = none ↔
        ∀ b, b ∈ bs → isM b.1 b.2 = false
  | [] => by simp [lowerBindings]
  | b :: bs => by
    by_cases hm : isM b.1 b.2
    · simp [lowerBindings, hm]
    · cases hlb : lowerBindings isM u bs with
      | none =>
        simp only [lowerBindings, if_neg hm, hlb, Option.map_none]
        constructor
        · intro _ b' hb'
          rcases List.mem_cons.mp hb' with rfl | hmem
          · exact Bool.eq_false_iff.mpr hm
          · exact (lowerBindings_none.mp hlb) b' hmem
        · intro _
          trivial
      | some out =>
        simp only [lowerBindings, if_neg hm, hlb, Option.map_some]
        constructor
        · intro h
          cases h
        · intro h
          have : lowerBindings isM u bs = none :=
            lowerBindings_none.mpr fun b' hb' =>
              h b' (List.mem_cons_of_mem _ hb')
          rw [this] at hlb
          cases hlb

/-- `lowerBindings`' success shape: the list splits at a membership
binding, which is rewritten to the minted value read. -/
theorem lowerBindings_some {isM : FieldId → Term → Bool} {u : VarId} :
    ∀ {bs : List (FieldId × Term)}
      {out : List (FieldId × Term) × FieldId × Term},
      lowerBindings isM u bs = some out →
      ∃ pre i t post,
        bs = pre ++ (i, t) :: post ∧ isM i t = true ∧
        out = (pre ++ (i, Term.var u) :: post, i, t)
  | [], out, h => by cases h
  | b :: bs, out, h => by
    by_cases hm : isM b.1 b.2
    · rw [lowerBindings, if_pos hm] at h
      cases h
      exact ⟨[], b.1, b.2, bs, rfl, hm, rfl⟩
    · rw [lowerBindings, if_neg hm] at h
      cases hlb : lowerBindings isM u bs with
      | none => rw [hlb] at h; cases h
      | some out' =>
        rw [hlb] at h
        cases h
        obtain ⟨pre, i, t, post, hbs, hmt, hout⟩ := lowerBindings_some hlb
        refine ⟨b :: pre, i, t, post, by simp [hbs], hmt, ?_⟩
        simp [hout]

/-- Lower the first membership binding of an atom list, returning the
rewritten atoms with the lowered atom's relation, field, and term. -/
def lowerAtoms (isM : RelId → FieldId → Term → Bool) (u : VarId) :
    List Atom → Option (List Atom × RelId × FieldId × Term)
  | [] => none
  | a :: rest =>
    match lowerBindings (isM a.relation) u a.bindings with
    | some (bs, i, t) =>
      some ((⟨a.relation, bs⟩ : Atom) :: rest, a.relation, i, t)
    | none => (lowerAtoms isM u rest).map fun out => (a :: out.1, out.2)

/-- `lowerAtoms` finds nothing exactly when every atom is
membership-free. -/
theorem lowerAtoms_none {isM : RelId → FieldId → Term → Bool}
    {u : VarId} :
    ∀ {atoms : List Atom},
      lowerAtoms isM u atoms = none ↔
        ∀ a, a ∈ atoms → ∀ b, b ∈ a.bindings →
          isM a.relation b.1 b.2 = false
  | [] => by simp [lowerAtoms]
  | a :: rest => by
    cases hlb : lowerBindings (isM a.relation) u a.bindings with
    | some out =>
      obtain ⟨bs, i, t⟩ := out
      simp only [lowerAtoms, hlb]
      constructor
      · intro h
        cases h
      · intro h
        have : lowerBindings (isM a.relation) u a.bindings = none :=
          lowerBindings_none.mpr (h a (List.mem_cons_self ..))
        rw [this] at hlb
        cases hlb
    | none =>
      cases hla : lowerAtoms isM u rest with
      | none =>
        simp only [lowerAtoms, hlb, hla, Option.map_none]
        constructor
        · intro _ a' ha'
          rcases List.mem_cons.mp ha' with rfl | hmem
          · exact lowerBindings_none.mp hlb
          · exact (lowerAtoms_none.mp hla) a' hmem
        · intro _
          trivial
      | some out =>
        simp only [lowerAtoms, hlb, hla, Option.map_some]
        constructor
        · intro h
          cases h
        · intro h
          have : lowerAtoms isM u rest = none :=
            lowerAtoms_none.mpr fun a' ha' =>
              h a' (List.mem_cons_of_mem _ ha')
          rw [this] at hla
          cases hla

/-- `lowerAtoms`' success shape: the atom list splits at the lowered
atom, whose bindings split at the lowered position. -/
theorem lowerAtoms_some {isM : RelId → FieldId → Term → Bool}
    {u : VarId} :
    ∀ {atoms : List Atom} {out : List Atom × RelId × FieldId × Term},
      lowerAtoms isM u atoms = some out →
      ∃ pre a post bs i t,
        atoms = pre ++ a :: post ∧
        lowerBindings (isM a.relation) u a.bindings = some (bs, i, t) ∧
        out = (pre ++ (⟨a.relation, bs⟩ : Atom) :: post, a.relation, i, t)
  | [], out, h => by cases h
  | a :: rest, out, h => by
    cases hlb : lowerBindings (isM a.relation) u a.bindings with
    | some triple =>
      obtain ⟨bs, i, t⟩ := triple
      rw [lowerAtoms, hlb] at h
      cases h
      exact ⟨[], a, rest, bs, i, t, rfl, hlb, rfl⟩
    | none =>
      rw [lowerAtoms, hlb] at h
      cases hla : lowerAtoms isM u rest with
      | none => rw [hla] at h; cases h
      | some out' =>
        rw [hla] at h
        cases h
        obtain ⟨pre, a', post, bs, i, t, hrest, hlb', hout⟩ :=
          lowerAtoms_some hla
        refine ⟨a :: pre, a', post, bs, i, t, by simp [hrest], hlb', ?_⟩
        simp [hout]

/-- One lowering step on a rule: rewrite the first membership binding
of the positive atoms to a value read of the mint `u`, type `u` at the
lowered field, and append the `pointIn` condition — the displaced term
on the point side, the mint on the interval side. `none` exactly when
the positive atoms are membership-free. -/
def stepLower (Γ : Typing) (u : VarId) (r : Rule) :
    Option (Typing × Rule) :=
  match lowerAtoms Γ.membership u r.atoms with
  | none => none
  | some (atoms', R, i, t) =>
    some (Γ.updateVar u (Γ.header.fieldType R i),
      { finds := r.finds, atoms := atoms', negated := r.negated,
        conditions :=
          r.conditions ++ [.leaf ⟨.pointIn, .var u, t⟩] })

/-- `stepLower` stalls exactly on membership-free positive atoms. -/
theorem stepLower_none {Γ : Typing} {u : VarId} {r : Rule} :
    stepLower Γ u r = none ↔
      ∀ a, a ∈ r.atoms → a.membershipFree Γ := by
  unfold stepLower
  cases hla : lowerAtoms Γ.membership u r.atoms with
  | none =>
    simp only []
    constructor
    · intro _
      exact fun a ha => (lowerAtoms_none.mp hla) a ha
    · intro _
      trivial
  | some out =>
    obtain ⟨atoms', R, i, t⟩ := out
    simp only []
    constructor
    · intro h
      cases h
    · intro h
      have : lowerAtoms Γ.membership u r.atoms = none :=
        lowerAtoms_none.mpr fun a ha => h a ha
      rw [this] at hla
      cases hla

/-- `stepLower`'s success shape, fully unpacked. -/
theorem stepLower_some {Γ : Typing} {u : VarId} {r : Rule}
    {out : Typing × Rule} (h : stepLower Γ u r = some out) :
    ∃ pre a post bpre i t bpost,
      r.atoms = pre ++ a :: post ∧
      a.bindings = bpre ++ (i, t) :: bpost ∧
      Γ.membership a.relation i t = true ∧
      out = (Γ.updateVar u (Γ.header.fieldType a.relation i),
        { finds := r.finds,
          atoms := pre ++
            (⟨a.relation, bpre ++ (i, Term.var u) :: bpost⟩ : Atom)
            :: post,
          negated := r.negated,
          conditions :=
            r.conditions ++ [.leaf ⟨.pointIn, .var u, t⟩] }) := by
  unfold stepLower at h
  cases hla : lowerAtoms Γ.membership u r.atoms with
  | none => rw [hla] at h; cases h
  | some quad =>
    rw [hla] at h
    obtain ⟨pre, a, post, bs, i, t, hatoms, hlb, hout⟩ :=
      lowerAtoms_some hla
    obtain ⟨bpre, i2, t2, bpost, hbind, hmem, hbs⟩ :=
      lowerBindings_some hlb
    injection hbs with hbs1 hrest
    injection hrest with hbs2 hbs3
    subst hbs1 hbs2 hbs3
    subst hout
    cases h
    exact ⟨pre, a, post, bpre, i, t, bpost, hatoms, hbind, hmem, rfl⟩

/-! ## The step preserves the surface denotation -/

/-- The freshness spends, atom-vars form: nothing the rule mentions is
the mint. -/
theorem not_mint_of_allVars {r : Rule} {u : VarId}
    (hu : u ∉ r.allVars) :
    (∀ v, v ∈ r.finds → v ≠ u) ∧
    (∀ a, a ∈ r.atoms → ∀ v, v ∈ a.vars → v ≠ u) ∧
    (∀ a, a ∈ r.negated → ∀ v, v ∈ a.vars → v ≠ u) ∧
    (∀ c, c ∈ r.conditions → ∀ v, v ∈ c.vars → v ≠ u) := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · rintro v hv rfl
    exact hu (mem_allVars.mpr (Or.inl hv))
  · rintro a ha v hv rfl
    exact hu (mem_allVars.mpr (Or.inr (Or.inl ⟨a, ha, hv⟩)))
  · rintro a ha v hv rfl
    exact hu (mem_allVars.mpr (Or.inr (Or.inr (Or.inl ⟨a, ha, hv⟩))))
  · rintro c hc v hv rfl
    exact hu (mem_allVars.mpr (Or.inr (Or.inr (Or.inr ⟨c, hc, hv⟩))))

/-- A binding's term mentions only atom variables. -/
theorem binding_vars_sub_atom {a : Atom} {b : FieldId × Term}
    (hb : b ∈ a.bindings) {v : VarId} (hv : v ∈ b.2.vars) :
    v ∈ a.vars :=
  List.mem_flatMap.mpr ⟨b, hb, hv⟩

/-- **The step theorem at the BINDING level, forward**: every surface
derivation of the written rule extends — at the mint alone — to one
of the lowered rule: the mint takes the witnessing fact's field value
and every other variable keeps its own. The fold-level companion
(`membership_lowering_preserves_fold`, `Exec/Dedup.lean` — finding
087) spends exactly this agreement. -/
theorem stepLower_derives_forward {Γ : Typing} {C : Classify}
    {r : Rule} {I : Instance} {ρ : ParamEnv} {u : VarId} {Γ' : Typing}
    {r' : Rule} (hu : u ∉ r.allVars)
    (hstep : stepLower Γ u r = some (Γ', r')) {σ : Assignment}
    (hσ : surfaceDerives Γ C r I ρ σ) :
    ∃ σ', surfaceDerives Γ' C r' I ρ σ' ∧ ∀ v, v ≠ u → σ' v = σ v := by
  obtain ⟨pre, a, post, bpre, i, t₀, bpost, hatoms, hbind, hmem, hout⟩ :=
    stepLower_some hstep
  injection hout with hΓ hr
  subst hΓ hr
  obtain ⟨hneF, hneA, hneN, hneC⟩ := not_mint_of_allVars hu
  have haMem : a ∈ r.atoms := by
    rw [hatoms]; exact List.mem_append.mpr (Or.inr (List.mem_cons_self ..))
  have hbMem : (i, t₀) ∈ a.bindings := by
    rw [hbind]; exact List.mem_append.mpr (Or.inr (List.mem_cons_self ..))
  have hIntF : (Γ.header.fieldType a.relation i).isInterval = true := by
    have h : (Γ.header.isInterval a.relation i
        && !(Γ.termInterval t₀)) = true := hmem
    rw [Bool.and_eq_true] at h
    exact Header.fieldType_isInterval.mp h.1
  -- The rewritten binding reads by VALUE under the updated typing.
  have hNewFalse :
      (Γ.updateVar u (Γ.header.fieldType a.relation i)).membership
        a.relation i (Term.var u) = false := by
    unfold Typing.membership Typing.termInterval Typing.updateVar
    simp [hIntF]
  obtain ⟨hpos, hneg, hcond⟩ := hσ
  obtain ⟨f, hf, hSM⟩ := hpos a haMem
  refine ⟨fun v => if v = u then f i else σ v, ⟨?_, ?_, ?_⟩,
    fun v hv => by simp [hv]⟩
  · -- positive atoms of the lowered rule
    intro a'' ha''
    rcases List.mem_append.mp ha'' with hpre | hmid
    · obtain ⟨g, hg, hSMg⟩ := hpos a'' (by
        rw [hatoms]; exact List.mem_append.mpr (Or.inl hpre))
      refine ⟨g, hg, (surfaceMatches_stable
        (hneA a'' (by rw [hatoms]; exact List.mem_append.mpr (Or.inl hpre)))
        (fun v hv => by
          simp [hneA a''
            (by rw [hatoms]; exact List.mem_append.mpr (Or.inl hpre))
            v hv]) ).mp hSMg⟩
    · rcases List.mem_cons.mp hmid with rfl | hpost
      · -- the lowered atom, matched by the SAME fact
        refine ⟨f, hf, ?_⟩
        intro b hb
        rcases List.mem_append.mp hb with hbpre | hbmid
        · have hbOrig : b ∈ a.bindings := by
            rw [hbind]; exact List.mem_append.mpr (Or.inl hbpre)
          exact (selectsAt_stable
            (fun v hv => hneA a haMem v (binding_vars_sub_atom hbOrig hv))
            (fun v hv => by
              simp [hneA a haMem v (binding_vars_sub_atom hbOrig hv)]))
            |>.mp (hSM b hbOrig)
        · rcases List.mem_cons.mp hbmid with rfl | hbpost
          · -- the minted binding pins the field value
            refine (selectsAt_of_not_membership hNewFalse).mpr ?_
            show (if u = u then f i else σ u) = f i
            simp
          · have hbOrig : b ∈ a.bindings := by
              rw [hbind]
              exact List.mem_append.mpr
                (Or.inr (List.mem_cons_of_mem _ hbpost))
            exact (selectsAt_stable
              (fun v hv => hneA a haMem v (binding_vars_sub_atom hbOrig hv))
              (fun v hv => by
                simp [hneA a haMem v (binding_vars_sub_atom hbOrig hv)]))
              |>.mp (hSM b hbOrig)
      · obtain ⟨g, hg, hSMg⟩ := hpos a'' (by
          rw [hatoms]
          exact List.mem_append.mpr (Or.inr (List.mem_cons_of_mem _ hpost)))
        have ha''r : a'' ∈ r.atoms := by
          rw [hatoms]
          exact List.mem_append.mpr (Or.inr (List.mem_cons_of_mem _ hpost))
        exact ⟨g, hg, (surfaceMatches_stable (hneA a'' ha''r)
          (fun v hv => by simp [hneA a'' ha''r v hv])).mp hSMg⟩
  · -- negated atoms: unchanged, transported
    rintro a'' ha'' ⟨g, hg, hSMg⟩
    exact hneg a'' ha'' ⟨g, hg, (surfaceMatches_stable (hneN a'' ha'')
      (fun v hv => by simp [hneN a'' ha'' v hv])).mpr hSMg⟩
  · -- conditions: the old ones transported, the new one discharged
    intro c hc
    rcases List.mem_append.mp hc with hold | hnew
    · exact (condHolds_congr C ρ σ _ c
        (fun v hv => by simp [hneC c hold v hv])).mp (hcond c hold)
    · rcases List.mem_cons.mp hnew with rfl | hemp
      · -- the appended pointIn condition
        have hsel := (selectsAt_of_membership hmem).mp (hSM _ hbMem)
        obtain ⟨x, hx, hpm⟩ := hsel
        refine ⟨f i, x, ?_, ?_, hpm⟩
        · show (if u = u then f i else σ u) = f i
          simp
        · refine (selects_congr fun v hv => ?_).mp hx
          have hvne : v ≠ u :=
            hneA a haMem v (binding_vars_sub_atom hbMem hv)
          simp [hvne]
      · cases hemp

/-- **The step theorem at the BINDING level, backward**: the SAME
assignment derives the written rule — the mint pinned the field's
value, and the appended `pointIn` condition said exactly what the
membership binding said. -/
theorem stepLower_derives_backward {Γ : Typing} {C : Classify}
    {r : Rule} {I : Instance} {ρ : ParamEnv} {u : VarId} {Γ' : Typing}
    {r' : Rule} (hu : u ∉ r.allVars)
    (hstep : stepLower Γ u r = some (Γ', r')) {σ : Assignment}
    (hσ : surfaceDerives Γ' C r' I ρ σ) :
    surfaceDerives Γ C r I ρ σ := by
  obtain ⟨pre, a, post, bpre, i, t₀, bpost, hatoms, hbind, hmem, hout⟩ :=
    stepLower_some hstep
  injection hout with hΓ hr
  subst hΓ hr
  obtain ⟨hneF, hneA, hneN, hneC⟩ := not_mint_of_allVars hu
  have haMem : a ∈ r.atoms := by
    rw [hatoms]; exact List.mem_append.mpr (Or.inr (List.mem_cons_self ..))
  have hIntF : (Γ.header.fieldType a.relation i).isInterval = true := by
    have h : (Γ.header.isInterval a.relation i
        && !(Γ.termInterval t₀)) = true := hmem
    rw [Bool.and_eq_true] at h
    exact Header.fieldType_isInterval.mp h.1
  have hNewFalse :
      (Γ.updateVar u (Γ.header.fieldType a.relation i)).membership
        a.relation i (Term.var u) = false := by
    unfold Typing.membership Typing.termInterval Typing.updateVar
    simp [hIntF]
  obtain ⟨hpos, hneg, hcond⟩ := hσ
  have hcNew : Condition.holds C ρ σ
      (.leaf ⟨.pointIn, .var u, t₀⟩) :=
    hcond _ (List.mem_append.mpr (Or.inr (List.mem_cons_self ..)))
  obtain ⟨A, hAMem, hSMA⟩ := hpos
    (⟨a.relation, bpre ++ (i, Term.var u) :: bpost⟩ : Atom)
    (List.mem_append.mpr (Or.inr (List.mem_cons_self ..)))
  refine ⟨?_, ?_, ?_⟩
  · intro a'' ha''
    rw [hatoms] at ha''
    rcases List.mem_append.mp ha'' with hpre | hmid
    · obtain ⟨g, hg, hSMg⟩ := hpos a''
        (List.mem_append.mpr (Or.inl hpre))
      have ha''r : a'' ∈ r.atoms := by
        rw [hatoms]; exact List.mem_append.mpr (Or.inl hpre)
      exact ⟨g, hg, (surfaceMatches_stable (hneA a'' ha''r)
        (fun v hv => rfl)).mpr hSMg⟩
    · rcases List.mem_cons.mp hmid with heq | hpost
      · -- the original membership atom, matched by the lowered
        -- atom's fact: the mint pins the field, the condition says
        -- the membership
        rw [heq]
        refine ⟨A, hAMem, ?_⟩
        have hAu : σ u = A i := by
          have := hSMA (i, Term.var u)
            (List.mem_append.mpr (Or.inr (List.mem_cons_self ..)))
          exact (selectsAt_of_not_membership hNewFalse).mp this
        intro b hb
        rw [hbind] at hb
        rcases List.mem_append.mp hb with hbpre | hbmid
        · have hbLow : b ∈
              (⟨a.relation, bpre ++ (i, Term.var u) :: bpost⟩ :
                Atom).bindings :=
            List.mem_append.mpr (Or.inl hbpre)
          have hbOrig : b ∈ a.bindings := by
            rw [hbind]; exact List.mem_append.mpr (Or.inl hbpre)
          exact (selectsAt_stable
            (fun v hv => hneA a haMem v (binding_vars_sub_atom hbOrig hv))
            (fun v hv => rfl)).mpr (hSMA b hbLow)
        · rcases List.mem_cons.mp hbmid with rfl | hbpost
          · -- the membership binding itself, from the condition
            refine (selectsAt_of_membership hmem).mpr ?_
            obtain ⟨x, y, hx, hy, hden⟩ := hcNew
            have hxu : σ u = x := hx
            exact ⟨y, hy, by
              show ∃ p, y.point = some p ∧ p ∈ (A i).points
              rw [← hAu, hxu]
              exact hden⟩
          · have hbLow : b ∈
                (⟨a.relation, bpre ++ (i, Term.var u) :: bpost⟩ :
                  Atom).bindings :=
              List.mem_append.mpr
                (Or.inr (List.mem_cons_of_mem _ hbpost))
            have hbOrig : b ∈ a.bindings := by
              rw [hbind]
              exact List.mem_append.mpr
                (Or.inr (List.mem_cons_of_mem _ hbpost))
            exact (selectsAt_stable
              (fun v hv =>
                hneA a haMem v (binding_vars_sub_atom hbOrig hv))
              (fun v hv => rfl)).mpr (hSMA b hbLow)
      · obtain ⟨g, hg, hSMg⟩ := hpos a''
          (List.mem_append.mpr (Or.inr (List.mem_cons_of_mem _ hpost)))
        have ha''r : a'' ∈ r.atoms := by
          rw [hatoms]
          exact List.mem_append.mpr (Or.inr (List.mem_cons_of_mem _ hpost))
        exact ⟨g, hg, (surfaceMatches_stable (hneA a'' ha''r)
          (fun v hv => rfl)).mpr hSMg⟩
  · rintro a'' ha'' ⟨g, hg, hSMg⟩
    exact hneg a'' ha'' ⟨g, hg, (surfaceMatches_stable (hneN a'' ha'')
      (fun v hv => rfl)).mp hSMg⟩
  · intro c hc
    exact hcond c (List.mem_append.mpr (Or.inl hc))

/-- The lowering never touches the head: the lowered rule's finds are
the written rule's. -/
theorem stepLower_finds {Γ : Typing} {u : VarId} {r : Rule}
    {Γ' : Typing} {r' : Rule}
    (hstep : stepLower Γ u r = some (Γ', r')) : r'.finds = r.finds := by
  obtain ⟨_, _, _, _, _, _, _, _, _, _, hout⟩ := stepLower_some hstep
  injection hout with _ hr
  rw [hr]

/-- **The step theorem**: one lowering step preserves the SURFACE
denotation — the binding-level halves, projected: the mint is not a
find, so the head reads only agreeing variables. -/
theorem stepLower_preserves {Γ : Typing} {C : Classify} {r : Rule}
    {I : Instance} {ρ : ParamEnv} {u : VarId} {Γ' : Typing} {r' : Rule}
    (hu : u ∉ r.allVars) (hstep : stepLower Γ u r = some (Γ', r')) :
    ∀ tup, tup ∈ surfaceRuleAnswers Γ C r I ρ ↔
      tup ∈ surfaceRuleAnswers Γ' C r' I ρ := by
  intro tup
  constructor
  · rintro ⟨σ, hσ, rfl⟩
    obtain ⟨σ', hσ', hag⟩ := stepLower_derives_forward hu hstep hσ
    refine ⟨σ', hσ', ?_⟩
    rw [stepLower_finds hstep]
    exact List.map_congr_left fun v hv =>
      (hag v ((not_mint_of_allVars hu).1 v hv)).symm
  · rintro ⟨σ, hσ, rfl⟩
    exact ⟨σ, stepLower_derives_backward hu hstep hσ,
      by rw [stepLower_finds hstep]⟩

/-! ## The membership count — the lowering's fuel -/

/-- Membership bindings in one binding list. -/
def memCountB (isM : FieldId → Term → Bool) :
    List (FieldId × Term) → Nat
  | [] => 0
  | b :: bs => (if isM b.1 b.2 then 1 else 0) + memCountB isM bs

/-- The count reads only the statuses. -/
theorem memCountB_ext {p q : FieldId → Term → Bool} :
    ∀ {bs : List (FieldId × Term)},
      (∀ b, b ∈ bs → p b.1 b.2 = q b.1 b.2) →
      memCountB p bs = memCountB q bs
  | [], _ => rfl
  | b :: bs, h => by
    unfold memCountB
    rw [h b (List.mem_cons_self ..),
      memCountB_ext fun b' hb' => h b' (List.mem_cons_of_mem _ hb')]

/-- The count splits over append. -/
theorem memCountB_append {p : FieldId → Term → Bool} :
    ∀ (xs ys : List (FieldId × Term)),
      memCountB p (xs ++ ys) = memCountB p xs + memCountB p ys
  | [], ys => by simp [memCountB]
  | x :: xs, ys => by
    simp [memCountB, memCountB_append xs ys, Nat.add_assoc]

/-- A zero count is membership-freeness. -/
theorem memCountB_eq_zero {p : FieldId → Term → Bool} :
    ∀ {bs : List (FieldId × Term)},
      memCountB p bs = 0 ↔ ∀ b, b ∈ bs → p b.1 b.2 = false
  | [] => by simp [memCountB]
  | b :: bs => by
    unfold memCountB
    by_cases hm : p b.1 b.2
    · simp [hm]
    · simp only [if_neg hm, Nat.zero_add]
      rw [memCountB_eq_zero]
      constructor
      · intro h b' hb'
        rcases List.mem_cons.mp hb' with rfl | hmem
        · exact Bool.eq_false_iff.mpr hm
        · exact h b' hmem
      · intro h b' hb'
        exact h b' (List.mem_cons_of_mem _ hb')

/-- Membership bindings across an atom list. -/
def memCount (isM : RelId → FieldId → Term → Bool) : List Atom → Nat
  | [] => 0
  | a :: rest =>
    memCountB (isM a.relation) a.bindings + memCount isM rest

/-- The atom-level count reads only the statuses. -/
theorem memCount_ext {p q : RelId → FieldId → Term → Bool} :
    ∀ {atoms : List Atom},
      (∀ a, a ∈ atoms → ∀ b, b ∈ a.bindings →
        p a.relation b.1 b.2 = q a.relation b.1 b.2) →
      memCount p atoms = memCount q atoms
  | [], _ => rfl
  | a :: rest, h => by
    unfold memCount
    rw [memCountB_ext (h a (List.mem_cons_self ..)),
      memCount_ext fun a' ha' => h a' (List.mem_cons_of_mem _ ha')]

/-- The atom-level count splits over append. -/
theorem memCount_append {p : RelId → FieldId → Term → Bool} :
    ∀ (xs ys : List Atom),
      memCount p (xs ++ ys) = memCount p xs + memCount p ys
  | [], ys => by simp [memCount]
  | x :: xs, ys => by
    simp [memCount, memCount_append xs ys, Nat.add_assoc]

/-- A zero atom-level count is membership-freeness everywhere. -/
theorem memCount_eq_zero {p : RelId → FieldId → Term → Bool} :
    ∀ {atoms : List Atom},
      memCount p atoms = 0 ↔
        ∀ a, a ∈ atoms → ∀ b, b ∈ a.bindings →
          p a.relation b.1 b.2 = false
  | [] => by simp [memCount]
  | a :: rest => by
    unfold memCount
    rw [Nat.add_eq_zero_iff, memCountB_eq_zero, memCount_eq_zero]
    constructor
    · rintro ⟨h1, h2⟩ a' ha'
      rcases List.mem_cons.mp ha' with rfl | hmem
      · exact h1
      · exact h2 a' hmem
    · intro h
      exact ⟨h a (List.mem_cons_self ..),
        fun a' ha' => h a' (List.mem_cons_of_mem _ ha')⟩

/-- One step lowers the count by exactly one — the fuel argument. -/
theorem stepLower_decrement {Γ : Typing} {u : VarId} {r : Rule}
    {Γ' : Typing} {r' : Rule} (hu : u ∉ r.allVars)
    (hstep : stepLower Γ u r = some (Γ', r')) :
    memCount Γ'.membership r'.atoms + 1 =
      memCount Γ.membership r.atoms := by
  obtain ⟨pre, a, post, bpre, i, t₀, bpost, hatoms, hbind, hmem, hout⟩ :=
    stepLower_some hstep
  injection hout with hΓ hr
  subst hΓ hr
  obtain ⟨-, hneA, -, -⟩ := not_mint_of_allVars hu
  have haMem : a ∈ r.atoms := by
    rw [hatoms]; exact List.mem_append.mpr (Or.inr (List.mem_cons_self ..))
  have hIntF : (Γ.header.fieldType a.relation i).isInterval = true := by
    have h : (Γ.header.isInterval a.relation i
        && !(Γ.termInterval t₀)) = true := hmem
    rw [Bool.and_eq_true] at h
    exact Header.fieldType_isInterval.mp h.1
  have hNewFalse :
      (Γ.updateVar u (Γ.header.fieldType a.relation i)).membership
        a.relation i (Term.var u) = false := by
    unfold Typing.membership Typing.termInterval Typing.updateVar
    simp [hIntF]
  -- statuses are stable off the mint
  have hstable : ∀ a'' ∈ r.atoms, ∀ b ∈ a''.bindings,
      (Γ.updateVar u (Γ.header.fieldType a.relation i)).membership
        a''.relation b.1 b.2 = Γ.membership a''.relation b.1 b.2 :=
    fun a'' ha'' b hb => Typing.membership_updateVar
      fun v hv => hneA a'' ha'' v (binding_vars_sub_atom hb hv)
  have hpre_mem : ∀ a'' ∈ pre, a'' ∈ r.atoms := fun a'' h => by
    rw [hatoms]; exact List.mem_append.mpr (Or.inl h)
  have hpost_mem : ∀ a'' ∈ post, a'' ∈ r.atoms := fun a'' h => by
    rw [hatoms]
    exact List.mem_append.mpr (Or.inr (List.mem_cons_of_mem _ h))
  have hbpre_mem : ∀ b ∈ bpre, b ∈ a.bindings := fun b h => by
    rw [hbind]; exact List.mem_append.mpr (Or.inl h)
  have hbpost_mem : ∀ b ∈ bpost, b ∈ a.bindings := fun b h => by
    rw [hbind]
    exact List.mem_append.mpr (Or.inr (List.mem_cons_of_mem _ h))
  rw [hatoms, memCount_append, memCount_append]
  simp only [memCount]
  rw [hbind, memCountB_append, memCountB_append]
  simp only [memCountB]
  rw [memCount_ext fun a'' ha'' b hb => hstable a'' (hpre_mem a'' ha'') b hb,
    memCount_ext fun a'' ha'' b hb => hstable a'' (hpost_mem a'' ha'') b hb,
    memCountB_ext fun b hb => hstable a haMem b (hbpre_mem b hb),
    memCountB_ext fun b hb => hstable a haMem b (hbpost_mem b hb),
    hNewFalse, hmem]
  rw [if_neg Bool.false_ne_true, if_pos rfl]
  omega

/-! ## The full lowering -/

/-- Iterate the step, fuel-bounded: each iteration mints the CURRENT
rule's ceiling variable, which is fresh for everything already
present — earlier mints included. -/
def lowerFuel : Nat → Typing → Rule → Typing × Rule
  | 0, Γ, r => (Γ, r)
  | n + 1, Γ, r =>
    match stepLower Γ r.freshVar r with
    | some (Γ', r') => lowerFuel n Γ' r'
    | none => (Γ, r)

/-- The full membership lowering: run the step to exhaustion — the
membership count is exactly enough fuel
(`stepLower_decrement`). Returns the extended typing (the mints' field
types) with the lowered rule. -/
def Rule.lowerMembership (Γ : Typing) (r : Rule) : Typing × Rule :=
  lowerFuel (memCount Γ.membership r.atoms) Γ r

/-- Every fuel level preserves the surface denotation — the step
theorem, iterated. -/
theorem lowerFuel_preserves (C : Classify) (I : Instance)
    (ρ : ParamEnv) :
    ∀ (n : Nat) (Γ : Typing) (r : Rule) (t : AnswerTuple),
      t ∈ surfaceRuleAnswers Γ C r I ρ ↔
        t ∈ surfaceRuleAnswers (lowerFuel n Γ r).1 C
          (lowerFuel n Γ r).2 I ρ
  | 0, Γ, r, t => Iff.rfl
  | n + 1, Γ, r, t => by
    cases hstep : stepLower Γ r.freshVar r with
    | none => simp only [lowerFuel, hstep]
    | some out =>
      obtain ⟨Γ', r'⟩ := out
      simp only [lowerFuel, hstep]
      exact (stepLower_preserves r.freshVar_not_mem hstep t).trans
        (lowerFuel_preserves C I ρ n Γ' r' t)

/-- The lowering never touches the negated atoms. -/
theorem lowerFuel_negated :
    ∀ (n : Nat) (Γ : Typing) (r : Rule),
      (lowerFuel n Γ r).2.negated = r.negated
  | 0, _, _ => rfl
  | n + 1, Γ, r => by
    cases hstep : stepLower Γ r.freshVar r with
    | none => simp only [lowerFuel, hstep]
    | some out =>
      obtain ⟨Γ', r'⟩ := out
      simp only [lowerFuel, hstep]
      rw [lowerFuel_negated n Γ' r']
      obtain ⟨_, _, _, _, _, _, _, _, _, _, hout⟩ := stepLower_some hstep
      injection hout with _ hr
      rw [hr]

/-- Negated membership-freeness survives the lowering: negated atoms
are untouched and each mint is fresh for their variables. -/
theorem lowerFuel_negFree :
    ∀ (n : Nat) (Γ : Typing) (r : Rule),
      (∀ a, a ∈ r.negated → Atom.membershipFree Γ a) →
      ∀ a, a ∈ (lowerFuel n Γ r).2.negated →
        Atom.membershipFree (lowerFuel n Γ r).1 a
  | 0, _, _, h => h
  | n + 1, Γ, r, h => by
    cases hstep : stepLower Γ r.freshVar r with
    | none => simpa only [lowerFuel, hstep] using h
    | some out =>
      obtain ⟨Γ', r'⟩ := out
      simp only [lowerFuel, hstep]
      refine lowerFuel_negFree n Γ' r' ?_
      obtain ⟨-, hneA, hneN, -⟩ :=
        not_mint_of_allVars r.freshVar_not_mem
      obtain ⟨pre, a, post, bpre, i, t₀, bpost, -, -, -, hout⟩ :=
        stepLower_some hstep
      injection hout with hΓ hr
      subst hΓ hr
      intro a'' ha'' b hb
      rw [Typing.membership_updateVar
        fun v hv => hneN a'' ha'' v (binding_vars_sub_atom hb hv)]
      exact h a'' ha'' b hb

/-- With the count as fuel, the lowered rule's positive atoms are
membership-free — the lowering RAN to exhaustion. -/
theorem lowerFuel_posFree :
    ∀ (n : Nat) (Γ : Typing) (r : Rule),
      memCount Γ.membership r.atoms ≤ n →
      ∀ a, a ∈ (lowerFuel n Γ r).2.atoms →
        Atom.membershipFree (lowerFuel n Γ r).1 a
  | 0, Γ, r, hle => by
    have h0 : memCount Γ.membership r.atoms = 0 := Nat.le_zero.mp hle
    exact fun a ha b hb => (memCount_eq_zero.mp h0) a ha b hb
  | n + 1, Γ, r, hle => by
    cases hstep : stepLower Γ r.freshVar r with
    | none =>
      simp only [lowerFuel, hstep]
      exact stepLower_none.mp hstep
    | some out =>
      obtain ⟨Γ', r'⟩ := out
      simp only [lowerFuel, hstep]
      refine lowerFuel_posFree n Γ' r' ?_
      have hdec := stepLower_decrement r.freshVar_not_mem hstep
      omega

/-! ## The binding-level lowering — what the fold companion spends -/

/-- One step never loses a variable: the displaced term's variables
move from the lowered binding into the appended condition, everything
else stays — `r.allVars ⊆ r'.allVars`, as membership. -/
theorem stepLower_allVars_sub {Γ : Typing} {u : VarId} {r : Rule}
    {Γ' : Typing} {r' : Rule}
    (hstep : stepLower Γ u r = some (Γ', r')) :
    ∀ v, v ∈ r.allVars → v ∈ r'.allVars := by
  obtain ⟨pre, a, post, bpre, i, t₀, bpost, hatoms, hbind, hmem, hout⟩ :=
    stepLower_some hstep
  injection hout with hΓ hr
  subst hΓ hr
  intro v hv
  rcases mem_allVars.mp hv with hf | ⟨x, hx, hvx⟩ | ⟨x, hx, hvx⟩ |
    ⟨c, hc, hvc⟩
  · exact mem_allVars.mpr (Or.inl hf)
  · rw [hatoms] at hx
    rcases List.mem_append.mp hx with hxpre | hxmid
    · exact mem_allVars.mpr (Or.inr (Or.inl
        ⟨x, List.mem_append.mpr (Or.inl hxpre), hvx⟩))
    · rcases List.mem_cons.mp hxmid with rfl | hxpost
      · -- the lowered atom: the binding either survives in place or
        -- its displaced term's variables live in the new condition
        obtain ⟨b, hb, hvb⟩ := List.mem_flatMap.mp hvx
        rw [hbind] at hb
        rcases List.mem_append.mp hb with hbpre | hbmid
        · exact mem_allVars.mpr (Or.inr (Or.inl
            ⟨⟨x.relation, bpre ++ (i, Term.var u) :: bpost⟩,
             List.mem_append.mpr (Or.inr (List.mem_cons_self ..)),
             List.mem_flatMap.mpr
               ⟨b, List.mem_append.mpr (Or.inl hbpre), hvb⟩⟩))
        · rcases List.mem_cons.mp hbmid with rfl | hbpost
          · refine mem_allVars.mpr (Or.inr (Or.inr (Or.inr
              ⟨.leaf ⟨.pointIn, .var u, t₀⟩,
               List.mem_append.mpr (Or.inr (List.mem_cons_self ..)),
               ?_⟩)))
            show v ∈ Term.vars (.var u) ++ t₀.vars
            exact List.mem_append.mpr (Or.inr hvb)
          · exact mem_allVars.mpr (Or.inr (Or.inl
              ⟨⟨x.relation, bpre ++ (i, Term.var u) :: bpost⟩,
               List.mem_append.mpr (Or.inr (List.mem_cons_self ..)),
               List.mem_flatMap.mpr
                 ⟨b, List.mem_append.mpr
                   (Or.inr (List.mem_cons_of_mem _ hbpost)), hvb⟩⟩))
      · exact mem_allVars.mpr (Or.inr (Or.inl
          ⟨x, List.mem_append.mpr
            (Or.inr (List.mem_cons_of_mem _ hxpost)), hvx⟩))
  · exact mem_allVars.mpr (Or.inr (Or.inr (Or.inl ⟨x, hx, hvx⟩)))
  · exact mem_allVars.mpr (Or.inr (Or.inr (Or.inr
      ⟨c, List.mem_append.mpr (Or.inl hc), hvc⟩)))

/-- The step subset, iterated: the full lowering never loses a
variable. -/
theorem lowerFuel_allVars_sub :
    ∀ (n : Nat) (Γ : Typing) (r : Rule) (v : VarId),
      v ∈ r.allVars → v ∈ (lowerFuel n Γ r).2.allVars
  | 0, _, _, _, h => h
  | n + 1, Γ, r, v, h => by
    cases hstep : stepLower Γ r.freshVar r with
    | none => simpa only [lowerFuel, hstep] using h
    | some out =>
      obtain ⟨Γ₁, r₁⟩ := out
      simp only [lowerFuel, hstep]
      exact lowerFuel_allVars_sub n Γ₁ r₁ v
        (stepLower_allVars_sub hstep v h)

/-- **The binding-level lowering, forward**: every surface derivation
of the written rule extends — at the mints alone — to one of the
lowered rule, agreeing on every variable the written rule mentions.
The mint takes the witnessing fact's field value at each step; the
fold companion (`membership_lowering_preserves_fold`,
`Exec/Dedup.lean`) spends exactly this agreement. -/
theorem lowerFuel_derives_forward (C : Classify) (I : Instance)
    (ρ : ParamEnv) :
    ∀ (n : Nat) (Γ : Typing) (r : Rule) {σ : Assignment},
      surfaceDerives Γ C r I ρ σ →
      ∃ σ', surfaceDerives (lowerFuel n Γ r).1 C
          (lowerFuel n Γ r).2 I ρ σ' ∧
        ∀ v, v ∈ r.allVars → σ' v = σ v
  | 0, _, _, σ, h => ⟨σ, h, fun _ _ => rfl⟩
  | n + 1, Γ, r, σ, h => by
    cases hstep : stepLower Γ r.freshVar r with
    | none =>
      simp only [lowerFuel, hstep]
      exact ⟨σ, h, fun _ _ => rfl⟩
    | some out =>
      obtain ⟨Γ₁, r₁⟩ := out
      simp only [lowerFuel, hstep]
      obtain ⟨σ₁, h₁, hag₁⟩ :=
        stepLower_derives_forward r.freshVar_not_mem hstep h
      obtain ⟨σ', h', hag'⟩ :=
        lowerFuel_derives_forward C I ρ n Γ₁ r₁ h₁
      refine ⟨σ', h', fun v hv => ?_⟩
      rw [hag' v (stepLower_allVars_sub hstep v hv)]
      exact hag₁ v fun heq => r.freshVar_not_mem (heq ▸ hv)

/-- **The binding-level lowering, backward**: an assignment deriving
the lowered rule derives the written rule UNCHANGED. -/
theorem lowerFuel_derives_backward (C : Classify) (I : Instance)
    (ρ : ParamEnv) :
    ∀ (n : Nat) (Γ : Typing) (r : Rule) {σ : Assignment},
      surfaceDerives (lowerFuel n Γ r).1 C (lowerFuel n Γ r).2 I ρ σ →
      surfaceDerives Γ C r I ρ σ
  | 0, _, _, _, h => h
  | n + 1, Γ, r, σ, h => by
    cases hstep : stepLower Γ r.freshVar r with
    | none => simpa only [lowerFuel, hstep] using h
    | some out =>
      obtain ⟨Γ₁, r₁⟩ := out
      simp only [lowerFuel, hstep] at h
      exact stepLower_derives_backward r.freshVar_not_mem hstep
        (lowerFuel_derives_backward C I ρ n Γ₁ r₁ h)

/-- On a rule whose atoms (both polarities) are membership-free, the
surface body judgment IS `derives` — the binding-level face of
`surface_eq_denotation_of_free`. -/
theorem surfaceDerives_iff_derives_of_free {Γ : Typing} {C : Classify}
    {r : Rule} {I : Instance} {ρ : ParamEnv} {σ : Assignment}
    (hpos : ∀ a, a ∈ r.atoms → Atom.membershipFree Γ a)
    (hneg : ∀ a, a ∈ r.negated → Atom.membershipFree Γ a) :
    surfaceDerives Γ C r I ρ σ ↔ derives C r I ρ σ := by
  constructor
  · rintro ⟨hp, hn, hc⟩
    refine ⟨?_, ?_, hc⟩
    · intro a ha
      obtain ⟨f, hf, hm⟩ := hp a ha
      exact ⟨f, hf, (surfaceMatches_of_membershipFree (hpos a ha)).mp hm⟩
    · rintro a ha ⟨f, hf, hm⟩
      exact hn a ha
        ⟨f, hf, (surfaceMatches_of_membershipFree (hneg a ha)).mpr hm⟩
  · rintro ⟨hp, hn, hc⟩
    refine ⟨?_, ?_, hc⟩
    · intro a ha
      obtain ⟨f, hf, hm⟩ := hp a ha
      exact ⟨f, hf, (surfaceMatches_of_membershipFree (hpos a ha)).mpr hm⟩
    · rintro a ha ⟨f, hf, hm⟩
      exact hn a ha
        ⟨f, hf, (surfaceMatches_of_membershipFree (hneg a ha)).mp hm⟩

/-! ## THE theorems -/

/-- **The full-roster half**: lowering preserves the SURFACE
denotation, with no hypothesis at all — var, param, literal, and
param-set terms, positive and negated atoms, repeated variables. The
lowered rule's positive atoms carry no membership binding
(`lowerFuel_posFree`). -/
theorem lowerMembership_preserves_surface (Γ : Typing) (C : Classify)
    (r : Rule) (I : Instance) (ρ : ParamEnv) :
    ∀ t, t ∈ surfaceRuleAnswers Γ C r I ρ ↔
      t ∈ surfaceRuleAnswers (r.lowerMembership Γ).1 C
        (r.lowerMembership Γ).2 I ρ :=
  fun t => lowerFuel_preserves C I ρ _ Γ r t

/-- **THE membership-lowering theorem — the seam-closer.** A written
rule's bivalent surface denotation equals THE denotation
(`ruleAnswers`, the pre-lowered `Matches`/`PointIn` form) of its
lowering, whenever the negated atoms are membership-free — the one
fragment the pre-lowered RULE syntax can spell (recorded narrowing: a
negated membership binding has no pre-lowered rule form; its lowered
home is the anti-probe occurrence, and
`membership_lowering_preserves_negated` below carries the full roster
against that form with no hypothesis).
The engine's normalize and the naive model each re-derive this
lowering; this theorem is the arbiter both are measured against.
Bridge: `ir/normalize/normalize.rs::is_membership` + `lower_atom` (the
engine's lowering), `ir/validate/context.rs::resolve_bivalents` (the
typing witness). -/
theorem membership_lowering_preserves (Γ : Typing) (C : Classify)
    (r : Rule) (I : Instance) (ρ : ParamEnv)
    (hneg : ∀ a, a ∈ r.negated → Atom.membershipFree Γ a) :
    ∀ t, t ∈ surfaceRuleAnswers Γ C r I ρ ↔
      t ∈ ruleAnswers C (r.lowerMembership Γ).2 I ρ := by
  intro t
  refine (lowerMembership_preserves_surface Γ C r I ρ t).trans
    (surface_eq_denotation_of_free ?_ ?_ t)
  · exact lowerFuel_posFree _ Γ r (Nat.le_refl _)
  · exact lowerFuel_negFree _ Γ r hneg

/-! ## The negated lowering — the anti-probe occurrence form

A negated membership binding has no pre-lowered RULE form (the mint
is unsafe under negation — module doc), so its lowered home is the
OCCURRENCE the engine executes: the anti-probe, domain bindings plus
membership filters, evaluated inside the probe. This section defines
that form, proves the role-blind lowering onto it answer-preserving
for the full roster (`membership_lowering_preserves_negated`, no
membership-free hypothesis), and composes it back to the plain
anti-join on membership-free negated atoms
(`antiprobe_eq_antijoin_of_negFree`) — closing the formerly recorded
remaining gap. -/

/-- A negated occurrence in the engine's lowered form: the domain
bindings (value reads — the anti-probe's probe keys) plus the
membership filters, each read as a same-fact `PointIn` against the
probed fact. Bridge: `ir/normalize/normalize.rs::lower_atom`
(role-blind: pass 1 keeps the domain bindings, pass 2 lowers the
membership positions to `PointIn`/`FieldsPointIn`/`AnyPointIn`
filters) and the `AntiProbe` descriptor (`probe_bindings` are the
occurrence's vars; its filters evaluate inside the probe). -/
structure AntiOccurrence where
  /-- The probed relation. -/
  relation : RelId
  /-- The value-read bindings — the probe keys. -/
  domain : List (FieldId × Term)
  /-- The membership positions — `PointIn` filters over the probed
  fact's interval fields. -/
  filters : List (FieldId × Term)

/-- The role-blind lowering of one negated atom: partition its
bindings by the membership status — exactly `lower_atom`'s two
passes, with no mint (a filter reads the fact in place; nothing
binds). -/
def Atom.lowerNegated (Γ : Typing) (a : Atom) : AntiOccurrence :=
  { relation := a.relation
    domain :=
      a.bindings.filter fun b => !(Γ.membership a.relation b.1 b.2)
    filters :=
      a.bindings.filter fun b => Γ.membership a.relation b.1 b.2 }

/-- A fact PASSES an anti-occurrence: it matches the domain bindings
by value and every membership filter's term selects a value whose
point lies in the fact's interval at the filter's field. A variable
scalar-anchored in the SAME negated atom needs no special case: the
domain binding pins `σ v` to the probed fact's field, so the filter's
read of `σ v` IS the engine's `FieldsPointIn` same-fact composition. -/
def AntiMatches (f : Fact) (o : AntiOccurrence) (σ : Assignment)
    (ρ : ParamEnv) : Prop :=
  Matches f ⟨o.relation, o.domain⟩ σ ρ ∧
  ∀ b, b ∈ o.filters → ∃ x, Term.selects ρ σ b.2 x ∧ x.pointMem (f b.1)

/-- The anti-probe rejection: no fact of the relation passes — the
`¬∃` the executor realizes by probe, filters inside. -/
def AntiOccurrence.rejects (o : AntiOccurrence) (I : Instance)
    (σ : Assignment) (ρ : ParamEnv) : Prop :=
  ¬ ∃ f, f ∈ I o.relation ∧ AntiMatches f o σ ρ

/-- The surface judgment IS the anti-occurrence pass, fact for fact —
`surfaceMatches_iff_occurrence` with the filter half carried by the
filtered binding list instead of the in-place quantification. -/
theorem surfaceMatches_iff_antiMatches {Γ : Typing} {f : Fact}
    {a : Atom} {σ : Assignment} {ρ : ParamEnv} :
    SurfaceMatches Γ f a σ ρ ↔ AntiMatches f (a.lowerNegated Γ) σ ρ := by
  rw [surfaceMatches_iff_occurrence]
  unfold AntiMatches Atom.lowerNegated
  constructor
  · rintro ⟨hdom, hflt⟩
    refine ⟨hdom, fun b hb => ?_⟩
    obtain ⟨hmem, hm⟩ := List.mem_filter.mp hb
    exact hflt b hmem hm
  · rintro ⟨hdom, hflt⟩
    refine ⟨hdom, fun b hb hm => ?_⟩
    exact hflt b (List.mem_filter.mpr ⟨hb, hm⟩)

/-- A negated atom's surface rejection is exactly its anti-probe's
rejection — the negated case of the lowering, one atom at a time. -/
theorem lowerNegated_rejects_iff {Γ : Typing} {a : Atom}
    {σ : Assignment} {ρ : ParamEnv} {I : Instance} :
    (¬ ∃ f, f ∈ I a.relation ∧ SurfaceMatches Γ f a σ ρ) ↔
      (a.lowerNegated Γ).rejects I σ ρ :=
  ⟨fun hn ⟨f, hf, h⟩ => hn ⟨f, hf, surfaceMatches_iff_antiMatches.mpr h⟩,
   fun hn ⟨f, hf, h⟩ => hn ⟨f, hf, surfaceMatches_iff_antiMatches.mp h⟩⟩

/-- The lowered rule denotation the engine executes: positive atoms
and conditions in the pre-lowered `Matches`/`Condition.holds` form
(the `ruleAnswers` reading), negated occurrences rejecting by
anti-probe. Bridge: `ir/normalize/normalize.rs::normalize_rule` — the
occurrence list plus the `anti_probes` descriptors. -/
def antiProbeDerives (Γ : Typing) (C : Classify) (r : Rule)
    (I : Instance) (ρ : ParamEnv) (σ : Assignment) : Prop :=
  (∀ a, a ∈ r.atoms → ∃ f, f ∈ I a.relation ∧ Matches f a σ ρ) ∧
  (∀ a, a ∈ r.negated → (a.lowerNegated Γ).rejects I σ ρ) ∧
  (∀ t, t ∈ r.conditions → Condition.holds C ρ σ t)

/-- One rule's answers under the anti-probe reading of its negated
atoms. -/
def antiProbeRuleAnswers (Γ : Typing) (C : Classify) (r : Rule)
    (I : Instance) (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ σ, antiProbeDerives Γ C r I ρ σ ∧ t = r.finds.map σ

/-- Membership in the anti-probe answers, unfolded. -/
theorem mem_antiProbeRuleAnswers {Γ : Typing} {C : Classify} {r : Rule}
    {I : Instance} {ρ : ParamEnv} {t : AnswerTuple} :
    t ∈ antiProbeRuleAnswers Γ C r I ρ ↔
      ∃ σ, antiProbeDerives Γ C r I ρ σ ∧ t = r.finds.map σ :=
  Iff.rfl

/-- On a rule whose POSITIVE atoms are membership-free, the surface
denotation IS the anti-probe denotation: positives collapse to
`Matches` (`surfaceMatches_of_membershipFree`), and every negated
atom — membership bindings or not — reads through its anti-probe
(`lowerNegated_rejects_iff`). -/
theorem surface_eq_antiprobe_of_posFree {Γ : Typing} {C : Classify}
    {r : Rule} {I : Instance} {ρ : ParamEnv}
    (hpos : ∀ a, a ∈ r.atoms → Atom.membershipFree Γ a) :
    ∀ t, t ∈ surfaceRuleAnswers Γ C r I ρ ↔
      t ∈ antiProbeRuleAnswers Γ C r I ρ := by
  intro t
  constructor
  · rintro ⟨σ, ⟨hp, hn, hc⟩, rfl⟩
    refine ⟨σ, ⟨?_, ?_, hc⟩, rfl⟩
    · intro a ha
      obtain ⟨f, hf, hm⟩ := hp a ha
      exact ⟨f, hf, (surfaceMatches_of_membershipFree (hpos a ha)).mp hm⟩
    · intro a ha
      exact lowerNegated_rejects_iff.mp (hn a ha)
  · rintro ⟨σ, ⟨hp, hn, hc⟩, rfl⟩
    refine ⟨σ, ⟨?_, ?_, hc⟩, rfl⟩
    · intro a ha
      obtain ⟨f, hf, hm⟩ := hp a ha
      exact ⟨f, hf, (surfaceMatches_of_membershipFree (hpos a ha)).mpr hm⟩
    · intro a ha
      exact lowerNegated_rejects_iff.mpr (hn a ha)

/-- **The negated case, closed — the full-roster lowering theorem.**
A written rule's bivalent surface denotation equals the anti-probe
denotation of its lowering, with NO hypothesis: positive membership
bindings lower to the minted `PointIn` conditions
(`Rule.lowerMembership`, run to exhaustion), and every negated atom's
membership bindings lower to its anti-probe's `PointIn` filters
(`Atom.lowerNegated` under the lowered typing — the mints are fresh
for the untouched negated atoms, so their membership statuses are
unchanged, `lowerFuel_negFree`'s argument). This is the arbiter for
the engine's role-blind `lower_atom` on NEGATED occurrences, the half
`membership_lowering_preserves` could not spell in rule syntax.
Bridge: `ir/normalize/normalize.rs::lower_atom` ("positive or negated
— the rules are identical"); the `AntiProbe` descriptors carry the
filters the executor evaluates inside the probe. -/
theorem membership_lowering_preserves_negated (Γ : Typing)
    (C : Classify) (r : Rule) (I : Instance) (ρ : ParamEnv) :
    ∀ t, t ∈ surfaceRuleAnswers Γ C r I ρ ↔
      t ∈ antiProbeRuleAnswers (r.lowerMembership Γ).1 C
        (r.lowerMembership Γ).2 I ρ :=
  fun t => (lowerMembership_preserves_surface Γ C r I ρ t).trans
    (surface_eq_antiprobe_of_posFree
      (lowerFuel_posFree _ Γ r (Nat.le_refl _)) t)

/-- **The anti-join composition.** On membership-free negated atoms
the anti-probe denotation IS the plain anti-join denotation
(`ruleAnswers`): the filters are empty and the domain is the whole
binding list, so a pass is a match and a rejection is the `¬∃` of
`derives`. Composing this with
`membership_lowering_preserves_negated` recovers exactly
`membership_lowering_preserves`'s scope — the hypothesis there is the
price of speaking rule syntax, not a semantic boundary. -/
theorem antiprobe_eq_antijoin_of_negFree {Γ : Typing} {C : Classify}
    {r : Rule} {I : Instance} {ρ : ParamEnv}
    (hneg : ∀ a, a ∈ r.negated → Atom.membershipFree Γ a) :
    ∀ t, t ∈ antiProbeRuleAnswers Γ C r I ρ ↔
      t ∈ ruleAnswers C r I ρ := by
  intro t
  constructor
  · rintro ⟨σ, ⟨hp, hn, hc⟩, rfl⟩
    refine mem_ruleAnswers.mpr ⟨σ, ⟨hp, ?_, hc⟩, rfl⟩
    rintro a ha ⟨f, hf, hm⟩
    exact lowerNegated_rejects_iff.mpr (hn a ha)
      ⟨f, hf, (surfaceMatches_of_membershipFree (hneg a ha)).mpr hm⟩
  · rintro ⟨σ, ⟨hp, hn, hc⟩, rfl⟩
    refine ⟨σ, ⟨hp, ?_, hc⟩, rfl⟩
    intro a ha
    refine lowerNegated_rejects_iff.mp ?_
    rintro ⟨f, hf, hm⟩
    exact hn a ha
      ⟨f, hf, (surfaceMatches_of_membershipFree (hneg a ha)).mp hm⟩

end Query
end Bumbledb
