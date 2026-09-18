import Std

/-!
Complete-binding Event query semantics. Operand participation is syntactic;
truth-function simplification cannot remove it. Diagnostics denote a set over
logical stage/program/operand descriptors, not the first traversal failure.

The Rust compiler must preserve body-binding membership, written provenance,
operand numbering and canonical descriptor identity. The kernel proves this
reference contract, not those implementation correspondences or the Rust parser.
-/
namespace Query

abbrev Region (W : Type) := W → Bool

inductive Expr (V : Type) where
  | operand (v : V)
  | empty (anchor : V)
  | full (anchor : V)
  | neg (a : Expr V)
  | apply (op : Bool → Bool → Bool) (a b : Expr V)
  | ite (c h l : Expr V)

def anchor {V : Type} : Expr V → V
  | .operand v | .empty v | .full v => v
  | .neg a | .apply _ a _ | .ite a _ _ => anchor a

def operands {V : Type} : Expr V → List V
  | .operand v | .empty v | .full v => [v]
  | .neg a => operands a
  | .apply _ a b => operands a ++ operands b
  | .ite c h l => operands c ++ operands h ++ operands l

def evaluate {V W : Type} (values : V → Region W) : Expr V → Region W
  | .operand v => values v
  | .empty _ => fun _ => false
  | .full _ => fun _ => true
  | .neg a => fun w => !(evaluate values a w)
  | .apply op a b => fun w => op (evaluate values a w) (evaluate values b w)
  | .ite c h l => fun w => if evaluate values c w then evaluate values h w else evaluate values l w

theorem anchor_participates {V : Type} (e : Expr V) : anchor e ∈ operands e := by
  induction e with
  | operand v | empty v | full v => simp [anchor, operands]
  | neg a ih => exact ih
  | apply op a b ia ib => simp only [anchor, operands, List.mem_append]; exact Or.inl ia
  | ite c h l ic ih il => simp only [anchor, operands, List.mem_append]; exact Or.inl (Or.inl ic)

def Compatible {V C : Type} (context : V → C) (e : Expr V) :=
  ∀ v ∈ operands e, context v = context (anchor e)

def Fault {V C : Type} (context : V → C) (e : Expr V) (v : V) :=
  v ∈ operands e ∧ context v ≠ context (anchor e)

theorem compatible_iff_no_fault {V C : Type} (context : V → C) (e : Expr V) :
    Compatible context e ↔ ∀ v, ¬ Fault context e v := by
  constructor
  · intro compatible v bad
    exact bad.2 (compatible v bad.1)
  · intro clear v member
    exact Classical.byContradiction (fun different => clear v ⟨member, different⟩)

theorem truth_function_preserves_participation {V : Type}
    (f g : Bool → Bool → Bool) (a b : Expr V) :
    operands (.apply f a b) = operands (.apply g a b) := rfl

theorem truth_function_preserves_faults {V C : Type} (context : V → C)
    (f g : Bool → Bool → Bool) (a b : Expr V) (v : V) :
    Fault context (.apply f a b) v ↔ Fault context (.apply g a b) v := Iff.rfl

theorem constant_false_still_checks_other_operand :
    Fault (fun n : Nat => n) (.apply (fun _ _ => false) (.operand 0) (.operand 1)) 1 := by
  simp [Fault, operands, anchor]

theorem constant_true_still_checks_other_operand :
    Fault (fun n : Nat => n) (.apply (fun _ _ => true) (.operand 0) (.operand 1)) 1 := by
  simp [Fault, operands, anchor]

theorem unselected_ite_branch_participates :
    Fault (fun n : Nat => n) (.ite (.full 0) (.operand 0) (.operand 1)) 1 := by
  simp [Fault, operands, anchor]

theorem empty_is_a_constructed_value {V W : Type} (values : V → Region W) (v : V) :
    evaluate values (.empty v) = fun _ => false := rfl

theorem complement_of_empty_is_full {V W : Type} (values : V → Region W) (v : V) :
    evaluate values (.neg (.empty v)) = evaluate values (.full v) := rfl

theorem contradiction_is_empty {V W : Type} (values : V → Region W) (a : Expr V) :
    evaluate values (.apply Bool.and a (.neg a)) = fun _ => false := by
  funext w
  simp [evaluate]

theorem excluded_middle_is_full {V W : Type} (values : V → Region W) (a : Expr V) :
    evaluate values (.apply Bool.or a (.neg a)) = fun _ => true := by
  funext w
  simp [evaluate]

def trueCount : List Bool → Nat
  | [] => 0
  | value :: rest => (if value then 1 else 0) + trueCount rest

def cardinality (minimum maximum : Nat) (values : List Bool) : Bool :=
  decide (minimum ≤ trueCount values ∧ trueCount values ≤ maximum)

theorem cardinality_exact (minimum maximum : Nat) (values : List Bool) :
    cardinality minimum maximum values = true ↔
      minimum ≤ trueCount values ∧ trueCount values ≤ maximum := by
  simp [cardinality]

theorem equal_roster_positions_are_counted_separately :
    cardinality 2 2 [true, true] = true ∧ cardinality 2 2 [true] = false := by decide

