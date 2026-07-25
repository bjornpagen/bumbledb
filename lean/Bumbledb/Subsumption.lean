import Bumbledb.Dependencies

/-!
# Subsumption — the extension form against the original vocabulary

The extension form (the capacity statement) EXTENDS the statement
grammar; nothing in it contradicts it. This module is that claim,
machine-checked: each theorem spends an extension judgment against an
original one.

* **A floored capacity law implies the reverse containment**
  (`window_floor_containment`): `B(Y | ψ) <=[w]{n..m} A(X | φ)` with
  `n ≥ 1` yields `B(Y | ψ) <= A(X | φ)` — a positive measure floor
  inhabits the group UNDER ANY WEIGHT (`natSum [] = 0`), so the
  extension strictly generalizes what the original vocabulary already
  says. The floor half generalizes to every weight; the `{1..*}` BAN
  fires on the Count instance only, because only there is the floor
  equivalent to containment — a weight-0 row satisfies containment
  but not a sum floor (the per-aggregate ban law,
  `docs/design/capacity-laws.md` § 6).
* **Keyed `==` is the `{1}` window at unit weight**
  (`keyed_eq_unit_window`, `unit_window_containsEq`): forward,
  key-backed equality forces the unit-weight point window; backward,
  that window plus the forward containment reconstructs bare `==`.
  The key premises stay ACCEPTANCE-side, exactly the acceptance ≠
  denotation discipline — the reconstruction returns `ContainsEq`,
  and upgrading it to `KeyBackedEquality` costs exactly the two key
  premises acceptance resolves (`TargetKeyAccepted`, each direction),
  never a new judgment.

The extension form this module reads — the capacity statement — is
ACCEPTED by the engine at declaration
(`StatementDescriptor::Capacity`, `crates/bumbledb-theory/src/schema.rs`;
the gate arm in `schema/validate.rs` implements the acceptance rules
named above) and JUDGED per commit
(`storage/commit/judgment.rs::check_capacity`). The discharge record
lives in `Capacity.lean`'s module doc. The sharing this module
licenses is spent conservatively: a floored statement MAY share the
containment's probe machinery — capacity edges are written exactly as
containment edges — but the engine never skips a declared statement's
check (`window_floor_containment` is subsumption, not an enforcement
shortcut).
-/

namespace Bumbledb

/-! ## Capacity laws against containment -/

/-- **A floored capacity law implies the reverse containment.** With
`1 ≤ w.lo` — under ANY weight — every selected parent's child group
carries a positive measure witness, whose list is nonempty
(`natSum [] = 0`), and any inhabitant is exactly the containment
witness: the extension subsumes, never contradicts, the original
vocabulary. -/
theorem window_floor_containment {A : Set Fact} {φ : Selection}
    {X : List FieldId} {wt : Weight} {w : CapWindow} {B : Set Fact}
    {ψ : Selection} {Y : List FieldId} (hlo : 1 ≤ w.lo)
    (h : CapacityLaw A φ X wt w B ψ Y) :
    Containment B ψ Y A φ X := by
  intro g hg hψ
  obtain ⟨l, hnd, hsub, hlen⟩ := (h g hg hψ).1
  cases l with
  | nil => exact absurd (Nat.le_trans hlo hlen) (Nat.not_succ_le_zero 0)
  | cons a l' =>
    have ha := hsub a (List.mem_cons_self)
    exact ⟨a, ha.1, ha.2.1, ha.2.2⟩

/-- **Keyed `==` forces the unit-weight point window.** Under
key-backed equality, every selected target fact's child group
measures exactly one at unit weight: the backward containment
supplies the floor witness, and the source key collapses any two
members — `==` is the `{1}` window, said in capacity vocabulary. -/
theorem keyed_eq_unit_window {A : Set Fact} {φ : Selection}
    {X : List FieldId} {B : Set Fact} {ψ : Selection}
    {Y : List FieldId} (h : KeyBackedEquality A φ X B ψ Y) :
    CapacityLaw A φ X .unit ⟨1, some (.lit 1)⟩ B ψ Y := by
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
    subst hm
    refine (Set.measureAtMost_unit_iff _ _).mpr ?_
    exact Set.atMost_one_of_subsingleton fun a b ha hb =>
      h.source_key a b ⟨ha.1, ha.2.1⟩ ⟨hb.1, hb.2.1⟩
        (ha.2.2.trans hb.2.2.symm)

/-- **The unit-weight point window reconstructs bare `==`.** The
`{1}` window plus the forward containment give both containment
directions — the backward half is `window_floor_containment` at
floor 1. Key premises are deliberately NOT reconstructed here: they
are acceptance's business (`TargetKeyAccepted`, each direction
independently), exactly as for the `==` lowering itself. -/
theorem unit_window_containsEq {A : Set Fact} {φ : Selection}
    {X : List FieldId} {B : Set Fact} {ψ : Selection}
    {Y : List FieldId}
    (hwin : CapacityLaw A φ X .unit ⟨1, some (.lit 1)⟩ B ψ Y)
    (hfwd : Containment A φ X B ψ Y) :
    ContainsEq A φ X B ψ Y :=
  ⟨hfwd, window_floor_containment (Nat.le_refl 1) hwin⟩

end Bumbledb
