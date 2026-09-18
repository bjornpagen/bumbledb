import Std

/-!
Checked full fibres, membership roles and the native relation-product reference.
Types X/Y/Z contain legal endpoint worlds. Their maps into E identify the shared
environment exactly. For the finite native backend these are complete Boolean
codes, never coarse cells standing in for equality of continuous parameters.

The proofs state constructor and lowering contracts. They do not verify the
Rust descriptor builder, graph kernel, mutexes, allocator or source solvers.
-/
namespace Relations

abbrev Region (X : Type) := X → Prop
abbrev Rel (X Y : Type) := X → Y → Prop
def Onto {X Y : Type} (f : X → Y) : Prop := ∀ y, ∃ x, f x = y
def Pair {X Y E : Type} (f : X → E) (g : Y → E) :=
  { p : X × Y // f p.1 = g p.2 }

theorem product_inhabited {X Y E : Type} [Nonempty E] (f : X → E) (g : Y → E)
    (lf : Onto f) (rg : Onto g) : Nonempty (Pair f g) := by
  obtain ⟨e⟩ := ‹Nonempty E›
  obtain ⟨x, hx⟩ := lf e
  obtain ⟨y, hy⟩ := rg e
  exact ⟨⟨(x, y), hx.trans hy.symm⟩⟩

theorem left_projection_onto {X Y E : Type} (f : X → E) (g : Y → E)
    (rg : Onto g) : Onto (fun p : Pair f g => p.val.1) := by
  intro x
  obtain ⟨y, hy⟩ := rg (f x)
  exact ⟨⟨(x, y), hy.symm⟩, rfl⟩

theorem right_projection_onto {X Y E : Type} (f : X → E) (g : Y → E)
    (lf : Onto f) : Onto (fun p : Pair f g => p.val.2) := by
  intro y
  obtain ⟨x, hx⟩ := lf (g y)
  exact ⟨⟨(x, y), hx⟩, rfl⟩

/-- Support is gated before the completed readouts are compared. -/
theorem product_completion_gates {X Y E : Type} (SX : Region X) (SY : Region Y)
    (rx : X → X) (ry : Y → Y) (fixX : ∀ x, SX x → rx x = x)
    (fixY : ∀ y, SY y → ry y = y) (f : X → E) (g : Y → E) (x : X) (y : Y) :
    (SX x ∧ SY y ∧ f (rx x) = g (ry y)) ↔ (SX x ∧ SY y ∧ f x = g y) := by
  constructor
  · rintro ⟨sx, sy, same⟩
    exact ⟨sx, sy, by simpa [fixX x sx, fixY y sy] using same⟩
  · rintro ⟨sx, sy, same⟩
    exact ⟨sx, sy, by simpa [fixX x sx, fixY y sy] using same⟩

/-- Bitwise equality is equality of the entire finite environment readout. -/
theorem readout_equality_exact {I : Type} (a b : I → Bool) :
    (∀ i, a i = b i) ↔ a = b := by
  constructor
  · exact funext
  · intro same i
    exact congrFun same i

def Complete {W X Y E : Type} (u : W → X) (v : W → Y) (f : X → E) (g : Y → E) : Prop :=
  ∀ x y, f x = g y → ∃ w, u w = x ∧ v w = y

def pairMap {W X Y E : Type} (u : W → X) (v : W → Y)
    (f : X → E) (g : Y → E) (commutes : ∀ w, f (u w) = g (v w)) : W → Pair f g :=
  fun w => ⟨(u w, v w), commutes w⟩

theorem joint_onto_iff_complete {W X Y E : Type} (u : W → X) (v : W → Y)
    (f : X → E) (g : Y → E) (commutes : ∀ w, f (u w) = g (v w)) :
    Onto (pairMap u v f g commutes) ↔ Complete u v f g := by
  constructor
  · intro onto x y compatible
    obtain ⟨w, same⟩ := onto ⟨(x, y), compatible⟩
    exact ⟨w, congrArg (fun p => p.val.1) same, congrArg (fun p => p.val.2) same⟩
  · intro complete p
    obtain ⟨w, hx, hy⟩ := complete p.val.1 p.val.2 p.property
    refine ⟨w, Subtype.ext ?_⟩
    exact Prod.ext hx hy

theorem square_existential_base_change {W X Y E : Type} (u : W → X) (v : W → Y)
    (f : X → E) (g : Y → E) (commutes : ∀ w, f (u w) = g (v w))
    (complete : Complete u v f g) (A : Region X) (y : Y) :
    (∃ w, A (u w) ∧ v w = y) ↔ (∃ x, A x ∧ f x = g y) := by
  constructor
  · rintro ⟨w, present, same⟩
    exact ⟨u w, present, by simpa [same] using commutes w⟩
  · rintro ⟨x, present, compatible⟩
    obtain ⟨w, hx, hy⟩ := complete x y compatible
    exact ⟨w, by simpa [hx] using present, hy⟩

theorem square_universal_base_change {W X Y E : Type} (u : W → X) (v : W → Y)
    (f : X → E) (g : Y → E) (commutes : ∀ w, f (u w) = g (v w))
    (complete : Complete u v f g) (A : Region X) (y : Y) :
    (∀ w, v w = y → A (u w)) ↔ (∀ x, f x = g y → A x) := by
  constructor
  · intro all x compatible
    obtain ⟨w, hx, hy⟩ := complete x y compatible
    simpa [hx] using all w hy
  · intro all w same
    exact all (u w) (by simpa [same] using commutes w)

def saturate {X Y : Type} (q : X → Y) (A : Region X) : Region X :=
  fun x => ∃ w, q w = q x ∧ A w
def MembershipFD {X Y : Type} (q : X → Y) (A : Region X) : Prop :=
  ∀ x w, q x = q w → (A x ↔ A w)

theorem descent_iff_membership_fd {X Y : Type} (q : X → Y) (A : Region X) :
    (∀ x, saturate q A x ↔ A x) ↔ MembershipFD q A := by
  constructor
  · intro fixed x w same
    constructor
    · intro ax; exact (fixed w).mp ⟨x, same, ax⟩
    · intro aw; exact (fixed x).mp ⟨w, same.symm, aw⟩
  · intro fd x
    constructor
    · rintro ⟨w, same, aw⟩; exact (fd w x same).mp aw
    · intro ax; exact ⟨x, rfl, ax⟩

theorem descent_unique {X Y : Type} (q : X → Y) (onto : Onto q)
    (A B : Region Y) (equal : ∀ x, A (q x) ↔ B (q x)) : ∀ y, A y ↔ B y := by
  intro y
  obtain ⟨x, same⟩ := onto y
  simpa [same] using equal x

def Triple {X Y Z E : Type} (f : X → E) (g : Y → E) (h : Z → E)
    (x : X) (y : Y) (z : Z) : Prop := f x = g y ∧ g y = h z

theorem triple_projects_complete_pairs {X Y Z E : Type}
    (f : X → E) (g : Y → E) (h : Z → E)
    (of : Onto f) (og : Onto g) (oh : Onto h) :
    (∀ x y, f x = g y → ∃ z, Triple f g h x y z) ∧
    (∀ y z, g y = h z → ∃ x, Triple f g h x y z) ∧
    (∀ x z, f x = h z → ∃ y, Triple f g h x y z) := by
  constructor
  · intro x y same
    obtain ⟨z, hz⟩ := oh (g y)
    exact ⟨z, same, hz.symm⟩
  constructor
  · intro y z same
    obtain ⟨x, hx⟩ := of (g y)
    exact ⟨x, hx, same⟩
  · intro x z same
    obtain ⟨y, hy⟩ := og (f x)
    exact ⟨y, hy.symm, hy.trans same⟩

def Supported {X Y E : Type} (f : X → E) (g : Y → E) (R : Rel X Y) : Prop :=
  ∀ x y, R x y → f x = g y
def comp {X Y Z : Type} (R : Rel X Y) (Q : Rel Y Z) : Rel X Z :=
  fun x z => ∃ y, R x y ∧ Q y z
def staged {X Y Z E : Type} (f : X → E) (g : Y → E) (h : Z → E)
    (R : Rel X Y) (Q : Rel Y Z) : Rel X Z :=
  fun x z => f x = h z ∧ ∃ y, Triple f g h x y z ∧ R x y ∧ Q y z

theorem shared_product_exact {X Y Z E : Type} (f : X → E) (g : Y → E) (h : Z → E)
    (R : Rel X Y) (Q : Rel Y Z) (sr : Supported f g R) (sq : Supported g h Q)
    (x : X) (z : Z) : staged f g h R Q x z ↔ comp R Q x z := by
  constructor
  · rintro ⟨_, y, _, r, q⟩
    exact ⟨y, r, q⟩
  · rintro ⟨y, r, q⟩
    exact ⟨(sr x y r).trans (sq y z q), y, ⟨sr x y r, sq y z q⟩, r, q⟩

def leftResidual {X Y Z E : Type} (g : Y → E) (h : Z → E)
    (R : Rel X Y) (V : Rel X Z) : Rel Y Z :=
  fun y z => g y = h z ∧ ∀ x, R x y → V x z
def rightResidual {X Y Z E : Type} (f : X → E) (g : Y → E)
    (V : Rel X Z) (Q : Rel Y Z) : Rel X Y :=
  fun x y => f x = g y ∧ ∀ z, Q y z → V x z
def included {X Y : Type} (R Q : Rel X Y) := ∀ x y, R x y → Q x y

theorem staged_left_residual {X Y Z E : Type} (f : X → E) (g : Y → E) (h : Z → E)
    (R : Rel X Y) (V : Rel X Z) (sr : Supported f g R) (y : Y) (z : Z) :
    (g y = h z ∧ ¬ ∃ x, Triple f g h x y z ∧ R x y ∧ ¬ V x z) ↔
      leftResidual g h R V y z := by
  classical
  constructor
  · rintro ⟨same, no⟩
    refine ⟨same, fun x r => Classical.byContradiction ?_⟩
    intro bad
    exact no ⟨x, ⟨sr x y r, same⟩, r, bad⟩
  · rintro ⟨same, all⟩
    refine ⟨same, ?_⟩
    rintro ⟨x, _, r, bad⟩
    exact bad (all x r)

theorem staged_right_residual {X Y Z E : Type} (f : X → E) (g : Y → E) (h : Z → E)
    (V : Rel X Z) (Q : Rel Y Z) (sq : Supported g h Q) (x : X) (y : Y) :
    (f x = g y ∧ ¬ ∃ z, Triple f g h x y z ∧ Q y z ∧ ¬ V x z) ↔
      rightResidual f g V Q x y := by
  classical
  constructor
  · rintro ⟨same, no⟩
    refine ⟨same, fun z q => Classical.byContradiction ?_⟩
    intro bad
    exact no ⟨z, ⟨same, sq y z q⟩, q, bad⟩
  · rintro ⟨same, all⟩
    refine ⟨same, ?_⟩
    rintro ⟨z, _, q, bad⟩
    exact bad (all z q)

theorem left_adjunction {X Y Z E : Type} (g : Y → E) (h : Z → E)
    (R : Rel X Y) (Q : Rel Y Z) (V : Rel X Z) (sq : Supported g h Q) :
    included (comp R Q) V ↔ included Q (leftResidual g h R V) := by
  constructor
  · intro bound y z q
    exact ⟨sq y z q, fun x r => bound x z ⟨y, r, q⟩⟩
  · intro bound x z ⟨y, r, q⟩
    exact (bound y z q).2 x r

theorem right_adjunction {X Y Z E : Type} (f : X → E) (g : Y → E)
    (R : Rel X Y) (Q : Rel Y Z) (V : Rel X Z) (sr : Supported f g R) :
    included (comp R Q) V ↔ included R (rightResidual f g V Q) := by
  constructor
  · intro bound x y r
    exact ⟨sr x y r, fun z q => bound x z ⟨y, r, q⟩⟩
  · intro bound x z ⟨y, r, q⟩
    exact (bound x y r).2 z q

/-- Clipping a function graph to a product can hide an environment mismatch. -/
theorem graph_total_iff_environment_preserved {X Y E : Type}
    (f : X → E) (g : Y → E) (readout : X → Y) :
    (∀ x, ∃ y, f x = g y ∧ readout x = y) ↔ (∀ x, f x = g (readout x)) := by
  constructor
  · intro total x
    obtain ⟨y, same, mapped⟩ := total x
    simpa [mapped] using same
  · intro preserves x
    exact ⟨readout x, preserves x, rfl⟩

def Functional {X Y : Type} (R : Rel X Y) := ∀ x y z, R x y → R x z → y = z
def BitConflict {X I : Type} (R : Rel X (I → Bool)) : Prop :=
  ∃ x i y z, R x y ∧ R x z ∧ y i = true ∧ z i = false

theorem bit_conflicts_iff_nonfunctional {X I : Type} (R : Rel X (I → Bool)) :
    ¬ BitConflict R ↔ Functional R := by
  constructor
  · intro no x y z ry rz
    funext i
    cases hy : y i <;> cases hz : z i
    · rfl
    · exact False.elim (no ⟨x, i, z, y, rz, ry, hz, hy⟩)
    · exact False.elim (no ⟨x, i, y, z, ry, rz, hy, hz⟩)
    · rfl
  · intro functional ⟨x, i, y, z, ry, rz, hy, hz⟩
    have same := congrFun (functional x y z ry rz) i
    rw [hy, hz] at same
    cases same

/-- In a total functional relation the May readout of each bit recovers exactly
    the corresponding coordinate of the unique related output. -/
theorem graph_bit_readout_exact {X I : Type} (R : Rel X (I → Bool))
    (functional : Functional R) (x : X) (y : I → Bool) (present : R x y) (i : I) :
    (∃ z, R x z ∧ z i = true) ↔ y i = true := by
  constructor
  · rintro ⟨z, related, bit⟩
    simpa [functional x z y related present] using bit
  · intro bit
    exact ⟨y, present, bit⟩

theorem individually_onto_is_not_complete :
    Onto (id : Bool → Bool) ∧ Onto (id : Bool → Bool) ∧
    ¬ Complete (id : Bool → Bool) id (fun _ => ()) (fun _ => ()) := by
  refine ⟨fun x => ⟨x, rfl⟩, fun x => ⟨x, rfl⟩, ?_⟩
  intro complete
  obtain ⟨w, hx, hy⟩ := complete false true rfl
  exact Bool.false_ne_true (hx.symm.trans hy)

theorem full_has_every_role_but_does_not_certify_product :
    MembershipFD (id : Bool → Bool) (fun _ => True) ∧
    ¬ Complete (id : Bool → Bool) id (fun _ => ()) (fun _ => ()) := by
  exact ⟨fun _ _ _ => Iff.rfl, individually_onto_is_not_complete.2.2⟩

theorem dropping_middle_witness_changes_composition :
    (∃ t : Bool, t = false) ∧ (∃ t : Bool, t = true) ∧
    ¬ comp (fun (_ : Unit) t => t = false) (fun t (_ : Unit) => t = true) () () := by
  refine ⟨⟨false, rfl⟩, ⟨true, rfl⟩, ?_⟩
  rintro ⟨t, hf, ht⟩
  exact Bool.false_ne_true (hf.symm.trans ht)

#print axioms Relations.product_inhabited
#print axioms Relations.left_projection_onto
#print axioms Relations.right_projection_onto
#print axioms Relations.product_completion_gates
#print axioms Relations.readout_equality_exact
#print axioms Relations.joint_onto_iff_complete
#print axioms Relations.square_existential_base_change
#print axioms Relations.square_universal_base_change
#print axioms Relations.descent_iff_membership_fd
#print axioms Relations.descent_unique
#print axioms Relations.triple_projects_complete_pairs
#print axioms Relations.shared_product_exact
#print axioms Relations.staged_left_residual
#print axioms Relations.staged_right_residual
#print axioms Relations.left_adjunction
#print axioms Relations.right_adjunction
#print axioms Relations.graph_total_iff_environment_preserved
#print axioms Relations.bit_conflicts_iff_nonfunctional
#print axioms Relations.graph_bit_readout_exact
#print axioms Relations.individually_onto_is_not_complete
#print axioms Relations.full_has_every_role_but_does_not_certify_product
#print axioms Relations.dropping_middle_witness_changes_composition

end Relations
