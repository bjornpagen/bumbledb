import Bumbledb.Values

/-!
# Schema — theories, instances, ground facts (Level 0, PRD 03)

The substrate the dependency judgments quantify over: headers
(relation signatures), facts, projections over field sets,
disjunctive selections, statements (the four declared forms and
nothing else), theories, instances, and the ground axioms of closed
relations as instance-independent sealed constants.

## The acceptance boundary is part of the model

* **Selections are membership-to-literal-set BY REPRESENTATION
  (E3, disjunctive selections).** `Selection` is a finite list of
  (field, literal-set) bindings read conjunctively, each binding a
  disjunction over its spelled set — no richer predicate is writable
  at this level. The ENGINE's accepted σ fragment is this same
  representation: `Side.selection` carries (field, literal-set)
  bindings (`LiteralSet` in `crates/bumbledb-theory/src/schema.rs`; the
  macro's `parse_side` parses `f == L` and `f == {A, B}`). A
  singleton set is exactly the equality binding
  (`Selection.singleton_satisfies_iff`) and stays the engine's
  zero-cost `One` arm, so the wider representation re-reads every
  previously accepted σ unchanged; the sets are first-class rather
  than per-literal sugar because counts over a union do not decompose
  (`Countermodels.disjunctive_window_not_literal_conjunction`). The
  decidability-firewall tripwire's recorded edge
  (`docs/architecture/30-dependencies.md` § the decidability
  firewall) is the same decision docs-side, executed 2026-07-14.
* **Statements are the three judgment forms**: functionality and
  containment exactly as `StatementDescriptor`, plus the cardinality
  window extension form with its denotation in `Cardinality.lean`.
  No constraint kinds, no modes, no triggers.
* **Ground axioms are constants of the THEORY.** A closed relation's
  extension is sealed at declaration and `Instance`-independent by
  type (`Theory.den` never consults the instance for it) —
  `den_closed_constant` is the one-line theorem.

## Narrowings recorded (law 5: narrow and record)

* **Discharged (2026-07-14): the literal-SET σ form.** The engine's
  accepted σ fragment is the (field, literal-set) disjunctive form —
  `Side.selection` is `Box<[(FieldId, LiteralSet)]>`
  (`crates/bumbledb-theory/src/schema.rs`), the sealed `CompiledCheck`
  set arms judge membership among the sealed encodings, and the
  canonical form is sorted and duplicate-free (validation rejects
  the degenerate spellings). The singleton `One` arm is
  byte-identical to the pre-set engine —
  `Selection.singleton_satisfies_iff` is that agreement, and the
  `Bridge.lean` row for the set form cites it.
* **A fact is a total field-indexed value assignment**
  (`Fact := FieldId → Value`). Arity and positional typing are the
  header's concern, and no PRD 03 theorem needs a typing premise —
  the judgments quantify over whatever fact sets an instance carries.
  Fields beyond a relation's arity are junk the judgments never read
  except through fact identity; instances of interest carry
  arity-respecting facts.
* **Finiteness is a named token, never ambient.** `Set.Finite` is the
  listability token; none of PRD 03's theorems demand it (they are
  subset and injectivity algebra, finite or not), so it is defined and
  deliberately unspent here. `den_closed_finite` shows the sealed
  extension carries it by construction.
* **`Point` is the tagged sum of the two interval element domains.**
  Every pointwise judgment quantifies over `Set Point`, which makes
  the judgments total without a typing premise: a scalar value denotes
  no points, and positional typing keeps accepted statements within
  one tag. **Q1 — element-domain typing at interval positions**: the
  tag IS the element domain and carries no width, so a fixed-width
  side against a general (or other-width) side of one element is
  already well-judged by every theorem in this file and its
  consumers — the judgments quantified over `Point` all along, so
  admitting mixed widths is the acceptance gate catching up to the
  denotation, not a semantic change (`Value.points_one_tag_u64`;
  Rust: `schema/validate.rs::positional_types_match`). Scalar
  positions keep exact structural equality.
