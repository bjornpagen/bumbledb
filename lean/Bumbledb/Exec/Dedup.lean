import Bumbledb.Query.Aggregates
import Bumbledb.Query.Membership
import Bumbledb.Exec.Rewrites

/-!
# Exec/Dedup — seen-set union and the elision licences (Level 1, PRD 07)

Deduplication as set union, the algorithmic essence only (the
mechanism fence): the seen-set is a first-occurrence fold
(`seenFold`), and every elision the engine performs names a theorem —
the `DistinctWitness` licence (`distinct_witness_licence`), the
`DisjointWitness` licence (`disjoint_witness_licence`), and the
multi-rule union regime's head-projection key law
(`union_regime_head_projection`). The aggregate face of ELIMINATION
lives here too (2026-07-14, the admission-calculus docket — this
module is where the fold domains live, hence the `Exec/Rewrites`
import): `elimination_agg_fold_domain` proves the eliminated
occurrence's key-backed join contributes exactly one extension per
surviving binding — the containment supplies existence
(`elim_extension_exists`), the target key uniqueness
(`elim_extension_unique`) — so the distinct-full-binding domain the
fold reads projects bijectively onto the surviving slots. The
aggregate face is then TWO theorems, deliberately:
`elimination_agg_domain_counts` (the key premise spent — the
full-slot-array domain the engine's sink keys and the surviving-slot
domain carry the same counts, floor and ceiling) and
`elimination_agg_sound` (the containment spent — answer identity
fiber for fiber, both folds read at the surviving slots; its carrier
`aggAnswersOn` is the normative `aggAnswers` with the fiber handed
through the slot projection, definitionally —
`aggAnswersOn_eq_aggAnswers`, the recorded link). Recorded
scope, loudly: an arbitrary abstract fold cannot be STATED over both
domains at once (the two domains' tuples have different widths), so
no single theorem carries "answer identity for the engine's
full-slot fold" for every fold parameter — a count-observing fold
gets it from the counts transport, a surviving-slot-reading fold
from the fiber identity, and any further fold shape must name which
of the two it spends.

## Bridge notes (the exact Rust consumers)

* **The sinks are where union lives** (`exec/sink.rs`'s module doc;
  the two consumers are `exec/sink.rs::ProjectionSink` and
  `exec/sink.rs::AggregateSink`): one sink hears every rule of a
  program, its seen-set
  spanning rules — no merge node, no concat-then-dedup pass exists.
  `seenfold_is_set_semantics` is that seen-set's spec: folding the
  emitted stream through first-occurrence filtering computes exactly
  PRD 04's `queryAnswers` set.
* **`DistinctWitness`** (`plan/fj/provably_distinct.rs::DistinctWitness`)
  is the only licence to construct an aggregate sink without a binding
  seen-set: `AggregateSink::without_seen_set`
  (`exec/sink/aggregate/new.rs::without_seen_set`) requires the
  witness by value, and the ordinary constructors cannot omit the set
  (the `seen: Option<WordMap<()>>` field of
  `exec/sink.rs::AggregateSink`).
  `BoundFieldsCoverKey` is the witness's premise;
  `distinct_witness_licence` is its theorem;
  `Countermodels.distinct_premise_load_bearing` is the double-count
  the premise forecloses.
* **`DisjointWitness`** (`plan/fj/provably_disjoint.rs::DisjointWitness`):
  the engine mints it
  (`plan/fj/provably_disjoint.rs::provably_disjoint_rules`) and spends it
  **diagnostically only** — plan introspection renders
  `disjoint_rules: proven (R.f)`, but execution always keeps the one
  spanning head-projection seen-set: the measured cross-rule elision
  refutation (docs/architecture/40-execution.md § set semantics,
  "Refutation — cross-rule dedup removal") is the doc-side authority,
  cited here and deliberately not restated — performance, not
  semantics. `disjoint_witness_licence` proves what the witness COULD
  license; the docs record why the engine declines.
* **`union_spans`** (`exec/sink.rs::union_spans`): the multi-rule union
  regime keys the **head projection** of the binding — per head
  position, the slot span the position reads from THIS rule's binding
  layout — never the rule's full slot array, because dedup keys must
  be rule-independent (a `VarId` is rule-scoped: the same id in two
  rules names two unrelated variables, so a full-binding key has no
  cross-rule meaning). `union_regime_head_projection` is the law. One
  vocabulary gap recorded: the nullary `Count` head position
  contributes NO words to the union key
  (`exec/sink/aggregate/new.rs::union_span` maps `over_slot: None` to
  absence) — a keyless head
  position is unrepresentable in the theorem's `VarId` finds; sound,
  since omitting a constant column never changes key equality.
* **The DNF re-key law (ruled 2026-07-23, R2).** Surface `or` is
  fold-transparent: a DNF-DERIVED rule set re-keys the union dedup on
  the SHARED SLOT ARRAY — the disjuncts of one written rule share one
  variable vocabulary and one binding layout (`Rule.lower` splits
  condition trees only), so the rule-scoped-`VarId` objection above
  dissolves and the fold domain never moves. `dnf_rekey_transparent`
  (the closing section) is the law, PROVED: the re-keyed union
  denotation of a lowering equals the written rule's own aggregate
  denotation. The head-projection key governs HAND-WRITTEN multi-rule
  programs only. The R2 agreement, DISCHARGED (2026-07-23 audit):
  `dnf_rekey_stream` is the executable face's spec — seen-filtering a
  complete enumeration's shared-slot rows computes exactly
  `dnfFoldDomain`, duplicate-free — and the conformance glue's
  re-keyed arm (`Conformance.lean`'s `dnfBindings`, taken on the
  serializer's derivation mark) with the engine's re-keyed union sink
  both read exactly that stream.
* **The R1 refusal (ruled 2026-07-23, R1), stated and justified** —
  a validation-model refusal (stated, never proved:
  `CountAcrossRulesAccepted`): a nullary `Count` in a fold-free head
  of a 2+-rule program is a typed validation error beside
  `ArgAcrossRules`. Under the head-projection law its answer is
  definitionally the constant 1 per group — `foldfree_head_constant`
  PROVES the uninformativeness (the head row is a function of the
  group key, so every union fiber is a singleton —
  `nullary_count_fiber_singleton`); the modeling answer is one Count
  per disjunct, host-merged.

## The `provably_distinct` reading (recorded; theorem 2's model)

`plan/fj/provably_distinct.rs::provably_distinct`: every
participating occurrence's bound fields — variable-bound (the `vars`
chain) or equality-pinned to one constant (the `Eq`-filter arm, which
admits words, bytes, intervals, params, and pending interns and
EXCLUDES sets: "set-bound fields pin nothing") — cover the projection
of one of the relation's declared keys (the closing
`keys().iter().any` screen). Negated occurrences bind nothing and
grounding-eliminated occurrences contribute no facts, so only
participating occurrences are quantified (the `participates` filter;
here the positive atom list `Rule.atoms` IS the participating set).
`Term.pins` mirrors the pinned-field screen exactly: `var`, `param`,
and `lit` pin one value under a fixed `(σ, ρ)`; `paramSet` matches any
element and pins nothing; `measure` never appears in a binding
(`Rule.WellTyped`). One asymmetry recorded: the Rust `Eq`-pin arm
admits `Word | Byte | Interval | Param | PendingIntern` and drops
`Const::Words` — the multi-word `bytes<N>` literal, a genuine
single-value pin — to the catch-all arm, so it never counts
toward key coverage; strictly conservative (fewer witness mints, the
seen-set retained), while `Term.pins` marks every `lit` as pinning.
`provably_different` on the disjointness side DOES compare
`Const::Words` payloads — the asymmetry is the mint's, not the
model's.

## The `provably_disjoint` reading (recorded; theorem 6's model)

