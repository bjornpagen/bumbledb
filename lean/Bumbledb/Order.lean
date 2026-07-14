import Bumbledb.Schema

/-!
# Order — the order mark's judgment (extension 2)

The second extension statement form:
`order A(pos) per A(parent) [by … -> K(rank)]` — a relation's ordinal
column is document geometry, not data the author knows. This module
is the judgment alone: against a finished instance, per parent group,
positions are exactly `1..k` (1-based, duplicate-free, contiguous)
and monotone with respect to the `by` ranks. Any generation reading —
an engine or host minting the positions — is mechanism downstream of
this level; a hand-built row set is judged exactly like a generated
one, which is what keeps hand-numbered columns honest.

## Contiguity as downward closure

"Exactly `1..k`" is spelled without naming `k`: positions are
duplicate-free (`OrdinalGroup.unique`), 1-based
(`OrdinalGroup.based`), and DOWNWARD CLOSED (`OrdinalGroup.closed` —
every ordinal from 1 up to any attained position is attained). On a
finite group that is precisely the contiguous `1..k`, and the
spelling stays total over arbitrary fact sets with no finiteness
token — the tree's counting discipline (`Cardinality.lean`).

## The exact tie-break law

The ranked form adds rank monotonicity: a strictly smaller rank sits
strictly earlier. In the judgment the residual order — whatever a
generator would instantiate ties with — IS the position order itself,
and `ranked_tiebreak_lex` states the law exactly: the position order
is the LEXICOGRAPHIC order on (rank, position), i.e. rank decides
first, and equal-rank ties are broken by the residual order and
nothing else. Deterministic rank reads are the form's acceptance
rule: each `by` hop must be key-backed, and
`chain_eval_deterministic` (`Subsumption.lean`) is that premise
spent — acceptance stays a hypothesis, never a conjunct of a
denotation.

## v0 refusals recorded

* **Order-mark sides are single atoms, permanently (E1)** — the same
  refusal `Cardinality.lean` records for window sides: a join inside
  the judge breaks the linear per-statement cost model.
* **`fresh` never appears in a rule head, and no arithmetic appears
  in a rule head** — both already unrepresentable in the modeled
  syntax; recorded here because the rank chain is the one place a
  head-adjacent computation could have crept in, and it deliberately
  reads stored payloads only (`RankHop.eval` probes relations, never
  computes).

## Countermodels

`Countermodels.order_gap` (positions `{1, 3}` — downward closure
fails at 2) and `Countermodels.order_duplicate` (two distinct facts
at position 1 — uniqueness fails): the gap and the duplicate, the two
ways a hand-numbered column lies.

## Narrowings recorded (law 5: narrow and record)

* **`Value.ordinal` is total without a typing premise**: a `u64`
  reads its numeral; every other value reads 0 — junk the judgment
  never accepts, since `OrdinalGroup.based` demands `1 ≤`. The same
  totalization move as `Value.points`.
* **Rank evaluation is relational, not functional, by definition**
  (`RankHop.eval`, `chainEval`): the key premise that makes it a
  function is acceptance's, spent in `Subsumption.lean` — never baked
  into the denotation.
* **Undischarged (spec-ahead): the engine has no order-mark statement
  form yet.** The engine's accepted statement forms today are
  functionality and containment, full stop
  (`crate::schema::StatementDescriptor`,
  `crates/bumbledb/src/schema.rs`);
  `order A(pos) per A(parent) [by … -> K(rank)]` is the 2026-07-14
  vocabulary campaign's admission, and its Rust discharge — macro
  form, the key-backed-hop acceptance rule, the statement-phase
  judgment — is decided and queued. That is why no `Bridge.lean` row
  cites this module: deliberate, not an omission. Nothing here claims
  the engine accepts, judges, or enforces an order mark today.
-/

namespace Bumbledb

/-! ## The ordinal reading of a position column -/

/-- The ordinal a value carries: a `u64` reads its numeral; every
other value reads 0 — junk the judgment never accepts, since
`OrdinalGroup.based` demands `1 ≤`. Total without a typing premise,
exactly as `Value.points` reads interval positions. -/
def Value.ordinal : Value → Nat
  | { type := .u64, val := v } => v.val
  | _ => 0

