import Bumbledb.Query.Denotation

/-!
# Txn — the lifecycle (Level 2, PRD 09)

The transaction state machine: op-order invariance, final-state
judgment, generation witnesses, snapshot isolation, the ETL identity.
The machine is deliberately tiny — no interleaving, no threads, no
concurrency beyond the generation tag (single-writer is the engine's
law: the PROTOCOL is modeled, never the mutex). Durability and crash
are REFUSED here (the covenant refusals): the crashpoint estate owns
that axis whole; this level models committed-state transitions only.

## The FinalStateView seam, as a signature

`judge : Theory → Instance → Result` — dependency judgment's ONLY
input is a theory and one final instance. Operation order is not in
the signature, so it cannot influence a verdict: the constitution made
judgment's input a type (`storage/commit/judgment.rs::FinalStateView`,
"operation order is no longer representable here"), and this file
gives that type its theorems (`final_state_judgment_order_free`).
`Delta` is a SET pair and `apply` is set algebra — order-free BY
CONSTRUCTION; the sequential write surface (`Op` lists) exists exactly
to state the invariance against it.

## The two failure kinds are two constructors

`WriteResult` separates `violations` (dependency failure: the COMPLETE
violated-statement set) from `generationMoved` (witness conflict: the
world moved after the snapshot) as distinct constructors — the type IS
the theorem "generation conflict ≠ dependency failure"
(`witness_conflict_distinct` states the never-converts contract
anyway; it is the API's contract sentence). Bridge:
`crate::error::Error::CommitRejected` vs `Error::GenerationMoved`;
`api/db/write.rs::write_witnessed`'s one integer compare inside the
critical section (write.rs:136-140) is `writeWitnessed`'s one `if`.

## Bridges

* `judge` / `commit` — `storage/commit/judgment.rs::judge` over
  `FinalStateView` (phases 1–2 applied the plan; read-your-writes is
  exactly `base + delta` in final set semantics).
* `violationSet` / `rejection_is_complete` —
  `crate::error::Violations`: the complete violation set of one
  rejected commit, both scan-complete sides.
* `writeFrom` / `writeWitnessed` — `api/db/write.rs`'s `Db::write_from`
  / `Db::write` sharing one body; the witness is the `Snapshot` the
  host read its premises on, consumed for its generation alone.
* `Snapshot.read` — `api/db/snapshot.rs`: every read runs against one
  parked read transaction, one generation.
* `scanLoad` — cookbook recipe 28 (migration is ETL): `Snapshot::scan`
  exports under one generation, the host transforms, `bulk_load`
  imports under the new theory's ordinary final-state judgment.

## Narrowings recorded (law 5: narrow and record)

* **`violationSet` is a `Set Statement`.** The Rust `Violations` seal
  (stable sort by citation, dedup, per-direction citations) is list
  REPRESENTATION mechanism; a set carries no duplicates and no order
  by construction. What the model keeps is the semantic content:
  membership completeness, membership soundness, and nonemptiness —
  `rejection_is_complete`, all three.
* **`Generation` is an opaque tag with decidable equality only** —
  the protocol is one compare ("unmoved or moved"), never arithmetic;
  mirrors `Snapshot` exposing no `generation()` accessor.
* **`writeWitnessed` models the protocol, not the environment**: the
  `ForeignSnapshot` environment-identity check and the writer mutex
  are mechanism outside the model.
* **`scanLoad` bulk-judges the transformed instance as ONE final
  state.** `bulk_load`'s 4096-fact chunking is mechanism: a chunked
  load is a SEQUENCE of ordinary commits, each judged
  (`committed_states_model` covers every prefix), which is exactly why
  recipe 28's first law — load containment targets first — is
  host-facing: an early chunk is judged before later chunks land.
  Recipe 28's second law (fresh identity survives, the mint catches
  up) is id-allocation mechanism, not modeled; the third law is
  `etl_lands_valid`.
* **The transform is one partial map `Fact → Option Fact`** applied
  uniformly (dropping facts is expressible; per-relation retargeting
  is host plumbing the model does not restate).
-/

namespace Bumbledb
namespace Txn

/-! ## States — the holds-carrying instance -/