`plan/fj/provably_disjoint.rs::provably_disjoint_rules`: a
witness `(R, f)` such that EVERY rule pair has, in each rule, a
positive occurrence of `R` whose filters `Eq`-pin `f` to provably
different concrete literals (`plan/fj/provably_disjoint.rs::pinned_fields`;
`plan/fj/provably_disjoint.rs::provably_different` — params, sets,
and mixed constant forms pin nothing, conservatively), AND some key
of `R` value-bound in both occurrences with every key column flowing
to a common head position
(`plan/fj/provably_disjoint.rs::key_flows_to_common_head`;
`plan/fj/provably_disjoint.rs::head_reads` — projected variables and
fold inputs enter the dedup key;
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
declared key PER RULE PAIR (the `keys().iter().any` of
`key_flows_to_common_head`, invoked per pair) — an acceptance
discharged by heterogeneous keys
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
never per rule — `exec/sink.rs`'s module doc). -/
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
(the bound-field collection in
`plan/fj/provably_distinct.rs::provably_distinct`): variable-bound
(`var`), equality-pinned to one constant (`lit`, and `param` —
resolved at bind, one value per execution). `paramSet` matches any
element of the slice and pins nothing ("set-bound fields pin
nothing");
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
`plan/fj/provably_distinct.rs::provably_distinct`. -/
def CoversKey (I : Instance) (a : Atom) : Prop :=
  ∃ K : List FieldId, Functionality (I a.relation) K ∧
    ∀ i, i ∈ K → ∃ t, (i, t) ∈ a.bindings ∧ t.pins

/-- **`BoundFieldsCoverKey`** — the distinct-bindings elision law's
premise: every participating occurrence's bound fields cover a key of
its relation. Positive atoms only — negated occurrences bind nothing
(they only reject: the anti-join `¬∃` of `derives`), exactly the
participation screen (the `participates` filter) of
`plan/fj/provably_distinct.rs::provably_distinct`. This is the
statement `DistinctWitness`
(`plan/fj/provably_distinct.rs::DistinctWitness`) carries as
evidence. -/
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
keys the WHOLE slot array — the `seen` field of
`exec/sink.rs::AggregateSink`), the events are
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
Bridge: `DistinctWitness` (`plan/fj/provably_distinct.rs::DistinctWitness`
— the only mint is `plan/fj/provably_distinct.rs::provably_distinct`);
`AggregateSink::without_seen_set`
(`exec/sink/aggregate/new.rs::without_seen_set`)
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
Bridge: `DisjointWitness` (`plan/fj/provably_disjoint.rs::DisjointWitness`).
The
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
Bridge: `union_spans` (`exec/sink.rs::union_spans`) — per head position,
the slot span the position reads from THIS rule's binding layout; the
extracted words are the head projection, rule-independent by
construction. The aggregate reading of this key law — "aggregates
read the head" — governs HAND-WRITTEN multi-rule programs only: a
DNF-derived rule set re-keys on the shared slot array instead (ruled
2026-07-23, R2), and the aggregate-object form this projection-head
statement deliberately does not carry is `union_regime_agg_heads`
(the union-fold section below). -/
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
`pair_disjoint` (`plan/fj/provably_disjoint.rs::pair_disjoint`): each
rule has
a positive occurrence of `R` pinning `fld` to provably different
literals (`lit` bindings, the model's `Eq`-pins — only concrete
literals are representable as pins, so `provably_different` is plain
`Value` disequality), and every field of the key `K` is variable-bound
in both occurrences with the two variables at a common head position
(the `zip` clause —
`plan/fj/provably_disjoint.rs::key_flows_to_common_head`). -/
def ArmPin (R : RelId) (fld : FieldId) (K : List FieldId)
    (r r' : Rule) : Prop :=
  ∃ a, a ∈ r.atoms ∧ ∃ a', a' ∈ r'.atoms ∧
    a.relation = R ∧ a'.relation = R ∧
    (∃ c c' : Value, (fld, Term.lit c) ∈ a.bindings ∧
      (fld, Term.lit c') ∈ a'.bindings ∧ c ≠ c') ∧
    ∀ i, i ∈ K → ∃ v v' : VarId, (i, Term.var v) ∈ a.bindings ∧
      (i, Term.var v') ∈ a'.bindings ∧ (v, v') ∈ r.finds.zip r'.finds

/-- The check, program-level: one witness discharging every rule pair
— `plan/fj/provably_disjoint.rs::provably_disjoint_rules`
("pairwise over all rules; one witness for every pair"). -/
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
defect to fix (the doc of
`plan/fj/provably_disjoint.rs::provably_disjoint_rules`,
"conservative and sound"). Bridge: `provably_disjoint_rules` is the
only mint of `DisjointWitness`; the semantic key premise is the
schema-declared `Functionality` the check reads (the
`keys().iter().any` of `key_flows_to_common_head`), discharged on
committed instances by PRD 03's `holds`. -/
theorem syntactic_disjointness_sound {C : Classify} {q : Query}
    {I : Instance} {ρ : ParamEnv} {R : RelId} {fld : FieldId}
    {K : List FieldId} (hkey : Functionality (I R) K)
    (hsyn : ProvablyDisjointRules q R fld K) :
    DisjointArms C q I ρ :=
  hsyn.imp fun hpin => armPin_disjoint hkey hpin

/-! ## The aggregate face of elimination — one extension per binding

`elimination_sound` (`Exec/Rewrites.lean`) is the projection-sink
face: answer SETS agree, so containment existence plus deadness
suffice. The aggregate sink folds the distinct FULL-BINDING domain of
each group fiber, and there set identity of answers is not enough —
a dropped occurrence whose dead variable took two values per
surviving binding would multiply the fold domain (Sum double-counts;
the aggregate-safety bullet of `plan/ground.rs`'s module doc is the
engine's argument). The
theorems below spend what the projection face never needed: the
KEY-NESS of `Y` (condition 1's full-key demand — `join_covers_full_key`
joins on a declared key of the target, entering here as the
`Functionality` hypothesis per the module narrowing, discharged on
committed instances by `holds`) and its FULL coverage by the join
pairs. Containment supplies the extension's existence
(`elim_extension_exists`), the key its uniqueness
(`elim_extension_unique`); `elimination_agg_fold_domain` packages
both as a bijective projection of the fold domain, and
`elimination_agg_sound` composes the answer identity — fiber for
fiber, the same key tuple over the same distinct slot-tuple domain,
which is exactly the domain `agg_over_distinct_bindings` hands every
fold (any one listing of it, dedup'd, feeds both sides). -/

/-- The dropped rule derives wherever the original does — dropping a
positive atom only removes constraints (elimination's easy direction,
at the BINDING level; `elimination_sound` states it for answer
sets). -/
theorem elim_derives_drop {C : Classify} {I : Instance} {ρ : ParamEnv}
    {r r' : Rule} {a b : Atom} {X Y : List FieldId} {φ ψ : Selection}
    (hs : ElimStep r r' a b X Y φ ψ) {σ : Assignment}
    (h : derives C r I ρ σ) : derives C r' I ρ σ := by
  obtain ⟨pre, post, hr, hr'⟩ := hs.atoms_split
  obtain ⟨hp, hn, hc⟩ := h
  refine ⟨?_, ?_, ?_⟩
  · intro x hx
    refine hp x ?_
    rw [hr'] at hx
    rw [hr]
    rcases List.mem_append.mp hx with h' | h'
    · exact List.mem_append.mpr (Or.inl h')
    · exact List.mem_append.mpr (Or.inr (List.mem_cons_of_mem _ h'))
  · intro x hx
    exact hn x (hs.negated_eq ▸ hx)
  · intro c hc'
    exact hc c (hs.conditions_eq ▸ hc')

/-- **Existence — the containment's payoff at the binding level**
(the forward half of `elimination_sound`, which keeps only the answer
sets; the aggregate face needs the binding correspondence itself):
every assignment deriving the dropped rule extends to one deriving
the original, agreeing on every surviving variable. The extension is
`elimAssign` — live variables keep their values, the dead ones read
the containment witness's fields. -/
theorem elim_extension_exists {C : Classify} {I : Instance}
    {ρ : ParamEnv} {r r' : Rule} {a b : Atom} {X Y : List FieldId}
    {φ ψ : Selection} (hs : ElimStep r r' a b X Y φ ψ)
    (hcont : Containment (I a.relation) φ X (I b.relation) ψ Y)
    {σ : Assignment} (hσ : derives C r' I ρ σ) :
    ∃ σ', derives C r I ρ σ' ∧ ∀ v, v ∈ r'.allVars → σ' v = σ v := by
  obtain ⟨pre, post, hr, hr'⟩ := hs.atoms_split
  obtain ⟨hatoms, hneg, hconds⟩ := hσ
  obtain ⟨fa, hfa, hma⟩ := hatoms a hs.source
  have hφ : φ.satisfies fa := by
    intro s hsmem
    obtain ⟨c, hcmem, hca⟩ := hs.carries_phi s hsmem
    exact (hma _ hca) ▸ hcmem
  obtain ⟨g, hg, hψ, hgproj⟩ := hcont fa hfa hφ
  have hlive : ∀ v, v ∈ r'.allVars → σ v = elimAssign σ r' b g v :=
    fun v hv => (elimAssign_live hv).symm
  have hmb : Matches g b (elimAssign σ r' b g) ρ := by
    intro bd hbd
    rcases hs.target_bindings bd hbd with ⟨v, hv⟩ | ⟨c, hc, hcψ⟩
    · rw [hv]
      show elimAssign σ r' b g v = g bd.1
      have hvb : (bd.1, Term.var v) ∈ b.bindings := by
        rw [← hv]
        exact hbd
      rcases hs.join_or_dead bd.1 v hvb with ⟨p, hp, hpi, hpa⟩ | hdead
      · have hvlive : v ∈ r'.allVars := by
          refine mem_allVars.mpr (Or.inr (Or.inl ⟨a, hs.source, ?_⟩))
          exact List.mem_flatMap.mpr
            ⟨(p.1, Term.var v), hpa, by simp [Term.vars]⟩
        have h1 : elimAssign σ r' b g v = σ v := (hlive v hvlive).symm
        have h2 : σ v = fa p.1 := hma _ hpa
        have h3 : fa p.1 = g p.2 := project_eq_zip hgproj.symm p hp
        rw [h1, h2, h3, hpi]
      · exact elimAssign_dead hdead hs.var_functional hvb
    · rw [hc]
      show c = g bd.1
      exact (Selection.satisfies_singleton hψ hcψ).symm
  refine ⟨elimAssign σ r' b g, ⟨?_, ?_, ?_⟩,
    fun v hv => elimAssign_live hv⟩
  · intro x hx
    rw [hr] at hx
    rcases List.mem_append.mp hx with hx' | hx'
    · have hxr' : x ∈ r'.atoms := by
        rw [hr']
        exact List.mem_append.mpr (Or.inl hx')
      obtain ⟨f, hf, hmf⟩ := hatoms x hxr'
      refine ⟨f, hf, (matches_congr fun v hv => ?_).mp hmf⟩
      exact hlive v (mem_allVars.mpr (Or.inr (Or.inl ⟨x, hxr', hv⟩)))
    · rcases List.mem_cons.mp hx' with rfl | hx''
      · exact ⟨g, hg, hmb⟩
      · have hxr' : x ∈ r'.atoms := by
          rw [hr']
          exact List.mem_append.mpr (Or.inr hx'')
        obtain ⟨f, hf, hmf⟩ := hatoms x hxr'
        refine ⟨f, hf, (matches_congr fun v hv => ?_).mp hmf⟩
        exact hlive v (mem_allVars.mpr (Or.inr (Or.inl ⟨x, hxr', hv⟩)))
  · intro n hn
    have hn' : n ∈ r'.negated := hs.negated_eq ▸ hn
    rintro ⟨f, hf, hmf⟩
    refine hneg n hn' ⟨f, hf, (matches_congr fun v hv => ?_).mpr hmf⟩
    exact hlive v
      (mem_allVars.mpr (Or.inr (Or.inr (Or.inl ⟨n, hn', hv⟩))))
  · intro c hc
    have hc' : c ∈ r'.conditions := hs.conditions_eq ▸ hc
    refine (condHolds_congr C ρ σ (elimAssign σ r' b g) c
      fun v hv => ?_).mp (hconds c hc')
    exact hlive v
      (mem_allVars.mpr (Or.inr (Or.inr (Or.inr ⟨c, hc', hv⟩))))

/-- **Uniqueness — the target key's payoff.** Two assignments
deriving the ORIGINAL rule that agree on the surviving variables
agree on EVERY variable of the rule: the join pairs pin the dropped
atom's witness fact's whole key tuple (`hYfull` — condition 1's
full-key demand, the join covers every field of `Y`), the key pins
the witness (`hkey` — the declared target key's semantic judgment,
the module narrowing: keys enter as `Functionality` hypotheses), and
the witness pins every variable the dropped atom binds. This is what
keeps a dead non-key variable from multiplying the fold domain — the
distinct-full-binding key COLLAPSES onto the surviving slots. -/
theorem elim_extension_unique {C : Classify} {I : Instance}
    {ρ : ParamEnv} {r r' : Rule} {a b : Atom} {X Y : List FieldId}
    {φ ψ : Selection} (hs : ElimStep r r' a b X Y φ ψ)
    (hkey : Functionality (I b.relation) Y)
    (hYfull : ∀ j, j ∈ Y → ∃ i, (i, j) ∈ X.zip Y)
    {σ₁ σ₂ : Assignment}
    (h₁ : derives C r I ρ σ₁) (h₂ : derives C r I ρ σ₂)
    (hagree : ∀ v, v ∈ r'.allVars → σ₁ v = σ₂ v) :
    ∀ v, v ∈ r.allVars → σ₁ v = σ₂ v := by
  obtain ⟨pre, post, hr, hr'⟩ := hs.atoms_split
  have hbmem : b ∈ r.atoms := by
    rw [hr]
    exact List.mem_append.mpr (Or.inr (List.mem_cons_self ..))
  obtain ⟨g₁, hg₁, hm₁⟩ := h₁.1 b hbmem
  obtain ⟨g₂, hg₂, hm₂⟩ := h₂.1 b hbmem
  -- the join pins the two witnesses' whole key tuples together
  have hproj : g₁.project Y = g₂.project Y := by
    refine (Fact.project_eq_iff g₁ g₂ Y).mpr fun j hj => ?_
    obtain ⟨i, hij⟩ := hYfull j hj
    obtain ⟨v, hva, hvb⟩ := hs.join_covers (i, j) hij
    have hvlive : v ∈ r'.allVars :=
      mem_allVars.mpr (Or.inr (Or.inl ⟨a, hs.source,
        List.mem_flatMap.mpr
          ⟨(i, Term.var v), hva, by simp [Term.vars]⟩⟩))
    have e₁ : σ₁ v = g₁ j := hm₁ _ hvb
    have e₂ : σ₂ v = g₂ j := hm₂ _ hvb
    rw [← e₁, ← e₂, hagree v hvlive]
  have hgg : g₁ = g₂ := hkey g₁ g₂ hg₁ hg₂ hproj
  intro v hv
  rcases mem_allVars.mp hv with hf | ⟨x, hx, hvx⟩ | ⟨x, hx, hvx⟩ |
    ⟨c, hc, hvc⟩
  · exact hagree v (mem_allVars.mpr (Or.inl (hs.finds_eq ▸ hf)))
  · -- a positive atom: a survivor, or the dropped atom itself
    rw [hr] at hx
    rcases List.mem_append.mp hx with hx' | hx'
    · refine hagree v (mem_allVars.mpr (Or.inr (Or.inl ⟨x, ?_, hvx⟩)))
      rw [hr']
      exact List.mem_append.mpr (Or.inl hx')
    · rcases List.mem_cons.mp hx' with heq | hx''
      · -- the dropped atom: the one witness pins the value
        rw [heq] at hvx
        obtain ⟨bd, hbd, hvbd⟩ := List.mem_flatMap.mp hvx
        rcases hs.target_bindings bd hbd with ⟨v', hv'⟩ | ⟨c', hc', -⟩
        · rw [hv'] at hvbd
          have hvv : v = v' := by simpa [Term.vars] using hvbd
          subst hvv
          have hvb : (bd.1, Term.var v) ∈ b.bindings := by
            rw [← hv']
            exact hbd
          have e₁ : σ₁ v = g₁ bd.1 := hm₁ _ hvb
          have e₂ : σ₂ v = g₂ bd.1 := hm₂ _ hvb
          rw [e₁, e₂, hgg]
        · rw [hc'] at hvbd
          simp [Term.vars] at hvbd
      · refine hagree v (mem_allVars.mpr (Or.inr (Or.inl ⟨x, ?_, hvx⟩)))
        rw [hr']
        exact List.mem_append.mpr (Or.inr hx'')
  · exact hagree v (mem_allVars.mpr (Or.inr (Or.inr (Or.inl
      ⟨x, hs.negated_eq ▸ hx, hvx⟩))))
  · exact hagree v (mem_allVars.mpr (Or.inr (Or.inr (Or.inr
      ⟨c, hs.conditions_eq ▸ hc, hvc⟩))))

/-! ### The fold domain, projected -/

/-- The variable a key position reads — `KeyTerm`'s one variable,
either face. -/
def KeyTerm.varOf : KeyTerm → VarId
  | .var v => v
  | .measure v => v

/-- The evaluated group key reads only its key variables. -/
theorem keyTuple_congr {keys : List KeyTerm} {σ σ' : Assignment}
    (h : ∀ k, k ∈ keys → σ k.varOf = σ' k.varOf) :
    keyTuple keys σ = keyTuple keys σ' := by
  unfold keyTuple
  refine List.map_congr_left fun k hk => ?_
  cases k with
  | var v => exact congrArg some (h _ hk)
  | measure v => exact congrArg Value.measure? (h _ hk)

/-- The distinct slot-tuple domain of one group fiber: the fiber's
bindings read through a slot list — the value-level carrier of the
binding seen-set key (the module narrowing: the model keys whole
values; `SlotWidth` word layout is mechanism). This is the set
`agg_over_distinct_bindings`' dedup'd listings enumerate. -/
def GroupSlots (C : Classify) (r : Rule) (I : Instance) (ρ : ParamEnv)
    (keys : List KeyTerm) (gk : List (Option Value))
    (slots : List VarId) : Set (List Value) :=
  fun t => ∃ σ, σ ∈ Group C r I ρ keys gk ∧ t = slots.map σ

/-- Dropping a prefix of a mapped append reads the suffix — the
slot-tuple projection, as a list identity. -/
theorem map_drop_append {α β : Type} (f : α → β) :
    ∀ (l₁ l₂ : List α), ((l₁ ++ l₂).map f).drop l₁.length = l₂.map f
  | [], _ => rfl
  | x :: xs, l₂ => by
    simp only [List.cons_append, List.map_cons, List.length_cons,
      List.drop_succ_cons]
    exact map_drop_append f xs l₂

/-- **The aggregate face of elimination — the fold domain projects
bijectively.** Lay the original rule's slot array as the dropped
atom's slots (`slotsE`, rule variables) followed by the surviving
slots (`slots'`, exactly the surviving rule's variables); then, fiber
for fiber: (1) the surviving-slot projection of the original rule's
distinct binding domain IS the dropped rule's domain — into because
dropping only removes constraints (`elim_derives_drop`), onto because
the containment extends every surviving binding
(`elim_extension_exists`); and (2) the projection merges NOTHING —
two full bindings agreeing on the surviving slots are one slot tuple
(`elim_extension_unique`: the target key pins the witness, the
witness pins the dead variables). Exactly one extension per binding,
so every fold of the distinct binding set reads the same domain
through either rule. Bridge: `plan/ground.rs::join_covers_full_key`
(condition 1 joins on a declared key of the target — the `hkey`/
`hYfull` premises' acceptance form); the elimination differential
under aggregate sinks is the empirical arm. -/
theorem elimination_agg_fold_domain {C : Classify} {I : Instance}
    {ρ : ParamEnv} {r r' : Rule} {a b : Atom} {X Y : List FieldId}
    {φ ψ : Selection} (hs : ElimStep r r' a b X Y φ ψ)
    (hcont : Containment (I a.relation) φ X (I b.relation) ψ Y)
    (hkey : Functionality (I b.relation) Y)
    (hYfull : ∀ j, j ∈ Y → ∃ i, (i, j) ∈ X.zip Y)
    {keys : List KeyTerm}
    (hkeys : ∀ k, k ∈ keys → KeyTerm.varOf k ∈ r'.allVars)
    {slotsE slots' : List VarId}
    (hE : ∀ v, v ∈ slotsE → v ∈ r.allVars)
    (hcov : ∀ v, v ∈ r'.allVars → v ∈ slots')
    (hmem : ∀ v, v ∈ slots' → v ∈ r'.allVars)
    (gk : List (Option Value)) :
    (∀ t', t' ∈ GroupSlots C r' I ρ keys gk slots' ↔
       ∃ t, t ∈ GroupSlots C r I ρ keys gk (slotsE ++ slots') ∧
         t.drop slotsE.length = t') ∧
    (∀ t₁ t₂, t₁ ∈ GroupSlots C r I ρ keys gk (slotsE ++ slots') →
       t₂ ∈ GroupSlots C r I ρ keys gk (slotsE ++ slots') →
       t₁.drop slotsE.length = t₂.drop slotsE.length → t₁ = t₂) := by
  constructor
  · intro t'
    constructor
    · rintro ⟨σ, ⟨hd, hk⟩, rfl⟩
      obtain ⟨σ', hd', hag⟩ := elim_extension_exists hs hcont hd
      refine ⟨(slotsE ++ slots').map σ', ⟨σ', ⟨hd', ?_⟩, rfl⟩, ?_⟩
      · rw [keyTuple_congr fun k hk' => hag _ (hkeys k hk')]
        exact hk
      · rw [map_drop_append]
        exact List.map_congr_left fun v hv => hag v (hmem v hv)
    · rintro ⟨t, ⟨σ, ⟨hd, hk⟩, rfl⟩, rfl⟩
      exact ⟨σ, ⟨elim_derives_drop hs hd, hk⟩,
        map_drop_append σ slotsE slots'⟩
  · rintro t₁ t₂ ⟨σ₁, ⟨hd₁, hk₁⟩, rfl⟩ ⟨σ₂, ⟨hd₂, hk₂⟩, rfl⟩ hdrop
    rw [map_drop_append, map_drop_append] at hdrop
    have hag : ∀ v, v ∈ r'.allVars → σ₁ v = σ₂ v :=
      fun v hv => map_eq_agree hdrop v (hcov v hv)
    have hall := elim_extension_unique hs hkey hYfull hd₁ hd₂ hag
    refine List.map_congr_left fun v hv => ?_
    rcases List.mem_append.mp hv with hvE | hv'
    · exact hall v (hE v hvE)
    · exact hag v (hmem v hv')

/-- **The keyed count transport — where the target key bites for the
engine's own fold shape.** The engine's aggregate sink keys the
ORIGINAL rule's full slot array (the `seen` field of
`exec/sink.rs::AggregateSink`); fiber for
fiber, that full-binding distinct domain and the dropped rule's
surviving-slot domain carry the SAME counts — every floor and every
ceiling transports both ways through `elimination_agg_fold_domain`'s
bijective projection (injectivity — the key premise's half — keeps
the pushed list duplicate-free; surjectivity — the containment's
half — lifts every surviving tuple). So a fold that observes its
domain through its size (the distinct-binding count) is
answer-identical across the elimination at the FULL slot array;
`elimination_agg_sound` below is the companion identity at the
surviving-slot reading. The recorded scope: an arbitrary abstract
fold cannot be STATED over both domains at once (their tuples have
different widths), so the aggregate face is this pair of theorems —
the count transport spending the key, the fiber identity spending
the containment — composed in prose nowhere: each claim is exactly
one of the two statements. -/
theorem elimination_agg_domain_counts {C : Classify} {I : Instance}
    {ρ : ParamEnv} {r r' : Rule} {a b : Atom} {X Y : List FieldId}
    {φ ψ : Selection} (hs : ElimStep r r' a b X Y φ ψ)
    (hcont : Containment (I a.relation) φ X (I b.relation) ψ Y)
    (hkey : Functionality (I b.relation) Y)
    (hYfull : ∀ j, j ∈ Y → ∃ i, (i, j) ∈ X.zip Y)
    {keys : List KeyTerm}
    (hkeys : ∀ k, k ∈ keys → KeyTerm.varOf k ∈ r'.allVars)
    {slotsE slots' : List VarId}
    (hE : ∀ v, v ∈ slotsE → v ∈ r.allVars)
    (hcov : ∀ v, v ∈ r'.allVars → v ∈ slots')
    (hmem : ∀ v, v ∈ slots' → v ∈ r'.allVars)
    (gk : List (Option Value)) (n : Nat) :
    ((GroupSlots C r I ρ keys gk (slotsE ++ slots')).AtLeast n ↔
       (GroupSlots C r' I ρ keys gk slots').AtLeast n) ∧
    ((GroupSlots C r I ρ keys gk (slotsE ++ slots')).AtMost n ↔
       (GroupSlots C r' I ρ keys gk slots').AtMost n) := by
  obtain ⟨honto, hinj⟩ :=
    elimination_agg_fold_domain hs hcont hkey hYfull hkeys hE hcov
      hmem gk
  have hpush : ∀ t,
      t ∈ GroupSlots C r I ρ keys gk (slotsE ++ slots') →
      t.drop slotsE.length ∈ GroupSlots C r' I ρ keys gk slots' :=
    fun t ht => (honto _).mpr ⟨t, ht, rfl⟩
  -- pushing a duplicate-free member list keeps it duplicate-free —
  -- the injectivity half, spent
  have hnodup_map : ∀ l : List (List Value),
      (∀ u, u ∈ l →
        u ∈ GroupSlots C r I ρ keys gk (slotsE ++ slots')) →
      l.Nodup → (l.map (List.drop slotsE.length)).Nodup := by
    intro l
    induction l with
    | nil => exact fun _ _ => List.Pairwise.nil
    | cons t l ih =>
      intro hsub hnd
      obtain ⟨hne, hnd'⟩ := List.pairwise_cons.mp hnd
      refine List.pairwise_cons.mpr
        ⟨?_, ih (fun u hu => hsub u (List.mem_cons_of_mem t hu)) hnd'⟩
      intro u hu heq
      obtain ⟨v, hv, rfl⟩ := List.mem_map.mp hu
      exact hne v hv (hinj t v (hsub t List.mem_cons_self)
        (hsub v (List.mem_cons_of_mem t hv)) heq)
  -- lifting a duplicate-free member list of the dropped domain —
  -- the surjectivity half, spent
  have hlift : ∀ l' : List (List Value), l'.Nodup →
      (∀ u, u ∈ l' → u ∈ GroupSlots C r' I ρ keys gk slots') →
      ∃ l : List (List Value), l.Nodup ∧
        (∀ u, u ∈ l →
          u ∈ GroupSlots C r I ρ keys gk (slotsE ++ slots')) ∧
        l.map (List.drop slotsE.length) = l' := by
    intro l'
    induction l' with
    | nil =>
      intro _ _
      refine ⟨[], List.Pairwise.nil, ?_, rfl⟩
      intro u hu
      cases hu
    | cons u l' ih =>
      intro hnd hsub
      obtain ⟨hne, hnd'⟩ := List.pairwise_cons.mp hnd
      obtain ⟨l, hlnd, hlsub, hlmap⟩ :=
        ih hnd' (fun v hv => hsub v (List.mem_cons_of_mem u hv))
      obtain ⟨t, htmem, htdrop⟩ :=
        (honto u).mp (hsub u List.mem_cons_self)
      refine ⟨t :: l, List.pairwise_cons.mpr ⟨?_, hlnd⟩, ?_, ?_⟩
      · intro v hv heq
        have hu' : u ∈ l' := by
          rw [← htdrop, heq, ← hlmap]
          exact List.mem_map.mpr ⟨v, hv, rfl⟩
        exact hne u hu' rfl
      · intro v hv
        rcases List.mem_cons.mp hv with rfl | hv'
        · exact htmem
        · exact hlsub v hv'
      · show (t :: l).map (List.drop slotsE.length) = u :: l'
        simp only [List.map_cons, hlmap, htdrop]
  constructor
  · constructor
    · rintro ⟨l, hnd, hsub, hlen⟩
      refine ⟨l.map (List.drop slotsE.length),
        hnodup_map l hsub hnd, ?_, ?_⟩
      · intro u hu
        obtain ⟨v, hv, rfl⟩ := List.mem_map.mp hu
        exact hpush v (hsub v hv)
      · rw [List.length_map]
        exact hlen
    · rintro ⟨l', hnd, hsub, hlen⟩
      obtain ⟨l, hlnd, hlsub, hlmap⟩ := hlift l' hnd hsub
      have hlen' : l'.length = l.length := by
        rw [← hlmap, List.length_map]
      exact ⟨l, hlnd, hlsub, by omega⟩
  · constructor
    · intro h l' hnd hsub
      obtain ⟨l, hlnd, hlsub, hlmap⟩ := hlift l' hnd hsub
      have hle := h l hlnd hlsub
      have hlen' : l'.length = l.length := by
        rw [← hlmap, List.length_map]
      omega
    · intro h l hnd hsub
      have hle := h (l.map (List.drop slotsE.length))
        (hnodup_map l hsub hnd) ?_
      · rw [List.length_map] at hle
        exact hle
      · intro u hu
        obtain ⟨v, hv, rfl⟩ := List.mem_map.mp hu
        exact hpush v (hsub v hv)

/-- Aggregate answers with each fiber read through its distinct
slot-tuple domain — `aggAnswers` with the group's carrier made the
value-level seen-set key the sinks actually fold
(the slot-array `seen` key of `exec/sink.rs::AggregateSink`). -/
def aggAnswersOn (C : Classify) (r : Rule) (I : Instance)
    (ρ : ParamEnv) (keys : List KeyTerm) (slots : List VarId)
    (fold : List (Option Value) → Set (List Value) → AnswerTuple) :
    Set AnswerTuple :=
  fun t => ∃ σ, derives C r I ρ σ ∧
    t = fold (keyTuple keys σ)
      (GroupSlots C r I ρ keys (keyTuple keys σ) slots)

/-- `aggAnswersOn` IS the normative `aggAnswers`
(`Query/Aggregates.lean`) with each group fiber handed to the fold
through the slot projection — the two denotations differ only in
which face of the fiber the abstract fold receives, definitionally.
No new answer notion was minted here; this is the recorded link. -/
theorem aggAnswersOn_eq_aggAnswers (C : Classify) (r : Rule)
    (I : Instance) (ρ : ParamEnv) (keys : List KeyTerm)
    (slots : List VarId)
    (fold : List (Option Value) → Set (List Value) → AnswerTuple) :
    aggAnswersOn C r I ρ keys slots fold =
      aggAnswers C r I ρ keys
        (fun gk grp => fold gk
          (fun t => ∃ σ, σ ∈ grp ∧ t = slots.map σ)) := rfl

/-- **`elimination_agg_sound` — removal is result-identical under the
aggregate sink, at the surviving-slot reading.** With BOTH fold
domains read at the surviving slots, the original and dropped rules
emit identical aggregate rows: every inhabited fiber of one is an
inhabited fiber of the other with the SAME key tuple over the SAME
distinct slot-tuple domain, and `agg_over_distinct_bindings` is the
composition point (any one listing of that shared domain, dedup'd,
is the fold input of both sides — no fold observes the dropped
atom). This statement spends the CONTAINMENT only — no key premise
appears, honestly: the key's work is the OTHER half of the aggregate
face, `elimination_agg_domain_counts` (the full-slot-array domain
the engine's sink actually keys has the same counts as this
surviving-slot domain), and the module doc records why the two
halves do not fuse into one abstract-fold statement. -/
theorem elimination_agg_sound {C : Classify} {I : Instance}
    {ρ : ParamEnv} {r r' : Rule} {a b : Atom} {X Y : List FieldId}
    {φ ψ : Selection} (hs : ElimStep r r' a b X Y φ ψ)
    (hcont : Containment (I a.relation) φ X (I b.relation) ψ Y)
    {keys : List KeyTerm}
    (hkeys : ∀ k, k ∈ keys → KeyTerm.varOf k ∈ r'.allVars)
    {slots' : List VarId} (hmem : ∀ v, v ∈ slots' → v ∈ r'.allVars)
    (fold : List (Option Value) → Set (List Value) → AnswerTuple) :
    ∀ t, t ∈ aggAnswersOn C r I ρ keys slots' fold ↔
      t ∈ aggAnswersOn C r' I ρ keys slots' fold := by
  have hfiber : ∀ gk : List (Option Value),
      GroupSlots C r I ρ keys gk slots' =
        GroupSlots C r' I ρ keys gk slots' := by
    intro gk
    funext u
    refine propext ?_
    constructor
    · rintro ⟨σ, ⟨hd, hk⟩, rfl⟩
      exact ⟨σ, ⟨elim_derives_drop hs hd, hk⟩, rfl⟩
    · rintro ⟨σ, ⟨hd, hk⟩, rfl⟩
      obtain ⟨σ', hd', hag⟩ := elim_extension_exists hs hcont hd
      refine ⟨σ', ⟨hd', ?_⟩, ?_⟩
      · rw [keyTuple_congr fun k hk' => hag _ (hkeys k hk')]
        exact hk
      · exact List.map_congr_left fun v hv => (hag v (hmem v hv)).symm
  intro t
  constructor
  · rintro ⟨σ, hd, rfl⟩
    refine ⟨σ, elim_derives_drop hs hd, ?_⟩
    rw [hfiber]
  · rintro ⟨σ, hd, rfl⟩
    obtain ⟨σ', hd', hag⟩ := elim_extension_exists hs hcont hd
    refine ⟨σ', hd', ?_⟩
    have hkeq : keyTuple keys σ' = keyTuple keys σ :=
      keyTuple_congr fun k hk' => hag _ (hkeys k hk')
    rw [hkeq, hfiber]

/-! ## The DNF re-key law — disjunction is fold-transparent (R2)

Surface `or` is fold-transparent (ruled 2026-07-23, R2): a
DNF-DERIVED rule set re-keys the union dedup on the SHARED SLOT
ARRAY — the slot list over the variables every disjunct binds — never
the head projection. The head-projection key exists because a `VarId`
is rule-scoped (the `union_spans` bullet, module doc); DNF
distribution mints no variable and touches no atom (`Rule.lower`
clones finds, atoms, and negated atoms — only the condition trees
split), so the disjuncts of ONE written rule share one variable
vocabulary and one binding layout, and the objection dissolves.
Disjunction then widens MEMBERSHIP without changing the FOLD DOMAIN:
`lower_preserves_derivations` is the widening (a binding derives the
written rule iff it derives some disjunct), the `Set` carrier is the
collapse (a binding derived by two disjuncts is one shared-slot row —
multiplicity, and with it any cross-disjunct double count, is
unrepresentable), and `dnf_rekey_transparent` — THE law, proved — is
the composition: the re-keyed union denotation of a lowering equals
the written rule's own aggregate denotation (`aggAnswersOn`), fiber
for fiber, key for key, uniformly in the fold. Hand-written
multi-rule programs keep the head-projection law
(`union_regime_head_projection`; `aggAnswersUnion` below carries its
aggregate object), with R1 policing its degenerate corner (module
doc).

The R2 agreement, discharged (2026-07-23 audit): `dnf_rekey_stream`
is the stream-level spec both implementations meet — the engine's
union sink re-keys DNF-minted rule sets on the shared slot array, the
conformance glue re-keys on the serializer's derivation mark
(`Conformance.lean`'s `dnfBindings`), and the OR+aggregate case class
enters the differential generator and the conformance corpus so the
law is gated, not merely stated. -/

/-- The shared-slot dedup key (ruled 2026-07-23, R2): the slot array
read over the disjuncts' ONE shared variable vocabulary — the union
key of the DNF regime, where hand-written rule sets key
`union_spans`' head projection. -/
def sharedSlotRow (slots : List VarId) (σ : Assignment) : List Value :=
  slots.map σ

/-- The fold domain the re-key induces: the disjuncts' binding sets
union-widened and read as distinct shared-slot rows. A `Set` —
multiplicity is unrepresentable, so a cross-disjunct re-derivation
cannot double-count by construction. -/
def dnfFoldDomain (C : Classify) (rules : List Rule) (I : Instance)
    (ρ : ParamEnv) (slots : List VarId) : Set (List Value) :=
  fun t => ∃ r, r ∈ rules ∧ ∃ σ, derives C r I ρ σ ∧
    t = sharedSlotRow slots σ

/-- One re-keyed group: `GroupSlots`, union-widened — the fiber of
the re-keyed domain over an evaluated group-key tuple. -/
def dnfGroupSlots (C : Classify) (rules : List Rule) (I : Instance)
    (ρ : ParamEnv) (keys : List KeyTerm) (gk : List (Option Value))
    (slots : List VarId) : Set (List Value) :=
  fun t => ∃ r, r ∈ rules ∧ ∃ σ, σ ∈ Group C r I ρ keys gk ∧
    t = sharedSlotRow slots σ

/-- **The normative union-regime aggregate denotation for a
DNF-derived rule set (ruled 2026-07-23, R2)**: `aggAnswersOn` with
the fiber union-widened over the disjuncts — one row per inhabited
re-keyed fiber. The recorded R2 proof obligation (section doc) is
that `evalUnion` and the engine's union sink agree with THIS on
DNF-derived rule sets. -/
def aggAnswersDNF (C : Classify) (rules : List Rule) (I : Instance)
    (ρ : ParamEnv) (keys : List KeyTerm) (slots : List VarId)
    (fold : List (Option Value) → Set (List Value) → AnswerTuple) :
    Set AnswerTuple :=
  fun t => ∃ r, r ∈ rules ∧ ∃ σ, derives C r I ρ σ ∧
    t = fold (keyTuple keys σ)
      (dnfGroupSlots C rules I ρ keys (keyTuple keys σ) slots)

/-- The re-key never splits a fiber: a group key reads shared slots
only, so slot-equal bindings carry equal evaluated key tuples — the
key is a function of the domain element, which is what makes fibering
the re-keyed domain well-defined. -/
theorem dnf_key_of_slot_row {keys : List KeyTerm} {slots : List VarId}
    (hcover : ∀ k, k ∈ keys → k.varOf ∈ slots) {σ σ' : Assignment}
    (hrow : sharedSlotRow slots σ = sharedSlotRow slots σ') :
    keyTuple keys σ = keyTuple keys σ' :=
  keyTuple_congr fun k hk => map_eq_agree hrow _ (hcover k hk)

/-- DNF lowering preserves DERIVATIONS, not just answers: a binding
derives the written rule iff it derives some disjunct —
`dnf_preserves_denotation` caught before the head projection eats the
binding; the membership-widening half of fold-transparency. -/
theorem lower_preserves_derivations {C : Classify} {r : Rule}
    {I : Instance} {ρ : ParamEnv} {σ : Assignment} :
    derives C r I ρ σ ↔ ∃ r', r' ∈ r.lower ∧ derives C r' I ρ σ := by
  constructor
  · rintro ⟨hatoms, hneg, hconds⟩
    obtain ⟨d, hd, hdis⟩ :=
      (Condition.lowerAll_holds C ρ σ r.conditions).mp
        ((Condition.allHold_iff r.conditions).mpr hconds)
    exact ⟨_, List.mem_map.mpr ⟨d, hd, rfl⟩,
      ⟨hatoms, hneg, holds_map_leaf.mpr hdis⟩⟩
  · rintro ⟨r', hr', hd'⟩
    obtain ⟨d, hd, rfl⟩ := List.mem_map.mp hr'
    obtain ⟨hatoms, hneg, hconds⟩ := hd'
    exact ⟨hatoms, hneg, (Condition.allHold_iff r.conditions).mp
      ((Condition.lowerAll_holds C ρ σ r.conditions).mpr
        ⟨d, hd, holds_map_leaf.mp hconds⟩)⟩

/-- The re-keyed fiber of a lowering IS the written rule's fiber:
disjunction widened membership per disjunct
(`lower_preserves_derivations`) and the shared-slot key collapsed it
back — same slot rows, same group. -/
theorem dnf_fibers_eq (C : Classify) (r : Rule) (I : Instance)
    (ρ : ParamEnv) (keys : List KeyTerm) (gk : List (Option Value))
    (slots : List VarId) :
    dnfGroupSlots C r.lower I ρ keys gk slots =
      GroupSlots C r I ρ keys gk slots := by
  funext t
  refine propext ?_
  constructor
  · rintro ⟨r', hr', σ, ⟨hd, hk⟩, rfl⟩
    exact ⟨σ, ⟨lower_preserves_derivations.mpr ⟨r', hr', hd⟩, hk⟩, rfl⟩
  · rintro ⟨σ, ⟨hd, hk⟩, rfl⟩
    obtain ⟨r', hr', hd'⟩ := lower_preserves_derivations.mp hd
    exact ⟨r', hr', σ, ⟨hd', hk⟩, rfl⟩

/-- **THE re-key law (ruled 2026-07-23, R2).** Surface `or` is
fold-transparent: the re-keyed union denotation of a rule's DNF
lowering equals the written rule's own aggregate denotation — every
fiber, every key tuple, every fold. Disjunction widened membership
and the shared-slot key collapsed it back; the fold domain never
moved. This is the aggregate-object law `union_regime_head_projection`
deliberately does not carry (its statement quantifies projection
heads), and the denotation the engine's re-keyed union sink and the
conformance glue's `evalUnion` are measured against (the recorded R2
proof obligation, section doc). -/
theorem dnf_rekey_transparent (C : Classify) (r : Rule) (I : Instance)
    (ρ : ParamEnv) (keys : List KeyTerm) (slots : List VarId)
    (fold : List (Option Value) → Set (List Value) → AnswerTuple) :
    aggAnswersDNF C r.lower I ρ keys slots fold =
      aggAnswersOn C r I ρ keys slots fold := by
  funext t
  refine propext ?_
  constructor
  · rintro ⟨r', hr', σ, hd, rfl⟩
    refine ⟨σ, lower_preserves_derivations.mpr ⟨r', hr', hd⟩, ?_⟩
    rw [dnf_fibers_eq]
  · rintro ⟨σ, hd, rfl⟩
    obtain ⟨r', hr', hd'⟩ := lower_preserves_derivations.mp hd
    exact ⟨r', hr', σ, hd', by rw [dnf_fibers_eq]⟩

/-- **The R2 stream agreement** — the executable face's spec:
seen-filtering the shared-slot rows of a complete enumeration of a
DNF-derived rule set's derivations computes exactly `dnfFoldDomain`,
duplicate-free. The re-keyed dedup (the engine's shared-slot union
seen-set; the conformance glue's re-keyed arm, `Conformance.lean`'s
`dnfBindings`) therefore reads exactly the fiber carrier
`dnf_rekey_transparent` equates to the written rule's own fold
domain — the two implementations meet the R2 denotation through this
statement. -/
theorem dnf_rekey_stream {C : Classify} {rules : List Rule}
    {I : Instance} {ρ : ParamEnv} {ε : Type} (events : List ε)
    (rule : ε → Rule) (bind : ε → Assignment) (slots : List VarId)
    (hvalid : ∀ e, e ∈ events →
      rule e ∈ rules ∧ derives C (rule e) I ρ (bind e))
    (hcomplete : ∀ r, r ∈ rules → ∀ σ, derives C r I ρ σ →
      sharedSlotRow slots σ ∈
        events.map fun e => sharedSlotRow slots (bind e)) :
    (∀ t, t ∈ seenFold (events.map fun e => sharedSlotRow slots (bind e))
        ↔ t ∈ dnfFoldDomain C rules I ρ slots) ∧
      (seenFold (events.map fun e =>
        sharedSlotRow slots (bind e))).Nodup := by
  refine ⟨fun t => ?_, seenFold_nodup _⟩
  rw [mem_seenFold]
  constructor
  · intro ht
    obtain ⟨e, he, rfl⟩ := List.mem_map.mp ht
    exact ⟨rule e, (hvalid e he).1, bind e, (hvalid e he).2, rfl⟩
  · rintro ⟨r, hr, σ, hd, rfl⟩
    exact hcomplete r hr σ hd

/-! ## The union aggregate fold — the head-projected domain, normative
(2026-07-23 audit, finding 027 — promoted mandatory by R2)

The hand-written multi-rule aggregate head folds the union of the
rules' head-projected binding sets. `union_regime_head_projection`
deliberately cannot state that (its finds are projection variables —
the trimmed scope, its docstring); the vocabulary and the law live
here. `HeadSlot` is the head shape at the key law's level (the
aggregate faces of `union_spans`): a key position projects and keys
(`KeyTerm` — var or measure), a fold input enters the union key with
its VALUE (distinct inputs are distinct fold contributions), a measure
fold input with its measure, and the nullary `Count` contributes no
words (`exec/sink/aggregate/new.rs::union_span` maps `over_slot: None`
to absence; the constant `none` here — omitting a constant column
never changes key equality). `unionFoldDomain` is the normative fold
domain, `aggAnswersUnion` the aggregate denotation (one row per
inhabited fiber, fibered by the shared key mask), and
`union_regime_agg_heads` the coverage law: seen-filtering the head-row
stream of a complete enumeration computes exactly the union fold
domain, duplicate-free — the aggregate-head analogue of theorem 5,
realized by the same spanning seen-set. DNF-derived rule sets never
take this fold (`dnf_rekey_transparent` above — ruled 2026-07-23, R2);
`FoldFreeNullaryCount`/`CountAcrossRulesAccepted` is R1's screen on
the degenerate corner, with `foldfree_head_constant` the proved
uninformativeness that justifies the refusal. -/

/-- One aggregate-head position, as the union key law reads it. -/
inductive HeadSlot where
  /-- A projected group-key position: a plain variable or the
  measure. -/
  | key (k : KeyTerm)
  /-- A fold input (`CountDistinct`/`Sum`/`Min`/`Max`/`Pack` over a
  variable): its value enters the union key. -/
  | fold (v : VarId)
  /-- A measure fold input: its evaluated measure enters the key. -/
  | foldMeasure (v : VarId)
  /-- The nullary `Count`: no words — a keyless head position. -/
  | count
deriving DecidableEq

/-- Whether a head position keys the group. -/
def HeadSlot.isKeyB : HeadSlot → Bool
  | .key _ => true
  | _ => false

/-- The key positions of a head, in head order. -/
def keysOf (head : List HeadSlot) : List KeyTerm :=
  head.filterMap fun s =>
    match s with
    | .key k => some k
    | _ => none

/-- The head row of one binding — what the union seen-set keys: key
positions project (`KeyTerm.value?` — a measure position's `none` is
the ray, corpus-excluded), fold positions carry their input, the
nullary `Count` its constant. -/
def headRow (head : List HeadSlot) (σ : Assignment) :
    List (Option Value) :=
  head.map fun s =>
    match s with
    | .key k => k.value? σ
    | .fold v => some (σ v)
    | .foldMeasure v => (σ v).measure?
    | .count => none

/-- The key selection of a head row: the entries at the head's key
positions. Structural recursion (not zip-and-filter) so the mask
congruence below is an induction, not a plumbing exercise. -/
def headKey : List HeadSlot → List (Option Value) →
    List (Option Value)
  | s :: t, x :: r => if s.isKeyB then x :: headKey t r else headKey t r
  | _, _ => []

/-- The key selection reads only the KEY MASK: two heads with one
`isKeyB` image select identically — validation aligns the rules'
head shapes positionally, so any rule's head serves as the shared
mask. -/
theorem headKey_mask_congr :
    ∀ {h h' : List HeadSlot},
      h.map HeadSlot.isKeyB = h'.map HeadSlot.isKeyB →
      ∀ row, headKey h row = headKey h' row
  | [], [], _, _ => rfl
  | [], _ :: _, heq, _ => by simp at heq
  | _ :: _, [], heq, _ => by simp at heq
  | s :: t, s' :: t', heq, row => by
    injection heq with h1 h2
    cases row with
    | nil => rfl
    | cons x xs =>
      show (if s.isKeyB then x :: headKey t xs else headKey t xs) =
        (if s'.isKeyB then x :: headKey t' xs else headKey t' xs)
      rw [h1, headKey_mask_congr h2 xs]

/-- The group key is a PROJECTION of the head row — fibering the
union domain by the key is well-defined (the head-projection twin of
`dnf_key_of_slot_row`). -/
theorem headKey_headRow :
    ∀ (head : List HeadSlot) (σ : Assignment),
      headKey head (headRow head σ) = keyTuple (keysOf head) σ
  | [], _ => rfl
  | s :: t, σ => by
    cases s with
    | key k =>
      show k.value? σ :: headKey t (headRow t σ) =
        k.value? σ :: keyTuple (keysOf t) σ
      exact congrArg _ (headKey_headRow t σ)
    | fold v => exact headKey_headRow t σ
    | foldMeasure v => exact headKey_headRow t σ
    | count => exact headKey_headRow t σ

/-- **The normative union fold domain** (2026-07-23 audit, 027): the
union of the rules' head-projected binding sets — a `Set`, so a later
rule's re-derivation of one head row is the same element
(multiplicity across written rules is unrepresentable, exactly the
head-projection law's collapse). -/
def unionFoldDomain (C : Classify) (rs : List (Rule × List HeadSlot))
    (I : Instance) (ρ : ParamEnv) : Set (List (Option Value)) :=
  fun t => ∃ p, p ∈ rs ∧ ∃ σ, derives C p.1 I ρ σ ∧ t = headRow p.2 σ

/-- One union group: the domain's fiber over a group-key tuple, read
through the shared key mask. -/
def unionGroup (C : Classify) (rs : List (Rule × List HeadSlot))
    (I : Instance) (ρ : ParamEnv) (mask : List HeadSlot)
    (gk : List (Option Value)) : Set (List (Option Value)) :=
  fun t => t ∈ unionFoldDomain C rs I ρ ∧ headKey mask t = gk

/-- **The normative multi-rule aggregate denotation**: one row per
inhabited union fiber — the fold reads the group-key tuple and the
fiber of distinct head rows (the per-position folds of the rules-IR
union regime, abstracted as `fold`). The witness `(p, σ)` is the
load-bearing shape, as in `aggAnswers`: a group exists only as the
fiber of an actual derivation. -/
def aggAnswersUnion (C : Classify) (rs : List (Rule × List HeadSlot))
    (I : Instance) (ρ : ParamEnv) (mask : List HeadSlot)
    (fold : List (Option Value) → Set (List (Option Value)) →
      AnswerTuple) : Set AnswerTuple :=
  fun t => ∃ p, p ∈ rs ∧ ∃ σ, derives C p.1 I ρ σ ∧
    t = fold (headKey mask (headRow p.2 σ))
      (unionGroup C rs I ρ mask (headKey mask (headRow p.2 σ)))

/-- **The aggregate-head coverage of the union key law** (the
companion `union_regime_head_projection`'s projection-head statement
deliberately does not carry): seen-filtering the HEAD-ROW stream of a
complete enumeration of a multi-rule program's derivations computes
exactly the normative union fold domain, duplicate-free — the
spanning seen-set keyed on `union_spans`' head projection hands every
per-position fold the distinct head-projected union, which is
`aggAnswersUnion`'s fiber carrier. Bridge: `union_spans`
(`exec/sink.rs::union_spans`); the aggregate sink's spanning seen-set
(`exec/sink.rs::AggregateSink`). -/
theorem union_regime_agg_heads {C : Classify}
    {rs : List (Rule × List HeadSlot)} {I : Instance} {ρ : ParamEnv}
    {ε : Type} (events : List ε) (arm : ε → Rule × List HeadSlot)
    (bind : ε → Assignment)
    (hvalid : ∀ e, e ∈ events →
      arm e ∈ rs ∧ derives C (arm e).1 I ρ (bind e))
    (hcomplete : ∀ p, p ∈ rs → ∀ σ, derives C p.1 I ρ σ →
      headRow p.2 σ ∈ events.map fun e => headRow (arm e).2 (bind e)) :
    (∀ t, t ∈ seenFold (events.map fun e => headRow (arm e).2 (bind e))
        ↔ t ∈ unionFoldDomain C rs I ρ) ∧
      (seenFold (events.map fun e =>
        headRow (arm e).2 (bind e))).Nodup := by
  refine ⟨fun t => ?_, seenFold_nodup _⟩
  rw [mem_seenFold]
  constructor
  · intro ht
    obtain ⟨e, he, rfl⟩ := List.mem_map.mp ht
    exact ⟨arm e, (hvalid e he).1, bind e, (hvalid e he).2, rfl⟩
  · rintro ⟨p, hp, σ, hd, rfl⟩
    exact hcomplete p hp σ hd

/-! ### The R1 screen — the fold-free nullary Count is refused -/

/-- The R1 head shape: a fold-free head carrying the nullary `Count`
— every position keys or counts, with a `Count` present. -/
def FoldFreeNullaryCount (head : List HeadSlot) : Prop :=
  .count ∈ head ∧ ∀ s, s ∈ head → s.isKeyB = true ∨ s = .count

/-- **The R1 acceptance screen (ruled 2026-07-23, R1)** — a
validation-model refusal, stated never proved: a 2+-rule program
whose head is fold-free with a nullary `Count` is a typed validation
error beside `ArgAcrossRules` (the modeling answer: one `Count` per
disjunct, host-merged). DNF-derived rule sets are untouched — the R2
re-key keeps their fold domain the written rule's full binding set,
so their `Count` counts. `foldfree_head_constant` below is the
uninformativeness that justifies the refusal. -/
def CountAcrossRulesAccepted (rs : List (Rule × List HeadSlot)) :
    Prop :=
  2 ≤ rs.length → ∀ p, p ∈ rs → ¬ FoldFreeNullaryCount p.2

/-- **R1's justification, proved**: on fold-free heads the head row
is a FUNCTION of the group key — shape-aligned heads with equal key
tuples project equal rows — so every union fiber is a singleton and
the nullary `Count` under the head-projection law is definitionally
the constant 1 per group: an uninformative query, made
unrepresentable by the screen. -/
theorem foldfree_head_constant :
    ∀ {h h' : List HeadSlot},
      h.map HeadSlot.isKeyB = h'.map HeadSlot.isKeyB →
      (∀ s, s ∈ h → s.isKeyB = true ∨ s = .count) →
      (∀ s, s ∈ h' → s.isKeyB = true ∨ s = .count) →
      ∀ {σ σ' : Assignment},
        keyTuple (keysOf h) σ = keyTuple (keysOf h') σ' →
        headRow h σ = headRow h' σ'
  | [], [], _, _, _, _, _, _ => rfl
  | [], _ :: _, heq, _, _, _, _, _ => by simp at heq
  | _ :: _, [], heq, _, _, _, _, _ => by simp at heq
  | s :: t, s' :: t', heq, hff, hff', σ, σ', hkey => by
    injection heq with h1 h2
    have hs := hff s (List.mem_cons_self ..)
    have hs' := hff' s' (List.mem_cons_self ..)
    have hfft := fun x hx => hff x (List.mem_cons_of_mem _ hx)
    have hfft' := fun x hx => hff' x (List.mem_cons_of_mem _ hx)
    cases hkb : s.isKeyB with
    | true =>
      obtain ⟨k, rfl⟩ : ∃ k, s = .key k := by
        cases s with
        | key k => exact ⟨k, rfl⟩
        | fold v => exact nomatch hkb
        | foldMeasure v => exact nomatch hkb
        | count => exact nomatch hkb
      obtain ⟨k', rfl⟩ : ∃ k', s' = .key k' := by
        rw [hkb] at h1
        cases s' with
        | key k' => exact ⟨k', rfl⟩
        | fold v => exact nomatch h1
        | foldMeasure v => exact nomatch h1
        | count => exact nomatch h1
      have hkey' : k.value? σ :: keyTuple (keysOf t) σ =
          k'.value? σ' :: keyTuple (keysOf t') σ' := hkey
      injection hkey' with hk1 hk2
      show k.value? σ :: headRow t σ = k'.value? σ' :: headRow t' σ'
      rw [hk1, foldfree_head_constant h2 hfft hfft' hk2]
    | false =>
      have hcnt : s = .count := by
        rcases hs with hk | hc
        · rw [hkb] at hk
          exact nomatch hk
        · exact hc
      have hcnt' : s' = .count := by
        rcases hs' with hk | hc
        · rw [← h1, hkb] at hk
          exact nomatch hk
        · exact hc
      subst hcnt hcnt'
      have hkey' : keyTuple (keysOf t) σ = keyTuple (keysOf t') σ' :=
        hkey
      show (none : Option Value) :: headRow t σ = none :: headRow t' σ'
      rw [foldfree_head_constant h2 hfft hfft' hkey']

/-- The fiber singleton, at the domain level: under one shared
fold-free shape, two rows of one union fiber are EQUAL — the group's
distinct head-row set is a singleton, so the union-regime `Count`
answers 1 per group uniformly (the R1 refusal's countermodel-free
form). -/
theorem nullary_count_fiber_singleton {C : Classify}
    {rs : List (Rule × List HeadSlot)} {I : Instance} {ρ : ParamEnv}
    {mask : List HeadSlot}
    (hshape : ∀ p, p ∈ rs →
      p.2.map HeadSlot.isKeyB = mask.map HeadSlot.isKeyB)
    (hff : ∀ p, p ∈ rs → ∀ s, s ∈ p.2 → s.isKeyB = true ∨ s = .count)
    {gk t t' : List (Option Value)}
    (ht : t ∈ unionGroup C rs I ρ mask gk)
    (ht' : t' ∈ unionGroup C rs I ρ mask gk) : t = t' := by
  obtain ⟨⟨p, hp, σ, hd, rfl⟩, hk⟩ := ht
  obtain ⟨⟨p', hp', σ', hd', rfl⟩, hk'⟩ := ht'
  have e1 : keyTuple (keysOf p.2) σ = gk := by
    rw [← headKey_headRow, headKey_mask_congr (hshape p hp)]
    exact hk
  have e2 : keyTuple (keysOf p'.2) σ' = gk := by
    rw [← headKey_headRow, headKey_mask_congr (hshape p' hp')]
    exact hk'
  exact foldfree_head_constant
    ((hshape p hp).trans (hshape p' hp').symm)
    (hff p hp) (hff p' hp') (e1.trans e2.symm)

/-! ## The membership fold companion — the mint is fold-invisible
(2026-07-23 audit, finding 087)

A membership term SELECTS, it never binds: the aggregate fold domain
of a lowered membership rule is the SURFACE rule's distinct binding
set — the minted interval variable is no fold slot. With the fiber
read at slots the written rule mentions (the surface width — every
mint sits above it), the lowered rule's aggregate denotation IS the
surface reading's: `membership_lowering_preserves_fold`, the
fold-level `membership_lowering_preserves` companion, composed from
the binding-level lowering (`Query/Membership.lean`:
`lowerFuel_derives_forward` extends at the mints alone, agreeing on
every written variable; `lowerFuel_derives_backward` reuses the
assignment unchanged) — the `Set` carrier collapses the mint's
multiplicity before any fold sees it (two facts covering one point
are one surface-slot row). The conformance glue folds the surface
width (`Conformance.lean` — the serializer's `"width"` key) and the
corpus fence (`Exclusion::AggregateMembership`) lifts, so the third
oracle adjudicates membership-under-additive-fold. -/

/-- The surface rule's group fiber, read at a slot list — the surface
twin of `GroupSlots`. -/
def SurfaceGroupSlots (Γ : Typing) (C : Classify) (r : Rule)
    (I : Instance) (ρ : ParamEnv) (keys : List KeyTerm)
    (gk : List (Option Value)) (slots : List VarId) :
    Set (List Value) :=
  fun t => ∃ σ, surfaceDerives Γ C r I ρ σ ∧ keyTuple keys σ = gk ∧
    t = slots.map σ

/-- The surface aggregate denotation at the slot reading — the
surface twin of `aggAnswersOn`. -/
def surfaceAggAnswersOn (Γ : Typing) (C : Classify) (r : Rule)
    (I : Instance) (ρ : ParamEnv) (keys : List KeyTerm)
    (slots : List VarId)
    (fold : List (Option Value) → Set (List Value) → AnswerTuple) :
    Set AnswerTuple :=
  fun t => ∃ σ, surfaceDerives Γ C r I ρ σ ∧
    t = fold (keyTuple keys σ)
      (SurfaceGroupSlots Γ C r I ρ keys (keyTuple keys σ) slots)

/-- The lowered rule's fibers ARE the surface fibers at surface
slots: forward extends at the mints alone, backward reuses the
assignment unchanged — the mint is projected away by the slot reading
before any fold observes it. -/
theorem membership_fibers_eq (Γ : Typing) (C : Classify) (r : Rule)
    (I : Instance) (ρ : ParamEnv)
    (hneg : ∀ a, a ∈ r.negated → Atom.membershipFree Γ a)
    {keys : List KeyTerm} (hkeys : ∀ k, k ∈ keys → k.varOf ∈ r.allVars)
    {slots : List VarId} (hslots : ∀ v, v ∈ slots → v ∈ r.allVars)
    (gk : List (Option Value)) :
    SurfaceGroupSlots Γ C r I ρ keys gk slots =
      GroupSlots C (r.lowerMembership Γ).2 I ρ keys gk slots := by
  have hposF := lowerFuel_posFree _ Γ r (Nat.le_refl _)
  have hnegF := lowerFuel_negFree (memCount Γ.membership r.atoms) Γ r hneg
  funext t
  refine propext ?_
  constructor
  · rintro ⟨σ, hd, hk, rfl⟩
    obtain ⟨σ', hd', hag⟩ := lowerFuel_derives_forward C I ρ _ Γ r hd
    refine ⟨σ', ⟨(surfaceDerives_iff_derives_of_free hposF hnegF).mp hd', ?_⟩, ?_⟩
    · rw [keyTuple_congr fun k hk' => hag _ (hkeys k hk')]
      exact hk
    · exact List.map_congr_left fun v hv => (hag v (hslots v hv)).symm
  · rintro ⟨σ, ⟨hd, hk⟩, rfl⟩
    exact ⟨σ, lowerFuel_derives_backward C I ρ _ Γ r
      ((surfaceDerives_iff_derives_of_free hposF hnegF).mpr hd), hk, rfl⟩

/-- **The fold-level `membership_lowering_preserves` companion
(2026-07-23 audit, finding 087)**: aggregates over the lowered rule
with the mint projected away — keys and slots reading only variables
the WRITTEN rule mentions — equal aggregates over the surface
reading, fiber for fiber, key for key, uniformly in the fold. The
mint is fold-invisible on the same ground it is answer-invisible. -/
theorem membership_lowering_preserves_fold (Γ : Typing) (C : Classify)
    (r : Rule) (I : Instance) (ρ : ParamEnv)
    (hneg : ∀ a, a ∈ r.negated → Atom.membershipFree Γ a)
    {keys : List KeyTerm} (hkeys : ∀ k, k ∈ keys → k.varOf ∈ r.allVars)
    {slots : List VarId} (hslots : ∀ v, v ∈ slots → v ∈ r.allVars)
    (fold : List (Option Value) → Set (List Value) → AnswerTuple) :
    surfaceAggAnswersOn Γ C r I ρ keys slots fold =
      aggAnswersOn C (r.lowerMembership Γ).2 I ρ keys slots fold := by
  have hposF := lowerFuel_posFree _ Γ r (Nat.le_refl _)
  have hnegF := lowerFuel_negFree (memCount Γ.membership r.atoms) Γ r hneg
  funext t
  refine propext ?_
  constructor
  · rintro ⟨σ, hd, rfl⟩
    obtain ⟨σ', hd', hag⟩ := lowerFuel_derives_forward C I ρ _ Γ r hd
    have hkeq : keyTuple keys σ' = keyTuple keys σ :=
      keyTuple_congr fun k hk' => hag _ (hkeys k hk')
    refine ⟨σ', (surfaceDerives_iff_derives_of_free hposF hnegF).mp hd', ?_⟩
    rw [hkeq, membership_fibers_eq Γ C r I ρ hneg hkeys hslots]
  · rintro ⟨σ, hd, rfl⟩
    refine ⟨σ, lowerFuel_derives_backward C I ρ _ Γ r
      ((surfaceDerives_iff_derives_of_free hposF hnegF).mpr hd), ?_⟩
    rw [membership_fibers_eq Γ C r I ρ hneg hkeys hslots]

end Bumbledb.Query