/-! ## Groups and the ordinal discipline -/

/-- The parent group of one grouping tuple: the facts of `R` whose
grouping projection `G` equals `t`. -/
def GroupOf (R : Set Fact) (G : List FieldId) (t : List Value) :
    Set Fact :=
  fun f => f ∈ R ∧ f.project G = t

/-- Membership in a parent group, unfolded — the definitional
reading. -/
theorem mem_groupOf {R : Set Fact} {G : List FieldId}
    {t : List Value} {f : Fact} :
    f ∈ GroupOf R G t ↔ f ∈ R ∧ f.project G = t :=
  Iff.rfl

/-- One group's ordinal discipline: positions are duplicate-free
(`unique`), 1-based (`based`), and downward closed (`closed`) —
contiguity without naming `k`. On a finite group the positions are
exactly `1..k`; an empty group satisfies all three vacuously. -/
structure OrdinalGroup (s : Set Fact) (pos : FieldId) : Prop where
  /-- No two distinct facts of the group share an ordinal. -/
  unique : ∀ f g, f ∈ s → g ∈ s →
    (f pos).ordinal = (g pos).ordinal → f = g
  /-- Every position is 1-based. -/
  based : ∀ f, f ∈ s → 1 ≤ (f pos).ordinal
  /-- Downward closure: every ordinal from 1 up to any attained
  position is attained — the no-gap law. -/
  closed : ∀ f, f ∈ s → ∀ n, 1 ≤ n → n ≤ (f pos).ordinal →
    ∃ g, g ∈ s ∧ (g pos).ordinal = n

/-- `order A(pos) per A(G)` — the plain order mark's judgment:
every parent group is ordinally disciplined. -/
def OrderMark (R : Set Fact) (pos : FieldId) (G : List FieldId) :
    Prop :=
  ∀ t, OrdinalGroup (GroupOf R G t) pos

/-! ## The plain-form theorems -/

/-- **Positions are unique within a group.** Two facts of one parent
group carrying the same ordinal are the same fact — the position
column is a per-group key (spent as semantic functionality by
`order_group_functionality`, `Subsumption.lean`). -/
theorem order_positions_unique {R : Set Fact} {pos : FieldId}
    {G : List FieldId} (h : OrderMark R pos G) {t : List Value}
    {f g : Fact} (hf : f ∈ GroupOf R G t) (hg : g ∈ GroupOf R G t)
    (hord : (f pos).ordinal = (g pos).ordinal) : f = g :=
  (h t).unique f g hf hg hord

/-- **Contiguity.** Below any attained position, every 1-based
ordinal is attained — no gap can hide under a witness. -/
theorem order_contiguous {R : Set Fact} {pos : FieldId}
    {G : List FieldId} (h : OrderMark R pos G) {t : List Value}
    {f : Fact} (hf : f ∈ GroupOf R G t) {n : Nat} (h1 : 1 ≤ n)
    (hn : n ≤ (f pos).ordinal) :
    ∃ g, g ∈ GroupOf R G t ∧ (g pos).ordinal = n :=
  (h t).closed f hf n h1 hn

/-- **A nonempty group starts at 1.** Any member forces position 1 to
be attained — 1-basedness and downward closure spent together. -/
theorem order_first_position {R : Set Fact} {pos : FieldId}
    {G : List FieldId} (h : OrderMark R pos G) {t : List Value}
    {f : Fact} (hf : f ∈ GroupOf R G t) :
    ∃ g, g ∈ GroupOf R G t ∧ (g pos).ordinal = 1 :=
  (h t).closed f hf 1 (Nat.le_refl 1) ((h t).based f hf)

/-! ## The ranked form -/

/-- One hop's relational reading: some fact of the hop's relation
carries the running value at the key field; the hop reads its payload
field. Relational because acceptance, not denotation, makes it a
FUNCTION: the key premise is spent by `chain_eval_deterministic`
(`Subsumption.lean`). -/
def RankHop.eval (hop : RankHop) (T : Theory) (I : Instance)
    (v w : Value) : Prop :=
  ∃ g, g ∈ T.den I hop.relation ∧ g hop.key = v ∧ g hop.read = w

