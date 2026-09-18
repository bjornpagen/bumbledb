# Event proof coverage and implementation obligations

**Revision 0.9.** The [proposal proof rerun](results/portable-proposal/check.json) checks
**246 theorem reports across 30 files; 160 reports use no axioms**. The remaining
reports use only Lean's standard `propext`, `Quot.sound` and/or `Classical.choice`.
The checker rejects missing reports, failed elaboration and other axioms,
including `sorryAx`. These are kernel-checked denotational results, not a proof
of the Rust engine, source solver, arena interner or generated machine code.

The totals comprise the unchanged 195-report central laboratory suite, its
18 separately retained Pack/separator reports, and 33 new public-contract
reports. The new set contains 15 modal/permission/closure reports and 18
finite-measure reports (including finite-sum lemmas); 16 are axiom-free.
The pinned installed binary is Lean 4.32.0 and its hash is recorded. No toolchain
or package was downloaded. Each run snapshots the exact sources and checker.
Failed elaboration attempts are retained separately and are not accepted evidence.

The native semantic modules add **255 checked reports across fifteen files** in
[the current native run](../../docs/event-evidence/native-query-map-semantics/check.json):
[Persistence](../../crates/bumbledb-event/semantics/Persistence.lean) and
[FixedPoint](../../crates/bumbledb-event/semantics/FixedPoint.lean), plus
[Admission](../../crates/bumbledb-event/semantics/Admission.lean) and
[CoordinateMaps](../../crates/bumbledb-event/semantics/CoordinateMaps.lean), plus
[Relations](../../crates/bumbledb-event/semantics/Relations.lean),
[Inspection](../../crates/bumbledb-event/semantics/Inspection.lean) and
[Information](../../crates/bumbledb-event/semantics/Information.lean),
[Programs](../../crates/bumbledb-event/semantics/Programs.lean) and
[Closure](../../crates/bumbledb-event/semantics/Closure.lean) and
[Partitions](../../crates/bumbledb-event/semantics/Partitions.lean) and
[Actions](../../crates/bumbledb-event/semantics/Actions.lean), plus
[Query](../../crates/bumbledb-event/semantics/Query.lean) and
[Pack](../../crates/bumbledb-event/semantics/Pack.lean) and
[Descriptors](../../crates/bumbledb-event/semantics/Descriptors.lean) and
[QueryMaps](../../crates/bumbledb-event/semantics/QueryMaps.lean). These are
additional denotational contracts, not an automatic proof of Rust because the
files live beside its source. The original readiness evidence remains unchanged.

## Contract-to-proof matrix