/-- A committed database state: an instance CARRYING its proof that it
models the theory. The type is the "free lunches" law — a query can
assume every declared dependency of any `State` it is handed, because
no constructor mints one from an unjudged instance
(`committed_states_model` is the induction, absorbed by this field). -/
structure State (T : Theory) where
  /-- The committed instance. -/
  inst : Instance
  /-- The commitment: the instance models the theory. -/
  models : holds T inst

/-! ## Deltas — the final-set write -/

/-- A transaction's net write: insert/delete fact multiop as a SET
pair. No order exists in this representation — that is the point
(`storage/delta.rs::WriteDelta`, whose per-fact coalescing computes
exactly this net pair). -/
structure Delta where
  /-- The facts the transaction establishes, per relation. -/
  adds : RelId → Set Fact
  /-- The facts the transaction disestablishes, per relation. -/
  removes : RelId → Set Fact

/-- The final state: `(base \ removes) ∪ adds`, set algebra — order-free
BY CONSTRUCTION (an add of a removed fact lands present: the add is the
net survivor, as in the coalesced delta). -/
def apply {T : Theory} (s : State T) (d : Delta) : Instance :=
  fun R f => (f ∈ s.inst R ∧ f ∉ d.removes R) ∨ f ∈ d.adds R

/-! ## The sequential write surface — what the set pair erases -/

/-- One write operation. A LIST of these carries an order — which is
exactly the thing `Delta` cannot represent and `judge`'s signature
cannot read. -/
inductive Op where
  /-- Establish fact `f` in relation `R`. -/
  | insert (R : RelId) (f : Fact)
  /-- Disestablish fact `f` from relation `R`. -/
  | delete (R : RelId) (f : Fact)

