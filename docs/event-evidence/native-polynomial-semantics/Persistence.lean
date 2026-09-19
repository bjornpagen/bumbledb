import Std

/-!
The semantic identity required of the BEVT codec and a resolved query namespace.
This models source identity, original support and supported membership. It does
not verify the Rust encoder, graph reduction, byte parser, mutexes or allocator.
In particular, `Registry.roundtrip` is an implementation obligation, not a
theorem about the Rust Registry merely because the names agree.
-/
namespace Persistence

structure Meaning (Source World : Type) where
  source : Source
  support : World → Bool
  membership : World → Bool

def meaning {S W : Type} (source : S) (support event : W → Bool) : Meaning S W :=
  ⟨source, support, fun w => support w && event w⟩

theorem supported_identity {W : Type} (H A B : W → Bool) :
    (fun w => H w && A w) = (fun w => H w && B w) ↔
      (∀ w, H w = true → A w = B w) := by
  constructor
  · intro equal w legal
    simpa only [legal, Bool.true_and] using congrFun equal w
  · intro equal
    funext w
    cases h : H w with
    | false => rfl
    | true => exact equal w h

theorem exact_identity {S W : Type} (s t : S) (H K A B : W → Bool) :
    meaning s H A = meaning t K B ↔
      s = t ∧ H = K ∧ (∀ w, H w = true → A w = B w) := by
  constructor
  · intro equal
    have source := congrArg Meaning.source equal
    have support := congrArg Meaning.support equal
    have membership := congrArg Meaning.membership equal
    change s = t at source
    change H = K at support
    change (fun w => H w && A w) = (fun w => K w && B w) at membership
    rw [← support] at membership
    exact ⟨source, support, (supported_identity H A B).mp membership⟩
  · rintro ⟨rfl, rfl, equal⟩
    have membership := (supported_identity H A B).mpr equal
    unfold meaning
    rw [membership]

theorem legal_decoder_erased {S W : Type} (s : S) (H A : W → Bool)
    (decode : W → W) (fixes : ∀ w, H w = true → decode w = w) :
    meaning s H (fun w => A (decode w)) = meaning s H A := by
  apply (exact_identity s s H H _ A).mpr
  exact ⟨rfl, rfl, fun w legal => congrArg A (fixes w legal)⟩

theorem arbitrary_completions_agree {S W : Type} (s : S) (H A B : W → Bool)
    (legal : ∀ w, H w = true → A w = B w) :
    meaning s H A = meaning s H B :=
  (exact_identity s s H H A B).mpr ⟨rfl, rfl, legal⟩

theorem complement_is_relative {S W : Type} (s : S) (H A : W → Bool) (w : W) :
    (meaning s H (fun v => !(A v))).membership w =
      (H w && !(meaning s H A).membership w) := by
  simp only [meaning]
  cases H w <;> cases A w <;> rfl

theorem empty_retains_source {S W : Type} (s t : S) (H K : W → Bool)
    (different : s ≠ t) :
    meaning s H (fun _ => false) ≠ meaning t K (fun _ => false) := by
  intro equal
  exact different ((exact_identity s t H K _ _).mp equal).1

theorem empty_retains_support {S W : Type} (s : S) (H K : W → Bool)
    (different : H ≠ K) :
    meaning s H (fun _ => false) ≠ meaning s K (fun _ => false) := by
  intro equal
  exact different ((exact_identity s s H K _ _).mp equal).2.1

theorem inhabited_space_distinguishes_constants {S W : Type} (s : S)
    (H : W → Bool) (w : W) (legal : H w = true) :
    meaning s H (fun _ => false) ≠ meaning s H (fun _ => true) := by
  intro equal
  have contradiction := ((exact_identity s s H H _ _).mp equal).2.2 w legal
  cases contradiction

/-- Equal counts under the same uniform law do not identify regions. -/
theorem equal_mass_distinct_events :
    ((false : Bool).toNat + true.toNat = true.toNat + false.toNat) ∧
      meaning () (fun _ : Bool => true) (fun w => w) ≠
      meaning () (fun _ : Bool => true) (fun w => !w) := by
  constructor
  · rfl
  · intro equal
    have contradiction := ((exact_identity () () _ _ _ _).mp equal).2.2 false rfl
    cases contradiction

structure Registry (S W K : Type) where
  admitted : Meaning S W → Prop
  key : (value : Meaning S W) → admitted value → K
  resolve : K → Option (Meaning S W)
  roundtrip : ∀ value (accepted : admitted value), resolve (key value accepted) = some value

/-- Within one retained namespace, registered key equality is semantic equality.
    The theorem needs canonical input meanings and the roundtrip invariant. -/
theorem registered_key_identity {S W K : Type} (r : Registry S W K)
    (a b : Meaning S W) (ha : r.admitted a) (hb : r.admitted b) :
    r.key a ha = r.key b hb ↔ a = b := by
  constructor
  · intro equal
    have resolved := congrArg r.resolve equal
    rw [r.roundtrip a ha, r.roundtrip b hb] at resolved
    exact Option.some.inj resolved
  · intro equal
    cases equal
    rfl

end Persistence

#print axioms Persistence.supported_identity
#print axioms Persistence.exact_identity
#print axioms Persistence.legal_decoder_erased
#print axioms Persistence.arbitrary_completions_agree
#print axioms Persistence.complement_is_relative
#print axioms Persistence.empty_retains_source
#print axioms Persistence.empty_retains_support
#print axioms Persistence.inhabited_space_distinguishes_constants
#print axioms Persistence.equal_mass_distinct_events
#print axioms Persistence.registered_key_identity
