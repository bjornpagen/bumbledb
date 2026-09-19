import Std

/-!
Finite observable rosters use ordinary labels and Event membership. These laws
cover clipping to explicit evidence, admission, value grouping, shared-world
refinement, pullback and conversion into Boolean readouts. World types below are
already legal; context/owner admission and native Vec/index/capacity behavior
remain separate Rust correspondence obligations. Empty cells retain their labels.
-/
namespace Partitions

abbrev Region (W : Type) := W → Prop
structure Valid {W I : Type} (parent : Region W) (cell : I → Region W) : Prop where
  bounded : ∀ i w, cell i w → parent w
  total : ∀ w, parent w → ∃ i, cell i w
  unique : ∀ i j w, cell i w → cell j w → i = j

def clip {W I : Type} (parent : Region W) (cell : I → Region W) : I → Region W :=
  fun i w => parent w ∧ cell i w

theorem clipped_admission_exact {W I : Type} (parent : Region W) (cell : I → Region W) :
    Valid parent (clip parent cell) ↔
      (∀ w, parent w → ∃ i, cell i w) ∧
      (∀ i j w, parent w → cell i w → cell j w → i = j) := by
  constructor
  · intro valid
    exact ⟨fun w hw => let ⟨i, _, h⟩ := valid.total w hw; ⟨i, h⟩,
      fun i j w hp hi hj => valid.unique i j w ⟨hp, hi⟩ ⟨hp, hj⟩⟩
  · rintro ⟨covered, disjoint⟩
    exact ⟨fun _ _ h => h.1,
      fun w hp => let ⟨i, hi⟩ := covered w hp; ⟨i, hp, hi⟩,
      fun i j w hi hj => disjoint i j w hi.1 hi.2 hj.2⟩

theorem empty_parent_retains_only_empty_cells {W I : Type} (parent : Region W)
    (cell : I → Region W) (valid : Valid parent cell) (empty : ∀ w, ¬ parent w) :
    ∀ i w, ¬ cell i w := by
  exact fun i w present => empty w (valid.bounded i w present)

theorem empty_roster_iff_empty_parent {W : Type} (parent : Region W) :
    Valid parent (fun (i : Empty) => nomatch i) ↔ ∀ w, ¬ parent w := by
  constructor
  · intro valid w hp
    obtain ⟨i, _⟩ := valid.total w hp
    exact nomatch i
  · intro empty
    constructor
    · intro i; exact nomatch i
    · exact fun w hp => (empty w hp).elim
    · intro i; exact nomatch i

def coarsen {W I J : Type} (f : I → J) (cell : I → Region W) : J → Region W :=
  fun j w => ∃ i, f i = j ∧ cell i w

theorem value_grouping_preserves_partition {W I J : Type} (parent : Region W)
    (cell : I → Region W) (valid : Valid parent cell) (f : I → J) :
    Valid parent (coarsen f cell) := by
  constructor
  · rintro j w ⟨i, _, present⟩
    exact valid.bounded i w present
  · intro w hp
    obtain ⟨i, hi⟩ := valid.total w hp
    exact ⟨f i, i, rfl, hi⟩
  · rintro j k w ⟨i, fi, hi⟩ ⟨l, fl, hl⟩
    have same := valid.unique i l w hi hl
    exact fi.symm.trans ((congrArg f same).trans fl)

theorem constant_grouping_is_certain_on_parent {W I J : Type} (parent : Region W)
    (cell : I → Region W) (valid : Valid parent cell) (value : J) (w : W) :
    coarsen (fun _ : I => value) cell value w ↔ parent w := by
  constructor
  · rintro ⟨i, _, hi⟩
    exact valid.bounded i w hi
  · intro hp
    obtain ⟨i, hi⟩ := valid.total w hp
    exact ⟨i, rfl, hi⟩

theorem pullback_preserves_partition {W X I : Type} (parent : Region W)
    (cell : I → Region W) (valid : Valid parent cell) (f : X → W) :
    Valid (fun x => parent (f x)) (fun i x => cell i (f x)) := by
  exact ⟨fun i x => valid.bounded i (f x), fun x => valid.total (f x),
    fun i j x => valid.unique i j (f x)⟩

theorem refinement_preserves_shared_world_partition {W I J : Type}
    (P Q : Region W) (left : I → Region W) (right : J → Region W)
    (lv : Valid P left) (rv : Valid Q right) :
    Valid (fun w => P w ∧ Q w) (fun (pair : I × J) w => left pair.1 w ∧ right pair.2 w) := by
  constructor
  · intro pair w h
    exact ⟨lv.bounded pair.1 w h.1, rv.bounded pair.2 w h.2⟩
  · rintro w ⟨hp, hq⟩
    obtain ⟨i, hi⟩ := lv.total w hp
    obtain ⟨j, hj⟩ := rv.total w hq
    exact ⟨(i, j), hi, hj⟩
  · intro a b w ha hb
    exact Prod.ext (lv.unique a.1 b.1 w ha.1 hb.1) (rv.unique a.2 b.2 w ha.2 hb.2)

