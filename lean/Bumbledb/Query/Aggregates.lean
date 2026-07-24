import Bumbledb.Query.Denotation

/-!
# Aggregates — folds, measure, Pack, Allen (Level 0, PRD 05)

The aggregate boundary contracts as theorems: every aggregate folds
the DISTINCT binding set of its group (grouping is the fibering of
PRD 04's binding sets over the EVALUATED head values — answers are
value tuples, and the group key is what each find term PROJECTS, the
F4 decision recorded at the grouping section), checked sums are the
`Overflow(Aggregate)` spec, the measure folds inherit the ray refusal
as `Option`-poisoning, `pack` is the coalescing fold (sort by start,
coalesce overlapping-or-adjacent, emit maximal segments), and
`classify` is the DEFINED 13-relation Allen classifier — the
refinement of PRD 04's abstract `Classify` parameter.

## The artifact divergence, recorded (empty global aggregates)

The seed artifact (`docs/formal/GPT55DependencyTheory.lean`) modeled
`aggEval sum [] = some 0` / `count [] = some 0` — SQL's ungrouped-
aggregate reading. The ENGINE's contract is the opposite, and the
engine is the authority: a global aggregate over the empty binding
set yields the EMPTY ANSWER SET — not a zero row ("the balance of an
account with no postings is an absent answer, not 0",
`20-query-ir.md` § aggregation). Mechanism: the finalize loop
iterates the group map and a group exists only on first sight of a
binding (`exec/sink/aggregate/finalize.rs`, `groups.rs::probe_group`),
so empty input emits zero rows. The model follows the engine —
`aggAnswers` demands a deriving witness (`empty_global_no_answer`) —
and the artifact's `sum_empty`/`count_empty` are deliberately NOT
ported. The refused zero-row reading gets its countermodel:
`Countermodels.sql_zero_row_from_no_binding`.

## The creation quarantine (module note — `20-query-ir.md`'s decision
record cites this)

The creators are boundary-only: atoms select, filters compare, and
value CREATION happens once, over finished binding sets, exiting to
the host. The inventory: the measure and the folds (`Sum`, `Count`,
`CountDistinct`) create values outside the active domain; `pack`
creates LATTICE-CLOSED values — a coalesced segment's endpoints are
SELECTED from stored endpoints, never invented
(`pack_lattice_closed`) — and `Min`/`Max`/`ArgMax`/`ArgMin` select
outright. The lattice-closedness is the chain-window fence's premise
(`20-query-ir.md` § engine recursion, the chain-window fence) and the
fence for every future interval
operator: only endpoint-selecting operations are ever candidates;
endpoint-inventing ones (shift, widen, bound arithmetic) are refused
categorically.

## Narrowings recorded (law 5: narrow and record)

