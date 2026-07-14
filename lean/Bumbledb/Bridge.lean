import Bumbledb.Values
import Bumbledb.Schema
import Bumbledb.Dependencies
import Bumbledb.Query.Syntax
import Bumbledb.Query.Denotation
import Bumbledb.Query.Aggregates
import Bumbledb.Exec.Sweep
import Bumbledb.Exec.Dedup
import Bumbledb.Exec.Rewrites
import Bumbledb.Txn
import Bumbledb.Countermodels

/-!
# Bridge — the obligation ledger (PRD 10)

The machine-listable Lean↔Rust boundary: one `Obligation` per premise
the Rust engine discharges, collated from the modules' inline `Bridge:`
notes (PRDs 02–09), replacing the prose theorem↔evidence table that
lived in `docs/architecture/30-dependencies.md`.

## The two checked halves

* **The Lean half is CHECKED BY THE BUILD.** Every row is constructed
  through `Obligation.row`, whose first argument is the theorem ITSELF
  (`@theoremName` — the chosen mechanism, recorded per the PRD: one
  term-level reference inside each row, so a renamed or deleted theorem
  is an unknown-constant elaboration error and `lake build` fails). The
  `theoremName : Lean.Name` field is the machine-listable rendering of
  the same reference.
* **The Rust/docs half is CHECKED BY THE CENSUS**
  (`scripts/spec-census.sh`, run via `scripts/lean.sh` and the CI lean
  job): every `mechanism` and `instrument` token of the form
  `symbol (path)` must find its path on disk and its symbol inside that
  path; bare `crates/…` / `fuzz/…` tokens must exist on disk; and every
  `lean/…` citation in `docs/architecture/` and `docs/cookbook.md` must
  resolve to a real declaration in this tree.

## String conventions (the census's parse contract)

* `premise` is ONE prose sentence — no `::`, no `crates/`, no `fuzz/`
  (any such token would make the census scan it).
* `mechanism` and `instrument` are census-scanned: semicolon-joined
  `symbol (path)` pairs (the symbol's final `::`-segment must grep
  word-bounded inside the path) and bare repository paths (existence).
  An instrument names a test fn, a fuzz target, or a trophy path.

## The inline residue

The modules' per-theorem `Bridge:` doc-comment notes REMAIN as the
in-context pointers (the allowed residue); this file is the collation
the census checks. The ledger row count is asserted at the bottom
(`ledger_count`) so a dropped row is a build failure, not a drift.
-/

namespace Bumbledb
namespace Bridge

/-- One row of the obligation ledger: a Lean premise, the Rust
mechanism that discharges it, and the instrument that empirically
watches the seam. -/
structure Obligation where
  /-- The fully qualified theorem name (the machine-listable half of
  the checked reference `Obligation.row` carries). -/
  theoremName : Lean.Name
  /-- One sentence: what Lean assumes. -/
  premise : String
  /-- The Rust discharge site, exact: `symbol (path)`. -/
  mechanism : String
  /-- What empirically watches the seam: a test fn, fuzz target, or
  trophy — `symbol (path)` or a bare repository path. -/
  instrument : String

/-- The checked row constructor — the PRD's "lightest mechanism that
makes `lake build` fail on a dangling name": the first argument is the
referenced theorem itself (`@theoremName`), so every ledger row carries
a term-level reference the elaborator must resolve. -/
def Obligation.row {α : Sort u} (_checked : α) (theoremName : Lean.Name)
    (premise mechanism instrument : String) : Obligation :=
  { theoremName, premise, mechanism, instrument }

