/-!
# Staged relation expressions — the successor composition model

Chapter 12's typed nonrecursive relation-expression graph, as a small
abstract model (chapter 13 §C): stages evaluate in an acyclic order,
each reading only EARLIER stages' complete outputs and producing a
complete row set or a semantic error. Nonrecursive derived stages may
expose aggregate/computed rows to later stages — the successor
replaces the old projection-only `Interior` premise for nonrecursive
composition (`Query/Syntax.lean` remains the retained rule-level IR;
citing its projection-only interiors as evidence about
aggregate-derived stages is exactly what this module exists to stop).

The theorems:

* **A stage reads only its declared inputs** (`runStage_congr`):
  environments agreeing at the read indices produce one outcome.
* **Later stages cannot affect earlier outcomes** (`evalFrom_stable`).
* **A required producer's error surfaces** (`consumer_of_error`): a
  stage reading an erroring producer errors — a downstream filter
  cannot suppress a required upstream error, by construction and by
  theorem.
* **Unreferenced stages are invisible** (`evalFrom_agree_except`,
  `unread_stage_invisible`): replacing a stage no later stage reads
  changes no other outcome — a named expression does not force
  materialization, and an unreferenced definition need not execute.
* **Inlining preserves values AND errors** (`inline_runStage`): a
  consumer of one produced table equals the composed stage that
  evaluates the producer inline — including the error path, so fusion
  cannot erase a producer's failure.
* **The restricted recursive node stays inside its frozen finite
  domain** (`iterate_subset_dom`): when every step selects only
  existing values of the frozen domain — which may include
  aggregate/computed PREDECESSOR outputs, frozen once, since the
  premise concerns the actual frozen values and not where a name was
  spelled — the whole iteration stays inside that finite domain. The
  countermodel `value_creation_escapes` shows one `+1` in the
  feedback cycle escaping every such domain: value creation in the
  cycle is the Turing-completeness door and stays shut.
* **Aggregate input grain is the distinct row set of the stage's
  input** — kernel-checked fixtures: projecting attempts to students
  before counting counts students; counting attempt bindings counts
  attempts; equal amounts on distinct bindings contribute separately;
  an inner projection changes the grain and NAMING does not; empty
  input forms no group.

## Narrowings recorded (law 5: narrow and record)

* **Stages are positional functions of their read tables.** The
  congruence that a stage consults nothing else is BY CONSTRUCTION
  (`Stage.eval` receives exactly the read tables), not a side
  condition to discharge.
* **Errors are one abstract case.** The engine's stage errors carry
  typed reasons (overflow, cast, measure); the model needs only
  their propagation discipline. Diagnostics are engine roster work.
* **Set semantics via `dedup`.** Row sets are deduplicated lists;
  `mem_dedup` is the carrier law. Physical representation (RAM table
  versus temporary LMDB) is invisible here by construction — which is
  the point: spilling cannot change the denotation.
* **Termination of the recursive iteration** is carried by the
  existing reach lemmas over the concrete IR (`Exec/Reach.lean`) and
  the bench models; the NEW successor obligation proved here is the
  frozen-finite-domain containment induction with aggregate/computed
  predecessors admitted.
-/

namespace Bumbledb
namespace Query
namespace Stages

/-! ## Row sets with set semantics -/

/-- Deduplicate, keeping one occurrence of each row — the set carrier.
Distinctness is the aggregate input grain. -/
def dedup {Row : Type} [DecidableEq Row] : List Row → List Row
  | [] => []
  | r :: rs => if r ∈ rs then dedup rs else r :: dedup rs

theorem mem_dedup {Row : Type} [DecidableEq Row] {r : Row} :
    ∀ {rs : List Row}, r ∈ dedup rs ↔ r ∈ rs := by
  intro rs
  induction rs with
  | nil => exact Iff.rfl
  | cons x xs ih =>
    unfold dedup
    by_cases hx : x ∈ xs
    · rw [if_pos hx, ih]
      constructor
      · exact List.mem_cons_of_mem x
      · intro h
        rcases List.mem_cons.mp h with rfl | h
        · exact hx
        · exact h
    · rw [if_neg hx]
      constructor
      · intro h
        rcases List.mem_cons.mp h with rfl | h
        · exact List.mem_cons_self
        · exact List.mem_cons_of_mem x (ih.mp h)
      · intro h
        rcases List.mem_cons.mp h with rfl | h
        · exact List.mem_cons_self
        · exact List.mem_cons_of_mem x (ih.mpr h)

