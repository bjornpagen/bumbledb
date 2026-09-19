import Std

/-!
Native information lowering, explicit evidence, bitwise FD factorization and
the environment gate on a relation view. Types below contain legal worlds;
finite Boolean encoding, exact image/pullback and support admission are separate
native obligations. No prior, probability, hidden information or observer memory
is inferred from an arbitrary supplied readout.
-/
namespace Information

abbrev Region (X : Type) := X → Prop
def image {X Y : Type} (f : X → Y) (A : Region X) : Region Y :=
  fun y => ∃ x, A x ∧ f x = y
def possible {X Y : Type} (f : X → Y) (A : Region X) : Region X :=
  fun w => image f A (f w)
def guaranteed {X Y : Type} (f : X → Y) (A : Region X) : Region X :=
  fun w => ¬ possible f (fun x => ¬ A x) w
def stable {X Y : Type} (f : X → Y) (A : Region X) :=
  ∀ v w, f v = f w → (A v ↔ A w)
def determines {X Y Z : Type} (f : X → Y) (g : X → Z) :=
  ∀ v w, f v = f w → g v = g w
def onto {X Y : Type} (f : X → Y) := ∀ y, ∃ x, f x = y

theorem image_pullback_exact {X Y : Type} (f : X → Y) (A : Region X) (w : X) :
    possible f A w ↔ ∃ v, f v = f w ∧ A v := by
  exact ⟨fun ⟨v, a, same⟩ => ⟨v, same, a⟩, fun ⟨v, same, a⟩ => ⟨v, a, same⟩⟩

theorem complement_image_exact {X Y : Type} (f : X → Y) (A : Region X) (w : X) :
    guaranteed f A w ↔ ∀ v, f v = f w → A v := by
  classical
  constructor
  · intro no v same
    exact Classical.byContradiction (fun bad => no ⟨v, bad, same⟩)
  · intro all ⟨v, bad, same⟩
    exact bad (all v same)

theorem observable_sandwich {X Y : Type} (f : X → Y) (A : Region X) (w : X) :
    (guaranteed f A w → A w) ∧ (A w → possible f A w) := by
  exact ⟨fun h => (complement_image_exact f A w).mp h w rfl, fun a => ⟨w, a, rfl⟩⟩

theorem possible_is_stable {X Y : Type} (f : X → Y) (A : Region X) :
    stable f (possible f A) := by
  intro v w same
  unfold possible
  rw [same]

theorem fixed_iff_membership_fd {X Y : Type} (f : X → Y) (A : Region X) :
    (∀ w, possible f A w ↔ A w) ↔ stable f A := by
  constructor
  · intro fixed v w same
    exact (fixed v).symm.trans ((possible_is_stable f A v w same).trans (fixed w))
  · intro fd w
    constructor
    · rintro ⟨v, present, same⟩
      exact (fd v w same).mp present
    · exact fun present => ⟨w, present, rfl⟩

def positive {X Y : Type} (f : X → Y) (A G : Region X) :=
  possible f (fun x => G x ∧ A x)
def negative {X Y : Type} (f : X → Y) (A G : Region X) :=
  possible f (fun x => G x ∧ ¬ A x)

theorem evidence_reach_is_union {X Y : Type} (f : X → Y) (A G : Region X) (w : X) :
    (positive f A G w ∨ negative f A G w) ↔ possible f G w := by
  classical
  constructor
  · rintro (⟨v, ⟨g, _⟩, same⟩ | ⟨v, ⟨g, _⟩, same⟩) <;> exact ⟨v, g, same⟩
  · rintro ⟨v, g, same⟩
    by_cases a : A v
    · exact Or.inl ⟨v, ⟨g, a⟩, same⟩
    · exact Or.inr ⟨v, ⟨g, a⟩, same⟩

theorem evidence_guarantee_exact {X Y : Type} (f : X → Y) (A G : Region X) (w : X) :
    (positive f A G w ∧ ¬ negative f A G w) ↔
      (possible f G w ∧ ∀ v, f v = f w → G v → A v) := by
  classical
  constructor
  · rintro ⟨yes, no⟩
    refine ⟨(evidence_reach_is_union f A G w).mp (Or.inl yes), ?_⟩
    intro v same g
    exact Classical.byContradiction (fun bad => no ⟨v, ⟨g, bad⟩, same⟩)
  · rintro ⟨⟨v, g, same⟩, all⟩
    refine ⟨⟨v, ⟨g, all v same g⟩, same⟩, ?_⟩
    rintro ⟨x, ⟨gx, bad⟩, hx⟩
    exact bad (all x hx gx)

theorem three_cases_partition (yes no : Prop) :
    (((yes ∧ ¬ no) ∨ (no ∧ ¬ yes) ∨ (yes ∧ no)) ↔ (yes ∨ no)) ∧
    ¬ ((yes ∧ ¬ no) ∧ (no ∧ ¬ yes)) ∧
    ¬ ((yes ∧ ¬ no) ∧ (yes ∧ no)) ∧
    ¬ ((no ∧ ¬ yes) ∧ (yes ∧ no)) := by
  classical
  by_cases hy : yes <;> by_cases hn : no <;> simp [hy, hn]

theorem impossible_evidence_has_no_cases {X Y : Type} (f : X → Y) (A G : Region X)
    (impossible : ∀ x, ¬ G x) (w : X) :
    ¬ positive f A G w ∧ ¬ negative f A G w ∧ ¬ possible f G w := by
  exact ⟨fun ⟨v, ⟨g, _⟩, _⟩ => impossible v g,
    fun ⟨v, ⟨g, _⟩, _⟩ => impossible v g,
    fun ⟨v, g, _⟩ => impossible v g⟩