/-- One operation's effect on an instance. -/
def Op.apply : Op → Instance → Instance
  | .insert R f, I => fun R' g => g ∈ I R' ∨ (R' = R ∧ g = f)
  | .delete R f, I => fun R' g => g ∈ I R' ∧ ¬(R' = R ∧ g = f)

/-- A whole op sequence's effect, applied in order. -/
def applyOps (I : Instance) : List Op → Instance
  | [] => I
  | op :: rest => applyOps (op.apply I) rest

/-! ## The complete violation set -/

/-- The violated statements of one final state — PRD 03's violation
predicate (the negated `Statement.judgment`), collected over the
theory's declared statements. Bridge: `crate::error::Violations`, the
complete violation set of one rejected commit; the narrowing to a
`Set` (sortedness and dedup are representation) is recorded in the
module doc. -/
def violationSet (T : Theory) (I : Instance) : Set Statement :=
  fun st => st ∈ T.statements ∧ ¬ st.judgment T I

/-- `holds` is exactly the empty violation set — the accept path never
carries a rejection (`Violations::seal` returns `None` on empty). -/
theorem holds_iff_no_violation (T : Theory) (I : Instance) :
    holds T I ↔ ∀ st, st ∉ violationSet T I :=
  ⟨fun h st hst => hst.2 (h st hst.1),
   fun h st hmem => Classical.byContradiction fun hj => h st ⟨hmem, hj⟩⟩

/-! ## Judgment — the two-constructor sum -/

/-- The commit verdict: a two-constructor sum, accepted state or
rejection payload. -/
inductive Result (α : Type u) (ε : Type v) where
  /-- The transaction committed: here is the new state. -/
  | ok (value : α)
  /-- The transaction aborted whole: here is why. -/
  | reject (err : ε)

open Classical in
/-- **The FinalStateView seam.** Dependency judgment's whole input is
this signature: a theory and ONE final instance — accept iff `holds`,
else the COMPLETE violation set. Operation order is not a parameter,
so no verdict can depend on it. Bridge:
`storage/commit/judgment.rs::judge(view: &FinalStateView)`, the sole
judge input; classical choice decides `holds` here because the model
judges arbitrary (not-necessarily-listable) fact sets — the engine's
instances are finite and its judge is the decision procedure. -/
noncomputable def judge (T : Theory) (I : Instance) :
    Result (State T) (Set Statement) :=
  if h : holds T I then .ok ⟨I, h⟩ else .reject (violationSet T I)

/-- `judge` on a modeling instance: accept, and the accepted state IS
the judged instance. -/
theorem judge_holds {T : Theory} {I : Instance} (h : holds T I) :
    judge T I = .ok ⟨I, h⟩ := by
  unfold judge
  exact dif_pos h

/-- `judge` on a non-modeling instance: reject with the complete
violation set — never a representative, never a prefix. -/
theorem judge_not_holds {T : Theory} {I : Instance} (h : ¬ holds T I) :
    judge T I = .reject (violationSet T I) := by
  unfold judge
  exact dif_neg h

/-- `commit` — judge the delta's final state against the theory:
accept iff `holds`, else the complete violation set. Bridge:
`Db::write`'s commit phase (`storage/commit`): phases 1–2 apply the
plan, phase 3 is `judge` over the `FinalStateView`, and a rejection
aborts the whole transaction (`Error::CommitRejected`). -/
noncomputable def commit {T : Theory} (s : State T) (d : Delta) :
    Result (State T) (Set Statement) :=
  judge T (apply s d)

/-- The sequential sibling: commit an op LIST — judged through the
same `judge`, on the sequence's final state alone. -/
noncomputable def commitOps {T : Theory} (s : State T)
    (ops : List Op) : Result (State T) (Set Statement) :=
  judge T (applyOps s.inst ops)

/-- An accepted commit's state is the delta's final state — `commit`
invents nothing and drops nothing. -/
theorem commit_ok_inst {T : Theory} {s : State T} {d : Delta}
    {s' : State T} (h : commit s d = .ok s') :
    s'.inst = apply s d := by
  by_cases hh : holds T (apply s d)
  · unfold commit at h
    rw [judge_holds hh] at h
    cases h
    rfl
  · unfold commit at h
    rw [judge_not_holds hh] at h
    exact nomatch h

/-! ## Item 1 — the final-state judgment is order-free -/

/-- **Item 1.** Any two op sequences with equal `apply` results
receive identical verdicts: insert/delete order inside a transaction
cannot change validity, because the verdict is a function of the final
state alone — `judge`'s signature (the FinalStateView law). The
transiently-violating-but-valid witness that a per-operation judge
would wrongly reject is `Countermodels.per_op_judgment_wrong`.
Bridge: `judgment.rs::FinalStateView`, the sole judge input. -/
theorem final_state_judgment_order_free {T : Theory} (s : State T)
    (ops₁ ops₂ : List Op)
    (h : applyOps s.inst ops₁ = applyOps s.inst ops₂) :
    commitOps s ops₁ = commitOps s ops₂ := by
  unfold commitOps
  rw [h]

/-- The delta form: two deltas with equal final states receive one
verdict — `commit` is extensional in the applied state. -/
theorem commit_extensional {T : Theory} (s : State T) (d₁ d₂ : Delta)
    (h : apply s d₁ = apply s d₂) : commit s d₁ = commit s d₂ := by
  unfold commit
  rw [h]

/-! ## Item 2 — committed states model the theory -/

/-- Reachability by accepted commits — the lifecycle's transition
relation: the induction over `commit` that `committed_states_model`
runs. -/
inductive Reachable (T : Theory) : State T → State T → Prop where
  /-- The trivial chain. -/
  | refl (s : State T) : Reachable T s s
  /-- One more accepted commit. -/
  | step {s₀ s₁ s₂ : State T} (d : Delta) :
      Reachable T s₀ s₁ → commit s₁ d = .ok s₂ → Reachable T s₀ s₂

/-- **Item 2.** Every committed state satisfies `holds` — the "free
lunches" law: queries may assume every declared dependency of every
committed state. The induction over `commit` is absorbed by the type:
`State` carries `models`, and `judge`'s only accepting arm mints the
state from the very proof it just judged — so the proof term is one
field projection, which is the design working as intended.
Bridge: `judgment.rs::judge` (delta-restricted, sound because an
untouched binding cannot change a judgment's truth) and
`Db::verify_store` (the global re-verification). -/
theorem committed_states_model {T : Theory} {s₀ s : State T}
    (_ : Reachable T s₀ s) : holds T s.inst :=
  s.models

/-! ## Item 3 — rejection is complete -/

/-- **Item 3.** A rejected delta's violation set contains EVERY
violated statement of the final state (completeness), ONLY violated
statements (soundness), and at least one (nonemptiness — the accept
path never rejects). Bridge: `crate::error::Violations` — sealed
sorted, deduplicated, nonempty; both judgment sides are scan-complete
(`judgment.rs`: "the reject path runs exactly the checks the accept
path runs"). -/
theorem rejection_is_complete {T : Theory} (s : State T) (d : Delta)
    {V : Set Statement} (h : commit s d = .reject V) :
    (∀ st, st ∈ T.statements → ¬ st.judgment T (apply s d) →
      st ∈ V) ∧
    (∀ st, st ∈ V →
      st ∈ T.statements ∧ ¬ st.judgment T (apply s d)) ∧
    (∃ st, st ∈ V) := by
  by_cases hh : holds T (apply s d)
  · unfold commit at h
    rw [judge_holds hh] at h
    exact nomatch h
  · unfold commit at h
    rw [judge_not_holds hh] at h
    injection h with hV
    subst hV
    refine ⟨fun st hm hj => ⟨hm, hj⟩, fun _ hst => hst, ?_⟩
    exact Classical.byContradiction fun hne =>
      hh fun st hm => Classical.byContradiction fun hj =>
        hne ⟨st, hm, hj⟩

/-! ## Snapshots, generations, the witnessed write -/

/-- The generation tag: an opaque token with decidable equality ONLY —
the protocol is one compare, never arithmetic (recorded narrowing).
Bridge: `crate::GenerationId`, the state-changing generation the image
cache keys on (a counters-only commit does not move it). -/
structure Generation where
  /-- The opaque tag. -/
  tag : Nat
deriving DecidableEq

/-- A snapshot: one committed state plus the generation it was taken
at. Every read runs against exactly this pair — and `Snapshot.read`
consults only the state, never the tag (`read_ignores_generation`).
Bridge: `api/db/snapshot.rs::Snapshot` (one parked read transaction);
the generation is consumed internally by `write_from`, never exposed. -/
structure Snapshot (T : Theory) where
  /-- The state the snapshot observes. -/
  state : State T
  /-- The generation it was taken at. -/
  generation : Generation

/-- The witnessed-write verdict. `violations` and `generationMoved`
are DISTINCT constructors — the two failure kinds cannot be confused
by type, which IS the theorem "generation conflict ≠ dependency
failure" (`witness_conflict_distinct` states the contract sentence
anyway). Bridge: `Error::CommitRejected` vs `Error::GenerationMoved`
in `api/db/write.rs`. -/
inductive WriteResult (T : Theory) where
  /-- The transaction committed. -/
  | ok (s : State T)
  /-- Dependency failure: the complete violated-statement set — the
  transaction ran and its final state was judged wanting. -/
  | violations (V : Set Statement)
  /-- Witness conflict: a state-changing commit landed after the
  witness — the transaction body NEVER RAN and nothing was judged.
  Retry is host policy; the engine ships the value, never a loop. -/
  | generationMoved (witnessed current : Generation)

/-- Lift a commit verdict into the witnessed-write sum — the success
and violation arms, untouched. -/
def liftCommit {T : Theory} :
    Result (State T) (Set Statement) → WriteResult T
  | .ok s => .ok s
  | .reject V => .violations V

/-- **The one write body** (`api/db/write.rs::write_witnessed`): an
optional witnessed generation is the only difference between `write`
and `writeFrom` — one compare against the head's generation, before
anything is judged (write.rs:136-140: "Mismatch aborts before any page
is touched"). `head` is the current committed state and its
generation; the delta is whatever the host derived from its witness
snapshot. -/
noncomputable def writeWitnessed {T : Theory} (head : Snapshot T) :
    Option Generation → Delta → WriteResult T
  | some witnessed, d =>
    if witnessed = head.generation then liftCommit (commit head.state d)
    else .generationMoved witnessed head.generation
  | none, d => liftCommit (commit head.state d)

/-- The unconditional write: no witness, straight to judgment
(`Db::write` = `write_witnessed(None, f)`). -/
noncomputable def write {T : Theory} (head : Snapshot T) (d : Delta) :
    WriteResult T :=
  writeWitnessed head none d

/-- The optimistic protocol (`Db::write_from`): derive from a witness
snapshot, commit iff its generation is unmoved, else `generationMoved`.
The witness is consumed for its generation ALONE — evidence, never a
raw integer the caller could fabricate (the recorded refusal). -/
noncomputable def writeFrom {T : Theory} (head : Snapshot T)
    (witness : Snapshot T) (d : Delta) : WriteResult T :=
  writeWitnessed head (some witness.generation) d

/-- A moved generation: the verdict is `generationMoved`, full stop —
the transaction body never runs (`f` never runs; the delta was never
judged). -/
theorem writeFrom_moved {T : Theory} {head witness : Snapshot T}
    (d : Delta) (h : witness.generation ≠ head.generation) :
    writeFrom head witness d =
      .generationMoved witness.generation head.generation := by
  have heq : writeFrom head witness d =
      if witness.generation = head.generation then
        liftCommit (commit head.state d)
      else .generationMoved witness.generation head.generation := rfl
  rw [heq, if_neg h]

/-- An unmoved generation: the verdict is exactly the unconditional
write's — the witness compare is invisible on the success path. -/
theorem writeFrom_unmoved {T : Theory} {head witness : Snapshot T}
    (d : Delta) (h : witness.generation = head.generation) :
    writeFrom head witness d = liftCommit (commit head.state d) := by
  have heq : writeFrom head witness d =
      if witness.generation = head.generation then
        liftCommit (commit head.state d)
      else .generationMoved witness.generation head.generation := rfl
  rw [heq, if_pos h]

/-! ## Item 4 — witness conflicts are not violations -/

/-- **Item 4.** `writeFrom` never converts a generation move into a
violation or vice versa — by construction (distinct constructors),
stated anyway because it is the API's contract sentence: a
`violations` verdict proves the generation matched AND the delta's
final state was judged wanting; a `generationMoved` verdict proves the
generation moved and carries exactly the two tags — nothing was
judged. Bridge: `Error::GenerationMoved { witnessed, current }` vs
`Error::CommitRejected`; the one-compare in `write_witnessed`. -/
theorem witness_conflict_distinct {T : Theory}
    (head witness : Snapshot T) (d : Delta) :
    (∀ V, writeFrom head witness d = .violations V →
      witness.generation = head.generation ∧
        commit head.state d = .reject V) ∧
    (∀ g g', writeFrom head witness d = .generationMoved g g' →
      witness.generation ≠ head.generation ∧
        g = witness.generation ∧ g' = head.generation) := by
  constructor
  · intro V h
    by_cases hg : witness.generation = head.generation
    · rw [writeFrom_unmoved d hg] at h
      cases hc : commit head.state d with
      | ok s' =>
        rw [hc] at h
        exact nomatch h
      | reject V' =>
        rw [hc] at h
        injection h with hV
        exact ⟨hg, congrArg Result.reject hV⟩
    · rw [writeFrom_moved d hg] at h
      exact nomatch h
  · intro g g' h
    by_cases hg : witness.generation = head.generation
    · rw [writeFrom_unmoved d hg] at h
      cases hc : commit head.state d with
      | ok s' =>
        rw [hc] at h
        exact nomatch h
      | reject V' =>
        rw [hc] at h
        exact nomatch h
    · rw [writeFrom_moved d hg] at h
      injection h with h1 h2
      exact ⟨hg, h1.symm, h2.symm⟩

/-! ## Item 5 — a snapshot reads one state -/

/-- A read through a snapshot: PRD 04's answer denotation, evaluated
at the snapshot's state — the whole read surface. -/
def Snapshot.read {T : Theory} (snap : Snapshot T)
    (C : Query.Classify) (r : Query.Rule) (ρ : Query.ParamEnv) :
    Set Query.AnswerTuple :=
  Query.ruleAnswers C r snap.state.inst ρ

/-- **Item 5.** Every read is a function of ONE state — the
signature-level fact: `Snapshot.read` factors through `state.inst`
and nothing else, so two snapshots of one state answer identically,
whatever else the database has done since. Bridge:
`api/db/snapshot.rs` (one parked read transaction, one generation);
`70-api.md`'s snapshot isolation. -/
theorem snapshot_reads_one_state {T : Theory}
    (snap₁ snap₂ : Snapshot T) (h : snap₁.state = snap₂.state)
    (C : Query.Classify) (r : Query.Rule) (ρ : Query.ParamEnv) :
    snap₁.read C r ρ = snap₂.read C r ρ := by
  unfold Snapshot.read
  rw [h]

/-- The generation tag is invisible to reads — definitionally. -/
theorem read_ignores_generation {T : Theory} (s : State T)
    (g g' : Generation) (C : Query.Classify) (r : Query.Rule)
    (ρ : Query.ParamEnv) :
    (Snapshot.mk s g).read C r ρ = (Snapshot.mk s g').read C r ρ :=
  rfl

/-! ## Item 6 — derived soundness vs freshness -/

/-- A declared scalar containment, spent on any modeling instance:
every selected source fact has its selected target witness — `holds`
plus the declaration yields the judgment directly. -/
theorem holds_scalar_containment {T : Theory} {I : Instance}
    (h : holds T I) {src tgt : Atom}
    (hdecl : Statement.containment src tgt ∈ T.statements)
    (hsplit : T.header.intervalSplit src.relation src.projection =
      none) :
    Containment (T.den I src.relation) src.selection src.projection
      (T.den I tgt.relation) tgt.selection tgt.projection := by
  have hj := h _ hdecl
  cases htgt : T.header.intervalSplit tgt.relation tgt.projection with
  | none =>
    simp only [Statement.judgment, hsplit, htgt] at hj
    exact hj
  | some x =>
    simp only [Statement.judgment, hsplit, htgt] at hj
    exact hj

/-- **Item 6.** The maintenance protocol's division of authority: a
containment-constrained derived relation is SOUND in every committed
state — every derived fact is backed by its source, in every state any
chain of accepted commits reaches (from item 2). FRESHNESS is not a
property of any committed state: no dependency statement can demand
that the derived relation has caught up, and
`Countermodels.stale_but_sound` is the committed state carrying a
stale-but-sound derived relation — the host-discipline gap, formal.
Bridge: constitution PRD 20's maintenance protocol (the engine judges
soundness at commit; recomputation timing is the host's witness-loop
discipline, `Db::write_from`). -/
theorem derived_soundness_vs_freshness {T : Theory} {s₀ s : State T}
    (hr : Reachable T s₀ s) {src tgt : Atom}
    (hdecl : Statement.containment src tgt ∈ T.statements)
    (hsplit : T.header.intervalSplit src.relation src.projection =
      none) :
    Containment (T.den s.inst src.relation) src.selection
      src.projection
      (T.den s.inst tgt.relation) tgt.selection tgt.projection :=
  holds_scalar_containment (committed_states_model hr) hdecl hsplit

/-! ## Item 7 — the ETL identity -/

/-- The transformed instance: export every fact, keep what the
transform maps, land it — `Some` keeps (possibly rewritten), `none`
drops. One uniform partial map (recorded narrowing). -/
def transform (t : Fact → Option Fact) (I : Instance) : Instance :=
  fun R g => ∃ f, f ∈ I R ∧ t f = some g

/-- The identity transform is invisible: `transform some` is the
identity on instances. -/
theorem transform_id (I : Instance) : transform some I = I := by
  funext R g
  refine propext ⟨?_, fun hg => ⟨g, hg, rfl⟩⟩
  rintro ⟨f, hf, hfg⟩
  exact Option.some.inj hfg ▸ hf

/-- The ETL loop, abstractly (`scanLoad`): export every fact of the
source state, transform, bulk-judge the whole load under the TARGET
theory — one final-state judgment (the chunking narrowing is in the
module doc). Bridge: recipe 28 — `Snapshot::scan` under one
generation, the host transform, `bulk_load` into the new store; the
fingerprint refusal (`SchemaMismatch`) is what forces this loop to be
the only migration. -/
noncomputable def scanLoad {T : Theory} (s : State T) (T' : Theory)
    (t : Fact → Option Fact) : Result (State T') (Set Statement) :=
  judge T' (transform t s.inst)

/-- **Item 7a.** The identity transform into the SAME theory
reproduces the state — export and reimport of a committed state is a
no-op, and its judgment is discharged by the state's own commitment. -/
theorem etl_identity {T : Theory} (s : State T) :
    scanLoad s T some = .ok s := by
  unfold scanLoad
  rw [transform_id, judge_holds s.models]

/-- **Item 7b — recipe 28's third law.** A transform into a new theory
either lands HOLDING the new theory or the load rejects (with the
complete violation set): "a migration that lands is already valid" —
there is no migrate-now-validate-later state. -/
theorem etl_lands_valid {T : Theory} (s : State T) (T' : Theory)
    (t : Fact → Option Fact) :
    (∃ s' : State T', scanLoad s T' t = .ok s' ∧ holds T' s'.inst) ∨
    scanLoad s T' t =
      .reject (violationSet T' (transform t s.inst)) := by
  rcases Classical.em (holds T' (transform t s.inst)) with h | h
  · exact .inl ⟨⟨_, h⟩, judge_holds h, h⟩
  · exact .inr (judge_not_holds h)

end Txn
end Bumbledb
