import Bumbledb.Query.Denotation

/-!
# Exec/Reach — interior DAG, one linear reach, the query denotation

Level 0: `evalInteriors`, `reachOp`, `reachDen = lfpS`, `evalQuery`.
No fuel. No strata. The denotation is `evalQuery`.

Level 1: `evalLinearReach`, `evalQueryList`, proved equal to Level 0.
`fueledLoop` is a **private** termination metric (`missingCount_le` is
why `cands.length + 1` always suffices). It is not a parameter of any
public def and not a Bridge incompleteness caveat.

Engine `DerivedBudgetExceeded` is incompleteness vs `evalQuery` —
one derived-tuples ledger over interior tables and `reachDen` alike —
the same class as `ResultBytesOverflow` vs `rulesAnswers`.

## Narrowings recorded (law 5)

* **`evalLinearReach` candidates.** The spec names
  `allTuples recDom rec.arity`. The proved evaluator uses the finite
  union of `allTuples recDom r.finds.length` over `base ++ rec`, so
  agreement does not assume head-arity equals `Rec.arity`. When they
  agree the two lists cover the same tuples.
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

/-- Finished interior tables after the first `n` defs (declaration
order). `evalInteriorsAt … 0` is empty. `evalInteriorsAt … (k+1)`
writes `InteriorId ⟨k⟩` from `defs[k]?` against the prefix env.
Call `evalInteriorsAt … defs.length` to finish every interior. -/
def evalInteriorsAt (C : Classify) (defs : List Interior) (I : Instance)
    (ρ : ParamEnv) : Nat → InteriorEnv
  | 0 => InteriorEnv.empty
  | n + 1 =>
    let prev := evalInteriorsAt C defs I ρ n
    fun c t =>
      if h : c.id < n then
        prev c t
      else if c.id = n then
        match defs[n]? with
        | some d => t ∈ rulesAnswers C d.rules (sourceDen I prev) ρ
        | none => False
      else False

def evalInteriors (C : Classify) (q : Query) (I : Instance) (ρ : ParamEnv) :
    InteriorEnv :=
  evalInteriorsAt C q.interiors I ρ q.interiors.length

theorem evalInteriorsAt_zero (C : Classify) (defs : List Interior)
    (I : Instance) (ρ : ParamEnv) :
    evalInteriorsAt C defs I ρ 0 = InteriorEnv.empty := rfl

theorem evalInteriorsAt_stable {C : Classify} {defs : List Interior}
    {I : Instance} {ρ : ParamEnv} :
    ∀ {n : Nat}, n ≥ defs.length →
      ∀ c, c.id < defs.length →
        evalInteriorsAt C defs I ρ n c =
          evalInteriorsAt C defs I ρ defs.length c
  | 0, hge, c, hc => by
    have : defs.length = 0 := Nat.eq_zero_of_le_zero hge
    exact absurd (this ▸ hc) (Nat.not_lt_zero _)
  | n + 1, hge, c, hc => by
    by_cases heq : n + 1 = defs.length
    · rw [heq]
    · have hle : defs.length ≤ n :=
        Nat.le_of_lt_succ (Nat.lt_of_le_of_ne hge (Ne.symm heq))
      have hlt : c.id < n := Nat.lt_of_lt_of_le hc hle
      have hrec :=
        evalInteriorsAt_stable (C := C) (I := I) (ρ := ρ) (n := n) hle c hc
      funext t
      change (if h : c.id < n then evalInteriorsAt C defs I ρ n c t
        else if c.id = n then
          match defs[n]? with
          | some d => t ∈ rulesAnswers C d.rules
              (sourceDen I (evalInteriorsAt C defs I ρ n)) ρ
          | none => False
        else False) = evalInteriorsAt C defs I ρ defs.length c t
      rw [dif_pos hlt]
      exact congrFun hrec t

/-- Every interior source an accepted query reads names a real
interior or the rec. Replaces `wellFormed_reads_real`. Bridge:
`ValidationError::UnknownInterior`. -/
theorem wellFormed_interior_reads_real {q : Query} (hwf : q.WellFormed)
    {r : Rule} (hr : r ∈ q.allRules) {a : Atom}
    (ha : a ∈ r.atoms ∨ a ∈ r.negated) {C : InteriorId}
    (hsrc : a.source = .interior C) :
    C.id < q.derivedCount :=
  hwf.1 r hr a ha C hsrc

/-! ## Reach — `reachOp`, `reachDen` -/

/-- The reach operator. Base does not see `X`. Rec arms see `X` at
`self` and finished interiors in `W`. -/
def reachOp (C : Classify) (rec : Rec) (self : InteriorId)
    (I : Instance) (W : InteriorEnv) (ρ : ParamEnv)
    (X : Set AnswerTuple) : Set AnswerTuple :=
  fun t =>
    t ∈ rulesAnswers C rec.base (sourceDen I W) ρ ∨
    t ∈ rulesAnswers C rec.rec (sourceDen I (W.update self X)) ρ

