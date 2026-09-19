import Std

/-!
An acyclic, signed diagram reference and its public reconstruction contract.
Every reference addresses an earlier admitted prefix. Arbitrarily shared nodes
are permitted. The reference uses newest-first prefix indices; the Rust snapshot
uses low-first postorder indices. No theorem verifies that native translation,
table packing, allocation, locking or hashing. Native tests cover those boundaries.

Table and split readouts are Boolean functions on raw codes. Legal support is a
separate root. A reconstruction may change representation or use a retraction,
provided its constructor interpretation satisfies the stated pointwise laws.
-/
namespace Inspection

abbrev Bits (W : Type) := W → Bool

inductive Ref (n : Nat) where
  | constant (value : Bool)
  | node (index : Fin n) (complemented : Bool)

inductive Node (W : Type) (n : Nat) where
  | table (value : Bits W)
  | split (readout : Bits W) (low high : Ref n)

inductive Graph (W : Type) : Nat → Type where
  | empty : Graph W 0
  | push {n : Nat} (prior : Graph W n) (node : Node W n) : Graph W (n + 1)

structure Algebra (W A : Type) where
  constant : Bool → A
  table : Bits W → A
  complement : A → A
  branch : Bits W → A → A → A

def boolean (W : Type) : Algebra W (Bits W) where
  constant b := fun _ => b
  table f := f
  complement f := fun w => !(f w)
  branch c high low := fun w => if c w then high w else low w

def read {W A : Type} {n : Nat} (alg : Algebra W A) (values : Fin n → A) : Ref n → A
  | .constant b => alg.constant b
  | .node i neg => if neg then alg.complement (values i) else values i

def buildNode {W A : Type} {n : Nat} (alg : Algebra W A) (values : Fin n → A) :
    Node W n → A
  | .table f => alg.table f
  | .split c low high => alg.branch c (read alg values high) (read alg values low)

def build {W A : Type} (alg : Algebra W A) : {n : Nat} → Graph W n → Fin n → A
  | _, .empty => Fin.elim0
  | _, .push prior node => Fin.cases (buildNode alg (build alg prior) node) (build alg prior)

/-- Constructor correspondence is an obligation of each carrier/adapter.
    rho is identity for a raw rebuild, or a decoder for completed constructors. -/
structure Sound {W V A : Type} (alg : Algebra W A) (denote : A → Bits V) (rho : V → W) : Prop where
  constant : ∀ b v, denote (alg.constant b) v = b
  table : ∀ f v, denote (alg.table f) v = f (rho v)
  complement : ∀ a v, denote (alg.complement a) v = !(denote a v)
  branch : ∀ c high low v, denote (alg.branch c high low) v =
    if c (rho v) then denote high v else denote low v

theorem read_exact {W V A : Type} {n : Nat} (alg : Algebra W A)
    (denote : A → Bits V) (rho : V → W) (sound : Sound alg denote rho)
    (values : Fin n → A) (meanings : Fin n → Bits W)
    (same : ∀ i v, denote (values i) v = meanings i (rho v)) (ref : Ref n) (v : V) :
    denote (read alg values ref) v = read (boolean W) meanings ref (rho v) := by
  cases ref with
  | constant b => exact sound.constant b v
  | node i neg =>
    cases neg
    · exact same i v
    · exact (sound.complement (values i) v).trans (congrArg Bool.not (same i v))

theorem node_exact {W V A : Type} {n : Nat} (alg : Algebra W A)
    (denote : A → Bits V) (rho : V → W) (sound : Sound alg denote rho)
    (values : Fin n → A) (meanings : Fin n → Bits W)
    (same : ∀ i v, denote (values i) v = meanings i (rho v)) (node : Node W n) (v : V) :
    denote (buildNode alg values node) v = buildNode (boolean W) meanings node (rho v) := by
  cases node with
  | table f => exact sound.table f v
  | split c low high =>
    simp only [buildNode, sound.branch, boolean]
    rw [read_exact alg denote rho sound values meanings same high v,
      read_exact alg denote rho sound values meanings same low v]
    rfl

