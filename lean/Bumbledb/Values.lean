/-!
# Values — the value universe (Level 0, PRD 02)

The value universe: the six structural types (Bool, U64, I64, Str,
FixedBytes n, Interval over an orderable element domain), the
nonempty half-open interval with rays and measure, and the
order-embedding encodings. Types are encodings — hard structural
typing; nominal safety lives in host Rust newtypes, never here.

## Deliberate absences (design facts, not gaps)

* **Str carries NO order.** `StrId` is an opaque intern identity with
  decidable equality only. The intern id is a per-database allocation
  accident: two databases intern the same string to different ids, so
  any order on ids would order the interning history, not the values.
  The order-refusal is a typing fact — no `LT`/`LE`/`Ord` instance
  exists for `StrId` (machine-checked in `Countermodels.lean`).
* **The empty interval is unrepresentable.** `Interval` carries the
  invariant `h : start < «end»` as a field, mirroring
  `crate::Interval` (whose constructors return `Option`). A fact never
  denotes nothing; the vacuous-coverage countermodel over a raw bounds
  pair lives in `Countermodels.lean` — its unrepresentability here is
  the point.
* **`Interval` itself carries NO order.** The lexicographic-by-start
  order the encoding has (`encode_interval_order`) is an encoding
  fact, not semantics — mirroring the Rust type's deliberate
  non-`Ord`. The order theorem is stated over the encoded pair, and
  no `LT` instance is installed on `Interval`.
* **The Allen mask is not a field type.** It anchors only the Allen
  comparison's mask position (PRD 05's territory); nothing storable
  carries one, so it has no place in the value universe.

## Narrowings recorded (law 5: narrow and record)

* `U64`/`I64` are modeled as the bounded subtypes
  `{n : Nat // n < 2^64}` and `{x : Int // -(2^63) ≤ x ∧ x < 2^63}`.
  The bound is the type, exactly as in Rust: the interval ceiling
  `end ≤ MAX_END` needs no extra invariant field.
* The word domain is `Nat` — the encodings are order-embedding
  CLAIMS, not byte layouts. `encodeI64` is the bias form
  `x ↦ (x + 2^63).toNat`, which is what the byte-level sign-bit flip
  computes on two's-complement words; the engine's exhaustive order
  suite samples the byte fact itself.
* `FixedBytes n` is a length-indexed word list; the zero-pad to the
  word boundary is encoding, not data, and is invisible at this
  level (constant for a fixed `n`, so injectivity is unaffected).
* `encode_interval_order` is stated concretely over the I64 domain
  (the sign-flipped, interesting half); `encode_interval_order_u64`
  is its U64 companion. Generalizing over an abstract order-embedding
  class would cost more proof plumbing than the two real domains
  justify.
* `Set` is defined in-tree (`α → Prop` with membership): core Lean
  v4.32.0 has no `Set`, and mathlib is refused.
-/

namespace Bumbledb

/-! ## Sets — the in-tree denotation carrier -/

/-- A set as a predicate — the denotation carrier for `points`. -/
def Set (α : Type u) : Type u := α → Prop

instance : Membership α (Set α) := ⟨fun s a => s a⟩

/-- A set is nonempty when it has a member. -/
def Set.Nonempty (s : Set α) : Prop := ∃ x, x ∈ s

/-! ## The element domains

The two orderable scalars an interval ranges over, as bounded
subtypes — the bound is the type, so `end ≤ MAX_END` holds by
construction, exactly as in Rust. -/

