# Current design decisions

Revision 0.7 retains the 0.5 relational algebra and reopens physical representation choices through the Rust laboratory. These are
proposal decisions, not repository-wide instructions or implemented engine behavior.

| ID | Decision | Consequence |
| --- | --- | --- |
| E1 | Public field `event`, owned value `Event` | TypeSafe Noul remains the name of its numeric provider primitive |
| E2 | Event denotes a scoped region of explicitly admissible worlds | Zero probability does not imply impossibility; equality is not almost-sure equality |
| E3 | Measurement is optional; a measured space designates one normalized joint law/family | Structural products need no invented coupling; no per-row weights or numeric normalization capacity |
| E4 | Pointwise keys reject overlap; INDs require union coverage | Partitions of full normalize by theorem; proper parents retain partial mass |
| E5 | Outcomes and rosters use existing relations/containments | No user-written outcome generic or model parameter on the field |
| E6 | Source allocation, copying, and a fresh draw are distinct | Repeated draws can share parameters without reusing an outcome |
| E7 | Unmarked cross-source dependence remains unknown | Source constructors must supply a joint law or admitted coupling family |
| E8 | Event includes empty/full through queries, transport and storage | Empty constructions retain their rows; pointwise-key distinctness requires separate nonemptiness evidence; see [storage closure](event-storage.md) |
| E9 | Conditioning preserves evidence mass and its positive domain | Impossible evidence, missing data, and source failure remain distinct |
| R1 | Resident EventKey is two u64 words | Scope identity and oriented partition ID fit two ordinary binding columns |
| R2 | One exact canonical resident manager per aligned space; its carrier is under experiment | Finite sets, runs, anchored and packed diagrams compete with the previous BDD pair |
| R3 | Keep a canonical oriented representative of a support-relative partition | Two roots are one implementation; finite normalization and anchored roots also give one-bit complement |
| R4 | BoolOp4 supplies all sixteen binary Boolean operations | One Apply engine; specialized truth-bitplane kernels share its semantics |
| R5 | A four-bit Venn occupancy signature decides emptiness of every binary Boolean expression | Operand symmetry becomes bit permutation; the [derived table](experiments/event-repr-lab/SIGNATURE-CALCULUS.md) is a sound composition envelope, provably not strong composition for finite Event scopes |
| R6 | Compile parameter predicates to constrained logical guards | Infinite source domains have finite event descriptions; guards carry no random mass |
| R7 | Keep law diagrams/exact scalar-function arenas beside event diagrams | Weighted contraction preserves correlated and shared-parameter sources |
| R8 | Freeze published order/indices; use owner-retained slabs | Reorder/rebase through explicit maps; no stale handles or per-binding Arc churn |
| R9 | Canonical wire bytes are rebuilt from semantic coordinates | Resident allocation order and content hashes alone do not define fact identity |
| R10 | Dense bitmaps may be the canonical resident carrier for materializable spaces | Mixed-carrier publication, deterministic storage and conversion costs require separate evidence |
| R11 | Source confidence is retained only as reported metadata | It is not an invented prior, interval width, or independence assertion |
| R12 | Finite structural fixed points are explicit sealed stages; no implicit causal intervention | Exact reachability without redefining ordinary recursive rules or claiming stochastic eventuality |
| A1 | World relations are Event regions with owned input/output faces | One carrier supports predicates, legal transitions, information, and higher-arity constraints |
| A2 | Composition is checked lifting, conjunction, and existential elimination | Preserve shared parameters; never identify a logical product with independent sampling |
| A3 | Domain, converse, modalities, and residuals are first-class operations | Construct preconditions and admissible behaviors instead of only testing supplied formulas |
| A4 | All is the vacuous universal box; Must adds enabledness | All composes on partial relations; Must need not, because dead intermediate branches matter |
| A5 | Star and monotone fixed points require a stable finite state quotient | Arbitrary continuous transformations and infinite-horizon probability require separate semantics |
| A6 | Information abstraction uses admitted observation partitions | Uniform actions must be chosen per visible case, with hidden-state quantification inside that choice |
| A7 | Observables are ordinary value/event partitions | Derived certainty, expectation, and cardinality use FD/IND coverage; no uncertain-number generic |
| A8 | Model import declares new judgment, forecast, constraint, revision, or likelihood intent | A posterior Noul cannot silently become a likelihood or an independent coin |
| A9 | Expose owned graph, face, map, witness, and contraction interfaces | Query, admission, sources, and external compilers reuse the same mathematical objects |

## Reconciliation with earlier research

Finite credal polytopes remain useful source constraints, but do not cover every
repeated shared-parameter experiment. Polynomial sampling remains a generative
core; guarded conditioning and constraints need the semialgebraic envelope.
The [retained process theory](research/process-theory.md) and [law registry](laws.md)
record that mathematics. A process's law equality is not the identity of one
particular event that can be intersected with another.

The earlier pair/BDD candidate establishes support equality, complement and query
emptiness, but the [Rust experiments](experiments/event-repr-lab/REPORT.md) show
that those properties do not uniquely choose that physical layout. It does not claim a
universally compact diagram, a constant-time weighted inference algorithm, or
an implemented real quantifier-elimination solver. The alternative representation
comparison and workload experiments are part of the design, not an unspecified
future choice of mathematical type.

The [implementation plan](implementation-plan.md) lists the remaining native work
and its acceptance criteria. [Research notes](research/representation-notes.md)
distinguish inspected source, derived invariants, executable probes, and empirical
choices. Older competing recommendations are behind the [history boundary](research/README.md).
