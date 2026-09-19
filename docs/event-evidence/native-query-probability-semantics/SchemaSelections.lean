import Std

/-!
Schema selections compare complete Event values. Context includes the named
coordinate signature; Law includes structural absence or the exact declared law.
The membership below is support-normalized. Encoding faithfulness is an explicit
implementation obligation, not a proof about Rust, BEVT or resident handles.
-/
namespace SchemaSelections

structure Value (Context Law World : Type) where
  context : Context
  support : World → Bool
  law : Law
  membership : World → Bool

def event {C L W : Type} (c : C) (h : W → Bool) (l : L) (a : W → Bool) : Value C L W :=
  ⟨c, h, l, fun w => h w && a w⟩

def selects {V : Type} (literals : List V) (actual : V) : Prop := actual ∈ literals

theorem singleton_exact {V : Type} (a b : V) : selects [b] a ↔ a = b := by
  simp [selects]

theorem alternatives_union {V : Type} (as bs : List V) (v : V) :
    selects (as ++ bs) v ↔ selects as v ∨ selects bs v := by
  simp [selects]

theorem order_irrelevant {V : Type} (as bs : List V) (same : ∀ v, v ∈ as ↔ v ∈ bs) (v : V) :
    selects as v ↔ selects bs v := same v

theorem duplicate_semantics {V : Type} (a v : V) (as : List V) :
    selects (a :: a :: as) v ↔ selects (a :: as) v := by
  simp [selects]

theorem exact_context {C L W : Type} (a b : Value C L W) (selected : selects [b] a) :
    a.context = b.context := congrArg Value.context ((singleton_exact a b).mp selected)

theorem exact_support {C L W : Type} (a b : Value C L W) (selected : selects [b] a) :
    a.support = b.support := congrArg Value.support ((singleton_exact a b).mp selected)

theorem exact_law {C L W : Type} (a b : Value C L W) (selected : selects [b] a) :
    a.law = b.law := congrArg Value.law ((singleton_exact a b).mp selected)

theorem empty_still_selects {C L W : Type} (c : C) (h : W → Bool) (l : L) :
    selects [event c h l (fun _ => false)] (event c h l (fun _ => false)) := by
  simp [selects]

theorem mass_not_used {V P : Type} (a : V) (mass : V → P) (zero : P)
    (_zeroMass : mass a = zero) : selects [a] a := by
  simp [selects]

theorem overlap_is_not_selection :
    let a := event () (fun _ : Bool => true) () id
    let full := event () (fun _ : Bool => true) () (fun _ => true)
    (∃ w, a.membership w = true ∧ full.membership w = true) ∧ ¬ selects [full] a := by
  constructor
  · exact ⟨true, rfl, rfl⟩
  · intro selected
    have equal := congrArg (fun v => v.membership false) ((singleton_exact _ _).mp selected)
    cases equal

/-- Owner-local representations may differ; their canonical encoding must agree
    exactly when their denotations do. Errors are outside this successful codec
    contract and may not be represented as false membership. -/
theorem portable_selection {R V B : Type} (meaning : R → V) (encode : R → B)
    (faithful : ∀ a b, encode a = encode b ↔ meaning a = meaning b)
    (literals : List R) (actual : R) :
    selects (literals.map encode) (encode actual) ↔
      selects (literals.map meaning) (meaning actual) := by
  simp only [selects, List.mem_map]
  constructor
  · rintro ⟨literal, member, equal⟩
    exact ⟨literal, member, (faithful literal actual).mp equal⟩
  · rintro ⟨literal, member, equal⟩
    exact ⟨literal, member, (faithful literal actual).mpr equal⟩

/-- Selecting a fact keeps its full projected value; it does not intersect that
    value with the selection literal. -/
theorem selection_preserves_projection {Row V P : Type}
    (field : Row → V) (project : Row → P) (literals : List V) (row : Row)
    (selected : selects literals (field row)) :
    (∃ kept, kept = row ∧ selects literals (field kept) ∧ project kept = project row) :=
  ⟨row, rfl, selected, rfl⟩

end SchemaSelections

#print axioms SchemaSelections.singleton_exact
#print axioms SchemaSelections.alternatives_union
#print axioms SchemaSelections.order_irrelevant
#print axioms SchemaSelections.duplicate_semantics
#print axioms SchemaSelections.exact_context
#print axioms SchemaSelections.exact_support
#print axioms SchemaSelections.exact_law
#print axioms SchemaSelections.empty_still_selects
#print axioms SchemaSelections.mass_not_used
#print axioms SchemaSelections.overlap_is_not_selection
#print axioms SchemaSelections.portable_selection
#print axioms SchemaSelections.selection_preserves_projection