| Contract | Kernel-checked evidence | Remaining boundary |
| --- | --- | --- |
| Canonical completed region, Boolean/complement preservation | `Retraction.exact_identity`, `boolean_homomorphism`, `complement_homomorphism`; Anchored and EssentialCoordinates | Rust unique-table correctness, capacities and memory ownership |
| Persistent identity and registered runtime equality | `Persistence.exact_identity`, `legal_decoder_erased`, `registered_key_identity` | Source/coordinate descriptors instantiate a fixed universe; Rust canonical graph/byte correctness and registry admission are obligations, not assumed proved by naming them |
| Portable finite maps/roles and checked descriptor import | Descriptors: legal-code reconstruction, preservation of image/pullback/surjectivity, full products and orientation; support, joint-fibre and environment counterexamples | BEDC/BEVT parsing, canonical payload extraction and native constructors must establish the premises; no parser or resource refinement is claimed |
| Empty remains a fact; pointwise-key distinctness needs a witness | EventStorage's ten reports, including `complete_key_nonempty` and `empty_key_counterexample` | Native storage/admission/planner correspondence |
| Streaming coverage, conflict, order independence and exact citation membership | `Admission.coverage_exact`, `conflict_exact`, `key_iff_no_conflicts`, `containment_iff_union`, `order_independent`, `citation_exact`; duplicate/deletion counterexamples | Native rows must first satisfy canonical whole-fact set semantics and checked common contexts; scratch storage, graph operations and cancellation are tested rather than kernel-verified |
| Contextual full gives scalar uniqueness and actual target presence in nonempty spaces | Native Admission's `full_key_iff_scalar_unique`, `full_full_iff_presence`, absence refusal and empty-world counterexamples | Native source/decoder admission must establish nonempty support; no Rust refinement is claimed |
| Distinct facts + pointwise key + coverage give one branch per world | `FiniteMeasure.key_and_coverage`, `schema_partition_mass` | Application of the finite enumeration and whole-fact identity premises to native rows |
| Full/proper partitions preserve total/parent mass | `partition_mass`, `partial_partition_mass` | Arbitrary real-parameter source construction and solver certificates |
| Conditional complement, bounded numerator, evidence reuse and positive denominator | `conditional_complement`, `intersection_bound`, `repeated_evidence`, `zero_evidence`, `positive_evidence` | Exact arithmetic codec, polynomial/semialgebraic contraction, adapter implementation |
| Zero mass need not be empty; full mass need not be full | `zero_mass_can_be_possible`, `full_mass_need_not_be_full` | Source admission must retain those legal worlds |
| Checked relation support, roles and shared witnesses | LegalRelations, ScopedProduct, ViewProduct, QuerySeparator environment counterexample | Rust role/face validation and certificate extraction |
| Native finite products, joint-fibre certificates, descent and residual lowering | Relations: `product_completion_gates`, `joint_onto_iff_complete`, `descent_iff_membership_fd`, `shared_product_exact`, both staged residual laws and adjunctions | Native builders and reindexing instantiate already-admitted endpoint/environment meanings; Rust construction, transport and solver-backed parameters remain separate obligations |
| Total functional graphs and recovered readout coordinates | `graph_total_iff_environment_preserved`, `bit_conflicts_iff_nonfunctional`, `graph_bit_readout_exact` | The native Boolean code must embed legal target worlds injectively; totality/support admission and graph operations are tested rather than kernel-verified |
| May/All composition and the exact extra premise for Must | `ModalContract.may_composition`, `all_composition`, `must_composition_exact`; dead-end and composition counterexamples | Native IR preserves enabledness and the correct intermediate face |
| Both residual adjunctions | `ModalContract.left_adjunction`, `right_adjunction`; LegalRelations support-aware adjunction | Native operator lowering and canonical publication |
| Greatest uniform permissions, excluding empty observation cases | `permission_exact`, `permission_sound`, `permission_greatest`, `uniform_action_counterexample` | Complete actor information, action enabledness and memory are supplied modeling contracts |
| Complete-binding Event heads and deterministic context faults | Query: participation, ignored branches, empty closure, cardinality, traversal invariance and complete-report laws | Rust must preserve binding membership, syntactic operand identities, canonical context/value bytes and every written-rule stamp; typed relation query integration remains separate |
| Captured readout queries with distinct input/output contexts | QueryMaps: whole-program congruence from demanded inputs, occurrence-indexed demand preservation, no faults iff admitted, map denotations, possible/guaranteed bounds and absent-fibre distinction | Rust shape inference must produce the indexed scopes; BEDC reconstruction, canonical marker comparison and owner alignment must instantiate the reference; no checker extraction is claimed |
| Native grouped Event Pack | Pack: union/presence laws, canonical minimum and fault invariance, `clear_iff_common_context`, provenance/saturation counterexamples and partial-key participation | Rust BEVT descriptor order, exact scratch keys, registry alignment, rule stamps and computed-key extraction must instantiate the reference; no Rust refinement or factoring license follows |
| Controlled actions and fully observed strategy witnesses | Actions: `flattened_must_then_image`, `ranked_policy_domain`, `decreasing_rank_guarantees_every_policy_run`, `policy_restriction_retains_progress`, enabled safety and greatest-permission laws | Native role checking, retained layers and policy construction must instantiate these references; actor visibility and belief-memory construction are not inferred |
| Finite-path closure and invariant/reachability unfolding | `star_transitive`, `star_least`, `star_may_unfold`, `star_all_unfold` | These hold even on infinite types; they do not prove native fixed-point termination or inevitability algorithms |
| Finite least/greatest iteration and uniform parameter bound | `FixedPoint.strict_rank`, `stabilizes`, `least_fixed_point`, `greatest_fixed_point`, `shared_environment_stabilizes`; environment-mixing and negation counterexamples | Exhaustive legal roster, fixed presentation, fibre preservation and monotone operator must be certified; native evaluator/resource handling must refine the bounded reference |
| Finite native carrier and exact early stopping | FixedPoint: `legal_roster_complete`, `legal_roster_counts_original_support`, `detection_sound`, `detection_complete`, detected least/greatest extremality and `finite_detection_completes` | Native support count and aligned canonical equality instantiate explicit reference premises; no runtime enumeration is required, and no continuous-source certificate follows |
| First-entry rank layers | FixedPoint: `ascending_between`, `first_entry_unique`, `first_entries_cover_iterate` | Native difference construction and retained Vec order must refine the reference; layer budgets and owned partition admission remain tested |
| Typed program variance and monotone admission | Programs: `binary_sound`, `ternary_sound`, `monotone_transports_variance`, `program_variance_sound`, `sealed_program_is_monotone` | Rust packed truth functions, checked DAG indices, pruning and iterative execution must refine the typed-expression reference; conservative refusal need not mean mathematical nonmonotonicity |
| Exact denotations of native closure helpers | Closure: `least_closure_program_is_path`, `least_reach_program_is_reach`, `greatest_safe_program_is_safe`, `least_force_program_is_forcing`, `paths_preserve_environment`; avoiding-cycle counterexample | Fixed-point extremality, typed role checks and modal kernel correspondence instantiate these premises; stochastic eventuality, fairness and strategy synthesis are not supplied |
| Native readout saturation, evidence cells, FD factors and relation kernels | Information: `image_pullback_exact`, `evidence_guarantee_exact`, `three_cases_partition`, `descended_bits_form_legal_factor`, `onto_factor_unique`, `kernel_product_gate_iff_bitwise_fd` | Exact maps, legal Boolean codes and environment admission must instantiate the reference; actor information adequacy, source parameters and Rust kernels remain separate obligations |
| Observable bounds, information order and FD rewrite premises | InformationReadout and ReadoutMaps | Native finite readout/factor kernels are tested rather than kernel-verified refinements; actor visibility and shared-parameter correspondence remain separate |
| Finite value/count partition admission and transformations | Partitions: `clipped_admission_exact`, grouping/refinement/pullback preservation, `readout_bits_exact`, `count_roster_forms_full_partition`, duplicate-position and image-overlap cases | Ordinary scalar grouping/row identity remain caller/query obligations; Rust clipping, arrays, code validation and descending ITE updates must refine the reference |
| Support image, projection-square completeness, gates and decoder repair | BaseChange, ProjectionGate, LocalCompletion, FusedProjection | Prove each native certificate corresponds to its exact inputs; a generic "safe map" bit is insufficient |
| Checked maps, image adjunctions, substitution and abstraction after last use | Native CoordinateMaps: `support_check_exact`, `image_left_adjoint`, `universal_right_adjoint`, `substitution_exact`, `recursive_image_exact`, `recursive_image_early_abstraction`; unsupported-target/alias/absent-fibre counterexamples | Rust readout validation, graph substitution, suffix dependency extraction, arena locking and memo/resource behavior must implement the reference; no Rust refinement is claimed |
| Factored scalar joins and typed aggregates | FactorizedPack, ParticipatingValidation, RelationPack, QuerySeparator | Native IR checker/search, cost model, stable diagnostic descriptors and error-set integration |
| Safe raw graph traversal and support-preserving reconstruction | Inspection: `read_exact`, `graph_exact`, `root_exact`, `two_root_reconstruction`, `retraction_rebuild_preserves_membership`; alias and sign laws | Rust snapshot capture/indexing, packed leaf words and reconstruction must satisfy explicit constructor correspondence; ownership, locks and resource refusal remain native tests |
| Fifteen pair signatures | EventSignatures; separate finite signature-calculus checks | Strong finite-scope composition is disproved; associativity of the pruning table and the saturated-count theorem remain finite-check/derived evidence rather than Lean theorems |