* **`Header.intervalSplit` reads the field SET, never the written
  order** — the FieldSet doctrine: `resolve_target_key` counts
  interval positions as a set and `key_permutation` bridges statement
  order to key order, so the engine's pointwise reading is
  order-canonical and the split matches it. Exactly one interval-typed
  field splits to the pointwise shape at ANY written position; zero or
  several split to `none` and receive the scalar reading downstream.
  The several-interval shape is gate-refused
  (`FunctionalityMultipleIntervals`; the pointwise gate in
  `resolve_target_key`); `holds` is consumed on accepted theories
  only.
* Acceptance's remaining shape checks (duplicate-free projections,
  arity and positional type match between sides, determinant width,
  the FD-side interval-finality demand — the neighbor probe's
  mechanism, not semantics — and the σ shape refusals
  `SelectedFieldProjected` and `DuplicateSelectionField`,
  `schema/validate.rs::validate_side_selection` and
  `::validate_side_shape`, which narrow the accepted σ
  fragment below `Selection`'s representable shapes, sound direction)
  are validator mechanism this level does not restate — only the
  exact-field-set target-key rule is modeled (`Dependencies.lean`),
  because it is the piece the theorems spend.
-/

namespace Bumbledb

/-! ## Set algebra — the subset order over PRD 02's carrier -/

instance : HasSubset (Set α) := ⟨fun s t => ∀ a, a ∈ s → a ∈ t⟩

/-- Subset is pointwise implication — the definitional unfold. -/
theorem Set.subset_def {s t : Set α} : s ⊆ t ↔ ∀ a, a ∈ s → a ∈ t :=
  Iff.rfl

