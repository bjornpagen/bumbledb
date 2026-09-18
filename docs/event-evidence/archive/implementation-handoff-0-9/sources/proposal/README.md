# Event proposal — implementation baseline

**Revision 0.9.** The proposal extends public BumbleDB **v1.3.1** with a first-class
`event` field and owned `Event` value. Implementation has started: the owned core
and unmeasured storage/binding slice exist on `codex/event-algebra`. The complete
proposal remains subject to explicit acceptance gates; it is not released.
The [native ledger](../docs/event-implementation.md) records what actually works.
The open-ended
representation research is closed. Its evidence and failed alternatives remain
preserved under the laboratory and archive.

An Event is a region of explicitly admissible worlds. Ordinary pointwise keys
and containments constrain those regions. Queries construct conditions,
relations, permissions and information bounds; a designated joint law supplies
probability observations. Empty Events remain values. Shared worlds and source
parameters remain shared through every operation.

The priority remains **native dependency-language fit, algebraic elegance, then
performance**. The initial general backend is a canonical completed function
with symbolic splits and essential-coordinate tables. Dense and packed carriers
remain independent semantic/performance controls. No universal speed ranking or
automatic adaptive backend is claimed.

## Implement from these documents

| Read | Purpose |
| --- | --- |
| [Proposal](proposal.md) | Current normative contract and explicit boundaries |
| [First principles](first-principles.md) | Facts, keys, containments and the interval analogy |
| [Representation](representation.md) | Initial backend, ownership, operation plans and persistence gate |
| [Schema surface](event-surface.md) | Proposed Rust fields, partitions and source construction |
| [Query algebra](query-algebra.md) | Staging, Event/Pack/Probability and constructive operators |
| [World relations](world-relations.md) / [operator catalog](event-algebra.md) | Typed faces, residuals, information and finite closure |
| [Source law](source-law.md) / [TypeSafe inference](typesafe-inference.md) | Exact observation and declared import meanings |
| [Storage closure](event-storage.md) | Empty values and the additional planner premises they require |
| [Proof coverage](semantics/README.md) | Lean results, theorem-to-contract mapping and unproved obligations |
| [Implementation plan](implementation-plan.md) | Ordered milestones, owning modules and acceptance gates |

`proposal.md`, the domain contracts linked above, and the decision register form
revision 0.9. Laboratory pages report experiments; they do not define additional
public APIs or override the contract. Syntax involving Event is proposed and
must not be presented as compiling in v1.3.1.

## Application and evidence

Coup remains the worked application: [schema](coup/schema.rs),
[query templates](coup/queries.rs), [advanced templates](coup/algebra-queries.rs),
[walkthrough](coup/query-walkthrough.md), and
[constructive applications](coup/algebra-applications.md). Game rules and actor
information are supplied contracts; a model forecast cannot establish either.

The [design review](design-review.md) explains the evidence behind the direction.
The [laboratory report](experiments/event-repr-lab/REPORT.md) retains alternative
carriers, native Free Join measurements, assembly and limitations. Its last
validated extension is [relational Pack](experiments/event-repr-lab/RELATIONAL-PACK.md).
The optional [participation-index draft](experiments/event-repr-lab/PARTICIPATION-STATUS.md)
remains shelved and unvalidated.

The [decision register](decisions.md), [law registry](laws.md), and
[research index](research/README.md) preserve mathematical provenance. Earlier
process/polytope designs are historical subtheories, not competing field types.
The [research handoff](RESEARCH-HANDOFF.md) records research closure. Exact
pre-cleanup documents are preserved in
[the revision 0.7 snapshot](archive/implementation-prep-0.7/manifest.json).

## Verification

The reproducible commands and their evidence scope are in
[proof coverage](semantics/README.md). The final readiness audit checks the whole
active proposal, fresh proof reports, retained experimental artifacts, semantic
reference checks and production-source isolation. It runs no performance sweep.

Revision 0.9 reconciles the plan with native progress and adds checked finite
fixed-point bounds and persistence-identity contracts. The exact revision 0.8
documents remain in [the baseline archive](archive/implementation-baseline-0.8/manifest.json).
Pointwise admission, the relation/query algebra, measured sources and integrated
qualification remain the work specified by the implementation plan.
