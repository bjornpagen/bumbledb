import Bumbledb.Cardinality

/-!
# Dependencies — the dependency theory (Level 0, PRD 03)

The heart of the covenant: views, functionality (scalar and
pointwise), containment, coverage, keyed equality, exact partition,
and `holds` — what it means for a committed instance to model its
theory. Statements ported from the audited inventory
(`docs/formal/GPT55DependencyTheory.lean`) onto the in-tree base.
The extension form seats its reading here too:
`Statement.judgment` dispatches the cardinality-window form to its
denotation (`Cardinality.lean`), and the extension-vs-original
subsumption theorems live downstream in `Subsumption.lean`.

## Acceptance ≠ denotation (the load-bearing distinction)

`Containment` is the DENOTATION — the judgment the checker runs.
`TargetKeyAccepted` is the ACCEPTANCE premise — the validator's
exact-field-set target-key rule. They are structurally distinct
definitions: the target-key premise is a hypothesis wherever a theorem
spends probe-ability (`keyed_eq_unique_correspondence` via the
semantic keys; `accepted_target_key_spent` for the theory-side form),
never a conjunct of `Containment` — exactly as
`schema/validate.rs::resolve_target_key` is separate from
`storage/commit/judgment.rs`.

## The `no_closure` model note (item 10 — the D1 evaluation's seat)

Acceptance resolves EXACT field sets: `resolve_target_key` demands the
target projection's set equal a declared key's set, full stop. The
logical superkey implication — a key on `X` entails a key on every
`X' ⊇ X` — is TRUE (`no_closure_superkey_implication`, the one-line
proof below) and deliberately UNSPENT by acceptance: the validator
computes no closure, resolves no implied key, and accepts no
containment against a merely-entailed superkey. The engine even names
the entailment as diagnostics-only — `SchemaWarning::RedundantSuperkey`
acknowledges a declared strict superkey without changing enforcement
or the fingerprint. The gap between entailment and acceptance is the
model's recorded fact, not an oversight.

## Notes on the non-theorems (countermodels and refused converses)

* **Bare `==` is not unique correspondence** — the two-row target
  countermodel `Countermodels.bare_eq_not_unique`.
* **A key proves uniqueness, never existence** —
  `functionality_of_empty`: every key holds of the empty fact set.
  Nothing in a functionality statement manufactures a fact; existence
  comes only from a containment's source side.
* **Coverage is one-way** — target overhang is legal
  (`Countermodels.one_way_overhang`, the [0,10)/[0,20) overshoot).
* **`selection_monotonicity`'s converses are invalid**: weakening the
  SOURCE selection admits new source facts with no witness, and
  strengthening the TARGET selection can evict the witnesses — both
  directions fail on a one-fact model whose witness sits exactly on
  the boundary binding.

## Narrowings recorded (law 5: narrow and record)

* `Statement.judgment` reads interval shape from the field SET
  (`Header.intervalSplit` — the FieldSet doctrine): exactly one
  interval-typed field is the pointwise reading at ANY written
  position, matching the engine's order canonicalization
  (`resolve_target_key` counts interval positions as a set;
  `key_permutation` bridges statement order to key order). The gate-
  refused shapes read truthfully as follows: mixed interval/scalar
  side pairs and several-interval projections split to `none` and
  default to the scalar reading; an FD with exactly ONE interval
  position written NON-FINALLY receives the POINTWISE reading — the
  set-canonical split is position-blind by design. That reading is
  moot for accepted theories: the engine refuses the non-final shape
  at declaration (`FunctionalityIntervalNotLast`,
  `schema/validate.rs` — the neighbor probe needs the scalar prefix
  as its determinant group), so no accepted theory reaches it, and
  `holds` is consumed on ACCEPTED theories only (Txn, PRD 09).
