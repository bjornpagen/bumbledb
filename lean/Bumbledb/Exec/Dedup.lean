import Bumbledb.Query.Aggregates

/-!
# Exec/Dedup — seen-set union and the elision licences (Level 1, PRD 07)

Deduplication as set union, the algorithmic essence only (the
mechanism fence): the seen-set is a first-occurrence fold
(`seenFold`), and every elision the engine performs names a theorem —
the `DistinctWitness` licence (`distinct_witness_licence`), the
`DisjointWitness` licence (`disjoint_witness_licence`), and the
multi-rule union regime's head-projection key law
(`union_regime_head_projection`).

## Bridge notes (the exact Rust consumers)

* **The sinks are where union lives** (`crates/bumbledb/src/exec/
  sink.rs:6-18`): one sink hears every rule of a program, its seen-set
  spanning rules — no merge node, no concat-then-dedup pass exists.
  `seenfold_is_set_semantics` is that seen-set's spec: folding the
  emitted stream through first-occurrence filtering computes exactly
  PRD 04's `queryAnswers` set.
* **`DistinctWitness`** (`plan/fj/provably_distinct.rs:11`) is the
  only licence to construct an aggregate sink without a binding
  seen-set: `AggregateSink::without_seen_set`
  (`exec/sink/aggregate/new.rs:138`) requires the witness by value,
  and the ordinary constructors cannot omit the set
  (`exec/sink.rs:384-388`, the `seen: Option<WordMap<()>>` field).
  `BoundFieldsCoverKey` is the witness's premise;
  `distinct_witness_licence` is its theorem;
  `Countermodels.distinct_premise_load_bearing` is the double-count
  the premise forecloses.
* **`DisjointWitness`** (`plan/fj/provably_disjoint.rs:11`): the
  engine mints it (`provably_disjoint_rules`) and spends it
  **diagnostically only** — plan introspection renders
  `disjoint_rules: proven (R.f)`, but execution always keeps the one
  spanning head-projection seen-set: the measured cross-rule elision
  refutation (docs/architecture/40-execution.md § set semantics,
  "Refutation — cross-rule dedup removal") is the doc-side authority,
  cited here and deliberately not restated — performance, not
  semantics. `disjoint_witness_licence` proves what the witness COULD
  license; the docs record why the engine declines.
* **`union_spans`** (`exec/sink.rs:390-398`): the multi-rule union
  regime keys the **head projection** of the binding — per head
  position, the slot span the position reads from THIS rule's binding
  layout — never the rule's full slot array, because dedup keys must
  be rule-independent (a `VarId` is rule-scoped: the same id in two
  rules names two unrelated variables, so a full-binding key has no
  cross-rule meaning). `union_regime_head_projection` is the law. One
  vocabulary gap recorded: the nullary `Count` head position
  contributes NO words to the union key (`union_span` maps
  `over_slot: None` to absence, `new.rs:388-390`) — a keyless head
  position is unrepresentable in the theorem's `VarId` finds; sound,
  since omitting a constant column never changes key equality.

## The `provably_distinct` reading (recorded; theorem 2's model)

`plan/fj/provably_distinct.rs:32-69`: every participating occurrence's
bound fields — variable-bound (`vars`, rs:42-45) or equality-pinned to
one constant (the `Eq`-filter arm, rs:46-58, which admits words,
bytes, intervals, params, and pending interns and EXCLUDES sets:
"set-bound fields pin nothing", rs:28-31) — cover the projection of
one of the relation's declared keys (rs:60-66). Negated occurrences
bind nothing and grounding-eliminated occurrences contribute no facts,
so only participating occurrences are quantified (rs:17-20; here the
positive atom list `Rule.atoms` IS the participating set).
`Term.pins` mirrors the pinned-field screen exactly: `var`, `param`,
and `lit` pin one value under a fixed `(σ, ρ)`; `paramSet` matches any
element and pins nothing; `measure` never appears in a binding
(`Rule.WellTyped`). One asymmetry recorded: the Rust `Eq`-pin arm
admits `Word | Byte | Interval | Param | PendingIntern` and drops
`Const::Words` — the multi-word `bytes<N>` literal, a genuine
single-value pin — to the catch-all (rs:57), so it never counts
toward key coverage; strictly conservative (fewer witness mints, the
seen-set retained), while `Term.pins` marks every `lit` as pinning.
`provably_different` on the disjointness side DOES compare
`Const::Words` payloads — the asymmetry is the mint's, not the
model's.

## The `provably_disjoint` reading (recorded; theorem 6's model)