The public modal module uses already-admitted legal state types at one fixed
source environment. In an indexed space, instantiate those types with the legal
fibres at that environment. It cannot justify erasing the environment, dropping
support gates or assuming a role; the separate support/role proofs supply those
premises.

The finite-measure module represents nonnegative rational finite laws with
integer masses and a positive common total. Its `Law` requires an exhaustive,
duplicate-free world roster. Observations retain an exact numerator/denominator
pair; the pair is not claimed to be a canonical arithmetic encoding. Applying
these laws to general source families is pointwise in the same retained parameter.
The module does not prove general parameter elimination, integration, bound
attainment, source feasibility or Jeffrey/Pearl updates.

## New semantic conclusions

The exact Must rewrite is:

```text
Must(R, Must(Q,E))
  = Must(Compose(R,Q),E) & All(R,Domain(Q))
```

Every intermediate R branch needs a Q continuation. A dead branch disappears
from composition, so dropping that enabledness condition changes the result.

For information relation `I: Observation→State` and `Good: State→Action`:

```text
Permitted = Domain(I) restricted LeftResidual(Converse(I),Good)
```

This is the largest sound permission relation on inhabited information cases.
`Good` must already include action enabledness and the required outcome property.
It is not a probability policy or a strategy that observes hidden state.

The schema partition proof uses distinct **fact identities**, not distinct
Event roots. Empty-valued facts can share the same empty region while contributing
no mass. Full coverage proves total mass; proper-parent coverage proves exactly
the parent's mass. Numeric normalization remains a source-constructor obligation.

