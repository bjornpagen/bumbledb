import Bumbledb.Query.Denotation

/-!
# Exec/Reach — interior DAG, one linear reach, the query denotation

Level 0: `evalInteriors`, `reachOp`, `reachDen = lfpS`, `evalQuery`.
The denotation is `evalQuery`; the budget is a resource abort.

Level 1: `evalLinearReach`, `evalQueryList`, proved equal to Level 0.
`fueledLoop` is a **private** termination metric (`missingCount_le` is
why `cands.length + 1` always suffices). It is not a parameter of any
public def and not a Bridge incompleteness caveat.

Engine `DerivedBudgetExceeded` is incompleteness vs `evalQuery` —
one derived-tuples ledger over interior tables and `reachDen` alike —
the same class as `ResultBytesOverflow` vs `rulesAnswers`.

## Narrowings recorded (law 5)

* **`evalLinearReach` candidates.** The proved evaluator uses the finite
  union of `allTuples recDom r.finds.length` over `baseRules ++
  stepRules`.
* **Private `fueledLoop_fixpoint`.** Inflation (`acc ⊆ step acc`) is
  an invariant hypothesis, not a property of every list. Public
  `reachStep` stays `T(acc) = base ++ rec(acc)` as specified; the
  naive chain from `[]` is inflationary by `reachOp_mono`.
-/

namespace Bumbledb.Query

/-! ## Least fixpoint of a set operator -/

/-- Least prefixed point of a set operator. -/
def lfpS {α : Type u} (T : Set α → Set α) : Set α :=
  fun a => ∀ X, (∀ x, x ∈ T X → x ∈ X) → a ∈ X

/-- Monotone in the recursive argument. -/
def MonoS {α : Type u} (T : Set α → Set α) : Prop :=
  ∀ X Y, (∀ a, a ∈ X → a ∈ Y) → ∀ a, a ∈ T X → a ∈ T Y

theorem lfpS_le {α} {T : Set α → Set α} {X : Set α}
    (h : ∀ x, x ∈ T X → x ∈ X) : ∀ a, a ∈ lfpS T → a ∈ X :=
  fun _ ht => ht X h

theorem lfpS_prefixed {α} {T : Set α → Set α} (hm : MonoS T) :
    ∀ a, a ∈ T (lfpS T) → a ∈ lfpS T := by
  intro a ht X hX
  exact hX a (hm (lfpS T) X (lfpS_le hX) a ht)

theorem lfpS_postfixed {α} {T : Set α → Set α} (hm : MonoS T) :
    ∀ a, a ∈ lfpS T → a ∈ T (lfpS T) :=
  lfpS_le (hm _ _ (lfpS_prefixed hm))

/-- **Knaster–Tarski** over one set: a monotone operator's `lfpS`
is a fixed point — and `lfpS_le` makes it the least one. -/
theorem lfpS_fixed {α} {T : Set α → Set α} (hm : MonoS T) :
    ∀ a, a ∈ T (lfpS T) ↔ a ∈ lfpS T :=
  fun a => ⟨fun h => lfpS_prefixed hm a h, fun h => lfpS_postfixed hm a h⟩

/-! ## Environment lemmas -/

theorem InteriorEnv.update_self (W : InteriorEnv) (c : InteriorId)
    (X : Set AnswerTuple) : W.update c X c = X := by
  simp [InteriorEnv.update]

theorem InteriorEnv.update_ne (W : InteriorEnv) {c d : InteriorId}
    (X : Set AnswerTuple) (h : d ≠ c) : W.update c X d = W d := by
  simp [InteriorEnv.update, h]

theorem ruleAnswers_congr {C : Classify} {r : Rule} {ρ : ParamEnv}
    {F G : AtomSource → Set Fact}
    (h : ∀ s f, f ∈ F s ↔ f ∈ G s) :
    ∀ t, t ∈ ruleAnswers C r F ρ ↔ t ∈ ruleAnswers C r G ρ := by
  intro t
  constructor
  · rintro ⟨σ, hd, ht⟩
    exact ⟨σ, (derives_congr h).mp hd, ht⟩
  · rintro ⟨σ, hd, ht⟩
    exact ⟨σ, (derives_congr h).mpr hd, ht⟩

theorem rulesAnswers_congr {C : Classify} {rules : List Rule}
    {ρ : ParamEnv} {F G : AtomSource → Set Fact}
    (h : ∀ s f, f ∈ F s ↔ f ∈ G s) :
    ∀ t, t ∈ rulesAnswers C rules F ρ ↔ t ∈ rulesAnswers C rules G ρ := by
  intro t
  constructor
  · rintro ⟨r, hr, ht⟩
    exact ⟨r, hr, (ruleAnswers_congr h t).mp ht⟩
  · rintro ⟨r, hr, ht⟩
    exact ⟨r, hr, (ruleAnswers_congr h t).mpr ht⟩

theorem derives_mono_pos {C : Classify} {r : Rule} {ρ : ParamEnv}
    {σ : Assignment} {F G : AtomSource → Set Fact}
    (hF : ∀ s f, f ∈ F s → f ∈ G s) (hneg : r.negated = []) :
    derives C r F ρ σ → derives C r G ρ σ := by
  rintro ⟨hpos, -, hcond⟩
  refine ⟨fun a ha => ?_, fun a ha => ?_, hcond⟩
  · obtain ⟨f, hf, hm⟩ := hpos a ha
    exact ⟨f, hF _ f hf, hm⟩
  · rw [hneg] at ha
    cases ha

theorem ruleAnswers_mono_pos {C : Classify} {r : Rule} {ρ : ParamEnv}
    {F G : AtomSource → Set Fact}
    (hF : ∀ s f, f ∈ F s → f ∈ G s) (hneg : r.negated = []) :
    ∀ t, t ∈ ruleAnswers C r F ρ → t ∈ ruleAnswers C r G ρ := by
  rintro t ⟨σ, hd, ht⟩
  exact ⟨σ, derives_mono_pos hF hneg hd, ht⟩

theorem rulesAnswers_mono_pos {C : Classify} {rules : List Rule}
    {ρ : ParamEnv} {F G : AtomSource → Set Fact}
    (hF : ∀ s f, f ∈ F s → f ∈ G s)
    (hneg : ∀ r, r ∈ rules → r.negated = []) :
    ∀ t, t ∈ rulesAnswers C rules F ρ → t ∈ rulesAnswers C rules G ρ := by
  rintro t ⟨r, hr, ht⟩
  exact ⟨r, hr, ruleAnswers_mono_pos hF (hneg r hr) t ht⟩