`plan/fj/provably_disjoint.rs:46-73` (`provably_disjoint_rules`): a
witness `(R, f)` such that EVERY rule pair has, in each rule, a
positive occurrence of `R` whose filters `Eq`-pin `f` to provably
different concrete literals (`pinned_fields`, rs:112-121;
`provably_different`, rs:126-145 — params, sets, and mixed constant
forms pin nothing, conservatively), AND some key of `R` value-bound in
both occurrences with every key column flowing to a common head
position (`key_flows_to_common_head`, rs:154-172; `head_reads`,
rs:188-203 — projected variables and fold inputs enter the dedup key;
the nullary `Count`, Arg terms, and the non-injective measure
positions witness nothing). Equal head answers would force the two
pinned facts to agree on the key — one fact whose `f` cannot equal two
different literals. `ProvablyDisjointRules` models this rule ONE KEY
AT A TIME: pins are `lit` bindings at the witness field (the model's
`Eq`-pin — `provably_different` degenerates to `Value` disequality,
since only concrete literals are representable as pins here), key
flow is positional agreement on the two find lists (`zip`), and the
key itself enters as a semantic `Functionality` hypothesis (PRD 03's
judgment — the schema-declared key the checker consults, discharged
on committed instances by `holds`). One quantifier gap recorded: the
model fixes a single `K` program-wide, while `pair_disjoint` picks a
declared key PER RULE PAIR (`key_flows_to_common_head`, rs:162,
invoked per pair) — an acceptance discharged by heterogeneous keys
across pairs is covered pair-by-pair by this theorem's statement but
not by one instantiation of it; diagnostic-only stakes (the witness
is never spent by execution). `syntactic_disjointness_sound` is the
SOUNDNESS direction only; completeness is explicitly a non-goal — the
checker may refuse truly disjoint rules (any pins it cannot compare,
any key that fails to reach a common head position), and that
conservatism is its correctness discipline, not a defect.

## Narrowings recorded (law 5: narrow and record)

* **Derivation events are an abstract type `ε`.** The licences
  quantify over an event list with observers (`facts`, `bind`) rather
  than modeling the join's enumeration order — WHICH events arrive is
  Free Join mechanism (doc-side); the theorems need only that each
  event is a valid match selection and that distinct events carry
  distinct fact tuples (the join visits each fact combination once).
* **Keys enter as `Functionality` hypotheses.** The Rust checks read
  DECLARED schema keys; the semantic content a declared key has on a
  committed instance is PRD 03's `Functionality` via `holds` (PRD 09),
  so the theorems take it directly — acceptance-vs-denotation kept
  separate, as in `Dependencies.lean`.
* **The single-rule slot-array key is `slots.map σ`** with `slots`
  covering the rule's atom variables — the `SlotWidth` word layout
  (how many words a value occupies) is mechanism; the model keys
  whole values.
-/

namespace Bumbledb.Query

/-! ## `seenFold` — the seen-set as a fold

First-occurrence filtering: the fold carries the seen-set and emits a
row exactly when its key is fresh — the Lean image of the sinks'
`WordMap` insert-if-absent. PRD 05's `dedup` (last-occurrence) has the
same membership and the same distinctness; `seenFold` is defined
separately because the ENGINE's fold is first-occurrence (a row is
emitted or absorbed the moment it arrives, never revised), and the
emission ORDER is the one observable that distinguishes the two. -/

/-- The seen-set fold, seeded: emit `x` iff `x` is not yet seen,
folding left with the seen-set accumulating. -/
def seenFoldAux {β : Type} [DecidableEq β] (seen : List β) :
    List β → List β
  | [] => []
  | x :: xs =>
    if x ∈ seen then seenFoldAux seen xs
    else x :: seenFoldAux (x :: seen) xs

/-- **`seenFold`** — first-occurrence filtering: the seen-set as a
fold, seeded empty (the sink's seen-set is reset once per execution,
never per rule — `exec/sink.rs:6-18`). -/
def seenFold {β : Type} [DecidableEq β] (l : List β) : List β :=
  seenFoldAux [] l

