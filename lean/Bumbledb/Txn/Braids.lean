import Bumbledb.Txn.DeltaRestriction

/-!
# Braids — component locality and replay idempotence

The braid derivation partitions a descriptor's relations into
components and guarantees that every declared statement's obligation
instances read and write relations of exactly one of them. This
module carries that guarantee into the transaction model as the two
theorems the replication design spends. **L9 — component locality**:
judgment and application over one braid are invariant under any other
braid's history, so cross-braid interleavings are semantically
invisible and per-braid logs commit concurrently with nothing
consulted across the seam. **L10 — replay idempotence**: a batch
whose effects the state already contains re-applies to the identical
state with an accepted verdict and no generation advance, so every
crash window heals by replaying forward. `Delta`, `Delta.applyTo`,
`judge`, `holds`, and the den-transfer lemmas of `DeltaRestriction`
are consumed, never restated.

## The hypotheses are the braid derivation's own guarantees

* `ComponentClosed` — every declared statement's consulted relations
  share one component: the defining property of the derived braid
  quotient, taken as the hypothesis it is.
* `LocalTo` — a batch writes only relations of one component: what a
  braid's writer submits by construction.

## Narrowings recorded (law 5: narrow and record)

* **The net delta is the write model.** `Delta` is the coalesced set
  pair; an op list reaches this model through its net pair, exactly
  as the write path coalesces before judgment.
* **L9 quantifies the braid's own batch freely.** Only the foreign
  history's locality is spent: whatever batch `d` a braid stacks on
  its base, a history local to another component moves no relation of
  the braid's component and no judgment anchored there.
-/

namespace Bumbledb
namespace Txn

/-! ## The touched facts and the net effect of a batch -/

/-- The facts one batch writes at `R`: its net insert and delete
sets, mode-blind — the touched notion every locality argument
projects. -/
def Delta.touches (d : Delta) (R : RelId) (f : Fact) : Prop :=
  f ∈ d.adds R ∨ f ∈ d.removes R

/-- An untouched fact is outside the insert set. -/
theorem Delta.not_adds_of_not_touches {d : Delta} {R : RelId}
    {f : Fact} (h : ¬ d.touches R f) : f ∉ d.adds R :=
  fun hx => h (Or.inl hx)

/-- An untouched fact is outside the delete set. -/
theorem Delta.not_removes_of_not_touches {d : Delta} {R : RelId}
    {f : Fact} (h : ¬ d.touches R f) : f ∉ d.removes R :=
  fun hx => h (Or.inr hx)

/-- The effective inserts of a batch at one base: the added facts the
base does not already hold. The publish law reads this — a commit is
state-changing exactly when an effective set is nonempty. -/
def Delta.effAdds (d : Delta) (J : Instance) (R : RelId) : Set Fact :=
  fun f => f ∈ d.adds R ∧ f ∉ J R

