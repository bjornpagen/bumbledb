import Bumbledb.Dependencies

/-!
# Subsumption — the extension form against the original vocabulary

The extension form (cardinality windows) EXTENDS the statement
grammar; nothing in it contradicts it. This module is that claim,
machine-checked: each theorem spends an extension judgment against an
original one.

* **A floored window implies the reverse containment**
  (`window_floor_containment`): `A(X | φ) in n..m per B(Y | ψ)` with
  `n ≥ 1` yields `B(Y | ψ) <= A(X | φ)` — the window's floor IS an
  existence obligation, so the extension strictly generalizes what
  the original vocabulary already says.
* **Keyed `==` is the `1..1` window** (`keyed_eq_unit_window`,
  `unit_window_containsEq`): forward, key-backed equality forces the
  unit window; backward, the unit window plus the forward containment
  reconstructs bare `==`. The key premises stay ACCEPTANCE-side,
  exactly the acceptance ≠ denotation discipline — the reconstruction
  returns `ContainsEq`, and upgrading it to `KeyBackedEquality` costs
  exactly the two key premises acceptance resolves
  (`TargetKeyAccepted`, each direction), never a new judgment.

The extension form this module reads — the cardinality window — is
ACCEPTED by the engine at declaration (2026-07-14:
`StatementDescriptor::Cardinality`, `crates/bumbledb-theory/src/schema.rs`;
the gate arm in `schema/validate.rs` implements the acceptance rules
named above) and JUDGED per commit
(`storage/commit/judgment.rs::check_windows`). The discharge record
lives in `Cardinality.lean`'s module doc. The sharing this module
licenses is spent conservatively: a floored window MAY share the
containment's probe machinery — window edges are written exactly as
containment edges — but the engine never skips a declared window's
check (`window_floor_containment` is subsumption, not an enforcement
shortcut).
-/

namespace Bumbledb

/-! ## Windows against containment -/

/-- **A floored window implies the reverse containment.** With
`1 ≤ w.lo`, every selected parent's child group is inhabited, and any
inhabitant is exactly the containment witness — the extension
subsumes, never contradicts, the original vocabulary. -/
theorem window_floor_containment {A : Set Fact} {φ : Selection}
    {X : List FieldId} {w : Window} {B : Set Fact} {ψ : Selection}
    {Y : List FieldId} (hlo : 1 ≤ w.lo)
    (h : CardinalityWindow A φ X w B ψ Y) :
    Containment B ψ Y A φ X := by
  intro g hg hψ
  obtain ⟨l, hnd, hsub, hlen⟩ := (h g hg hψ).1
  cases l with
  | nil => exact absurd (Nat.le_trans hlo hlen) (Nat.not_succ_le_zero 0)
  | cons a l' =>
    have ha := hsub a (List.mem_cons_self)
    exact ⟨a, ha.1, ha.2.1, ha.2.2⟩

/-- **Keyed `==` forces the unit window.** Under key-backed equality,
every selected target fact's child group counts exactly one: the
backward containment supplies the floor witness, and the source key
collapses any two members — `==` is the `1..1` window, said in window
vocabulary. -/
theorem keyed_eq_unit_window {A : Set Fact} {φ : Selection}
    {X : List FieldId} {B : Set Fact} {ψ : Selection}
    {Y : List FieldId} (h : KeyBackedEquality A φ X B ψ Y) :
    CardinalityWindow A φ X (Window.mk 1 (some 1)) B ψ Y := by
  intro g hg hψ
  obtain ⟨f, hfA, hfφ, hfproj⟩ := h.eq.backward g hg hψ
  constructor
  · exact ⟨[f],
      List.Pairwise.cons (fun x hx => nomatch hx) List.Pairwise.nil,
      fun a ha => by
        rw [List.mem_singleton] at ha
        rw [ha]
        exact ⟨hfA, hfφ, hfproj⟩,
      Nat.le_refl 1⟩
  · intro m hm
    injection hm with hm
    intro l hnd hsub
    cases l with
    | nil => exact hm ▸ Nat.zero_le 1
    | cons a l' =>
      cases l' with
      | nil => exact hm ▸ Nat.le_refl 1
      | cons b l'' =>
        have ha := hsub a (List.mem_cons_self)
        have hb := hsub b
          (List.mem_cons_of_mem a (List.mem_cons_self))
        have hab : a = b := h.source_key a b ⟨ha.1, ha.2.1⟩
          ⟨hb.1, hb.2.1⟩ (ha.2.2.trans hb.2.2.symm)
        cases hnd with
        | cons hne _ =>
          exact absurd hab (hne b (List.mem_cons_self))

/-- **The unit window reconstructs bare `==`.** The `1..1` window
plus the forward containment give both containment directions — the
backward half is `window_floor_containment` at floor 1. Key premises
are deliberately NOT reconstructed here: they are acceptance's
business (`TargetKeyAccepted`, each direction independently), exactly
as for the `==` lowering itself. -/
theorem unit_window_containsEq {A : Set Fact} {φ : Selection}
    {X : List FieldId} {B : Set Fact} {ψ : Selection}
    {Y : List FieldId}
    (hwin : CardinalityWindow A φ X (Window.mk 1 (some 1)) B ψ Y)
    (hfwd : Containment A φ X B ψ Y) :
    ContainsEq A φ X B ψ Y :=
  ⟨hfwd, window_floor_containment (Nat.le_refl 1) hwin⟩

end Bumbledb