/-- Membership through the seeded fold: emitted iff present and not
already seen. -/
theorem mem_seenFoldAux {β : Type} [DecidableEq β] {x : β} :
    ∀ {l seen : List β}, x ∈ seenFoldAux seen l ↔ x ∈ l ∧ x ∉ seen
  | [], seen => by simp [seenFoldAux]
  | y :: ys, seen => by
    unfold seenFoldAux
    by_cases hy : y ∈ seen
    · rw [if_pos hy]
      constructor
      · intro h
        obtain ⟨hx, hns⟩ := mem_seenFoldAux (l := ys).mp h
        exact ⟨List.mem_cons_of_mem _ hx, hns⟩
      · rintro ⟨hx, hns⟩
        refine mem_seenFoldAux (l := ys).mpr ⟨?_, hns⟩
        rcases List.mem_cons.mp hx with rfl | hx'
        · exact absurd hy hns
        · exact hx'
    · rw [if_neg hy]
      constructor
      · intro h
        rcases List.mem_cons.mp h with rfl | h'
        · exact ⟨List.mem_cons_self .., hy⟩
        · obtain ⟨hx, hns⟩ := mem_seenFoldAux (l := ys).mp h'
          exact ⟨List.mem_cons_of_mem _ hx,
            fun hs => hns (List.mem_cons_of_mem _ hs)⟩
      · rintro ⟨hx, hns⟩
        by_cases hxy : x = y
        · exact List.mem_cons.mpr (.inl hxy)
        · refine List.mem_cons.mpr
            (.inr (mem_seenFoldAux (l := ys).mpr ⟨?_, ?_⟩))
          · rcases List.mem_cons.mp hx with h | h
            · exact absurd h hxy
            · exact h
          · intro hs
            rcases List.mem_cons.mp hs with h | h
            · exact hxy h
            · exact hns h

/-- The seen-set filter preserves membership exactly: what survives is
what arrived. -/
theorem mem_seenFold {β : Type} [DecidableEq β] {x : β} {l : List β} :
    x ∈ seenFold l ↔ x ∈ l :=
  ⟨fun h => (mem_seenFoldAux.mp h).1,
   fun h => mem_seenFoldAux.mpr ⟨h, fun hs => nomatch hs⟩⟩

/-- The seeded fold's output is distinct: an emitted row enters the
seen-set, and the recursion never re-emits a seen key. -/
theorem seenFoldAux_nodup {β : Type} [DecidableEq β] :
    ∀ (l seen : List β), (seenFoldAux seen l).Nodup
  | [], _ => List.Pairwise.nil
  | y :: ys, seen => by
    unfold seenFoldAux
    by_cases hy : y ∈ seen
    · rw [if_pos hy]
      exact seenFoldAux_nodup ys seen
    · rw [if_neg hy]
      refine List.pairwise_cons.mpr
        ⟨?_, seenFoldAux_nodup ys (y :: seen)⟩
      intro z hz heq
      obtain ⟨-, hns⟩ := mem_seenFoldAux.mp hz
      exact hns (by rw [← heq]; exact List.mem_cons_self ..)

/-- The seen-set's output is duplicate-free. -/
theorem seenFold_nodup {β : Type} [DecidableEq β] (l : List β) :
    (seenFold l).Nodup :=
  seenFoldAux_nodup l []

/-- On a duplicate-free stream the seeded fold is the identity — the
elision reading: a seen-set over a provably distinct stream filters
nothing. -/
theorem seenFoldAux_eq_of_nodup {β : Type} [DecidableEq β] :
    ∀ {l : List β} (seen : List β), l.Nodup → (∀ x ∈ l, x ∉ seen) →
      seenFoldAux seen l = l
  | [], _, _, _ => rfl
  | y :: ys, seen, hnd, hdisj => by
    obtain ⟨hhd, htl⟩ := List.pairwise_cons.mp hnd
    unfold seenFoldAux
    rw [if_neg (hdisj y (List.mem_cons_self ..))]
    refine congrArg (y :: ·) (seenFoldAux_eq_of_nodup (y :: seen) htl ?_)
    intro x hx hs
    rcases List.mem_cons.mp hs with rfl | hs'
    · exact hhd x hx rfl
    · exact hdisj x (List.mem_cons_of_mem _ hx) hs'

/-- `seenFold` is the identity on duplicate-free streams. -/
theorem seenFold_eq_of_nodup {β : Type} [DecidableEq β] {l : List β}
    (h : l.Nodup) : seenFold l = l :=
  seenFoldAux_eq_of_nodup [] h (fun _ _ hs => nomatch hs)

/-- PRD 05's `dedup` is also the identity on duplicate-free streams —
the bridge between the two representations of "the distinct set". -/
theorem dedup_eq_of_nodup {β : Type} [DecidableEq β] :
    ∀ {l : List β}, l.Nodup → dedup l = l
  | [], _ => rfl
  | x :: xs, h => by
    obtain ⟨hhd, htl⟩ := List.pairwise_cons.mp h
    unfold dedup
    rw [if_neg (fun hmem => hhd x hmem rfl), dedup_eq_of_nodup htl]

/-! ## Theorem 1 — the seen-set IS set semantics -/