theorem bitwise_fd_iff_readout_fd {X Y I : Type} (f : X → Y) (g : X → I → Bool) :
    (∀ i, stable f (fun x => g x i = true)) ↔ determines f g := by
  constructor
  · intro bits v w same
    funext i
    have equal := bits i v w same
    cases hv : g v i <;> cases hw : g w i <;> simp_all
  · intro fd i v w same
    change g v i = true ↔ g w i = true
    rw [fd v w same]

/-- Once every coordinate descends through one readout, those coordinates
    assemble one factor. Onto plus original map support certifies its range. -/
theorem descended_bits_form_legal_factor {X Y I : Type} (f : X → Y)
    (reaches : onto f) (g : X → I → Bool) (bits : I → Y → Bool)
    (descended : ∀ x i, bits i (f x) = g x i)
    (legal : (I → Bool) → Prop) (maps : ∀ x, legal (g x)) :
    (∀ x, (fun i => bits i (f x)) = g x) ∧
    (∀ y, legal (fun i => bits i y)) := by
  have factor : ∀ x, (fun i => bits i (f x)) = g x := fun x => funext (descended x)
  refine ⟨factor, ?_⟩
  intro y
  obtain ⟨x, same⟩ := reaches y
  rw [← same, factor x]
  exact maps x

theorem onto_factor_unique {X Y Z : Type} (f : X → Y) (reaches : onto f)
    (a b : Y → Z) (same : ∀ x, a (f x) = b (f x)) : a = b := by
  funext y
  obtain ⟨x, hx⟩ := reaches y
  simpa [hx] using same x

def kernel {X Y : Type} (f : X → Y) := fun x y => f x = f y

theorem kernel_is_equivalence {X Y : Type} (f : X → Y) :
    (∀ x, kernel f x x) ∧
    (∀ x y, kernel f x y → kernel f y x) ∧
    (∀ x y z, kernel f x y → kernel f y z → kernel f x z) := by
  exact ⟨fun _ => rfl, fun _ _ => Eq.symm, fun _ _ _ => Eq.trans⟩

theorem kernel_may_is_possible {X Y : Type} (f : X → Y) (A : Region X) (w : X) :
    (∃ v, kernel f w v ∧ A v) ↔ possible f A w := by
  exact ⟨fun ⟨v, same, a⟩ => ⟨v, a, same.symm⟩,
    fun ⟨v, a, same⟩ => ⟨v, same.symm, a⟩⟩

theorem kernel_all_is_guaranteed {X Y : Type} (f : X → Y) (A : Region X) (w : X) :
    (∀ v, kernel f w v → A v) ↔ guaranteed f A w := by
  rw [complement_image_exact]
  exact ⟨fun all v same => all v same.symm, fun all v same => all v same.symm⟩

theorem kernel_inclusion_iff_fd {X Y Z : Type} (f : X → Y) (g : X → Z) :
    (∀ v w, kernel f v w → kernel g v w) ↔ determines f g := by
  rfl

/-- A pair-product environment must already be determined by the observation.
    Otherwise intersecting it with the kernel changes the information cells. -/
theorem kernel_product_gate_iff_bitwise_fd {X Y I : Type}
    (f : X → Y) (environment : X → I → Bool) :
    (∀ v w, (environment v = environment w ∧ kernel f v w) ↔ kernel f v w) ↔
      (∀ i, stable f (fun x => environment x i = true)) := by
  rw [bitwise_fd_iff_readout_fd]
  constructor
  · intro exact v w same
    exact ((exact v w).mpr same).1
  · intro fd v w
    exact ⟨And.right, fun same => ⟨fd v w same, same⟩⟩

theorem clipping_changes_an_insufficient_observation :
    possible (fun _ : Bool => ()) (fun b => b = true) false ∧
    ¬ (∃ b : Bool, (false = b ∧ kernel (fun _ : Bool => ()) false b) ∧ b = true) := by
  refine ⟨⟨true, rfl, rfl⟩, ?_⟩
  rintro ⟨b, ⟨same, _⟩, yes⟩
  exact Bool.false_ne_true (same.trans yes)

theorem evidence_answers_whole_cells :
    positive (fun _ : Bool => ()) (fun b => b = true) (fun b => b = true) false ∧
    ¬ negative (fun _ : Bool => ()) (fun b => b = true) (fun b => b = true) false ∧
    ¬ (false = true) := by
  exact ⟨⟨true, ⟨rfl, rfl⟩, rfl⟩, fun ⟨_, ⟨yes, no⟩, _⟩ => no yes, Bool.false_ne_true⟩

#print axioms Information.image_pullback_exact
#print axioms Information.complement_image_exact
#print axioms Information.observable_sandwich
#print axioms Information.possible_is_stable
#print axioms Information.fixed_iff_membership_fd
#print axioms Information.evidence_reach_is_union
#print axioms Information.evidence_guarantee_exact
#print axioms Information.three_cases_partition
#print axioms Information.impossible_evidence_has_no_cases
#print axioms Information.bitwise_fd_iff_readout_fd
#print axioms Information.descended_bits_form_legal_factor
#print axioms Information.onto_factor_unique
#print axioms Information.kernel_is_equivalence
#print axioms Information.kernel_may_is_possible
#print axioms Information.kernel_all_is_guaranteed
#print axioms Information.kernel_inclusion_iff_fd
#print axioms Information.kernel_product_gate_iff_bitwise_fd
#print axioms Information.clipping_changes_an_insufficient_observation
#print axioms Information.evidence_answers_whole_cells

end Information