/-! ## Stages and the acyclic graph -/

/-- One stage: the indices of the earlier stages it reads, and a total
function from exactly those tables to a complete row set or a semantic
error. The function receives ONLY the read tables — a stage cannot
consult anything it did not declare, by construction. -/
structure Stage (Row : Type) where
  /-- Indices of earlier stages this stage reads. -/
  reads : List Nat
  /-- The stage meaning: complete input tables to a complete output
  set or a semantic error. Aggregate and computed outputs are ordinary
  such functions — a derived stage's rows are ordinary rows to its
  consumers, with finalized scalars, never a hidden accumulator. -/
  eval : List (List Row) → Except Unit (List Row)

/-- One outcome per stage: a complete row set or an error. -/
abbrev Outcome (Row : Type) := Except Unit (List Row)

variable {Row : Type}

/-- Collect the read tables from the environment: `none` when a read
is out of range (the validator refuses this shape) or when a required
producer errored. -/
def readTables (env : List (Outcome Row)) : List Nat →
    Option (List (List Row))
  | [] => some []
  | j :: js =>
    match env[j]? with
    | some (.ok rows) => (readTables env js).map (rows :: ·)
    | _ => none

/-- Run one stage against the prefix environment: a required
producer's error (or an out-of-range read) is this stage's error —
its own evaluation never runs on incomplete input. -/
def runStage (env : List (Outcome Row)) (s : Stage Row) : Outcome Row :=
  match readTables env s.reads with
  | some tables => s.eval tables
  | none => .error ()

/-- Evaluate a stage list left to right, extending the environment —
acyclicity by construction: a stage sees exactly its predecessors. -/
def evalFrom (env : List (Outcome Row)) :
    List (Stage Row) → List (Outcome Row)
  | [] => env
  | s :: rest => evalFrom (env ++ [runStage env s]) rest

/-- The whole graph's outcomes. -/
def evalGraph (g : List (Stage Row)) : List (Outcome Row) :=
  evalFrom [] g

/-! ## Structural laws -/

theorem evalFrom_append (env : List (Outcome Row))
    (g₁ g₂ : List (Stage Row)) :
    evalFrom env (g₁ ++ g₂) = evalFrom (evalFrom env g₁) g₂ := by
  induction g₁ generalizing env with
  | nil => rfl
  | cons s g₁ ih => exact ih (env ++ [runStage env s])

/-- Later stages cannot affect earlier outcomes: evaluation only
appends. -/
theorem evalFrom_stable (g : List (Stage Row)) :
    ∀ (env : List (Outcome Row)) {i : Nat}, i < env.length →
      (evalFrom env g)[i]? = env[i]? := by
  induction g with
  | nil => intro env i _; rfl
  | cons s rest ih =>
    intro env i hi
    have hstep := ih (env ++ [runStage env s])
      (i := i) (by simp; omega)
    rw [show evalFrom env (s :: rest) =
      evalFrom (env ++ [runStage env s]) rest from rfl, hstep]
    exact List.getElem?_append_left hi

theorem evalFrom_length (g : List (Stage Row)) :
    ∀ env : List (Outcome Row),
      (evalFrom env g).length = env.length + g.length := by
  induction g with
  | nil => intro env; simp [evalFrom]
  | cons s rest ih =>
    intro env
    show (evalFrom (env ++ [runStage env s]) rest).length = _
    rw [ih]
    simp
    omega