## Reproduce the final checks

From the repository root, with the installed pinned toolchain:

```sh
python3 proposal/semantics/check.py --label my-readiness-check --lean /path/to/lean-4.32.0
python3 scripts/check-event-semantics.py \
  --lean /path/to/lean-4.32.0 \
  --output docs/event-evidence/my-native-check
python3 scripts/check-event-readiness.py \
  --proposal-proof proposal/semantics/results/my-readiness-check \
  --native-proof docs/event-evidence/my-native-check \
  --output docs/event-evidence/my-handoff-check
```

Replace `/path/to/lean-4.32.0` with the installed compiler executable. Both runners
require Lean 4.32.0 and record the executable hash; no compiler is downloaded.
Choose a new label for each proof run; existing evidence is never overwritten.
The first command runs the 30 proof files serially and records exact reports,
source hashes, binary hash, logs and standard-axiom dependencies. `--only-new`
checks only the two new public-contract modules.

The second command checks the native denotational modules. The third verifies
current links/fences, exact proof snapshots/logs and retained semantic reference
results, preserving revision 0.8 in its archive. It performs no native build or
benchmark and cannot pass an implementation milestone by itself.

`verify_ready.py` and [its readiness record](results/implementation-ready.json)
are historical evidence for revision 0.8 **before native edits**. Its document
hashes and production-isolation checks are expected to differ now; do not rewrite
them or present that command as a current integration check. The earlier
[revision 0.7 snapshot](../archive/implementation-prep-0.7/manifest.json) and the
[revision 0.8 snapshot](../archive/implementation-baseline-0.8/manifest.json)
preserve both boundaries.
