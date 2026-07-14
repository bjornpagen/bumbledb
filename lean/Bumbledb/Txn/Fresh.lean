import Bumbledb.Txn

/-!
# Fresh — the mint (Level 2, the allocation model)

The fresh generator as a state machine: one monotone high-water mark
per (relation, field). `alloc` returns the mark and advances it; an
explicit-value write advances the mark past the supplied value; an
ABORTED transaction's run vanishes whole (nothing it minted was
observably returned); a SUCCESSFUL commit persists the final mark.
The model is bumbledb-authored: ids are ORDINARY WRITABLE VALUES that
happen to have a generator, so everything identity-shaped is enforced
by the ordinary final-state judgment, and the mint's own laws are
exactly the observability laws proved here — never-reissue-observable
(`never_reissue_observable`: the generator never re-issues ANY id a
committed transaction made observable, explicitly-supplied ids
included; `never_reissue_observed` is its generator-returns
projection), legal re-supply (`resupply_legal_monotone`), and the
materialized key riding plain `holds`
(`materialized_key_ordinary`).

## Why primer's route is not ours (recorded)

Primer's fresh model derives ids content-addressed through an
injective recipe and proves the auto-key FROM generator injectivity
(its `derive_inj` law) — idempotent re-derivation replacing the
collision hazard. That refinement is primer's, not bumbledb's:
bumbledb ids are writable-by-default — explicit values are legal on
the normal write path (`resupply_legal_monotone`; the correcting
write `delete old; insert new with the same id` is the load-bearing
idiom) — so NO generator law could carry the FD: a host can write any
id it likes, and two explicit writes can collide. The auto-
materialized statement `R(field) -> R` is therefore an ORDINARY
judged statement: enforcement is the final-state judgment
(`materialized_key_ordinary`, `materialized_key_is_functionality`),
the mint is a convenience that never re-issues an OBSERVED id, and
the two concerns never meet.

## The gravestones — law text (the creation quarantine, restated)

* **`fresh` never appears in a rule head.** No minting term exists in
  the query IR — `Query.Term` has no mint constructor and heads are
  projected variables; the mint lives here, on the write path, at
  Level 2. Unrepresentable today, permanent law.
* **No arithmetic appears in a rule head.** The measure is the one
  arithmetic the denotation defines, and its legal positions are
  boundary-only — one side of an order comparison, never a binding,
  never a head (`Query.Rule.WellTyped`, `Query/Syntax.lean`).
  Unrepresentable today, permanent law.
* When recursion lands, its safety roster (`MeasureInRecursiveHead`
  and kin) is this same creation-quarantine law restated for fixpoint
  topology, not a new rule: value invention inside a fixpoint is the
  Turing-completeness door, and it stays shut
  (`docs/architecture/20-query-ir.md` § the creation quarantine).

## Narrowings recorded (law 5: narrow and record)

* **Ids are `Nat`.** The engine's ids are u64 words; the width — and
  the typed exhaustion at the ceiling (`FreshExhausted`; an explicit
  maximal value legally exhausting the generator) — is representation
  mechanism. The model keeps the mark's evolution and what is
  observable.