def reachDen (C : Classify) (rec : Rec) (self : InteriorId)
    (I : Instance) (W : InteriorEnv) (ρ : ParamEnv) : Set AnswerTuple :=
  lfpS (reachOp C rec self I W ρ)

theorem selfCount_eq_one_mem {r : Rule} {self : InteriorId}
    (h : r.selfCount self = 1) :
    ∃ a, a ∈ r.atoms ∧ a.source = .interior self := by
  have hf : (r.atoms.filter fun a =>
      decide (a.source = .interior self)).length = 1 := h
  cases hfil : r.atoms.filter fun a =>
      decide (a.source = .interior self) with
  | nil =>
    simp [hfil] at hf
  | cons a rest =>
    have ha : a ∈ r.atoms.filter fun a =>
        decide (a.source = .interior self) := by
      rw [hfil]
      exact List.mem_cons_self
    have ⟨hmem, hdec⟩ := List.mem_filter.mp ha
    exact ⟨a, hmem, of_decide_eq_true hdec⟩

/-- Linearity and the roster's no-negation-in-rec premise make the
reach operator monotone. The wall is the self case
(`Countermodels.odd_not_monotone`); do not cite this theorem as
evidence that finished-table negation in a rec arm is non-monotone. -/
theorem reachOp_mono {C : Classify} {rec : Rec} {self : InteriorId}
    {I : Instance} {W : InteriorEnv} {ρ : ParamEnv}
    (hlin : ∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ r.negated = []) :
    MonoS (reachOp C rec self I W ρ) := by
  intro X Y hXY t ht
  rcases ht with hbase | hrec
  · exact Or.inl hbase
  · refine Or.inr (rulesAnswers_mono_pos
      (sourceDen_mono (InteriorEnv.update_le hXY))
      (fun r hr => (hlin r hr).2) t hrec)

/-- Round 0 is base: a positive self-atom against an empty table
derives nothing, so `rec(∅) = ∅` and `T(∅) = base`. -/
theorem reachOp_empty {C : Classify} {rec : Rec} {self : InteriorId}
    {I : Instance} {W : InteriorEnv} {ρ : ParamEnv}
    (hpos : ∀ r, r ∈ rec.rec → r.selfCount self = 1) :
    ∀ t, t ∈ reachOp C rec self I W ρ (fun _ => False) ↔
         t ∈ rulesAnswers C rec.base (sourceDen I W) ρ := by
  intro t
  constructor
  · rintro (hbase | hrec)
    · exact hbase
    · obtain ⟨r, hr, ⟨σ, ⟨hpos', -, -⟩, -⟩⟩ := mem_rulesAnswers.mp hrec
      obtain ⟨a, ha, hsrc⟩ := selfCount_eq_one_mem (hpos r hr)
      obtain ⟨f, hf, -⟩ := hpos' a ha
      rw [hsrc] at hf
      obtain ⟨u, hu, -⟩ := hf
      unfold InteriorEnv.update at hu
      rw [if_pos rfl] at hu
      exact False.elim hu
  · exact Or.inl

/-! ## Semi-naive, at the operator level -/

theorem setExt {α : Type u} {s t : Set α} (h : ∀ a, s a ↔ t a) :
    s = t :=
  funext fun a => propext (h a)

/-- The naive chain: start empty, keep everything, add everything
the operator derives. -/
def naiveIter {α : Type u} (T : Set α → Set α) : Nat → Set α
  | 0 => fun _ => False
  | k + 1 => fun a => naiveIter T k a ∨ T (naiveIter T k) a

