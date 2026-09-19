# Event research handoff

**Historical research closeout, followed by revision 0.8 implementation preparation.**
The [current proposal](proposal.md), [proof matrix](semantics/README.md) and
[implementation plan](implementation-plan.md) now supply the implementation
baseline. The exact documents pinned by the original closeout hashes are in the
[revision 0.7 snapshot](archive/implementation-prep-0.7/manifest.json). The evidence
counts below describe that research closeout; the fresh combined Lean run has
246 reports, including 33 new public-contract reports.

The user has ended the open-ended research goal. The proposal and laboratory
are retained for implementation. This closes the research phase; it does not
claim a universally fastest representation or a production Event implementation.

Read the [design review](design-review.md) for the complete direction and the
[implementation plan](implementation-plan.md) for acceptance criteria. Historical
experiments remain available without becoming additional public APIs.

## The design to carry forward

**Store a condition with its world identity, and let the database construct
further conditions from it.** Probability is a later observation under an
explicit normalized joint law. Structural possibility remains distinct from
positive probability; an unmeasured context is meaningful.

The proposed row value is a sixteen-byte owner/region handle. The preferred
normal-form direction is a canonical completed function over legal worlds,
with symbolic splits and small tables over essential coordinates. The owner
supplies legal support, coordinates, checked maps and any designated law.
Complement changes the region polarity. Arena handles never become wire identity.

This is one public Event algebra with competing physical carriers. Dense tables
remain strong bounded controls; packed and symbolic carriers have different
strengths. The evidence does not justify making one carrier or coordinate layout
mandatory for every operation. Identity and exact semantics must survive changes
of working representation, with conversion costs included in comparisons.

The algebra includes containment, complement, Boolean construction, converse,
composition, domain/range, modalities, residuals, information abstraction and
finite closure under its stated finiteness contract. Relation composition retains
one shared middle world. Source parameters and environments remain shared.

The native dependency language supplies the schema meaning: pointwise keys forbid
overlap; containments prove coverage; a disjoint cover of full has probability
one under every admitted normalized law. No row-weight normalization field is
needed. TypeSafe judgments enter through a declared source/import contract; a
numeric probability alone does not specify the coupling between judgments.

Empty Events remain storable values. A present empty Event and an absent group
have different query meanings. Scalar constraints and owner validation still
apply to empty-valued facts. See [storage closure](event-storage.md).

## What the experiments established

The most useful performance result is that dependencies can eliminate work
before choosing a faster kernel. A proved scalar separator permits branch
reduction before expensive Event combinations. The actual native Free Join
executor performs the branch scans and summary join.

For the two relational programs, the valid rewrites are:

```text
union i,j: Compose(R[i], S[j])
  = Compose(union i R[i], union j S[j])

intersection i,j: LeftResidual(R[i], T[j])
  = LeftResidual(union i R[i], intersection j T[j])
```

The second program has an intersection aggregate; ordinary union Pack does not
license that rewrite. Both require their scalar factorization premise and
compatible checked relation roles. Participating inputs remain validated after
saturation. Unmatched invalid rows remain unevaluated. Experimental diagnostics
preserve fault sets, not first-error order.

| Evidence | Established scope |
| --- | --- |
| [Lean record](experiments/event-repr-lab/LEAN.md) | 195 central theorem reports; 126 axiom-free. The Pack work retains 18 additional axiom-free reports separately. These are denotational proofs, not Rust or machine-code refinement proofs. |
| [Relational Pack](experiments/event-repr-lab/RELATIONAL-PACK.md) | 11,904 native differential executions across six carriers; 12,288 separator-check cases. |
| [Main relational comparison](experiments/event-repr-lab/RELATIONAL-PACK-MEASUREMENTS.md) | 384 configurations with seven samples each; all 192 matched fresh schedule pairs improve. |
| [Independent repeat](experiments/event-repr-lab/RELATIONAL-PACK-REPEAT.md) | 64 configurations with eleven samples each; confirms the gains and the small warm-query crossover. |

In one repeated dense, width-five, bit-major, fanout-eight case with memoization:

| Query | Complete pairs, fresh | Factored branches, fresh |
| --- | ---: | ---: |
| Composition | 2.523 ms | 0.463 ms |
| Left-residual aggregate | 4.513 ms | 0.608 ms |

These are bounded fixtures, not whole-game performance guarantees. Small warm
queries can lose to summary construction. Factorization also reverses the
preferred packed layout in a measured composition workload. The planner must
cost a legal rewrite; algebraic equality alone does not prove speed.

Retained Event/memo estimates, root-capacity censuses and process RSS have
different scopes. None should be relabelled as a precise query peak-allocation
measurement. The [full laboratory report](experiments/event-repr-lab/REPORT.md)
retains the other carriers, counterexamples, losses and kernel evidence.

## What was shelved

The [participation-index draft](experiments/event-repr-lab/PARTICIPATION-STATUS.md)
would trade a repeated branch scan for fewer staged roots. Its release build
was cancelled at the user's stop request. No successful build, correctness
result or timing exists. Its source, draft harnesses and cancellation log are
preserved; its disposable engine/build directory was removed. It is excluded
from the accepted design evidence and is not a prerequisite for implementation.

## What remains production work

The tracked engine is unchanged. Native Event schema/macro integration,
admission, persistence, transaction ownership, planner integration and complete
source-law integration remain proposed work.

The next bounded milestone should establish owned canonical Event values through
native storage and pointwise dependencies: preserve empty/full, validate owners,
round-trip canonical identity across reopen and allocation changes, and prove
the empty-value planner behavior with native tests. Keep the carrier boundary
explicit and use the retained workload evidence to choose its initial backend.
The larger algebra remains the public design target throughout implementation.

Broader coordinate capacity, exact source-solver contracts, resource refusal,
concurrent publication, optimal separator discovery and production diagnostic
ordering retain explicit obligations in the implementation plan. No conclusion
here silently promotes a laboratory shortcut to an engine guarantee.

## Closeout verification

The [research closeout record](experiments/event-repr-lab/results/research-closeout.json)
pins this handoff and the updated entry points. The
[closeout audit](experiments/event-repr-lab/results/research-closeout-audit.json)
rechecks retained evidence, original production-source hashes, local links and
code fences without rerunning experiments. Earlier frozen audit records remain
historical evidence for their own snapshots.