* **One `Mint` is ONE (relation, field) sequence.** The per-field
  family is pointwise — sequences never interact ("fresh ids order
  within their relation and nowhere else", `10-data-model.md`) — so
  the model quantifies over one.
* **Deletes are unrepresentable mint events.** No event retreats the
  mark — deletes never touch the sequence — which is exactly why
  re-supplying a deleted id is the ordinary `supply` and why
  never-reissue needs no delete cases.
* **The lazy committed-mark read and the dirty-mark flush are
  mechanism** (`storage/delta/alloc.rs::fresh_mark` reads once per
  transaction; commit writes only advanced marks, no-op commits
  included). The model's `Mint.run` / `Reachable.commit` keep their
  semantic content: in-transaction visibility of a transaction's own
  allocations, and persistence of exactly the final mark.
-/

namespace Bumbledb
namespace Txn
namespace Fresh

/-! ## The mint — one high-water mark -/

/-- One fresh sequence's committed state: the high-water mark — the
next id the generator would return; every id ever observable sits
strictly below it. A sequence that has never issued reads `0`.
Bridge: the committed next-value the write path reads once per
transaction (`storage/delta/alloc.rs::fresh_mark`). -/
structure Mint where
  /-- The next id to issue. -/
  next : Nat
deriving DecidableEq

/-- One in-transaction event against the sequence: `alloc` — the
generator returns the mark and advances it — or `supply v` — the host
writes the explicit value `v` (ids are writable-by-default) and the
mark advances past it. -/
inductive Event where
  /-- The generator: return `next`, advance by one.
  Bridge: `WriteDelta::alloc (storage/delta/alloc.rs)`. -/
  | alloc
  /-- An explicit-value write, legal for EVERY `v`. Bridge:
  `storage/delta/insert.rs::advance_fresh_marks` (the running
  maximum). -/
  | supply (v : Nat)

/-- One event's effect on the mark. `supply` takes the running
maximum — the mark never retreats (`step_monotone`). -/
def Mint.step (m : Mint) : Event → Mint
  | .alloc => ⟨m.next + 1⟩
  | .supply v => ⟨max m.next (v + 1)⟩

/-- A transaction's whole event run, applied in order. -/
def Mint.run (m : Mint) : List Event → Mint
  | [] => m
  | e :: es => (m.step e).run es

/-- The ids the GENERATOR returned during a run — the observable
allocations. A supplied value is the host's own, never a return. -/
def Mint.returned (m : Mint) : List Event → List Nat
  | [] => []
  | .alloc :: es => m.next :: (m.step .alloc).returned es
  | .supply v :: es => (m.step (.supply v)).returned es

/-- Every id a run makes OBSERVABLE: the generator's returns plus the
host's explicit supplies. An id can enter a committed state's fresh
field only through one of the two events, so this list is the whole
observability surface — the carrier of the strengthened never-reissue
law (`never_reissue_observable`). -/
def Mint.observed (m : Mint) : List Event → List Nat
  | [] => []
  | .alloc :: es => m.next :: (m.step .alloc).observed es
  | .supply v :: es => v :: (m.step (.supply v)).observed es

/-- A generator return is observable — `returned` selects from
`observed`. -/
theorem returned_subset_observed {m : Mint} {es : List Event} {i : Nat}
    (h : i ∈ m.returned es) : i ∈ m.observed es := by
  induction es generalizing m with
  | nil => exact nomatch h
  | cons e es ih =>
    cases e with
    | alloc =>
      rcases List.mem_cons.mp h with h | h
      · exact h ▸ List.mem_cons_self ..
      · exact List.mem_cons_of_mem _ (ih h)
    | supply v => exact List.mem_cons_of_mem _ (ih h)

/-! ## The high-water laws -/

/-- One event never retreats the mark. -/
theorem step_monotone (m : Mint) (e : Event) :
    m.next ≤ (m.step e).next := by
  cases e with
  | alloc => exact Nat.le_succ m.next
  | supply v => exact Nat.le_max_left m.next (v + 1)

/-- A run never retreats the mark — the high-water law. -/
theorem run_monotone (m : Mint) (es : List Event) :
    m.next ≤ (m.run es).next := by
  induction es generalizing m with
  | nil => exact Nat.le_refl m.next
  | cons e es ih => exact Nat.le_trans (step_monotone m e) (ih (m.step e))

/-- Every returned id sits at or above its run's entry mark. -/
theorem returned_ge_start {m : Mint} {es : List Event} {i : Nat}
    (h : i ∈ m.returned es) : m.next ≤ i := by
  induction es generalizing m with
  | nil => exact nomatch h
  | cons e es ih =>
    cases e with
    | alloc =>
      rcases List.mem_cons.mp h with h | h
      · exact Nat.le_of_eq h.symm
      · exact Nat.le_trans (step_monotone m .alloc) (ih h)
    | supply v =>
      exact Nat.le_trans (step_monotone m (.supply v)) (ih h)

/-- Every returned id sits strictly below its run's final mark — the
half that makes a persisted mark a fence against re-issue. -/
theorem returned_lt_final {m : Mint} {es : List Event} {i : Nat}
    (h : i ∈ m.returned es) : i < (m.run es).next := by
  induction es generalizing m with
  | nil => exact nomatch h
  | cons e es ih =>
    cases e with
    | alloc =>
      rcases List.mem_cons.mp h with h | h
      · subst h
        exact Nat.lt_of_lt_of_le (Nat.lt_succ_self m.next)
          (run_monotone (m.step .alloc) es)
      · exact ih h
    | supply v => exact ih h

/-- Every OBSERVABLE id — returned or explicitly supplied — sits
strictly below its run's final mark: `supply v` advances the mark
past `v` exactly as `alloc` advances it past the return. The persisted
mark fences everything observable, not just the generator's returns. -/
theorem observed_lt_final {m : Mint} {es : List Event} {i : Nat}
    (h : i ∈ m.observed es) : i < (m.run es).next := by
  induction es generalizing m with
  | nil => exact nomatch h
  | cons e es ih =>
    cases e with
    | alloc =>
      rcases List.mem_cons.mp h with h | h
      · subst h
        exact Nat.lt_of_lt_of_le (Nat.lt_succ_self m.next)
          (run_monotone (m.step .alloc) es)
      · exact ih h
    | supply v =>
      rcases List.mem_cons.mp h with h | h
      · subst h
        exact Nat.lt_of_lt_of_le
          (Nat.lt_of_lt_of_le (Nat.lt_succ_self i)
            (Nat.le_max_right m.next (i + 1)))
          (run_monotone (m.step (.supply i)) es)
      · exact ih h

/-! ## The committed lifecycle -/

/-- The committed mint lifecycle. A SUCCESSFUL commit persists its
run's final mark; an ABORTED transaction's run is consumed and
discarded whole — the constructor takes the events and moves nothing,
which is the law "an aborted transaction's allocations vanish;
nothing it minted was observably returned" as a transition shape.
Bridge: `storage/delta/accessors.rs::dirty_fresh_marks` (commit
flushes advanced marks); aborted write transactions drop the delta
whole. -/
inductive Reachable : Mint → Mint → Prop where
  /-- The trivial chain. -/
  | refl (m : Mint) : Reachable m m
  /-- One more SUCCESSFUL transaction: its final mark persists — even
  when no facts changed (the escaped ids survive a no-op commit). -/
  | commit {m₀ m₁ : Mint} (es : List Event) :
      Reachable m₀ m₁ → Reachable m₀ (m₁.run es)
  /-- One ABORTED transaction: its whole run is discarded. -/
  | abort {m₀ m₁ : Mint} (es : List Event) :
      Reachable m₀ m₁ → Reachable m₀ m₁

/-- The committed mark never retreats across the lifecycle. -/
theorem reachable_monotone {m m' : Mint} (h : Reachable m m') :
    m.next ≤ m'.next := by
  induction h with
  | refl => exact Nat.le_refl _
  | commit es _ ih => exact Nat.le_trans ih (run_monotone _ es)
  | abort _ _ ih => exact ih

/-! ## (a) Never-reissue-observable -/

/-- **Never-reissue-observable — the strengthened law.** ANY id a
committed transaction made observable — generator-returned OR
explicitly supplied (an id can enter a committed state's fresh field
only through the two events) — is never returned by any transaction
minting from any later reachable mark: every observable id sits below
the persisted mark (`observed_lt_final`), the mark never retreats
(`reachable_monotone`), and every later return sits at or above its
own entry mark (`returned_ge_start`). This is the doc sentence "never
re-issuing any value observable in a committed state" made a theorem
whole (`docs/architecture/10-data-model.md` § fields);
`never_reissue_observed` is its generator-returns projection. Bridge:
`WriteDelta::alloc (crates/bumbledb/src/storage/delta/alloc.rs)` +
`advance_fresh_marks (crates/bumbledb/src/storage/delta/insert.rs)`. -/
theorem never_reissue_observable {m : Mint} {es : List Event} {i : Nat}
    (h : i ∈ m.observed es) {m' : Mint}
    (hr : Reachable (m.run es) m') (es' : List Event) :
    i ∉ m'.returned es' := by
  intro h'
  exact absurd (returned_ge_start h')
    (Nat.not_le.mpr (Nat.lt_of_lt_of_le (observed_lt_final h)
      (reachable_monotone hr)))

/-- **Never-reissue-observed** — the generator-returns projection of
`never_reissue_observable`: an id the generator returned inside a
COMMITTED transaction is never returned again from any later reachable
mark. Aborted transactions are exempt by construction —
`Reachable.abort` discards its run, and nothing it returned was
observable. Bridge: `WriteDelta::alloc
(crates/bumbledb/src/storage/delta/alloc.rs)` — "aborted transactions
never touch the committed sequence". -/
theorem never_reissue_observed {m : Mint} {es : List Event} {i : Nat}
    (h : i ∈ m.returned es) {m' : Mint}
    (hr : Reachable (m.run es) m') (es' : List Event) :
    i ∉ m'.returned es' :=
  never_reissue_observable (returned_subset_observed h) hr es'

/-! ## (b) Explicit re-supply -/

/-- **Explicit re-supply is legal and monotone.** `supply` accepts ANY
value — a previously deleted id included: deletes never touch the
sequence, so re-supplying one is the ordinary event — and the mark
never retreats and lands strictly past the supplied value. The
correcting write `delete old; insert new with the same id` rides
this. Bridge: `storage/delta/insert.rs::advance_fresh_marks` —
"explicit values are legal on the normal write path". -/
theorem resupply_legal_monotone (m : Mint) (v : Nat) :
    m.next ≤ (m.step (.supply v)).next ∧
      v < (m.step (.supply v)).next :=
  ⟨Nat.le_max_left m.next (v + 1),
   Nat.lt_of_lt_of_le (Nat.lt_succ_self v)
     (Nat.le_max_right m.next (v + 1))⟩

/-- A supplied value is never subsequently RETURNED by the generator:
the mark moved strictly past it and marks never retreat — an explicit
write spends its id for the generator forever. -/
theorem supplied_never_returned (m : Mint) (v : Nat)
    (es : List Event) : v ∉ (m.step (.supply v)).returned es := by
  intro h
  exact absurd (returned_ge_start h)
    (Nat.not_le.mpr (resupply_legal_monotone m v).2)

/-! ## (c) The auto-key rides the ordinary discipline -/

/-- **The materialized statement, judged ordinarily.** The auto-
materialized key `R(field) -> R` is an ordinary functionality
statement of the theory, so in every committed state its judgment
holds by ONE `holds` projection — the materialized-statement doctrine
(`10-data-model.md` § fields: the statement, not the generator, owns
the invariant). NOT generator injectivity: ids are writable-by-
default, so no generator law could carry the FD (module doc — why
primer's route is not ours). Bridge:
`SchemaDescriptor::materialized_statements (crates/bumbledb/src/schema.rs)`
— one auto-Functionality per fresh field, first in materialized
statement order, targetable like any declared key. -/
theorem materialized_key_ordinary {T : Theory} (s : State T)
    {R : RelId} {i : FieldId}
    (hdecl : Statement.functionality R [i] ∈ T.statements) :
    (Statement.functionality R [i]).judgment T s.inst :=
  s.models _ hdecl

/-- The scalar reading, spent: a fresh field is a scalar (u64), so
the auto-key's judgment IS plain `Functionality` on the singleton
projection — two facts sharing a fresh id cannot coexist in any
committed state, enforced by judgment, never by the generator. -/
theorem materialized_key_is_functionality {T : Theory}
    (s : State T) {R : RelId} {i : FieldId}
    (hdecl : Statement.functionality R [i] ∈ T.statements)
    (hscalar : T.header.isInterval R i = false) :
    Functionality (T.den s.inst R) [i] := by
  have hj := materialized_key_ordinary s hdecl
  have hnone : T.header.intervalSplit R [i] = none :=
    T.header.intervalSplit_scalar R [i] fun j hjm => by
      rw [List.mem_singleton] at hjm
      rw [hjm]
      exact hscalar
  simp only [Statement.judgment, hnone] at hj
  exact hj

end Fresh
end Txn
end Bumbledb