def bitReadout {W I B : Type} (cell : I → Region W) (code : I → B → Bool) (bit : B) : Region W :=
  fun w => ∃ i, cell i w ∧ code i bit = true

theorem readout_bits_exact {W I B : Type} (parent : Region W) (cell : I → Region W)
    (valid : Valid parent cell) (code : I → B → Bool) (i : I) (w : W) (present : cell i w)
    (bit : B) : bitReadout cell code bit w ↔ code i bit = true := by
  constructor
  · rintro ⟨j, hj, value⟩
    have same := valid.unique j i w hj present
    exact same ▸ value
  · exact fun value => ⟨i, present, value⟩

theorem full_partition_readout_is_total_and_legal {W I B : Type}
    (cell : I → Region W) (valid : Valid (fun _ => True) cell)
    (code : I → B → Bool) (legal : (B → Bool) → Prop)
    (admitted : ∀ i, legal (code i)) :
    ∀ w, ∃ bits, legal bits ∧ ∀ bit, (bitReadout cell code bit w ↔ bits bit = true) := by
  intro w
  obtain ⟨i, hi⟩ := valid.total w True.intro
  exact ⟨code i, admitted i, fun bit => readout_bits_exact _ _ valid code i w hi bit⟩

theorem collapsing_image_can_overlap :
    Valid (fun _ : Bool => True) (fun i w : Bool => i = w) ∧
      (∃ w : Bool, w = false ∧ (fun _ : Bool => ()) w = ()) ∧
      (∃ w : Bool, w = true ∧ (fun _ : Bool => ()) w = ()) := by
  exact ⟨⟨fun _ _ _ => True.intro, fun w _ => ⟨w, rfl⟩,
    fun _ _ _ hi hj => hi.trans hj.symm⟩,
    ⟨false, rfl, rfl⟩, ⟨true, rfl, rfl⟩⟩

/-! Count buckets are indexed by roster positions. The reference recurrence is
the native descending ITE update on the previously processed prefix. Aliasing
two Event values never removes either roster position. -/

def count {W : Type} (events : List (W → Bool)) (w : W) : Nat :=
  events.countP (fun e => e w)

def bucket {W : Type} : List (W → Bool) → Nat → W → Bool
  | [], k, _ => decide (k = 0)
  | e :: es, k, w => if e w then match k with
      | 0 => false
      | k + 1 => bucket es k w
    else bucket es k w

theorem bucket_exact {W : Type} (events : List (W → Bool)) (k : Nat) (w : W) :
    bucket events k w = true ↔ count events w = k := by
  induction events generalizing k with
  | nil => simp [bucket, count, eq_comm]
  | cons e es ih =>
    cases he : e w
    · simpa [bucket, count, List.countP_cons, he] using ih k
    · cases k with
      | zero => simp [bucket, count, he]
      | succ k =>
        simp only [bucket, he, ↓reduceIte]
        rw [ih k]
        simp only [count, List.countP_cons, he, ↓reduceIte]
        omega

theorem count_roster_forms_full_partition {W : Type} (events : List (W → Bool)) :
    Valid (fun _ => True)
      (fun (k : Fin (events.length + 1)) w => bucket events k.val w = true) := by
  constructor
  · exact fun _ _ _ => True.intro
  · intro w _
    have bounded := List.countP_le_length (p := fun e => e w) (l := events)
    let k : Fin (events.length + 1) := ⟨count events w, by dsimp [count]; omega⟩
    exact ⟨k, (bucket_exact events k.val w).mpr rfl⟩
  · intro i j w hi hj
    exact Fin.ext (((bucket_exact events i.val w).mp hi).symm.trans
      ((bucket_exact events j.val w).mp hj))

theorem duplicate_positions_count_twice {W : Type} (events : List (W → Bool))
    (e : W → Bool) (w : W) :
    count (e :: e :: events) w = count events w + if e w then 2 else 0 := by
  cases h : e w <;> simp [count, h, Nat.add_assoc]

#print axioms Partitions.clipped_admission_exact
#print axioms Partitions.empty_parent_retains_only_empty_cells
#print axioms Partitions.empty_roster_iff_empty_parent
#print axioms Partitions.value_grouping_preserves_partition
#print axioms Partitions.constant_grouping_is_certain_on_parent
#print axioms Partitions.pullback_preserves_partition
#print axioms Partitions.refinement_preserves_shared_world_partition
#print axioms Partitions.readout_bits_exact
#print axioms Partitions.full_partition_readout_is_total_and_legal
#print axioms Partitions.collapsing_image_can_overlap
#print axioms Partitions.bucket_exact
#print axioms Partitions.count_roster_forms_full_partition
#print axioms Partitions.duplicate_positions_count_twice

end Partitions