/-- A chain of hops, evaluated relationally: the empty chain is
identity; a hop feeds its read into the rest. -/
def chainEval (T : Theory) (I : Instance) :
    List RankHop → Value → Value → Prop
  | [], v, w => v = w
  | hop :: rest, v, w => ∃ u, hop.eval T I v u ∧ chainEval T I rest u w

/-- The rank a `by` chain assigns a fact: chase the hops from the
fact's link field; the final payload's ordinal is the rank. -/
def RankChain.rankOf (c : RankChain) (T : Theory) (I : Instance)
    (f : Fact) (r : Nat) : Prop :=
  ∃ w, chainEval T I c.hops (f c.link) w ∧ w.ordinal = r

/-- `order A(pos) per A(G) by …` — the ranked order mark's judgment:
the plain discipline plus rank monotonicity — within a group, a
strictly smaller rank sits strictly earlier. Ties are NOT constrained
beyond uniqueness: the equal-rank order is the residual order, which
is exactly what `ranked_tiebreak_lex` makes precise. -/
structure RankedOrderMark (T : Theory) (I : Instance) (R : Set Fact)
    (pos : FieldId) (G : List FieldId) (c : RankChain) : Prop where
  /-- The plain discipline: every group ordinally disciplined. -/
  mark : OrderMark R pos G
  /-- Rank monotonicity within each group. -/
  mono : ∀ t f g, f ∈ GroupOf R G t → g ∈ GroupOf R G t →
    ∀ rf rg, c.rankOf T I f rf → c.rankOf T I g rg →
      rf < rg → (f pos).ordinal < (g pos).ordinal

/-- **The ranked form subsumes the plain judgment.** Dropping the
chain weakens, never changes, the judgment — the `by`-less form is
the same discipline with no rank clause. -/
theorem ranked_order_shadow {T : Theory} {I : Instance}
    {R : Set Fact} {pos : FieldId} {G : List FieldId} {c : RankChain}
    (h : RankedOrderMark T I R pos G c) : OrderMark R pos G :=
  h.mark

/-- **The exact tie-break law.** Under a ranked order mark, the
position order within a group is EXACTLY the lexicographic order on
(rank, position) — `lexLt`, the same lexicographic reading the
interval encoding uses. Rank decides first; equal-rank ties are
broken deterministically by the residual order, which in the judgment
is the position order itself. Nothing else influences placement. -/
theorem ranked_tiebreak_lex {T : Theory} {I : Instance}
    {R : Set Fact} {pos : FieldId} {G : List FieldId} {c : RankChain}
    (h : RankedOrderMark T I R pos G c) {t : List Value} {f g : Fact}
    (hf : f ∈ GroupOf R G t) (hg : g ∈ GroupOf R G t)
    {rf rg : Nat} (hrf : c.rankOf T I f rf)
    (hrg : c.rankOf T I g rg) :
    (f pos).ordinal < (g pos).ordinal ↔
      lexLt (rf, (f pos).ordinal) (rg, (g pos).ordinal) := by
  constructor
  · intro hlt
    show rf < rg ∨ (rf = rg ∧ (f pos).ordinal < (g pos).ordinal)
    cases Nat.lt_or_ge rf rg with
    | inl hlt' => exact Or.inl hlt'
    | inr hge =>
      cases Nat.lt_or_ge rg rf with
      | inl hgt =>
        exact absurd hlt
          (Nat.lt_asymm (h.mono t g f hg hf rg rf hrg hrf hgt))
      | inr hge' => exact Or.inr ⟨Nat.le_antisymm hge' hge, hlt⟩
  · intro hlex
    have hor : rf < rg ∨
        (rf = rg ∧ (f pos).ordinal < (g pos).ordinal) := hlex
    cases hor with
    | inl hlt' => exact h.mono t f g hf hg rf rg hrf hrg hlt'
    | inr hand => exact hand.2

end Bumbledb