theorem sourceDen_toEnv_update (W : ListInstance) (T : InteriorTables)
    (c : InteriorId) (rows : List AnswerTuple) :
    ∀ s f, f ∈ sourceDen W.den (T.update c rows).toEnv s ↔
      f ∈ sourceDen W.den (InteriorEnv.update T.toEnv c (fun u => u ∈ rows)) s :=
  sourceDen_congr (InteriorTables.toEnv_update T c rows)

/-! ## Interiors — `evalInteriors` -/

/-- Declaration-order fold. Slot `⟨i⟩` is the publish id of the `i`th
interior; a later or unknown id is empty because it was never updated. -/
def evalInteriorsFold (C : Classify) (I : Instance) (ρ : ParamEnv) :
    Nat → List Interior → InteriorEnv → InteriorEnv
  | _, [], W => W
  | i, d :: ds, W =>
      evalInteriorsFold C I ρ (i + 1) ds
        (W.update ⟨i⟩ (fun t => t ∈ rulesAnswers C d.rules (sourceDen I W) ρ))

def evalInteriors (C : Classify) (defs : List Interior) (I : Instance)
    (ρ : ParamEnv) : InteriorEnv :=
  evalInteriorsFold C I ρ 0 defs InteriorEnv.empty

/-! ## Reach — `reachOp`, `reachDen` -/

/-- The reach operator. Base does not see `X`. Step arms see `X` at
`self` and finished interiors in `W`. Negation is unrepresentable in
`LinearRec`, so monotonicity is structural. -/
def reachOp (C : Classify) (rec : LinearRec) (self : InteriorId)
    (I : Instance) (W : InteriorEnv) (ρ : ParamEnv)
    (X : Set AnswerTuple) : Set AnswerTuple :=
  fun t =>
    t ∈ rulesAnswers C rec.baseRules (sourceDen I W) ρ ∨
    t ∈ rulesAnswers C (rec.stepRules self) (sourceDen I (W.update self X)) ρ

def reachDen (C : Classify) (rec : LinearRec) (self : InteriorId)
    (I : Instance) (W : InteriorEnv) (ρ : ParamEnv) : Set AnswerTuple :=
  lfpS (reachOp C rec self I W ρ)

/-- The roster's no-negation-in-rec is structural on `LinearRec`, so
the reach operator is monotone with no extra premise. The wall is the
self case (`Countermodels.odd_not_monotone`); do not cite this theorem
as evidence that finished-table negation in a rec arm is non-monotone. -/
theorem reachOp_mono {C : Classify} {rec : LinearRec} {self : InteriorId}
    {I : Instance} {W : InteriorEnv} {ρ : ParamEnv} :
    MonoS (reachOp C rec self I W ρ) := by
  intro X Y hXY t ht
  rcases ht with hbase | hrec
  · exact Or.inl hbase
  · refine Or.inr (rulesAnswers_mono_pos
      (sourceDen_mono (InteriorEnv.update_le hXY))
      (fun r hr => LinearRec.stepRules_negated self rec hr) t hrec)

/-- Round 0 is base: a positive self-atom against an empty table
derives nothing, so `rec(∅) = ∅` and `T(∅) = base`. -/
theorem reachOp_empty {C : Classify} {rec : LinearRec} {self : InteriorId}
    {I : Instance} {W : InteriorEnv} {ρ : ParamEnv} :
    ∀ t, t ∈ reachOp C rec self I W ρ (fun _ => False) ↔
         t ∈ rulesAnswers C rec.baseRules (sourceDen I W) ρ := by
  intro t
  constructor
  · rintro (hbase | hrec)
    · exact hbase
    · obtain ⟨r, hr, ⟨σ, ⟨hpos', -, -⟩, -⟩⟩ := mem_rulesAnswers.mp hrec
      obtain ⟨a, ha, hsrc⟩ := LinearRec.stepRules_self_atom self rec hr
      obtain ⟨f, hf, -⟩ := hpos' a ha
      rw [hsrc] at hf
      obtain ⟨u, hu, -⟩ := hf
      unfold InteriorEnv.update at hu
      rw [if_pos rfl] at hu
      exact False.elim hu
  · exact Or.inl

/-! ## The candidate space -/

/-- Every tuple of a given length over a domain. -/
def allTuples (dom : List Value) : Nat → List AnswerTuple
  | 0 => [[]]
  | n + 1 => dom.flatMap fun v => (allTuples dom n).map (v :: ·)

theorem mem_allTuples {dom : List Value} :
    ∀ {n : Nat} {t : AnswerTuple},
      t ∈ allTuples dom n ↔ (t.length = n ∧ ∀ v, v ∈ t → v ∈ dom)
  | 0, t => by
    show t ∈ [[]] ↔ _
    rw [List.mem_singleton]
    constructor
    · rintro rfl
      exact ⟨rfl, fun v hv => absurd hv (by simp)⟩
    · rintro ⟨hlen, -⟩
      exact List.eq_nil_of_length_eq_zero hlen
  | n + 1, t => by
    show t ∈ dom.flatMap _ ↔ _
    constructor
    · intro h
      obtain ⟨v, hv, hmem⟩ := List.mem_flatMap.mp h
      obtain ⟨t', ht', rfl⟩ := List.mem_map.mp hmem
      obtain ⟨hlen, hall⟩ := (mem_allTuples (n := n)).mp ht'
      refine ⟨by simp [hlen], fun w hw => ?_⟩
      rcases List.mem_cons.mp hw with rfl | hw'
      · exact hv
      · exact hall w hw'
    · rintro ⟨hlen, hall⟩
      cases t with
      | nil => simp at hlen
      | cons v t' =>
        refine List.mem_flatMap.mpr
          ⟨v, hall v (List.mem_cons_self ..), List.mem_map.mpr
            ⟨t', (mem_allTuples (n := n)).mpr
              ⟨by simpa using hlen,
                fun w hw => hall w (List.mem_cons_of_mem _ hw)⟩, rfl⟩⟩

/-- Active domain of a rec: filler plus stored columns and finished
interior columns. Ignores the accumulating self: its rows are already
candidates. -/
def LinearRec.nonSelfAtoms (rec : LinearRec) : List Atom :=
  rec.baseRules.flatMap (·.atoms) ++
    (rec.step.1 :: rec.step.2).flatMap (·.atoms)