/-- The named finiteness token: the set is listable. Finiteness is
NEVER ambient — a theorem needing it demands this token by name
(none of PRD 03's do; recorded in the module doc). -/
def Set.Finite (s : Set α) : Prop :=
  ∃ l : List α, ∀ a, a ∈ s ↔ a ∈ l

/-! ## Identities -/

/-- A field position within a relation (`crate::schema::FieldId`). -/
structure FieldId where
  id : Nat
deriving DecidableEq

/-- A relation identity within a theory
(`crate::schema::RelationId`). -/
structure RelId where
  id : Nat
deriving DecidableEq

/-! ## Facts and projections -/

/-- A fact: the field-indexed value assignment one stored row denotes
(`crate::value` rows under `crate::encoding::FactLayout`). Identity is
extensional — the canonical-bytes law `value_eq_iff_encode_eq` carries
it fieldwise to the encoding. -/
def Fact : Type := FieldId → Value

/-- πX — the projected value tuple over the ordered field list `X`.
The tuple keeps statement order (execution derives the target-key
permutation from it) while identity is the field SET (`sameFields`;
key resolution is by set) — mirroring `Projection { ordered, fields }`
in `crates/bumbledb/src/schema/validate.rs`. -/
def Fact.project (f : Fact) (X : List FieldId) : List Value :=
  X.map f

/-- Projected tuples agree exactly when the facts agree on every
projected field — the bridge between tuple identity and fieldwise
agreement every determinant argument walks through. -/
theorem Fact.project_eq_iff (f g : Fact) (X : List FieldId) :
    f.project X = g.project X ↔ ∀ i, i ∈ X → f i = g i := by
  induction X with
  | nil =>
    exact ⟨fun _ i hi => (nomatch hi), fun _ => rfl⟩
  | cons a X ih =>
    constructor
    · intro h i hi
      have hcons : f a :: f.project X = g a :: g.project X := h
      injection hcons with h1 h2
      cases hi with
      | head => exact h1
      | tail _ hmem => exact (ih.mp h2) i hmem
    · intro h
      have h1 : f a = g a := h a (.head X)
      have h2 : f.project X = g.project X :=
        ih.mpr fun i hi => h i (.tail a hi)
      show f a :: f.project X = g a :: g.project X
      rw [h1, h2]

/-- Canonical set identity of a projection — `FieldSet` in
`schema/validate.rs`: key resolution and duplicate rejection compare
THIS, never the order. -/
def sameFields (X Y : List FieldId) : Prop :=
  ∀ i, i ∈ X ↔ i ∈ Y

/-! ## Selections — the accepted σ fragment (disjunctive, E3) -/

/-- σ — the spec's selection fragment: a finite list of
(field, literal-SET) bindings, read conjunctively; each binding is
the DISJUNCTION over its spelled set (the field's value is a MEMBER).
Membership-to-literal-set BY REPRESENTATION: no richer predicate is
writable. The engine's accepted σ is this same fragment
(`LiteralSet` in `crates/bumbledb-theory/src/schema.rs`;
`validate_side_selection` and the sealed `CompiledCheck` arms consume
it), and `Selection.singleton_satisfies_iff` proves the singleton
reading is exactly the equality binding — the engine's zero-cost
`One` arm, unchanged in meaning. The sets are
first-class, not sugar: a window over a disjunctive selection is not
any conjunction of per-literal windows
(`Countermodels.disjunctive_window_not_literal_conjunction`). -/
structure Selection where
  bindings : List (FieldId × List Value)

/-- The empty selection — `R(X)` with no σ. -/
def Selection.empty : Selection := ⟨[]⟩

/-- σ over one fact: every binding's field carries a member of its
literal set. -/
def Selection.satisfies (φ : Selection) (f : Fact) : Prop :=
  ∀ b, b ∈ φ.bindings → f b.1 ∈ b.2

/-- The empty selection accepts every fact. -/
theorem Selection.empty_satisfies (f : Fact) :
    Selection.empty.satisfies f :=
  fun _ hb => by cases hb

/-- **A singleton set is today's equality.** The one-binding
one-literal selection satisfies exactly the facts carrying that
literal — every σ written before the disjunctive extension means what
it meant. -/
theorem Selection.singleton_satisfies_iff (i : FieldId) (v : Value)
    (f : Fact) :
    (Selection.mk [(i, [v])]).satisfies f ↔ f i = v := by
  constructor
  · intro h
    exact List.mem_singleton.mp (h _ (List.mem_singleton.mpr rfl))
  · intro heq b hb
    rcases List.mem_singleton.mp hb with rfl
    exact List.mem_singleton.mpr heq

/-- A satisfied singleton binding reads as the equality it spells —
the directional form consumers spend binding by binding. -/
theorem Selection.satisfies_singleton {φ : Selection} {f : Fact}
    (h : φ.satisfies f) {i : FieldId} {v : Value}
    (hb : (i, [v]) ∈ φ.bindings) : f i = v :=
  List.mem_singleton.mp (h _ hb)

/-- Adding bindings strengthens σ: if every binding of `φ` appears
among `φ'`'s, then `φ'` implies `φ` — the accepted fragment's only
syntactic strengthening move, spent by `selection_monotonicity`.
(Shrinking a binding's literal set is the other strengthening;
`selection_monotonicity`'s semantic hypotheses carry it already.) -/
theorem Selection.satisfies_of_superset {φ φ' : Selection}
    (h : ∀ b, b ∈ φ.bindings → b ∈ φ'.bindings) {f : Fact}
    (hf : φ'.satisfies f) : φ.satisfies f :=
  fun b hb => hf b (h b hb)

/-! ## Points — the pointwise reading of interval positions -/

/-- A point of an interval element domain, tagged by the domain — the
one carrier every pointwise judgment quantifies over. The sum makes
the judgments total without a typing premise; Q1's element-domain
typing at interval positions keeps accepted statements within one
tag (a point carries an element domain, never a width — so a
fixed-width side against a general side of one element meets in one
tag: `Value.points_one_tag_u64`/`_i64`). -/
inductive Point where
  | u64 (x : U64)
  | i64 (x : I64)

/-- The point-family a VALUE denotes: an interval value denotes its
half-open `Interval.points` (tagged); every scalar value denotes no
points. The pointwise judgments (`PointwiseKey`, `Coverage`) read
interval positions through exactly this set — "a fact stands for its
point-family". -/
def Value.points : Value → Set Point
  | { type := .interval .u64, val := iv } => fun p =>
      match p with
      | .u64 x => x ∈ Interval.points iv
      | .i64 _ => False
  | { type := .interval .i64, val := iv } => fun p =>
      match p with
      | .i64 x => x ∈ Interval.points iv
      | .u64 _ => False
  -- A fixed-width value denotes its DERIVED interval's points,
  -- `[s, s + w)` — the width is the type's, and the judgments read
  -- the same `Interval.points` the general type feeds them.
  | { type := .intervalFixed .u64 _, val := v } => fun p =>
      match p with
      | .u64 x => x ∈ Interval.points v.toInterval
      | .i64 _ => False
  | { type := .intervalFixed .i64 _, val := v } => fun p =>
      match p with
      | .i64 x => x ∈ Interval.points v.toInterval
      | .u64 _ => False
  | _ => fun _ => False

/-- An interval-typed value denotes a NONEMPTY point family —
`interval_nonempty` read through the fact-level denotation. This is
how consumers discharge a "some point exists" premise on
arity-respecting instances (`keyprobe_pointwise_key_spent`'s typing
hypothesis): a stored interval column carries interval values, and
every representable interval is inhabited. -/
theorem Value.points_nonempty {v : Value} {e : Elem}
    (h : v.type = .interval e) : ∃ p, p ∈ v.points := by
  obtain ⟨t, val⟩ := v
  cases h
  cases e with
  | u64 =>
    obtain ⟨x, hx⟩ := interval_nonempty val
    exact ⟨.u64 x, hx⟩
  | i64 =>
    obtain ⟨x, hx⟩ := interval_nonempty val
    exact ⟨.i64 x, hx⟩

/-- The fixed-width companion of `Value.points_nonempty`: a stored
`interval<E, w>` column carries fixed-width values, and every one
denotes the nonempty derived `[s, s + w)` — `interval_nonempty` read
through `FixedU64.toInterval`. -/
theorem Value.points_nonempty_fixed {v : Value} {e : Elem} {w : Nat}
    (h : v.type = .intervalFixed e w) : ∃ p, p ∈ v.points := by
  obtain ⟨t, val⟩ := v
  cases h
  cases e with
  | u64 =>
    obtain ⟨x, hx⟩ := interval_nonempty val.toInterval
    exact ⟨.u64 x, hx⟩
  | i64 =>
    obtain ⟨x, hx⟩ := interval_nonempty val.toInterval
    exact ⟨.i64 x, hx⟩

/-- **Q1, the element-domain rule made concrete (u64):** every point
an interval-SHAPED value of element domain `u64` denotes carries the
`u64` tag — GENERAL or FIXED, any width. Mixed-width pairs of one
element domain therefore meet in ONE `Point` tag, so every pointwise
judgment (`PointwiseKey`, `Coverage`, `ExactPartition`, the window
count — all quantifying over `Point`, never over widths) engages
across them with zero changes: element-domain typing at interval
positions is the spec catching up to its OWN denotation, not a new
rule. Scalar positions are untouched (a scalar value denotes no
points; its typing is exact structural equality,
`schema/validate.rs::positional_types_match`). -/
theorem Value.points_one_tag_u64 {v : Value} {p : Point}
    (h : v.type = .interval .u64 ∨ ∃ w, v.type = .intervalFixed .u64 w)
    (hp : p ∈ v.points) : ∃ x, p = .u64 x := by
  obtain ⟨t, val⟩ := v
  rcases h with h | ⟨w, h⟩ <;> cases h <;> cases p with
  | u64 x => exact ⟨x, rfl⟩
  | i64 x => exact nomatch hp

/-- The `i64` twin of `Value.points_one_tag_u64`. -/
theorem Value.points_one_tag_i64 {v : Value} {p : Point}
    (h : v.type = .interval .i64 ∨ ∃ w, v.type = .intervalFixed .i64 w)
    (hp : p ∈ v.points) : ∃ x, p = .i64 x := by
  obtain ⟨t, val⟩ := v
  rcases h with h | ⟨w, h⟩ <;> cases h <;> cases p with
  | u64 x => exact nomatch hp
  | i64 x => exact ⟨x, rfl⟩

/-! ## Headers -/

/-- A header: each relation's positional field types — the signature
acceptance validates statements against. -/
structure Header where
  sig : RelId → List ValueType

/-- Whether field `i` of relation `R` is interval-typed — the general
type AND the fixed-width family: `interval<E, w>` is interval-shaped
to every consumer of this discriminator, which is why `intervalSplit`
and BOTH pointwise judgments engage for fixed-width positions with
ZERO changes to any dependency judgment (they quantify over
`Value.points` and this split alone — the design's beauty; verified
against `Dependencies.lean` and `Admission.lean`, 2026-07-15). -/
def Header.isInterval (h : Header) (R : RelId) (i : FieldId) : Bool :=
  match (h.sig R)[i.id]? with
  | some (.interval _) => true
  | some (.intervalFixed _ _) => true
  | _ => false

/-- The set-canonical interval shape of a determinant or projection —
the FieldSet doctrine (`schema/validate.rs::resolve_target_key` counts
interval positions as a SET; `key_permutation` bridges written order
to key order): a projection whose field set carries EXACTLY ONE
interval-typed field splits to `some` — the scalar rest in written
order, paired with that interval field, wherever it was written.
Every other shape splits to `none`, truthfully: zero interval fields
is the classical reading, and two or more are gate-refused
(`FunctionalityMultipleIntervals`; the pointwise gate in
`resolve_target_key`) and take the scalar-reading default recorded in
the module doc. -/
def Header.intervalSplit (h : Header) (R : RelId)
    (X : List FieldId) : Option (List FieldId × FieldId) :=
  match X.filter (fun i => h.isInterval R i) with
  | [i] => some (X.filter (fun i => !h.isInterval R i), i)
  | _ => none

/-- An all-scalar projection splits to `none` — the classical-judgment
arm of `Statement.judgment`. -/
theorem Header.intervalSplit_scalar (h : Header) (R : RelId)
    (X : List FieldId)
    (hall : ∀ i, i ∈ X → h.isInterval R i = false) :
    h.intervalSplit R X = none := by
  have hfil : X.filter (fun i => h.isInterval R i) = [] :=
    List.filter_eq_nil_iff.mpr fun i hi => by simp [hall i hi]
  unfold Header.intervalSplit
  rw [hfil]

/-- What a pointwise split PINS — the inversion consumers walk back
to the field set: the returned interval field is the sole
interval-typed member of the projection, and the returned scalar
prefix is exactly its interval-free remainder
(`keyprobe_pointwise_key_spent` spends both halves). -/
theorem Header.intervalSplit_some {h : Header} {R : RelId}
    {X S : List FieldId} {i : FieldId}
    (hs : h.intervalSplit R X = some (S, i)) :
    X.filter (fun j => h.isInterval R j) = [i] ∧
      S = X.filter (fun j => !h.isInterval R j) := by
  rcases hfil : X.filter (fun j => h.isInterval R j) with
    _ | ⟨a, _ | ⟨b, l⟩⟩
  · simp only [Header.intervalSplit, hfil] at hs
    exact nomatch hs
  · simp only [Header.intervalSplit, hfil, Option.some.injEq,
      Prod.mk.injEq] at hs
    obtain ⟨hS, hai⟩ := hs
    exact ⟨by rw [hai], hS.symm⟩
  · simp only [Header.intervalSplit, hfil] at hs
    exact nomatch hs

/-! ## Extension syntax — windows

The statement form the dependency-vocabulary extension adds carries
syntax of its own: the count window of `A(X | φ) in n..m per
B(Y | ψ)`. Syntax only — the denotation lives in
`Cardinality.lean`. -/

/-- A cardinality window `n..m`. `hi = none` is the `*` spelling —
the ONLY spelling of "no upper bound", and the DEFAULT posture: the
`0..*` window is provably vacuous and universal (`zero_star_admits`,
`star_subsumes` in `Cardinality.lean`), so a spelled statement is
always a strengthening of the default, never a repair of it. -/
structure Window where
  /-- The inclusive lower count bound. -/
  lo : Nat
  /-- The inclusive upper count bound; `none` is `*`. -/
  hi : Option Nat

/-! ## Statements — the declared forms -/

/-- One side of a containment: the single-atom query `R(X | φ)`.
Dependencies and queries share one representation — a dependency is a
required property of an ordinary query, not a new kind of thing. -/
structure Atom where
  relation : RelId
  projection : List FieldId
  selection : Selection

/-- A declared dependency statement — the two original judgment forms
(`crate::schema::StatementDescriptor`) plus the cardinality-window
extension form, judged in the STATEMENT phase like every other
statement. `==` is not a form: the macro lowers it to two adjacent
containments, each judged independently. Readings live in
`Statement.judgment`. -/
inductive Statement where
  /-- `R(X) -> R`: functionality, key form only (the acceptance
  gate refuses non-key and selected FDs — they are relation splits
  waiting to happen). -/
  | functionality (relation : RelId) (projection : List FieldId)
  /-- `A(X | φ) <= B(Y | ψ)`: containment. -/
  | containment (source target : Atom)
  /-- `A(X | φ) in n..m per B(Y | ψ)`: the cardinality window —
  per selected target fact, the count of selected source facts
  sharing its projected tuple lies in the window
  (`CardinalityWindow`, `Cardinality.lean`). Acceptance gate as for
  `<=`: `Y` must be a key of `B` — an ACCEPTANCE premise, never a
  conjunct of the denotation. -/
  | cardinality (source : Atom) (window : Window) (target : Atom)

/-! ## Theories, instances, ground axioms -/

/-- A closed relation's sealed extension: the ground axioms, a finite
fact list fixed at declaration (`crate::schema::SealedRow`; the ≤256
roster cap is mechanism, not modeled). -/
structure GroundExtension where
  facts : List Fact

/-- A theory: the header, the closed relations' sealed extensions,
and the declared statements (`crate::schema::Schema`, sealed). -/
structure Theory where
  header : Header
  /-- Closed relations: ground axioms as sealed constants of the
  THEORY — `Instance`-independent by type. -/
  closed : RelId → Option GroundExtension
  statements : List Statement

/-- A database state: each relation denotes a set of facts.
Finiteness is the NAMED token `Instance.FiniteAt`, never ambient. -/
def Instance : Type := RelId → Set Fact

/-- The finiteness token at one relation. -/
def Instance.FiniteAt (I : Instance) (R : RelId) : Prop :=
  (I R).Finite

/-- The denotation of relation `R` under theory `T` at instance `I`:
a closed relation reads its sealed ground axioms (instance-independent
— `den_closed_constant`); an ordinary relation reads the instance. -/
def Theory.den (T : Theory) (I : Instance) (R : RelId) : Set Fact :=
  match T.closed R with
  | some ext => fun f => f ∈ ext.facts
  | none => I R

/-- **Ground axioms are instance-independent**: a closed relation
denotes the same fact set at every instance — the sealed extension is
a constant of the theory, which is why a statement between constants
is decided at validate outright (`validate_containment`'s
closed-source scan; `ClosedStatementRefuted`).
Bridge: `crate::schema::validate` seals the extension; no commit ever
touches the relation. -/
theorem den_closed_constant {T : Theory} {R : RelId}
    {ext : GroundExtension} (h : T.closed R = some ext)
    (I J : Instance) : T.den I R = T.den J R := by
  unfold Theory.den
  rw [h]

/-- A closed relation's denotation carries the finiteness token by
construction — the sealed list is the witness. -/
theorem den_closed_finite {T : Theory} {R : RelId}
    {ext : GroundExtension} (h : T.closed R = some ext)
    (I : Instance) : (T.den I R).Finite := by
  refine ⟨ext.facts, fun f => ?_⟩
  unfold Theory.den
  rw [h]
  exact Iff.rfl

end Bumbledb