/-- The `u64` domain: naturals below `2^64`. -/
abbrev U64 : Type := { n : Nat // n < 2 ^ 64 }

instance : LT U64 := ⟨fun a b => a.val < b.val⟩
instance : LE U64 := ⟨fun a b => a.val ≤ b.val⟩
instance : DecidableLT U64 := fun a b => inferInstanceAs (Decidable (a.val < b.val))
instance : DecidableLE U64 := fun a b => inferInstanceAs (Decidable (a.val ≤ b.val))

/-- The domain ceiling `u64::MAX` (`crate::Interval::<u64>::MAX_END`). -/
def U64.maxEnd : U64 := ⟨2 ^ 64 - 1, by omega⟩

/-- The `i64` domain: integers in `[-2^63, 2^63)`. -/
abbrev I64 : Type := { x : Int // -(2 ^ 63) ≤ x ∧ x < 2 ^ 63 }

instance : LT I64 := ⟨fun a b => a.val < b.val⟩
instance : LE I64 := ⟨fun a b => a.val ≤ b.val⟩
instance : DecidableLT I64 := fun a b => inferInstanceAs (Decidable (a.val < b.val))
instance : DecidableLE I64 := fun a b => inferInstanceAs (Decidable (a.val ≤ b.val))

/-- The domain ceiling `i64::MAX` (`crate::Interval::<i64>::MAX_END`). -/
def I64.maxEnd : I64 := ⟨2 ^ 63 - 1, by omega⟩

/-- What an interval element domain provides: the ceiling `MAX_END`,
the measure payload `gap` (`«end» − start` as a `Nat`), and
reflexivity of `≤` (all `interval_nonempty` needs). The point domain
is `[MIN, maxEnd)`; the lower bound is carried by the element type
itself, so only the ceiling is named. -/
class PointDomain (α : Type) [LT α] [LE α] where
  /-- The domain ceiling: `«end» = maxEnd` denotes the unbounded ray. -/
  maxEnd : α
  /-- The measure payload: `gap start «end»` is `«end» − start`. -/
  gap : α → α → Nat
  /-- Reflexivity of the element order. -/
  le_refl : ∀ a : α, a ≤ a

instance : PointDomain U64 where
  maxEnd := U64.maxEnd
  gap a b := b.val - a.val
  le_refl a := Nat.le_refl a.val

instance : PointDomain I64 where
  maxEnd := I64.maxEnd
  gap a b := (b.val - a.val).toNat
  le_refl a := Int.le_refl a.val

/-! ## Intervals -/

/-- A half-open interval `[start, «end»)`: a set of points written as
its bounds, strictly `start < «end»` — nonemptiness by construction,
carried as the field `h`, mirroring `crate::Interval` (whose
constructors return `Option`; `crate::Interval::new` is the Bridge
mechanism that discharges `h`). No constructor bypasses the
invariant: `Interval.mk` demands the proof.

Deliberately NO `LT`/`Ord` instance: the value order the encoding has
is an encoding fact (`encode_interval_order`), not semantics. -/
structure Interval (α : Type) [LT α] where
  /-- The inclusive lower bound. -/
  start : α
  /-- The exclusive upper bound. -/
  «end» : α
  /-- The invariant: nonemptiness by construction. -/
  h : start < «end»

variable {α : Type} [LT α] [LE α] [PointDomain α]

omit [LE α] [PointDomain α] in
/-- Two intervals with equal bounds are equal (the invariant proof is
irrelevant). -/
theorem Interval.ext {iv jv : Interval α}
    (hs : iv.start = jv.start) (he : iv.«end» = jv.«end») : iv = jv := by
  cases iv; cases jv; cases hs; cases he; rfl

/-- The half-open denotation: the set of points `[start, «end»)`. -/
def Interval.points (iv : Interval α) : Set α :=
  fun x => iv.start ≤ x ∧ x < iv.«end»

/-- Whether the interval is the unbounded ray `[start, ∞)`: `«end»`
IS the ceiling — ∞ is a value of the representation, not a sentinel
(`crate::Interval::is_ray`). -/
def Interval.isRay (iv : Interval α) : Prop :=
  iv.«end» = PointDomain.maxEnd

instance [DecidableEq α] (iv : Interval α) : Decidable iv.isRay :=
  inferInstanceAs (Decidable (iv.«end» = PointDomain.maxEnd))

/-- The measure: `none` on rays (the MeasureOfRay law), else
`some («end» − start)` via the domain's `gap`. -/
def Interval.measure [DecidableEq α] (iv : Interval α) : Option Nat :=
  if iv.isRay then none else some (PointDomain.gap iv.start iv.«end»)

/-- Interval membership is decidable by the two boundary comparisons
(PRD 13 wants computable forms). -/
instance [DecidableLT α] [DecidableLE α] (x : α) (iv : Interval α) :
    Decidable (x ∈ iv.points) :=
  inferInstanceAs (Decidable (iv.start ≤ x ∧ x < iv.«end»))

/-! ## The interval theorems (the module's spine) -/

/-- **Theorem 1.** Every representable interval denotes a nonempty
point set — the premise the Rust constructor discharges.
Bridge: `crate::Interval::new` (`crates/bumbledb/src/interval.rs`). -/
theorem interval_nonempty (iv : Interval α) : iv.points.Nonempty :=
  ⟨iv.start, PointDomain.le_refl iv.start, iv.h⟩

omit [PointDomain α] in
/-- **Theorem 2.** Membership is exactly the half-open reading —
inclusive at `start`, exclusive at `«end»`.
Bridge: the half-open contract every `crate::Interval` consumer
assumes (`start`/`end` accessors; `crate::interval::sweep`;
`crate::allen::classify`). -/
theorem points_halfopen (iv : Interval α) (x : α) :
    x ∈ iv.points ↔ iv.start ≤ x ∧ x < iv.«end» :=
  Iff.rfl

/-- **Theorem 3.** Over the point domain (`x < maxEnd`; the lower
bound is the element type's), a ray's points are exactly the
unbounded tail `start ≤ x` — "∞ is a value of the representation"
made a theorem.
Bridge: `crate::Interval::ray` / `crate::Interval::is_ray`. -/
theorem ray_is_unbounded_tail (iv : Interval α) (hray : iv.isRay)
    (x : α) (hx : x < PointDomain.maxEnd) :
    x ∈ iv.points ↔ iv.start ≤ x := by
  have he : iv.«end» = (PointDomain.maxEnd : α) := hray
  exact ⟨fun hmem => hmem.1, fun hle => ⟨hle, he ▸ hx⟩⟩

/-- **Theorem 4a.** A ray has no measure — the MeasureOfRay law.
Bridge: the `crate::Error::MeasureOfRay` guard on
`crate::ir::Term::Measure` evaluation. -/
theorem measure_ray_none [DecidableEq α] (iv : Interval α)
    (hray : iv.isRay) : iv.measure = none := by
  unfold Interval.measure
  rw [if_pos hray]

/-- **Theorem 4b.** A bounded interval's measure is exactly
`«end» − start`.
Bridge: the happy path of the same `Term::Measure` evaluation. -/
theorem measure_finite [DecidableEq α] (iv : Interval α)
    (hbounded : ¬ iv.isRay) :
    iv.measure = some (PointDomain.gap iv.start iv.«end») := by
  unfold Interval.measure
  rw [if_neg hbounded]

/-! ## Encodings — order-embedding claims into the word domain -/

/-- The abstract word domain the encodings embed into: the encodings
are order-embedding claims, not byte layouts. -/
abbrev Word : Type := Nat

/-- The `u64` encoding: the identity embedding (big-endian bytes sort
numerically — at this level, the identity). -/
def encodeU64 (a : U64) : Word := a.val

/-- The `i64` encoding: the sign-flip as its bias form `x + 2^63` —
what flipping the sign bit computes on two's-complement words. -/
def encodeI64 (a : I64) : Word := (a.val + 2 ^ 63).toNat

theorem encodeU64_le_iff (a b : U64) :
    encodeU64 a ≤ encodeU64 b ↔ a ≤ b := Iff.rfl

theorem encodeU64_lt_iff (a b : U64) :
    encodeU64 a < encodeU64 b ↔ a < b := Iff.rfl

theorem encodeU64_eq_iff (a b : U64) :
    encodeU64 a = encodeU64 b ↔ a = b :=
  ⟨fun h => Subtype.ext h, fun h => h ▸ rfl⟩

theorem encodeI64_lt_iff (a b : I64) :
    encodeI64 a < encodeI64 b ↔ a < b := by
  have ha := a.property
  have hb := b.property
  show (a.val + 2 ^ 63).toNat < (b.val + 2 ^ 63).toNat ↔ a.val < b.val
  omega

theorem encodeI64_eq_iff (a b : I64) :
    encodeI64 a = encodeI64 b ↔ a = b := by
  have ha := a.property
  have hb := b.property
  constructor
  · intro heq
    exact Subtype.ext (by
      have : (a.val + 2 ^ 63).toNat = (b.val + 2 ^ 63).toNat := heq
      omega)
  · intro heq
    rw [heq]

/-- **Theorem 5 (U64 companion).** The `u64` encoding is an order
embedding. Bridge: `crate::encoding::encode::encode_u64`. -/
theorem encode_u64_order_embedding (a b : U64) :
    a ≤ b ↔ encodeU64 a ≤ encodeU64 b :=
  (encodeU64_le_iff a b).symm

/-- **Theorem 5.** The sign-flip law: the `i64` encoding is an order
embedding — lexicographic word order equals numeric order.
Bridge: `crate::encoding::encode::encode_i64`, sampled exhaustively
by the engine's order suite. -/
theorem encode_i64_order_embedding (a b : I64) :
    a ≤ b ↔ encodeI64 a ≤ encodeI64 b := by
  have ha := a.property
  have hb := b.property
  show a.val ≤ b.val ↔ (a.val + 2 ^ 63).toNat ≤ (b.val + 2 ^ 63).toNat
  omega

/-- Lexicographic order on encoded word pairs — the order the
determinant walks read off the two-half interval encoding. -/
def lexLt (p q : Word × Word) : Prop :=
  p.1 < q.1 ∨ (p.1 = q.1 ∧ p.2 < q.2)

/-- The two-half `i64` interval encoding: `start ‖ «end»`, each half
`encodeI64`. -/
def encodeIntervalI64 (iv : Interval I64) : Word × Word :=
  (encodeI64 iv.start, encodeI64 iv.«end»)

/-- The two-half `u64` interval encoding: `start ‖ «end»`, each half
`encodeU64`. -/
def encodeIntervalU64 (iv : Interval U64) : Word × Word :=
  (encodeU64 iv.start, encodeU64 iv.«end»)

/-- **Theorem 6.** The two-half encoding preserves the
`(start, «end»)` lexicographic order used by the determinant walks —
stated over the encoded pair, because `Interval` itself deliberately
carries no order. Bridge: `crate::encoding::encode::encode_interval_i64`
(the storage layer's neighbor probes). -/
theorem encode_interval_order (iv jv : Interval I64) :
    lexLt (encodeIntervalI64 iv) (encodeIntervalI64 jv) ↔
      (iv.start < jv.start ∨
        (iv.start = jv.start ∧ iv.«end» < jv.«end»)) := by
  unfold lexLt encodeIntervalI64
  rw [encodeI64_lt_iff, encodeI64_lt_iff, encodeI64_eq_iff]

/-- **Theorem 6 (U64 companion).** Bridge:
`crate::encoding::encode::encode_interval_u64`. -/
theorem encode_interval_order_u64 (iv jv : Interval U64) :
    lexLt (encodeIntervalU64 iv) (encodeIntervalU64 jv) ↔
      (iv.start < jv.start ∨
        (iv.start = jv.start ∧ iv.«end» < jv.«end»)) := by
  unfold lexLt encodeIntervalU64
  rw [encodeU64_lt_iff, encodeU64_lt_iff, encodeU64_eq_iff]

/-! ## The value universe -/

/-- An interval's element type: the two orderable scalars. -/
inductive Elem where
  | u64
  | i64
deriving DecidableEq

/-- The six structural value types. Types are encodings; there is no
nominal typing anywhere in the universe. -/
inductive ValueType where
  | bool
  | u64
  | i64
  /-- Interned string identity — equality only, NO order (see the
  module doc). -/
  | str
  /-- `bytes<N>`: the length is the type. -/
  | fixedBytes (n : Nat)
  /-- A nonempty half-open interval over an orderable scalar. -/
  | interval (e : Elem)
deriving DecidableEq

/-- An opaque intern id: equality only. NO `LT`/`LE`/`Ord` instance
exists — a deliberate absence (see the module doc; machine-checked in
`Countermodels.lean`). -/
structure StrId where
  id : Nat
deriving DecidableEq

/-- A `bytes<N>` payload: exactly `n` words. The zero-pad to the word
boundary is encoding, not data (constant for fixed `n`). -/
abbrev FixedBytes (n : Nat) : Type := { l : List Word // l.length = n }

/-- Each value type's carrier. -/
def ValueType.carrier : ValueType → Type
  | .bool => Bool
  | .u64 => U64
  | .i64 => I64
  | .str => StrId
  | .fixedBytes n => FixedBytes n
  | .interval .u64 => Interval U64
  | .interval .i64 => Interval I64

/-- A value: the dependent sum over `ValueType` — a type together
with an inhabitant of its carrier (`crate::value::Value`). -/
structure Value where
  type : ValueType
  val : type.carrier

/-- The canonical encoding of a value of known type, as words —
abstract canonical bytes (`crate::encoding::encode::encode_literal`).
Str encodes its intern id: canonical WITHIN one database (interning
is per-database — the Bridge row carries that caveat). -/
def encodeAt : (t : ValueType) → t.carrier → List Word
  | .bool, b => [cond b 1 0]
  | .u64, v => [encodeU64 v]
  | .i64, v => [encodeI64 v]
  | .str, s => [s.id]
  | .fixedBytes _, bs => bs.val
  | .interval .u64, iv => [(encodeIntervalU64 iv).1, (encodeIntervalU64 iv).2]
  | .interval .i64, iv => [(encodeIntervalI64 iv).1, (encodeIntervalI64 iv).2]

/-- A value's canonical encoding. -/
def Value.encode (v : Value) : List Word := encodeAt v.type v.val

/-- **Theorem 7.** Canonical-bytes identity: within one value type,
two values are equal exactly when their canonical encodings are —
the fact-identity law. Stated per type, because cross-type injectivity
is deliberately FALSE (a str intern id and a u64 encode alike; the
column type disambiguates). Bridge:
`crate::encoding::encode::encode_literal` / `encode_fact`. -/
theorem value_eq_iff_encode_eq (t : ValueType) (a b : t.carrier) :
    a = b ↔ encodeAt t a = encodeAt t b := by
  refine ⟨fun heq => heq ▸ rfl, fun heq => ?_⟩
  match t, a, b with
  | .bool, a, b =>
    cases a <;> cases b <;> simp_all [encodeAt]
  | .u64, a, b =>
    simp only [encodeAt, List.cons.injEq, and_true] at heq
    exact (encodeU64_eq_iff a b).mp heq
  | .i64, a, b =>
    simp only [encodeAt, List.cons.injEq, and_true] at heq
    exact (encodeI64_eq_iff a b).mp heq
  | .str, a, b =>
    simp only [encodeAt, List.cons.injEq, and_true] at heq
    cases a; cases b; cases heq; rfl
  | .fixedBytes n, a, b =>
    exact Subtype.ext heq
  | .interval .u64, a, b =>
    simp only [encodeAt, encodeIntervalU64, List.cons.injEq, and_true] at heq
    exact Interval.ext ((encodeU64_eq_iff _ _).mp heq.1)
      ((encodeU64_eq_iff _ _).mp heq.2)
  | .interval .i64, a, b =>
    simp only [encodeAt, encodeIntervalI64, List.cons.injEq, and_true] at heq
    exact Interval.ext ((encodeI64_eq_iff _ _).mp heq.1)
      ((encodeI64_eq_iff _ _).mp heq.2)

end Bumbledb
