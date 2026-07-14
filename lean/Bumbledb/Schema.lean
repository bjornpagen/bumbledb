import Bumbledb.Values

/-!
# Schema — theories, instances, ground facts (Level 0, PRD 03)

The substrate the dependency judgments quantify over: headers
(relation signatures), facts, projections over field sets,
equality-only selections, statements (the two declared forms and
nothing else), theories, instances, and the ground axioms of closed
relations as instance-independent sealed constants.

## The acceptance boundary is part of the model

* **Selections are equality-only BY REPRESENTATION.** `Selection` is a
  finite list of (field, literal) bindings read conjunctively — no
  other predicate is writable, mirroring the accepted σ fragment
  (`docs/architecture/30-dependencies.md` § the two judgments;
  `crates/bumbledb/src/schema/validate.rs::validate_side_selection`).
* **Statements are the two judgment forms**, functionality and
  containment, exactly as `StatementDescriptor` — no constraint kinds,
  no modes, no triggers.
* **Ground axioms are constants of the THEORY.** A closed relation's
  extension is sealed at declaration and `Instance`-independent by
  type (`Theory.den` never consults the instance for it) —
  `den_closed_constant` is the one-line theorem.

## Narrowings recorded (law 5: narrow and record)

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
  no points, and positional structural typing keeps accepted
  statements within one tag.
* **`Header.intervalSplit` reads the ACCEPTED determinant shape** — at
  most one interval position, final (`validate_functionality`'s gate).
  Projections outside that shape split to `none` and receive the
  scalar reading downstream; `holds` is consumed on accepted theories
  only.
* Acceptance's remaining shape checks (duplicate-free projections,
  arity and positional type match between sides, determinant width)
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

/-! ## Selections — the accepted σ fragment -/

/-- σ — the accepted selection fragment: a finite list of
(field, literal) equality bindings, read conjunctively. Equality-only
BY REPRESENTATION: no other predicate is writable, so the acceptance
boundary is part of the model (`validate_side_selection`; the sealed
`CompiledCheck` byte compares in `storage/commit/judgment.rs`). -/
structure Selection where
  bindings : List (FieldId × Value)

/-- The empty selection — `R(X)` with no σ. -/
def Selection.empty : Selection := ⟨[]⟩

/-- σ over one fact: every binding's field carries its literal. -/
def Selection.satisfies (φ : Selection) (f : Fact) : Prop :=
  ∀ b, b ∈ φ.bindings → f b.1 = b.2

/-- The empty selection accepts every fact. -/
theorem Selection.empty_satisfies (f : Fact) :
    Selection.empty.satisfies f :=
  fun _ hb => by cases hb

/-- Adding bindings strengthens σ: if every binding of `φ` appears
among `φ'`'s, then `φ'` implies `φ` — the accepted fragment's only
strengthening move, spent by `selection_monotonicity`. -/
theorem Selection.satisfies_of_superset {φ φ' : Selection}
    (h : ∀ b, b ∈ φ.bindings → b ∈ φ'.bindings) {f : Fact}
    (hf : φ'.satisfies f) : φ.satisfies f :=
  fun b hb => hf b (h b hb)

/-! ## Points — the pointwise reading of interval positions -/

/-- A point of an interval element domain, tagged by the domain — the
one carrier every pointwise judgment quantifies over. The sum makes
the judgments total without a typing premise; positional structural
typing keeps accepted statements within one tag. -/
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
  | _ => fun _ => False

/-! ## Headers -/

/-- A header: each relation's positional field types — the signature
acceptance validates statements against. -/
structure Header where
  sig : RelId → List ValueType

/-- Whether field `i` of relation `R` is interval-typed. -/
def Header.isInterval (h : Header) (R : RelId) (i : FieldId) : Bool :=
  match (h.sig R)[i.id]? with
  | some (.interval _) => true
  | _ => false

/-- The accepted interval shape of a determinant or projection: the
scalar prefix and the final interval-typed position, or `none` for an
all-scalar list. Acceptance (`validate_functionality`, the pointwise
gate) forces at most one interval position, placed last; this reads
that accepted shape, and every other shape splits to `none` — the
scalar-reading default recorded in the module doc. -/
def Header.intervalSplit (h : Header) (R : RelId) :
    List FieldId → Option (List FieldId × FieldId)
  | [] => none
  | [i] => if h.isInterval R i then some ([], i) else none
  | i :: j :: rest =>
    (h.intervalSplit R (j :: rest)).map fun (S, k) => (i :: S, k)

/-- An all-scalar projection splits to `none` — the classical-judgment
arm of `Statement.judgment`. -/
theorem Header.intervalSplit_scalar (h : Header) (R : RelId) :
    ∀ X : List FieldId,
      (∀ i, i ∈ X → h.isInterval R i = false) →
      h.intervalSplit R X = none
  | [], _ => rfl
  | [i], hall => by
    simp [Header.intervalSplit, hall i (List.mem_singleton.mpr rfl)]
  | i :: j :: rest, hall => by
    have ih := Header.intervalSplit_scalar h R (j :: rest)
      fun k hk => hall k (List.mem_cons_of_mem i hk)
    simp [Header.intervalSplit, ih]

/-! ## Statements — the two declared forms -/

/-- One side of a containment: the single-atom query `R(X | φ)`.
Dependencies and queries share one representation — a dependency is a
required property of an ordinary query, not a new kind of thing. -/
structure Atom where
  relation : RelId
  projection : List FieldId
  selection : Selection

/-- A declared dependency statement — the two judgment forms, nothing
else (`crate::schema::StatementDescriptor`). `==` is not a form: the
macro lowers it to two adjacent containments, each judged
independently. -/
inductive Statement where
  /-- `R(X) -> R`: functionality, key form only (the acceptance
  gate refuses non-key and selected FDs — they are relation splits
  waiting to happen). -/
  | functionality (relation : RelId) (projection : List FieldId)
  /-- `A(X | φ) <= B(Y | ψ)`: containment. -/
  | containment (source target : Atom)

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