* The pointwise judgments quantify over the tagged `Point` sum
  (`Schema.lean`), so no typing premise appears; accepted statements
  stay within one tag by positional typing — which at interval
  positions is ELEMENT-DOMAIN typing (Q1): the tag carries no width,
  so a fixed-width projection position against a general one of the
  same element meets in one tag and every judgment below holds
  unchanged (`Value.points_one_tag_u64`/`_i64` — the spec catching
  up to its own denotation). Scalar positions keep exact structural
  equality (`schema/validate.rs::positional_types_match`).
* Finiteness is never demanded: the ten items are subset and
  injectivity algebra, valid over arbitrary fact sets; the named token
  (`Set.Finite`) stays unspent in this module.
* **Closed-target key resolution is STRICTER than
  `TargetKeyAccepted`.** A containment into a closed relation resolves
  only the synthetic `[FieldId(0)]` id key
  (`schema/validate.rs::resolve_target_key`, the sealed-extension
  arm); a user-declared non-id key on a
  closed relation satisfies `TargetKeyAccepted` here yet Rust refuses
  the containment — acceptance strictly narrower, sound direction.
  Likewise `ClosedContainmentInterval`
  (`schema/validate.rs::validate_containment`) refuses
  interval-typed projections under a closed target outright — a v0
  refusal this model does not restate.
-/

namespace Bumbledb

/-! ## Views — selected projected value sets -/

/-- σφ(R): the selected fact subset. -/
def Selected (R : Set Fact) (φ : Selection) : Set Fact :=
  fun f => f ∈ R ∧ φ.satisfies f

/-- `View R φ X` — the selected projected value set πX(σφ(R)): the
answer denotation of the single-atom query `R(X | φ)` (the artifact's
`View`, ported). -/
def View (R : Set Fact) (φ : Selection) (X : List FieldId) :
    Set (List Value) :=
  fun t => ∃ f, f ∈ R ∧ φ.satisfies f ∧ f.project X = t

/-- Membership in a view, unfolded — the definitional reading. -/
theorem mem_view {R : Set Fact} {φ : Selection} {X : List FieldId}
    {t : List Value} :
    t ∈ View R φ X ↔ ∃ f, f ∈ R ∧ φ.satisfies f ∧ f.project X = t :=
  Iff.rfl

/-! ## The two judgment forms -/

/-- Functionality (scalar): πX is injective on `R` — no two distinct
facts agree on the determinant projection `X`. Determinants are field
LISTS whose SET is identity (`functionality_respects_field_set`);
composite determinants are the general case, not an extension.
Bridge: `schema/validate.rs::validate_functionality` accepts the
declaration; `storage/commit/applier.rs::Applier` rejects colliding
determinant images during the insert phase. -/
def Functionality (R : Set Fact) (X : List FieldId) : Prop :=
  ∀ f g, f ∈ R → g ∈ R → f.project X = g.project X → f = g

/-- The keyed read: under a functionality statement, a fixed determinant
image identifies at most one fact — the read surface's at-most-one answer
is the FD's injectivity INSTANTIATED, derived, never a new axiom. The
host surfaces (Rust `Snapshot::get`/`WriteTx::get`, the SDK's
`get(relation, keyStatement, key)`) are read conveniences over this fact:
keyed get changes no commit judgment and no query denotation, so no
conformance case moves.
Bridge: `key_statement_of (crates/bumbledb/src/api/db/get.rs)`;
`get (crates/bumbledb/src/api/db/snapshot.rs)`. -/
theorem keyed_get_at_most_one {R : Set Fact} {X : List FieldId}
    (h : Functionality R X) (v : List Value) :
    ∀ f g, f ∈ R → g ∈ R → f.project X = v → g.project X = v → f = g :=
  fun f g hf hg hfv hgv => h f g hf hg (hfv.trans hgv.symm)

