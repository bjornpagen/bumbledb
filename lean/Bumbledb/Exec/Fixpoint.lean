import Bumbledb.Query.Denotation

/-!
# Exec/Fixpoint — the stratified fixpoint, denoted and executable

Engine recursion's spec model, landing in `Exec/` beside the proved
rewrites — the prepared home `docs/architecture/20-query-ir.md`
promised ("the stratified semantics lands beside the proved rewrites,
never in a new tree"). One file carries both refinement levels of the
one feature, recorded deliberately: **Level 0** is the stratified
denotation (`programDen` — per-stratum least fixpoints by strong
induction on the stratum index, over `Query/Syntax.lean`'s program
cut), and **Level 1** is the fueled round loop (`evalProgram` — round
= evaluate every rule against the current predicate tables, union,
stop on no change) proved sound AND complete against Level 0
(`program_eval_sound`). The blueprint is
`docs/reference/recursion-design.md`; §1 (the IR cut), §2
(stratification and the safety theorem), §3 (the delta rewrite's
operator-level face), and §5 (the driver's round loop) land here; §4
and the rest of §5 (delta images, watermarks, budgets, plan variants)
are mechanism and stay in the docs, whole — the mechanism fence.

## The shape of the semantics

* **Predicates carry answer-tuple sets** (`PredSets`), and an `idb`
  atom matches a tuple through `tupleFact` — `FieldId i` reads head
  position `i`, the positional reading the syntax promised.
  Out-of-arity positions read `fillerValue` (the conformance lane's
  filler, never readable by an accepted program — the arity roster is
  validator mechanism).
* **The per-stratum operator** (`stratumOp`) reads finished lower
  strata as parameters and the working stratum through its argument;
  its monotonicity (`stratumOp_mono`) carries the stratification
  premise, because negated occurrences must read FINISHED sets — this
  is exactly why stratification is the fence, and
  `Countermodels.odd_not_monotone` is the wall on the other side.
* **The least fixpoint is Knaster–Tarski over an impredicative
  intersection** (`lfpP`, `lfpP_fixed`) — elementary, core-Lean, no
  positivity gymnastics: the inductive-predicate formulation would
  need the stratification premise to even elaborate, and the
  intersection form makes that premise a THEOREM input instead of a
  kernel obligation.
* **The stratified denotation is witness-relative** (`programDen`
  takes the `strat` witness). Narrowing recorded: independence of the
  denotation from the choice of stratification witness is the
  classical fact NOT restated here — the design's validation computes
  ONE witness (SCC condensation, mechanism — queued with the engine
  discharge), and the prepared recursive arm (`evalProgram`, no
  consumer yet) evaluates under the witness it is handed.
* **The unknown-PredId gap and its screen** (the loud record is
  `Query/Syntax.lean`'s module doc): an out-of-range `idb` read
  denotes the empty fact set here — `sourceDen`'s phantom default —
  so a negated phantom read is vacuously satisfied, and
  `StratifiedBy` never refuses the shape. The agreement theorems
  (`evalProgramAt_den`, `program_eval_sound`, `program_den_finite`)
  are exact equalities WITHOUT the screen — the evaluator and the
  denotation read the same phantom-empty set — which is why they do
  not carry `Program.WellFormed` as a premise; the screen exists
  (`Program.WellFormed`, spent by `wellFormed_reads_real`) for
  acceptance readings, and the engine-side refusal is validation's
  roster item (`docs/reference/recursion-design.md` §1), queued with
  the engine discharge.

## The safety theorem, made formal (§2)

Heads project bound variables only (`finds : List VarId` — value
creation is unrepresentable), so every derived value is drawn from
stored columns or lower predicates: `pevalRule_dom` walks
`antijoin_over_active_domain` through the source coding, every
predicate is a subset of a finite product over `progDom` (the active
domain plus the filler — `stratumCands`), and the fueled loop reaches
the least fixpoint within candidate-count rounds
(`fueledLoop_fixpoint`; the fuel bound is a LEMMA — `missingCount_le`
— not a hope). `program_den_finite` is the theorem: on finite
instances every predicate's fixpoint is finite and `evalProgram`
lists it. `Countermodels.succ_prefixed_infinite` is the wall when the
premise falls: a head-creating (successor-style) operator ascends
forever.

## The semi-naive face (§3, operator level)

`semi_naive_agrees`: iterating on `new = T(acc) \ acc` walks exactly
the naive chain — the union algebra, no Free Join vocabulary, no
delta-variant plans (those are §3's mechanism, docs-side). Monotone
`T` is what makes the shared chain reach the least fixpoint
(`fueledLoop_fixpoint` is the executable form of that fact).

## The chain-window fence and the gravestones (§8; law text)

* **Value creation in a recursive head exits the safety theorem AT
  ITS PREMISE** — `w = w₁ ∩ w₂` in a head is not a bound variable,
  and the design's §8 fences the whole chain-window class outside
  this model. Lattice-closed, endpoint-SELECTING operations are the
  only future candidates; endpoint-inventing ones (shift, widen,
  arithmetic on bounds) are refused categorically.
* **The creation-quarantine gravestones hold in the program cut
  verbatim** (`Query/Syntax.lean`'s module doc): `fresh` never
  appears in a rule head and no arithmetic appears in a rule head —
  both unrepresentable in `PRule`, so the successor countermodel
  lives at the OPERATOR level, outside the syntax, where it belongs.
* **`MAX_PREDICATES`/`MAX_RULES` are boundary guards** — mechanism,
  noted, not modeled.
* **Statements never reference predicates**: no statement form
  carries a `PredId` position (`Query/Syntax.lean`, the program cut's
  module doc) — the stored-relations decision by unrepresentability.

## The coding transport (Level-1 device, recorded)

The executable evaluator reuses `evalRule`/`eval_sound` WHOLE through
an even/odd source coding (`AtomSource.code`): a program rule over
sources is a plain rule over coded relation ids, and `pderives_code`
carries the denotation across. The coding is a proof-reuse device,
never a semantic claim — `pderives` is the honest Level-0 judgment,
stated first, and the coding lemma is proved against it.
-/

namespace Bumbledb.Query

/-! ## Carriers — predicate tables and the tuple-fact reading -/

/-- Predicate tables: one answer-tuple set per predicate id — the
stratified denotation's carrier. -/
abbrev PredSets : Type := PredId → Set AnswerTuple

/-- Pointwise membership order on predicate tables. -/
def PredSets.le (X Y : PredSets) : Prop :=
  ∀ P t, t ∈ X P → t ∈ Y P

/-- The out-of-arity filler (the conformance lane's filler value):
`tupleFact` is total, and accepted programs never read past a head's
arity — the arity roster is validator mechanism. -/
def fillerValue : Value := ⟨.bool, false⟩

/-- The fact a head tuple denotes: `FieldId i` reads head position
`i` — the positional addressing the program cut promised. -/
def tupleFact (t : AnswerTuple) : Fact :=
  fun i => (t[i.id]?).getD fillerValue

/-- A tuple-fact field is a tuple value or the filler — the
active-domain walk's `idb` leg. -/
theorem tupleFact_mem_or_filler (t : AnswerTuple) (i : FieldId) :
    tupleFact t i ∈ t ∨ tupleFact t i = fillerValue := by
  unfold tupleFact
  cases h : t[i.id]? with
  | none => exact Or.inr rfl
  | some v =>
    exact Or.inl (by
      simpa using List.mem_of_getElem? h)

/-- What an atom source denotes: a stored relation reads the
instance; a predicate reads the table through the tuple-fact
reading. -/
def sourceDen (I : Instance) (X : PredSets) : AtomSource → Set Fact
  | .edb R => I R
  | .idb P => fun f => ∃ t, t ∈ X P ∧ f = tupleFact t

/-! ## The program-level body judgment (Level 0) -/

/-- The matching equation over a program atom — `Matches`, verbatim,
on `PAtom.bindings` (the equation never read the relation position). -/
def PMatches (f : Fact) (a : PAtom) (σ : Assignment) (ρ : ParamEnv) :
    Prop :=
  ∀ b, b ∈ a.bindings → Term.selects ρ σ b.2 (f b.1)

/-- The program-level body judgment: `derives`, with every occurrence
read through an `AtomSource → Set Fact` environment — positive atoms
demand a matching fact, negated atoms are the same anti-join (`¬∃`
over the source's extension, never a complement), conditions conjoin. -/
def pderives (C : Classify) (r : PRule) (F : AtomSource → Set Fact)
    (ρ : ParamEnv) (σ : Assignment) : Prop :=
  (∀ a, a ∈ r.atoms → ∃ f, f ∈ F a.source ∧ PMatches f a σ ρ) ∧
  (∀ a, a ∈ r.negated → ¬ ∃ f, f ∈ F a.source ∧ PMatches f a σ ρ) ∧
  (∀ t, t ∈ r.conditions → Condition.holds C ρ σ t)

/-- One program rule's answers: deriving environments projected
through the finds — `ruleAnswers`, source-generalized. -/
def pruleAnswers (C : Classify) (r : PRule) (F : AtomSource → Set Fact)
    (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ σ, pderives C r F ρ σ ∧ t = r.finds.map σ

/-- The judgment reads the environment extensionally. -/
theorem pderives_congr {C : Classify} {r : PRule} {ρ : ParamEnv}
    {σ : Assignment} {F G : AtomSource → Set Fact}
    (h : ∀ s f, f ∈ F s ↔ f ∈ G s) :
    pderives C r F ρ σ ↔ pderives C r G ρ σ := by
  unfold pderives
  constructor
  · rintro ⟨hpos, hneg, hcond⟩
    refine ⟨fun a ha => ?_, fun a ha hex => ?_, hcond⟩
    · obtain ⟨f, hf, hm⟩ := hpos a ha
      exact ⟨f, (h _ f).mp hf, hm⟩
    · obtain ⟨f, hf, hm⟩ := hex
      exact hneg a ha ⟨f, (h _ f).mpr hf, hm⟩
  · rintro ⟨hpos, hneg, hcond⟩
    refine ⟨fun a ha => ?_, fun a ha hex => ?_, hcond⟩
    · obtain ⟨f, hf, hm⟩ := hpos a ha
      exact ⟨f, (h _ f).mpr hf, hm⟩
    · obtain ⟨f, hf, hm⟩ := hex
      exact hneg a ha ⟨f, (h _ f).mp hf, hm⟩

/-- Answers read the environment extensionally. -/
theorem pruleAnswers_congr {C : Classify} {r : PRule} {ρ : ParamEnv}
    {F G : AtomSource → Set Fact} (h : ∀ s f, f ∈ F s ↔ f ∈ G s) :
    ∀ t, t ∈ pruleAnswers C r F ρ ↔ t ∈ pruleAnswers C r G ρ := by
  intro t
  unfold pruleAnswers
  exact exists_congr fun σ =>
    and_congr_left fun _ => pderives_congr h

/-- The well-formedness screen, spent: on a `Program.WellFormed`
program every `idb` source any rule reads — positive or negated —
resolves to a REAL predicate definition, so the phantom-empty default
(`sourceDen` at an out-of-range id) is never exercised through
accepted syntax. The unknown-PredId gap's record is the module doc
and `Query/Syntax.lean`'s; this is the form a consumer cashes. -/
theorem wellFormed_reads_real {p : Program} (hwf : p.WellFormed)
    {r : PRule} (hr : r ∈ p.rulesList) {a : PAtom}
    (ha : a ∈ r.atoms ∨ a ∈ r.negated) {Q : PredId}
    (hsrc : a.source = .idb Q) :
    ∃ d, p.predicates[Q.id]? = some d := by
  have hlt : Q.id < p.predicates.length := hwf r hr a ha Q hsrc
  exact ⟨p.predicates[Q.id], List.getElem?_eq_getElem hlt⟩

/-! ## The per-stratum operator -/

/-- The stratum-`s` reading of the predicate tables: finished strata
read `lower`, the working stratum reads `X`, higher strata read
nothing (unreachable on stratified programs — totality filler). -/
def stratumSets (strat : PredId → Nat) (s : Nat) (lower X : PredSets) :
    PredSets :=
  fun P =>
    if strat P < s then lower P
    else if strat P = s then X P
    else fun _ => False

/-- Below the working stratum, the reading is the finished tables —
whatever the working argument holds. -/
theorem stratumSets_lt {strat : PredId → Nat} {s : Nat}
    {lower X : PredSets} {P : PredId} (h : strat P < s) :
    stratumSets strat s lower X P = lower P :=
  if_pos h

/-- At the working stratum, the reading is the argument. -/
theorem stratumSets_at {strat : PredId → Nat} {s : Nat}
    {lower X : PredSets} {P : PredId} (h : strat P = s) :
    stratumSets strat s lower X P = X P :=
  ((if_neg (by omega : ¬ strat P < s)).trans (if_pos h) :
    (if strat P < s then lower P else
      if strat P = s then X P else fun _ => False) = X P)

/-- Above the working stratum, the reading is empty (unreachable on
stratified programs — totality filler). -/
theorem stratumSets_gt {strat : PredId → Nat} {s : Nat}
    {lower X : PredSets} {P : PredId} (h1 : ¬ strat P < s)
    (h2 : ¬ strat P = s) :
    stratumSets strat s lower X P = fun _ => False :=
  ((if_neg h1).trans (if_neg h2) :
    (if strat P < s then lower P else
      if strat P = s then X P else fun _ => False) = fun _ => False)

/-- The stratum-`s` environment: sources read the instance and the
stratum reading. -/
def stratumEnv (I : Instance) (strat : PredId → Nat) (s : Nat)
    (lower X : PredSets) : AtomSource → Set Fact :=
  sourceDen I (stratumSets strat s lower X)

/-- Below the working stratum, an `idb` source reads the same set
whatever the working argument holds — the negated occurrences'
stability, which is monotonicity's whole burden. -/
theorem stratumEnv_idb_lower {I : Instance} {strat : PredId → Nat}
    {s : Nat} {lower X Y : PredSets} {Q : PredId} (hQ : strat Q < s) :
    stratumEnv I strat s lower X (.idb Q) =
      stratumEnv I strat s lower Y (.idb Q) := by
  show (fun f => ∃ t, t ∈ stratumSets strat s lower X Q ∧
      f = tupleFact t) =
    (fun f => ∃ t, t ∈ stratumSets strat s lower Y Q ∧ f = tupleFact t)
  rw [stratumSets_lt (X := X) hQ, stratumSets_lt (X := Y) hQ]

/-- The immediate-consequence operator of stratum `s`: a predicate at
stratum `s` holds a tuple when one of its rules derives it under the
stratum environment — lower strata finished parameters, the working
stratum the recursive argument. -/
def stratumOp (C : Classify) (p : Program) (strat : PredId → Nat)
    (I : Instance) (ρ : ParamEnv) (s : Nat) (lower : PredSets)
    (X : PredSets) : PredSets :=
  fun P t => strat P = s ∧ ∃ d, p.predicates[P.id]? = some d ∧
    ∃ r, r ∈ d.rules ∧
      t ∈ pruleAnswers C r (stratumEnv I strat s lower X) ρ

/-! ## Knaster–Tarski, elementary (the least fixpoint as the
intersection of the prefixed points) -/

/-- The least fixpoint: the intersection of every prefixed point. -/
def lfpP (T : PredSets → PredSets) : PredSets :=
  fun P t => ∀ X : PredSets, PredSets.le (T X) X → t ∈ X P

/-- Monotone in the recursive argument. -/
def MonoP (T : PredSets → PredSets) : Prop :=
  ∀ X Y, PredSets.le X Y → PredSets.le (T X) (T Y)

/-- `lfpP` is below every prefixed point — leastness, definitional. -/
theorem lfpP_le {T : PredSets → PredSets} {X : PredSets}
    (h : PredSets.le (T X) X) : PredSets.le (lfpP T) X :=
  fun _ _ ht => ht X h

/-- `lfpP` is itself prefixed (one Knaster–Tarski half). -/
theorem lfpP_prefixed {T : PredSets → PredSets} (hm : MonoP T) :
    PredSets.le (T (lfpP T)) (lfpP T) := by
  intro P t ht X hX
  exact hX P t (hm (lfpP T) X (lfpP_le hX) P t ht)

/-- `lfpP` is postfixed (the other half). -/
theorem lfpP_postfixed {T : PredSets → PredSets} (hm : MonoP T) :
    PredSets.le (lfpP T) (T (lfpP T)) :=
  lfpP_le (hm _ _ (lfpP_prefixed hm))

/-- **Knaster–Tarski**: a monotone operator's `lfpP` is a fixed
point — and `lfpP_le` makes it the least one. -/
theorem lfpP_fixed {T : PredSets → PredSets} (hm : MonoP T) :
    ∀ P t, t ∈ T (lfpP T) P ↔ t ∈ lfpP T P :=
  fun P t => ⟨fun h => lfpP_prefixed hm P t h,
    fun h => lfpP_postfixed hm P t h⟩

/-- A constant operator's least fixpoint is its value — the
degenerate embedding's engine (an all-`edb` stratum never reads its
recursive argument). -/
theorem lfpP_const {T : PredSets → PredSets}
    (hconst : ∀ X Y P t, t ∈ T X P ↔ t ∈ T Y P) :
    ∀ P t, t ∈ lfpP T P ↔ t ∈ T (fun _ _ => False) P := by
  intro P t
  constructor
  · intro h
    exact lfpP_le (fun Q u hu => (hconst _ _ Q u).mp hu) P t h
  · intro h
    exact lfpP_prefixed
      (fun X Y _ Q u hu => (hconst X Y Q u).mp hu) P t
      ((hconst _ _ P t).mp h)

/-! ## Monotonicity — where stratification pays (§2) -/

/-- The stratum reading is monotone in the working stratum. -/
theorem stratumSets_mono {strat : PredId → Nat} {s : Nat}
    {lower X Y : PredSets} (h : PredSets.le X Y) :
    PredSets.le (stratumSets strat s lower X)
      (stratumSets strat s lower Y) := by
  intro P t ht
  by_cases h1 : strat P < s
  · rw [stratumSets_lt h1] at ht ⊢
    exact ht
  · by_cases h2 : strat P = s
    · rw [stratumSets_at h2] at ht ⊢
      exact h P t ht
    · rw [stratumSets_gt h1 h2] at ht
      exact absurd ht (fun h => h)

/-- Source denotation is monotone in the tables. -/
theorem sourceDen_mono {I : Instance} {X Y : PredSets}
    (h : PredSets.le X Y) :
    ∀ src f, f ∈ sourceDen I X src → f ∈ sourceDen I Y src := by
  intro src f hf
  cases src with
  | edb R => exact hf
  | idb P =>
    obtain ⟨t, ht, rfl⟩ := hf
    exact ⟨t, h P t ht, rfl⟩

/-- Two tables agreeing on the working stratum give one stratum
reading. -/
theorem stratumSets_congr {strat : PredId → Nat} {s : Nat}
    {lower X Y : PredSets}
    (h : ∀ Q, strat Q = s → ∀ u, u ∈ X Q ↔ u ∈ Y Q) :
    ∀ Q u, u ∈ stratumSets strat s lower X Q ↔
      u ∈ stratumSets strat s lower Y Q := by
  intro Q u
  by_cases h1 : strat Q < s
  · rw [stratumSets_lt (X := X) h1, stratumSets_lt (X := Y) h1]
  · by_cases h2 : strat Q = s
    · rw [stratumSets_at (X := X) h2, stratumSets_at (X := Y) h2]
      exact h Q h2 u
    · rw [stratumSets_gt (X := X) h1 h2, stratumSets_gt (X := Y) h1 h2]

/-- Source denotation reads the tables extensionally. -/
theorem sourceDen_congr {I : Instance} {X Y : PredSets}
    (h : ∀ Q u, u ∈ X Q ↔ u ∈ Y Q) :
    ∀ src f, f ∈ sourceDen I X src ↔ f ∈ sourceDen I Y src := by
  intro src f
  cases src with
  | edb R => exact Iff.rfl
  | idb P =>
    exact exists_congr fun t => and_congr_left fun _ => h P t

/-- **Monotonicity of the per-stratum operator** — the theorem the
stratification premise buys: positive occurrences read the working
stratum monotonically, and NEGATED occurrences never read it at all
(their targets sit strictly lower — `StratifiedBy.negated_lt` — so
both sides of the anti-join see the same finished set). Without the
premise the operator can flip: `Countermodels.odd_not_monotone`. -/
theorem stratumOp_mono {C : Classify} {p : Program}
    {strat : PredId → Nat} {I : Instance} {ρ : ParamEnv} {s : Nat}
    {lower : PredSets} (hstrat : p.StratifiedBy strat) :
    MonoP (stratumOp C p strat I ρ s lower) := by
  intro X Y hXY P t ht
  obtain ⟨hPs, d, hd, r, hr, σ, ⟨hpos, hneg, hcond⟩, hproj⟩ := ht
  refine ⟨hPs, d, hd, r, hr, σ, ⟨fun a ha => ?_, fun a ha hex => ?_,
    hcond⟩, hproj⟩
  · obtain ⟨f, hf, hm⟩ := hpos a ha
    exact ⟨f, sourceDen_mono (stratumSets_mono hXY) a.source f hf, hm⟩
  · obtain ⟨f, hf, hm⟩ := hex
    refine hneg a ha ⟨f, ?_, hm⟩
    cases hsrc : a.source with
    | edb R =>
      rw [hsrc] at hf
      exact hf
    | idb Q =>
      have hQ : strat Q < s :=
        hPs ▸ hstrat.negated_lt hd hr ha hsrc
      rw [hsrc] at hf
      rw [stratumEnv_idb_lower (X := X) (Y := Y) hQ]
      exact hf

/-- Every derived tuple carries the stratum tag — the operator's
output never leaves its stratum. -/
theorem lfp_stratumOp_tag {C : Classify} {p : Program}
    {strat : PredId → Nat} {I : Instance} {ρ : ParamEnv} {s : Nat}
    {lower : PredSets} {P : PredId} {t : AnswerTuple}
    (h : t ∈ lfpP (stratumOp C p strat I ρ s lower) P) :
    strat P = s :=
  h (fun Q _ => strat Q = s) (fun _ _ hu => hu.1)

/-- The operator reads the working stratum's positions only. -/
theorem stratumOp_congr {C : Classify} {p : Program}
    {strat : PredId → Nat} {I : Instance} {ρ : ParamEnv} {s : Nat}
    {lower : PredSets} {X Y : PredSets}
    (h : ∀ Q, strat Q = s → ∀ u, u ∈ X Q ↔ u ∈ Y Q) :
    ∀ P t, t ∈ stratumOp C p strat I ρ s lower X P ↔
      t ∈ stratumOp C p strat I ρ s lower Y P := by
  intro P t
  unfold stratumOp
  refine and_congr_right fun _ => exists_congr fun d =>
    and_congr_right fun _ => exists_congr fun r =>
    and_congr_right fun _ => ?_
  exact pruleAnswers_congr
    (sourceDen_congr (stratumSets_congr h)) t

/-! ## The stratified denotation (Level 0) — strata by strong
induction on the stratum index -/

/-- The finished tables below stratum `s`: stratum `j < s` holds the
least fixpoint of its operator with everything below `j` already
finished — the strong induction the design's §2 sketches. -/
def finished (C : Classify) (p : Program) (strat : PredId → Nat)
    (I : Instance) (ρ : ParamEnv) : Nat → PredSets
  | 0 => fun _ _ => False
  | s + 1 => fun P t =>
    finished C p strat I ρ s P t ∨
      t ∈ lfpP (stratumOp C p strat I ρ s
        (finished C p strat I ρ s)) P

/-- Finished tuples carry strata strictly below the ladder point. -/
theorem finished_tag {C : Classify} {p : Program}
    {strat : PredId → Nat} {I : Instance} {ρ : ParamEnv} :
    ∀ s {P : PredId} {t : AnswerTuple},
      finished C p strat I ρ s P t → strat P < s
  | 0, _, _, h => absurd h (fun h => h)
  | s + 1, P, t, h => by
    cases h with
    | inl h => exact Nat.lt_succ_of_lt (finished_tag s h)
    | inr h =>
      have := lfp_stratumOp_tag h
      omega

/-- A predicate's table is finished at its own stratum's close and
never touched again — reading `programDen` at any later ladder point
is the same set. -/
theorem finished_stable {C : Classify} {p : Program}
    {strat : PredId → Nat} {I : Instance} {ρ : ParamEnv} {P : PredId} :
    ∀ s, strat P + 1 ≤ s → ∀ t,
      (finished C p strat I ρ s P t ↔
        finished C p strat I ρ (strat P + 1) P t) := by
  intro s
  induction s with
  | zero => intro h; omega
  | succ s ih =>
    intro hle t
    by_cases hs : strat P + 1 ≤ s
    · constructor
      · intro h
        cases h with
        | inl h => exact (ih hs t).mp h
        | inr h =>
          have := lfp_stratumOp_tag h
          omega
      · intro h
        exact Or.inl ((ih hs t).mpr h)
    · have : s = strat P := by omega
      subst this
      exact Iff.rfl

/-- **The stratified denotation** (Level 0): each predicate reads its
own stratum's least fixpoint, lower strata finished beneath it. -/
def programDen (C : Classify) (p : Program) (strat : PredId → Nat)
    (I : Instance) (ρ : ParamEnv) : PredSets :=
  fun P => finished C p strat I ρ (strat P + 1) P

/-- A program's answers: the output predicate's table. -/
def programAnswers (C : Classify) (p : Program) (strat : PredId → Nat)
    (I : Instance) (ρ : ParamEnv) : Set AnswerTuple :=
  programDen C p strat I ρ p.output

/-! ## The degenerate embedding (§1's theorem) -/

/-- Embedded rules derive exactly as their originals whenever the
environment reads stored relations through the instance. -/
theorem pderives_toPRule {C : Classify} {r : Rule} {I : Instance}
    {ρ : ParamEnv} {σ : Assignment} {F : AtomSource → Set Fact}
    (hF : ∀ R, F (.edb R) = I R) :
    pderives C r.toPRule F ρ σ ↔ derives C r I ρ σ := by
  unfold pderives derives
  constructor
  · rintro ⟨hpos, hneg, hcond⟩
    refine ⟨fun a ha => ?_, fun a ha hex => ?_, hcond⟩
    · obtain ⟨f, hf, hm⟩ :=
        hpos a.toPAtom (List.mem_map.mpr ⟨a, ha, rfl⟩)
      rw [show a.toPAtom.source = .edb a.relation from rfl, hF] at hf
      exact ⟨f, hf, hm⟩
    · obtain ⟨f, hf, hm⟩ := hex
      refine hneg a.toPAtom (List.mem_map.mpr ⟨a, ha, rfl⟩) ⟨f, ?_, hm⟩
      rw [show a.toPAtom.source = .edb a.relation from rfl, hF]
      exact hf
  · rintro ⟨hpos, hneg, hcond⟩
    refine ⟨fun a' ha' => ?_, fun a' ha' hex => ?_, hcond⟩
    · obtain ⟨a, ha, rfl⟩ := List.mem_map.mp ha'
      obtain ⟨f, hf, hm⟩ := hpos a ha
      refine ⟨f, ?_, hm⟩
      rw [show a.toPAtom.source = .edb a.relation from rfl, hF]
      exact hf
    · obtain ⟨a, ha, rfl⟩ := List.mem_map.mp ha'
      obtain ⟨f, hf, hm⟩ := hex
      rw [show a.toPAtom.source = .edb a.relation from rfl, hF] at hf
      exact hneg a ha ⟨f, hf, hm⟩

/-- **The degenerate embedding**: a one-predicate program with no
`idb` atom denotes exactly today's `queryAnswers` — the design's §1
claim ("the degenerate form is today's `Query`"; `Query::single` is
the Rust precedent), as a theorem. The all-`edb` stratum operator is
constant in its recursive argument, so its least fixpoint is one
application (`lfpP_const`), and embedded rules derive as their
originals (`pderives_toPRule`). -/
theorem degenerate_embedding (C : Classify) (q : Query) (I : Instance)
    (ρ : ParamEnv) :
    ∀ t, t ∈ programAnswers C q.toProgram (fun _ => 0) I ρ ↔
      t ∈ queryAnswers C q I ρ := by
  intro t
  have key : ∀ (X : PredSets) (u : AnswerTuple),
      u ∈ stratumOp C q.toProgram (fun _ => 0) I ρ 0
        (fun _ _ => False) X ⟨0⟩ ↔ u ∈ queryAnswers C q I ρ := by
    intro X u
    constructor
    · rintro ⟨-, d, hd, r', hr', σ, hder, hproj⟩
      have hd' : (Query.toProgram q).predicates[(0 : Nat)]? = some d := hd
      simp only [Query.toProgram, List.getElem?_cons_zero,
        Option.some.injEq] at hd'
      rw [← hd'] at hr'
      obtain ⟨r, hr, rfl⟩ := List.mem_map.mp hr'
      exact ⟨r, hr, σ, (pderives_toPRule fun R => rfl).mp hder, hproj⟩
    · rintro ⟨r, hr, σ, hder, hproj⟩
      exact ⟨rfl, _, rfl, r.toPRule, List.mem_map.mpr ⟨r, hr, rfl⟩, σ,
        (pderives_toPRule fun R => rfl).mpr hder, hproj⟩
  have hconst : ∀ (X Y : PredSets) (P : PredId) (u : AnswerTuple),
      u ∈ stratumOp C q.toProgram (fun _ => 0) I ρ 0
        (fun _ _ => False) X P ↔
      u ∈ stratumOp C q.toProgram (fun _ => 0) I ρ 0
        (fun _ _ => False) Y P := by
    intro X Y P u
    obtain ⟨i⟩ := P
    cases i with
    | zero => exact (key X u).trans (key Y u).symm
    | succ n =>
      constructor
      · rintro ⟨-, d, hd, -⟩
        cases hd
      · rintro ⟨-, d, hd, -⟩
        cases hd
  show finished C q.toProgram (fun _ => 0) I ρ (0 + 1) ⟨0⟩ t ↔
    t ∈ queryAnswers C q I ρ
  have hun : finished C q.toProgram (fun _ => 0) I ρ (0 + 1) ⟨0⟩ t ↔
      (finished C q.toProgram (fun _ => 0) I ρ 0 ⟨0⟩ t ∨
        t ∈ lfpP (stratumOp C q.toProgram (fun _ => 0) I ρ 0
          (finished C q.toProgram (fun _ => 0) I ρ 0)) ⟨0⟩) := Iff.rfl
  rw [hun]
  constructor
  · rintro (h | h)
    · exact absurd h (fun h => h)
    · exact (key _ t).mp ((lfpP_const hconst ⟨0⟩ t).mp h)
  · intro h
    exact Or.inr ((lfpP_const hconst ⟨0⟩ t).mpr ((key _ t).mpr h))

/-! ## The coding transport — `evalRule` reused whole (Level 1) -/

/-- The even/odd source coding: stored relations on the even ids,
predicates on the odd — a proof-reuse device (module doc), injective
by parity. -/
def AtomSource.code : AtomSource → RelId
  | .edb R => ⟨2 * R.id⟩
  | .idb P => ⟨2 * P.id + 1⟩

/-- The coding is injective — coded worlds never alias two sources. -/
theorem AtomSource.code_injective :
    ∀ (s s' : AtomSource), s.code = s'.code → s = s'
  | .edb ⟨a⟩, .edb ⟨b⟩, h => by
    have hab : 2 * a = 2 * b := congrArg RelId.id h
    have : a = b := by omega
    rw [this]
  | .edb ⟨a⟩, .idb ⟨b⟩, h => by
    have hab : 2 * a = 2 * b + 1 := congrArg RelId.id h
    omega
  | .idb ⟨a⟩, .edb ⟨b⟩, h => by
    have hab : 2 * a + 1 = 2 * b := congrArg RelId.id h
    omega
  | .idb ⟨a⟩, .idb ⟨b⟩, h => by
    have hab : 2 * a + 1 = 2 * b + 1 := congrArg RelId.id h
    have : a = b := by omega
    rw [this]

/-- A program atom, coded: the same bindings over the coded relation
id — `Matches` never read the relation, so matching is untouched. -/
def PAtom.code (a : PAtom) : Atom :=
  { relation := a.source.code, bindings := a.bindings }

/-- A program rule, coded: a plain `Rule` over coded relation ids. -/
def PRule.code (r : PRule) : Rule :=
  { finds := r.finds, atoms := r.atoms.map PAtom.code,
    negated := r.negated.map PAtom.code, conditions := r.conditions }

/-- The coded reading of a source environment: an instance whose
even relations read `edb` sources and odd relations `idb` sources. -/
def codeInst (F : AtomSource → Set Fact) : Instance :=
  fun R => if R.id % 2 = 0 then F (.edb ⟨R.id / 2⟩)
    else F (.idb ⟨R.id / 2⟩)

/-- The coded instance reads back the environment. -/
theorem codeInst_code (F : AtomSource → Set Fact) :
    ∀ s, codeInst F (AtomSource.code s) = F s := by
  intro s
  cases s with
  | edb R =>
    show (if (2 * R.id) % 2 = 0 then F (.edb ⟨2 * R.id / 2⟩)
      else F (.idb ⟨2 * R.id / 2⟩)) = F (.edb R)
    rw [if_pos (by omega : (2 * R.id) % 2 = 0),
      (by omega : 2 * R.id / 2 = R.id)]
  | idb P =>
    show (if (2 * P.id + 1) % 2 = 0 then F (.edb ⟨(2 * P.id + 1) / 2⟩)
      else F (.idb ⟨(2 * P.id + 1) / 2⟩)) = F (.idb P)
    rw [if_neg (by omega : ¬ (2 * P.id + 1) % 2 = 0),
      (by omega : (2 * P.id + 1) / 2 = P.id)]

/-- **The coding lemma**: the program-level judgment IS `derives`
over the coded rule, whenever the instance agrees with the
environment on the rule's own sources — the bridge every Level-1
theorem walks. -/
theorem pderives_code {C : Classify} {r : PRule} {ρ : ParamEnv}
    {σ : Assignment} {F : AtomSource → Set Fact} {I : Instance}
    (hI : ∀ a : PAtom, a ∈ r.atoms ∨ a ∈ r.negated →
      I a.source.code = F a.source) :
    pderives C r F ρ σ ↔ derives C r.code I ρ σ := by
  unfold pderives derives
  constructor
  · rintro ⟨hpos, hneg, hcond⟩
    refine ⟨fun a' ha' => ?_, fun a' ha' hex => ?_, hcond⟩
    · obtain ⟨a, ha, rfl⟩ := List.mem_map.mp ha'
      obtain ⟨f, hf, hm⟩ := hpos a ha
      refine ⟨f, ?_, hm⟩
      show f ∈ I a.source.code
      rw [hI a (Or.inl ha)]
      exact hf
    · obtain ⟨a, ha, rfl⟩ := List.mem_map.mp ha'
      obtain ⟨f, hf, hm⟩ := hex
      refine hneg a ha ⟨f, ?_, hm⟩
      have hf' : f ∈ I a.source.code := hf
      rwa [hI a (Or.inr ha)] at hf'
  · rintro ⟨hpos, hneg, hcond⟩
    refine ⟨fun a ha => ?_, fun a ha hex => ?_, hcond⟩
    · obtain ⟨f, hf, hm⟩ := hpos a.code (List.mem_map.mpr ⟨a, ha, rfl⟩)
      refine ⟨f, ?_, hm⟩
      have hf' : f ∈ I a.source.code := hf
      rwa [hI a (Or.inl ha)] at hf'
    · obtain ⟨f, hf, hm⟩ := hex
      refine hneg a.code (List.mem_map.mpr ⟨a, ha, rfl⟩) ⟨f, ?_, hm⟩
      show f ∈ I a.source.code
      rw [hI a (Or.inr ha)]
      exact hf

/-! ## The executable rule stage — `evalRule` over a coded world -/

/-- The sources a rule mentions, positive and negated — the coded
world's domain. -/
def PRule.sources (r : PRule) : List AtomSource :=
  (r.atoms ++ r.negated).map PAtom.source

/-- The coded world of one rule: each mentioned source's fact list
under its coded id. -/
def codeWorld (r : PRule) (env : AtomSource → List Fact) :
    ListInstance :=
  ⟨r.sources.map fun s => (s.code, env s)⟩

/-- Coded lookup lands on the source's own fact list (injectivity of
the coding is what makes the association list functional). -/
theorem find?_code_map (env : AtomSource → List Fact) :
    ∀ (l : List AtomSource) (s : AtomSource), s ∈ l →
      (l.map fun s' => (s'.code, env s')).find?
        (fun e => e.1 == s.code) = some (s.code, env s)
  | [], _, h => absurd h (by simp)
  | s' :: l, s, h => by
    show List.find? _ ((s'.code, env s') :: _) = _
    by_cases heq : s'.code = s.code
    · have hss : s' = s := AtomSource.code_injective s' s heq
      subst hss
      exact List.find?_cons_of_pos (by simp)
    · rw [List.find?_cons_of_neg (by simpa using heq)]
      have hmem : s ∈ l := by
        rcases List.mem_cons.mp h with rfl | hmem
        · exact absurd rfl heq
        · exact hmem
      exact find?_code_map env l s hmem

/-- The coded world's fact lists read back the environment on the
rule's sources. -/
theorem codeWorld_facts {r : PRule} {env : AtomSource → List Fact}
    {s : AtomSource} (hs : s ∈ r.sources) :
    (codeWorld r env).facts s.code = env s := by
  unfold ListInstance.facts
  rw [show (codeWorld r env).rels =
      r.sources.map (fun s' => (s'.code, env s')) from rfl,
    find?_code_map env r.sources s hs]

/-- The list-level environment, read as sets. -/
def envDen (env : AtomSource → List Fact) : AtomSource → Set Fact :=
  fun s f => f ∈ env s

/-- The coded world denotes the environment on a rule's occurrences. -/
theorem codeWorld_den {r : PRule} {env : AtomSource → List Fact}
    {a : PAtom} (ha : a ∈ r.atoms ∨ a ∈ r.negated) :
    (codeWorld r env).den a.source.code = envDen env a.source := by
  have hs : a.source ∈ r.sources :=
    List.mem_map.mpr ⟨a, List.mem_append.mpr ha, rfl⟩
  show (fun f => f ∈ (codeWorld r env).facts a.source.code) = _
  rw [codeWorld_facts hs]
  rfl

/-- One rule, executed: `evalRule` — join, negation filter, condition
filter, projection, PROVED once at the query level — over the coded
world. Nothing recursive happens inside a rule; rounds are the
loop's. -/
def pevalRule (C : Classify) (env : AtomSource → List Fact)
    (ρ : ParamEnv) (r : PRule) : List AnswerTuple :=
  evalRule C (codeWorld r env) ρ r.code

/-- The rule stage is sound — unconditionally, like `evalRule`. -/
theorem pevalRule_sound {C : Classify} {env : AtomSource → List Fact}
    {ρ : ParamEnv} {r : PRule} {t : AnswerTuple}
    (h : t ∈ pevalRule C env ρ r) :
    t ∈ pruleAnswers C r (envDen env) ρ := by
  obtain ⟨σ, hder, hproj⟩ := evalRule_sound h
  exact ⟨σ, (pderives_code fun a ha => codeWorld_den ha).mpr hder, hproj⟩

/-! ## Safety over program rules — the transported premise -/

/-- Positive range restriction over a program rule — `Safe`,
verbatim: every mentioned variable is bound by a positive atom.
The acceptance premise recursion inherits unchanged. -/
def PRule.Safe (r : PRule) : Prop :=
  ∀ v, v ∈ r.allVars → v ∈ r.positiveVars

/-- Coded atoms mention the same variables. -/
theorem PAtom.code_vars (a : PAtom) : a.code.vars = a.vars := rfl

/-- Coded atoms bind the same variables. -/
theorem PAtom.code_boundVars (a : PAtom) :
    a.code.boundVars = a.boundVars := rfl

/-- Positive binding sites survive the coding, membership for
membership. -/
theorem code_positiveVars {r : PRule} {v : VarId} :
    v ∈ r.code.positiveVars ↔ v ∈ r.positiveVars := by
  unfold Rule.positiveVars PRule.positiveVars
  constructor
  · intro h
    obtain ⟨a', ha', hv⟩ := List.mem_flatMap.mp h
    obtain ⟨a, ha, rfl⟩ := List.mem_map.mp ha'
    exact List.mem_flatMap.mpr ⟨a, ha, hv⟩
  · intro h
    obtain ⟨a, ha, hv⟩ := List.mem_flatMap.mp h
    exact List.mem_flatMap.mpr
      ⟨a.code, List.mem_map.mpr ⟨a, ha, rfl⟩, hv⟩

/-- Variable mentions survive the coding. -/
theorem code_allVars {r : PRule} {v : VarId} :
    v ∈ r.code.allVars ↔ v ∈ r.allVars := by
  unfold Rule.allVars PRule.allVars
  simp only [List.mem_append, List.mem_flatMap]
  constructor
  · rintro (((h | ⟨a', ha', hv⟩) | ⟨a', ha', hv⟩) | h)
    · exact Or.inl (Or.inl (Or.inl h))
    · obtain ⟨a, ha, rfl⟩ := List.mem_map.mp ha'
      exact Or.inl (Or.inl (Or.inr ⟨a, ha, hv⟩))
    · obtain ⟨a, ha, rfl⟩ := List.mem_map.mp ha'
      exact Or.inl (Or.inr ⟨a, ha, hv⟩)
    · exact Or.inr h
  · rintro (((h | ⟨a, ha, hv⟩) | ⟨a, ha, hv⟩) | h)
    · exact Or.inl (Or.inl (Or.inl h))
    · exact Or.inl (Or.inl (Or.inr
        ⟨a.code, List.mem_map.mpr ⟨a, ha, rfl⟩, hv⟩))
    · exact Or.inl (Or.inr ⟨a.code, List.mem_map.mpr ⟨a, ha, rfl⟩, hv⟩)
    · exact Or.inr h

/-- Safety survives the coding — the executable completeness premise
is exactly the program rule's own safety. -/
theorem code_safe {r : PRule} : Query.Safe r.code ↔ r.Safe := by
  unfold Query.Safe PRule.Safe
  constructor
  · intro h v hv
    exact code_positiveVars.mp (h v (code_allVars.mpr hv))
  · intro h v hv
    exact code_positiveVars.mpr (h v (code_allVars.mp hv))

/-- Measure-free bindings over a program rule's positive atoms — the
binding shape discipline `evalRule_complete` spends (the validator's
`DurationInBinding`, program-side). -/
def PRule.BindingsMeasureFree (r : PRule) : Prop :=
  ∀ a, a ∈ r.atoms → ∀ b, b ∈ a.bindings → ¬ b.2.isMeasure

/-- The rule stage is complete under safety and the binding shape
discipline — `evalRule_complete`, transported. -/
theorem pevalRule_complete {C : Classify} {env : AtomSource → List Fact}
    {ρ : ParamEnv} {r : PRule} {t : AnswerTuple} (hsafe : r.Safe)
    (hnm : r.BindingsMeasureFree)
    (h : t ∈ pruleAnswers C r (envDen env) ρ) :
    t ∈ pevalRule C env ρ r := by
  obtain ⟨σ, hder, hproj⟩ := h
  refine evalRule_complete (code_safe.mpr hsafe) ?_
    ⟨σ, (pderives_code fun a ha => codeWorld_den ha).mp hder, hproj⟩
  intro a' ha' b hb
  obtain ⟨a, ha, rfl⟩ := List.mem_map.mp
    (show a' ∈ r.atoms.map PAtom.code from ha')
  exact hnm a ha b hb

/-- The rule stage agrees with the program-level answers,
membership for membership. -/
theorem pevalRule_iff {C : Classify} {env : AtomSource → List Fact}
    {ρ : ParamEnv} {r : PRule} (hsafe : r.Safe)
    (hnm : r.BindingsMeasureFree) :
    ∀ t, t ∈ pevalRule C env ρ r ↔ t ∈ pruleAnswers C r (envDen env) ρ :=
  fun _ => ⟨pevalRule_sound, pevalRule_complete hsafe hnm⟩

/-! ## Predicate tables as data — the round state -/

/-- The executable table: derived (predicate, tuple) pairs. -/
abbrev Tab : Type := List (PredId × AnswerTuple)

/-- One predicate's tuples. -/
def tabAt (tab : Tab) (P : PredId) : List AnswerTuple :=
  (tab.filter fun e => e.1 == P).map (·.2)

/-- The table's set reading. -/
def tabSets (tab : Tab) : PredSets :=
  fun P t => (P, t) ∈ tab

theorem mem_tabAt {tab : Tab} {P : PredId} {t : AnswerTuple} :
    t ∈ tabAt tab P ↔ (P, t) ∈ tab := by
  unfold tabAt
  constructor
  · intro h
    obtain ⟨e, he, rfl⟩ := List.mem_map.mp h
    obtain ⟨hmem, heq⟩ := List.mem_filter.mp he
    have hP : e.1 = P := by simpa using heq
    rw [← hP]
    exact hmem
  · intro h
    exact List.mem_map.mpr
      ⟨(P, t), List.mem_filter.mpr ⟨h, by simp⟩, rfl⟩

/-- The executable environment of a table: stored relations from the
world, predicates from the table through the tuple-fact reading. -/
def progEnv (W : ListInstance) (tab : Tab) : AtomSource → List Fact
  | .edb R => W.facts R
  | .idb P => (tabAt tab P).map tupleFact

/-- The executable environment denotes the table's set reading. -/
theorem progEnv_den (W : ListInstance) (tab : Tab) :
    ∀ src f, f ∈ envDen (progEnv W tab) src ↔
      f ∈ sourceDen W.den (tabSets tab) src := by
  intro src f
  cases src with
  | edb R => exact Iff.rfl
  | idb P =>
    show f ∈ (tabAt tab P).map tupleFact ↔
      ∃ t, (P, t) ∈ tab ∧ f = tupleFact t
    constructor
    · intro h
      obtain ⟨t, ht, rfl⟩ := List.mem_map.mp h
      exact ⟨t, mem_tabAt.mp ht, rfl⟩
    · rintro ⟨t, ht, rfl⟩
      exact List.mem_map.mpr ⟨t, mem_tabAt.mpr ht, rfl⟩

/-- Under the running invariants — lower strata finished in the
table, nothing above the working stratum — the executable
environment IS the stratum environment. -/
theorem progEnv_stratumEnv {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {p : Program} {strat : PredId → Nat} {s : Nat}
    {tab : Tab}
    (hlow : ∀ P t, strat P < s →
      ((P, t) ∈ tab ↔ finished C p strat W.den ρ s P t))
    (hle : ∀ P t, (P, t) ∈ tab → strat P ≤ s) :
    ∀ src f, f ∈ envDen (progEnv W tab) src ↔
      f ∈ stratumEnv W.den strat s (finished C p strat W.den ρ s)
        (tabSets tab) src := by
  intro src f
  rw [progEnv_den]
  cases src with
  | edb R => exact Iff.rfl
  | idb Q =>
    show (∃ t, t ∈ tabSets tab Q ∧ f = tupleFact t) ↔
      ∃ t, t ∈ stratumSets strat s _ (tabSets tab) Q ∧ f = tupleFact t
    by_cases h1 : strat Q < s
    · rw [stratumSets_lt h1]
      exact exists_congr fun t =>
        and_congr_left fun _ => hlow Q t h1
    · by_cases h2 : strat Q = s
      · rw [stratumSets_at h2]
      · rw [stratumSets_gt h1 h2]
        constructor
        · rintro ⟨t, ht, -⟩
          exact absurd (hle Q t ht) (by omega)
        · rintro ⟨t, ht, -⟩
          exact absurd ht (fun h => h)

/-! ## One round — every stratum rule against the current tables -/

/-- One round of stratum `s`: every rule of every stratum-`s`
predicate runs against the current tables, and the results union in
(list append; the set reading is the union — `mem_stratumStep`). -/
def stratumStep (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (p : Program) (strat : PredId → Nat) (s : Nat) (tab : Tab) : Tab :=
  tab ++ (List.range p.predicates.length).flatMap fun i =>
    match p.predicates[i]? with
    | some d =>
      if strat ⟨i⟩ = s then
        d.rules.flatMap fun r =>
          (pevalRule C (progEnv W tab) ρ r).map ((⟨i⟩, ·))
      else []
    | none => []

/-- Round membership: kept, or newly derived by a stratum rule. -/
theorem mem_stratumStep {C : Classify} {W : ListInstance}
    {ρ : ParamEnv} {p : Program} {strat : PredId → Nat} {s : Nat}
    {tab : Tab} {P : PredId} {t : AnswerTuple} :
    (P, t) ∈ stratumStep C W ρ p strat s tab ↔
      (P, t) ∈ tab ∨ (strat P = s ∧ ∃ d, p.predicates[P.id]? = some d ∧
        ∃ r, r ∈ d.rules ∧ t ∈ pevalRule C (progEnv W tab) ρ r) := by
  unfold stratumStep
  rw [List.mem_append]
  refine or_congr Iff.rfl ?_
  constructor
  · intro h
    obtain ⟨i, hi, hmem⟩ := List.mem_flatMap.mp h
    cases hd : p.predicates[i]? with
    | none =>
      rw [hd] at hmem
      have hmem' : (P, t) ∈ ([] : Tab) := hmem
      exact absurd hmem' (by simp)
    | some d =>
      rw [hd] at hmem
      have hmem' : (P, t) ∈ (if strat (⟨i⟩ : PredId) = s then
          d.rules.flatMap fun r =>
            (pevalRule C (progEnv W tab) ρ r).map ((⟨i⟩, ·))
        else []) := hmem
      by_cases hs : strat (⟨i⟩ : PredId) = s
      · rw [if_pos hs] at hmem'
        obtain ⟨r, hr, hpe⟩ := List.mem_flatMap.mp hmem'
        obtain ⟨t', ht', heq⟩ := List.mem_map.mp hpe
        rw [Prod.mk.injEq] at heq
        obtain ⟨hP, ht⟩ := heq
        rw [← hP, ← ht]
        exact ⟨hs, d, hd, r, hr, ht'⟩
      · rw [if_neg hs] at hmem'
        exact absurd hmem' (by simp)
  · rintro ⟨hPs, d, hd, r, hr, hpe⟩
    have hlt : P.id < p.predicates.length := by
      rcases Nat.lt_or_ge P.id p.predicates.length with h | h
      · exact h
      · rw [List.getElem?_eq_none h] at hd
        cases hd
    refine List.mem_flatMap.mpr ⟨P.id, List.mem_range.mpr hlt, ?_⟩
    rw [hd]
    show (P, t) ∈ (if strat (⟨P.id⟩ : PredId) = s then
        d.rules.flatMap fun r =>
          (pevalRule C (progEnv W tab) ρ r).map ((⟨P.id⟩, ·))
      else [])
    rw [if_pos (show strat (⟨P.id⟩ : PredId) = s from hPs)]
    exact List.mem_flatMap.mpr
      ⟨r, hr, List.mem_map.mpr ⟨t, hpe, rfl⟩⟩

/-! ## The fueled loop (abstract): chaotic iteration, counted -/

/-- Fewer predicates satisfied, no longer a filter. -/
theorem length_filter_mono {α : Type} {l : List α} {p q : α → Bool}
    (h : ∀ a, q a = true → p a = true) :
    (l.filter q).length ≤ (l.filter p).length := by
  induction l with
  | nil => exact Nat.le_refl _
  | cons a l ih =>
    rw [List.filter_cons, List.filter_cons]
    by_cases hq : q a = true
    · rw [if_pos hq, if_pos (h a hq)]
      exact Nat.succ_le_succ ih
    · rw [if_neg hq]
      by_cases hp : p a = true
      · rw [if_pos hp]
        exact Nat.le_succ_of_le ih
      · rw [if_neg hp]
        exact ih

/-- A witnessed strict filter drop: some listed element satisfies the
old test and fails the new. -/
theorem length_filter_lt {α : Type} {l : List α} {p q : α → Bool}
    (h : ∀ a, q a = true → p a = true) {x : α} (hx : x ∈ l)
    (hpx : p x = true) (hqx : q x = false) :
    (l.filter q).length < (l.filter p).length := by
  induction l with
  | nil => cases hx
  | cons a l ih =>
    rw [List.filter_cons, List.filter_cons]
    rcases List.mem_cons.mp hx with rfl | hx'
    · rw [if_pos hpx, if_neg (by simp [hqx])]
      exact Nat.lt_succ_of_le (length_filter_mono h)
    · by_cases hq : q a = true
      · rw [if_pos hq, if_pos (h a hq)]
        exact Nat.succ_lt_succ (ih hx')
      · rw [if_neg hq]
        by_cases hp : p a = true
        · rw [if_pos hp]
          exact Nat.lt_succ_of_lt (ih hx')
        · rw [if_neg hp]
          exact ih hx'

section FueledLoop

variable {α : Type} [DecidableEq α]

/-- The fueled round loop: step, stop on no change (subset check),
spend one fuel per growing round. -/
def fueledLoop (step : List α → List α) : Nat → List α → List α
  | 0, acc => acc
  | fuel + 1, acc =>
    if (step acc).all (fun x => decide (x ∈ acc)) then acc
    else fueledLoop step fuel (step acc)

/-- How many candidates the state still misses — the loop's measure. -/
def missingCount (cands acc : List α) : Nat :=
  (cands.filter fun c => decide (c ∉ acc)).length

/-- **The fuel bound is a lemma, not a hope**: the measure never
exceeds the candidate count, so `candidate count + 1` fuel always
reaches the fixpoint (`fueledLoop_fixpoint`'s premise). -/
theorem missingCount_le (cands acc : List α) :
    missingCount cands acc ≤ cands.length :=
  List.length_filter_le _ _

/-- **Termination of chaotic iteration, counted**: an inflationary
step whose growth is confined to a finite candidate list reaches a
step-closed state within `missing + 1` rounds — each growing round
claims at least one candidate. The invariant is threaded so callers
can carry semantic state through the loop. -/
theorem fueledLoop_fixpoint (step : List α → List α) (cands : List α)
    (Inv : List α → Prop)
    (hext : ∀ acc x, x ∈ acc → x ∈ step acc)
    (hinv : ∀ acc, Inv acc → Inv (step acc))
    (hbound : ∀ acc, Inv acc → ∀ x, x ∈ step acc → x ∈ acc ∨ x ∈ cands) :
    ∀ fuel acc, Inv acc → missingCount cands acc < fuel →
      Inv (fueledLoop step fuel acc) ∧
      (∀ x, x ∈ acc → x ∈ fueledLoop step fuel acc) ∧
      (∀ x, x ∈ step (fueledLoop step fuel acc) →
        x ∈ fueledLoop step fuel acc)
  | 0, _, _, hfuel => absurd hfuel (by omega)
  | fuel + 1, acc, hI, hfuel => by
    have hunfold : fueledLoop step (fuel + 1) acc =
        if (step acc).all (fun x => decide (x ∈ acc)) then acc
        else fueledLoop step fuel (step acc) := rfl
    rw [hunfold]
    by_cases hstop : (step acc).all (fun x => decide (x ∈ acc)) = true
    · rw [if_pos hstop]
      refine ⟨hI, fun x hx => hx, fun x hx => ?_⟩
      exact of_decide_eq_true (List.all_eq_true.mp hstop x hx)
    · rw [if_neg hstop]
      have hex : ∃ x, x ∈ step acc ∧ x ∉ acc := by
        apply Classical.byContradiction
        intro hall
        apply hstop
        refine List.all_eq_true.mpr fun x hx => decide_eq_true ?_
        exact Classical.byContradiction fun hxn => hall ⟨x, hx, hxn⟩
      obtain ⟨x, hxs, hxn⟩ := hex
      have hxc : x ∈ cands := (hbound acc hI x hxs).resolve_left hxn
      have hdec : missingCount cands (step acc) <
          missingCount cands acc := by
        refine length_filter_lt ?_ hxc (decide_eq_true hxn)
          (decide_eq_false fun h => h hxs)
        intro a ha
        exact decide_eq_true
          (fun hmem => of_decide_eq_true ha (hext acc a hmem))
      have hrec := fueledLoop_fixpoint step cands Inv hext hinv hbound
        fuel (step acc) (hinv acc hI) (by omega)
      exact ⟨hrec.1, fun y hy => hrec.2.1 y (hext acc y hy), hrec.2.2⟩

end FueledLoop

/-! ## The candidate space — the safety theorem's finite product -/

/-- Every tuple of a given length over a domain — the finite product
of active-domain words the design's §2 names. -/
def allTuples (dom : List Value) : Nat → List AnswerTuple
  | 0 => [[]]
  | n + 1 => dom.flatMap fun v => (allTuples dom n).map (v :: ·)

theorem mem_allTuples {dom : List Value} :
    ∀ {n : Nat} {t : AnswerTuple},
      t ∈ allTuples dom n ↔ (t.length = n ∧ ∀ v, v ∈ t → v ∈ dom)
  | 0, t => by
    show t ∈ [[]] ↔ _
    rw [List.mem_singleton]
    constructor
    · rintro rfl
      exact ⟨rfl, fun v hv => absurd hv (by simp)⟩
    · rintro ⟨hlen, -⟩
      exact List.eq_nil_of_length_eq_zero hlen
  | n + 1, t => by
    show t ∈ dom.flatMap _ ↔ _
    constructor
    · intro h
      obtain ⟨v, hv, hmem⟩ := List.mem_flatMap.mp h
      obtain ⟨t', ht', rfl⟩ := List.mem_map.mp hmem
      obtain ⟨hlen, hall⟩ := (mem_allTuples (n := n)).mp ht'
      refine ⟨by simp [hlen], fun w hw => ?_⟩
      rcases List.mem_cons.mp hw with rfl | hw'
      · exact hv
      · exact hall w hw'
    · rintro ⟨hlen, hall⟩
      cases t with
      | nil => simp at hlen
      | cons v t' =>
        refine List.mem_flatMap.mpr
          ⟨v, hall v (List.mem_cons_self ..), List.mem_map.mpr
            ⟨t', (mem_allTuples (n := n)).mpr
              ⟨by simpa using hlen,
                fun w hw => hall w (List.mem_cons_of_mem _ hw)⟩, rfl⟩⟩

/-- The program's active domain over a concrete world: the filler
plus every stored value some rule can bind — a finite list, because
rules read facts through their finitely many bindings only. Derived
values never leave it (`pevalRule_dom`): heads project bound
variables, bound variables read stored columns or predicate tuples,
and predicate tuples are themselves derived — the creation
quarantine, cashed as finiteness. -/
def progDom (p : Program) (W : ListInstance) : List Value :=
  fillerValue :: p.rulesList.flatMap fun r =>
    r.atoms.flatMap fun a =>
      match a.source with
      | .edb R => a.bindings.flatMap fun b => (W.facts R).map (· b.1)
      | .idb _ => []

/-- A rule of a listed predicate is a listed rule. -/
theorem rule_mem_rulesList {p : Program} {i : Nat} {d : PredicateDef}
    {r : PRule} (hd : p.predicates[i]? = some d) (hr : r ∈ d.rules) :
    r ∈ p.rulesList :=
  List.mem_flatMap.mpr ⟨d, List.mem_of_getElem? hd, hr⟩

/-- **Derived values stay on the active domain** — the §2 safety
argument's executable form: an emitted tuple's values come off
matched facts at bound positions (`antijoin_over_active_domain`,
walked through the coding), and a matched fact is a stored fact or a
table tuple's fact — stored values, table values (inductively on the
domain), or the filler. -/
theorem pevalRule_dom {C : Classify} {W : ListInstance} {ρ : ParamEnv}
    {p : Program} {tab : Tab} {r : PRule} (hr : r ∈ p.rulesList)
    (hsafe : r.Safe)
    (htab : ∀ P t, (P, t) ∈ tab → ∀ v, v ∈ t → v ∈ progDom p W) :
    ∀ t, t ∈ pevalRule C (progEnv W tab) ρ r →
      ∀ v, v ∈ t → v ∈ progDom p W := by
  intro t ht v hv
  have hans : t ∈ ruleAnswers C r.code
      (codeWorld r (progEnv W tab)).den ρ := evalRule_sound ht
  have hdom := antijoin_over_active_domain (code_safe.mpr hsafe) t hans
    v hv
  obtain ⟨a', ha', f, hf0, b, hb, hfb⟩ := hdom
  obtain ⟨a, ha, rfl⟩ := List.mem_map.mp
    (show a' ∈ r.atoms.map PAtom.code from ha')
  have hf : f ∈ (codeWorld r (progEnv W tab)).den a.source.code := hf0
  rw [codeWorld_den (Or.inl ha)] at hf
  cases hsrc : a.source with
  | edb R =>
    rw [hsrc] at hf
    refine List.mem_cons_of_mem _ (List.mem_flatMap.mpr ⟨r, hr,
      List.mem_flatMap.mpr ⟨a, ha, ?_⟩⟩)
    rw [hsrc]
    exact List.mem_flatMap.mpr
      ⟨b, hb, List.mem_map.mpr ⟨f, hf, hfb⟩⟩
  | idb Q =>
    rw [hsrc] at hf
    obtain ⟨t', ht', rfl⟩ := List.mem_map.mp hf
    rcases tupleFact_mem_or_filler t' b.1 with hmem | hfill
    · rw [← hfb]
      exact htab Q t' (mem_tabAt.mp ht') _ hmem
    · rw [← hfb, hfill]
      exact List.mem_cons_self ..

/-- The stratum's candidate space: for every stratum rule, every
tuple of its head length over the active domain — finite by
construction, and the fuel bound reads its length. -/
def stratumCands (p : Program) (strat : PredId → Nat) (s : Nat)
    (W : ListInstance) : Tab :=
  (List.range p.predicates.length).flatMap fun i =>
    match p.predicates[i]? with
    | some d =>
      if strat ⟨i⟩ = s then
        d.rules.flatMap fun r =>
          (allTuples (progDom p W) r.finds.length).map ((⟨i⟩, ·))
      else []
    | none => []

/-- Membership in the candidate space, from the pieces. -/
theorem mem_stratumCands {p : Program} {strat : PredId → Nat} {s : Nat}
    {W : ListInstance} {P : PredId} {t : AnswerTuple} {d : PredicateDef}
    {r : PRule} (hd : p.predicates[P.id]? = some d) (hs : strat P = s)
    (hr : r ∈ d.rules) (hlen : t.length = r.finds.length)
    (hdom : ∀ v, v ∈ t → v ∈ progDom p W) :
    (P, t) ∈ stratumCands p strat s W := by
  unfold stratumCands
  have hlt : P.id < p.predicates.length := by
    rcases Nat.lt_or_ge P.id p.predicates.length with h | h
    · exact h
    · rw [List.getElem?_eq_none h] at hd
      cases hd
  refine List.mem_flatMap.mpr ⟨P.id, List.mem_range.mpr hlt, ?_⟩
  rw [hd]
  show (P, t) ∈ (if strat (⟨P.id⟩ : PredId) = s then
      d.rules.flatMap fun r =>
        (allTuples (progDom p W) r.finds.length).map ((⟨P.id⟩, ·))
    else [])
  rw [if_pos (show strat (⟨P.id⟩ : PredId) = s from hs)]
  exact List.mem_flatMap.mpr ⟨r, hr, List.mem_map.mpr
    ⟨t, mem_allTuples.mpr ⟨hlen, hdom⟩, rfl⟩⟩

/-! ## The per-stratum theorem — the loop computes the stratum's
least fixpoint -/

/-- The running invariant of stratum `s`'s round loop: derived values
on the active domain; lower strata exactly the finished tables and
untouched; nothing above the working stratum; and every working-
stratum tuple already inside the least fixpoint (round soundness). -/
def StratumInv (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (p : Program) (strat : PredId → Nat) (s : Nat) (tab : Tab) : Prop :=
  (∀ P t, (P, t) ∈ tab → ∀ v, v ∈ t → v ∈ progDom p W) ∧
  (∀ P t, strat P < s →
    ((P, t) ∈ tab ↔ finished C p strat W.den ρ s P t)) ∧
  (∀ P t, (P, t) ∈ tab → strat P ≤ s) ∧
  (∀ P t, (P, t) ∈ tab → strat P = s →
    t ∈ lfpP (stratumOp C p strat W.den ρ s
      (finished C p strat W.den ρ s)) P)

/-- One round preserves the invariant — the soundness half rides
`stratumOp_mono` (new tuples land inside the least fixpoint because
the table's working stratum already sits inside it). -/
theorem stratumStep_inv (C : Classify) (W : ListInstance)
    (ρ : ParamEnv) (p : Program) (strat : PredId → Nat) (s : Nat)
    (hstrat : p.StratifiedBy strat)
    (hsafe : ∀ r, r ∈ p.rulesList → r.Safe) :
    ∀ tab, StratumInv C W ρ p strat s tab →
      StratumInv C W ρ p strat s (stratumStep C W ρ p strat s tab) := by
  rintro tab ⟨hdom, hlow, hle, hlfp⟩
  have henv := progEnv_stratumEnv (C := C) (ρ := ρ) hlow hle
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro P t ht v hv
    rcases mem_stratumStep.mp ht with h | ⟨hs', d, hd, r, hr, hpe⟩
    · exact hdom P t h v hv
    · exact pevalRule_dom (rule_mem_rulesList hd hr)
        (hsafe r (rule_mem_rulesList hd hr)) hdom t hpe v hv
  · intro P t hPlt
    constructor
    · intro ht
      rcases mem_stratumStep.mp ht with h | ⟨hs', -⟩
      · exact (hlow P t hPlt).mp h
      · omega
    · intro h
      exact List.mem_append.mpr (Or.inl ((hlow P t hPlt).mpr h))
  · intro P t ht
    rcases mem_stratumStep.mp ht with h | ⟨hs', -⟩
    · exact hle P t h
    · omega
  · intro P t ht hPs
    rcases mem_stratumStep.mp ht with h | ⟨-, d, hd, r, hr, hpe⟩
    · exact hlfp P t h hPs
    · have hT : t ∈ stratumOp C p strat W.den ρ s
          (finished C p strat W.den ρ s) (tabSets tab) P :=
        ⟨hPs, d, hd, r, hr,
          (pruleAnswers_congr henv t).mp (pevalRule_sound hpe)⟩
      have hmono := stratumOp_mono (C := C) (p := p) (I := W.den)
        (ρ := ρ) (s := s)
        (lower := finished C p strat W.den ρ s) hstrat
      have hle' : PredSets.le (tabSets tab)
          (fun Q u => if strat Q = s then
            u ∈ lfpP (stratumOp C p strat W.den ρ s
              (finished C p strat W.den ρ s)) Q
          else u ∈ tabSets tab Q) := by
        intro Q u hu
        show (if strat Q = s then
          u ∈ lfpP (stratumOp C p strat W.den ρ s
            (finished C p strat W.den ρ s)) Q
          else u ∈ tabSets tab Q)
        by_cases hQ : strat Q = s
        · rw [if_pos hQ]
          exact hlfp Q u hu hQ
        · rw [if_neg hQ]
          exact hu
      have hTZ := hmono _ _ hle' P t hT
      have hZL := stratumOp_congr (C := C) (p := p) (strat := strat)
        (I := W.den) (ρ := ρ) (s := s)
        (lower := finished C p strat W.den ρ s)
        (X := fun Q u => if strat Q = s then
          u ∈ lfpP (stratumOp C p strat W.den ρ s
            (finished C p strat W.den ρ s)) Q
        else u ∈ tabSets tab Q)
        (Y := lfpP (stratumOp C p strat W.den ρ s
          (finished C p strat W.den ρ s)))
        (fun Q hQ u => by
          show (if strat Q = s then _ else _) ↔ _
          rw [if_pos hQ])
      exact lfpP_prefixed hmono P t ((hZL P t).mp hTZ)

/-- One round grows only inside the stratum's candidate space — the
fuel bound's premise. -/
theorem stratumStep_bound (C : Classify) (W : ListInstance)
    (ρ : ParamEnv) (p : Program) (strat : PredId → Nat) (s : Nat)
    (hsafe : ∀ r, r ∈ p.rulesList → r.Safe) :
    ∀ tab, StratumInv C W ρ p strat s tab →
      ∀ x, x ∈ stratumStep C W ρ p strat s tab →
        x ∈ tab ∨ x ∈ stratumCands p strat s W := by
  rintro tab ⟨hdom, -, -, -⟩ ⟨P, t⟩ hx
  rcases mem_stratumStep.mp hx with h | ⟨hs', d, hd, r, hr, hpe⟩
  · exact Or.inl h
  · refine Or.inr (mem_stratumCands hd hs' hr ?_ ?_)
    · obtain ⟨σ, -, hproj⟩ := pevalRule_sound hpe
      rw [hproj]
      exact List.length_map _
    · exact fun v hv => pevalRule_dom (rule_mem_rulesList hd hr)
        (hsafe r (rule_mem_rulesList hd hr)) hdom t hpe v hv

/-- **The stratum theorem**: entering with lower strata finished, the
fueled loop leaves with THIS stratum finished — the table reads
exactly `finished (s+1)`, in at most candidate-count growing rounds
(the §2 safety theorem's executable half, one stratum at a time). -/
theorem stratumLoop_finished (C : Classify) (W : ListInstance)
    (ρ : ParamEnv) (p : Program) (strat : PredId → Nat) (s : Nat)
    (hstrat : p.StratifiedBy strat)
    (hsafe : ∀ r, r ∈ p.rulesList → r.Safe)
    (hnm : ∀ r, r ∈ p.rulesList → r.BindingsMeasureFree) {tab : Tab}
    (hdom : ∀ P t, (P, t) ∈ tab → ∀ v, v ∈ t → v ∈ progDom p W)
    (hfin : ∀ P t, ((P, t) ∈ tab ↔ finished C p strat W.den ρ s P t)) :
    (∀ P t, ((P, t) ∈ fueledLoop (stratumStep C W ρ p strat s)
        ((stratumCands p strat s W).length + 1) tab ↔
      finished C p strat W.den ρ (s + 1) P t)) ∧
    (∀ P t, (P, t) ∈ fueledLoop (stratumStep C W ρ p strat s)
        ((stratumCands p strat s W).length + 1) tab →
      ∀ v, v ∈ t → v ∈ progDom p W) := by
  have hInv0 : StratumInv C W ρ p strat s tab := by
    refine ⟨hdom, fun P t _ => hfin P t, ?_, ?_⟩
    · intro P t ht
      exact Nat.le_of_lt (finished_tag s ((hfin P t).mp ht))
    · intro P t ht hPs
      have := finished_tag s ((hfin P t).mp ht)
      omega
  obtain ⟨⟨hdom', hlow', hle', hlfp'⟩, hkeep, hclosed⟩ :=
    fueledLoop_fixpoint (stratumStep C W ρ p strat s)
      (stratumCands p strat s W) (StratumInv C W ρ p strat s)
      (fun _ x hx => List.mem_append.mpr (Or.inl hx))
      (stratumStep_inv C W ρ p strat s hstrat hsafe)
      (stratumStep_bound C W ρ p strat s hsafe)
      ((stratumCands p strat s W).length + 1) tab hInv0
      (by
        have := missingCount_le (stratumCands p strat s W) tab
        omega)
  refine ⟨?_, hdom'⟩
  intro P t
  constructor
  · intro ht
    rcases Nat.lt_or_ge (strat P) s with hlt | hge
    · exact Or.inl ((hlow' P t hlt).mp ht)
    · exact Or.inr (hlfp' P t ht (Nat.le_antisymm (hle' P t ht) hge))
  · intro ht
    rcases ht with h | h
    · exact (hlow' P t (finished_tag s h)).mpr h
    · have hpre : PredSets.le (stratumOp C p strat W.den ρ s
          (finished C p strat W.den ρ s)
          (tabSets (fueledLoop (stratumStep C W ρ p strat s)
            ((stratumCands p strat s W).length + 1) tab)))
          (tabSets (fueledLoop (stratumStep C W ρ p strat s)
            ((stratumCands p strat s W).length + 1) tab)) := by
        intro Q u hu
        obtain ⟨hQs, d, hd, r, hr, hans⟩ := hu
        have henv := progEnv_stratumEnv (C := C) (ρ := ρ) hlow' hle'
        have hpe : u ∈ pevalRule C
            (progEnv W (fueledLoop (stratumStep C W ρ p strat s)
              ((stratumCands p strat s W).length + 1) tab)) ρ r :=
          pevalRule_complete (hsafe r (rule_mem_rulesList hd hr))
            (hnm r (rule_mem_rulesList hd hr))
            ((pruleAnswers_congr henv u).mpr hans)
        exact hclosed (Q, u)
          (mem_stratumStep.mpr (Or.inr ⟨hQs, d, hd, r, hr, hpe⟩))
      exact lfpP_le hpre P t h

/-! ## The strata driver and the main theorems -/

/-- The strata, evaluated in order: stratum `s` runs its fueled loop
with the sufficient fuel (`missingCount_le` — candidate count plus
one). -/
def strataEval (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (p : Program) (strat : PredId → Nat) (n : Nat) : Tab :=
  (List.range n).foldl
    (fun tab s => fueledLoop (stratumStep C W ρ p strat s)
      ((stratumCands p strat s W).length + 1) tab) []

theorem strataEval_succ (C : Classify) (W : ListInstance)
    (ρ : ParamEnv) (p : Program) (strat : PredId → Nat) (n : Nat) :
    strataEval C W ρ p strat (n + 1) =
      fueledLoop (stratumStep C W ρ p strat n)
        ((stratumCands p strat n W).length + 1)
        (strataEval C W ρ p strat n) := by
  unfold strataEval
  rw [List.range_succ, List.foldl_append]
  rfl

/-- After the first `n` strata, the table reads `finished n`, on the
active domain — the strata induction. -/
theorem strataEval_finished (C : Classify) (W : ListInstance)
    (ρ : ParamEnv) (p : Program) (strat : PredId → Nat)
    (hstrat : p.StratifiedBy strat)
    (hsafe : ∀ r, r ∈ p.rulesList → r.Safe)
    (hnm : ∀ r, r ∈ p.rulesList → r.BindingsMeasureFree) :
    ∀ n, (∀ P t, ((P, t) ∈ strataEval C W ρ p strat n ↔
        finished C p strat W.den ρ n P t)) ∧
      (∀ P t, (P, t) ∈ strataEval C W ρ p strat n →
        ∀ v, v ∈ t → v ∈ progDom p W) := by
  intro n
  induction n with
  | zero =>
    constructor
    · intro P t
      constructor
      · intro h
        exact absurd h (by simp [strataEval])
      · intro h
        exact absurd h (fun h => h)
    · intro P t h
      exact absurd h (by simp [strataEval])
  | succ n ih =>
    rw [strataEval_succ]
    exact stratumLoop_finished C W ρ p strat n hstrat hsafe hnm
      ih.2 ih.1

/-- The executable evaluation of one predicate: run the strata up
through the predicate's own, then read its table. -/
def evalProgramAt (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (p : Program) (strat : PredId → Nat) (P : PredId) :
    List AnswerTuple :=
  tabAt (strataEval C W ρ p strat (strat P + 1)) P

/-- **The executable evaluator** — the PREPARED recursive arm: the
output predicate's table after its own stratum closes. No consumer
runs it yet — the conformance driver (`Main.lean`) evaluates query
cases only today; the conformance consumer for this arm lands with
the Rust-swoop corpus, alongside the engine discharge. -/
def evalProgram (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (p : Program) (strat : PredId → Nat) : List AnswerTuple :=
  evalProgramAt C W ρ p strat p.output

/-- Executable evaluation agrees with the stratified denotation at
every predicate, membership for membership — sound AND complete,
under exactly the acceptance premises (stratified, safe, measure-free
bindings). -/
theorem evalProgramAt_den (C : Classify) (W : ListInstance)
    (ρ : ParamEnv) (p : Program) (strat : PredId → Nat)
    (hstrat : p.StratifiedBy strat)
    (hsafe : ∀ r, r ∈ p.rulesList → r.Safe)
    (hnm : ∀ r, r ∈ p.rulesList → r.BindingsMeasureFree) (P : PredId) :
    ∀ t, t ∈ evalProgramAt C W ρ p strat P ↔
      t ∈ programDen C p strat W.den ρ P := by
  intro t
  unfold evalProgramAt
  rw [mem_tabAt]
  exact (strataEval_finished C W ρ p strat hstrat hsafe hnm
    (strat P + 1)).1 P t

/-- **`program_eval_sound`** — the recursive `eval_sound`: the fueled
round loop computes exactly `programAnswers` given sufficient fuel,
and the fuel it spends IS the sufficient bound (`missingCount_le`).
The premises are the acceptance rules, program-shaped: stratified
(the strata judge), safe (positive range restriction, per rule), and
measure-free bindings (`DurationInBinding`) — the covenant's Level-1
pattern, verbatim. -/
theorem program_eval_sound (C : Classify) (W : ListInstance)
    (ρ : ParamEnv) (p : Program) (strat : PredId → Nat)
    (hstrat : p.StratifiedBy strat)
    (hsafe : ∀ r, r ∈ p.rulesList → r.Safe)
    (hnm : ∀ r, r ∈ p.rulesList → r.BindingsMeasureFree) :
    ∀ t, t ∈ evalProgram C W ρ p strat ↔
      t ∈ programAnswers C p strat W.den ρ :=
  evalProgramAt_den C W ρ p strat hstrat hsafe hnm p.output

/-- **The safety theorem, cashed (§2)**: over a concrete finite
world, every predicate of a stratified, safe, well-shaped program
denotes a FINITE set — each predicate is a subset of a finite product
of active-domain words, and the executable evaluation lists it. The
walls when a premise falls: `Countermodels.odd_no_fixpoint` (an
unstratified program has no consistent semantics) and
`Countermodels.succ_prefixed_infinite` (a head-creating operator's
chain never stabilizes). -/
theorem program_den_finite (C : Classify) (W : ListInstance)
    (ρ : ParamEnv) (p : Program) (strat : PredId → Nat)
    (hstrat : p.StratifiedBy strat)
    (hsafe : ∀ r, r ∈ p.rulesList → r.Safe)
    (hnm : ∀ r, r ∈ p.rulesList → r.BindingsMeasureFree) :
    ∀ P, (programDen C p strat W.den ρ P).Finite :=
  fun P => ⟨evalProgramAt C W ρ p strat P,
    fun t => (evalProgramAt_den C W ρ p strat hstrat hsafe hnm P t).symm⟩

/-! ## Semi-naive, at the operator level (§3's abstract face) -/

/-- Pointwise-equivalent sets are equal (`funext` + `propext` — core
Lean, the carrier is `α → Prop`). -/
theorem setExt {α : Type u} {s t : Set α} (h : ∀ a, s a ↔ t a) :
    s = t :=
  funext fun a => propext (h a)

/-- The naive chain: start empty, keep everything, add everything
the operator derives — the model fixpoint's round (§6's naive
oracle, abstracted to one operator). -/
def naiveIter {α : Type u} (T : Set α → Set α) : Nat → Set α
  | 0 => fun _ => False
  | k + 1 => fun a => naiveIter T k a ∨ T (naiveIter T k) a

/-- The semi-naive chain: an accumulator and the frontier
`new = T(acc) \ acc` — the delta rewrite's operator-level face. -/
def semiNaiveIter {α : Type u} (T : Set α → Set α) :
    Nat → Set α × Set α
  | 0 => (fun _ => False, fun a => T (fun _ => False) a ∧ ¬ False)
  | k + 1 =>
    (fun a => (semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a,
     fun a => T (fun b => (semiNaiveIter T k).1 b ∨
         (semiNaiveIter T k).2 b) a ∧
       ¬ ((semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a))

/-- The frontier is always `T(acc) \ acc` — the uniform reading of
both equations. -/
theorem semiNaive_delta {α : Type u} (T : Set α → Set α) :
    ∀ k, (semiNaiveIter T k).2 =
      fun a => T (semiNaiveIter T k).1 a ∧ ¬ (semiNaiveIter T k).1 a
  | 0 => rfl
  | _ + 1 => rfl

/-- **Semi-naive agrees with naive (§3)**: iterating on
`new = T(acc) \ acc` walks exactly the naive chain — round for
round, so the two reach every fixpoint together. The union algebra
(`acc ∪ (T(acc) \ acc) = acc ∪ T(acc)`) is the whole content;
re-derivation is absorbed, never re-counted (set semantics). The
delta-variant plans that realize the frontier are §3's mechanism —
docs-side, whole. -/
theorem semi_naive_agrees {α : Type u} (T : Set α → Set α) :
    ∀ k, (semiNaiveIter T k).1 = naiveIter T k
  | 0 => rfl
  | k + 1 => by
    have ih := semi_naive_agrees T k
    show (fun a => (semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a)
      = naiveIter T (k + 1)
    rw [semiNaive_delta T k, ih]
    refine setExt fun a => ?_
    show naiveIter T k a ∨ (T (naiveIter T k) a ∧ ¬ naiveIter T k a) ↔
      naiveIter T k a ∨ T (naiveIter T k) a
    constructor
    · rintro (h | ⟨h, -⟩)
      · exact Or.inl h
      · exact Or.inr h
    · intro h
      by_cases hk : naiveIter T k a
      · exact Or.inl hk
      · rcases h with h | h
        · exact absurd h hk
        · exact Or.inr ⟨h, hk⟩

/-- The frontier accounting loses nothing: accumulator plus frontier
at round `k` is the naive chain's round `k + 1`. -/
theorem semi_naive_same_fixpoint {α : Type u} (T : Set α → Set α)
    (k : Nat) :
    (fun a => (semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a) =
      naiveIter T (k + 1) :=
  semi_naive_agrees T (k + 1)

end Bumbledb.Query