/-- The outcome at one stage's own index is that stage run against its
complete prefix environment. -/
theorem evalGraph_at (g₁ : List (Stage Row)) (s : Stage Row)
    (g₂ : List (Stage Row)) :
    (evalGraph (g₁ ++ s :: g₂))[g₁.length]? =
      some (runStage (evalGraph g₁) s) := by
  unfold evalGraph
  rw [evalFrom_append]
  have hlen : (evalFrom ([] : List (Outcome Row)) g₁).length =
      g₁.length := by
    rw [evalFrom_length]
    exact Nat.zero_add _
  show (evalFrom (evalFrom [] g₁) (s :: g₂))[g₁.length]? = _
  rw [show evalFrom (evalFrom [] g₁) (s :: g₂) =
    evalFrom (evalFrom [] g₁ ++ [runStage (evalFrom [] g₁) s]) g₂
    from rfl]
  rw [evalFrom_stable g₂ _ (by simp [hlen])]
  rw [List.getElem?_append_right (by omega)]
  rw [hlen]
  simp

/-! ## Reading only declared inputs -/

/-- `readTables` consults exactly the read indices. -/
theorem readTables_congr {env env' : List (Outcome Row)}
    (js : List Nat) (h : ∀ j ∈ js, env[j]? = env'[j]?) :
    readTables env js = readTables env' js := by
  induction js with
  | nil => rfl
  | cons j js ih =>
    unfold readTables
    rw [h j List.mem_cons_self,
      ih fun j hj => h j (List.mem_cons_of_mem _ hj)]

/-- A stage's outcome is a function of its declared reads alone. -/
theorem runStage_congr {env env' : List (Outcome Row)} (s : Stage Row)
    (h : ∀ j ∈ s.reads, env[j]? = env'[j]?) :
    runStage env s = runStage env' s := by
  unfold runStage
  rw [readTables_congr s.reads h]

/-! ## Producer errors surface -/

/-- Reading an erroring producer is an error — one read suffices. -/
theorem runStage_error_of_read {env : List (Outcome Row)}
    (s : Stage Row) {j : Nat} (hj : j ∈ s.reads)
    (herr : env[j]? = some (.error ())) :
    runStage env s = .error () := by
  unfold runStage
  suffices h : readTables env s.reads = none by rw [h]
  generalize s.reads = js at hj ⊢
  induction js with
  | nil => exact nomatch hj
  | cons k ks ih =>
    unfold readTables
    rcases List.mem_cons.mp hj with rfl | hk
    · rw [herr]
    · cases hk' : env[k]? with
      | none => rfl
      | some o =>
        cases o with
        | error e => rfl
        | ok rows => rw [ih hk]; rfl

/-- **A later consumer cannot hide a required producer's error**: if
stage `k = g₁.length` (the producer) errors and the consumer at index
`g₁.length + 1 + g₂.length` reads it, the consumer errors — stated
through the graph so filters, projections and any other consumer
logic are quantified over. -/
theorem consumer_of_error (g₁ : List (Stage Row)) (p : Stage Row)
    (g₂ : List (Stage Row)) (c : Stage Row) (g₃ : List (Stage Row))
    (hread : g₁.length ∈ c.reads)
    (herr : (evalGraph (g₁ ++ p :: (g₂ ++ c :: g₃)))[g₁.length]? =
      some (.error ())) :
    (evalGraph (g₁ ++ p :: (g₂ ++ c :: g₃)))[(g₁ ++ p :: g₂).length]? =
      some (.error ()) := by
  have hsplit : g₁ ++ p :: (g₂ ++ c :: g₃) =
      (g₁ ++ p :: g₂) ++ c :: g₃ := by
    simp
  rw [hsplit] at herr ⊢
  rw [evalGraph_at]
  have hprefix_len : (evalGraph (g₁ ++ p :: g₂)).length =
      (g₁ ++ p :: g₂).length := by
    unfold evalGraph
    rw [evalFrom_length]
    exact Nat.zero_add _
  have hlt : g₁.length < (g₁ ++ p :: g₂).length := by
    simp
  have hpre : (evalGraph (g₁ ++ p :: g₂))[g₁.length]? =
      some (.error ()) := by
    have hstab := evalFrom_stable (c :: g₃)
      (evalGraph (g₁ ++ p :: g₂)) (i := g₁.length)
      (by rw [hprefix_len]; exact hlt)
    rw [← hstab]
    show (evalFrom (evalFrom [] (g₁ ++ p :: g₂)) (c :: g₃))[g₁.length]? = _
    rw [← evalFrom_append]
    exact herr
  rw [runStage_error_of_read c hread hpre]

/-! ## Unreferenced stages are invisible -/

/-- Environments agreeing everywhere except one index nobody reads
evaluate to outcomes agreeing everywhere except that index. -/
theorem evalFrom_agree_except (bad : Nat) :
    ∀ (g : List (Stage Row)) (env env' : List (Outcome Row)),
      env.length = env'.length →
      (∀ i, i ≠ bad → env[i]? = env'[i]?) →
      (∀ t ∈ g, bad ∉ t.reads) →
      ∀ i, i ≠ bad → (evalFrom env g)[i]? = (evalFrom env' g)[i]?
  | [], env, env', _, hagree, _, i, hi => hagree i hi
  | t :: g, env, env', hlen, hagree, hreads, i, hi => by
    have hrun : runStage env t = runStage env' t :=
      runStage_congr t fun j hj =>
        hagree j fun hbad => hreads t List.mem_cons_self (hbad ▸ hj)
    have hlen' : (env ++ [runStage env t]).length =
        (env' ++ [runStage env' t]).length := by
      simp [hlen]
    have hagree' : ∀ i, i ≠ bad →
        (env ++ [runStage env t])[i]? =
          (env' ++ [runStage env' t])[i]? := by
      intro i hi
      by_cases hlt : i < env.length
      · rw [List.getElem?_append_left hlt,
          List.getElem?_append_left (by omega : i < env'.length)]
        exact hagree i hi
      · by_cases heq : i = env.length
        · subst heq
          rw [List.getElem?_append_right (Nat.le_refl _),
            List.getElem?_append_right
              (by omega : env'.length ≤ env.length)]
          rw [hlen, Nat.sub_self, hrun]
        · have hgt : env.length < i := by omega
          rw [List.getElem?_append_right (by omega),
            List.getElem?_append_right (by omega)]
          rw [hlen]
          have h1 : [runStage env t][i - env'.length]? = none := by
            refine List.getElem?_eq_none ?_
            simp
            omega
          have h2 : [runStage env' t][i - env'.length]? = none := by
            refine List.getElem?_eq_none ?_
            simp
            omega
          rw [h1, h2]
      -- both none when past the appended element
    exact evalFrom_agree_except bad g _ _ hlen' hagree'
      (fun t' ht' => hreads t' (List.mem_cons_of_mem t ht')) i hi

/-- **Replacing a stage no later stage reads changes no other
outcome** — an unreferenced definition need not execute, and a name
is not a materialization command. -/
theorem unread_stage_invisible (g₁ : List (Stage Row))
    (s s' : Stage Row) (g₂ : List (Stage Row))
    (hunread : ∀ t ∈ g₂, g₁.length ∉ t.reads) :
    ∀ i, i ≠ g₁.length →
      (evalGraph (g₁ ++ s :: g₂))[i]? =
        (evalGraph (g₁ ++ s' :: g₂))[i]? := by
  intro i hi
  unfold evalGraph
  rw [evalFrom_append, evalFrom_append]
  show (evalFrom (evalFrom [] g₁) (s :: g₂))[i]? = _
  rw [show evalFrom (evalFrom [] g₁) (s :: g₂) =
    evalFrom (evalFrom [] g₁ ++ [runStage (evalFrom [] g₁) s]) g₂
    from rfl]
  rw [show evalFrom (evalFrom [] g₁) (s' :: g₂) =
    evalFrom (evalFrom [] g₁ ++ [runStage (evalFrom [] g₁) s']) g₂
    from rfl]
  have hlen : (evalFrom ([] : List (Outcome Row)) g₁).length =
      g₁.length := by
    rw [evalFrom_length]
    exact Nat.zero_add _
  refine evalFrom_agree_except g₁.length g₂ _ _ (by simp) ?_ hunread i hi
  intro k hk
  by_cases hklt :
      k < (evalFrom ([] : List (Outcome Row)) g₁).length
  · rw [List.getElem?_append_left hklt,
      List.getElem?_append_left hklt]
  · have : g₁.length < k := by omega
    rw [List.getElem?_append_right (by omega),
      List.getElem?_append_right (by omega)]
    have h1 : [runStage (evalFrom [] g₁) s][k -
        (evalFrom ([] : List (Outcome Row)) g₁).length]? = none := by
      refine List.getElem?_eq_none ?_
      simp
      omega
    have h2 : [runStage (evalFrom [] g₁) s'][k -
        (evalFrom ([] : List (Outcome Row)) g₁).length]? = none := by
      refine List.getElem?_eq_none ?_
      simp
      omega
    rw [h1, h2]

/-! ## Inlining preserves values and errors -/

/-- The inlined composition: evaluate the producer's function on its
own reads, then feed the consumer — value and error paths both. -/
def inlineStage (p : Stage Row)
    (consume : List Row → Except Unit (List Row)) : Stage Row :=
  ⟨p.reads, fun tables => (p.eval tables).bind consume⟩

/-- **Inlining preserves the outcome, including errors**: a consumer
reading exactly the producer's table equals the inlined composed
stage — fusion can avoid storing the intermediate rows but cannot
erase the producer's error boundary or change its complete value. -/
theorem inline_runStage (env : List (Outcome Row)) (p : Stage Row)
    (k : Nat) (consume : List Row → Except Unit (List Row))
    (hk : env[k]? = some (runStage env p)) :
    runStage env ⟨[k], fun tables => consume (tables.headD [])⟩ =
      runStage env (inlineStage p consume) := by
  show (match readTables env [k] with
    | some tables => consume (tables.headD [])
    | none => .error ()) =
    (match readTables env p.reads with
    | some tables => (p.eval tables).bind consume
    | none => .error ())
  cases hp : readTables env p.reads with
  | none =>
    have : runStage env p = .error () := by
      unfold runStage
      rw [hp]
    rw [this] at hk
    have hread : readTables env [k] = none := by
      unfold readTables
      rw [hk]
    rw [hread]
  | some tables =>
    have hrun : runStage env p = p.eval tables := by
      unfold runStage
      rw [hp]
    rw [hrun] at hk
    cases he : p.eval tables with
    | error e =>
      cases e
      rw [he] at hk
      have hread : readTables env [k] = none := by
        unfold readTables
        rw [hk]
      rw [hread]
      show Except.error () = (p.eval tables).bind consume
      rw [he]
      rfl
    | ok rows =>
      rw [he] at hk
      have hread : readTables env [k] = some [rows] := by
        unfold readTables
        rw [hk]
        rfl
      rw [hread]
      show consume rows = (p.eval tables).bind consume
      rw [he]
      rfl

/-! ## The restricted recursive node: frozen finite domain -/

/-- One semi-naive-shaped round: keep what is derived, add the step's
selections, deduplicate. -/
def roundStep [DecidableEq Row] (f : List Row → List Row)
    (s : List Row) : List Row :=
  dedup (s ++ f s)

/-- Iterate the recursive step a bounded number of rounds. -/
def iterate [DecidableEq Row] (f : List Row → List Row) :
    Nat → List Row → List Row
  | 0, s => s
  | n + 1, s => iterate f n (roundStep f s)

/-- Growth: every derived row survives every later round. -/
theorem iterate_grows [DecidableEq Row] (f : List Row → List Row)
    (n : Nat) (s : List Row) {r : Row} (h : r ∈ s) :
    r ∈ iterate f n s := by
  induction n generalizing s with
  | zero => exact h
  | succ n ih =>
    show r ∈ iterate f n (roundStep f s)
    refine ih _ ?_
    unfold roundStep
    exact mem_dedup.mpr (List.mem_append.mpr (Or.inl h))

/-- **The frozen-finite-domain containment induction**: when the base
selects from the frozen domain and every step over a domain-contained
state selects only domain values — projection-only recursive heads
over frozen inputs, which may include aggregate/computed PREDECESSOR
outputs — the whole iteration stays inside that finite domain. The
premise concerns the actual frozen values, not where a name was
spelled in source. -/
theorem iterate_subset_dom [DecidableEq Row] (f : List Row → List Row)
    (dom : List Row)
    (hf : ∀ s : List Row, (∀ r ∈ s, r ∈ dom) →
      ∀ r ∈ f s, r ∈ dom) :
    ∀ (n : Nat) (s : List Row), (∀ r ∈ s, r ∈ dom) →
      ∀ r ∈ iterate f n s, r ∈ dom := by
  intro n
  induction n with
  | zero => intro s hs r hr; exact hs r hr
  | succ n ih =>
    intro s hs r hr
    refine ih (roundStep f s) ?_ r hr
    intro x hx
    have hx' := mem_dedup.mp hx
    rcases List.mem_append.mp hx' with hx' | hx'
    · exact hs x hx'
    · exact hf s hs x hx'

/-- **Value creation in the feedback cycle escapes every frozen
domain** — the countermodel: one `+1` in the step manufactures a row
outside the domain after enough rounds. The recursive fragment
excludes aggregation/arithmetic/value creation from the cycle for
exactly this reason. -/
theorem value_creation_escapes :
    ∃ r ∈ iterate (fun s => s.map (· + 1)) 4 [0], r ∉ [0, 1, 2, 3] :=
  ⟨4, by decide, by decide⟩

/-! ## Aggregate input grain — kernel-checked fixtures -/

/-- Count the distinct rows — every aggregate folds the distinct
binding set of its input. -/
def countDistinct [DecidableEq Row] (rows : List Row) : Nat :=
  (dedup rows).length

/-- Sum a projected `Nat` measure over the DISTINCT input rows. -/
def sumOver [DecidableEq Row] (rows : List Row) (f : Row → Nat) : Nat :=
  ((dedup rows).map f).foldr (· + ·) 0

-- Attempts (attempt-id, student): counting attempt BINDINGS counts
-- attempts; projecting to student FIRST (a new deduplicated relation)
-- counts students — an explicit projection changes the grain.
#guard countDistinct [(1, 10), (2, 10), (3, 20)] = 3
#guard countDistinct ([(1, 10), (2, 10), (3, 20)].map Prod.snd) = 2
-- Naming changes neither: an identity stage around either expression
-- is invisible (`unread_stage_invisible` is the general theorem; the
-- identity map is the fixture).
#guard countDistinct (([(1, 10), (2, 10), (3, 20)].map id)) = 3
-- Equal amounts on DISTINCT bindings contribute separately; projecting
-- identity away first deliberately leaves one distinct row.
#guard sumOver [(1, 5), (2, 5)] Prod.snd = 10
#guard sumOver ([(1, 5), (2, 5)].map Prod.snd) id = 5
-- No input rows, no group: the aggregate of empty input is no answer
-- row (the schema capacity's existing-parent empty-child ZERO total
-- is the deliberately different law — `Capacity.lean`).
#guard countDistinct ([] : List Nat) = 0
#guard dedup ([] : List Nat) = []

/-- The producer-error fixture as a concrete three-stage graph: the
producer errors (an overflowing aggregate), the consumer filters
everything out — and still errors. A filter cannot un-require a
required producer. -/
def errorGraph : List (Stage Nat) :=
  [⟨[], fun _ => .ok [1, 2, 3]⟩,
   ⟨[0], fun _ => .error ()⟩,
   ⟨[1], fun tables => .ok ((tables.headD []).filter fun _ => false)⟩]

#guard match (evalGraph errorGraph)[2]? with
  | some (Except.error ()) => true
  | _ => false
-- The same consumer over a HEALTHY producer succeeds with the empty
-- set: the filter itself is not the error.
#guard match (evalGraph
    [⟨[], fun _ => .ok [1, 2, 3]⟩,
     ⟨[0], fun tables => .ok (tables.headD [])⟩,
     ⟨[1], fun tables => .ok ((tables.headD []).filter fun _ => false)⟩])[2]? with
  | some (Except.ok []) => true
  | _ => false

end Stages
end Query
end Bumbledb
