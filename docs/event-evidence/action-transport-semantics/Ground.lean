import Std

/-!
Closed ground values and transaction contributions use the same contextual
coverage law. This model makes context retention explicit: an empty union or
contextual full target acquires the first source's context, and cannot silently
accept later foreign Events. Context includes support and law, not resident
manager identity. These are denotational contracts, not proofs of Rust, BEVT,
the schema fingerprint or resolver implementations.
-/
namespace Ground

inductive Coverage (C W : Type) where
  | empty
  | full
  | region (context : C) (membership : W → Bool)

def bind {C W : Type} [DecidableEq C] (target : Coverage C W) (context : C) :
    Option (W → Bool) :=
  match target with
  | .empty => some (fun _ => false)
  | .full => some (fun _ => true)
  | .region owner event => if context = owner then some event else none

def retain {C W : Type} [DecidableEq C] (target : Coverage C W) (context : C) :
    Option (Coverage C W) := (bind target context).map (Coverage.region context)

def included {W : Type} (source target : W → Bool) : Prop :=
  ∀ w, source w = true → target w = true

def accepts {C W : Type} [DecidableEq C] (target : Coverage C W)
    (context : C) (source : W → Bool) : Prop :=
  ∃ event, bind target context = some event ∧ included source event

theorem retain_empty {C W : Type} [DecidableEq C] (c : C) :
    retain (.empty : Coverage C W) c = some (.region c (fun _ => false)) := rfl

theorem retain_full {C W : Type} [DecidableEq C] (c : C) :
    retain (.full : Coverage C W) c = some (.region c (fun _ => true)) := rfl

theorem same_context {C W : Type} [DecidableEq C] (c : C) (event : W → Bool) :
    retain (.region c event) c = some (.region c event) := by
  simp [retain, bind]

theorem foreign_refused {C W : Type} [DecidableEq C] (c d : C) (different : c ≠ d)
    (source target : W → Bool) : ¬ accepts (.region d target) c source := by
  simp [accepts, bind, different]

theorem empty_still_remembers {C W : Type} [DecidableEq C] (c d : C)
    (different : c ≠ d) :
    retain (.empty : Coverage C W) d = some (.region d (fun _ => false)) ∧
      ¬ accepts (.region d (fun _ : W => false)) c (fun _ => false) :=
  ⟨retain_empty d, foreign_refused c d different _ _⟩

theorem full_still_remembers {C W : Type} [DecidableEq C] (c d : C)
    (different : c ≠ d) :
    retain (.full : Coverage C W) d = some (.region d (fun _ => true)) ∧
      ¬ accepts (.region d (fun _ : W => true)) c (fun _ => false) :=
  ⟨retain_full d, foreign_refused c d different _ _⟩

theorem empty_needs_no_target {C W : Type} [DecidableEq C] (c : C) :
    accepts (.empty : Coverage C W) c (fun _ => false) := by
  exact ⟨_, rfl, by simp [included]⟩

theorem full_covers_every_event {C W : Type} [DecidableEq C] (c : C)
    (source : W → Bool) : accepts (.full : Coverage C W) c source := by
  exact ⟨_, rfl, by simp [included]⟩

theorem nonempty_requires_witness {C W : Type} [DecidableEq C] (c : C)
    (source : W → Bool) (possible : ∃ w, source w = true) :
    ¬ accepts (.empty : Coverage C W) c source := by
  rintro ⟨event, equal, cover⟩
  have e : event = (fun _ => false) := (Option.some.inj equal).symm
  obtain ⟨w, present⟩ := possible
  have h := cover w present
  rw [e] at h
  cases h

/-- A later execution's owner may differ. A successful binding must preserve
    context and incidence; under that premise coverage is representation-free. -/
theorem binding_preserves_coverage {R C W : Type} [DecidableEq C]
    (context : R → C) (meaning : R → W → Bool) (bindValue : R → R)
    (sameContext : ∀ r, context (bindValue r) = context r)
    (sameMeaning : ∀ r, meaning (bindValue r) = meaning r)
    (target : Coverage C W) (source : R) :
    accepts target (context (bindValue source)) (meaning (bindValue source)) ↔
      accepts target (context source) (meaning source) := by
  rw [sameContext, sameMeaning]

/-- Canonical fingerprint bytes are allowed to forget presentation identity
    only under this explicit faithfulness obligation. -/
theorem portable_identity {R V B : Type} (meaning : R → V) (encode : R → B)
    (faithful : ∀ a b, encode a = encode b ↔ meaning a = meaning b)
    (a b : R) (same : meaning a = meaning b) : encode a = encode b :=
  (faithful a b).mpr same

end Ground

#print axioms Ground.retain_empty
#print axioms Ground.retain_full
#print axioms Ground.same_context
#print axioms Ground.foreign_refused
#print axioms Ground.empty_still_remembers
#print axioms Ground.full_still_remembers
#print axioms Ground.empty_needs_no_target
#print axioms Ground.full_covers_every_event
#print axioms Ground.nonempty_requires_witness
#print axioms Ground.binding_preserves_coverage
#print axioms Ground.portable_identity