/-- **Theorem 1 (`seenfold_is_set_semantics`).** Folding an
enumeration of the emitted answers through the seen-set computes the
answer SET: same membership as PRD 04's `queryAnswers`, no duplicates
— dedup-by-fold is the denotation, which is why "union is not an
operator" is implementable at all. Bridge: the projection and
aggregate sinks' seen-sets (`exec/sink.rs` — the module doc's
"the sinks are where union lives"); `union_idempotent` is the same
fact at the denotation level. -/
theorem seenfold_is_set_semantics {C : Classify} {q : Query}
    {I : Instance} {ρ : ParamEnv} {l : List AnswerTuple}
    (henum : ∀ t, t ∈ l ↔ t ∈ queryAnswers C q I ρ) :
    (∀ t, t ∈ seenFold l ↔ t ∈ queryAnswers C q I ρ) ∧
      (seenFold l).Nodup :=
  ⟨fun t => mem_seenFold.trans (henum t), seenFold_nodup l⟩

/-! ## Pinned bindings — the bound-field screen -/

/-- A term PINS its field: under a fixed `(σ, ρ)` it forces the
field to exactly one value. The `provably_distinct` bound-field screen
(`plan/fj/provably_distinct.rs:42-58`): variable-bound (`var`),
equality-pinned to one constant (`lit`, and `param` — resolved at
bind, one value per execution). `paramSet` matches any element of the
slice and pins nothing (rs:28-31, "set-bound fields pin nothing");
`measure` never appears in an accepted binding
(`ValidationError::DurationInBinding`, `Rule.WellTyped`). -/
def Term.pins : Term → Prop
  | .var _ | .param _ | .lit _ => True
  | .paramSet _ | .measure _ => False

