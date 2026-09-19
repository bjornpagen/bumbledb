import Std

/-!
Shared-parameter source transport. Values are evaluated at one actual parameter;
parameter-function holes are not zero. A total family uses a zero default only
where no cell is active. Decoding must preserve cells, active partial coefficients,
parameter identity, complete named posterior contexts and all indexed receipt
regions. Replay, rather than a wire tag, establishes normalization and validity.

These are denotations under explicit faithful-decoding and replay premises. They
do not verify Rust constructors, exact solvers, graph alignment, BESC/BEVT/BEGF
codecs, arithmetic budgets, allocation, cancellation or the database compiler.
-/
namespace FamilyTransport

/-- A missing active coefficient propagates undefinedness. Missing inactive
coefficients do not matter; only absence of an active cell supplies zero. Native
family admission additionally establishes disjoint cells and active totality. -/
def value {C Θ W : Type} : List C → (C → Θ → W → Bool) →
    (C → Θ → Option Rat) → Θ → W → Option Rat
  | [], _, _, _, _ => some 0
  | c :: cs, region, coefficient, theta, world => do
      let head ← if region c theta world then coefficient c theta else some 0
      let tail ← value cs region coefficient theta world
      pure (head + tail)

theorem value_reconstruction {C Θ W : Type} (cs : List C)
    (region decodedRegion : C → Θ → W → Bool)
    (coefficient decodedCoefficient : C → Θ → Option Rat) (theta : Θ) (world : W)
    (sameRegion : ∀ c, c ∈ cs → region c theta world = decodedRegion c theta world)
    (sameActive : ∀ c, c ∈ cs → region c theta world = true →
      coefficient c theta = decodedCoefficient c theta) :
    value cs region coefficient theta world =
      value cs decodedRegion decodedCoefficient theta world := by
  induction cs with
  | nil => rfl
  | cons c cs ih =>
    have tail := ih (fun d hd => sameRegion d (by simp [hd]))
      (fun d hd => sameActive d (by simp [hd]))
    cases h : region c theta world
    · simp [value, tail, ← sameRegion c (by simp), h]
    · simp [value, tail, ← sameRegion c (by simp), h, sameActive c (by simp) h]

theorem active_totality_gives_total_value {C Θ W : Type} (cs : List C)
    (region : C → Θ → W → Bool) (coefficient : C → Θ → Option Rat)
    (theta : Θ) (world : W)
    (total : ∀ c, c ∈ cs → region c theta world = true →
      ∃ q, coefficient c theta = some q) :
    ∃ q, value cs region coefficient theta world = some q := by
  induction cs with
  | nil => exact ⟨0,rfl⟩
  | cons c cs ih =>
    obtain ⟨tail, ht⟩ := ih (fun d hd => total d (by simp [hd]))
    cases h : region c theta world
    · exact ⟨tail, by simp [value, h, ht, Rat.zero_add]⟩
    · obtain ⟨head, hh⟩ := total c (by simp) h
      exact ⟨head+tail, by simp [value, h, ht, hh]⟩

theorem zero_default {C Θ W : Type} (cs : List C)
    (region : C → Θ → W → Bool) (coefficient : C → Θ → Option Rat)
    (theta : Θ) (world : W)
    (absent : ∀ c, c ∈ cs → region c theta world = false) :
    value cs region coefficient theta world = some 0 := by
  induction cs with
  | nil => rfl
  | cons c cs ih =>
    simp [value, absent c (by simp), ih (fun d hd => absent d (by simp [hd])), Rat.zero_add]

theorem active_hole_is_not_zero :
    value [()] (fun _ _ _ => true) (fun _ _ => none) () () = none := rfl

theorem inactive_hole_has_zero_default :
    value [()] (fun _ _ _ => false) (fun _ _ => none) () () = some 0 := by decide +kernel

theorem parameter_hole_is_not_zero : (none : Option Rat) ≠ some 0 := by decide

def sum {X : Type} : List X → (X → Rat) → Rat
  | [], _ => 0
  | x :: xs, f => f x + sum xs f

theorem sum_congr {X : Type} (xs : List X) (f g : X → Rat)
    (same : ∀ x, x ∈ xs → f x = g x) : sum xs f = sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    simp only [sum]
    rw [same x (by simp), ih (fun y hy => same y (by simp [hy]))]

def mapWorld {Θ X Y : Type} (parent : Θ → X → Y) (world : Θ × X) : Θ × Y :=
  (world.1, parent world.1 world.2)

theorem map_keeps_actual_parameter {Θ X Y : Type} (parent : Θ → X → Y)
    (world : Θ × X) : (mapWorld parent world).1 = world.1 := rfl