/-- The effective deletes of a batch at one base: the removed facts
the base holds and the batch does not re-add (an add of a removed
fact lands present — the net survivor, `apply`'s own reading). -/
def Delta.effRemoves (d : Delta) (J : Instance) (R : RelId) :
    Set Fact :=
  fun f => f ∈ d.removes R ∧ f ∉ d.adds R ∧ f ∈ J R

/-- A batch whose effects the state already contains: every add is
present, every remove is absent. L10's hypothesis — the state a crash
window replays into after its own commit landed. -/
def Delta.containedIn (d : Delta) (I : Instance) : Prop :=
  (∀ R f, f ∈ d.adds R → f ∈ I R) ∧
    ∀ R f, f ∈ d.removes R → f ∉ I R

namespace Braids

/-! ## Locality congruence — one relation's slice decides everything
read there -/

/-- Raw presence is stable at an untouched fact — one apply layer. -/
theorem applyTo_untouched {d : Delta} {R : RelId} {f : Fact}
    (h : ¬ d.touches R f) (I : Instance) :
    f ∈ d.applyTo I R ↔ f ∈ I R := by
  rw [mem_applyTo]
  constructor
  · rintro (⟨h1, _⟩ | h2)
    · exact h1
    · exact absurd h2 (Delta.not_adds_of_not_touches h)
  · intro h1
    exact Or.inl ⟨h1, Delta.not_removes_of_not_touches h⟩

/-- Application at one relation reads only that relation's pre-state:
equal pre-slices at `R` land equal final slices at `R`. -/
theorem applyTo_congr_at {d : Delta} {J J' : Instance} {R : RelId}
    (h : J R = J' R) : d.applyTo J R = d.applyTo J' R := by
  funext f
  simp only [Delta.applyTo]
  rw [h]

/-- The denotation at one relation reads only that relation's
instance slice — a closed relation reads its sealed axioms either
way. -/
theorem den_congr {T : Theory} {J J' : Instance} {R : RelId}
    (h : J R = J' R) : T.den J R = T.den J' R := by
  cases hc : T.closed R with
  | some ext => exact den_closed_constant hc J J'
  | none =>
    simp only [Theory.den, hc]
    exact h

/-! ## The statement graph -/

/-- The relations one statement consults — the braid derivation's
edge set: a statement is one hyperedge over its relations. -/
def stmtRels : Statement → List RelId
  | .functionality R _ => [R]
  | .containment src tgt => [src.relation, tgt.relation]
  | .capacity tgt _ _ src => [src.relation, tgt.relation]

/-- Statements never span braid components: every theory statement's
consulted relations share one component — the braid derivation's
defining guarantee, taken as the hypothesis it is. -/
def ComponentClosed (comp : RelId → Nat) (T : Theory) : Prop :=
  ∀ st, st ∈ T.statements → ∀ R R', R ∈ stmtRels st →
    R' ∈ stmtRels st → comp R = comp R'

/-- A batch writing inside one braid component. -/
def LocalTo (comp : RelId → Nat) (d : Delta) (c : Nat) : Prop :=
  ∀ R f, d.touches R f → comp R = c

/-- One statement's judgment reads the denotation at its own
consulted relations and nothing else — the congruence L9 spends, arm
for arm over `Statement.judgment`'s dispatch. -/
theorem judgment_congr {T : Theory} {J J' : Instance} {st : Statement}
    (h : ∀ R, R ∈ stmtRels st → T.den J R = T.den J' R) :
    st.judgment T J ↔ st.judgment T J' := by
  cases st with
  | functionality R X =>
    have hR := h R List.mem_cons_self
    simp only [Statement.judgment]
    rw [hR]
  | containment src tgt =>
    have hsrc := h src.relation List.mem_cons_self
    have htgt := h tgt.relation
      (List.mem_cons_of_mem _ List.mem_cons_self)
    simp only [Statement.judgment]
    rw [hsrc, htgt]
  | capacity tgt wt w src =>
    have hsrc := h src.relation List.mem_cons_self
    have htgt := h tgt.relation
      (List.mem_cons_of_mem _ List.mem_cons_self)
    simp only [Statement.judgment]
    rw [hsrc, htgt]

/-! ## L9 — component locality -/

/-- **L9 — component locality.** A statement's obligation instances
read and write only relations inside one component, so judgment and
application over one braid are invariant under any other braid's
history. With `s` local to a foreign component, every relation of the
braid's component holds the same rows over the moved base as over the
raw one — whatever batch `d` the braid stacks on top — and every
statement anchored in the braid judges the moved final state exactly
as it judges the unmoved one. Cross-braid interleavings are
semantically invisible, which is why per-braid logs commit
concurrently with nothing consulted across the seam. The braid's own
batch is quantified freely: only the foreign history's locality is
spent. -/
theorem L9 {T : Theory} {comp : RelId → Nat} {s d : Delta}
    {c c' : Nat} (hT : ComponentClosed comp T)
    (hs : LocalTo comp s c) (hne : c ≠ c') (I : Instance) :
    (∀ R, comp R = c' → d.applyTo (s.applyTo I) R = d.applyTo I R) ∧
    ∀ st, st ∈ T.statements →
      (∃ R, R ∈ stmtRels st ∧ comp R = c') →
      (st.judgment T (d.applyTo (s.applyTo I)) ↔
        st.judgment T (d.applyTo I)) := by
  have happ : ∀ R, comp R = c' →
      d.applyTo (s.applyTo I) R = d.applyTo I R := by
    intro R hR
    refine applyTo_congr_at ?_
    funext f
    refine propext (applyTo_untouched (fun ht => ?_) I)
    exact hne ((hs R f ht).symm.trans hR)
  refine ⟨happ, ?_⟩
  rintro st hst ⟨R0, hR0, hc0⟩
  exact judgment_congr fun R hR =>
    den_congr (happ R ((hT st hst R R0 hR hR0).trans hc0))

/-! ## L10 — replay idempotence -/

/-- A contained batch applies to nothing: `(base ∖ removes) ∪ adds`
is `base` when every add is present and every remove absent. -/
theorem applyTo_contained {d : Delta} {I : Instance}
    (h : d.containedIn I) : d.applyTo I = I := by
  funext R f
  refine propext ⟨fun hf => ?_, fun hf => ?_⟩
  · rcases hf with ⟨hf, _⟩ | hf
    · exact hf
    · exact h.1 R f hf
  · exact Or.inl ⟨hf, fun hr => h.2 R f hr hf⟩

/-- One op whose facts the state already disposes: an insert of
present facts, a delete of absent facts. -/
def opDisposed (I : Instance) : Op → Prop
  | .insert R fs => ∀ f, f ∈ fs → f ∈ I R
  | .delete R fs => ∀ f, f ∈ fs → f ∉ I R

/-- Inserting present facts is the identity — each singleton step
lands a fact the instance already holds. -/
theorem insertFacts_disposed {R : RelId} {I : Instance} :
    ∀ {fs : List Fact}, (∀ f, f ∈ fs → f ∈ I R) →
      insertFacts R fs I = I
  | [], _ => rfl
  | f :: fs, h => by
    show insertFacts R fs (fun R' g => g ∈ I R' ∨ (R' = R ∧ g = f)) = I
    have heq : (fun R' g => g ∈ I R' ∨ (R' = R ∧ g = f)) = I := by
      funext R' g
      refine propext ⟨fun hg => ?_, Or.inl⟩
      rcases hg with hg | ⟨rfl, rfl⟩
      · exact hg
      · exact h g List.mem_cons_self
    rw [heq]
    exact insertFacts_disposed fun g hg =>
      h g (List.mem_cons_of_mem f hg)

/-- Deleting absent facts is the identity. -/
theorem deleteFacts_disposed {R : RelId} {I : Instance} :
    ∀ {fs : List Fact}, (∀ f, f ∈ fs → f ∉ I R) →
      deleteFacts R fs I = I
  | [], _ => rfl
  | f :: fs, h => by
    show deleteFacts R fs
      (fun R' g => g ∈ I R' ∧ ¬(R' = R ∧ g = f)) = I
    have heq : (fun R' g => g ∈ I R' ∧ ¬(R' = R ∧ g = f)) = I := by
      funext R' g
      refine propext ⟨fun hg => hg.1, fun hg => ⟨hg, ?_⟩⟩
      rintro ⟨rfl, rfl⟩
      exact h g List.mem_cons_self hg
    rw [heq]
    exact deleteFacts_disposed fun g hg =>
      h g (List.mem_cons_of_mem f hg)

/-- **Every op net-disposes**: a whole op sequence the state already
disposes applies to the identical state — each op is the identity, so
the fold is. -/
theorem applyOps_disposed {I : Instance} :
    ∀ {ops : List Op}, (∀ op, op ∈ ops → opDisposed I op) →
      applyOps I ops = I
  | [], _ => rfl
  | op :: rest, h => by
    have hop : op.apply I = I := by
      cases op with
      | insert R fs => exact insertFacts_disposed (h _ List.mem_cons_self)
      | delete R fs => exact deleteFacts_disposed (h _ List.mem_cons_self)
    show applyOps (op.apply I) rest = I
    rw [hop]
    exact applyOps_disposed fun op' hop' =>
      h op' (List.mem_cons_of_mem op hop')

/-- The op-sequence face of L10: a disposed sequence commits to the
SAME state, accepted — the verdict is the pre-state's own commitment,
never a fresh judgment. -/
theorem commitOps_disposed {T : Theory} (b : State T) {ops : List Op}
    (h : ∀ op, op ∈ ops → opDisposed b.inst op) :
    commitOps b ops = .ok b := by
  unfold commitOps
  rw [applyOps_disposed h, judge_holds b.models]

/-- **L10 — replay idempotence.** Re-applying a batch whose effects
the state already contains yields the identical state (`apply` is the
identity), an empty effective delta (nothing effective to insert or
delete — the no-op arm's own reading, so the engine never reaches
judgment: the verdict below is discharged by the state's OWN
commitment, `State.models`, with the delta never consulted), and an
accepted verdict whose state is the pre-state ITSELF — so the
state-changing predicate is false and the generation does not
advance (`crate::GenerationId` moves only on state change; a re-landed
slot is a proven no-op). The theorem the recovery design stands on:
crash windows heal by replaying forward, because replaying
backward-overlap is harmless — `replay_heals` is exactly that
composition, and `commitOps_disposed`/`applyOps_disposed` carry the
per-op reading (every op net-disposes). -/
theorem L10 {T : Theory} (b : State T) {d : Delta}
    (h : d.containedIn b.inst) :
    apply b d = b.inst ∧
      (∀ R f, f ∉ d.effAdds b.inst R) ∧
      (∀ R f, f ∉ d.effRemoves b.inst R) ∧
      commit b d = .ok b := by
  have happ : apply b d = b.inst := by
    rw [apply_eq_applyTo]
    exact applyTo_contained h
  refine ⟨happ, ?_, ?_, ?_⟩
  · rintro R f ⟨hfa, hnp⟩
    exact hnp (h.1 R f hfa)
  · rintro R f ⟨hfr, _, hp⟩
    exact h.2 R f hfr hp
  · unfold commit
    rw [happ, judge_holds b.models]

/-- The recovery composition: a coalesced batch that COMMITTED is
contained in its own post-state, so replaying it there is L10's
no-op — identical state, accepted verdict, no generation advance.
The crash window between a local commit and its publish heals by
replaying forward. -/
theorem replay_heals {T : Theory} {b b' : State T} {d : Delta}
    (hco : ∀ R f, f ∈ d.adds R → f ∉ d.removes R)
    (h : commit b d = .ok b') :
    apply b' d = b'.inst ∧ commit b' d = .ok b' := by
  have hcon : d.containedIn b'.inst := by
    rw [commit_ok_inst h, apply_eq_applyTo]
    constructor
    · intro R f hf
      exact Or.inr hf
    · intro R f hf hmem
      rcases hmem with ⟨_, hnr⟩ | hadd
      · exact hnr hf
      · exact hco R f hadd hf
  obtain ⟨h1, _, _, h4⟩ := L10 b' hcon
  exact ⟨h1, h4⟩

end Braids
end Txn
end Bumbledb