/-- A pinning term selects at most ONE value: two selections under
one `(σ, ρ)` agree — the pin, cashed. -/
theorem Term.pins_selects_unique {ρ : ParamEnv} {σ : Assignment}
    {t : Term} {w w' : Value} (hp : t.pins)
    (h : Term.selects ρ σ t w) (h' : Term.selects ρ σ t w') :
    w = w' := by
  cases t with
  | var v => exact h.symm.trans h'
  | param p => exact h.symm.trans h'
  | lit c => exact h.symm.trans h'
  | paramSet p => exact hp.elim
  | measure v => exact hp.elim

/-- Two equal variable-projections agree on every projected
variable. -/
theorem map_eq_agree {σ σ' : Assignment} :
    ∀ {slots : List VarId}, slots.map σ = slots.map σ' →
      ∀ v, v ∈ slots → σ v = σ' v
  | [], _, _, hv => nomatch hv
  | s :: ss, heq, v, hv => by
    rw [List.map_cons, List.map_cons] at heq
    injection heq with h1 h2
    rcases List.mem_cons.mp hv with rfl | hv'
    · exact h1
    · exact map_eq_agree h2 v hv'

/-! ## `BoundFieldsCoverKey` — the `DistinctWitness` premise -/

/-- One occurrence's bound fields cover a key: some field list `K`
that is a semantic key of the atom's relation extension
(PRD 03's `Functionality` — the declared key's judgment on the
instance) with every field of `K` pinned by one of the atom's
bindings. The per-occurrence clause of
`plan/fj/provably_distinct.rs:40-67`. -/
def CoversKey (I : Instance) (a : Atom) : Prop :=
  ∃ K : List FieldId, Functionality (I a.relation) K ∧
    ∀ i, i ∈ K → ∃ t, (i, t) ∈ a.bindings ∧ t.pins

/-- **`BoundFieldsCoverKey`** — the distinct-bindings elision law's
premise: every participating occurrence's bound fields cover a key of
its relation. Positive atoms only — negated occurrences bind nothing
(they only reject: the anti-join `¬∃` of `derives`), exactly the
participation screen of `provably_distinct.rs:37-39`. This is the
statement `DistinctWitness` (`plan/fj/provably_distinct.rs:11`)
carries as evidence. -/
def BoundFieldsCoverKey (r : Rule) (I : Instance) : Prop :=
  ∀ a, a ∈ r.atoms → CoversKey I a

/-- A key-covered occurrence is FUNCTIONAL in the binding: under one
assignment, at most one fact of the extension matches — every pinned
key field forces one value, and the key forces one fact. The
per-occurrence pigeonhole every licence spends. -/
theorem covered_occurrence_functional {I : Instance} {a : Atom}
    {ρ : ParamEnv} {σ : Assignment} (hcov : CoversKey I a)
    {f g : Fact} (hf : f ∈ I a.relation) (hg : g ∈ I a.relation)
    (hmf : Matches f a σ ρ) (hmg : Matches g a σ ρ) : f = g := by
  obtain ⟨K, hkey, hpin⟩ := hcov
  refine hkey f g hf hg ((Fact.project_eq_iff f g K).mpr fun i hi => ?_)
  obtain ⟨t, hb, hp⟩ := hpin i hi
  exact Term.pins_selects_unique hp (hmf (i, t) hb) (hmg (i, t) hb)

/-! ## Theorem 2 — the `DistinctWitness` licence -/

/-- A match selection: one matching fact per positive atom — the
fact-tuple face of one derivation event (the join emits one binding
per fact combination). -/
def MatchSelection (r : Rule) (I : Instance) (ρ : ParamEnv)
    (σ : Assignment) (w : Atom → Fact) : Prop :=
  ∀ a, a ∈ r.atoms → w a ∈ I a.relation ∧ Matches (w a) a σ ρ

/-- **Distinct facts yield distinct full bindings** (contrapositive
form): under the witness premise, one binding admits at most ONE
match selection — two selections producing assignments that agree on
the atoms' variables select the same facts. The heart of the
`DistinctWitness` argument. -/
theorem binding_determines_facts {r : Rule} {I : Instance}
    {ρ : ParamEnv} (DistinctWitness : BoundFieldsCoverKey r I)
    {σ σ' : Assignment} {w w' : Atom → Fact}
    (h : MatchSelection r I ρ σ w) (h' : MatchSelection r I ρ σ' w')
    (hagree : ∀ a, a ∈ r.atoms → ∀ v, v ∈ a.vars → σ v = σ' v) :
    ∀ a, a ∈ r.atoms → w a = w' a := by
  intro a ha
  have hm' : Matches (w' a) a σ ρ :=
    (matches_congr fun v hv => hagree a ha v hv).mpr (h' a ha).2
  exact covered_occurrence_functional (DistinctWitness a ha)
    (h a ha).1 (h' a ha).1 (h a ha).2 hm'

/-- The emitted key stream is duplicate-free under the witness: the
key is the slot array (`slots.map (bind e)` — the single-rule regime
keys the WHOLE slot array, `exec/sink.rs:384-388`), the events are
the join's fact-tuple enumeration (each combination once), and equal
keys would force equal fact tuples through
`binding_determines_facts`. -/
theorem key_stream_nodup {r : Rule} {I : Instance} {ρ : ParamEnv}
    (DistinctWitness : BoundFieldsCoverKey r I) {ε : Type}
    (facts : ε → Atom → Fact) (bind : ε → Assignment)
    (slots : List VarId)
    (hslots : ∀ a, a ∈ r.atoms → ∀ v, v ∈ a.vars → v ∈ slots) :
    ∀ {events : List ε},
      (∀ e, e ∈ events → MatchSelection r I ρ (bind e) (facts e)) →
      events.Pairwise (fun e e' =>
        ∃ a, a ∈ r.atoms ∧ facts e a ≠ facts e' a) →
      (events.map fun e => slots.map (bind e)).Nodup
  | [], _, _ => List.Pairwise.nil
  | e :: es, hvalid, hpw => by
    obtain ⟨hhd, htl⟩ := List.pairwise_cons.mp hpw
    rw [List.map_cons]
    refine List.pairwise_cons.mpr
      ⟨?_, key_stream_nodup DistinctWitness facts bind slots hslots
        (fun e' he' => hvalid e' (List.mem_cons_of_mem _ he')) htl⟩
    intro k hk hkeq
    obtain ⟨e', he', rfl⟩ := List.mem_map.mp hk
    obtain ⟨a, ha, hne⟩ := hhd e' he'
    have hagree := map_eq_agree hkeq
    exact hne (binding_determines_facts DistinctWitness
      (hvalid e (List.mem_cons_self ..))
      (hvalid e' (List.mem_cons_of_mem _ he'))
      (fun a' ha' v hv => hagree v (hslots a' ha' v hv)) a ha)

/-- **Theorem 2 (`distinct_witness_licence`).** Under
`BoundFieldsCoverKey` — the hypothesis is NAMED after the witness the
plan mints — distinct facts yield distinct full bindings, so the
emitted key stream is already duplicate-free and folding WITHOUT the
seen-set computes the same aggregate as folding the distinct set:
`fold stream = fold (dedup stream)` — the right side is the normative
fold domain ("every aggregate folds the DISTINCT binding set",
`agg_over_distinct_bindings`), the left side is the elided path.
Bridge: `DistinctWitness` (`plan/fj/provably_distinct.rs:11` — the
only mint is `provably_distinct`, rs:32);
`AggregateSink::without_seen_set` (`exec/sink/aggregate/new.rs:138`)
requires the witness by value — construction cannot enter the elided
regime without this theorem's premise. Single-rule only: the
multi-rule union keeps its spanning head-projection seen-set even
when every rule carries its own witness
(docs/architecture/40-execution.md § the rule loop). The premise is
load-bearing: `Countermodels.distinct_premise_load_bearing` is the
unkeyed occurrence whose `Sum` double-counts under elision. -/
theorem distinct_witness_licence {γ : Type} {r : Rule} {I : Instance}
    {ρ : ParamEnv} (DistinctWitness : BoundFieldsCoverKey r I)
    {ε : Type} (events : List ε) (facts : ε → Atom → Fact)
    (bind : ε → Assignment) (slots : List VarId)
    (hslots : ∀ a, a ∈ r.atoms → ∀ v, v ∈ a.vars → v ∈ slots)
    (hvalid : ∀ e, e ∈ events → MatchSelection r I ρ (bind e) (facts e))
    (hdistinct : events.Pairwise fun e e' =>
      ∃ a, a ∈ r.atoms ∧ facts e a ≠ facts e' a)
    (fold : List (List Value) → γ) :
    (events.map fun e => slots.map (bind e)).Nodup ∧
      fold (events.map fun e => slots.map (bind e)) =
        fold (dedup (events.map fun e => slots.map (bind e))) := by
  have hnd := key_stream_nodup DistinctWitness facts bind slots hslots
    hvalid hdistinct
  exact ⟨hnd, by rw [dedup_eq_of_nodup hnd]⟩

/-! ## Theorem 4 — the `DisjointWitness` licence -/

/-- **`DisjointArms`** — the semantic property the syntactic check
approximates: no answer tuple derives from two different rules of the
program (pairwise over rule positions, so a literally duplicated rule
is NOT disjoint from itself — `union_idempotent` owns that case). -/
def DisjointArms (C : Classify) (q : Query) (I : Instance)
    (ρ : ParamEnv) : Prop :=
  q.rules.Pairwise fun r r' =>
    ∀ t, t ∈ ruleAnswers C r I ρ → t ∉ ruleAnswers C r' I ρ

/-- The induction behind the licence, over plain rule lists: per-arm
distinct enumerations concatenate — under pairwise arm disjointness —
into a duplicate-free enumeration of the union. -/
theorem disjoint_flatten {C : Classify} {I : Instance} {ρ : ParamEnv} :
    ∀ {arms : List (List AnswerTuple)} {rules : List Rule},
      arms.length = rules.length →
      (∀ p, p ∈ arms.zip rules →
        (∀ t, t ∈ p.1 ↔ t ∈ ruleAnswers C p.2 I ρ) ∧ p.1.Nodup) →
      rules.Pairwise (fun r r' => ∀ t, t ∈ ruleAnswers C r I ρ →
        t ∉ ruleAnswers C r' I ρ) →
      arms.flatten.Nodup ∧
        ∀ t, t ∈ arms.flatten ↔ ∃ r, r ∈ rules ∧ t ∈ ruleAnswers C r I ρ
  | [], [], _, _, _ => ⟨List.Pairwise.nil, by simp⟩
  | [], _ :: _, hlen, _, _ => by simp at hlen
  | _ :: _, [], hlen, _, _ => by simp at hlen
  | l :: ls, r :: rs, hlen, henum, hpw => by
    obtain ⟨hhd, htl⟩ := List.pairwise_cons.mp hpw
    have hp := henum (l, r)
      (by rw [List.zip_cons_cons]; exact List.mem_cons_self ..)
    have ih := disjoint_flatten (arms := ls) (rules := rs)
      (Nat.succ.inj hlen)
      (fun p hp' => henum p
        (by rw [List.zip_cons_cons]; exact List.mem_cons_of_mem _ hp'))
      htl
    constructor
    · rw [List.flatten_cons]
      refine List.pairwise_append.mpr ⟨hp.2, ih.1, ?_⟩
      intro a ha b hb heq
      subst heq
      obtain ⟨r', hr', hmem'⟩ := (ih.2 a).mp hb
      exact hhd r' hr' a ((hp.1 a).mp ha) hmem'
    · intro t
      rw [List.flatten_cons, List.mem_append]
      constructor
      · rintro (h | h)
        · exact ⟨r, List.mem_cons_self .., (hp.1 t).mp h⟩
        · obtain ⟨r', hr', hm⟩ := (ih.2 t).mp h
          exact ⟨r', List.mem_cons_of_mem _ hr', hm⟩
      · rintro ⟨r', hr', hm⟩
        rcases List.mem_cons.mp hr' with rfl | hr''
        · exact .inl ((hp.1 t).mpr hm)
        · exact .inr ((ih.2 t).mpr ⟨r', hr'', hm⟩)

/-- **Theorem 4 (`disjoint_witness_licence`).** Under `DisjointArms`
— the hypothesis is NAMED after the witness — cross-rule dedup is a
no-op: concatenating the rules' distinct answer streams is already
duplicate-free, its set is exactly the query union, and the spanning
seen-set filters nothing (`seenFold` is the identity on it).
Bridge: `DisjointWitness` (`plan/fj/provably_disjoint.rs:11`). The
engine SPENDS this witness diagnostically only — plan introspection's
`disjoint_rules: proven (R.f)` line — and keeps the spanning
head-projection seen-set regardless: the measured cross-rule elision
refutation (docs/architecture/40-execution.md § set semantics,
"Refutation — cross-rule dedup removal") rejected the per-rule-drain
representation on the clock, and that record is doc-side authority,
cited here, not restated. This theorem proves the elision SOUND; the
docs record why sound is not the same as worth it. -/
theorem disjoint_witness_licence {C : Classify} {q : Query}
    {I : Instance} {ρ : ParamEnv}
    (DisjointWitness : DisjointArms C q I ρ)
    {arms : List (List AnswerTuple)}
    (hlen : arms.length = q.rules.length)
    (henum : ∀ p, p ∈ arms.zip q.rules →
      (∀ t, t ∈ p.1 ↔ t ∈ ruleAnswers C p.2 I ρ) ∧ p.1.Nodup) :
    arms.flatten.Nodup ∧
      seenFold arms.flatten = arms.flatten ∧
      ∀ t, t ∈ arms.flatten ↔ t ∈ queryAnswers C q I ρ := by
  obtain ⟨hnd, hmem⟩ := disjoint_flatten hlen henum DisjointWitness
  exact ⟨hnd, seenFold_eq_of_nodup hnd,
    fun t => (hmem t).trans mem_queryAnswers.symm⟩

/-! ## Theorem 5 — the union regime keys the head projection -/

/-- **Theorem 5 (`union_regime_head_projection`).** When rules share
the union seen-set, dedup keys the projected HEAD tuple — never the
full binding: seen-filtering the head-projected derivation stream of
a multi-rule program computes exactly `queryAnswers`, with a later
rule's re-derivation absorbed like a within-rule duplicate. The key
must be head-shaped for the spanning set to mean anything: a `VarId`
is rule-scoped (two rules' slot arrays are incomparable), and
`answer_identity_canonical` is why the head tuple is a COMPLETE key.
Bridge: `union_spans` (`exec/sink.rs:390-398`) — per head position,
the slot span the position reads from THIS rule's binding layout; the
extracted words are the head projection, rule-independent by
construction ("aggregates read the head: the fold domain is the union
of the rules' binding sets projected to the head"). -/
theorem union_regime_head_projection {C : Classify} {q : Query}
    {I : Instance} {ρ : ParamEnv} {ε : Type} (events : List ε)
    (rule : ε → Rule) (bind : ε → Assignment)
    (hvalid : ∀ e, e ∈ events →
      rule e ∈ q.rules ∧ derives C (rule e) I ρ (bind e))
    (hcomplete : ∀ r, r ∈ q.rules → ∀ σ, derives C r I ρ σ →
      ((r.finds.map σ : AnswerTuple) ∈
        events.map fun e => (rule e).finds.map (bind e))) :
    (∀ t, t ∈ seenFold (events.map fun e => (rule e).finds.map (bind e))
        ↔ t ∈ queryAnswers C q I ρ) ∧
      (seenFold (events.map fun e =>
        (rule e).finds.map (bind e))).Nodup := by
  refine ⟨fun t => ?_, seenFold_nodup _⟩
  rw [mem_seenFold]
  constructor
  · intro ht
    obtain ⟨e, he, rfl⟩ := List.mem_map.mp ht
    exact mem_queryAnswers.mpr
      ⟨rule e, (hvalid e he).1, bind e, (hvalid e he).2, rfl⟩
  · intro ht
    obtain ⟨r, hr, σ, hd, rfl⟩ := mem_queryAnswers.mp ht
    exact hcomplete r hr σ hd

/-! ## Theorem 6 — the syntactic check is sound -/

/-- Positional head agreement carried through equal projections: the
common head position forces the two assignments to agree on the
zipped variable pair. -/
theorem map_eq_of_zip_mem {σ σ' : Assignment} {v v' : VarId} :
    ∀ {l l' : List VarId}, l.map σ = l'.map σ' →
      (v, v') ∈ l.zip l' → σ v = σ' v'
  | [], [], _, hmem => nomatch hmem
  | [], _ :: _, _, hmem => nomatch hmem
  | _ :: _, [], _, hmem => nomatch hmem
  | a :: l, a' :: l', heq, hmem => by
    rw [List.map_cons, List.map_cons] at heq
    injection heq with h1 h2
    rw [List.zip_cons_cons] at hmem
    rcases List.mem_cons.mp hmem with hpair | hmem'
    · injection hpair with hv hv'
      subst hv; subst hv'
      exact h1
    · exact map_eq_of_zip_mem h2 hmem'

/-- One rule pair under one witness `(R, fld, K)` — the model of
`pair_disjoint` (`plan/fj/provably_disjoint.rs:78-90`): each rule has
a positive occurrence of `R` pinning `fld` to provably different
literals (`lit` bindings, the model's `Eq`-pins — only concrete
literals are representable as pins, so `provably_different` is plain
`Value` disequality), and every field of the key `K` is variable-bound
in both occurrences with the two variables at a common head position
(the `zip` clause — `key_flows_to_common_head`, rs:154-172). -/
def ArmPin (R : RelId) (fld : FieldId) (K : List FieldId)
    (r r' : Rule) : Prop :=
  ∃ a, a ∈ r.atoms ∧ ∃ a', a' ∈ r'.atoms ∧
    a.relation = R ∧ a'.relation = R ∧
    (∃ c c' : Value, (fld, Term.lit c) ∈ a.bindings ∧
      (fld, Term.lit c') ∈ a'.bindings ∧ c ≠ c') ∧
    ∀ i, i ∈ K → ∃ v v' : VarId, (i, Term.var v) ∈ a.bindings ∧
      (i, Term.var v') ∈ a'.bindings ∧ (v, v') ∈ r.finds.zip r'.finds

/-- The check, program-level: one witness discharging every rule pair
— `provably_disjoint_rules` (`plan/fj/provably_disjoint.rs:46-73`,
"pairwise over all rules; one witness for every pair"). -/
def ProvablyDisjointRules (q : Query) (R : RelId) (fld : FieldId)
    (K : List FieldId) : Prop :=
  q.rules.Pairwise (ArmPin R fld K)

/-- The pair soundness: equal head answers force the two pinned facts
through the key onto ONE fact of `R`, whose `fld` cannot equal two
different literals. -/
theorem armPin_disjoint {C : Classify} {I : Instance} {ρ : ParamEnv}
    {R : RelId} {fld : FieldId} {K : List FieldId}
    (hkey : Functionality (I R) K) {r r' : Rule}
    (hpin : ArmPin R fld K r r') :
    ∀ t, t ∈ ruleAnswers C r I ρ → t ∉ ruleAnswers C r' I ρ := by
  intro t ht ht'
  obtain ⟨σ, hd, heq⟩ := mem_ruleAnswers.mp ht
  obtain ⟨σ', hd', heq'⟩ := mem_ruleAnswers.mp ht'
  obtain ⟨a, ha, a', ha', hR, hR', ⟨c, c', hc, hc', hne⟩, hflow⟩ := hpin
  obtain ⟨f, hf, hmf⟩ := hd.1 a ha
  obtain ⟨f', hf', hmf'⟩ := hd'.1 a' ha'
  have hpin1 : c = f fld := hmf (fld, Term.lit c) hc
  have hpin2 : c' = f' fld := hmf' (fld, Term.lit c') hc'
  have hproj : f.project K = f'.project K := by
    refine (Fact.project_eq_iff f f' K).mpr fun i hi => ?_
    obtain ⟨v, v', hbv, hbv', hz⟩ := hflow i hi
    have h1 : σ v = f i := hmf (i, Term.var v) hbv
    have h2 : σ' v' = f' i := hmf' (i, Term.var v') hbv'
    have h3 : σ v = σ' v' :=
      map_eq_of_zip_mem (heq.symm.trans heq') hz
    rw [← h1, ← h2, h3]
  have hone : f = f' := hkey f f' (hR ▸ hf) (hR' ▸ hf') hproj
  exact hne (by rw [hpin1, hpin2, hone])

/-- **Theorem 6 (`syntactic_disjointness_sound`).** The syntactic
check is SOUND: a program `provably_disjoint_rules` accepts under a
witness `(R, fld)` and a semantically keyed `K` has `DisjointArms` on
every instance where the key holds. The SOUNDNESS direction only —
completeness is explicitly a non-goal: the checker may refuse truly
disjoint programs (pins it cannot compare — params, mixed constant
forms; keys that never reach a common head position), and its
conservatism is the discipline that keeps `None` honest, never a
defect to fix (`plan/fj/provably_disjoint.rs:26-44`, "conservative
and sound"). Bridge: `provably_disjoint_rules` is the only mint of
`DisjointWitness`; the semantic key premise is the schema-declared
`Functionality` the check reads (`schema.relation(..).keys()`,
rs:162), discharged on committed instances by PRD 03's `holds`. -/
theorem syntactic_disjointness_sound {C : Classify} {q : Query}
    {I : Instance} {ρ : ParamEnv} {R : RelId} {fld : FieldId}
    {K : List FieldId} (hkey : Functionality (I R) K)
    (hsyn : ProvablyDisjointRules q R fld K) :
    DisjointArms C q I ρ :=
  hsyn.imp fun hpin => armPin_disjoint hkey hpin

end Bumbledb.Query