/-- The pointwise key `R(S…, i) -> R`: two DISTINCT facts agreeing on
the scalar prefix `S` share no point of their interval position `i`
(read via `Value.points`, i.e. `Interval.points`). The "exclusion
constraint" is this judgment on this type, not a feature.
Bridge: `validate_functionality` admits one final interval position
and mints `DisjointDeterminantProof`;
`storage/commit/applier.rs::Applier::probe_neighbors` rejects overlap
with predecessor or successor. -/
def PointwiseKey (R : Set Fact) (S : List FieldId) (i : FieldId) :
    Prop :=
  ∀ f g, f ∈ R → g ∈ R → f.project S = g.project S → f ≠ g →
    ∀ x, x ∈ (f i).points → x ∉ (g i).points

/-- The containment judgment `A(X | φ) <= B(Y | ψ)`, fact-level — the
form the checker runs (`judgment.rs` source side): every selected
source fact has a selected target witness with the same projected
tuple.

ACCEPTANCE IS NOT HERE: the target-key premise (`TargetKeyAccepted`)
is a structurally separate definition, carried as a hypothesis where a
theorem spends it — never baked into this denotation. -/
def Containment (A : Set Fact) (φ : Selection) (X : List FieldId)
    (B : Set Fact) (ψ : Selection) (Y : List FieldId) : Prop :=
  ∀ f, f ∈ A → φ.satisfies f →
    ∃ g, g ∈ B ∧ ψ.satisfies g ∧ g.project Y = f.project X

/-- The ACCEPTANCE premise, distinct from the denotation: the target
projection resolves — as an exact FIELD SET — to a declared
functionality statement of the theory. This is
`schema/validate.rs::resolve_target_key`'s exact-field-set rule
(probe-ability: one determinant get answers "is this tuple present");
set equality also means a resolved key carries any interval field of
the projection, so the pointwise gate's "key carries its interval"
demand is discharged by construction, exactly as in Rust. -/
def TargetKeyAccepted (T : Theory) (target : Atom) : Prop :=
  ∃ K, Statement.functionality target.relation K ∈ T.statements ∧
    sameFields K target.projection

/-- Bare `==`: mutual containment, each direction judged
independently — projected view equality, NOT unique correspondence
(`Countermodels.bare_eq_not_unique`).
Bridge: `bumbledb-macros::parse_statement` lowers `==` to two adjacent
containment descriptors; the `schema_macro` locks pin their order and
pairing. -/
structure ContainsEq (A : Set Fact) (φ : Selection) (X : List FieldId)
    (B : Set Fact) (ψ : Selection) (Y : List FieldId) : Prop where
  forward : Containment A φ X B ψ Y
  backward : Containment B ψ Y A φ X

/-- Accepted `==`: mutual containment with BOTH selected projections
keyed — a conjunction of ordinary judgments, not a new primitive law.
The key premises are the SEMANTIC form the runtime discharges;
`TargetKeyAccepted` (each direction independently) is the theory-side
premise that licenses them.
Bridge: both lowered containments independently pass
`resolve_target_key`; the ==-reverse-key locks
(`equality_rejects_a_singleton_reverse_projection_without_a_left_key`,
its composite sibling, and the macro reverse-half lock) pin the
requirement. -/
structure KeyBackedEquality (A : Set Fact) (φ : Selection)
    (X : List FieldId) (B : Set Fact) (ψ : Selection)
    (Y : List FieldId) : Prop where
  eq : ContainsEq A φ X B ψ Y
  source_key : Functionality (Selected A φ) X
  target_key : Functionality (Selected B ψ) Y

/-! ## The pointwise judgments -/

/-- The pointwise support of `R(S…, i | φ)` at scalar group `s`:
every point some selected fact of the group covers (the artifact's
`IntervalSupport`, ported to `Value.points`). -/
def Support (R : Set Fact) (φ : Selection) (S : List FieldId)
    (i : FieldId) (s : List Value) : Set Point :=
  fun x => ∃ f, f ∈ R ∧ φ.satisfies f ∧ f.project S = s ∧
    x ∈ (f i).points