def row {Θ X Y : Type} [DecidableEq Y] (xs : List X) (parent : Θ → X → Y)
    (density : Θ → X → Rat) (theta : Θ) (y : Y) : Rat :=
  sum xs (fun x => if parent theta x = y then density theta x else 0)

theorem row_reconstruction {Θ X Y : Type} [DecidableEq Y] (xs : List X)
    (parent decodedParent : Θ → X → Y) (density decodedDensity : Θ → X → Rat)
    (theta : Θ) (y : Y)
    (sameParent : ∀ x, x ∈ xs → parent theta x = decodedParent theta x)
    (sameDensity : ∀ x, x ∈ xs → density theta x = decodedDensity theta x) :
    row xs parent density theta y = row xs decodedParent decodedDensity theta y := by
  apply sum_congr
  intro x hx
  rw [sameParent x hx, sameDensity x hx]

theorem normalization_reconstructed {Θ X Y : Type} [DecidableEq Y] (xs : List X)
    (parent decodedParent : Θ → X → Y) (density decodedDensity : Θ → X → Rat)
    (sameParent : ∀ t x, x ∈ xs → parent t x = decodedParent t x)
    (sameDensity : ∀ t x, x ∈ xs → density t x = decodedDensity t x)
    (normalized : ∀ t y, row xs parent density t y = 1) :
    ∀ t y, row xs decodedParent decodedDensity t y = 1 := by
  intro t y
  rw [← row_reconstruction xs parent decodedParent density decodedDensity t y
    (sameParent t) (sameDensity t)]
  exact normalized t y

/-- Each position owns its own unsupported parameter region. -/
def unsupported {Θ : Type} (old target : Θ → Rat) (theta : Θ) : Prop :=
  old theta = 0 ∧ 0 < target theta

def valid {Θ : Type} (domain : Θ → Prop) (cells : List Nat)
    (old target : Nat → Θ → Rat) (theta : Θ) : Prop :=
  domain theta ∧ ∀ i, i ∈ cells → ¬unsupported (old i) (target i) theta

theorem indexed_unsupported_reconstructed {Θ : Type}
    (old decodedOld target decodedTarget : Nat → Θ → Rat) (theta : Θ) (i : Nat)
    (sameOld : old i theta = decodedOld i theta)
    (sameTarget : target i theta = decodedTarget i theta) :
    unsupported (old i) (target i) theta ↔
      unsupported (decodedOld i) (decodedTarget i) theta := by
  simp only [unsupported, sameOld, sameTarget]

theorem valid_domain_reconstructed {Θ : Type} (domain decodedDomain : Θ → Prop)
    (cells : List Nat) (old decodedOld target decodedTarget : Nat → Θ → Rat) (theta : Θ)
    (sameDomain : domain theta ↔ decodedDomain theta)
    (sameOld : ∀ i, i ∈ cells → old i theta = decodedOld i theta)
    (sameTarget : ∀ i, i ∈ cells → target i theta = decodedTarget i theta) :
    valid domain cells old target theta ↔ valid decodedDomain cells decodedOld decodedTarget theta := by
  unfold valid
  constructor
  · rintro ⟨hd, hv⟩
    exact ⟨sameDomain.mp hd, fun i hi h => hv i hi
      ((indexed_unsupported_reconstructed old decodedOld target decodedTarget theta i
        (sameOld i hi) (sameTarget i hi)).mpr h)⟩
  · rintro ⟨hd, hv⟩
    exact ⟨sameDomain.mpr hd, fun i hi h => hv i hi
      ((indexed_unsupported_reconstructed old decodedOld target decodedTarget theta i
        (sameOld i hi) (sameTarget i hi)).mp h)⟩

theorem zero_target_never_unsupported {Θ : Type} (old target : Θ → Rat) (theta : Θ)
    (zero : target theta = 0) : ¬unsupported old target theta := by
  simp [unsupported,zero]

/-- Context includes the named original support, law definition and refinement;
it is not just the numerical posterior probability of one observed Event. -/
structure Receipt (Id Θ Context : Type) where
  identity : Id
  defined : Θ → Prop
  values : Nat → Θ → Option Rat
  unsupported : Nat → Θ → Prop
  outcome : Option Context

def Matches {Id Θ Context : Type} (actual claimed : Receipt Id Θ Context) : Prop :=
  actual.identity = claimed.identity ∧
  (∀ t, actual.defined t ↔ claimed.defined t) ∧
  (∀ i t, actual.values i t = claimed.values i t) ∧
  (∀ i t, actual.unsupported i t ↔ claimed.unsupported i t) ∧
  actual.outcome = claimed.outcome