/-- The semi-naive chain: an accumulator and the frontier
`new = T(acc) \ acc`. -/
def semiNaiveIter {α : Type u} (T : Set α → Set α) :
    Nat → Set α × Set α
  | 0 => (fun _ => False, fun a => T (fun _ => False) a ∧ ¬ False)
  | k + 1 =>
    (fun a => (semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a,
     fun a => T (fun b => (semiNaiveIter T k).1 b ∨
         (semiNaiveIter T k).2 b) a ∧
       ¬ ((semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a))

theorem semiNaive_delta {α : Type u} (T : Set α → Set α) :
    ∀ k, (semiNaiveIter T k).2 =
      fun a => T (semiNaiveIter T k).1 a ∧ ¬ (semiNaiveIter T k).1 a
  | 0 => rfl
  | _ + 1 => rfl

/-- **Semi-naive agrees with naive**: iterating on
`new = T(acc) \ acc` walks exactly the naive chain. Instantiates at
`T := reachOp C rec self I W ρ`. -/
theorem semi_naive_agrees {α : Type u} (T : Set α → Set α) :
    ∀ k, (semiNaiveIter T k).1 = naiveIter T k
  | 0 => rfl
  | k + 1 => by
    have ih := semi_naive_agrees T k
    show (fun a => (semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a)
      = naiveIter T (k + 1)
    rw [semiNaive_delta T k, ih]
    refine setExt fun a => ?_
    show naiveIter T k a ∨ (T (naiveIter T k) a ∧ ¬ naiveIter T k a) ↔
      naiveIter T k a ∨ T (naiveIter T k) a
    constructor
    · rintro (h | ⟨h, -⟩)
      · exact Or.inl h
      · exact Or.inr h
    · intro h
      by_cases hk : naiveIter T k a
      · exact Or.inl hk
      · rcases h with h | h
        · exact absurd h hk
        · exact Or.inr ⟨h, hk⟩

theorem semi_naive_same_fixpoint {α : Type u} (T : Set α → Set α)
    (k : Nat) :
    (fun a => (semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a) =
      naiveIter T (k + 1) :=
  semi_naive_agrees T (k + 1)

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
interior columns. Ignores the accumulating self (same as ignoring
`idb` on the old program domain). -/
def recDom (rec : Rec) (W : ListInstance) (V : InteriorTables) : List Value :=
  fillerValue ::
    (rec.base ++ rec.rec).flatMap fun r =>
      r.atoms.flatMap fun a =>
        match a.source with
        | .edb R => a.bindings.flatMap fun b => (W.facts R).map (· b.1)
        | .interior C => a.bindings.flatMap fun b =>
            (V C).flatMap fun t => (t[b.1.id]?).toList

/-- Candidate tuples: the finite product at each rule's head length. -/
def recCands (rec : Rec) (W : ListInstance) (V : InteriorTables) :
    List AnswerTuple :=
  (rec.base ++ rec.rec).flatMap fun r =>
    allTuples (recDom rec W V) r.finds.length

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
    (rec : Rec) (self : InteriorId) (V : InteriorTables)
    (base : List AnswerTuple) (acc : List AnswerTuple) :
    List AnswerTuple :=
  let T := V.update self acc
  base ++ evalList C W T ρ rec.rec

def evalLinearReach (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (rec : Rec) (self : InteriorId) (V : InteriorTables) : List AnswerTuple :=
  let base := evalList C W V ρ rec.base
  let cands := recCands rec W V
  fueledLoop (reachStep C W ρ rec self V base) (cands.length + 1) []

theorem mem_reachStep {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {rec : Rec} {self : InteriorId} {V : InteriorTables}
    {base acc : List AnswerTuple} {t : AnswerTuple} :
    t ∈ reachStep C W ρ rec self V base acc ↔
      t ∈ base ∨ t ∈ evalList C W (V.update self acc) ρ rec.rec := by
  simp [reachStep, List.mem_append]

theorem evalRule_length {C : Classify} {W : ListInstance}
    {T : InteriorTables} {ρ : ParamEnv} {r : Rule} {t : AnswerTuple}
    (ht : t ∈ evalRule C W T ρ r) : t.length = r.finds.length := by
  obtain ⟨σp, -, rfl⟩ := List.mem_map.mp ht
  simp

theorem recDom_edb {rec : Rec} {W : ListInstance} {V : InteriorTables}
    {r : Rule} (hr : r ∈ rec.base ++ rec.rec) {a : Atom}
    (ha : a ∈ r.atoms) {R : RelId} (hsrc : a.source = .edb R)
    {b : FieldId × Term} (hb : b ∈ a.bindings) {f : Fact}
    (hf : f ∈ W.facts R) : f b.1 ∈ recDom rec W V := by
  refine List.mem_cons_of_mem _ (List.mem_flatMap.mpr ⟨r, hr,
    List.mem_flatMap.mpr ⟨a, ha, ?_⟩⟩)
  rw [hsrc]
  exact List.mem_flatMap.mpr ⟨b, hb, List.mem_map.mpr ⟨f, hf, rfl⟩⟩

theorem recDom_interior {rec : Rec} {W : ListInstance} {V : InteriorTables}
    {r : Rule} (hr : r ∈ rec.base ++ rec.rec) {a : Atom}
    (ha : a ∈ r.atoms) {C : InteriorId} (hsrc : a.source = .interior C)
    {b : FieldId × Term} (hb : b ∈ a.bindings) {row : AnswerTuple}
    (hrow : row ∈ V C) {v : Value} (hv : row[b.1.id]? = some v) :
    v ∈ recDom rec W V := by
  refine List.mem_cons_of_mem _ (List.mem_flatMap.mpr ⟨r, hr,
    List.mem_flatMap.mpr ⟨a, ha, ?_⟩⟩)
  rw [hsrc]
  exact List.mem_flatMap.mpr ⟨b, hb, List.mem_flatMap.mpr
    ⟨row, hrow, by simp [hv]⟩⟩

theorem recDom_filler (rec : Rec) (W : ListInstance) (V : InteriorTables) :
    fillerValue ∈ recDom rec W V :=
  List.mem_cons_self ..

theorem evalRule_in_cands {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {rec : Rec} {self : InteriorId} {V : InteriorTables}
    {acc : List AnswerTuple} {r : Rule}
    (hr : r ∈ rec.base ++ rec.rec)
    (hsafe : Safe r)
    (hacc : ∀ u, u ∈ acc → u ∈ recCands rec W V) {t : AnswerTuple}
    (ht : t ∈ evalRule C W (V.update self acc) ρ r) :
    t ∈ recCands rec W V := by
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
    exact recDom_edb hr ha hsrc hb hf
  | interior Q =>
    rw [hsrc] at hf
    obtain ⟨row, hrow, rfl⟩ := hf
    rw [← hfb]
    unfold InteriorTables.toEnv InteriorTables.update at hrow
    by_cases hQ : Q = self
    · subst hQ
      rw [if_pos rfl] at hrow
      rcases tupleFact_mem_or_filler row b.1 with hmem | hfill
      · have hrowc : row ∈ recCands rec W V := hacc row hrow
        obtain ⟨_, _, htup⟩ := List.mem_flatMap.mp hrowc
        exact (mem_allTuples.mp htup).2 _ hmem
      · rw [hfill]
        exact recDom_filler rec W V
    · rw [if_neg hQ] at hrow
      cases hidx : row[b.1.id]? with
      | none =>
        have : tupleFact row b.1 = fillerValue := by
          unfold tupleFact; simp [hidx]
        rw [this]
        exact recDom_filler rec W V
      | some w =>
        have : tupleFact row b.1 = w := by
          unfold tupleFact; simp [hidx]
        rw [this]
        exact recDom_interior hr ha hsrc hb hrow hidx

theorem evalList_in_cands {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {rec : Rec} {self : InteriorId} {V : InteriorTables}
    {acc : List AnswerTuple} {rules : List Rule}
    (hrules : ∀ r, r ∈ rules → r ∈ rec.base ++ rec.rec)
    (hsafe : ∀ r, r ∈ rules → Safe r)
    (hacc : ∀ u, u ∈ acc → u ∈ recCands rec W V) {t : AnswerTuple}
    (ht : t ∈ evalList C W (V.update self acc) ρ rules) :
    t ∈ recCands rec W V := by
  obtain ⟨r, hr, ht'⟩ := List.mem_flatMap.mp ht
  exact evalRule_in_cands (hrules r hr) (hsafe r hr) hacc ht'

theorem evalRule_base_in_cands {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {rec : Rec} {V : InteriorTables} {r : Rule}
    (hr : r ∈ rec.base ++ rec.rec) (hsafe : Safe r) {t : AnswerTuple}
    (ht : t ∈ evalRule C W V ρ r) :
    t ∈ recCands rec W V := by
  have hans : t ∈ ruleAnswers C r (sourceDen W.den V.toEnv) ρ :=
    evalRule_sound ht
  have hlen : t.length = r.finds.length := evalRule_length ht
  refine List.mem_flatMap.mpr ⟨r, hr, mem_allTuples.mpr ⟨hlen, fun v hv => ?_⟩⟩
  have hdom := antijoin_over_active_domain hsafe t hans v hv
  obtain ⟨a, ha, f, hf0, b, hb, hfb⟩ := hdom
  have hf : f ∈ sourceDen W.den V.toEnv a.source := hf0
  cases hsrc : a.source with
  | edb R =>
    rw [hsrc] at hf
    rw [← hfb]
    exact recDom_edb hr ha hsrc hb hf
  | interior Q =>
    rw [hsrc] at hf
    obtain ⟨row, hrow, rfl⟩ := hf
    rw [← hfb]
    cases hidx : row[b.1.id]? with
    | none =>
      have : tupleFact row b.1 = fillerValue := by
        unfold tupleFact; simp [hidx]
      rw [this]
      exact recDom_filler rec W V
    | some w =>
      have : tupleFact row b.1 = w := by
        unfold tupleFact; simp [hidx]
      rw [this]
      exact recDom_interior hr ha hsrc hb hrow hidx

theorem evalList_base_in_cands {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {rec : Rec} {V : InteriorTables} {t : AnswerTuple}
    (hsafe : ∀ r, r ∈ rec.base → Safe r)
    (ht : t ∈ evalList C W V ρ rec.base) :
    t ∈ recCands rec W V := by
  obtain ⟨r, hr, ht'⟩ := List.mem_flatMap.mp ht
  exact evalRule_base_in_cands (List.mem_append.mpr (Or.inl hr))
    (hsafe r hr) ht'

theorem mem_reachStep_op {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {rec : Rec} {self : InteriorId} {V : InteriorTables}
    (hsafeB : ∀ r, r ∈ rec.base → Safe r)
    (hwtB : ∀ r, r ∈ rec.base → r.WellTyped)
    (hsafeR : ∀ r, r ∈ rec.rec → Safe r)
    (hwtR : ∀ r, r ∈ rec.rec → r.WellTyped)
    {acc : List AnswerTuple} {t : AnswerTuple} :
    t ∈ reachStep C W ρ rec self V (evalList C W V ρ rec.base) acc ↔
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
    {ρ : ParamEnv} {rec : Rec} {self : InteriorId} {V : InteriorTables}
    (hsafe : ∀ r, r ∈ rec.base ++ rec.rec → Safe r)
    (hwt : ∀ r, r ∈ rec.base ++ rec.rec → r.WellTyped)
    (hlin : ∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ r.negated = []) :
    ∀ t, t ∈ evalLinearReach C W ρ rec self V ↔
         t ∈ reachDen C rec self W.den V.toEnv ρ := by
  intro t
  let base := evalList C W V ρ rec.base
  let step := reachStep C W ρ rec self V base
  let cands := recCands rec W V
  let T := reachOp C rec self W.den V.toEnv ρ
  have hm : MonoS T := reachOp_mono hlin
  have hsafeB : ∀ r, r ∈ rec.base → Safe r :=
    fun r hr => hsafe r (List.mem_append.mpr (Or.inl hr))
  have hwtB : ∀ r, r ∈ rec.base → r.WellTyped :=
    fun r hr => hwt r (List.mem_append.mpr (Or.inl hr))
  have hsafeR : ∀ r, r ∈ rec.rec → Safe r :=
    fun r hr => hsafe r (List.mem_append.mpr (Or.inr hr))
  have hwtR : ∀ r, r ∈ rec.rec → r.WellTyped :=
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
    (rec : Rec) (self : InteriorId) (V : InteriorTables)
    (hsafe : ∀ r, r ∈ rec.base ++ rec.rec → Safe r)
    (hwt : ∀ r, r ∈ rec.base ++ rec.rec → r.WellTyped)
    (hlin : ∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ r.negated = []) :
    (reachDen C rec self W.den V.toEnv ρ).Finite :=
  ⟨evalLinearReach C W ρ rec self V,
    fun t => (evalLinearReach_eq_lfp hsafe hwt hlin t).symm⟩

/-! ## `evalQuery` — the denotation of a Query -/

/-- Match `q.rec` only. Do not match `recId` beside it. -/
def evalQuery (C : Classify) (q : Query) (I : Instance) (ρ : ParamEnv) :
    Set AnswerTuple :=
  let V := evalInteriors C q I ρ
  let V' :=
    match q.rec with
    | none => V
    | some rec =>
        let self : InteriorId := ⟨q.interiors.length⟩
        V.update self (reachDen C rec self I V ρ)
  rulesAnswers C q.rules (sourceDen I V') ρ

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
  let T₀ := evalInteriorTables C W ρ q.interiors
  let T :=
    match q.rec with
    | none => T₀
    | some rec =>
        let self : InteriorId := ⟨q.interiors.length⟩
        T₀.update self (evalLinearReach C W ρ rec self T₀)
  evalList C W T ρ q.rules

theorem getElem?_of_drop_cons {α} {l : List α} {i : Nat} {a : α}
    {as : List α} (h : l.drop i = a :: as) : l[i]? = some a := by
  induction i generalizing l with
  | zero =>
    cases l with
    | nil => cases h
    | cons b bs =>
      have h' : b :: bs = a :: as := by simpa using h
      cases h'
      rfl
  | succ i ih =>
    cases l with
    | nil => cases h
    | cons _ _ =>
      simp at h
      exact ih h

theorem evalInteriorsAt_agree_prefix {C : Classify} {defs : List Interior}
    {I : Instance} {ρ : ParamEnv} {n : Nat} {c : InteriorId}
    (hlt : c.id < n) :
    evalInteriorsAt C defs I ρ (n + 1) c =
      evalInteriorsAt C defs I ρ n c := by
  funext t
  change (if h : c.id < n then evalInteriorsAt C defs I ρ n c t
    else _) = _
  rw [dif_pos hlt]

theorem InteriorId.eq_mk (c : InteriorId) (n : Nat) : c = ⟨n⟩ ↔ c.id = n := by
  cases c; simp

/-- Stage `n` has written only `InteriorId ⟨k⟩` for `k < n`. -/
theorem evalInteriorsAt_out {C : Classify} {defs : List Interior}
    {I : Instance} {ρ : ParamEnv} {n : Nat} {c : InteriorId}
    (hge : n ≤ c.id) (t : AnswerTuple) :
    ¬ evalInteriorsAt C defs I ρ n c t := by
  induction n with
  | zero =>
    simp [evalInteriorsAt, InteriorEnv.empty]
  | succ n _ =>
    have hnlt : ¬ c.id < n :=
      Nat.not_lt_of_ge (Nat.le_trans (Nat.le_succ n) hge)
    have hne : c.id ≠ n := Nat.ne_of_gt (Nat.lt_of_succ_le hge)
    change ¬ (if h : c.id < n then evalInteriorsAt C defs I ρ n c t
      else if c.id = n then
        match defs[n]? with
        | some d => t ∈ rulesAnswers C d.rules
            (sourceDen I (evalInteriorsAt C defs I ρ n)) ρ
        | none => False
      else False)
    simp [hnlt, hne]

/-- One `go` step: write `defs[i]` into `T` and agree with stage `i+1`. -/
theorem evalInteriorTables_step {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {defs : List Interior} {i : Nat} {d : Interior}
    {T : InteriorTables}
    (hget : defs[i]? = some d)
    (hT : ∀ c t, t ∈ T c ↔ evalInteriorsAt C defs W.den ρ i c t)
    (hsafe : ∀ r, r ∈ d.rules → Safe r)
    (hwt : ∀ r, r ∈ d.rules → r.WellTyped) :
    ∀ c t, t ∈ (T.update ⟨i⟩ (evalList C W T ρ d.rules)) c ↔
      evalInteriorsAt C defs W.den ρ (i + 1) c t := by
  intro c t
  rcases Nat.lt_trichotomy c.id i with hlt | heq | hgt
  · have hne : c ≠ ⟨i⟩ := mt (InteriorId.eq_mk c i).mp (Nat.ne_of_lt hlt)
    have hleft : (T.update ⟨i⟩ (evalList C W T ρ d.rules)) c = T c := by
      simp [InteriorTables.update, hne]
    have hright :=
      congrFun (evalInteriorsAt_agree_prefix (C := C) (defs := defs)
        (I := W.den) (ρ := ρ) (n := i) (c := c) hlt) t
    rw [hleft, hright]
    exact hT c t
  · have hc : c = ⟨i⟩ := (InteriorId.eq_mk c i).mpr heq
    have hnlt : ¬ c.id < i := by
      rw [heq]
      exact Nat.lt_irrefl _
    have hleft : (T.update ⟨i⟩ (evalList C W T ρ d.rules)) c =
        evalList C W T ρ d.rules := by
      simp [InteriorTables.update, hc]
    change t ∈ (T.update ⟨i⟩ (evalList C W T ρ d.rules)) c ↔
      (if h : c.id < i then evalInteriorsAt C defs W.den ρ i c t
        else if c.id = i then
          match defs[i]? with
          | some d' => t ∈ rulesAnswers C d'.rules
              (sourceDen W.den (evalInteriorsAt C defs W.den ρ i)) ρ
          | none => False
        else False)
    rw [hleft, dif_neg hnlt, if_pos heq, hget]
    exact (eval_sound (C := C) (W := W) (ρ := ρ) (T := T) hsafe hwt t).trans
      (rulesAnswers_congr (sourceDen_congr hT) t)
  · have hne : c ≠ ⟨i⟩ := mt (InteriorId.eq_mk c i).mp (Nat.ne_of_gt hgt)
    have hnlt : ¬ c.id < i := Nat.not_lt_of_gt hgt
    have hnei : c.id ≠ i := Nat.ne_of_gt hgt
    have hleft : (T.update ⟨i⟩ (evalList C W T ρ d.rules)) c = T c := by
      simp [InteriorTables.update, hne]
    change t ∈ (T.update ⟨i⟩ (evalList C W T ρ d.rules)) c ↔
      (if h : c.id < i then evalInteriorsAt C defs W.den ρ i c t
        else if c.id = i then
          match defs[i]? with
          | some d' => t ∈ rulesAnswers C d'.rules
              (sourceDen W.den (evalInteriorsAt C defs W.den ρ i)) ρ
          | none => False
        else False)
    rw [hleft, dif_neg hnlt, if_neg hnei]
    constructor
    · intro ht
      exact (evalInteriorsAt_out (Nat.le_of_lt hgt) t) ((hT c t).mp ht)
    · intro hf
      exact False.elim hf

/-- `go` from index `i` over `defs.drop i` agrees with stage
`i + suffix.length`. -/
theorem evalInteriorTables_go_sound {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {defs : List Interior} :
    ∀ (i : Nat) (suffix : List Interior) (T : InteriorTables),
      defs.drop i = suffix →
      (∀ c t, t ∈ T c ↔ evalInteriorsAt C defs W.den ρ i c t) →
      (∀ d, d ∈ suffix → ∀ r, r ∈ d.rules → Safe r) →
      (∀ d, d ∈ suffix → ∀ r, r ∈ d.rules → r.WellTyped) →
      ∀ c t, t ∈ evalInteriorTables.go C W ρ i suffix T c ↔
        evalInteriorsAt C defs W.den ρ (i + suffix.length) c t
  | i, [], T, _, hT, _, _ => by
    intro c t
    simpa [evalInteriorTables.go] using hT c t
  | i, d :: ds, T, hdrop, hT, hsafe, hwt => by
    have hget : defs[i]? = some d := getElem?_of_drop_cons hdrop
    have hT' :=
      evalInteriorTables_step (C := C) (W := W) (ρ := ρ) (defs := defs)
        (i := i) (d := d) (T := T) hget hT
        (fun r hr => hsafe d List.mem_cons_self r hr)
        (fun r hr => hwt d List.mem_cons_self r hr)
    have hdrop' : defs.drop (i + 1) = ds := by
      have h1 : (defs.drop i).drop 1 = ds := by
        rw [hdrop]; rfl
      have h2 : (defs.drop i).drop 1 = defs.drop (i + 1) :=
        List.drop_drop (i := 1) (j := i) (l := defs)
      exact h2.symm.trans h1
    have ih :=
      evalInteriorTables_go_sound (C := C) (W := W) (ρ := ρ) (defs := defs)
        (i + 1) ds (T.update ⟨i⟩ (evalList C W T ρ d.rules)) hdrop' hT'
        (fun d' hd' => hsafe d' (List.mem_cons_of_mem _ hd'))
        (fun d' hd' => hwt d' (List.mem_cons_of_mem _ hd'))
    intro c t
    have hlen : i + (d :: ds).length = i + 1 + ds.length := by
      simp [Nat.add_comm, Nat.add_left_comm]
    rw [evalInteriorTables.go, hlen]
    exact ih c t

/-- Interior DAG: `evalInteriorTables` lists `evalInteriorsAt` at
`defs.length`. Premises: `Safe` / `WellTyped` on every interior rule
(`eval_sound` at each declaration-order write). -/
theorem evalInteriorTables_sound {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {defs : List Interior}
    (hsafe : ∀ d, d ∈ defs → ∀ r, r ∈ d.rules → Safe r)
    (hwt : ∀ d, d ∈ defs → ∀ r, r ∈ d.rules → r.WellTyped) :
    ∀ c t, t ∈ evalInteriorTables C W ρ defs c ↔
      evalInteriorsAt C defs W.den ρ defs.length c t := by
  intro c t
  simpa [evalInteriorTables] using
    evalInteriorTables_go_sound (C := C) (W := W) (ρ := ρ) (defs := defs)
      0 defs InteriorTables.empty (by simp)
      (by
        intro c t
        simp [InteriorTables.empty, evalInteriorsAt, InteriorEnv.empty])
      hsafe hwt c t

theorem mem_allRules_interior {q : Query} {d : Interior}
    (hd : d ∈ q.interiors) {r : Rule} (hr : r ∈ d.rules) :
    r ∈ q.allRules := by
  unfold Query.allRules
  exact List.mem_append.mpr <| Or.inl <| List.mem_append.mpr <| Or.inl <|
    List.mem_flatMap.mpr ⟨d, hd, hr⟩

theorem mem_allRules_rec {q : Query} {rec : Rec} (hrec : q.rec = some rec)
    {r : Rule} (hr : r ∈ rec.base ++ rec.rec) :
    r ∈ q.allRules := by
  unfold Query.allRules
  rw [hrec]
  exact List.mem_append.mpr <| Or.inl <| List.mem_append.mpr <| Or.inr hr

theorem mem_allRules_main {q : Query} {r : Rule} (hr : r ∈ q.rules) :
    r ∈ q.allRules := by
  unfold Query.allRules
  exact List.mem_append.mpr (Or.inr hr)

theorem recLinear_arms {q : Query} {rec : Rec}
    (hlin : q.recLinear) (hrec : q.rec = some rec) :
    ∀ r, r ∈ rec.rec → r.selfCount ⟨q.interiors.length⟩ = 1 ∧ r.negated = [] := by
  unfold Query.recLinear at hlin
  rw [hrec] at hlin
  intro r hr
  exact ⟨(hlin.2.2.2.1 r hr).1,
    hlin.2.2.2.2 r (List.mem_append.mpr (Or.inr hr))⟩

/-- Interior DAG once, optional `reachDen`, then main `rulesAnswers` —
listed by `evalQueryList`. Premises: `Safe` / `WellTyped` / `recLinear`,
not full `WellFormed`. -/
theorem evalQuery_sound {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {q : Query}
    (hsafe : ∀ r, r ∈ q.allRules → Safe r)
    (hwt : ∀ r, r ∈ q.allRules → r.WellTyped)
    (hlin : q.recLinear) :
    ∀ t, t ∈ evalQueryList C W ρ q ↔ t ∈ evalQuery C q W.den ρ := by
  intro t
  have hinterS : ∀ d, d ∈ q.interiors → ∀ r, r ∈ d.rules → Safe r :=
    fun d hd r hr => hsafe r (mem_allRules_interior hd hr)
  have hinterW : ∀ d, d ∈ q.interiors → ∀ r, r ∈ d.rules → r.WellTyped :=
    fun d hd r hr => hwt r (mem_allRules_interior hd hr)
  have hT0 :=
    evalInteriorTables_sound (C := C) (W := W) (ρ := ρ) hinterS hinterW
  have hmainS : ∀ r, r ∈ q.rules → Safe r :=
    fun r hr => hsafe r (mem_allRules_main hr)
  have hmainW : ∀ r, r ∈ q.rules → r.WellTyped :=
    fun r hr => hwt r (mem_allRules_main hr)
  cases hrec : q.rec with
  | none =>
    have hlist : evalQueryList C W ρ q =
        evalList C W (evalInteriorTables C W ρ q.interiors) ρ q.rules := by
      unfold evalQueryList
      rw [hrec]
    have hden : evalQuery C q W.den ρ =
        rulesAnswers C q.rules
          (sourceDen W.den (evalInteriors C q W.den ρ)) ρ := by
      unfold evalQuery
      rw [hrec]
    rw [hlist, hden]
    refine (eval_sound (C := C) (W := W) (ρ := ρ)
      (T := evalInteriorTables C W ρ q.interiors) hmainS hmainW t).trans
      (rulesAnswers_congr (sourceDen_congr hT0) t)
  | some rec =>
    let self : InteriorId := ⟨q.interiors.length⟩
    let T₀ := evalInteriorTables C W ρ q.interiors
    have harmsr := recLinear_arms hlin hrec
    have hrecS : ∀ r, r ∈ rec.base ++ rec.rec → Safe r :=
      fun r hr => hsafe r (mem_allRules_rec hrec hr)
    have hrecW : ∀ r, r ∈ rec.base ++ rec.rec → r.WellTyped :=
      fun r hr => hwt r (mem_allRules_rec hrec hr)
    have hreach := evalLinearReach_eq_lfp (C := C) (W := W) (ρ := ρ)
      (V := T₀) hrecS hrecW harmsr
    have hlist : evalQueryList C W ρ q =
        evalList C W (T₀.update self (evalLinearReach C W ρ rec self T₀))
          ρ q.rules := by
      unfold evalQueryList
      rw [hrec]
    have hden : evalQuery C q W.den ρ =
        rulesAnswers C q.rules
          (sourceDen W.den
            ((evalInteriors C q W.den ρ).update self
              (reachDen C rec self W.den (evalInteriors C q W.den ρ) ρ))) ρ := by
      unfold evalQuery
      rw [hrec]
    rw [hlist, hden]
    refine (eval_sound (C := C) (W := W) (ρ := ρ)
      (T := T₀.update self (evalLinearReach C W ρ rec self T₀))
      hmainS hmainW t).trans (rulesAnswers_congr ?_ t)
    intro s f
    apply sourceDen_congr
    intro c u
    unfold InteriorTables.toEnv InteriorTables.update InteriorEnv.update
    by_cases hc : c = self
    · subst hc
      simp only [↓reduceIte]
      have henv : T₀.toEnv = evalInteriors C q W.den ρ := by
        funext d v
        exact propext (hT0 d v)
      rw [← henv]
      exact hreach u
    · simp only [hc, ↓reduceIte]
      exact hT0 c u

theorem evalQuery_plain (C : Classify) (q : Query) (I : Instance)
    (ρ : ParamEnv) (hp : q.Plain) :
    ∀ t, t ∈ evalQuery C q I ρ ↔
         t ∈ rulesAnswers C q.rules (edbEnv I) ρ := by
  intro t
  obtain ⟨hinter, hrec⟩ := hp
  simp [evalQuery, evalInteriors, hinter, hrec, evalInteriorsAt, edbEnv]

theorem evalQueryList_plain {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {q : Query} (hp : q.Plain) :
    evalQueryList C W ρ q = evalList C W InteriorTables.empty ρ q.rules := by
  obtain ⟨hinter, hrec⟩ := hp
  simp [evalQueryList, evalInteriorTables, hinter, hrec,
    evalInteriorTables.go]

theorem evalQuery_empty_rules {C : Classify} {q : Query} {I : Instance}
    {ρ : ParamEnv} (hr : q.rules = []) :
    ∀ t, t ∉ evalQuery C q I ρ := by
  intro t ht
  simp [evalQuery, hr, mem_rulesAnswers] at ht

end Bumbledb.Query

