import Std

/-!
Reference for heterogeneous Event query scopes. Each World type is already its
admitted legal support, and every readout is total on that support. The indexed
expression is the result required of query shape/context checking; the Rust
checker is not extracted or verified here. Demands retain syntactic occurrences
even for constants and ignored branches. Native BEDC/BEVT admission must connect
captured context markers and readouts to these indices and functions.
-/
namespace QueryMaps

inductive Expr (Scope : Type) (World : Scope → Type) (Slot : Type) : Scope → Type where
  | leaf (scope : Scope) (slot : Slot) : Expr Scope World Slot scope
  | empty {s} : Expr Scope World Slot s → Expr Scope World Slot s
  | full {s} : Expr Scope World Slot s → Expr Scope World Slot s
  | neg {s} : Expr Scope World Slot s → Expr Scope World Slot s
  | both {s} : Expr Scope World Slot s → Expr Scope World Slot s → Expr Scope World Slot s
  | choose {s} : Expr Scope World Slot s → Expr Scope World Slot s →
      Expr Scope World Slot s → Expr Scope World Slot s
  | pullback {s t} (f : World s → World t) : Expr Scope World Slot t → Expr Scope World Slot s
  | image {s t} (f : World s → World t) : Expr Scope World Slot s → Expr Scope World Slot t
  | universal {s t} (f : World s → World t) : Expr Scope World Slot s → Expr Scope World Slot t
  | inhabited {s t} (f : World s → World t) : Expr Scope World Slot s → Expr Scope World Slot t
  | possible {s t} (f : World s → World t) : Expr Scope World Slot s → Expr Scope World Slot s
  | guaranteed {s t} (f : World s → World t) : Expr Scope World Slot s → Expr Scope World Slot s

variable {S I : Type} {W : S → Type}

def demands : {s : S} → Expr S W I s → List (I × S)
  | _, .leaf s i => [(i, s)]
  | _, .empty e | _, .full e | _, .neg e => demands e
  | _, .both a b => demands a ++ demands b
  | _, .choose c h l => demands c ++ demands h ++ demands l
  | _, .pullback _ e | _, .image _ e | _, .universal _ e | _, .inhabited _ e
  | _, .possible _ e | _, .guaranteed _ e => demands e

def admitted (actual : I → S) {s} (e : Expr S W I s) :=
  ∀ i c, (i, c) ∈ demands e → actual i = c

def eval (values : (s : S) → I → W s → Prop) : {s : S} → Expr S W I s → W s → Prop
  | _, .leaf s i, w => values s i w
  | _, .empty _, _ => False
  | _, .full _, _ => True
  | _, .neg e, w => ¬ eval values e w
  | _, .both a b, w => eval values a w ∧ eval values b w
  | _, .choose c h l, w =>
      (eval values c w ∧ eval values h w) ∨ (¬ eval values c w ∧ eval values l w)
  | _, .pullback f e, w => eval values e (f w)
  | _, .image f e, w => ∃ u, f u = w ∧ eval values e u
  | _, .universal f e, w => ∀ u, f u = w → eval values e u
  | _, .inhabited f e, w => (∃ u, f u = w) ∧ ∀ u, f u = w → eval values e u
  | _, .possible f e, w => ∃ u, f u = f w ∧ eval values e u
  | _, .guaranteed f e, w => ∀ u, f u = f w → eval values e u

/-- Retaining or reconstructing the demanded inputs suffices for the entire
heterogeneous program. Values at other slots/contexts are irrelevant; the
separate admission pass still requires every written demand. -/
theorem evaluation_depends_only_on_demands
    (first second : (s : S) → I → W s → Prop) {s} (e : Expr S W I s)
    (same : ∀ c i, (i, c) ∈ demands e → first c i = second c i) :
    eval first e = eval second e := by
  induction e with
  | leaf c i => exact same c i (by simp [demands])
  | empty e ih | full e ih => rfl
  | neg e ih => simp only [eval, ih same]
  | both a b ia ib =>
      have ha := ia (fun c i h => same c i (List.mem_append_left _ h))
      have hb := ib (fun c i h => same c i (List.mem_append_right _ h))
      simp only [eval, ha, hb]
  | choose c h l ic ih il =>
      have hc := ic (fun s i m => same s i
        (List.mem_append_left _ (List.mem_append_left _ m)))
      have hh := ih (fun s i m => same s i
        (List.mem_append_left _ (List.mem_append_right _ m)))
      have hl := il (fun s i m => same s i (List.mem_append_right _ m))
      simp only [eval, hc, hh, hl]
  | pullback f e ih | image f e ih | universal f e ih | inhabited f e ih
  | possible f e ih | guaranteed f e ih => simp only [eval, ih same]

theorem constant_results_retain_demands (actual : I → S) {s} (e : Expr S W I s) :
    (admitted actual (.empty e) ↔ admitted actual e) ∧
    (admitted actual (.full e) ↔ admitted actual e) := ⟨Iff.rfl, Iff.rfl⟩

theorem conjunction_checks_both (actual : I → S) {s} (a b : Expr S W I s) :
    admitted actual (.both a b) ↔ admitted actual a ∧ admitted actual b := by
  simp only [admitted, demands, List.mem_append, or_imp, forall_and]

theorem choice_checks_every_branch (actual : I → S) {s} (c h l : Expr S W I s) :
    admitted actual (.choose c h l) ↔
      (admitted actual c ∧ admitted actual h) ∧ admitted actual l := by
  simp only [admitted, demands, List.mem_append, or_imp, forall_and]