theorem matches_preserves_holes {Id Θ Context : Type}
    (actual claimed : Receipt Id Θ Context) (same : Matches actual claimed) (i : Nat) (t : Θ) :
    actual.values i t = none ↔ claimed.values i t = none := by rw [same.2.2.1 i t]

theorem impossible_keeps_requested_identity {Id Θ Context : Type}
    (actual claimed : Receipt Id Θ Context) (requested : Id)
    (same : Matches actual claimed) (identity : actual.identity = requested)
    (impossible : actual.outcome = none) :
    claimed.identity = requested ∧ claimed.outcome = none :=
  ⟨same.1.symm.trans identity, same.2.2.2.2.symm.trans impossible⟩

noncomputable def admit {Id Θ Context Input Error : Type}
    (replay : Id → Input → Except Error (Receipt Id Θ Context)) (identity : Id)
    (input : Input) (claimed : Receipt Id Θ Context) :
    Except (Option Error) (Receipt Id Θ Context) := by
  classical
  exact match replay identity input with
  | .error e => .error (some e)
  | .ok actual => if Matches actual claimed then .ok actual else .error none

theorem admission_returns_replayed_object {Id Θ Context Input Error : Type}
    (replay : Id → Input → Except Error (Receipt Id Θ Context)) (identity : Id)
    (input : Input) (claimed result : Receipt Id Θ Context)
    (accepted : admit replay identity input claimed = .ok result) :
    replay identity input = .ok result ∧ Matches result claimed := by
  classical
  cases h : replay identity input with
  | error e => simp [admit,h] at accepted
  | ok actual =>
    by_cases hm : Matches actual claimed
    · simp only [admit,h,hm,ite_true,Except.ok.injEq] at accepted
      subst result
      exact ⟨rfl,hm⟩
    · simp [admit,h,hm] at accepted

theorem replay_error_preserved {Id Θ Context Input Error : Type}
    (replay : Id → Input → Except Error (Receipt Id Θ Context)) (identity : Id)
    (input : Input) (claimed : Receipt Id Θ Context) (e : Error)
    (failed : replay identity input = .error e) :
    admit replay identity input claimed = .error (some e) := by simp [admit,failed]

theorem false_receipt_rejected {Id Θ Context Input Error : Type}
    (replay : Id → Input → Except Error (Receipt Id Θ Context)) (identity : Id)
    (input : Input) (claimed actual : Receipt Id Θ Context)
    (replayed : replay identity input = .ok actual) (falseClaim : ¬Matches actual claimed) :
    admit replay identity input claimed = .error none := by simp [admit,replayed,falseClaim]

/-- Agreement away from a hole does not establish receipt equivalence. -/
theorem matching_interior_can_hide_endpoint_hole :
    let total : Bool → Option Rat := fun _ => some 1
    let withHole : Bool → Option Rat := fun t => if t then some 1 else none
    total true = withHole true ∧ total false ≠ withHole false := by decide +kernel

/-- Even equal coarse guard codes cannot substitute another actual parameter. -/
theorem guard_code_is_not_parameter_identity :
    let guard : Rat → Bool := fun p => p > 0
    guard (1/4) = guard (3/4) ∧ (1/4 : Rat) ≠ 3/4 := by decide +kernel

#print axioms FamilyTransport.value_reconstruction
#print axioms FamilyTransport.active_totality_gives_total_value
#print axioms FamilyTransport.zero_default
#print axioms FamilyTransport.active_hole_is_not_zero
#print axioms FamilyTransport.inactive_hole_has_zero_default
#print axioms FamilyTransport.parameter_hole_is_not_zero
#print axioms FamilyTransport.sum_congr
#print axioms FamilyTransport.map_keeps_actual_parameter
#print axioms FamilyTransport.row_reconstruction
#print axioms FamilyTransport.normalization_reconstructed
#print axioms FamilyTransport.indexed_unsupported_reconstructed
#print axioms FamilyTransport.valid_domain_reconstructed
#print axioms FamilyTransport.zero_target_never_unsupported
#print axioms FamilyTransport.matches_preserves_holes
#print axioms FamilyTransport.impossible_keeps_requested_identity
#print axioms FamilyTransport.admission_returns_replayed_object
#print axioms FamilyTransport.replay_error_preserved
#print axioms FamilyTransport.false_receipt_rejected
#print axioms FamilyTransport.matching_interior_can_hide_endpoint_hole
#print axioms FamilyTransport.guard_code_is_not_parameter_identity

end FamilyTransport