theorem cardinality_counts_same_world {W : Type} (a b : Region W) (w : W) :
    cardinality 2 2 [a w, a w, b w] = (a w && !(b w)) := by
  cases a w <;> cases b w <;> decide

def collect {B F : Type} (fault : B → F → Prop) : List B → F → Prop
  | [] => fun _ => False
  | row :: rest => fun f => fault row f ∨ collect fault rest f

theorem collect_exact {B F : Type} (fault : B → F → Prop) (rows : List B) (f : F) :
    collect fault rows f ↔ ∃ row ∈ rows, fault row f := by
  induction rows with
  | nil => simp [collect]
  | cons row rest ih => simp [collect, ih]

theorem traversal_invariant {B F : Type} (fault : B → F → Prop)
    (first second : List B) (same : ∀ row, row ∈ first ↔ row ∈ second) (f : F) :
    collect fault first f ↔ collect fault second f := by
  simp only [collect_exact]
  constructor
  · rintro ⟨row, member, bad⟩; exact ⟨row, (same row).mp member, bad⟩
  · rintro ⟨row, member, bad⟩; exact ⟨row, (same row).mpr member, bad⟩

theorem duplicate_binding_is_idempotent {B F : Type} (fault : B → F → Prop)
    (row : B) (rest : List B) (f : F) :
    collect fault (row :: row :: rest) f ↔ collect fault (row :: rest) f := by
  simp [collect]

theorem unmatched_operand_is_not_a_fault {B F : Type} (rows : List B)
    (fault : B → F → Prop) (f : F) (none : ∀ row ∈ rows, ¬ fault row f) :
    ¬ collect fault rows f := by
  intro present
  obtain ⟨row, member, bad⟩ := (collect_exact fault rows f).mp present
  exact none row member bad

theorem later_fault_survives_earlier_saturation {B F : Type} (fault : B → F → Prop)
    (first second : B) (f : F) (bad : fault second f) :
    collect fault [first, second] f := Or.inr (Or.inl bad)

/-- Normalization may combine arms, but must retain every written-rule stamp. -/
def programFaults {B P R F : Type} (bindings : List B) (programs : List P)
    (written : P → List R) (fault : B → P → R → F → Prop) (f : F) :=
  ∃ b ∈ bindings, ∃ p ∈ programs, ∃ r ∈ written p, fault b p r f

theorem program_schedule_invariant {B P R F : Type} (a b : List B) (ps qs : List P)
    (rows : ∀ x, x ∈ a ↔ x ∈ b) (programs : ∀ x, x ∈ ps ↔ x ∈ qs)
    (written : P → List R) (fault : B → P → R → F → Prop) (f : F) :
    programFaults a ps written fault f ↔ programFaults b qs written fault f := by
  unfold programFaults
  constructor
  · rintro ⟨x, hx, p, hp, r, hr, bad⟩
    exact ⟨x, (rows x).mp hx, p, (programs p).mp hp, r, hr, bad⟩
  · rintro ⟨x, hx, p, hp, r, hr, bad⟩
    exact ⟨x, (rows x).mpr hx, p, (programs p).mpr hp, r, hr, bad⟩

inductive Finished (F A : Type) where
  | success (answer : A)
  | faults (complete : F → Prop)
  | refused

/-- A finished fault set needs equality with the complete participating set.
Resource/scalar refusal is not a claim about any partial accumulated set. -/
def Certified {F A : Type} (allFaults : F → Prop) : Finished F A → Prop
  | .success _ => ∀ f, ¬ allFaults f
  | .faults reported => (∃ f, reported f) ∧ (∀ f, reported f ↔ allFaults f)
  | .refused => True

theorem no_answer_with_participating_fault {F A : Type} (allFaults : F → Prop)
    (answer : A) (f : F) (bad : allFaults f) : ¬ Certified allFaults (.success answer) := by
  intro certified
  exact certified f bad

theorem partial_collection_cannot_claim_completion {F A : Type}
    (allFaults reported : F → Prop) (f : F) (bad : allFaults f) (missing : ¬ reported f) :
    ¬ Certified allFaults (Finished.faults reported : Finished F A) := by
  intro certified
  exact missing ((certified.2 f).mpr bad)

#print axioms Query.anchor_participates
#print axioms Query.compatible_iff_no_fault
#print axioms Query.truth_function_preserves_participation
#print axioms Query.truth_function_preserves_faults
#print axioms Query.constant_false_still_checks_other_operand
#print axioms Query.constant_true_still_checks_other_operand
#print axioms Query.unselected_ite_branch_participates
#print axioms Query.empty_is_a_constructed_value
#print axioms Query.complement_of_empty_is_full
#print axioms Query.contradiction_is_empty
#print axioms Query.excluded_middle_is_full
#print axioms Query.cardinality_exact
#print axioms Query.equal_roster_positions_are_counted_separately
#print axioms Query.cardinality_counts_same_world
#print axioms Query.collect_exact
#print axioms Query.traversal_invariant
#print axioms Query.duplicate_binding_is_idempotent
#print axioms Query.unmatched_operand_is_not_a_fault
#print axioms Query.later_fault_survives_earlier_saturation
#print axioms Query.program_schedule_invariant
#print axioms Query.no_answer_with_participating_fault
#print axioms Query.partial_collection_cannot_claim_completion

end Query