/-- The obligation ledger. Ordered by PRD (02 → 09); collated
exhaustively from the module docs' `Bridge:` notes. -/
def ledger : List Obligation := [

  /- ## PRD 02 — Values -/

  .row @interval_nonempty `Bumbledb.interval_nonempty
    "Every representable interval denotes a nonempty point set — the invariant the constructor discharges by parsing, never validating."
    "crate::Interval::new (crates/bumbledb/src/interval.rs)"
    "new_parses_strict_start_before_end (crates/bumbledb/src/interval.rs); value_variants_accept_only_checked_intervals (crates/bumbledb/src/interval.rs)",

  .row @points_halfopen `Bumbledb.points_halfopen
    "Interval membership is exactly the half-open reading — inclusive at start, exclusive at end — the contract every consumer assumes."
    "crate::Interval::start (crates/bumbledb/src/interval.rs); crate::allen::classify (crates/bumbledb/src/allen.rs)"
    "accessors_return_the_parsed_bounds (crates/bumbledb/src/interval.rs); adjacency_continues_and_the_minimal_gap_breaks (crates/bumbledb/src/interval/sweep.rs)",

  .row @ray_is_unbounded_tail `Bumbledb.ray_is_unbounded_tail
    "A ray is the interval whose end IS the domain ceiling — infinity is a value of the representation, never a sentinel."
    "crate::Interval::ray (crates/bumbledb/src/interval.rs); crate::Interval::is_ray (crates/bumbledb/src/interval.rs)"
    "ray_is_the_unbounded_denotation (crates/bumbledb/src/interval.rs)",

  .row @measure_ray_none `Bumbledb.measure_ray_none
    "A ray has no measure — the model reads none where the engine raises the typed error."
    "crate::Error::MeasureOfRay (crates/bumbledb/src/error.rs)"
    "a_ray_reaching_duration_raises_and_a_filtered_query_succeeds (crates/bumbledb/src/api/prepared/tests/measure.rs)",

  .row @measure_finite `Bumbledb.measure_finite
    "A bounded interval's measure is exactly end minus start — the happy path of measure evaluation."
    "crate::ir::Term::Measure (crates/bumbledb/src/ir.rs)"
    "duration_find_projects_the_measure_u64 (crates/bumbledb/src/api/prepared/tests/measure.rs); duration_find_projects_the_measure_i64 (crates/bumbledb/src/api/prepared/tests/measure.rs)",

  .row @encode_u64_order_embedding `Bumbledb.encode_u64_order_embedding
    "The unsigned encoding is an order embedding — lexicographic word order equals numeric order."
    "crate::encoding::encode::encode_u64 (crates/bumbledb/src/encoding/encode.rs)"
    "exhaustive_u64_encoding_preserves_order_at_byte_boundaries (crates/bumbledb/src/encoding/tests.rs); u64_order_preservation (crates/bumbledb/src/encoding/tests.rs)",

  .row @encode_i64_order_embedding `Bumbledb.encode_i64_order_embedding
    "The sign-flip law: the signed encoding is an order embedding, the bias form of the two's-complement sign-bit flip."
    "crate::encoding::encode::encode_i64 (crates/bumbledb/src/encoding/encode.rs)"
    "exhaustive_i64_encoding_preserves_order_across_the_sign_boundary (crates/bumbledb/src/encoding/tests.rs); i64_order_preservation_across_sign_boundary (crates/bumbledb/src/encoding/tests.rs)",

  .row @encode_interval_order `Bumbledb.encode_interval_order
    "The two-half interval encoding preserves the start-then-end lexicographic order the determinant walks read (u64 companion in-tree beside it)."
    "crate::encoding::encode::encode_interval_i64 (crates/bumbledb/src/encoding/encode.rs); crate::encoding::encode::encode_interval_u64 (crates/bumbledb/src/encoding/encode.rs)"
    "interval_encoding_orders_by_start_then_end (crates/bumbledb/src/encoding/tests.rs)",

  .row @value_eq_iff_encode_eq `Bumbledb.value_eq_iff_encode_eq
    "Canonical-bytes identity: within one value type, value equality is exactly canonical-encoding equality — the fact-identity law, per-database for interned strings."
    "crate::encoding::encode::encode_literal (crates/bumbledb/src/encoding/encode.rs); crate::encoding::encode::encode_fact (crates/bumbledb/src/encoding/encode.rs)"
    "encode_fact_matches_independent_field_encodings (crates/bumbledb/src/encoding/tests.rs)",

  /- ## PRD 03 — Schema and Dependencies -/

  .row @den_closed_constant `Bumbledb.den_closed_constant
    "Ground axioms are constants of the theory: a closed relation denotes the same sealed fact set at every instance, so closed-to-closed statements are decided at validate outright."
    "schema/validate.rs::validate_containment (crates/bumbledb/src/schema/validate.rs); crate::Error::ClosedStatementRefuted (crates/bumbledb/src/error.rs)"
    "a_closed_relation_seals_pre_encoded_ground_axioms (crates/bumbledb/src/schema/tests/valid.rs); a_satisfied_closed_to_closed_containment_validates (crates/bumbledb/src/schema/tests/valid.rs)",

  .row @contains_iff_view_subset `Bumbledb.contains_iff_view_subset
    "The containment judgment is exactly subset inclusion of selected projected views — the checker's per-fact probe and the denotation are one statement."
    "schema/validate.rs::resolve_target_key (crates/bumbledb/src/schema/validate.rs); judgment.rs::Checker (crates/bumbledb/src/storage/commit/judgment.rs)"
    "a_coherently_deleted_scalar_target_is_a_judgment_violation (crates/bumbledb/src/verify_store/tests.rs)",

  .row @accepted_target_key_spent `Bumbledb.accepted_target_key_spent
    "Acceptance spent: on a holding instance, an accepted target key is semantic functionality of the target denotation — the exact-field-set premise enters as a hypothesis, never a conjunct of the denotation."
    "schema/validate.rs::resolve_target_key (crates/bumbledb/src/schema/validate.rs); judgment.rs::judge (crates/bumbledb/src/storage/commit/judgment.rs)"
    "a_coherently_deleted_scalar_target_is_a_judgment_violation (crates/bumbledb/src/verify_store/tests.rs)",

  .row @containsEq_iff_view_ext `Bumbledb.containsEq_iff_view_ext
    "Bare mutual containment is projected view equality and nothing more — the equality statement lowers to two adjacent containments."
    "bumbledb-macros::parse_statement (crates/bumbledb-macros/src/lib.rs)"
    "statements_land_in_source_order_with_equality_lowered (crates/bumbledb/tests/schema_macro.rs); the_equality_pair_seals_mirror_links (crates/bumbledb/tests/schema_macro.rs)",

  .row @keyed_eq_unique_correspondence `Bumbledb.keyed_eq_unique_correspondence
    "Accepted key-backed equality is a one-to-one correspondence between the selected subsets, on whole projected products, both directions keyed."
    "schema/validate.rs::resolve_target_key (crates/bumbledb/src/schema/validate.rs)"
    "three_field_reordered_key_equality_validates_and_enforces_both_directions (crates/bumbledb/tests/schema_macro.rs); equality_rejects_a_singleton_reverse_projection_without_a_left_key (crates/bumbledb/src/schema/tests/reject.rs)",

  .row @functionality_unique_witness `Bumbledb.functionality_unique_witness
    "Under a functionality statement there is at most one fact per determinant tuple — a key proves uniqueness, never existence."
    "schema/validate.rs::validate_functionality (crates/bumbledb/src/schema/validate.rs); applier.rs::Applier (crates/bumbledb/src/storage/commit/applier.rs)"
    "scalar_key_conflict_in_one_delta_aborts_with_the_statement_id (crates/bumbledb/src/storage/commit/tests/commit.rs); scalar_key_conflict_across_deltas_aborts_with_the_statement_id (crates/bumbledb/src/storage/commit/tests/commit.rs)",

  .row @pointwise_key_disjoint `Bumbledb.pointwise_key_disjoint
    "A pointwise key gives per-scalar-group pairwise disjointness of interval point sets — the premise the coverage sweep's witness token attests."
    "crate::schema::DisjointDeterminantProof (crates/bumbledb/src/schema.rs); Applier::probe_neighbors (crates/bumbledb/src/storage/commit/applier.rs)"
    "overlap_left_in_delta_aborts (crates/bumbledb/src/storage/commit/tests/functionality.rs); pointwise_overlap_is_found_by_the_ordered_walk (crates/bumbledb/src/verify_store/tests.rs)",

  .row @coverage_is_support_inclusion `Bumbledb.coverage_is_support_inclusion
    "One-way interval coverage is exactly pointwise support inclusion per scalar group — inclusion only, so target overhang is legal."
    "Checker::check_coverage (crates/bumbledb/src/storage/commit/judgment.rs)"
    "r26_exact_partition_commit_matrix (crates/bumbledb-query/tests/cookbook.rs)",

  .row @exact_partition_iff `Bumbledb.exact_partition_iff
    "Target disjointness plus mutual coverage is exactly exact partition — five ordinary statements, no partition primitive."
    "Enforcement::IntervalCoverage (crates/bumbledb/src/schema.rs)"
    "r26_exact_partition_commit_matrix (crates/bumbledb-query/tests/cookbook.rs)",

  .row @selection_monotonicity `Bumbledb.selection_monotonicity
    "Containment is preserved by strengthening the source selection and weakening the target selection — a never-interned source literal is the strongest source selection, held vacuously."
    "SelectionCheck::Never (crates/bumbledb/src/storage/commit/judgment.rs)"
    "an_uninterned_sigma_literal_resolves_to_never (crates/bumbledb/src/storage/commit/tests/sealed_checks.rs)",

  .row @no_closure_superkey_implication `Bumbledb.no_closure_superkey_implication
    "The decidability firewall: the superkey implication is true and deliberately unspent — acceptance resolves exact field sets, computes no closure, and names the entailment as diagnostics only."
    "schema/validate.rs::resolve_target_key (crates/bumbledb/src/schema/validate.rs); SchemaWarning::RedundantSuperkey (crates/bumbledb/src/schema.rs)"
    "a_redundant_pointwise_superkey_seals_with_a_warning (crates/bumbledb/src/schema/tests/valid.rs); redundant_superkey_warns_without_weakening_either_enforcement_plan (crates/bumbledb/tests/schema_macro.rs); equality_rejects_a_singleton_reverse_projection_without_a_left_key (crates/bumbledb/src/schema/tests/reject.rs)",

  /- ## PRD 04 — Query denotation -/

  .row @Query.matches_def `Bumbledb.Query.matches_def
    "The matching equation: a fact matches an atom iff every binding's term selects the fact's value at that field, absence of a field being the wildcard."
    "crate::ir::Atom (crates/bumbledb/src/ir.rs)"
    "fuzz/fuzz_targets/query.rs",

  .row @Query.repeated_var_unifies `Bumbledb.Query.repeated_var_unifies
    "A repeated variable unifies: within one atom it forces same-fact field equality, and across atoms of one rule it denotes the equijoin (the cross-atom companion sits beside it)."
    "crate::ir::Term::Var (crates/bumbledb/src/ir.rs)"
    "repeated_variable_lowers_and_executes_through_the_evaluator (crates/bumbledb/src/ir/normalize/tests.rs)",

  .row @Query.param_selects_not_binds `Bumbledb.Query.param_selects_not_binds
    "A parameter position selects and never binds — read from the environment, independent of the assignment."
    "crate::ir::Term::Param (crates/bumbledb/src/ir.rs)"
    "a_param_position_does_not_bind_a_negated_variable_even_when_written_after_it (crates/bumbledb/src/ir/validate/tests/reject.rs); string_params_resolve_per_execution (crates/bumbledb/src/api/prepared/tests/params.rs)",

  .row @Query.paramSet_selects_membership `Bumbledb.Query.paramSet_selects_membership
    "A set-parameter position selects membership of the bind-time slice — never a fresh binding."
    "crate::ir::Term::ParamSet (crates/bumbledb/src/ir.rs)"
    "set_membership_matches_any_element (crates/bumbledb/src/api/prepared/tests/sets.rs); in_family_equals_the_union_of_per_element_executions (crates/bumbledb/src/api/prepared/tests/sets.rs)",

  .row @Query.antijoin_over_active_domain `Bumbledb.Query.antijoin_over_active_domain
    "Safety is positive range restriction: under it every answer value lives in the rule's active domain, and negation is the anti-join over finite extensions, never an infinite complement."
    "ValidationError::NegatedVariableUnbound (crates/bumbledb/src/error.rs); ValidationError::ComparisonOnlyVariable (crates/bumbledb/src/error.rs); context.rs::check_atoms (crates/bumbledb/src/ir/validate/context.rs)"
    "rejects_a_negated_atom_variable_unbound_by_positive_atoms (crates/bumbledb/src/ir/validate/tests/reject.rs)",

  .row @Query.membership_only_unsafe `Bumbledb.Query.membership_only_unsafe
    "A point variable bound only by membership has no enumerable domain — the rule is unsafe, exactly the membership-only refusal."
    "ValidationError::MembershipOnlyVariable (crates/bumbledb/src/error.rs); context.rs::check_membership_domains (crates/bumbledb/src/ir/validate/context.rs)"
    "rejects_a_membership_only_variable (crates/bumbledb/src/ir/validate/tests/reject.rs)",

  .row @Query.pointIn_unfold `Bumbledb.Query.pointIn_unfold
    "Point membership unfolds to the half-open endpoint comparisons — inclusive at start, exclusive at end (i64 companion in-tree beside it)."
    "crate::ir::CmpOp::PointIn (crates/bumbledb/src/ir.rs)"
    "membership_of_the_last_point_in_a_ray_is_true_and_the_ceiling_rejects (crates/bumbledb/src/api/prepared/tests/sets.rs)",

  .row @Query.allen_mask_denotation `Bumbledb.Query.allen_mask_denotation
    "The Allen comparison denotes mask membership of the classification (i64 companion in-tree beside it)."
    "crate::allen::AllenMask::contains (crates/bumbledb/src/allen.rs)"
    "composites_mean_their_point_set_definitions (crates/bumbledb/src/allen.rs)",

  .row @Query.dnf_preserves_denotation `Bumbledb.Query.dnf_preserves_denotation
    "Lowering condition trees to DNF preserves the rule's answers — the engine never sees a disjunction."
    "dnf.rs::distribute (crates/bumbledb/src/ir/normalize/dnf.rs); dnf.rs::collapse (crates/bumbledb/src/ir/normalize/dnf.rs)"
    "fuzz/fuzz_targets/query.rs",

  .row @Query.union_idempotent `Bumbledb.Query.union_idempotent
    "A duplicated rule adds nothing: duplicate derivations, one answer — set semantics at the program level."
    "exec/sink.rs::seen (crates/bumbledb/src/exec/sink.rs)"
    "r22_union_read_round_trips (crates/bumbledb-query/tests/cookbook.rs)",

  .row @Query.answer_identity_canonical `Bumbledb.Query.answer_identity_canonical
    "Answer identity is the projected head tuple: two body environments with one projected tuple are one answer, which is why head-shaped dedup keys are complete."
    "crate::exec::sink::projection (crates/bumbledb/src/exec/sink/projection.rs)"
    "duplicate_witness_projection_dedups_and_skips_suffixes (crates/bumbledb/src/exec/sink/tests/projection.rs)",

  .row @Query.snapshot_single `Bumbledb.Query.snapshot_single
    "The denotation reads one instance: two instances agreeing on every mentioned relation answer identically."
    "Db::read (crates/bumbledb/src/api/db/read.rs); Snapshot::execute (crates/bumbledb/src/api/db/snapshot.rs)"
    "pinned_plan_reads_fresh_data_at_newer_generations (crates/bumbledb/src/api/prepared/tests/snapshot.rs)",

  .row @Query.eval_sound `Bumbledb.Query.eval_sound
    "The refinement theorem: list-backed evaluation over a concrete finite world equals the set denotation, under exactly the two premises the validator discharges — safety and the measure-free binding shape."
    "context.rs::check_atoms (crates/bumbledb/src/ir/validate/context.rs)"
    "fuzz/fuzz_targets/query.rs",

  /- ## PRD 05 — Aggregates -/

  .row @checkedSum_sound `Bumbledb.checkedSum_sound
    "A successful checked sum is the mathematical sum within bounds — an emitted Sum is exact, and overflow is a typed error, never a wrap."
    "finalize.rs::finalize_acc (crates/bumbledb/src/exec/sink/aggregate/finalize.rs)"
    "sum_of_durations_overflow_is_the_typed_overflow_error (crates/bumbledb/src/api/prepared/tests/measure.rs); sum_is_order_independent_near_the_boundary (crates/bumbledb/src/exec/sink/tests/semantics.rs)",

  .row @wide_accumulator_exact `Bumbledb.wide_accumulator_exact
    "The wide-accumulator argument: fewer than two-to-the-64 terms of 64-bit values cannot overflow the 128-bit accumulator, so the only narrowing point is finalization."
    "fold_row.rs::fold_scratch_row (crates/bumbledb/src/exec/sink/aggregate/fold_row.rs); finalize.rs::finalize_acc (crates/bumbledb/src/exec/sink/aggregate/finalize.rs)"
    "aggregate_leaf_batches_match_the_scalar_fold_at_the_boundary (crates/bumbledb/src/exec/sink/tests/aggregate.rs)",

  .row @pack_canonical `Bumbledb.pack_canonical
    "Pack output is canonical: sorted, pairwise-disjoint, non-adjacent — only a strict gap breaks a run."
    "interval/sweep.rs::sweep (crates/bumbledb/src/interval/sweep.rs)"
    "r18_pack_round_trips (crates/bumbledb-query/tests/cookbook.rs)",

  .row @pack_extensional `Bumbledb.pack_extensional
    "Packing changes the representation of the claim union, never its points."
    "interval/sweep.rs::sweep (crates/bumbledb/src/interval/sweep.rs)"
    "packed_output_matches_the_naive_point_set (crates/bumbledb/src/interval/sweep.rs)",

  .row @pack_adjacency `Bumbledb.pack_adjacency
    "Half-open adjacency continues a run: touching claims share no point yet leave no hole, so they coalesce into one segment."
    "interval/sweep.rs::sweep (crates/bumbledb/src/interval/sweep.rs)"
    "adjacency_continues_and_the_minimal_gap_breaks (crates/bumbledb/src/interval/sweep.rs)",

  .row @pack_lattice_closed `Bumbledb.pack_lattice_closed
    "Lattice closedness — the creation quarantine: every packed endpoint is selected from stored claims' endpoints; pack never invents a bound."
    "interval/sweep.rs::sweep (crates/bumbledb/src/interval/sweep.rs)"
    "packed_output_matches_the_naive_point_set (crates/bumbledb/src/interval/sweep.rs)",

  .row @Query.allen_jepd `Bumbledb.Query.allen_jepd
    "The thirteen basic Allen relations are jointly exhaustive and pairwise disjoint over nonempty half-open intervals — every pair satisfies exactly one."
    "crate::allen::classify (crates/bumbledb/src/allen.rs)"
    "classify_matches_the_point_set_oracle_jepd (crates/bumbledb/src/allen.rs)",

  .row @Query.allen_converse_involution `Bumbledb.Query.allen_converse_involution
    "Converse composed with converse is the identity on the basics — one bit-reversal in the palindromic mask order."
    "crate::allen::AllenMask::converse (crates/bumbledb/src/allen.rs)"
    "exhaustive_converse_involution_over_all_8192_masks (crates/bumbledb/src/allen.rs)",

  .row @Query.classify_swap `Bumbledb.Query.classify_swap
    "Classification dualizes under operand swap — what frees the executor to orient its Allen filters."
    "crate::allen::classify (crates/bumbledb/src/allen.rs)"
    "converse_is_an_involution_and_dualizes_classification (crates/bumbledb/src/allen.rs)",

  .row @Query.agg_over_distinct_bindings `Bumbledb.Query.agg_over_distinct_bindings
    "Every aggregate folds the distinct binding set of its group — no fold can observe a duplicate, set semantics through aggregation."
    "fold_row.rs::fold_scratch_row (crates/bumbledb/src/exec/sink/aggregate/fold_row.rs); exec/sink.rs::seen (crates/bumbledb/src/exec/sink.rs)"
    "dedup_constant_group_collapses_duplicates_before_folding (crates/bumbledb/src/exec/sink/tests/aggregate.rs); count_distinct_collapses_multiplicities_per_group (crates/bumbledb/src/exec/sink/tests/aggregate.rs)",

  .row @Query.empty_global_no_answer `Bumbledb.Query.empty_global_no_answer
    "An aggregate over the empty binding set yields the empty answer set — not a zero row; the SQL reading is refused."
    "finalize.rs::finalize_into (crates/bumbledb/src/exec/sink/aggregate/finalize.rs)"
    "global_aggregate_over_empty_input_yields_zero_rows (crates/bumbledb/src/exec/sink/tests/semantics.rs)",

  .row @Query.measure_fold_laws `Bumbledb.Query.measure_fold_laws
    "The measure column is poisoned exactly by a ray in the group — one unbounded interval makes the whole group's measure erroneous, never a value."
    "fold_row.rs::fold_scratch_row (crates/bumbledb/src/exec/sink/aggregate/fold_row.rs); crate::Error::MeasureOfRay (crates/bumbledb/src/error.rs)"
    "a_ray_reaching_duration_raises_and_a_filtered_query_succeeds (crates/bumbledb/src/api/prepared/tests/measure.rs)",

  .row @Query.argmax_ties_all_kept `Bumbledb.Query.argmax_ties_all_kept
    "Arg ties are set-honest: every extreme-attaining binding survives the restriction, and equal projected rows are one answer — this dedup is never elided."
    "fold_row.rs::fold_arg (crates/bumbledb/src/exec/sink/aggregate/fold_row.rs)"
    "arg_ties_keep_every_attaining_row_as_a_set (crates/bumbledb/src/exec/sink/tests/aggregate.rs); arg_ties_are_set_honest (crates/bumbledb/src/api/prepared/tests/aggregates.rs)",

  /- ## PRD 06 — the sweep -/

  .row @Exec.sweep_covered_sound_complete `Bumbledb.Exec.sweep_covered_sound_complete
    "THE witness-token theorem: under ordered-and-disjoint — precisely what the proof token attests — the one-pass coverage verdict equals the point-subset denotation (soundness needs no premise; completeness spends only order; disjointness licences the predecessor-seek entry below the fold)."
    "crate::schema::DisjointDeterminantProof (crates/bumbledb/src/schema.rs); Checker::check_coverage (crates/bumbledb/src/storage/commit/judgment.rs)"
    "coverage_verdict_matches_the_naive_subset_check (crates/bumbledb/src/interval/sweep.rs); pointwise_overlap_is_found_by_the_ordered_walk (crates/bumbledb/src/verify_store/tests.rs)",

  .row @Exec.sweep_early_exit_sound `Bumbledb.Exec.sweep_early_exit_sound
    "Once the frontier passes the window end the verdict is accept on any remaining input — the early return loses nothing."
    "interval/sweep.rs::sweep (crates/bumbledb/src/interval/sweep.rs)"
    "consumed_segments_are_handed_over_in_order_and_gaps_convict_first (crates/bumbledb/src/interval/sweep.rs)",

  .row @Exec.sweep_ignores_spent_segments `Bumbledb.Exec.sweep_ignores_spent_segments
    "A segment wholly at or before the frontier is a no-op, so the predecessor-seek entry skips only segments the fold would ignore anyway — the seam is mechanism, not semantics."
    "Checker::check_coverage (crates/bumbledb/src/storage/commit/judgment.rs)"
    "coverage_verdict_matches_the_naive_subset_check (crates/bumbledb/src/interval/sweep.rs)",

  .row @Exec.pack_is_the_sweep `Bumbledb.Exec.pack_is_the_sweep
    "One fold, two consumers: the run-emitting sweep with the sort pass IS the Pack spec function — the code-sharing claim, proved."
    "interval/sweep.rs::sweep (crates/bumbledb/src/interval/sweep.rs); finalize.rs::finalize_into (crates/bumbledb/src/exec/sink/aggregate/finalize.rs)"
    "packed_output_matches_the_naive_point_set (crates/bumbledb/src/interval/sweep.rs)",

  .row @Exec.ray_needs_ray `Bumbledb.Exec.ray_needs_ray
    "A source ray is covered only by a chain reaching a target ray — coverage to infinity, with infinity an ordinary largest end word."
    "Checker::check_coverage (crates/bumbledb/src/storage/commit/judgment.rs)"
    "rays_are_ordinary_largest_end_words (crates/bumbledb/src/interval/sweep.rs)",

  .row @Exec.adjacent_segments_cover `Bumbledb.Exec.adjacent_segments_cover
    "Touching segments cover across the seam: exact tiles leave no hole, so the walk accepts the composed window."
    "interval/sweep.rs::sweep (crates/bumbledb/src/interval/sweep.rs)"
    "adjacency_continues_and_the_minimal_gap_breaks (crates/bumbledb/src/interval/sweep.rs)",

  /- ## PRD 07 — dedup and the elision licences -/

  .row @Query.seenfold_is_set_semantics `Bumbledb.Query.seenfold_is_set_semantics
    "The seen-set IS set semantics: first-occurrence filtering of the emitted stream computes exactly the answer set — the sinks are where union lives."
    "exec/sink.rs::seen (crates/bumbledb/src/exec/sink.rs)"
    "duplicate_witness_projection_dedups_and_skips_suffixes (crates/bumbledb/src/exec/sink/tests/projection.rs)",

  .row @Query.distinct_witness_licence `Bumbledb.Query.distinct_witness_licence
    "The distinct-bindings licence: when every participating occurrence's bound fields cover a key, the key stream is already duplicate-free and the binding seen-set may be elided — single-rule only."
    "plan/fj/provably_distinct.rs::DistinctWitness (crates/bumbledb/src/plan/fj/provably_distinct.rs); AggregateSink::without_seen_set (crates/bumbledb/src/exec/sink/aggregate/new.rs)"
    "witnessed_elision_matches_the_seen_set_path (crates/bumbledb/src/exec/sink/tests/semantics.rs); elision_skips_binding_dedup_but_count_distinct_still_collapses (crates/bumbledb/src/api/prepared/tests/aggregates.rs)",

  .row @Query.disjoint_witness_licence `Bumbledb.Query.disjoint_witness_licence
    "The disjoint-arms licence: under pairwise arm disjointness, cross-rule dedup is a no-op — proved sound, and spent diagnostically only (the measured refutation keeps the spanning seen-set)."
    "plan/fj/provably_disjoint.rs::DisjointWitness (crates/bumbledb/src/plan/fj/provably_disjoint.rs)"
    "the_du_arm_union_proves_and_an_unselected_arm_unproves (crates/bumbledb/src/api/prepared/tests/disjoint.rs)",

  .row @Query.union_regime_head_projection `Bumbledb.Query.union_regime_head_projection
    "The multi-rule union regime keys the head projection, never a rule's full slot array — dedup keys must be rule-independent, and the head tuple is a complete key."
    "exec/sink.rs::union_spans (crates/bumbledb/src/exec/sink.rs)"
    "the_union_seen_set_keys_head_projections_across_rule_layouts (crates/bumbledb/src/exec/sink/tests/aggregate.rs); aggregates_fold_the_union_of_head_projected_bindings (crates/bumbledb/src/api/prepared/tests/rules.rs)",

  .row @Query.syntactic_disjointness_sound `Bumbledb.Query.syntactic_disjointness_sound
    "The syntactic disjointness check is sound — and conservatively incomplete by design: any pin it cannot compare refuses the witness."
    "plan/fj/provably_disjoint.rs::provably_disjoint_rules (crates/bumbledb/src/plan/fj/provably_disjoint.rs)"
    "the_du_arm_union_proves_and_an_unselected_arm_unproves (crates/bumbledb/src/api/prepared/tests/disjoint.rs)",

  /- ## PRD 08 — the rewrites -/

  .row @Query.grounding_preserves_answers `Bumbledb.Query.grounding_preserves_answers
    "Grounding is denotation-preserving partial evaluation: on any instance agreeing with the ground axioms, the folded contribution means exactly what the closed atom meant, and rule death is honest emptiness."
    "evaluate.rs::surviving_ids (crates/bumbledb/src/plan/ground/evaluate.rs); evaluate.rs::fold_positive (crates/bumbledb/src/plan/ground/evaluate.rs)"
    "fuzz/fuzz_targets/rewrites.rs",

  .row @Query.elimination_sound `Bumbledb.Query.elimination_sound
    "Under the elimination shape and the theory's containment, dropping the target atom preserves the rule's answers — existence rides the containment."
    "Role::Eliminated (crates/bumbledb/src/ir/normalize.rs)"
    "fuzz/fuzz_targets/rewrites.rs",

  .row @Query.keyprobe_equiv_join `Bumbledb.Query.keyprobe_equiv_join
    "Under the accepted shape and the key's uniqueness, the point-probe evaluation equals the join denotation — one get finds exactly the one deriving fact."
    "PreparedRule::KeyProbe (crates/bumbledb/src/api/prepared.rs); PreparedRule::KeyProbe (crates/bumbledb/src/api/prepared/build.rs)"
    "key_probe_fast_lane_hits_misses_and_type_errors (crates/bumbledb/src/api/prepared/tests/key_probe.rs); pointwise_key_point_lookup_uses_key_probe_and_is_image_free (crates/bumbledb/src/api/prepared/tests/key_probe.rs)",

  .row @Query.statically_empty_sound `Bumbledb.Query.statically_empty_sound
    "A statically refuted rule contributes the empty answer set on every instance — the verdict never consulted one."
    "Program::Empty (crates/bumbledb/src/api/prepared.rs); NormalizedQuery::dead (crates/bumbledb/src/ir/normalize.rs)"
    "an_all_dead_program_prepares_to_empty_and_binds_params_first (crates/bumbledb/src/api/prepared/tests/statically_empty.rs)",

  /- ## PRD 09 — the lifecycle -/

  .row @Txn.final_state_judgment_order_free `Bumbledb.Txn.final_state_judgment_order_free
    "Judgment is a function of the final state alone: any two op sequences with one final state receive one verdict — operation order is not representable in the judge's input."
    "judgment.rs::FinalStateView (crates/bumbledb/src/storage/commit/judgment.rs)"
    "delete_plus_insert_of_same_key_succeeds_in_either_user_order (crates/bumbledb/src/storage/commit/tests/apply.rs)",

  .row @Txn.committed_states_model `Bumbledb.Txn.committed_states_model
    "Every committed state models its theory — the free-lunches law: queries may assume every declared dependency of every committed state."
    "judgment.rs::judge (crates/bumbledb/src/storage/commit/judgment.rs); Db::verify_store (crates/bumbledb/src/verify_store.rs)"
    "clean_store_reports_nothing_and_counts_the_leak (crates/bumbledb/src/verify_store/tests.rs)",

  .row @Txn.rejection_is_complete `Bumbledb.Txn.rejection_is_complete
    "A rejection carries the complete violated-statement set — every violated statement, only violated statements, and at least one."
    "crate::error::Violations (crates/bumbledb/src/error.rs)"
    "fuzz/trophies/ops/multi-violation-citation-order",

  .row @Txn.witness_conflict_distinct `Bumbledb.Txn.witness_conflict_distinct
    "Witness conflicts are not dependency violations: the two failure kinds are distinct constructors, and the one generation compare aborts before anything is judged."
    "Error::CommitRejected (crates/bumbledb/src/error.rs); Error::GenerationMoved (crates/bumbledb/src/error.rs); write.rs::write_witnessed (crates/bumbledb/src/api/db/write.rs)"
    "the_interleaved_second_sequence_aborts_with_the_payload (crates/bumbledb-bench/src/differential/tests/witness.rs); a_noop_commit_between_read_and_write_does_not_abort (crates/bumbledb-bench/src/differential/tests/witness.rs)",

  .row @Txn.snapshot_reads_one_state `Bumbledb.Txn.snapshot_reads_one_state
    "Every read is a function of one committed state — snapshot isolation as a signature, with the generation tag invisible to reads."
    "api/db.rs::Snapshot (crates/bumbledb/src/api/db.rs); Db::read (crates/bumbledb/src/api/db/read.rs)"
    "pinned_plan_reads_fresh_data_at_newer_generations (crates/bumbledb/src/api/prepared/tests/snapshot.rs)",

  .row @Txn.derived_soundness_vs_freshness `Bumbledb.Txn.derived_soundness_vs_freshness
    "A containment-constrained derived relation is sound in every committed state; freshness is host witness-loop discipline, not a property any committed state can carry."
    "Db::write_from (crates/bumbledb/src/api/db/write.rs)"
    "r27_maintenance_rederives_after_generation_movement (crates/bumbledb-query/tests/cookbook.rs)",

  .row @Txn.etl_lands_valid `Bumbledb.Txn.etl_lands_valid
    "The ETL identity: a migration that lands is already valid — export under one generation, transform, bulk-judge as one final state (the identity round-trip theorem sits beside it)."
    "Snapshot::scan (crates/bumbledb/src/api/db/snapshot.rs); Db::bulk_load (crates/bumbledb/src/api/db/write.rs)"
    "r28_migration_is_etl (crates/bumbledb-query/tests/cookbook.rs)"

]

/-- The ledger count, asserted: a dropped or added row moves this
number, so the census (which re-derives the count by grep) and the
build (which checks this literal) both notice. -/
theorem ledger_count : ledger.length = 68 := rfl

end Bridge
end Bumbledb