def recDom (rec : LinearRec) (self : InteriorId) (W : ListInstance)
    (V : InteriorTables) : List Value :=
  let _ := self
  fillerValue ::
    rec.nonSelfAtoms.flatMap fun a =>
      match a.source with
      | .edb R => a.bindings.flatMap fun b => (W.facts R).map (· b.1)
      | .interior C => a.bindings.flatMap fun b =>
          (V C).flatMap fun t => (t[b.1.id]?).toList

/-- Candidate tuples: the finite product at each rule's head length. -/
def recCands (rec : LinearRec) (self : InteriorId) (W : ListInstance)
    (V : InteriorTables) : List AnswerTuple :=
  (rec.rules self).flatMap fun r =>
    allTuples (recDom rec self W V) r.finds.length

/-! ## Private termination metric -/

theorem length_filter_mono {α : Type} {l : List α} {p q : α → Bool}
    (h : ∀ a, q a = true → p a = true) :
    (l.filter q).length ≤ (l.filter p).length := by
  induction l with
  | nil => exact Nat.le_refl _
  | cons a l ih =>
    rw [List.filter_cons, List.filter_cons]
    by_cases hq : q a = true
    · rw [if_pos hq, if_pos (h a hq)]
      exact Nat.succ_le_succ ih
    · rw [if_neg hq]
      by_cases hp : p a = true
      · rw [if_pos hp]
        exact Nat.le_succ_of_le ih
      · rw [if_neg hp]
        exact ih