* **`LinearElem` is the order toolkit, as a `Prop`-class.**
  `PointDomain` (PRD 02) deliberately carries only `le_refl`; the
  interval algorithms here need the linear-order facts (trichotomy,
  transitivity, the `≤`/`<` bridge). Rather than widen PRD 02's
  class, the facts live in a separate `Prop`-class instantiated by
  the two real element domains (by `omega`) — mirroring `Ord + Copy`
  on the Rust side. The general `allen_jepd` therefore needed NO
  two-domain narrowing (the spec's recorded fallback went unspent).
* **Arg keys compare as encoded words.** `argMaxSet` orders its key
  by a `Nat`-valued observer — the engine compares encoded words
  (`fold_row.rs::fold_arg`), and the encodings are order embeddings
  (`encode_u64_order_embedding` / `encode_i64_order_embedding`), so
  word order IS value order.
* **`AggOp` is the head-shape row** (the narrowing PRD 04 recorded:
  finds degenerate to variables there; the aggregate find shapes
  arrive here). The theorems are stated over the underlying folds
  and sets, not by recursion over `AggOp` — one fold law per
  contract, uniformly quantified where the contract is op-generic
  (`agg_over_distinct_bindings` holds for EVERY fold, which is
  exactly "set semantics through aggregation").
* **`measure_fold_laws` models the error as `Option`-poisoning.**
  The engine raises the typed `crate::Error::MeasureOfRay` and drops
  the execution (`fold_row.rs`: a poisoned sink folds nothing more);
  this level has no effect to carry, so a ray in the group makes the
  whole measure column `none` — erroneous, never a value.
-/

namespace Bumbledb

/-! ## The order toolkit — `LinearElem` -/

/-- The linear-order facts the interval algorithms spend (`pack`'s
coalesce, `classify`'s endpoint trichotomy) — a `Prop`-class over the
element order, instantiated by the two real element domains. Mirrors
the Rust bound `T: Ord` (`interval/sweep.rs`, `allen.rs`). -/
class LinearElem (α : Type) [LT α] [LE α] : Prop where
  /-- Strict order is irreflexive. -/
  lt_irrefl : ∀ a : α, ¬ a < a
  /-- Strict order is transitive. -/
  lt_trans : ∀ {a b c : α}, a < b → b < c → a < c
  /-- Any two elements compare: exactly the 3-way `cmp`. -/
  trichotomy : ∀ a b : α, a < b ∨ a = b ∨ b < a
  /-- The `≤`/`<` bridge. -/
  le_iff : ∀ a b : α, a ≤ b ↔ a < b ∨ a = b

instance : LinearElem U64 where
  lt_irrefl a := Nat.lt_irrefl a.val
  lt_trans := Nat.lt_trans
  trichotomy a b := by
    rcases Nat.lt_trichotomy a.val b.val with h | h | h
    · exact .inl h
    · exact .inr (.inl (Subtype.ext h))
    · exact .inr (.inr h)
  le_iff a b := by
    constructor
    · intro h
      rcases Nat.lt_or_ge a.val b.val with hlt | hge
      · exact .inl hlt
      · exact .inr (Subtype.ext (Nat.le_antisymm h hge))
    · rintro (h | rfl)
      · exact Nat.le_of_lt h
      · exact Nat.le_refl a.val

instance : LinearElem I64 where
  lt_irrefl a := Int.lt_irrefl a.val
  lt_trans := Int.lt_trans
  trichotomy a b := by
    rcases Int.lt_trichotomy a.val b.val with h | h | h
    · exact .inl h
    · exact .inr (.inl (Subtype.ext h))
    · exact .inr (.inr h)
  le_iff a b := by
    constructor
    · intro h
      rcases Int.lt_or_le a.val b.val with hlt | hge
      · exact .inl hlt
      · exact .inr (Subtype.ext (Int.le_antisymm h hge))
    · rintro (h | rfl)
      · exact Int.le_of_lt h
      · exact Int.le_refl a.val

section OrderLemmas

variable {α : Type} [LT α] [LE α] [LinearElem α]

theorem LinearElem.le_refl (a : α) : a ≤ a :=
  (le_iff a a).mpr (.inr rfl)

theorem LinearElem.le_of_lt {a b : α} (h : a < b) : a ≤ b :=
  (le_iff a b).mpr (.inl h)

theorem LinearElem.lt_asymm {a b : α} (h : a < b) : ¬ b < a :=
  fun h' => lt_irrefl a (lt_trans h h')

theorem LinearElem.ne_of_lt {a b : α} (h : a < b) : a ≠ b :=
  fun heq => lt_irrefl a (heq ▸ h)

theorem LinearElem.lt_of_le_of_lt {a b c : α} (h : a ≤ b) (h' : b < c) :
    a < c := by
  rcases (le_iff a b).mp h with hlt | rfl
  · exact lt_trans hlt h'
  · exact h'

theorem LinearElem.lt_of_lt_of_le {a b c : α} (h : a < b) (h' : b ≤ c) :
    a < c := by
  rcases (le_iff b c).mp h' with hlt | rfl
  · exact lt_trans h hlt
  · exact h

theorem LinearElem.le_trans {a b c : α} (h : a ≤ b) (h' : b ≤ c) :
    a ≤ c := by
  rcases (le_iff a b).mp h with hlt | rfl
  · exact le_of_lt (lt_of_lt_of_le hlt h')
  · exact h'

theorem LinearElem.le_of_not_lt {a b : α} (h : ¬ b < a) : a ≤ b := by
  rcases trichotomy a b with hlt | rfl | hgt
  · exact le_of_lt hlt
  · exact le_refl a
  · exact absurd hgt h

theorem LinearElem.not_lt_of_le {a b : α} (h : a ≤ b) : ¬ b < a := by
  intro h'
  rcases (le_iff a b).mp h with hlt | rfl
  · exact lt_asymm hlt h'
  · exact lt_irrefl a h'

theorem LinearElem.le_total (a b : α) : a ≤ b ∨ b ≤ a := by
  rcases trichotomy a b with hlt | rfl | hgt
  · exact .inl (le_of_lt hlt)
  · exact .inl (le_refl a)
  · exact .inr (le_of_lt hgt)

end OrderLemmas

/-! ## Checked sums — the Overflow(Aggregate) spec -/

/-- Checked addition for a bounded result domain: `none` past the
limit (port of the artifact's `checkedAdd`) — the model of the
finalize range check (`finalize.rs::finalize_acc`, `i64::try_from` /
`u64::try_from`). -/
def checkedAdd (limit a b : Nat) : Option Nat :=
  if a + b ≤ limit then some (a + b) else none

/-- Checked sum: fold `checkedAdd`, poisoning on the first overflow
(port of the artifact's `checkedSum`). -/
def checkedSum (limit : Nat) : List Nat → Option Nat
  | [] => some 0
  | x :: xs =>
    match checkedSum limit xs with
    | none => none
    | some s => checkedAdd limit x s

/-- The mathematical sum the checked forms are measured against. -/
def natSum : List Nat → Nat
  | [] => 0
  | x :: xs => x + natSum xs

/-- Port of the artifact's `checkedAdd_sound`: success is the exact
sum, within the limit. -/
theorem checkedAdd_sound {limit a b s : Nat}
    (h : checkedAdd limit a b = some s) : s = a + b ∧ s ≤ limit := by
  unfold checkedAdd at h
  by_cases hle : a + b ≤ limit
  · rw [if_pos hle] at h
    cases h
    exact ⟨rfl, hle⟩
  · rw [if_neg hle] at h
    cases h

/-- **Theorem 3 (`checkedSum_sound`).** A successful checked sum IS
the mathematical sum, within bounds — the `Overflow(Aggregate)` spec:
an emitted Sum value is exact, and overflow is a typed error, never a
wrap. Bridge: `finalize.rs::finalize_acc` (the once-per-group range
check); the artifact's `checkedAdd_sound`, ported and folded. -/
theorem checkedSum_sound {limit : Nat} :
    ∀ {xs : List Nat} {s : Nat},
      checkedSum limit xs = some s → s = natSum xs ∧ s ≤ limit
  | [], s, h => by
    cases h
    exact ⟨rfl, Nat.zero_le _⟩
  | x :: xs, s, h => by
    unfold checkedSum at h
    cases hxs : checkedSum limit xs with
    | none => rw [hxs] at h; cases h
    | some t =>
      rw [hxs] at h
      obtain ⟨rfl, hlim⟩ := checkedAdd_sound h
      obtain ⟨rfl, -⟩ := checkedSum_sound hxs
      exact ⟨rfl, hlim⟩

/-- A sum within the limit always succeeds — the completeness half
`wide_accumulator_exact` spends. -/
theorem checkedSum_complete {limit : Nat} :
    ∀ {xs : List Nat}, natSum xs ≤ limit →
      checkedSum limit xs = some (natSum xs)
  | [], _ => rfl
  | x :: xs, h => by
    have hxs : natSum xs ≤ limit :=
      Nat.le_trans (Nat.le_add_left _ _) h
    unfold checkedSum
    rw [checkedSum_complete hxs]
    show checkedAdd limit x (natSum xs) = some (natSum (x :: xs))
    unfold checkedAdd
    rw [if_pos (show x + natSum xs ≤ limit from h)]
    rfl

/-- The sum of bounded terms is bounded by count × bound. -/
theorem natSum_le_length_mul {bound : Nat} :
    ∀ {xs : List Nat}, (∀ x ∈ xs, x ≤ bound) →
      natSum xs ≤ xs.length * bound
  | [], _ => Nat.zero_le _
  | x :: xs, h => by
    have hx : x ≤ bound := h x (List.mem_cons_self ..)
    have hxs := natSum_le_length_mul fun y hy =>
      h y (List.mem_cons_of_mem _ hy)
    show x + natSum xs ≤ (xs.length + 1) * bound
    calc x + natSum xs ≤ bound + xs.length * bound :=
          Nat.add_le_add hx hxs
      _ = (xs.length + 1) * bound := by
          rw [Nat.succ_mul, Nat.add_comm]

/-- **The i128-accumulator argument, stated abstractly.** Fewer than
`2^64` terms, each a 64-bit value, cannot overflow the 128-bit
accumulator: the wide checked sum ALWAYS succeeds exactly, so the
only narrowing point is finalization (`checkedSum_sound` at the
result limit). Bridge: `fold_row.rs`/`sink.rs` accumulate in
`i128`/`u128` and never check; `finalize.rs` range-checks once —
"deterministic by construction". -/
theorem wide_accumulator_exact {xs : List Nat}
    (hterm : ∀ x ∈ xs, x ≤ 2 ^ 64 - 1) (hlen : xs.length < 2 ^ 64) :
    checkedSum (2 ^ 128 - 1) xs = some (natSum xs) := by
  have hsum := natSum_le_length_mul hterm
  have hbound : natSum xs ≤ 2 ^ 128 - 1 := by
    have hlen' : xs.length * (2 ^ 64 - 1) ≤
        (2 ^ 64 - 1) * (2 ^ 64 - 1) :=
      Nat.mul_le_mul_right _ (by omega)
    omega
  exact checkedSum_complete hbound

/-! ## Pack — the coalescing fold

`pack` is structured exactly as the proof guidance demands: sort by
start, then ONE coalescing fold (`coalesce`) — the Lean image of the
engine's `sort_unstable` pass + windowless sweep
(`finalize.rs::finalize_into`, `interval/sweep.rs`). Insertion sort,
not core's `mergeSort`: the coalesce examples are kernel-evaluated
(`decide`), and `mergeSort`'s well-founded recursion does not
kernel-reduce — a recorded representation choice, not semantics
(sortedness is all any theorem reads). The engine sorts claims
lexicographically on `[start, end]` where this sort reads starts
alone; the tie order among equal starts is provably invisible —
`pack_input_order_irrelevant` (pack is a function of the point union)
and the Level-1 transfer `Exec/Sweep.lean:
sweepRuns_tie_order_irrelevant`. -/

section Pack

variable {α : Type} [LT α] [LE α] [LinearElem α] [DecidableLT α]
  [DecidableLE α]

/-- Insert into a start-sorted list, keeping it sorted. -/
def insertByStart (iv : Interval α) : List (Interval α) → List (Interval α)
  | [] => [iv]
  | jv :: rest =>
    if iv.start ≤ jv.start then iv :: jv :: rest
    else jv :: insertByStart iv rest

/-- Sort by start — pack's first pass. -/
def sortByStart : List (Interval α) → List (Interval α)
  | [] => []
  | iv :: rest => insertByStart iv (sortByStart rest)

omit [LinearElem α] [DecidableLT α] in
theorem mem_insertByStart {iv jv : Interval α} :
    ∀ {l : List (Interval α)}, jv ∈ insertByStart iv l ↔ jv = iv ∨ jv ∈ l
  | [] => by simp [insertByStart]
  | kv :: rest => by
    unfold insertByStart
    by_cases hle : iv.start ≤ kv.start
    · rw [if_pos hle]
      simp [List.mem_cons]
    · rw [if_neg hle]
      rw [List.mem_cons, mem_insertByStart (l := rest), List.mem_cons]
      constructor
      · rintro (h | h | h)
        · exact .inr (.inl h)
        · exact .inl h
        · exact .inr (.inr h)
      · rintro (h | h | h)
        · exact .inr (.inl h)
        · exact .inl h
        · exact .inr (.inr h)

omit [DecidableLT α] in
theorem pairwise_insertByStart {iv : Interval α} :
    ∀ {l : List (Interval α)},
      l.Pairwise (fun a b => a.start ≤ b.start) →
      (insertByStart iv l).Pairwise (fun a b => a.start ≤ b.start)
  | [], _ => List.pairwise_cons.mpr ⟨by simp, List.Pairwise.nil⟩
  | kv :: rest, h => by
    obtain ⟨hkv, hrest⟩ := List.pairwise_cons.mp h
    unfold insertByStart
    by_cases hle : iv.start ≤ kv.start
    · rw [if_pos hle]
      refine List.pairwise_cons.mpr ⟨?_, h⟩
      intro jv hjv
      rcases List.mem_cons.mp hjv with rfl | hmem
      · exact hle
      · exact LinearElem.le_trans hle (hkv jv hmem)
    · rw [if_neg hle]
      have hkle : kv.start ≤ iv.start := by
        rcases LinearElem.le_total iv.start kv.start with h' | h'
        · exact absurd h' hle
        · exact h'
      refine List.pairwise_cons.mpr
        ⟨?_, pairwise_insertByStart hrest⟩
      intro jv hjv
      rcases mem_insertByStart.mp hjv with rfl | hmem
      · exact hkle
      · exact hkv jv hmem

omit [DecidableLT α] in
theorem pairwise_sortByStart :
    ∀ (l : List (Interval α)),
      (sortByStart l).Pairwise (fun a b => a.start ≤ b.start)
  | [] => List.Pairwise.nil
  | _ :: rest => pairwise_insertByStart (pairwise_sortByStart rest)

omit [LinearElem α] [DecidableLT α] in
theorem mem_sortByStart {jv : Interval α} :
    ∀ {l : List (Interval α)}, jv ∈ sortByStart l ↔ jv ∈ l
  | [] => Iff.rfl
  | iv :: rest => by
    unfold sortByStart
    rw [mem_insertByStart, mem_sortByStart (l := rest), List.mem_cons]

omit [LinearElem α] [DecidableLT α] in
/-- Inserting below the whole list lands at the head. -/
theorem insertByStart_of_le {iv : Interval α} :
    ∀ {l : List (Interval α)}, (∀ jv ∈ l, iv.start ≤ jv.start) →
      insertByStart iv l = iv :: l
  | [], _ => rfl
  | kv :: rest, h => by
    unfold insertByStart
    rw [if_pos (h kv (List.mem_cons_self ..))]

omit [LinearElem α] [DecidableLT α] in
/-- A start-sorted list is `sortByStart`'s fixpoint — the sort seam
the tie-order transfer crosses (`Exec/Sweep.lean:
sweepRuns_tie_order_irrelevant`): the engine's sort pass hands the
fold a start-ordered list, on which the spec's sort changes
nothing. -/
theorem sortByStart_id_of_sorted :
    ∀ {l : List (Interval α)},
      l.Pairwise (fun a b => a.start ≤ b.start) → sortByStart l = l
  | [], _ => rfl
  | iv :: rest, h => by
    obtain ⟨hhd, hrest⟩ := List.pairwise_cons.mp h
    unfold sortByStart
    rw [sortByStart_id_of_sorted hrest, insertByStart_of_le hhd]

/-- The frontier join: the larger bound. -/
def maxE (a b : α) : α := if a ≤ b then b else a

omit [DecidableLT α] in
theorem le_maxE_left (a b : α) : a ≤ maxE a b := by
  unfold maxE
  by_cases h : a ≤ b
  · rw [if_pos h]; exact h
  · rw [if_neg h]; exact LinearElem.le_refl a

omit [DecidableLT α] in
theorem le_maxE_right (a b : α) : b ≤ maxE a b := by
  unfold maxE
  by_cases h : a ≤ b
  · rw [if_pos h]; exact LinearElem.le_refl b
  · rw [if_neg h]
    rcases LinearElem.le_total a b with h' | h'
    · exact absurd h' h
    · exact h'

omit [LT α] [LinearElem α] [DecidableLT α] in
theorem maxE_eq_or (a b : α) : maxE a b = a ∨ maxE a b = b := by
  unfold maxE
  by_cases h : a ≤ b
  · rw [if_pos h]; exact .inr rfl
  · rw [if_neg h]; exact .inl rfl

/-- The coalescing fold over the start-sorted tail, carrying the open
run `[s, f)`: `start ≤ frontier` (overlap OR half-open adjacency)
extends the frontier to the max; `frontier < start` is the gap that
emits the maximal segment and opens a new run; exhaustion emits the
last run. The Lean image of the windowless sweep
(`interval/sweep.rs::sweep`, Pack's shape). -/
def coalesce (s f : α) (h : s < f) :
    List (Interval α) → List (Interval α)
  | [] => [⟨s, f, h⟩]
  | iv :: rest =>
    if f < iv.start then
      ⟨s, f, h⟩ :: coalesce iv.start iv.«end» iv.h rest
    else
      coalesce s (maxE f iv.«end»)
        (LinearElem.lt_of_lt_of_le h (le_maxE_left f iv.«end»)) rest

/-- The coalescing fold over an already-sorted list. -/
def packSorted : List (Interval α) → List (Interval α)
  | [] => []
  | iv :: rest => coalesce iv.start iv.«end» iv.h rest

/-- **`pack`** — sort by start, coalesce overlapping-or-adjacent,
emit maximal segments (`20-query-ir.md` § aggregation; computable —
PRD 13 evaluates it). Its specs are `pack_canonical`,
`pack_extensional`, `pack_adjacency`, `pack_lattice_closed`. -/
def pack (l : List (Interval α)) : List (Interval α) :=
  packSorted (sortByStart l)

/-! ### Pack's specs -/

/-- The canonical-output predicate: consecutive segments separated by
a REAL gap (`«end» < start` — disjoint AND non-adjacent; half-open
adjacency would have been coalesced). Implies start-sortedness and
all-pairs disjointness (`separated_pairwise`); with
`pack_extensional` it is exactly "maximal segments". -/
def Separated : List (Interval α) → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => a.«end» < b.start ∧ Separated (b :: rest)

/-- The run start is pinned: `coalesce` always emits a first segment
starting at `s`, with a frontier no smaller than `f`. -/
theorem coalesce_head :
    ∀ (l : List (Interval α)) (s f : α) (h : s < f),
      ∃ f', ∃ h' : s < f', ∃ tl : List (Interval α),
        coalesce s f h l = ⟨s, f', h'⟩ :: tl ∧ f ≤ f'
  | [], s, f, h => ⟨f, h, [], rfl, LinearElem.le_refl f⟩
  | iv :: rest, s, f, h => by
    unfold coalesce
    by_cases hgap : f < iv.start
    · rw [if_pos hgap]
      exact ⟨f, h, coalesce iv.start iv.«end» iv.h rest, rfl,
        LinearElem.le_refl f⟩
    · rw [if_neg hgap]
      obtain ⟨f', h', tl, heq, hle⟩ :=
        coalesce_head rest s (maxE f iv.«end»)
          (LinearElem.lt_of_lt_of_le h (le_maxE_left f iv.«end»))
      exact ⟨f', h', tl, heq,
        LinearElem.le_trans (le_maxE_left f iv.«end») hle⟩

theorem coalesce_separated :
    ∀ (l : List (Interval α)) (s f : α) (h : s < f),
      Separated (coalesce s f h l)
  | [], _, _, _ => trivial
  | iv :: rest, s, f, h => by
    unfold coalesce
    by_cases hgap : f < iv.start
    · rw [if_pos hgap]
      obtain ⟨f', h', tl, heq, -⟩ :=
        coalesce_head rest iv.start iv.«end» iv.h
      rw [heq]
      exact ⟨hgap, heq ▸ coalesce_separated rest iv.start iv.«end» iv.h⟩
    · rw [if_neg hgap]
      exact coalesce_separated rest s (maxE f iv.«end») _

/-- **Theorem 5 (`pack_canonical`).** Pack output is canonical:
consecutive segments strictly separated (`«end» < start`) — sorted,
pairwise-disjoint, non-adjacent; with `pack_extensional` this IS
maximality (a coalescible pair cannot survive). Bridge: the sweep's
gap law — "only `start > frontier` breaks a run"
(`interval/sweep.rs`); the r18 suites sample it. -/
theorem pack_canonical (l : List (Interval α)) : Separated (pack l) := by
  unfold pack
  cases sortByStart l with
  | nil => trivial
  | cons iv rest => exact coalesce_separated rest iv.start iv.«end» iv.h

omit [DecidableLT α] [DecidableLE α] in
/-- `Separated` propagates past the head: everything later starts
strictly past the head's end. -/
theorem Separated.head_lt :
    ∀ {b : Interval α} {l : List (Interval α)},
      Separated (b :: l) → ∀ jv ∈ l, b.«end» < jv.start
  | _, [], _, _, hjv => nomatch hjv
  | b, c :: rest, h, jv, hjv => by
    obtain ⟨hbc, hrest⟩ := h
    rcases List.mem_cons.mp hjv with rfl | hmem
    · exact hbc
    · exact LinearElem.lt_trans (LinearElem.lt_trans hbc c.h)
        (Separated.head_lt hrest jv hmem)

omit [DecidableLT α] [DecidableLE α] in
/-- The all-pairs reading of `pack_canonical`: every pair of packed
segments, not just consecutive ones, is gap-separated. -/
theorem separated_pairwise :
    ∀ {l : List (Interval α)}, Separated l →
      l.Pairwise (fun a b => a.«end» < b.start)
  | [], _ => List.Pairwise.nil
  | [_], _ => List.pairwise_cons.mpr
      ⟨(fun _ h => nomatch h), List.Pairwise.nil⟩
  | _ :: b :: rest, h => by
    obtain ⟨hab, hrest⟩ := h
    exact List.pairwise_cons.mpr
      ⟨Separated.head_lt ⟨hab, hrest⟩, separated_pairwise hrest⟩

/-! ### Extensionality — the support-union law -/

/-- The union of a claim list's point sets — the support
`pack_extensional` preserves. -/
def unionPoints (l : List (Interval α)) : Set α :=
  fun x => ∃ iv, iv ∈ l ∧ x ∈ iv.points

omit [LinearElem α] [DecidableLT α] [DecidableLE α] in
theorem mem_unionPoints_cons {iv : Interval α} {l : List (Interval α)}
    {x : α} :
    x ∈ unionPoints (iv :: l) ↔ x ∈ iv.points ∨ x ∈ unionPoints l := by
  constructor
  · rintro ⟨jv, hjv, hx⟩
    rcases List.mem_cons.mp hjv with rfl | hmem
    · exact .inl hx
    · exact .inr ⟨jv, hmem, hx⟩
  · rintro (hx | ⟨jv, hmem, hx⟩)
    · exact ⟨iv, List.mem_cons_self .., hx⟩
    · exact ⟨jv, List.mem_cons_of_mem _ hmem, hx⟩

/-- The coalescing fold accounts for every point exactly: the output
union is the open run's points plus the input union — the invariant
that makes `pack_extensional` an induction. Sortedness is
load-bearing HERE (an unsorted merge could orphan a claim behind the
frontier), where `coalesce_separated` needed none. -/
theorem coalesce_points :
    ∀ (l : List (Interval α)) (s f : α) (h : s < f),
      (∀ jv ∈ l, s ≤ jv.start) →
      l.Pairwise (fun a b => a.start ≤ b.start) →
      ∀ x, x ∈ unionPoints (coalesce s f h l) ↔
        (s ≤ x ∧ x < f) ∨ x ∈ unionPoints l
  | [], s, f, h, _, _, x => by
    rw [show coalesce s f h [] = [⟨s, f, h⟩] from rfl,
      mem_unionPoints_cons]
    constructor
    · rintro (hx | ⟨jv, hjv, hx⟩)
      · exact .inl hx
      · nomatch hjv
    · rintro (hx | ⟨jv, hjv, hx⟩)
      · exact .inl hx
      · nomatch hjv
  | iv :: rest, s, f, h, hall, hpw, x => by
    obtain ⟨hhd, hpw'⟩ := List.pairwise_cons.mp hpw
    unfold coalesce
    by_cases hgap : f < iv.start
    · rw [if_pos hgap, mem_unionPoints_cons,
        coalesce_points rest iv.start iv.«end» iv.h hhd hpw' x,
        mem_unionPoints_cons]
      constructor
      · rintro (hx | hx | hx)
        · exact .inl hx
        · exact .inr (.inl hx)
        · exact .inr (.inr hx)
      · rintro (hx | hx | hx)
        · exact .inl hx
        · exact .inr (.inl hx)
        · exact .inr (.inr hx)
    · rw [if_neg hgap]
      have hst : iv.start ≤ f := LinearElem.le_of_not_lt hgap
      have hsiv : s ≤ iv.start := hall iv (List.mem_cons_self ..)
      have hall' : ∀ jv ∈ rest, s ≤ jv.start := fun jv hjv =>
        LinearElem.le_trans hsiv (hhd jv hjv)
      rw [coalesce_points rest s (maxE f iv.«end»)
        (LinearElem.lt_of_lt_of_le h (le_maxE_left f iv.«end»))
        hall' hpw' x, mem_unionPoints_cons]
      have hkey : (s ≤ x ∧ x < maxE f iv.«end») ↔
          (s ≤ x ∧ x < f) ∨ x ∈ iv.points := by
        constructor
        · rintro ⟨hsx, hxm⟩
          have hcase : x < f ∨ f ≤ x := by
            rcases LinearElem.trichotomy x f with h' | rfl | h'
            · exact .inl h'
            · exact .inr (LinearElem.le_refl x)
            · exact .inr (LinearElem.le_of_lt h')
          rcases hcase with hxf | hfx
          · exact .inl ⟨hsx, hxf⟩
          · refine .inr ?_
            show iv.start ≤ x ∧ x < iv.«end»
            refine ⟨LinearElem.le_trans hst hfx, ?_⟩
            rcases maxE_eq_or f iv.«end» with hm | hm
            · rw [hm] at hxm
              exact absurd hxm (LinearElem.not_lt_of_le hfx)
            · rw [hm] at hxm
              exact hxm
        · rintro (⟨hsx, hxf⟩ | hx)
          · exact ⟨hsx,
              LinearElem.lt_of_lt_of_le hxf (le_maxE_left f iv.«end»)⟩
          · have hx' : iv.start ≤ x ∧ x < iv.«end» := hx
            exact ⟨LinearElem.le_trans hsiv hx'.1,
              LinearElem.lt_of_lt_of_le hx'.2 (le_maxE_right f iv.«end»)⟩
      rw [hkey]
      constructor
      · rintro ((hx | hx) | hx)
        · exact .inl hx
        · exact .inr (.inl hx)
        · exact .inr (.inr hx)
      · rintro (hx | hx | hx)
        · exact .inl (.inl hx)
        · exact .inl (.inr hx)
        · exact .inr hx

/-- **Theorem 6 (`pack_extensional`).** The support-union law:
`⋃ points (pack ivs) = ⋃ points ivs` — packing changes the
representation of the claim union, never its points. Bridge:
`interval/sweep.rs`, sampled by the r18 suites'
`packed_output_matches_the_naive_point_set`. -/
theorem pack_extensional (l : List (Interval α)) (x : α) :
    x ∈ unionPoints (pack l) ↔ x ∈ unionPoints l := by
  have hsort : x ∈ unionPoints (sortByStart l) ↔ x ∈ unionPoints l := by
    constructor
    · rintro ⟨iv, hiv, hy⟩
      exact ⟨iv, mem_sortByStart.mp hiv, hy⟩
    · rintro ⟨iv, hiv, hy⟩
      exact ⟨iv, mem_sortByStart.mpr hiv, hy⟩
  rw [← hsort]
  unfold pack
  cases hs : sortByStart l with
  | nil => exact Iff.rfl
  | cons iv rest =>
    have hpw := pairwise_sortByStart l
    rw [hs] at hpw
    obtain ⟨hhd, hpw'⟩ := List.pairwise_cons.mp hpw
    show x ∈ unionPoints (coalesce iv.start iv.«end» iv.h rest) ↔
      x ∈ unionPoints (iv :: rest)
    rw [coalesce_points rest iv.start iv.«end» iv.h hhd hpw' x,
      mem_unionPoints_cons]
    constructor
    · rintro (hx | hx)
      · exact .inl hx
      · exact .inr hx
    · rintro (hx | hx)
      · exact .inl hx
      · exact .inr hx

/-! ### Canonical-form uniqueness — the input-order theorem -/

omit [DecidableLT α] [DecidableLE α] in
/-- The head start of a `Separated` list is a lower bound on its
whole point union — the canonical form's minimum, attained at the
head. -/
theorem Separated.start_le_mem {a : Interval α} {l : List (Interval α)}
    (h : Separated (a :: l)) {x : α}
    (hx : x ∈ unionPoints (a :: l)) : a.start ≤ x := by
  obtain ⟨jv, hjv, hxj⟩ := hx
  have hxj' : jv.start ≤ x ∧ x < jv.«end» := hxj
  rcases List.mem_cons.mp hjv with rfl | hmem
  · exact hxj'.1
  · exact LinearElem.le_of_lt
      (LinearElem.lt_of_lt_of_le
        (LinearElem.lt_trans a.h (Separated.head_lt h jv hmem)) hxj'.1)

omit [DecidableLT α] [DecidableLE α] in
/-- The head end of a `Separated` list is OUTSIDE its point union:
the head's points stop strictly below it and every later segment
starts strictly past it — the seam the uniqueness induction pivots
on. -/
theorem Separated.end_not_mem {a : Interval α} {l : List (Interval α)}
    (h : Separated (a :: l)) : a.«end» ∉ unionPoints (a :: l) := by
  rintro ⟨jv, hjv, hxj⟩
  have hxj' : jv.start ≤ a.«end» ∧ a.«end» < jv.«end» := hxj
  rcases List.mem_cons.mp hjv with rfl | hmem
  · exact LinearElem.lt_irrefl _ hxj'.2
  · exact LinearElem.lt_irrefl _
      (LinearElem.lt_of_lt_of_le (Separated.head_lt h jv hmem) hxj'.1)

omit [LE α] [LinearElem α] [DecidableLT α] [DecidableLE α] in
/-- `Separated` survives beheading. -/
theorem Separated.tail :
    ∀ {a : Interval α} {l : List (Interval α)},
      Separated (a :: l) → Separated l
  | _, [], _ => trivial
  | _, _ :: _, h => h.2

omit [DecidableLT α] [DecidableLE α] in
/-- **Canonical-form uniqueness.** Two `Separated` lists carrying the
same point union are EQUAL — `pack_canonical`'s output predicate plus
extensionality pins the representation, which is exactly the
"maximal segments" reading as an equation. Spent by
`pack_input_order_irrelevant` and, through it, the tie-order transfer
(`Exec/Sweep.lean: sweepRuns_tie_order_irrelevant`). -/
theorem separated_eq_of_unionPoints :
    ∀ {l₁ l₂ : List (Interval α)}, Separated l₁ → Separated l₂ →
      (∀ x, x ∈ unionPoints l₁ ↔ x ∈ unionPoints l₂) → l₁ = l₂
  | [], [], _, _, _ => rfl
  | [], b :: r₂, _, _, hext => by
    obtain ⟨jv, hjv, -⟩ := (hext b.start).mpr
      ⟨b, List.mem_cons_self .., LinearElem.le_refl b.start, b.h⟩
    nomatch hjv
  | a :: r₁, [], _, _, hext => by
    obtain ⟨jv, hjv, -⟩ := (hext a.start).mp
      ⟨a, List.mem_cons_self .., LinearElem.le_refl a.start, a.h⟩
    nomatch hjv
  | a :: r₁, b :: r₂, h₁, h₂, hext => by
    have hba : b.start ≤ a.start := Separated.start_le_mem h₂
      ((hext a.start).mp
        ⟨a, List.mem_cons_self .., LinearElem.le_refl a.start, a.h⟩)
    have hab : a.start ≤ b.start := Separated.start_le_mem h₁
      ((hext b.start).mpr
        ⟨b, List.mem_cons_self .., LinearElem.le_refl b.start, b.h⟩)
    have hs : a.start = b.start := by
      rcases LinearElem.trichotomy a.start b.start with hlt | heq | hgt
      · exact absurd hlt (LinearElem.not_lt_of_le hba)
      · exact heq
      · exact absurd hgt (LinearElem.not_lt_of_le hab)
    have he : a.«end» = b.«end» := by
      rcases LinearElem.trichotomy a.«end» b.«end» with hlt | heq | hgt
      · exact absurd ((hext a.«end»).mpr
          ⟨b, List.mem_cons_self .., hs ▸ LinearElem.le_of_lt a.h, hlt⟩)
          (Separated.end_not_mem h₁)
      · exact heq
      · exact absurd ((hext b.«end»).mp
          ⟨a, List.mem_cons_self .., hs.symm ▸ LinearElem.le_of_lt b.h,
            hgt⟩)
          (Separated.end_not_mem h₂)
    have hext' : ∀ x, x ∈ unionPoints r₁ ↔ x ∈ unionPoints r₂ := by
      intro x
      constructor
      · rintro ⟨jv, hjv, hxj⟩
        have hxj' : jv.start ≤ x ∧ x < jv.«end» := hxj
        have hax : a.«end» < x :=
          LinearElem.lt_of_lt_of_le (Separated.head_lt h₁ jv hjv) hxj'.1
        rcases mem_unionPoints_cons.mp
          ((hext x).mp ⟨jv, List.mem_cons_of_mem _ hjv, hxj⟩) with
            hxb | hxr
        · have hxb' : b.start ≤ x ∧ x < b.«end» := hxb
          exact absurd (he.symm ▸ hxb'.2) (LinearElem.lt_asymm hax)
        · exact hxr
      · rintro ⟨jv, hjv, hxj⟩
        have hxj' : jv.start ≤ x ∧ x < jv.«end» := hxj
        have hbx : b.«end» < x :=
          LinearElem.lt_of_lt_of_le (Separated.head_lt h₂ jv hjv) hxj'.1
        rcases mem_unionPoints_cons.mp
          ((hext x).mpr ⟨jv, List.mem_cons_of_mem _ hjv, hxj⟩) with
            hxa | hxr
        · have hxa' : a.start ≤ x ∧ x < a.«end» := hxa
          exact absurd (he ▸ hxa'.2) (LinearElem.lt_asymm hbx)
        · exact hxr
    rw [Interval.ext hs he,
      separated_eq_of_unionPoints (Separated.tail h₁) (Separated.tail h₂)
        hext']

/-- **The input-order theorem (`pack_input_order_irrelevant`).**
`pack` is a function of the point union alone: inputs carrying the
same union — permutations, duplications, and re-sorted tie orders
among them — pack IDENTICALLY. `pack_canonical` + `pack_extensional`
+ canonical-form uniqueness, composed. In particular the engine's
`sort_unstable` on lexicographic `[start, end]` pairs
(`finalize.rs::finalize_into`) and this module's start-only insertion
sort are indistinguishable through `pack` — the Level-1 transfer is
`Exec/Sweep.lean: sweepRuns_tie_order_irrelevant`. -/
theorem pack_input_order_irrelevant (l₁ l₂ : List (Interval α))
    (hext : ∀ x, x ∈ unionPoints l₁ ↔ x ∈ unionPoints l₂) :
    pack l₁ = pack l₂ :=
  separated_eq_of_unionPoints (pack_canonical l₁) (pack_canonical l₂)
    fun x => (pack_extensional l₁ x).trans
      ((hext x).trans (pack_extensional l₂ x).symm)

/-- **Theorem 7 (`pack_adjacency`).** Half-open adjacency CONTINUES a
run: `a.«end» = b.start` shares no point yet leaves no hole, so the
two claims coalesce into ONE segment — `[0,2), [2,5)` packs to
`[0,5)` (the kernel-evaluated example below). THE boundary the docs
kept explaining, now a lemma. Bridge: the sweep's one adjacency law —
"`start == frontier` continues a run" (`interval/sweep.rs`, its home
and nowhere else). -/
theorem pack_adjacency (a b : Interval α) (hadj : a.«end» = b.start) :
    pack [a, b] = [⟨a.start, b.«end»,
      LinearElem.lt_trans a.h (hadj.symm ▸ b.h)⟩] := by
  have hab : a.start ≤ b.start := hadj ▸ LinearElem.le_of_lt a.h
  have hsort : sortByStart [a, b] = [a, b] := by
    show insertByStart a [b] = [a, b]
    unfold insertByStart
    rw [if_pos hab]
  have hnogap : ¬ a.«end» < b.start := by
    intro hlt
    rw [← hadj] at hlt
    exact LinearElem.lt_irrefl _ hlt
  have hbe : a.«end» ≤ b.«end» := by
    rw [hadj]
    exact LinearElem.le_of_lt b.h
  have hmax : maxE a.«end» b.«end» = b.«end» := by
    unfold maxE
    rw [if_pos hbe]
  have hsingle : ∀ (x y : Interval α),
      x.start = y.start → x.«end» = y.«end» → [x] = [y] :=
    fun x y hs he => by rw [Interval.ext hs he]
  unfold pack
  rw [hsort]
  show coalesce a.start a.«end» a.h [b] = _
  unfold coalesce
  rw [if_neg hnogap]
  exact hsingle _ _ rfl hmax

/-! ### Lattice-closedness — the creation-quarantine fence -/

theorem coalesce_lattice_closed :
    ∀ (l : List (Interval α)) (s f : α) (h : s < f) (jv : Interval α),
      jv ∈ coalesce s f h l →
      (jv.start = s ∨ ∃ iv ∈ l, jv.start = iv.start) ∧
      (jv.«end» = f ∨ ∃ iv ∈ l, jv.«end» = iv.«end»)
  | [], s, f, h, jv, hjv => by
    rcases List.mem_singleton.mp hjv with rfl
    exact ⟨.inl rfl, .inl rfl⟩
  | iv :: rest, s, f, h, jv, hjv => by
    unfold coalesce at hjv
    by_cases hgap : f < iv.start
    · rw [if_pos hgap] at hjv
      rcases List.mem_cons.mp hjv with rfl | hmem
      · exact ⟨.inl rfl, .inl rfl⟩
      · obtain ⟨h1, h2⟩ :=
          coalesce_lattice_closed rest iv.start iv.«end» iv.h jv hmem
        constructor
        · rcases h1 with h1 | ⟨kv, hkv, h1⟩
          · exact .inr ⟨iv, List.mem_cons_self .., h1⟩
          · exact .inr ⟨kv, List.mem_cons_of_mem _ hkv, h1⟩
        · rcases h2 with h2 | ⟨kv, hkv, h2⟩
          · exact .inr ⟨iv, List.mem_cons_self .., h2⟩
          · exact .inr ⟨kv, List.mem_cons_of_mem _ hkv, h2⟩
    · rw [if_neg hgap] at hjv
      obtain ⟨h1, h2⟩ := coalesce_lattice_closed rest s (maxE f iv.«end»)
        (LinearElem.lt_of_lt_of_le h (le_maxE_left f iv.«end»)) jv hjv
      constructor
      · rcases h1 with h1 | ⟨kv, hkv, h1⟩
        · exact .inl h1
        · exact .inr ⟨kv, List.mem_cons_of_mem _ hkv, h1⟩
      · rcases h2 with h2 | ⟨kv, hkv, h2⟩
        · rcases maxE_eq_or f iv.«end» with hm | hm
          · exact .inl (h2.trans hm)
          · exact .inr ⟨iv, List.mem_cons_self .., h2.trans hm⟩
        · exact .inr ⟨kv, List.mem_cons_of_mem _ hkv, h2⟩

/-- **The lattice-closedness theorem — the creation-quarantine note,
made checkable.** Every packed segment's endpoints are SELECTED from
the stored claims' endpoints; `pack` never invents a bound. This is
the chain-window fence's premise (`20-query-ir.md` § engine
recursion, the chain-window fence) and the fence
for every future interval operator: only endpoint-selecting
operations are ever candidates. Bridge: the sweep emits `(run_start,
frontier)` with both words copied from input segments, never computed
(`interval/sweep.rs`); `20-query-ir.md`'s creation-quarantine
decision record cites this theorem. -/
theorem pack_lattice_closed {l : List (Interval α)} {jv : Interval α}
    (hjv : jv ∈ pack l) :
    (∃ iv ∈ l, jv.start = iv.start) ∧
      (∃ iv ∈ l, jv.«end» = iv.«end») := by
  unfold pack at hjv
  cases hs : sortByStart l with
  | nil =>
    rw [hs] at hjv
    nomatch hjv
  | cons iv rest =>
    rw [hs] at hjv
    obtain ⟨h1, h2⟩ :=
      coalesce_lattice_closed rest iv.start iv.«end» iv.h jv hjv
    have hmem : ∀ kv, kv ∈ iv :: rest → kv ∈ l := fun kv hkv =>
      mem_sortByStart.mp (by rw [hs]; exact hkv)
    constructor
    · rcases h1 with h1 | ⟨kv, hkv, h1⟩
      · exact ⟨iv, hmem iv (List.mem_cons_self ..), h1⟩
      · exact ⟨kv, hmem kv (List.mem_cons_of_mem _ hkv), h1⟩
    · rcases h2 with h2 | ⟨kv, hkv, h2⟩
      · exact ⟨iv, hmem iv (List.mem_cons_self ..), h2⟩
      · exact ⟨kv, hmem kv (List.mem_cons_of_mem _ hkv), h2⟩

end Pack

/-! ### Pack, kernel-evaluated (the PRD's two example evaluations) -/

/-- A `U64` interval literal — example material. -/
private def u64Iv (s e : Nat) (hs : s < 2 ^ 64 := by omega)
    (he : e < 2 ^ 64 := by omega) (hlt : s < e := by omega) :
    Interval U64 := ⟨⟨s, hs⟩, ⟨e, he⟩, hlt⟩

/-- Adjacency coalesces: `[0,2), [2,5)` packs to `[0,5)` — the
`pack_adjacency` boundary, evaluated. -/
example : pack [u64Iv 0 2, u64Iv 2 5] = [u64Iv 0 5] := by decide

/-- Sorting, containment, and a real gap: `[7,9), [0,4), [1,3)` packs
to `[0,4), [7,9)` — the contained claim vanishes, the gap survives. -/
example : pack [u64Iv 7 9, u64Iv 0 4, u64Iv 1 3] =
    [u64Iv 0 4, u64Iv 7 9] := by decide

namespace Query

/-! ## Allen — the 13-relation classifier, DEFINED -/

section Allen

variable {α : Type} [LT α] [LE α] [LinearElem α] [DecidableLT α]
  [DecidableEq α]

/-- Each basic relation's endpoint-comparison definition over
nonempty half-open intervals — the SEMANTIC side `classifyI` is
measured against (`allen.rs`'s per-variant doc comments; the
point-set oracle's endpoint form). -/
def AllenRel.holds : AllenRel → Interval α → Interval α → Prop
  | .before, a, b => a.«end» < b.start
  | .meets, a, b => a.«end» = b.start
  | .overlaps, a, b =>
    a.start < b.start ∧ b.start < a.«end» ∧ a.«end» < b.«end»
  | .starts, a, b => a.start = b.start ∧ a.«end» < b.«end»
  | .during, a, b => b.start < a.start ∧ a.«end» < b.«end»
  | .finishes, a, b => b.start < a.start ∧ a.«end» = b.«end»
  | .equals, a, b => a.start = b.start ∧ a.«end» = b.«end»
  | .finishedBy, a, b => a.start < b.start ∧ a.«end» = b.«end»
  | .contains, a, b => a.start < b.start ∧ b.«end» < a.«end»
  | .startedBy, a, b => a.start = b.start ∧ b.«end» < a.«end»
  | .overlappedBy, a, b =>
    b.start < a.start ∧ a.start < b.«end» ∧ b.«end» < a.«end»
  | .metBy, a, b => b.«end» = a.start
  | .after, a, b => b.«end» < a.start

/-- The three-way endpoint comparison the classifier is written in
(`Ord::cmp` on the Rust side). -/
def cmp3 (x y : α) : Ordering :=
  if x < y then .lt else if x = y then .eq else .gt

omit [LE α] [LinearElem α] in
theorem cmp3_lt {x y : α} : cmp3 x y = .lt ↔ x < y := by
  unfold cmp3
  by_cases h1 : x < y
  · rw [if_pos h1]
    exact iff_of_true rfl h1
  · rw [if_neg h1]
    by_cases h2 : x = y
    · rw [if_pos h2]
      exact iff_of_false nofun h1
    · rw [if_neg h2]
      exact iff_of_false nofun h1

theorem cmp3_eq {x y : α} : cmp3 x y = .eq ↔ x = y := by
  unfold cmp3
  by_cases h1 : x < y
  · rw [if_pos h1]
    exact iff_of_false nofun
      (fun heq => LinearElem.lt_irrefl y (heq ▸ h1))
  · rw [if_neg h1]
    by_cases h2 : x = y
    · rw [if_pos h2]
      exact iff_of_true rfl h2
    · rw [if_neg h2]
      exact iff_of_false nofun h2

theorem cmp3_gt {x y : α} : cmp3 x y = .gt ↔ y < x := by
  unfold cmp3
  by_cases h1 : x < y
  · rw [if_pos h1]
    exact iff_of_false nofun (fun hgt => LinearElem.lt_asymm h1 hgt)
  · rw [if_neg h1]
    by_cases h2 : x = y
    · rw [if_pos h2]
      exact iff_of_false nofun
        (fun hgt => LinearElem.lt_irrefl y (h2 ▸ hgt))
    · rw [if_neg h2]
      refine iff_of_true rfl ?_
      rcases LinearElem.trichotomy x y with h | h | h
      · exact absurd h h1
      · exact absurd h h2
      · exact h

/-- **The classifier, DEFINED** — the endpoint-comparison decision
tree, matching `allen.rs::classify_bounds` case for case: the 3 × 3
grid on `(cmp start, cmp end)`, with `(lt,lt)`/`(gt,gt)` refined by
the cross comparison. Total over the in-tree nonempty `Interval` — no
empty cases exist — and computable (the examples below evaluate it).
Refines PRD 04's abstract `Classify` (`classifyRefined`). -/
def classifyI (a b : Interval α) : AllenRel :=
  match cmp3 a.start b.start, cmp3 a.«end» b.«end» with
  | .eq, .eq => .equals
  | .eq, .lt => .starts
  | .eq, .gt => .startedBy
  | .lt, .eq => .finishedBy
  | .gt, .eq => .finishes
  | .gt, .lt => .during
  | .lt, .gt => .contains
  | .lt, .lt =>
    match cmp3 a.«end» b.start with
    | .lt => .before
    | .eq => .meets
    | .gt => .overlaps
  | .gt, .gt =>
    match cmp3 b.«end» a.start with
    | .lt => .after
    | .eq => .metBy
    | .gt => .overlappedBy

/-- The classified relation HOLDS — the "jointly exhaustive" half of
JEPD, as the classifier's soundness. -/
theorem classify_holds (a b : Interval α) :
    (classifyI a b).holds a b := by
  unfold classifyI
  cases h1 : cmp3 a.start b.start with
  | lt =>
    cases h2 : cmp3 a.«end» b.«end» with
    | lt =>
      cases h3 : cmp3 a.«end» b.start with
      | lt => exact cmp3_lt.mp h3
      | eq => exact cmp3_eq.mp h3
      | gt => exact ⟨cmp3_lt.mp h1, cmp3_gt.mp h3, cmp3_lt.mp h2⟩
    | eq => exact ⟨cmp3_lt.mp h1, cmp3_eq.mp h2⟩
    | gt => exact ⟨cmp3_lt.mp h1, cmp3_gt.mp h2⟩
  | eq =>
    cases h2 : cmp3 a.«end» b.«end» with
    | lt => exact ⟨cmp3_eq.mp h1, cmp3_lt.mp h2⟩
    | eq => exact ⟨cmp3_eq.mp h1, cmp3_eq.mp h2⟩
    | gt => exact ⟨cmp3_eq.mp h1, cmp3_gt.mp h2⟩
  | gt =>
    cases h2 : cmp3 a.«end» b.«end» with
    | lt => exact ⟨cmp3_gt.mp h1, cmp3_lt.mp h2⟩
    | eq => exact ⟨cmp3_gt.mp h1, cmp3_eq.mp h2⟩
    | gt =>
      cases h3 : cmp3 b.«end» a.start with
      | lt => exact cmp3_lt.mp h3
      | eq => exact cmp3_eq.mp h3
      | gt => exact ⟨cmp3_gt.mp h1, cmp3_gt.mp h3, cmp3_gt.mp h2⟩

/-- A holding relation IS the classification — the "pairwise
disjoint" half of JEPD, as the classifier's completeness. Each case
derives the full endpoint-comparison signature from the relation's
definition plus the two nonemptiness invariants. -/
theorem holds_classify {a b : Interval α} {rel : AllenRel}
    (h : rel.holds a b) : classifyI a b = rel := by
  cases rel with
  | before =>
    have h' : a.«end» < b.start := h
    unfold classifyI
    rw [cmp3_lt.mpr (LinearElem.lt_trans a.h h'),
      cmp3_lt.mpr (LinearElem.lt_trans h' b.h), cmp3_lt.mpr h']
  | meets =>
    have h' : a.«end» = b.start := h
    unfold classifyI
    rw [cmp3_lt.mpr (h' ▸ a.h), cmp3_lt.mpr (h'.symm ▸ b.h),
      cmp3_eq.mpr h']
  | overlaps =>
    obtain ⟨h1, h2, h3⟩ := h
    unfold classifyI
    rw [cmp3_lt.mpr h1, cmp3_lt.mpr h3, cmp3_gt.mpr h2]
  | starts =>
    obtain ⟨h1, h2⟩ := h
    unfold classifyI
    rw [cmp3_eq.mpr h1, cmp3_lt.mpr h2]
  | during =>
    obtain ⟨h1, h2⟩ := h
    unfold classifyI
    rw [cmp3_gt.mpr h1, cmp3_lt.mpr h2]
  | finishes =>
    obtain ⟨h1, h2⟩ := h
    unfold classifyI
    rw [cmp3_gt.mpr h1, cmp3_eq.mpr h2]
  | equals =>
    obtain ⟨h1, h2⟩ := h
    unfold classifyI
    rw [cmp3_eq.mpr h1, cmp3_eq.mpr h2]
  | finishedBy =>
    obtain ⟨h1, h2⟩ := h
    unfold classifyI
    rw [cmp3_lt.mpr h1, cmp3_eq.mpr h2]
  | contains =>
    obtain ⟨h1, h2⟩ := h
    unfold classifyI
    rw [cmp3_lt.mpr h1, cmp3_gt.mpr h2]
  | startedBy =>
    obtain ⟨h1, h2⟩ := h
    unfold classifyI
    rw [cmp3_eq.mpr h1, cmp3_gt.mpr h2]
  | overlappedBy =>
    obtain ⟨h1, h2, h3⟩ := h
    unfold classifyI
    rw [cmp3_gt.mpr h1, cmp3_gt.mpr h3, cmp3_gt.mpr h2]
  | metBy =>
    have h' : b.«end» = a.start := h
    unfold classifyI
    rw [cmp3_gt.mpr (h' ▸ b.h), cmp3_gt.mpr (h'.symm ▸ a.h),
      cmp3_eq.mpr h']
  | after =>
    have h' : b.«end» < a.start := h
    unfold classifyI
    rw [cmp3_gt.mpr (LinearElem.lt_trans b.h h'),
      cmp3_gt.mpr (LinearElem.lt_trans h' a.h), cmp3_lt.mpr h']

/-- **Theorem 8 (`allen_jepd`).** The 13 basic relations are jointly
exhaustive and pairwise disjoint over nonempty half-open intervals: a
relation holds IFF it is the classification, so every pair satisfies
EXACTLY one basic. Proved generally over any `LinearElem` domain (the
spec's two-concrete-domain fallback went unspent). Bridge:
`allen.rs::classify` ("JEPD is a theorem of the match shape") — the
point-set-oracle property test and the 8192-mask exhaustive suite
sample this theorem. -/
theorem allen_jepd (a b : Interval α) (rel : AllenRel) :
    rel.holds a b ↔ classifyI a b = rel :=
  ⟨holds_classify, fun h => h ▸ classify_holds a b⟩

/-- JE alone: some basic always holds. -/
theorem allen_exhaustive (a b : Interval α) :
    ∃ rel : AllenRel, rel.holds a b :=
  ⟨classifyI a b, classify_holds a b⟩

/-- PD alone: at most one basic holds. -/
theorem allen_disjoint {a b : Interval α} {r₁ r₂ : AllenRel}
    (h₁ : r₁.holds a b) (h₂ : r₂.holds a b) : r₁ = r₂ :=
  (holds_classify h₁).symm.trans (holds_classify h₂)

/-- **`DISJOINT` is the point statement.** Two nonempty half-open
intervals share no point exactly when their classification lands in
the `DISJOINT` composite — before ∪ meets ∪ met-by ∪ after,
`INTERSECTS`' complement (`docs/architecture/20-query-ir.md` § the
Allen operator's named constants). This is the vocabulary tie the
pointwise key judgment cites: per-group pairwise disjointness
(`Dependencies.pointwise_key_disjoint`) and the query surface's
`DISJOINT` mask are one statement — one vocabulary, both sides of
the engine, as a theorem. -/
theorem points_disjoint_iff_disjoint_mask (a b : Interval α) :
    (∀ x, ¬(x ∈ a.points ∧ x ∈ b.points)) ↔
      classifyI a b ∈
        ([.before, .meets, .metBy, .after] : AllenMask) := by
  constructor
  · intro h
    cases hcl : classifyI a b with
    | before => exact .head _
    | meets => exact .tail _ (.head _)
    | metBy => exact .tail _ (.tail _ (.head _))
    | after => exact .tail _ (.tail _ (.tail _ (.head _)))
    | overlaps =>
      have hh := classify_holds a b
      rw [hcl] at hh
      obtain ⟨h1, h2, h3⟩ := hh
      exact absurd ⟨⟨LinearElem.le_of_lt h1, h2⟩,
        ⟨LinearElem.le_refl b.start, b.h⟩⟩ (h b.start)
    | starts =>
      have hh := classify_holds a b
      rw [hcl] at hh
      obtain ⟨h1, h2⟩ := hh
      refine absurd ⟨⟨LinearElem.le_refl a.start, a.h⟩,
        ⟨?_, LinearElem.lt_trans a.h h2⟩⟩ (h a.start)
      rw [← h1]
      exact LinearElem.le_refl a.start
    | during =>
      have hh := classify_holds a b
      rw [hcl] at hh
      obtain ⟨h1, h2⟩ := hh
      exact absurd ⟨⟨LinearElem.le_refl a.start, a.h⟩,
        ⟨LinearElem.le_of_lt h1, LinearElem.lt_trans a.h h2⟩⟩
        (h a.start)
    | finishes =>
      have hh := classify_holds a b
      rw [hcl] at hh
      obtain ⟨h1, h2⟩ := hh
      refine absurd ⟨⟨LinearElem.le_refl a.start, a.h⟩,
        ⟨LinearElem.le_of_lt h1, ?_⟩⟩ (h a.start)
      rw [← h2]
      exact a.h
    | equals =>
      have hh := classify_holds a b
      rw [hcl] at hh
      obtain ⟨h1, h2⟩ := hh
      refine absurd ⟨⟨LinearElem.le_refl a.start, a.h⟩,
        ⟨?_, ?_⟩⟩ (h a.start)
      · rw [← h1]
        exact LinearElem.le_refl a.start
      · rw [← h2]
        exact a.h
    | finishedBy =>
      have hh := classify_holds a b
      rw [hcl] at hh
      obtain ⟨h1, h2⟩ := hh
      refine absurd ⟨⟨LinearElem.le_of_lt h1, ?_⟩,
        ⟨LinearElem.le_refl b.start, b.h⟩⟩ (h b.start)
      rw [h2]
      exact b.h
    | contains =>
      have hh := classify_holds a b
      rw [hcl] at hh
      obtain ⟨h1, h2⟩ := hh
      exact absurd ⟨⟨LinearElem.le_of_lt h1, LinearElem.lt_trans b.h h2⟩,
        ⟨LinearElem.le_refl b.start, b.h⟩⟩ (h b.start)
    | startedBy =>
      have hh := classify_holds a b
      rw [hcl] at hh
      obtain ⟨h1, h2⟩ := hh
      refine absurd ⟨⟨?_, LinearElem.lt_trans b.h h2⟩,
        ⟨LinearElem.le_refl b.start, b.h⟩⟩ (h b.start)
      rw [h1]
      exact LinearElem.le_refl b.start
    | overlappedBy =>
      have hh := classify_holds a b
      rw [hcl] at hh
      obtain ⟨h1, h2, h3⟩ := hh
      exact absurd ⟨⟨LinearElem.le_refl a.start, a.h⟩,
        ⟨LinearElem.le_of_lt h1, h2⟩⟩ (h a.start)
  · intro hmem x hx
    have hxa : a.start ≤ x ∧ x < a.«end» := hx.1
    have hxb : b.start ≤ x ∧ x < b.«end» := hx.2
    have hh := classify_holds a b
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hmem
    rcases hmem with hcl | hcl | hcl | hcl <;> rw [hcl] at hh
    · exact LinearElem.lt_irrefl x
        (LinearElem.lt_of_lt_of_le (LinearElem.lt_trans hxa.2 hh) hxb.1)
    · refine LinearElem.lt_irrefl x (LinearElem.lt_of_lt_of_le ?_ hxb.1)
      show x < b.start
      rw [← hh]
      exact hxa.2
    · refine LinearElem.lt_irrefl x (LinearElem.lt_of_lt_of_le ?_ hxa.1)
      show x < a.start
      rw [← hh]
      exact hxb.2
    · exact LinearElem.lt_irrefl x
        (LinearElem.lt_of_lt_of_le (LinearElem.lt_trans hxb.2 hh) hxa.1)

end Allen

/-! ## The converse — the mask algebra's involution -/

/-- The converse basic (`allen.rs::Basic::converse` — the mirrored
bit position `12 − i`; here the table itself, the bit order being the
encoding's business). Hazard recorded: the `AllenRel` CONSTRUCTOR
order here is NOT the encoding's bit order — `allen.rs` places
`Starts=3 … FinishedBy=7 … StartedBy=9` where this sum's positions
3↔5 and 7↔9 differ. Both orders are palindromic, so converse is a
mirror in both, and the modeled mask is name-keyed (a `List AllenRel`
read by membership) — no theorem reads positions; equating
constructor index with bit index is the one misuse this note
forecloses. -/
def AllenRel.converse : AllenRel → AllenRel
  | .before => .after
  | .meets => .metBy
  | .overlaps => .overlappedBy
  | .starts => .startedBy
  | .during => .contains
  | .finishes => .finishedBy
  | .equals => .equals
  | .finishedBy => .finishes
  | .contains => .during
  | .startedBy => .starts
  | .overlappedBy => .overlaps
  | .metBy => .meets
  | .after => .before

/-- A mask's converse: pointwise — the 13-bit reversal's abstract
form (`allen.rs::AllenMask::converse`). -/
def AllenMask.converse (m : AllenMask) : AllenMask :=
  m.map AllenRel.converse

/-- **Theorem 9 (`allen_converse_involution`).** `converse ∘ converse
= id` on the basics. Bridge: `allen.rs` — the palindromic bit order
makes it one bit-reversal; the exhaustive 8192-mask involution test
samples the mask corollary (`mask_converse_involution`). -/
theorem allen_converse_involution (rel : AllenRel) :
    rel.converse.converse = rel := by
  cases rel <;> rfl

/-- The involution lifts to masks pointwise. -/
theorem mask_converse_involution (m : AllenMask) :
    m.converse.converse = m := by
  unfold AllenMask.converse
  rw [List.map_map]
  have h : m.map (AllenRel.converse ∘ AllenRel.converse) = m.map id :=
    List.map_congr_left fun rel _ => allen_converse_involution rel
  rw [h, List.map_id]

/-- Mask converse agrees with basic converse membership-wise — the
mask-law half of `allen.rs`'s converse test. -/
theorem mem_mask_converse {m : AllenMask} {rel : AllenRel} :
    rel.converse ∈ m.converse ↔ rel ∈ m := by
  unfold AllenMask.converse
  constructor
  · intro h
    obtain ⟨r, hr, heq⟩ := List.mem_map.mp h
    have hrrel : r = rel := by
      have h2 := congrArg AllenRel.converse heq
      rwa [allen_converse_involution, allen_converse_involution] at h2
    exact hrrel ▸ hr
  · intro h
    exact List.mem_map.mpr ⟨rel, h, rfl⟩

section AllenSwap

variable {α : Type} [LT α] [LE α] [LinearElem α] [DecidableLT α]
  [DecidableEq α]

omit [LE α] [LinearElem α] [DecidableLT α] [DecidableEq α] in
/-- Swapping the operands converses the relation: each basic's
endpoint definition is literally its converse's, read right to
left. -/
theorem holds_converse {a b : Interval α} {rel : AllenRel}
    (h : rel.holds a b) : rel.converse.holds b a := by
  cases rel with
  | before => exact h
  | meets => exact h
  | overlaps => exact h
  | starts => exact ⟨h.1.symm, h.2⟩
  | during => exact h
  | finishes => exact ⟨h.1, h.2.symm⟩
  | equals => exact ⟨h.1.symm, h.2.symm⟩
  | finishedBy => exact ⟨h.1, h.2.symm⟩
  | contains => exact h
  | startedBy => exact ⟨h.1.symm, h.2⟩
  | overlappedBy => exact h
  | metBy => exact h
  | after => exact h

/-- **Theorem 9 (companion).** Classification dualizes under operand
swap: `classify b a = (classify a b)⁻¹`. Bridge: `allen.rs`'s
`converse_is_an_involution_and_dualizes_classification`. -/
theorem classify_swap (a b : Interval α) :
    classifyI b a = (classifyI a b).converse :=
  holds_classify (holds_converse (classify_holds a b))

/-- The mask-level swap law: `Allen(a, b, m) ≡ Allen(b, a, m⁻¹)` —
what makes the executor free to orient its Allen filters. -/
theorem allen_swap_mask (m : AllenMask) (a b : Interval α) :
    classifyI a b ∈ m ↔ classifyI b a ∈ m.converse := by
  rw [classify_swap a b]
  exact mem_mask_converse.symm

end AllenSwap

/-- The REFINEMENT of PRD 04's abstract `Classify` parameter: the
defined classifier at both element domains. Every PRD 04 theorem
quantified over `Classify` holds for the real classifier by
instantiation — exactly why PRD 04 kept it opaque. -/
def classifyRefined : Classify where
  u64 := classifyI
  i64 := classifyI

/-- Interval value equality IS `Allen(EQUALS)` under the refinement —
the provable equality PRD 04's `cmpDen` doc promised (the engine
canonicalizes interval `Eq` to `EQUALS` in normalization). -/
theorem classify_equals_iff {α : Type} [LT α] [LE α] [LinearElem α]
    [DecidableLT α] [DecidableEq α] (a b : Interval α) :
    classifyI a b = .equals ↔ a = b := by
  constructor
  · intro h
    have h' := (allen_jepd a b .equals).mpr h
    exact Interval.ext h'.1 h'.2
  · rintro rfl
    exact holds_classify ⟨rfl, rfl⟩

/-! ### Classify, kernel-evaluated (the PRD's two example evaluations) -/

/-- `[0,2)` meets `[2,5)`: half-open adjacency shares no point. -/
example : classifyI (u64Iv 0 2) (u64Iv 2 5) = .meets := by decide

/-- `[3,7)` during `[0,10)`, and the swap classifies as the
converse. -/
example : classifyI (u64Iv 3 7) (u64Iv 0 10) = .during ∧
    classifyI (u64Iv 0 10) (u64Iv 3 7) = .contains := by decide

/-! ## Grouping — the fibering of the distinct binding set

The decision, recorded (the F4 alignment): **answers are value
tuples, and grouping is fibering over head VALUES** — the group key
of a binding is the EVALUATED value of each non-aggregate find term,
not the binding's raw variable valuation. A plain `var` position
contributes its value (unchanged from the variable-keyed reading); a
`measure` position contributes its evaluated `U64` measure — so two
bindings over DISTINCT intervals with colliding measures are ONE
group (`equal_key_values_share_fiber`), exactly as both
implementations behave (the engine keys its group map on the sink's
find columns — `groups.rs::probe_group` over the projected words —
and the naive model keys on the projected measure value,
`naive/query.rs::project`; the conformance model's `keyOf` is the
same evaluation). The superseded variable-keyed reading split those
bindings into two fibers — a spec-only semantics no implementation
ever had. -/

/-- The distinct binding set a rule denotes — PRD 04's deriving
assignments, PRE-projection (the fold domain's carrier). A `Set`:
binding multiplicity is unrepresentable, which is "set semantics
through aggregation" at the representation level. -/
def bindingSet (C : Classify) (r : Rule) (I : Instance) (ρ : ParamEnv) :
    Set Assignment :=
  fun σ => derives C r I ρ σ

/-- A group-key head position: the non-aggregate faces of the
head-shape row (`crate::ir::HeadTerm`'s `Var` and `Measure`; the
conformance model's `CFind.var`/`CFind.measure`). Grouping keys on
what these positions PROJECT, never on the raw valuation. -/
inductive KeyTerm where
  | var (v : VarId)
  | measure (v : VarId)
deriving DecidableEq

/-- The value a key position projects under a binding: a plain
variable its value, a measure position its evaluated `U64` measure.
`none` is the ray — the MeasureOfRay narrowing again (the engine
raises the typed error; this level carries no effect), unreachable
on error-free executions. -/
def KeyTerm.value? (σ : Assignment) : KeyTerm → Option Value
  | .var v => some (σ v)
  | .measure v => (σ v).measure?

/-- The evaluated group key of a binding: the projected values of the
key positions, in head order — the value tuple the answer row
carries. -/
def keyTuple (keys : List KeyTerm) (σ : Assignment) :
    List (Option Value) :=
  keys.map (KeyTerm.value? σ)

/-- One group: the FIBER of the binding set over an evaluated
group-key tuple — grouping IS fibering over head values
(`20-query-ir.md` § aggregation: group key = the projected values of
the non-aggregated find terms). -/
def Group (C : Classify) (r : Rule) (I : Instance) (ρ : ParamEnv)
    (keys : List KeyTerm) (g : List (Option Value)) : Set Assignment :=
  fun σ => derives C r I ρ σ ∧ keyTuple keys σ = g

/-- Fibers are disjoint: a binding lives in exactly one group. -/
theorem group_fibers_disjoint {C : Classify} {r : Rule} {I : Instance}
    {ρ : ParamEnv} {keys : List KeyTerm} {g g' : List (Option Value)}
    {σ : Assignment} (h : σ ∈ Group C r I ρ keys g)
    (h' : σ ∈ Group C r I ρ keys g') : g = g' :=
  h.2.symm.trans h'.2

/-- Fibers exhaust: every deriving binding lands in its evaluated
key's group. -/
theorem group_fibers_exhaust {C : Classify} {r : Rule} {I : Instance}
    {ρ : ParamEnv} (keys : List KeyTerm) {σ : Assignment}
    (h : derives C r I ρ σ) :
    σ ∈ Group C r I ρ keys (keyTuple keys σ) :=
  ⟨h, rfl⟩

/-- **The F4 content, stated positively.** Two deriving bindings with
equal EVALUATED key tuples share one fiber — in particular two
bindings over distinct intervals whose measures collide MERGE under a
measure key, as both implementations group. Under the superseded
variable-keyed fibering they split. Bridge:
`groups.rs::probe_group` (group creation keys the projected find
columns); `naive/query.rs::project` (the key pushes
`measure_value(...)`, not the interval). -/
theorem equal_key_values_share_fiber {C : Classify} {r : Rule}
    {I : Instance} {ρ : ParamEnv} {keys : List KeyTerm}
    {σ σ' : Assignment} (h : derives C r I ρ σ)
    (h' : derives C r I ρ σ')
    (hkey : keyTuple keys σ = keyTuple keys σ') :
    σ ∈ Group C r I ρ keys (keyTuple keys σ) ∧
      σ' ∈ Group C r I ρ keys (keyTuple keys σ) :=
  ⟨⟨h, rfl⟩, ⟨h', hkey.symm⟩⟩

/-- On a plain-variable key list the evaluated tuple IS the valuation
tuple (each entry `some (σ v)`) — the restatement changed nothing for
var keys; only measure positions gained their value reading. -/
theorem keyTuple_vars (vs : List VarId) (σ : Assignment) :
    keyTuple (vs.map .var) σ = vs.map (fun v => some (σ v)) := by
  unfold keyTuple
  rw [List.map_map]
  rfl

/-! ## The fold domain is distinct — set semantics through
aggregation -/

/-- Duplicate elimination over any listing (keeps the last
occurrence; only membership matters — `mem_dedup`). -/
def dedup {β : Type} [DecidableEq β] : List β → List β
  | [] => []
  | x :: xs => if x ∈ xs then dedup xs else x :: dedup xs

theorem mem_dedup {β : Type} [DecidableEq β] {x : β} :
    ∀ {l : List β}, x ∈ dedup l ↔ x ∈ l
  | [] => Iff.rfl
  | y :: ys => by
    unfold dedup
    by_cases hmem : y ∈ ys
    · rw [if_pos hmem, mem_dedup (l := ys), List.mem_cons]
      constructor
      · exact .inr
      · rintro (rfl | h)
        · exact hmem
        · exact h
    · rw [if_neg hmem]
      simp only [List.mem_cons, mem_dedup (l := ys)]

/-- The dedup really is distinct: no element twice. -/
theorem dedup_nodup {β : Type} [DecidableEq β] :
    ∀ (l : List β), (dedup l).Nodup
  | [] => List.Pairwise.nil
  | x :: xs => by
    unfold dedup
    by_cases hmem : x ∈ xs
    · rw [if_pos hmem]
      exact dedup_nodup xs
    · rw [if_neg hmem]
      exact List.pairwise_cons.mpr
        ⟨(fun y hy heq =>
          hmem (by rw [heq]; exact mem_dedup.mp hy)), dedup_nodup xs⟩

/-- **Theorem 1 (`agg_over_distinct_bindings`).** Every aggregate
folds the DISTINCT binding set of its group: the fold domain is
dedup-invariant under duplicated input, UNIFORMLY in the fold — no op
can observe a duplicate, which is set semantics through aggregation
("two postings of amount 100 are two distinct bindings; the same
posting twice is one"). Bridge: the binding seen-set (`fold_row.rs`:
single-rule programs key the whole slot array, the union regime keys
the head projection) and its elision licence — `DistinctWitness`,
whose proof is PRD 07's; `CountDistinct`'s value set dedups beneath
it (distinct bindings ⊇ distinct values). -/
theorem agg_over_distinct_bindings {β γ : Type} [DecidableEq β]
    (fold : List β → γ) {x : β} {l : List β} (hx : x ∈ l) :
    fold (dedup (x :: l)) = fold (dedup l) := by
  have h : dedup (x :: l) = dedup l := by
    show (if x ∈ l then dedup l else x :: dedup l) = dedup l
    rw [if_pos hx]
  rw [h]

/-! ## Aggregate answers — one row per inhabited fiber -/

/-- The aggregate answer set, fold-abstract: one row per INHABITED
group fiber — the row is the fold of the evaluated key tuple and the
group (key columns + accumulator finalization, abstracted as `fold`).
The witness `σ` is the load-bearing shape: a group exists only as the
fiber of an ACTUAL deriving binding (`groups.rs::probe_group` — a
group is created on first sight), which is exactly what refuses SQL's
zero row. The key handed to the fold is `keyTuple` — head VALUES, the
F4 decision (module doc). -/
def aggAnswers (C : Classify) (r : Rule) (I : Instance) (ρ : ParamEnv)
    (keys : List KeyTerm)
    (fold : List (Option Value) → Set Assignment → AnswerTuple) :
    Set AnswerTuple :=
  fun t => ∃ σ, derives C r I ρ σ ∧
    t = fold (keyTuple keys σ) (Group C r I ρ keys (keyTuple keys σ))

/-- **Theorem 2 (`empty_global_no_answer`).** An aggregate over the
empty binding set yields the EMPTY answer set — stated for every
group-key list; the global aggregate (empty key) is the case the
docs shout about: not a zero row, not a NULL row ("the balance of an
account with no postings is an absent answer, not 0"). THE artifact
divergence (module doc): the seed artifact's `sum [] = 0` /
`count [] = 0` reading is refused — the engine is the authority.
Bridge: `finalize.rs::finalize_into` — "Empty input yields zero
rows"; the refused reading's countermodel is
`Countermodels.sql_zero_row_from_no_binding`. -/
theorem empty_global_no_answer {C : Classify} {r : Rule}
    {I : Instance} {ρ : ParamEnv} {keys : List KeyTerm}
    {fold : List (Option Value) → Set Assignment → AnswerTuple}
    (hempty : ∀ σ, ¬ derives C r I ρ σ) :
    ∀ t, t ∉ aggAnswers C r I ρ keys fold := by
  rintro t ⟨σ, hσ, -⟩
  exact hempty σ hσ

/-! ## The measure folds — Option-poisoning on rays -/

/-- The measure column of a group listing: `none` the moment ANY
binding's interval is a ray — `Option`-poisoning at the group level
(the MeasureOfRay spec; the module doc's recorded narrowing). -/
def measureColumn (v : VarId) : List Assignment → Option (List Value)
  | [] => some []
  | σ :: σs =>
    match (σ v).measure?, measureColumn v σs with
    | some m, some ms => some (m :: ms)
    | _, _ => none

/-- **Theorem 4 (`measure_fold_laws`).** The measure column is
poisoned EXACTLY by a ray in the group: one unbounded interval makes
the whole group's measure erroneous, never a value — so Sum/Min/Max
over `measure v` (ANY fold over the column) inherit 02's ray refusal
(`measure_fold_poisons` composes it; `Value.measure?` reads `none`
exactly on rays and non-intervals — `measure_ray_none`,
`measure?_ray_none`). Bridge: `fold_row.rs::fold_scratch_row` — "a
ray poisons the sink and the row is dropped"; the engine's answer is
the typed `crate::Error::MeasureOfRay`. -/
theorem measure_fold_laws (v : VarId) :
    ∀ σs : List Assignment,
      measureColumn v σs = none ↔ ∃ σ ∈ σs, (σ v).measure? = none
  | [] => iff_of_false nofun (fun ⟨_, hσ, _⟩ => nomatch hσ)
  | σ :: σs => by
    unfold measureColumn
    cases hm : (σ v).measure? with
    | none =>
      exact iff_of_true rfl ⟨σ, List.mem_cons_self .., hm⟩
    | some m =>
      cases hc : measureColumn v σs with
      | none =>
        refine iff_of_true rfl ?_
        obtain ⟨σ', hσ', hm'⟩ := (measure_fold_laws v σs).mp hc
        exact ⟨σ', List.mem_cons_of_mem _ hσ', hm'⟩
      | some ms =>
        refine iff_of_false nofun ?_
        rintro ⟨σ', hσ', hm'⟩
        rcases List.mem_cons.mp hσ' with rfl | hmem
        · rw [hm] at hm'
          cases hm'
        · have hnone : measureColumn v σs = none :=
            (measure_fold_laws v σs).mpr ⟨σ', hmem, hm'⟩
          rw [hc] at hnone
          cases hnone

/-- Any fold over a poisoned column is poisoned — the group-level
error, composed: the erroneous group never becomes a value. -/
theorem measure_fold_poisons {v : VarId} {σs : List Assignment}
    {σ : Assignment} (hσ : σ ∈ σs) (hray : (σ v).measure? = none)
    (fold : List Value → Value) :
    (measureColumn v σs).map fold = none := by
  rw [(measure_fold_laws v σs).mpr ⟨σ, hσ, hray⟩]
  rfl

/-- The happy half: a ray-free group has its full measure column —
the fold domain exists and is exactly the pointwise measures. -/
theorem measureColumn_total (v : VarId) :
    ∀ σs : List Assignment,
      (∀ σ ∈ σs, ∃ m, (σ v).measure? = some m) →
      ∃ ms, measureColumn v σs = some ms
  | [], _ => ⟨[], rfl⟩
  | σ :: σs, h => by
    obtain ⟨m, hm⟩ := h σ (List.mem_cons_self ..)
    obtain ⟨ms, hms⟩ := measureColumn_total v σs
      (fun σ' hσ' => h σ' (List.mem_cons_of_mem _ hσ'))
    exact ⟨m :: ms, by unfold measureColumn; rw [hm, hms]⟩

/-- The value-level ray law lifted through `Value.measure?` (02's
`measure_ray_none`) — the poison's source, `u64` face. -/
theorem measure?_ray_none {iv : Interval U64} (hray : iv.isRay) :
    Value.measure? ⟨.interval .u64, iv⟩ = none := by
  show iv.measure.bind measureOfNat = none
  rw [measure_ray_none iv hray]
  rfl

/-- The `i64` face. -/
theorem measure?_ray_none_i64 {iv : Interval I64} (hray : iv.isRay) :
    Value.measure? ⟨.interval .i64, iv⟩ = none := by
  show iv.measure.bind measureOfNat = none
  rw [measure_ray_none iv hray]
  rfl

/-! ## The ray verdict — the Kleene three-valued fold (R6)

Error propagation through condition trees is STRONG KLEENE
three-valued logic (ruled 2026-07-23, R6): one condition evaluation
renders one of three verdicts — `holds`, `fails`, or `ray`, the third
being the measure-of-a-ray refusal (`Value.measure?` reads `none`
exactly on rays — `measure?_ray_none`) — and `and`/`or` nodes fold
verdicts in the Kleene lattice. Both connectives are commutative and
associative (`Verdict3.and_comm` and kin), and conjunction distributes
over disjunction (`Verdict3.and_or_distrib`), so a tree's verdict is a
function of its leaf multiset and agrees with DNF lowering by
construction — evaluation order is UNOBSERVABLE, which is what makes
an error semantics well-defined over an IR whose condition lists
compare as sets (`ruleAnswers_conditions_congr` is the
order-blindness the verdict must respect; a reach-dependent poison
flag contradicts it). A binding raises iff its folded verdict is
`ray`; `orFold_eq_ray` is the disjunct-set reading — raise iff some
disjunct is poisoned and none holds — and `andFold_eq_ray` its
conjunctive face. The effect-free boolean model (PRD 04's recorded
narrowing: a ray's measure comparison is FALSE at that level) is the
`holds`-PROJECTION of this fold — `Verdict3.toBool` is a homomorphism
for both connectives (`toBool_and` / `toBool_or`) — so the
three-valued refinement never disturbs the proved denotation.
Bridge: the engine's typed `crate::Error::MeasureOfRay` raise; the
naive oracle folds verdicts commutatively in this lattice — the
order-insensitive verdict IS its spec. -/

/-- The three-valued verdict of one condition evaluation: the two
boolean outcomes plus the ray poison (ruled 2026-07-23, R6). -/
inductive Verdict3 where
  | holds
  | fails
  | ray
deriving DecidableEq

namespace Verdict3

/-- A leaf's verdict from its `Option`-poisoned boolean reading:
`none` is the ray, exactly `Value.measure?`'s refusal arm. -/
def ofOption : Option Bool → Verdict3
  | some true => .holds
  | some false => .fails
  | none => .ray

/-- Strong Kleene conjunction: `fails` dominates, `ray` survives
everything but a `fails`. -/
def and : Verdict3 → Verdict3 → Verdict3
  | .fails, _ => .fails
  | _, .fails => .fails
  | .ray, _ => .ray
  | _, b => b

/-- Strong Kleene disjunction: `holds` dominates, `ray` survives
everything but a `holds`. -/
def or : Verdict3 → Verdict3 → Verdict3
  | .holds, _ => .holds
  | _, .holds => .holds
  | .ray, _ => .ray
  | _, b => b

/-- Conjunction is commutative — half of order-unobservability. -/
theorem and_comm (a b : Verdict3) : a.and b = b.and a := by
  cases a <;> cases b <;> rfl

/-- Conjunction is associative — the other half. -/
theorem and_assoc (a b c : Verdict3) :
    (a.and b).and c = a.and (b.and c) := by
  cases a <;> cases b <;> cases c <;> rfl

/-- Disjunction is commutative. -/
theorem or_comm (a b : Verdict3) : a.or b = b.or a := by
  cases a <;> cases b <;> rfl

/-- Disjunction is associative. -/
theorem or_assoc (a b c : Verdict3) :
    (a.or b).or c = a.or (b.or c) := by
  cases a <;> cases b <;> cases c <;> rfl

/-- Conjunction distributes over disjunction — the DNF-agreement law:
distributing a condition tree moves no verdict, so the Kleene fold
and the lowered disjunct-set reading are one semantics by
construction. -/
theorem and_or_distrib (a b c : Verdict3) :
    a.and (b.or c) = (a.and b).or (a.and c) := by
  cases a <;> cases b <;> cases c <;> rfl

/-- The `holds`-projection: the boolean reading the effect-free
denotation takes — a `ray` reads `false` (PRD 04's recorded
narrowing). -/
def toBool : Verdict3 → Bool
  | .holds => true
  | _ => false

/-- The projection is a homomorphism for conjunction: refining the
booleans to three values never disturbs the proved denotation. -/
theorem toBool_and (a b : Verdict3) :
    (a.and b).toBool = (a.toBool && b.toBool) := by
  cases a <;> cases b <;> rfl

/-- The homomorphism, disjunctive face. -/
theorem toBool_or (a b : Verdict3) :
    (a.or b).toBool = (a.toBool || b.toBool) := by
  cases a <;> cases b <;> rfl

/-- An `and` node's verdict: the Kleene fold of its children
(`holds` is the unit). -/
def andFold (l : List Verdict3) : Verdict3 := l.foldr and .holds

/-- An `or` node's verdict (`fails` is the unit). -/
def orFold (l : List Verdict3) : Verdict3 := l.foldr or .fails

/-- A conjunction fails iff SOME child fails — membership, not
position: the fold cannot observe child order. -/
theorem andFold_eq_fails : ∀ {l : List Verdict3},
    andFold l = .fails ↔ .fails ∈ l
  | [] => by simp [andFold]
  | x :: l => by
    have ih := andFold_eq_fails (l := l)
    show (x.and (andFold l)) = .fails ↔ .fails ∈ x :: l
    cases x <;> cases h : andFold l <;>
      simp_all [Verdict3.and, List.mem_cons]

/-- **The conjunctive raise law, order-free**: a conjunction is
poisoned iff some child is a ray and NO child fails — never "the
first reached"; the reach-dependent reading is unrepresentable. -/
theorem andFold_eq_ray : ∀ {l : List Verdict3},
    andFold l = .ray ↔ .ray ∈ l ∧ .fails ∉ l
  | [] => by simp [andFold]
  | x :: l => by
    have ihr := andFold_eq_ray (l := l)
    have ihf := andFold_eq_fails (l := l)
    show (x.and (andFold l)) = .ray ↔ _
    cases x <;> cases h : andFold l <;>
      simp_all [Verdict3.and, List.mem_cons]

/-- A disjunction holds iff SOME child holds — the dual membership
law. -/
theorem orFold_eq_holds : ∀ {l : List Verdict3},
    orFold l = .holds ↔ .holds ∈ l
  | [] => by simp [orFold]
  | x :: l => by
    have ih := orFold_eq_holds (l := l)
    show (x.or (orFold l)) = .holds ↔ .holds ∈ x :: l
    cases x <;> cases h : orFold l <;>
      simp_all [Verdict3.or, List.mem_cons]

/-- **The raise law (the disjunct-set reading, ruled 2026-07-23,
R6)**: a disjunction is poisoned iff some disjunct is a ray and NONE
holds — a binding raises exactly when the order-insensitive
denotation NEEDS the measure of a ray to decide. -/
theorem orFold_eq_ray : ∀ {l : List Verdict3},
    orFold l = .ray ↔ .ray ∈ l ∧ .holds ∉ l
  | [] => by simp [orFold]
  | x :: l => by
    have ihr := orFold_eq_ray (l := l)
    have ihh := orFold_eq_holds (l := l)
    show (x.or (orFold l)) = .ray ↔ _
    cases x <;> cases h : orFold l <;>
      simp_all [Verdict3.or, List.mem_cons]

end Verdict3

/-! ## Arg-restriction — restrict-then-project -/

/-- The Arg restriction of a binding set: the fiber attaining the
key's extreme (`max` direction; `argMinSet` mirrors — the engine's
one `arg.max` flag). The key is a `Nat` observer — the encoded word,
which IS value order for both orderable domains
(`encode_u64_order_embedding` / `encode_i64_order_embedding`; the
module doc's recorded narrowing). -/
def argMaxSet (B : Set Assignment) (key : Assignment → Nat) :
    Set Assignment :=
  fun σ => σ ∈ B ∧ ∀ σ', σ' ∈ B → key σ' ≤ key σ

/-- The mirrored direction. -/
def argMinSet (B : Set Assignment) (key : Assignment → Nat) :
    Set Assignment :=
  fun σ => σ ∈ B ∧ ∀ σ', σ' ∈ B → key σ ≤ key σ'

/-- Arg answers: rows projected from the RESTRICTED set — a `Set`,
so tied bindings projecting equal rows collapse into one answer by
the carrier itself. -/
def argAnswers (B : Set Assignment) (key : Assignment → Nat)
    (finds : List VarId) : Set AnswerTuple :=
  fun t => ∃ σ, σ ∈ argMaxSet B key ∧ t = finds.map σ

/-- **Theorem 10 (`argmax_ties_all_kept`).** Ties are set-honest:
key-equality with a survivor IS survival — every extreme-attaining
binding is retained by the restriction, and each projects its answer
into `argAnswers`, where equal rows are ONE answer (the `Set` carrier
makes the dedup definitional — `answer_identity_canonical` is the
same law at PRD 04's boundary). Bridge: `fold_row.rs::fold_arg` —
"push with row-level dedup — ties are set-honest ... this dedup is
never elided"; the ArgMax contract, `20-query-ir.md` § aggregation:
"a tie yields every attaining answer". -/
theorem argmax_ties_all_kept {B : Set Assignment}
    {key : Assignment → Nat} {σ σ' : Assignment}
    (hσ : σ ∈ argMaxSet B key) (hσ' : σ' ∈ B)
    (htie : key σ' = key σ) :
    σ' ∈ argMaxSet B key ∧
      ∀ finds : List VarId,
        (finds.map σ' : AnswerTuple) ∈ argAnswers B key finds := by
  have hmem : σ' ∈ argMaxSet B key :=
    ⟨hσ', fun σ'' hσ'' => htie.symm ▸ hσ.2 σ'' hσ''⟩
  exact ⟨hmem, fun finds => ⟨σ', hmem, rfl⟩⟩

/-! ## The op inventory — the head-shape row -/

/-- The scalar folds a measure column feeds. -/
inductive ScalarFold where
  | sum
  | min
  | max
deriving DecidableEq

/-- The executable aggregate ops — the head-shape row PRD 04's
recorded narrowing deferred here (the aggregate faces of
`crate::ir::HeadTerm`). The theorems of this module are these ops'
laws: every op folds its group's distinct binding set
(`agg_over_distinct_bindings`), emits nothing over the empty set
(`empty_global_no_answer`), sums checked (`checkedSum_sound`,
`wide_accumulator_exact`), poisons on rays (`measure_fold_laws`),
packs canonically and extensionally (`pack_canonical`,
`pack_extensional`, `pack_adjacency`, `pack_lattice_closed`), and
keeps ties (`argmax_ties_all_kept`). -/
inductive AggOp where
  | count
  | countDistinct (v : VarId)
  | sum (v : VarId)
  | min (v : VarId)
  | max (v : VarId)
  | pack (v : VarId)
  | argMax (v k : VarId)
  | argMin (v k : VarId)
  | measureFold (op : ScalarFold) (v : VarId)
deriving DecidableEq

end Query

end Bumbledb