/-- Induction over an admitted acyclic prefix covers arbitrary DAG sharing,
    signed child references and all roots, without expanding paths to a tree. -/
theorem graph_exact {W V A : Type} {n : Nat} (alg : Algebra W A)
    (denote : A → Bits V) (rho : V → W) (sound : Sound alg denote rho)
    (graph : Graph W n) : ∀ i v,
    denote (build alg graph i) v = build (boolean W) graph i (rho v) := by
  induction graph with
  | empty => intro i; exact Fin.elim0 i
  | push prior node ih =>
    intro i v
    refine Fin.cases ?_ (fun j => ?_) i
    · exact node_exact alg denote rho sound _ _ ih node v
    · exact ih j v

theorem root_exact {W V A : Type} {n : Nat} (alg : Algebra W A)
    (denote : A → Bits V) (rho : V → W) (sound : Sound alg denote rho)
    (graph : Graph W n) (root : Ref n) (v : V) :
    denote (read alg (build alg graph) root) v =
      read (boolean W) (build (boolean W) graph) root (rho v) := by
  exact read_exact alg denote rho sound _ _ (graph_exact alg denote rho sound graph) root v

def observe {W : Type} (support region : Bits W) (w : W) : Option Bool :=
  if support w then some (region w) else none

/-- Original support must be checked as a raw predicate before reconstruction
    can be interpreted as legal Event membership. Constant regions obey this too. -/
theorem two_root_reconstruction {W A : Type} {n : Nat} (alg : Algebra W A)
    (denote : A → Bits W) (sound : Sound alg denote id) (graph : Graph W n)
    (support region : Ref n) (w : W) :
    observe (denote (read alg (build alg graph) support))
      (denote (read alg (build alg graph) region)) w =
    observe (read (boolean W) (build (boolean W) graph) support)
      (read (boolean W) (build (boolean W) graph) region) w := by
  unfold observe
  rw [root_exact alg denote id sound graph support w,
    root_exact alg denote id sound graph region w]
  rfl

theorem original_support_removes_decoder_aliases {W : Type} (support region : Bits W)
    (rho : W → W) (fixes : ∀ w, support w = true → rho w = w) (w : W) :
    (support w && region (rho w)) = (support w && region w) := by
  cases legal : support w
  · simp
  · simp [fixes w legal]

theorem retraction_rebuild_preserves_membership {W A : Type} {n : Nat}
    (alg : Algebra W A) (denote : A → Bits W) (rho : W → W)
    (sound : Sound alg denote rho) (graph : Graph W n) (root : Ref n)
    (support : Bits W) (fixes : ∀ w, support w = true → rho w = w) (w : W) :
    observe support (denote (read alg (build alg graph) root)) w =
      observe support (read (boolean W) (build (boolean W) graph) root) w := by
  unfold observe
  cases legal : support w
  · rfl
  · simp only [↓reduceIte]
    rw [root_exact alg denote rho sound graph root w, fixes w legal]

/-- Native complements are sign bits. Inspection must propagate a split's sign
    to both children, including constants and complemented local tables. -/
theorem split_polarity (condition high low : Bool) :
    (!(if condition then high else low)) = (if condition then !high else !low) := by
  cases condition <;> rfl

theorem regular_sign_involution (value sign : Bool) :
    (if sign then !(if sign then !value else value) else (if sign then !value else value)) = value := by
  cases sign <;> simp

theorem true_leaf_is_not_a_legal_witness :
    (fun _ : Bool => true) false = true ∧
    observe (fun w : Bool => w) (fun _ => true) false = none := by
  exact ⟨rfl, rfl⟩

#print axioms Inspection.read_exact
#print axioms Inspection.node_exact
#print axioms Inspection.graph_exact
#print axioms Inspection.root_exact
#print axioms Inspection.two_root_reconstruction
#print axioms Inspection.original_support_removes_decoder_aliases
#print axioms Inspection.retraction_rebuild_preserves_membership
#print axioms Inspection.split_polarity
#print axioms Inspection.regular_sign_involution
#print axioms Inspection.true_leaf_is_not_a_legal_witness

end Inspection