theorem image_keeps_input_scope {s t} (f : W s → W t) (e : Expr S W I s) :
    demands (.image f e) = demands e := rfl

theorem pullback_keeps_target_scope {s t} (f : W s → W t) (e : Expr S W I t) :
    demands (.pullback f e) = demands e := rfl

theorem typed_boundaries_cannot_retag_a_shared_variable (actual : I → S) {s t}
    (different : s ≠ t) (f : W s → W t) (i : I) :
    ¬ admitted actual (.both (.image f (.leaf s i)) (.leaf t i)) := by
  intro admitted
  have source := admitted i s (by simp [demands])
  have target := admitted i t (by simp [demands])
  exact different (source.symm.trans target)

def faultAt (actual : I → S) (roster : List (I × S)) (n : Nat) :=
  ∃ i s, roster[n]? = some (i, s) ∧ actual i ≠ s

theorem repeated_variables_retain_distinct_expected_contexts :
    faultAt (fun _ : Unit => false) [((), false), ((), true)] 1 ∧
    ¬ faultAt (fun _ : Unit => false) [((), false), ((), true)] 0 := by
  simp [faultAt]

theorem no_faults_iff_every_occurrence_is_admitted [DecidableEq S]
    (actual : I → S) {s} (e : Expr S W I s) :
    (∀ n, ¬ faultAt actual (demands e) n) ↔ admitted actual e := by
  constructor
  · intro clear i c member
    obtain ⟨n, get⟩ := List.mem_iff_getElem?.mp member
    by_cases same : actual i = c
    · exact same
    · exact False.elim (clear n ⟨i, c, get, same⟩)
  · intro good n ⟨i, c, get, different⟩
    exact different (good i c (List.mem_iff_getElem?.mpr ⟨n, get⟩))

theorem image_query_exact (values : (s : S) → I → W s → Prop) {s t}
    (f : W s → W t) (e : Expr S W I s) (y : W t) :
    eval values (.image f e) y ↔ ∃ x, f x = y ∧ eval values e x := Iff.rfl

theorem universal_query_exact (values : (s : S) → I → W s → Prop) {s t}
    (f : W s → W t) (e : Expr S W I s) (y : W t) :
    eval values (.universal f e) y ↔ ∀ x, f x = y → eval values e x := Iff.rfl

theorem inhabited_query_keeps_nonvacuity (values : (s : S) → I → W s → Prop) {s t}
    (f : W s → W t) (e : Expr S W I s) (y : W t) :
    eval values (.inhabited f e) y ↔ (∃ x, f x = y) ∧ eval values (.universal f e) y := Iff.rfl

theorem possible_is_image_then_pullback (values : (s : S) → I → W s → Prop) {s t}
    (f : W s → W t) (e : Expr S W I s) (x : W s) :
    eval values (.possible f e) x ↔ eval values (.pullback f (.image f e)) x := Iff.rfl

theorem guaranteed_is_universal_then_pullback (values : (s : S) → I → W s → Prop) {s t}
    (f : W s → W t) (e : Expr S W I s) (x : W s) :
    eval values (.guaranteed f e) x ↔ eval values (.pullback f (.universal f e)) x := Iff.rfl

theorem possible_contains_the_input (values : (s : S) → I → W s → Prop) {s t}
    (f : W s → W t) (e : Expr S W I s) (x : W s) :
    eval values e x → eval values (.possible f e) x := fun good => ⟨x, rfl, good⟩

theorem guaranteed_is_inside_the_input (values : (s : S) → I → W s → Prop) {s t}
    (f : W s → W t) (e : Expr S W I s) (x : W s) :
    eval values (.guaranteed f e) x → eval values e x := fun good => good x rfl

theorem pullback_preserves_conditional (values : (s : S) → I → W s → Prop) {s t}
    (f : W s → W t) (c h l : Expr S W I t) (x : W s) :
    eval values (.pullback f (.choose c h l)) x ↔
      eval values (.choose (.pullback f c) (.pullback f h) (.pullback f l)) x := Iff.rfl

theorem unreachable_image_distinguishes_universal_and_nonvacuous :
    let world : Bool → Type := fun b => if b then Bool else Unit
    let f : world false → world true := fun _ => false
    let input : Expr Bool world Unit false := .empty (.leaf false ())
    let values : (s : Bool) → Unit → world s → Prop := fun _ _ _ => False
    eval values (.universal f input) true ∧ ¬ eval values (.inhabited f input) true := by
  simp [eval]

#print axioms QueryMaps.evaluation_depends_only_on_demands
#print axioms QueryMaps.constant_results_retain_demands
#print axioms QueryMaps.conjunction_checks_both
#print axioms QueryMaps.choice_checks_every_branch
#print axioms QueryMaps.image_keeps_input_scope
#print axioms QueryMaps.pullback_keeps_target_scope
#print axioms QueryMaps.typed_boundaries_cannot_retag_a_shared_variable
#print axioms QueryMaps.repeated_variables_retain_distinct_expected_contexts
#print axioms QueryMaps.no_faults_iff_every_occurrence_is_admitted
#print axioms QueryMaps.image_query_exact
#print axioms QueryMaps.universal_query_exact
#print axioms QueryMaps.inhabited_query_keeps_nonvacuity
#print axioms QueryMaps.possible_is_image_then_pullback
#print axioms QueryMaps.guaranteed_is_universal_then_pullback
#print axioms QueryMaps.possible_contains_the_input
#print axioms QueryMaps.guaranteed_is_inside_the_input
#print axioms QueryMaps.pullback_preserves_conditional
#print axioms QueryMaps.unreachable_image_distinguishes_universal_and_nonvacuous

end QueryMaps