theorem length_filter_lt {α : Type} {l : List α} {p q : α → Bool}
    (h : ∀ a, q a = true → p a = true) {x : α} (hx : x ∈ l)
    (hpx : p x = true) (hqx : q x = false) :
    (l.filter q).length < (l.filter p).length := by
  induction l with
  | nil => cases hx
  | cons a l ih =>
    rw [List.filter_cons, List.filter_cons]
    rcases List.mem_cons.mp hx with rfl | hx'
    · rw [if_pos hpx, if_neg (by simp [hqx])]
      exact Nat.lt_succ_of_le (length_filter_mono h)
    · by_cases hq : q a = true
      · rw [if_pos hq, if_pos (h a hq)]
        exact Nat.succ_lt_succ (ih hx')
      · rw [if_neg hq]
        by_cases hp : p a = true
        · rw [if_pos hp]
          exact Nat.lt_succ_of_lt (ih hx')
        · rw [if_neg hp]
          exact ih hx'

section FueledLoop

variable {α : Type} [DecidableEq α]

private def fueledLoop (step : List α → List α) : Nat → List α → List α
  | 0, acc => acc
  | fuel + 1, acc =>
    if (step acc).all (fun x => decide (x ∈ acc)) then acc
    else fueledLoop step fuel (step acc)

private def missingCount (cands acc : List α) : Nat :=
  (cands.filter fun c => decide (c ∉ acc)).length

private theorem missingCount_le (cands acc : List α) :
    missingCount cands acc ≤ cands.length :=
  List.length_filter_le _ _

/-- Inflation is an invariant, not a property of every list — public
`reachStep` is `T(acc)`, and the chain from `[]` is inflationary. -/
private theorem fueledLoop_fixpoint (step : List α → List α)
    (cands : List α) (Inv : List α → Prop)
    (hext : ∀ acc, Inv acc → ∀ x, x ∈ acc → x ∈ step acc)
    (hinv : ∀ acc, Inv acc → Inv (step acc))
    (hbound : ∀ acc, Inv acc → ∀ x, x ∈ step acc → x ∈ acc ∨ x ∈ cands) :
    ∀ fuel acc, Inv acc → missingCount cands acc < fuel →
      Inv (fueledLoop step fuel acc) ∧
      (∀ x, x ∈ acc → x ∈ fueledLoop step fuel acc) ∧
      (∀ x, x ∈ step (fueledLoop step fuel acc) →
        x ∈ fueledLoop step fuel acc)
  | 0, _, _, hfuel => absurd hfuel (by omega)
  | fuel + 1, acc, hI, hfuel => by
    have hunfold : fueledLoop step (fuel + 1) acc =
        if (step acc).all (fun x => decide (x ∈ acc)) then acc
        else fueledLoop step fuel (step acc) := rfl
    rw [hunfold]
    by_cases hstop : (step acc).all (fun x => decide (x ∈ acc)) = true
    · rw [if_pos hstop]
      refine ⟨hI, fun x hx => hx, fun x hx => ?_⟩
      exact of_decide_eq_true (List.all_eq_true.mp hstop x hx)
    · rw [if_neg hstop]
      have hex : ∃ x, x ∈ step acc ∧ x ∉ acc := by
        apply Classical.byContradiction
        intro hall
        apply hstop
        refine List.all_eq_true.mpr fun x hx => decide_eq_true ?_
        exact Classical.byContradiction fun hxn => hall ⟨x, hx, hxn⟩
      obtain ⟨x, hxs, hxn⟩ := hex
      have hxc : x ∈ cands := (hbound acc hI x hxs).resolve_left hxn
      have hdec : missingCount cands (step acc) <
          missingCount cands acc := by
        refine length_filter_lt ?_ hxc (decide_eq_true hxn)
          (decide_eq_false fun h => h hxs)
        intro a ha
        exact decide_eq_true
          (fun hmem => of_decide_eq_true ha (hext acc hI a hmem))
      have hrec := fueledLoop_fixpoint step cands Inv hext hinv hbound
        fuel (step acc) (hinv acc hI) (by omega)
      exact ⟨hrec.1, fun y hy => hrec.2.1 y (hext acc hI y hy), hrec.2.2⟩

end FueledLoop

/-! ## `evalLinearReach` -/

/-- Naive step: `T(acc) = base ∪ rec(acc)`. List concat is the union;
membership is the set. This is **not** the engine's delta loop. -/
def reachStep (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (rec : LinearRec) (self : InteriorId) (V : InteriorTables)
    (base : List AnswerTuple) (acc : List AnswerTuple) :
    List AnswerTuple :=
  let T := V.update self acc
  base ++ evalList C W T ρ (rec.stepRules self)

def evalLinearReach (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (rec : LinearRec) (self : InteriorId) (V : InteriorTables) : List AnswerTuple :=
  let base := evalList C W V ρ rec.baseRules
  let cands := recCands rec self W V
  fueledLoop (reachStep C W ρ rec self V base) (cands.length + 1) []

theorem mem_reachStep {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {rec : LinearRec} {self : InteriorId} {V : InteriorTables}
    {base acc : List AnswerTuple} {t : AnswerTuple} :
    t ∈ reachStep C W ρ rec self V base acc ↔
      t ∈ base ∨ t ∈ evalList C W (V.update self acc) ρ (rec.stepRules self) := by
  simp [reachStep, List.mem_append]

theorem evalRule_length {C : Classify} {W : ListInstance}
    {T : InteriorTables} {ρ : ParamEnv} {r : Rule} {t : AnswerTuple}
    (ht : t ∈ evalRule C W T ρ r) : t.length = r.finds.length := by
  obtain ⟨σp, -, rfl⟩ := List.mem_map.mp ht
  simp

theorem recDom_edb {rec : LinearRec} {self : InteriorId} {W : ListInstance}
    {V : InteriorTables} {a : Atom} (ha : a ∈ rec.nonSelfAtoms)
    {R : RelId} (hsrc : a.source = .edb R)
    {b : FieldId × Term} (hb : b ∈ a.bindings) {f : Fact}
    (hf : f ∈ W.facts R) : f b.1 ∈ recDom rec self W V := by
  refine List.mem_cons_of_mem _ (List.mem_flatMap.mpr ⟨a, ha, ?_⟩)
  rw [hsrc]
  exact List.mem_flatMap.mpr ⟨b, hb, List.mem_map.mpr ⟨f, hf, rfl⟩⟩

theorem recDom_interior {rec : LinearRec} {self : InteriorId}
    {W : ListInstance} {V : InteriorTables} {a : Atom}
    (ha : a ∈ rec.nonSelfAtoms) {C : InteriorId} (hsrc : a.source = .interior C)
    {b : FieldId × Term} (hb : b ∈ a.bindings) {row : AnswerTuple}
    (hrow : row ∈ V C) {v : Value} (hv : row[b.1.id]? = some v) :
    v ∈ recDom rec self W V := by
  refine List.mem_cons_of_mem _ (List.mem_flatMap.mpr ⟨a, ha, ?_⟩)
  rw [hsrc]
  exact List.mem_flatMap.mpr ⟨b, hb, List.mem_flatMap.mpr
    ⟨row, hrow, by simp [hv]⟩⟩

theorem recDom_filler (rec : LinearRec) (self : InteriorId)
    (W : ListInstance) (V : InteriorTables) :
    fillerValue ∈ recDom rec self W V :=
  List.mem_cons_self ..

theorem evalRule_in_cands {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {rec : LinearRec} {self : InteriorId} {V : InteriorTables}
    {acc : List AnswerTuple} {r : Rule}
    (hr : r ∈ rec.rules self)
    (hsafe : Safe r)
    (hacc : ∀ u, u ∈ acc → u ∈ recCands rec self W V) {t : AnswerTuple}
    (ht : t ∈ evalRule C W (V.update self acc) ρ r) :
    t ∈ recCands rec self W V := by
  have hans : t ∈ ruleAnswers C r
      (sourceDen W.den (V.update self acc).toEnv) ρ := evalRule_sound ht
  have hlen : t.length = r.finds.length := evalRule_length ht
  refine List.mem_flatMap.mpr ⟨r, hr, mem_allTuples.mpr ⟨hlen, fun v hv => ?_⟩⟩
  have hdom := antijoin_over_active_domain hsafe t hans v hv
  obtain ⟨a, ha, f, hf0, b, hb, hfb⟩ := hdom
  have hf : f ∈ sourceDen W.den (V.update self acc).toEnv a.source := hf0
  cases hsrc : a.source with
  | edb R =>
    rw [hsrc] at hf
    rw [← hfb]
    have haNS : a ∈ rec.nonSelfAtoms := by
      rcases List.mem_append.mp hr with hbase | hstep
      · exact List.mem_append.mpr (Or.inl (List.mem_flatMap.mpr ⟨r, hbase, ha⟩))
      · obtain ⟨s, hs, rfl⟩ := List.mem_map.mp hstep
        rcases List.mem_cons.mp ha with hself | haRest
        · cases hself; nomatch hsrc
        · exact List.mem_append.mpr (Or.inr (List.mem_flatMap.mpr ⟨s, hs, haRest⟩))
    exact recDom_edb haNS hsrc hb hf
  | interior Q =>
    rw [hsrc] at hf
    obtain ⟨row, hrow, rfl⟩ := hf
    rw [← hfb]
    unfold InteriorTables.toEnv InteriorTables.update at hrow
    by_cases hselfBind : Q = self
    · rw [if_pos hselfBind] at hrow
      rcases tupleFact_mem_or_filler row b.1 with hmem | hfill
      · have hrowc : row ∈ recCands rec self W V := hacc row hrow
        obtain ⟨_, _, htup⟩ := List.mem_flatMap.mp hrowc
        exact (mem_allTuples.mp htup).2 _ hmem
      · rw [hfill]
        exact recDom_filler rec self W V
    · rw [if_neg hselfBind] at hrow
      have haNS : a ∈ rec.nonSelfAtoms := by
        rcases List.mem_append.mp hr with hbase | hstep
        · exact List.mem_append.mpr (Or.inl (List.mem_flatMap.mpr ⟨r, hbase, ha⟩))
        · obtain ⟨s, hs, rfl⟩ := List.mem_map.mp hstep
          rcases List.mem_cons.mp ha with hself | haRest
          · cases hself
            injection hsrc with hQ
            exact (hselfBind hQ.symm).elim
          · exact List.mem_append.mpr (Or.inr (List.mem_flatMap.mpr ⟨s, hs, haRest⟩))
      cases hidx : row[b.1.id]? with
      | none =>
        have : tupleFact row b.1 = fillerValue := by
          unfold tupleFact; simp [hidx]
        rw [this]
        exact recDom_filler rec self W V
      | some w =>
        have : tupleFact row b.1 = w := by
          unfold tupleFact; simp [hidx]
        rw [this]
        exact recDom_interior haNS hsrc hb hrow hidx

theorem evalList_in_cands {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {rec : LinearRec} {self : InteriorId} {V : InteriorTables}
    {acc : List AnswerTuple} {rules : List Rule}
    (hrules : ∀ r, r ∈ rules → r ∈ rec.rules self)
    (hsafe : ∀ r, r ∈ rules → Safe r)
    (hacc : ∀ u, u ∈ acc → u ∈ recCands rec self W V) {t : AnswerTuple}
    (ht : t ∈ evalList C W (V.update self acc) ρ rules) :
    t ∈ recCands rec self W V := by
  obtain ⟨r, hr, ht'⟩ := List.mem_flatMap.mp ht
  exact evalRule_in_cands (hrules r hr) (hsafe r hr) hacc ht'

theorem evalRule_base_in_cands {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {rec : LinearRec} {self : InteriorId} {V : InteriorTables}
    {r : Rule}
    (hr : r ∈ rec.baseRules) (hsafe : Safe r) {t : AnswerTuple}
    (ht : t ∈ evalRule C W V ρ r) :
    t ∈ recCands rec self W V := by
  have hans : t ∈ ruleAnswers C r (sourceDen W.den V.toEnv) ρ :=
    evalRule_sound ht
  have hlen : t.length = r.finds.length := evalRule_length ht
  refine List.mem_flatMap.mpr ⟨r, List.mem_append.mpr (Or.inl hr),
    mem_allTuples.mpr ⟨hlen, fun v hv => ?_⟩⟩
  have hdom := antijoin_over_active_domain hsafe t hans v hv
  obtain ⟨a, ha, f, hf0, b, hb, hfb⟩ := hdom
  have hf : f ∈ sourceDen W.den V.toEnv a.source := hf0
  have haNS : a ∈ rec.nonSelfAtoms :=
    List.mem_append.mpr (Or.inl (List.mem_flatMap.mpr ⟨r, hr, ha⟩))
  cases hsrc : a.source with
  | edb R =>
    rw [hsrc] at hf
    rw [← hfb]
    exact recDom_edb haNS hsrc hb hf
  | interior Q =>
    rw [hsrc] at hf
    obtain ⟨row, hrow, rfl⟩ := hf
    rw [← hfb]
    cases hidx : row[b.1.id]? with
    | none =>
      have : tupleFact row b.1 = fillerValue := by
        unfold tupleFact; simp [hidx]
      rw [this]
      exact recDom_filler rec self W V
    | some w =>
      have : tupleFact row b.1 = w := by
        unfold tupleFact; simp [hidx]
      rw [this]
      exact recDom_interior haNS hsrc hb hrow hidx

theorem evalList_base_in_cands {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {rec : LinearRec} {self : InteriorId} {V : InteriorTables}
    {t : AnswerTuple}
    (hsafe : ∀ r, r ∈ rec.baseRules → Safe r)
    (ht : t ∈ evalList C W V ρ rec.baseRules) :
    t ∈ recCands rec self W V := by
  obtain ⟨r, hr, ht'⟩ := List.mem_flatMap.mp ht
  exact evalRule_base_in_cands hr (hsafe r hr) ht'

theorem mem_reachStep_op {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {rec : LinearRec} {self : InteriorId} {V : InteriorTables}
    (hsafeB : ∀ r, r ∈ rec.baseRules → Safe r)
    (hwtB : ∀ r, r ∈ rec.baseRules → r.WellTyped)
    (hsafeR : ∀ r, r ∈ rec.stepRules self → Safe r)
    (hwtR : ∀ r, r ∈ rec.stepRules self → r.WellTyped)
    {acc : List AnswerTuple} {t : AnswerTuple} :
    t ∈ reachStep C W ρ rec self V (evalList C W V ρ rec.baseRules) acc ↔
      t ∈ reachOp C rec self W.den V.toEnv ρ (fun u => u ∈ acc) := by
  rw [mem_reachStep]
  constructor
  · rintro (hbase | hrec)
    · exact Or.inl ((eval_sound hsafeB hwtB t).mp hbase)
    · refine Or.inr ?_
      have := (eval_sound hsafeR hwtR t).mp hrec
      exact (rulesAnswers_congr (sourceDen_toEnv_update W V self acc)
        t).mp this
  · rintro (hbase | hrec)
    · exact Or.inl ((eval_sound hsafeB hwtB t).mpr hbase)
    · refine Or.inr ?_
      exact (eval_sound hsafeR hwtR t).mpr
        ((rulesAnswers_congr (sourceDen_toEnv_update W V self acc) t).mpr
          hrec)

/-- The executable reach lists exactly `reachDen`. No fuel hypothesis. -/
theorem evalLinearReach_eq_lfp {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {rec : LinearRec} {self : InteriorId} {V : InteriorTables}
    (hsafe : ∀ r, r ∈ rec.rules self → Safe r)
    (hwt : ∀ r, r ∈ rec.rules self → r.WellTyped) :
    ∀ t, t ∈ evalLinearReach C W ρ rec self V ↔
         t ∈ reachDen C rec self W.den V.toEnv ρ := by
  intro t
  let base := evalList C W V ρ rec.baseRules
  let step := reachStep C W ρ rec self V base
  let cands := recCands rec self W V
  let T := reachOp C rec self W.den V.toEnv ρ
  have hm : MonoS T := reachOp_mono
  have hsafeB : ∀ r, r ∈ rec.baseRules → Safe r :=
    fun r hr => hsafe r (List.mem_append.mpr (Or.inl hr))
  have hwtB : ∀ r, r ∈ rec.baseRules → r.WellTyped :=
    fun r hr => hwt r (List.mem_append.mpr (Or.inl hr))
  have hsafeR : ∀ r, r ∈ rec.stepRules self → Safe r :=
    fun r hr => hsafe r (List.mem_append.mpr (Or.inr hr))
  have hwtR : ∀ r, r ∈ rec.stepRules self → r.WellTyped :=
    fun r hr => hwt r (List.mem_append.mpr (Or.inr hr))
  have hop :
      ∀ {acc t}, t ∈ reachStep C W ρ rec self V base acc ↔
        t ∈ reachOp C rec self W.den V.toEnv ρ (fun u => u ∈ acc) :=
    fun {acc t} =>
      mem_reachStep_op (C := C) (W := W) (ρ := ρ) hsafeB hwtB hsafeR hwtR
  let Inv : List AnswerTuple → Prop := fun acc =>
    (∀ u, u ∈ acc → u ∈ reachDen C rec self W.den V.toEnv ρ) ∧
    (∀ u, u ∈ acc → u ∈ cands) ∧
    (∀ u, u ∈ acc → u ∈ step acc)
  have hext : ∀ acc, Inv acc → ∀ x, x ∈ acc → x ∈ step acc :=
    fun acc hI => hI.2.2
  have hinv : ∀ acc, Inv acc → Inv (step acc) := by
    intro acc hI
    refine ⟨?sub, ?cand, ?ext⟩
    · intro u hu
      have huT : u ∈ T (fun v => v ∈ acc) :=
        (hop (acc := acc) (t := u)).mp hu
      have huT' : u ∈ T (reachDen C rec self W.den V.toEnv ρ) :=
        hm (fun v => v ∈ acc) _ hI.1 u huT
      exact (lfpS_fixed hm u).mp huT'
    · intro u hu
      rcases (mem_reachStep (C := C) (W := W) (ρ := ρ) (t := u)).mp hu with
        hbase | hrec
      · exact evalList_base_in_cands hsafeB hbase
      · exact evalList_in_cands (fun r hr => List.mem_append.mpr (Or.inr hr))
          hsafeR hI.2.1 hrec
    · intro u hu
      have huT : u ∈ T (fun v => v ∈ acc) :=
        (hop (acc := acc) (t := u)).mp hu
      have huT' : u ∈ T (fun v => v ∈ step acc) :=
        hm (fun v => v ∈ acc) _ hI.2.2 u huT
      exact (hop (acc := step acc) (t := u)).mpr huT'
  have hbound : ∀ acc, Inv acc → ∀ x, x ∈ step acc → x ∈ acc ∨ x ∈ cands :=
    fun acc hI x hx => Or.inr ((hinv acc hI).2.1 x hx)
  have hI0 : Inv [] := by
    refine ⟨?_, ⟨?_, ?_⟩⟩
    · intro _ h; simp at h
    · intro _ h; simp at h
    · intro _ h; simp at h
  have hloop := fueledLoop_fixpoint step cands Inv hext hinv hbound
    (cands.length + 1) [] hI0 (Nat.lt_succ_of_le (missingCount_le cands []))
  have hclosed : ∀ u, u ∈ step (fueledLoop step (cands.length + 1) []) →
      u ∈ fueledLoop step (cands.length + 1) [] := hloop.2.2
  have haccL : ∀ u, u ∈ fueledLoop step (cands.length + 1) [] →
      u ∈ reachDen C rec self W.den V.toEnv ρ := hloop.1.1
  constructor
  · intro ht
    exact haccL t ht
  · intro ht
    have hpre : ∀ x, x ∈ T (fun u => u ∈ fueledLoop step (cands.length + 1) []) →
        x ∈ fueledLoop step (cands.length + 1) [] := by
      intro x hx
      exact hclosed x
        ((hop (acc := fueledLoop step (cands.length + 1) []) (t := x)).mpr hx)
    exact lfpS_le hpre t ht

theorem reach_den_finite (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (rec : LinearRec) (self : InteriorId) (V : InteriorTables)
    (hsafe : ∀ r, r ∈ rec.rules self → Safe r)
    (hwt : ∀ r, r ∈ rec.rules self → r.WellTyped) :
    (reachDen C rec self W.den V.toEnv ρ).Finite :=
  ⟨evalLinearReach C W ρ rec self V,
    fun t => (evalLinearReach_eq_lfp hsafe hwt t).symm⟩

/-! ## `evalQuery` — the denotation of a Query -/

/-- One function by constructor cases. The `reach` case is the one site
that computes the rec's id (`⟨interiors.length⟩`) and publishes the
finished table there. -/
def evalQuery (C : Classify) (q : Query) (I : Instance) (ρ : ParamEnv) :
    Set AnswerTuple :=
  match q with
  | .cq interiors rules =>
      rulesAnswers C rules (sourceDen I (evalInteriors C interiors I ρ)) ρ
  | .reach interiors rec rules =>
      let V := evalInteriors C interiors I ρ
      let self : InteriorId := ⟨interiors.length⟩
      rulesAnswers C rules
        (sourceDen I (V.update self (reachDen C rec self I V ρ))) ρ

def evalInteriorTables.go (C : Classify) (W : ListInstance) (ρ : ParamEnv) :
    Nat → List Interior → InteriorTables → InteriorTables
  | _, [], T => T
  | i, d :: ds, T =>
      evalInteriorTables.go C W ρ (i + 1) ds
        (T.update ⟨i⟩ (evalList C W T ρ d.rules))

def evalInteriorTables (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (defs : List Interior) : InteriorTables :=
  evalInteriorTables.go C W ρ 0 defs InteriorTables.empty

def evalQueryList (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (q : Query) : List AnswerTuple :=
  match q with
  | .cq interiors rules =>
      evalList C W (evalInteriorTables C W ρ interiors) ρ rules
  | .reach interiors rec rules =>
      let T₀ := evalInteriorTables C W ρ interiors
      let self : InteriorId := ⟨interiors.length⟩
      evalList C W (T₀.update self (evalLinearReach C W ρ rec self T₀)) ρ rules

theorem evalInteriorTables_go_sound {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} :
    ∀ (i : Nat) (suffix : List Interior) (T : InteriorTables) (E : InteriorEnv),
      (∀ c t, t ∈ T.toEnv c ↔ t ∈ E c) →
      (∀ d, d ∈ suffix → ∀ r, r ∈ d.rules → Safe r) →
      (∀ d, d ∈ suffix → ∀ r, r ∈ d.rules → r.WellTyped) →
      ∀ c t, t ∈ (evalInteriorTables.go C W ρ i suffix T).toEnv c ↔
        t ∈ evalInteriorsFold C W.den ρ i suffix E c
  | _, [], T, E, hTE, _, _ => by
    intro c t
    simpa [evalInteriorTables.go, evalInteriorsFold] using hTE c t
  | i, d :: ds, T, E, hTE, hsafe, hwt => by
    have hrows : ∀ t, t ∈ evalList C W T ρ d.rules ↔
        t ∈ rulesAnswers C d.rules (sourceDen W.den E) ρ := by
      intro t
      exact (eval_sound (C := C) (W := W) (ρ := ρ) (T := T)
        (fun r hr => hsafe d List.mem_cons_self r hr)
        (fun r hr => hwt d List.mem_cons_self r hr) t).trans
        (rulesAnswers_congr (sourceDen_congr hTE) t)
    have hTE' : ∀ c t,
        t ∈ (T.update ⟨i⟩ (evalList C W T ρ d.rules)).toEnv c ↔
        t ∈ E.update ⟨i⟩ (fun u =>
          u ∈ rulesAnswers C d.rules (sourceDen W.den E) ρ) c := by
      intro c t
      exact (InteriorTables.toEnv_update T ⟨i⟩
          (evalList C W T ρ d.rules) c t).trans
        (InteriorEnv.update_congr hTE hrows c t)
    intro c t
    rw [evalInteriorTables.go, evalInteriorsFold]
    exact evalInteriorTables_go_sound (i + 1) ds
      (T.update ⟨i⟩ (evalList C W T ρ d.rules))
      (E.update ⟨i⟩ (fun u =>
        u ∈ rulesAnswers C d.rules (sourceDen W.den E) ρ))
      hTE'
      (fun d' hd' => hsafe d' (List.mem_cons_of_mem _ hd'))
      (fun d' hd' => hwt d' (List.mem_cons_of_mem _ hd')) c t

/-- Interior DAG: `evalInteriorTables` lists the declaration-order fold.
Premises: `Safe` / `WellTyped` on every interior rule (`eval_sound` at
each write). -/
theorem evalInteriorTables_sound {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {defs : List Interior}
    (hsafe : ∀ d, d ∈ defs → ∀ r, r ∈ d.rules → Safe r)
    (hwt : ∀ d, d ∈ defs → ∀ r, r ∈ d.rules → r.WellTyped) :
    ∀ c t, t ∈ evalInteriorTables C W ρ defs c ↔
      t ∈ evalInteriors C defs W.den ρ c := by
  intro c t
  change t ∈ (evalInteriorTables.go C W ρ 0 defs InteriorTables.empty).toEnv c ↔
    t ∈ evalInteriorsFold C W.den ρ 0 defs InteriorEnv.empty c
  exact evalInteriorTables_go_sound (C := C) (W := W) (ρ := ρ)
    0 defs InteriorTables.empty InteriorEnv.empty
    (fun c t => by
      change (t ∈ ([] : List AnswerTuple)) ↔ False
      simp)
    hsafe hwt c t

/-- Interior DAG once, then either main `rulesAnswers` (`.cq`) or
`reachDen` plus main `rulesAnswers` (`.reach`) — listed by
`evalQueryList`. Premises: `Safe` / `WellTyped` per lane. -/
theorem evalQuery_sound {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {q : Query}
    (hInter : ∀ d, d ∈ q.interiors → ∀ r, r ∈ d.rules → Safe r ∧ r.WellTyped)
    (hMain : ∀ r, r ∈ q.rules → Safe r ∧ r.WellTyped)
    (hRec : ∀ interiors rec rules, q = .reach interiors rec rules →
      ∀ r, r ∈ rec.rules ⟨interiors.length⟩ → Safe r ∧ r.WellTyped) :
    ∀ t, t ∈ evalQueryList C W ρ q ↔ t ∈ evalQuery C q W.den ρ := by
  intro t
  cases q with
  | cq interiors rules =>
    have hinterS : ∀ d, d ∈ interiors → ∀ r, r ∈ d.rules → Safe r :=
      fun d hd r hr =>
        (hInter d (by simpa [Query.interiors] using hd) r hr).1
    have hinterW : ∀ d, d ∈ interiors → ∀ r, r ∈ d.rules → r.WellTyped :=
      fun d hd r hr =>
        (hInter d (by simpa [Query.interiors] using hd) r hr).2
    have hT0 :=
      evalInteriorTables_sound (C := C) (W := W) (ρ := ρ) hinterS hinterW
    have hmainS : ∀ r, r ∈ rules → Safe r :=
      fun r hr => (hMain r (by simpa [Query.rules] using hr)).1
    have hmainW : ∀ r, r ∈ rules → r.WellTyped :=
      fun r hr => (hMain r (by simpa [Query.rules] using hr)).2
    refine (eval_sound (C := C) (W := W) (ρ := ρ)
      (T := evalInteriorTables C W ρ interiors) hmainS hmainW t).trans
      (rulesAnswers_congr (sourceDen_congr hT0) t)
  | reach interiors rec rules =>
    let T₀ := evalInteriorTables C W ρ interiors
    have hinterS : ∀ d, d ∈ interiors → ∀ r, r ∈ d.rules → Safe r :=
      fun d hd r hr =>
        (hInter d (by simpa [Query.interiors] using hd) r hr).1
    have hinterW : ∀ d, d ∈ interiors → ∀ r, r ∈ d.rules → r.WellTyped :=
      fun d hd r hr =>
        (hInter d (by simpa [Query.interiors] using hd) r hr).2
    have hT0 :=
      evalInteriorTables_sound (C := C) (W := W) (ρ := ρ) hinterS hinterW
    have hmainS : ∀ r, r ∈ rules → Safe r :=
      fun r hr => (hMain r (by simpa [Query.rules] using hr)).1
    have hmainW : ∀ r, r ∈ rules → r.WellTyped :=
      fun r hr => (hMain r (by simpa [Query.rules] using hr)).2
    have hrecS : ∀ r, r ∈ rec.rules ⟨interiors.length⟩ → Safe r :=
      fun r hr => (hRec interiors rec rules rfl r hr).1
    have hrecW : ∀ r, r ∈ rec.rules ⟨interiors.length⟩ → r.WellTyped :=
      fun r hr => (hRec interiors rec rules rfl r hr).2
    have hreach := evalLinearReach_eq_lfp (C := C) (W := W) (ρ := ρ)
      (rec := rec) (self := ⟨interiors.length⟩) (V := T₀) hrecS hrecW
    refine (eval_sound (C := C) (W := W) (ρ := ρ)
      (T := T₀.update ⟨interiors.length⟩
        (evalLinearReach C W ρ rec ⟨interiors.length⟩ T₀))
      hmainS hmainW t).trans (rulesAnswers_congr ?_ t)
    intro s f
    apply sourceDen_congr
    intro c u
    unfold InteriorTables.toEnv InteriorTables.update InteriorEnv.update
    by_cases hc : c = ⟨interiors.length⟩
    · subst hc
      simp only [↓reduceIte]
      have henv : T₀.toEnv =
          evalInteriors C interiors W.den ρ := by
        funext d v
        exact propext (hT0 d v)
      rw [← henv]
      exact hreach u
    · simp only [hc, ↓reduceIte]
      exact hT0 c u

/-- Empty-prefix `.cq` denotes the union of its main rules over the instance. -/
theorem evalQuery_cq (C : Classify) (rules : List Rule)
    (I : Instance) (ρ : ParamEnv) :
    evalQuery C (.cq [] rules) I ρ =
      rulesAnswers C rules (edbEnv I) ρ := by
  simp [evalQuery, evalInteriors, evalInteriorsFold, edbEnv]

theorem evalQuery_empty_rules {C : Classify} {q : Query} {I : Instance}
    {ρ : ParamEnv} (hr : q.rules = []) :
    ∀ t, t ∉ evalQuery C q I ρ := by
  intro t ht
  cases q with
  | cq interiors rules =>
    simp [Query.rules] at hr
    simp [evalQuery, hr, mem_rulesAnswers] at ht
  | reach interiors rec rules =>
    simp [Query.rules] at hr
    simp [evalQuery, hr, mem_rulesAnswers] at ht

theorem evalInteriorsFold_instance {C : Classify} {I J : Instance}
    {ρ : ParamEnv} {ds : List Interior}
    (h : ∀ d, d ∈ ds → ∀ r, r ∈ d.rules → ∀ R, R ∈ r.relations → I R = J R) :
    ∀ i W W',
      (∀ c t, t ∈ W c ↔ t ∈ W' c) →
      ∀ c t,
        t ∈ evalInteriorsFold C I ρ i ds W c ↔
        t ∈ evalInteriorsFold C J ρ i ds W' c := by
  induction ds with
  | nil =>
    intro i W W' hWW c t
    simpa [evalInteriorsFold] using hWW c t
  | cons d ds ih =>
    intro i W W' hWW c t
    simp only [evalInteriorsFold]
    refine ih (fun d' hd' => h d' (List.mem_cons_of_mem _ hd'))
      (i + 1) _ _ ?_ c t
    exact InteriorEnv.update_congr hWW
      (rulesAnswers_instance_env
        (fun r hr R hR => h d List.mem_cons_self r hr R hR) hWW)

theorem evalInteriors_instance {C : Classify} {defs : List Interior}
    {I J : Instance} {ρ : ParamEnv}
    (h : ∀ d, d ∈ defs → ∀ r, r ∈ d.rules → ∀ R, R ∈ r.relations → I R = J R) :
    ∀ c t, t ∈ evalInteriors C defs I ρ c ↔ t ∈ evalInteriors C defs J ρ c :=
  evalInteriorsFold_instance h 0 InteriorEnv.empty InteriorEnv.empty
    (fun _ _ => Iff.rfl)

theorem lfpS_congr {α} {T U : Set α → Set α}
    (h : ∀ X a, a ∈ T X ↔ a ∈ U X) :
    ∀ a, a ∈ lfpS T ↔ a ∈ lfpS U := by
  intro a
  constructor
  · intro hT Y hpre
    exact hT Y fun b hb => hpre b ((h Y b).mp hb)
  · intro hU Y hpre
    exact hU Y fun b hb => hpre b ((h Y b).mpr hb)

theorem reachOp_instance {C : Classify} {rec : LinearRec} {self : InteriorId}
    {I J : Instance} {W W' : InteriorEnv} {ρ : ParamEnv}
    (hedb : ∀ r, r ∈ rec.rules self → ∀ R, R ∈ r.relations → I R = J R)
    (henv : ∀ c t, t ∈ W c ↔ t ∈ W' c) :
    ∀ X X', (∀ t, t ∈ X ↔ t ∈ X') → ∀ t,
      t ∈ reachOp C rec self I W ρ X ↔ t ∈ reachOp C rec self J W' ρ X' := by
  intro X X' hX t
  have hbase : ∀ r, r ∈ rec.baseRules → ∀ R, R ∈ r.relations → I R = J R :=
    fun r hr => hedb r (List.mem_append.mpr (Or.inl hr))
  have hstep : ∀ r, r ∈ rec.stepRules self → ∀ R, R ∈ r.relations → I R = J R :=
    fun r hr => hedb r (List.mem_append.mpr (Or.inr hr))
  unfold reachOp
  exact or_congr
    (rulesAnswers_instance_env hbase henv t)
    (rulesAnswers_instance_env hstep (InteriorEnv.update_congr henv hX) t)

theorem reachDen_instance {C : Classify} {rec : LinearRec} {self : InteriorId}
    {I J : Instance} {W W' : InteriorEnv} {ρ : ParamEnv}
    (hedb : ∀ r, r ∈ rec.rules self → ∀ R, R ∈ r.relations → I R = J R)
    (henv : ∀ c t, t ∈ W c ↔ t ∈ W' c) :
    ∀ t, t ∈ reachDen C rec self I W ρ ↔ t ∈ reachDen C rec self J W' ρ :=
  lfpS_congr fun X a =>
    reachOp_instance hedb henv X X (fun (_ : AnswerTuple) => Iff.rfl) a

/-- **Theorem 9.** The denotation is a function of ONE `Instance`:
two instances agreeing on every mentioned stored relation yield
identical answers. Interior and rec tables are determined by those
relations. Bridge: snapshot isolation — an execution runs against one
storage snapshot (`crate::Db::query` pins one read transaction); PRD 09
owns the transaction side. -/
theorem snapshot_single {q : Query} {I J : Instance} (C : Classify)
    (ρ : ParamEnv) (h : ∀ R, R ∈ q.relations → I R = J R) :
    ∀ t, t ∈ evalQuery C q I ρ ↔ t ∈ evalQuery C q J ρ := by
  intro t
  cases q with
  | cq interiors rules =>
    simp only [evalQuery]
    have hW := evalInteriors_instance (C := C) (ρ := ρ)
      (fun d hd r hr R hR => h R (by
        simp [Query.relations]
        exact Or.inl ⟨r, ⟨d, hd, hr⟩, hR⟩))
    exact rulesAnswers_instance_env
      (fun r hr R hR => h R (by
        simp [Query.relations]
        exact Or.inr ⟨r, hr, hR⟩))
      hW t
  | reach interiors rec rules =>
    simp only [evalQuery]
    have hW := evalInteriors_instance (C := C) (ρ := ρ)
      (fun d hd r hr R hR => h R (by
        simp [Query.relations]
        exact Or.inl ⟨r, ⟨d, hd, hr⟩, hR⟩))
    have hrec := reachDen_instance (C := C) (ρ := ρ)
      (fun r hr R hR => h R (by
        simp [Query.relations]
        exact Or.inr (Or.inl ⟨r, hr, hR⟩)))
      hW
    exact rulesAnswers_instance_env
      (fun r hr R hR => h R (by
        simp [Query.relations]
        exact Or.inr (Or.inr ⟨r, hr, hR⟩)))
      (InteriorEnv.update_congr hW hrec) t

end Bumbledb.Query

