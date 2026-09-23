# Event proof coverage and implementation obligations

**Revision 0.8.** The fresh [readiness run](results/readiness/check.json) checks
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

## Contract-to-proof matrix

| Contract | Kernel-checked evidence | Remaining boundary |
| --- | --- | --- |
| Canonical completed region, Boolean/complement preservation | `Retraction.exact_identity`, `boolean_homomorphism`, `complement_homomorphism`; Anchored and EssentialCoordinates | Rust unique-table correctness, capacities, memory ownership, canonical per-fact bytes |
| Empty remains a fact; pointwise-key distinctness needs a witness | EventStorage's ten reports, including `complete_key_nonempty` and `empty_key_counterexample` | Native storage/admission/planner correspondence |
| Distinct facts + pointwise key + coverage give one branch per world | `FiniteMeasure.key_and_coverage`, `schema_partition_mass` | Application of the finite enumeration and whole-fact identity premises to native rows |
| Full/proper partitions preserve total/parent mass | `partition_mass`, `partial_partition_mass` | Arbitrary real-parameter source construction and solver certificates |
| Conditional complement, bounded numerator, evidence reuse and positive denominator | `conditional_complement`, `intersection_bound`, `repeated_evidence`, `zero_evidence`, `positive_evidence` | Exact arithmetic codec, polynomial/semialgebraic contraction, adapter implementation |
| Zero mass need not be empty; full mass need not be full | `zero_mass_can_be_possible`, `full_mass_need_not_be_full` | Source admission must retain those legal worlds |
| Checked relation support, roles and shared witnesses | LegalRelations, ScopedProduct, ViewProduct, QuerySeparator environment counterexample | Rust role/face validation and certificate extraction |
| May/All composition and the exact extra premise for Must | `ModalContract.may_composition`, `all_composition`, `must_composition_exact`; dead-end and composition counterexamples | Native IR preserves enabledness and the correct intermediate face |
| Both residual adjunctions | `ModalContract.left_adjunction`, `right_adjunction`; LegalRelations support-aware adjunction | Native operator lowering and canonical publication |
| Greatest uniform permissions, excluding empty observation cases | `permission_exact`, `permission_sound`, `permission_greatest`, `uniform_action_counterexample` | Complete actor information, action enabledness and memory are supplied modeling contracts |
| Finite-path closure and invariant/reachability unfolding | `star_transitive`, `star_least`, `star_may_unfold`, `star_all_unfold` | These hold even on infinite types; they do not prove native fixed-point termination or inevitability algorithms |
| Observable bounds, information order and FD rewrite premises | InformationReadout and ReadoutMaps | Native observation descriptors and complete information-case construction |
| Support image, projection-square completeness, gates and decoder repair | BaseChange, ProjectionGate, LocalCompletion, FusedProjection | Prove each native certificate corresponds to its exact inputs; a generic "safe map" bit is insufficient |
| Factored scalar joins and typed aggregates | FactorizedPack, ParticipatingValidation, RelationPack, QuerySeparator | Native IR checker/search, cost model, stable diagnostic descriptors and error-set integration |
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
python3 proposal/semantics/check.py --label my-readiness-check
python3 proposal/semantics/verify_ready.py
```

Choose a new label for each proof run; existing evidence is never overwritten.
The first command runs the 30 proof files serially and records exact reports,
source hashes, binary hash, logs and standard-axiom dependencies. `--only-new`
checks only the two new public-contract modules.

The second command verifies the implementation-readiness record, all active
proposal links/fences, fresh proof sources/logs, semantic reference results and
the retained lab artifact audit. It performs no build or benchmark. The
[current readiness record](results/implementation-ready.json) is the entry point
for that evidence. Earlier research-closeout hashes refer to the preserved
[revision 0.7 document snapshot](../archive/implementation-prep-0.7/manifest.json),
not to documents subsequently cleaned up in revision 0.8.