/-- Membership in a support, unfolded — the definitional reading. -/
theorem mem_support {R : Set Fact} {φ : Selection} {S : List FieldId}
    {i : FieldId} {s : List Value} {x : Point} :
    x ∈ Support R φ S i s ↔
      ∃ f, f ∈ R ∧ φ.satisfies f ∧ f.project S = s ∧
        x ∈ (f i).points :=
  Iff.rfl

/-- Coverage — the pointwise containment
`A(S…, i | φ) <= B(U…, j | ψ)`: every point of every selected source
fact's interval is covered by a selected target fact of the SAME
scalar group (the artifact's `IntervalContains`, ported). Direction
law: this covers the source support only; target overhang is legal
(`Countermodels.one_way_overhang`).
Bridge: `Enforcement::IntervalCoverage` carries the validator-minted
`DisjointDeterminantProof` into `Checker::check_coverage`, whose
signature requires the proof — no boolean can license the sweep. -/
def Coverage (A : Set Fact) (φ : Selection) (S : List FieldId)
    (i : FieldId) (B : Set Fact) (ψ : Selection) (U : List FieldId)
    (j : FieldId) : Prop :=
  ∀ f, f ∈ A → φ.satisfies f → ∀ x, x ∈ (f i).points →
    ∃ g, g ∈ B ∧ ψ.satisfies g ∧ g.project U = f.project S ∧
      x ∈ (g j).points

/-- Exact partition: pointwise-keyed target plus two-sided support
equality per scalar group — the mathematically honest strengthening of
a mere disjoint cover (the artifact's `ExactPointPartition`,
ported). -/
def ExactPartition (A : Set Fact) (φ : Selection) (S : List FieldId)
    (i : FieldId) (B : Set Fact) (ψ : Selection) (U : List FieldId)
    (j : FieldId) : Prop :=
  PointwiseKey (Selected B ψ) U j ∧
    ∀ s x, x ∈ Support A φ S i s ↔ x ∈ Support B ψ U j s

/-! ## `holds` — a committed instance models its theory -/

/-- One statement's judgment over theory `T`'s denotation at instance
`I` — interval positions read through the denotation (a fact stands
for its point-family): an all-scalar projection is the classical
judgment unchanged; a projection whose field set carries exactly one
interval field is the pointwise lifting, whatever its written
position (the FieldSet doctrine — `Header.intervalSplit`).
Gate-refused shapes default to the scalar reading (recorded
narrowing — `holds` is consumed on accepted theories only). The
extension form reads its own denotation: a cardinality statement
is the per-parent window judgment (`Cardinality.lean` — window
projections refuse interval positions at the gate, the recorded v0
trigger, so no split is consulted). -/
def Statement.judgment (T : Theory) (I : Instance) :
    Statement → Prop
  | .functionality R X =>
    match T.header.intervalSplit R X with
    | some (S, i) => PointwiseKey (T.den I R) S i
    | none => Functionality (T.den I R) X
  | .containment src tgt =>
    match T.header.intervalSplit src.relation src.projection,
          T.header.intervalSplit tgt.relation tgt.projection with
    | some (S, i), some (U, j) =>
      Coverage (T.den I src.relation) src.selection S i
        (T.den I tgt.relation) tgt.selection U j
    | _, _ =>
      Containment (T.den I src.relation) src.selection src.projection
        (T.den I tgt.relation) tgt.selection tgt.projection
  | .cardinality src w tgt =>
    CardinalityWindow (T.den I src.relation) src.selection
      src.projection w (T.den I tgt.relation) tgt.selection
      tgt.projection

/-- `holds T I` — a committed instance models its theory: every
declared statement's judgment holds of the final state. This is the
final-state judgment's SPEC — dependencies are properties of
COMMITTED databases, checked once at commit against the transaction's
final state; Txn (PRD 09) consumes this.
Bridge: `storage/commit/judgment.rs::judge` (delta-restricted, sound
because an untouched binding keeps its pre-state verdict — the
restriction theorems, `Txn/DeltaRestriction.lean`) and
`Db::verify_store` (the global re-verification). -/
def holds (T : Theory) (I : Instance) : Prop :=
  ∀ s, s ∈ T.statements → s.judgment T I

/-! ## Item 1 — containment is view inclusion -/

/-- **Item 1 (port).** The fact-level containment judgment is exactly
subset inclusion of selected projected views — the checker's per-fact
probe and the denotation `πX(σφ(A)) ⊆ πY(σψ(B))` are one statement.
Bridge: `resolve_target_key` requires the exact-field-set target key;
`judgment.rs::Checker` checks each delta-touched source against the
final state. -/
theorem contains_iff_view_subset
    (A : Set Fact) (φ : Selection) (X : List FieldId)
    (B : Set Fact) (ψ : Selection) (Y : List FieldId) :
    Containment A φ X B ψ Y ↔ View A φ X ⊆ View B ψ Y := by
  constructor
  · intro h t ht
    obtain ⟨f, hfA, hfφ, hft⟩ := mem_view.mp ht
    obtain ⟨g, hgB, hgψ, hgf⟩ := h f hfA hfφ
    exact ⟨g, hgB, hgψ, hgf.trans hft⟩
  · intro h f hfA hfφ
    exact mem_view.mp (h (f.project X) ⟨f, hfA, hfφ, rfl⟩)

/-! ## Item 2 — bare `==` is view equality -/

/-- **Item 2 (port).** Bare `==` (two independent containments) is
exactly extensional equality of the two views — and NOTHING more:
unique correspondence needs the key premises
(`Countermodels.bare_eq_not_unique` is the two-row countermodel).
Bridge: the `==` lowering to two adjacent containment descriptors;
`schema_macro::statements_land_in_source_order_with_equality_lowered`
and `the_equality_pair_seals_mirror_links` pin order and pairing. -/
theorem containsEq_iff_view_ext
    (A : Set Fact) (φ : Selection) (X : List FieldId)
    (B : Set Fact) (ψ : Selection) (Y : List FieldId) :
    ContainsEq A φ X B ψ Y ↔
      ∀ t, t ∈ View A φ X ↔ t ∈ View B ψ Y := by
  constructor
  · intro h t
    exact ⟨fun ht => (contains_iff_view_subset A φ X B ψ Y).mp
             h.forward t ht,
           fun ht => (contains_iff_view_subset B ψ Y A φ X).mp
             h.backward t ht⟩
  · intro h
    exact ⟨(contains_iff_view_subset A φ X B ψ Y).mpr
             fun t ht => (h t).mp ht,
           (contains_iff_view_subset B ψ Y A φ X).mpr
             fun t ht => (h t).mpr ht⟩

/-! ## Item 3 — accepted `==` is a keyed bijection -/

/-- **Item 3 (port, restated as one statement).** Key-backed equality
is a one-to-one correspondence between the σ-subsets: every selected
source fact has EXACTLY ONE selected target witness with the same
projected tuple, and symmetrically. The composite-projection
generality is explicit — `X` and `Y` are field lists (determinants are
field SETS; `functionality_respects_field_set` is the order-invariance
of the key premise), so the correspondence is on whole projected
PRODUCTS, never per-column. This is not whole-fact equality:
unprojected payloads may differ.
Bridge: the ==-reverse-key locks and
`three_field_reordered_key_equality_validates_and_enforces_both_directions`
(mixed-type composite product, permutation, both existence directions,
uniqueness, differing payloads). -/
theorem keyed_eq_unique_correspondence
    {A : Set Fact} {φ : Selection} {X : List FieldId}
    {B : Set Fact} {ψ : Selection} {Y : List FieldId}
    (h : KeyBackedEquality A φ X B ψ Y) :
    (∀ f, f ∈ Selected A φ →
      ∃ g, (g ∈ Selected B ψ ∧ g.project Y = f.project X) ∧
        ∀ g', g' ∈ Selected B ψ → g'.project Y = f.project X →
          g' = g) ∧
    (∀ g, g ∈ Selected B ψ →
      ∃ f, (f ∈ Selected A φ ∧ f.project X = g.project Y) ∧
        ∀ f', f' ∈ Selected A φ → f'.project X = g.project Y →
          f' = f) := by
  constructor
  · intro f hf
    obtain ⟨g, hgB, hgψ, hgproj⟩ := h.eq.forward f hf.1 hf.2
    refine ⟨g, ⟨⟨hgB, hgψ⟩, hgproj⟩, ?_⟩
    intro g' hg' hproj'
    exact h.target_key g' g hg' ⟨hgB, hgψ⟩ (hproj'.trans hgproj.symm)
  · intro g hg
    obtain ⟨f, hfA, hfφ, hfproj⟩ := h.eq.backward g hg.1 hg.2
    refine ⟨f, ⟨⟨hfA, hfφ⟩, hfproj⟩, ?_⟩
    intro f' hf' hproj'
    exact h.source_key f' f hf' ⟨hfA, hfφ⟩ (hproj'.trans hfproj.symm)

/-! ## Item 4 — a key proves at most one fact per determinant tuple -/

/-- **Item 4.** Under a functionality statement there is AT MOST ONE
fact per determinant tuple. The non-theorem twin is a note, backed by
`functionality_of_empty`: keys prove uniqueness, never existence —
nothing here manufactures a fact.
Bridge: `validate_functionality` accepts the declaration;
`Applier` rejects colliding determinant images but never inserts. -/
theorem functionality_unique_witness
    {R : Set Fact} {X : List FieldId} (h : Functionality R X)
    (t : List Value) :
    ∀ f, f ∈ R → f.project X = t →
      ∀ g, g ∈ R → g.project X = t → g = f :=
  fun f hf hft g hg hgt => h g f hg hf (hgt.trans hft.symm)

/-- The existence gap, machine-backed: every key holds of the EMPTY
fact set. A key constrains what may coexist; it never demands that
anything exist — existence obligations are containments' alone. -/
theorem functionality_of_empty (X : List FieldId) :
    Functionality (fun _ => False) X :=
  fun _ _ hf => False.elim hf

/-! ## Item 5 — pointwise keys give per-group disjointness -/

/-- **Item 5.** Under a pointwise key, two distinct facts of one
scalar group have DISJOINT point sets — the per-group pairwise
disjointness (and start-orderability) the coverage walk's forward
sweep relies on.
Bridge: the pointwise gate mints `DisjointDeterminantProof` at
validate; `Applier::probe_neighbors` maintains the judgment;
`Checker::check_coverage` requires the proof by signature — the sweep
cannot be selected by an unchecked flag. -/
theorem pointwise_key_disjoint
    {R : Set Fact} {S : List FieldId} {i : FieldId}
    (h : PointwiseKey R S i) {f g : Fact} (hf : f ∈ R) (hg : g ∈ R)
    (hgroup : f.project S = g.project S) (hne : f ≠ g) :
    ∀ x, ¬(x ∈ (f i).points ∧ x ∈ (g i).points) :=
  fun x hx => h f g hf hg hgroup hne x hx.1 hx.2

/-! ## Item 6 — coverage is support inclusion -/

/-- **Item 6 (port).** One-way interval coverage is exactly pointwise
support inclusion per scalar group — and only INCLUSION: target
overhang is legal (`Countermodels.one_way_overhang`, the
[0,10)/[0,20) overshoot that kills the tiling over-read).
Bridge: `Checker::check_coverage` advances only across the demanded
source interval; the walk verifies no gap before the source's end and
never convicts overhang. -/
theorem coverage_is_support_inclusion
    (A : Set Fact) (φ : Selection) (S : List FieldId) (i : FieldId)
    (B : Set Fact) (ψ : Selection) (U : List FieldId) (j : FieldId) :
    Coverage A φ S i B ψ U j ↔
      ∀ s, Support A φ S i s ⊆ Support B ψ U j s := by
  constructor
  · intro h s x hx
    obtain ⟨f, hfA, hfφ, hfs, hxf⟩ := mem_support.mp hx
    obtain ⟨g, hgB, hgψ, hgU, hxg⟩ := h f hfA hfφ x hxf
    exact ⟨g, hgB, hgψ, hgU.trans hfs, hxg⟩
  · intro h f hfA hfφ x hxf
    exact mem_support.mp (h (f.project S) x ⟨f, hfA, hfφ, rfl, hxf⟩)

/-! ## Item 7 — mutual coverage is support equality -/

/-- **Item 7.** Both coverage directions together give EQUAL point
supports per scalar group — the two halves of recipe 26's `==` pair,
before the disjointness that upgrades equality to partition.
Bridge: cookbook recipe 26's commit matrix
(`r26_exact_partition_commit_matrix`: forward-gap rejection,
reverse-overhang rejection). -/
theorem mutual_coverage_support_equality
    {A : Set Fact} {φ : Selection} {S : List FieldId} {i : FieldId}
    {B : Set Fact} {ψ : Selection} {U : List FieldId} {j : FieldId}
    (hAB : Coverage A φ S i B ψ U j)
    (hBA : Coverage B ψ U j A φ S i) :
    ∀ s x, x ∈ Support A φ S i s ↔ x ∈ Support B ψ U j s :=
  fun s x =>
    ⟨fun hx => (coverage_is_support_inclusion A φ S i B ψ U j).mp
       hAB s x hx,
     fun hx => (coverage_is_support_inclusion B ψ U j A φ S i).mp
       hBA s x hx⟩

/-! ## Item 8 — the exact-partition equivalence -/

/-- **Item 8 (port of the tiling equivalence).** Target disjointness
plus mutual coverage is EXACTLY exact partition — the five-statement
idiom's theorem: no partition primitive exists, only ordinary
statements whose conjunction is provably the partition.
Bridge: cookbook recipe 26 spells the five statements;
`r26_exact_partition_commit_matrix` locks exact acceptance, gap and
overhang rejection, half-open adjacency, and a two-scalar-prefix
instance. -/
theorem exact_partition_iff
    (A : Set Fact) (φ : Selection) (S : List FieldId) (i : FieldId)
    (B : Set Fact) (ψ : Selection) (U : List FieldId) (j : FieldId) :
    (PointwiseKey (Selected B ψ) U j ∧
      Coverage A φ S i B ψ U j ∧ Coverage B ψ U j A φ S i) ↔
    ExactPartition A φ S i B ψ U j := by
  constructor
  · intro h
    exact ⟨h.1, mutual_coverage_support_equality h.2.1 h.2.2⟩
  · intro h
    refine ⟨h.1, ?_, ?_⟩
    · exact (coverage_is_support_inclusion A φ S i B ψ U j).mpr
        fun s x hx => (h.2 s x).mp hx
    · exact (coverage_is_support_inclusion B ψ U j A φ S i).mpr
        fun s x hx => (h.2 s x).mpr hx

/-! ## Item 9 — selection monotonicity -/

/-- **Item 9 (port, both valid directions).** Containment is preserved
by STRENGTHENING the source selection and by WEAKENING the target
selection — the two monotone moves. The converses are INVALID
(module doc): weakening the source admits unwitnessed facts;
strengthening the target evicts witnesses.
`Selection.satisfies_of_superset` supplies the hypotheses for the
accepted fragment's syntactic strengthening (more bindings).
Bridge: `judgment.rs::SelectionCheck::Never` spends the strengthening
limit — a never-interned σ literal proves its side unsatisfiable, the
strongest source selection, under which the judgment holds
vacuously. -/
theorem selection_monotonicity
    {A : Set Fact} {φ : Selection} {X : List FieldId}
    {B : Set Fact} {ψ : Selection} {Y : List FieldId}
    (h : Containment A φ X B ψ Y) :
    (∀ φ' : Selection, (∀ f, φ'.satisfies f → φ.satisfies f) →
      Containment A φ' X B ψ Y) ∧
    (∀ ψ' : Selection, (∀ g, ψ.satisfies g → ψ'.satisfies g) →
      Containment A φ X B ψ' Y) := by
  constructor
  · intro φ' hφ f hfA hfφ'
    exact h f hfA (hφ f hfφ')
  · intro ψ' hψ f hfA hfφ
    obtain ⟨g, hgB, hgψ, hgY⟩ := h f hfA hfφ
    exact ⟨g, hgB, hψ g hgψ, hgY⟩

/-! ## Item 10 — the superkey implication, proved and unspent -/

/-- **Item 10 (the `no_closure` note's one-line implication).** A key
on `X` entails a key on every superset `X'`: agreement on the larger
determinant restricts to agreement on the smaller. TRUE, and
deliberately UNSPENT by acceptance — the model note in the module doc
records the entailment-vs-acceptance gap explicitly.
Bridge: `resolve_target_key`'s exact-field-set rule spends NO
entailment; `SchemaWarning::RedundantSuperkey` names the implication
as diagnostics only, outside enforcement and the fingerprint. -/
theorem no_closure_superkey_implication
    {R : Set Fact} {X X' : List FieldId} (h : Functionality R X)
    (hsub : ∀ i, i ∈ X → i ∈ X') :
    Functionality R X' :=
  fun f g hf hg hproj =>
    h f g hf hg ((Fact.project_eq_iff f g X).mpr
      fun i hi => (Fact.project_eq_iff f g X').mp hproj i (hsub i hi))

/-- Key identity is the field SET: functionality over one field set is
functionality over any reordering — why duplicate FDs over one set are
rejected regardless of projection order, and why `key_permutation`
only reorders fields and weakens nothing. -/
theorem functionality_respects_field_set
    {R : Set Fact} {X X' : List FieldId} (hset : sameFields X X') :
    Functionality R X ↔ Functionality R X' :=
  ⟨fun h => no_closure_superkey_implication h fun i hi =>
     (hset i).mp hi,
   fun h => no_closure_superkey_implication h fun i hi =>
     (hset i).mpr hi⟩

/-! ## Spending acceptance -/

/-- A key on a fact set restricts to a key on any selected subset —
how the whole-relation FD the schema declares supplies the σ-subset
key premises of `KeyBackedEquality`. -/
theorem functionality_selected
    {R : Set Fact} {X : List FieldId} (φ : Selection)
    (h : Functionality R X) : Functionality (Selected R φ) X :=
  fun f g hf hg => h f g hf.1 hg.1

/-- Acceptance SPENT (the scalar arm): on a holding instance, a
containment's accepted target key IS semantic functionality of the
target denotation over the target projection — acceptance (the
theory-side premise the validator resolves) plus judgment (`holds`)
yields the injectivity the probe relies on. The field-set transfer is
`functionality_respects_field_set`; the acceptance premise enters as a
HYPOTHESIS, which is the criteria's point.
Bridge: `resolve_target_key` (acceptance) +
`judgment.rs`/`Applier` (the runtime discharge). -/
theorem accepted_target_key_spent
    {T : Theory} {I : Instance} (hI : holds T I) {tgt : Atom}
    (hacc : TargetKeyAccepted T tgt)
    (hscalar : ∀ i, i ∈ tgt.projection →
      T.header.isInterval tgt.relation i = false) :
    Functionality (T.den I tgt.relation) tgt.projection := by
  obtain ⟨K, hmem, hset⟩ := hacc
  have hj := hI _ hmem
  have hnone : T.header.intervalSplit tgt.relation K = none :=
    T.header.intervalSplit_scalar tgt.relation K
      fun i hi => hscalar i ((hset i).mp hi)
  simp only [Statement.judgment, hnone] at hj
  exact (functionality_respects_field_set hset).mp hj

end Bumbledb
