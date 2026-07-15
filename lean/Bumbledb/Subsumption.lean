import Bumbledb.Dependencies

/-!
# Subsumption — the extension forms against the original vocabulary

The extension forms (cardinality windows, order marks) EXTEND the
statement grammar; nothing in them contradicts it. This module is
that claim, machine-checked: each theorem spends an extension
judgment against an original one.

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
* **An order mark keys each group on its position**
  (`order_group_functionality`): the per-group position uniqueness is
  semantic `Functionality` of the group over the position field —
  the extension's uniqueness half is the original key judgment on a
  restricted fact set.
* **Key-backed chains rank deterministically**
  (`chain_eval_deterministic`, `rank_of_deterministic`): the form's
  acceptance rule demands every `by` hop be key-backed (each step an
  already-declared inclusion against a key); spending those key
  premises makes the relational chain evaluation a function — the
  ranked judgment never reads an ambiguous rank on an accepted,
  holding instance. Acceptance enters as HYPOTHESES throughout, never
  as a conjunct of any denotation.

Both extension forms this module reads — cardinality windows and
order marks — are ACCEPTED by the engine at declaration
(2026-07-14: `StatementDescriptor::Cardinality` / `::Order`,
`crates/bumbledb/src/schema.rs`; the gate arms in
`schema/validate.rs` implement the acceptance rules named above,
the key-backed-hop rule included) and, for WRITABLE subjects, JUDGED
per commit (`storage/commit/judgment.rs::check_windows` /
`::check_orders`); a CLOSED subject's order mark is plain-only — the
engine gate-refuses the ranked form there
(`SchemaError::RankedOrderClosedSubject`, the sound narrowing
recorded in `Order.lean` § narrowings) and decides the plain
discipline at validate against the sealed extension. The discharge
records live in `Cardinality.lean`'s and `Order.lean`'s module docs. The sharing this module licenses is spent conservatively:
a floored window MAY share the containment's probe machinery — window
edges are written exactly as containment edges — but the engine never
skips a declared window's check (`window_floor_containment` is
subsumption, not an enforcement shortcut).
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

/-! ## Order marks against functionality -/

/-- **An order mark keys each group on its position.** Per-group
position uniqueness IS semantic functionality of the group over the
position field: value agreement implies ordinal agreement, and the
ordinal discipline collapses the facts — the extension's uniqueness
half is the original key judgment on a restricted fact set. -/
theorem order_group_functionality {R : Set Fact} {pos : FieldId}
    {G : List FieldId} (h : OrderMark R pos G) (t : List Value) :
    Functionality (GroupOf R G t) [pos] := by
  intro f g hf hg hproj
  have hv : f pos = g pos :=
    (Fact.project_eq_iff f g [pos]).mp hproj pos
      (List.mem_singleton.mpr rfl)
  exact (h t).unique f g hf hg (congrArg Value.ordinal hv)

/-! ## Rank chains against the key discipline -/

/-- **Key-backed hops evaluate deterministically.** With every hop's
relation keyed on the hop's key field (the acceptance gate's demand —
each `by` step an already-declared inclusion against a key), the
relational chain evaluation is a function of its input value. -/
theorem chain_eval_deterministic {T : Theory} {I : Instance} :
    ∀ (hops : List RankHop),
      (∀ hop, hop ∈ hops →
        Functionality (T.den I hop.relation) [hop.key]) →
      ∀ v w w', chainEval T I hops v w → chainEval T I hops v w' →
        w = w'
  | [], _, v, w, w', h, h' => by
    have h1 : v = w := h
    have h2 : v = w' := h'
    exact h1.symm.trans h2
  | hop :: rest, hkeys, v, w, w', h, h' => by
    obtain ⟨u, ⟨g, hgden, hgkey, hgread⟩, hrest⟩ := h
    obtain ⟨u', ⟨g', hgden', hgkey', hgread'⟩, hrest'⟩ := h'
    have hproj : g.project [hop.key] = g'.project [hop.key] :=
      (Fact.project_eq_iff g g' [hop.key]).mpr fun j hj => by
        rw [List.mem_singleton] at hj
        rw [hj, hgkey, hgkey']
    have hgg : g = g' :=
      hkeys hop (List.mem_cons_self) g g' hgden hgden' hproj
    have huu : u = u' := by
      rw [← hgread, ← hgread', hgg]
    exact chain_eval_deterministic rest
      (fun h hh => hkeys h (List.mem_cons_of_mem hop hh)) u w w'
      hrest (huu.symm ▸ hrest')

/-- **Ranks are single-valued on key-backed chains.** The ranked
order mark's monotonicity clause quantifies relationally over ranks;
this is the premise-spending theorem that the quantification is
never ambiguous on an accepted instance. -/
theorem rank_of_deterministic {T : Theory} {I : Instance}
    {c : RankChain}
    (hkeys : ∀ hop, hop ∈ c.hops →
      Functionality (T.den I hop.relation) [hop.key])
    {f : Fact} {r r' : Nat} (h : c.rankOf T I f r)
    (h' : c.rankOf T I f r') : r = r' := by
  obtain ⟨w, hw, hr⟩ := h
  obtain ⟨w', hw', hr'⟩ := h'
  have hww := chain_eval_deterministic c.hops hkeys (f c.link) w w'
    hw hw'
  rw [← hr, ← hr', hww]

end Bumbledb
