# Decisions, alternatives, and review rounds

Status: draft 0.2. These are proposed design decisions, not approved implementation work. All artifacts in this campaign stay in `proposal/`. The user has asked to develop and iterate the proposal before moving forward.

## The objective that changed the recommendation

The earlier research assembled useful results around interval bounds, a bilattice, t-norms, lower-bound semirings, and lineage evaluation. The first review recommended a practical interval-box representation. The user clarified that neither simplicity nor raw power was the deciding criterion: Allen was chosen for the beauty and unity of its algebra, and its machine implementation was a consequence of that choice.

The active proposal therefore asks whether one mathematical object can own the operations, their meanings, normal forms, and consequences. The candidate is finite credal information with convex choice. Interval boxes, evidence counts, lineage, and numeric kernels are evaluated against that choice rather than assembled into the definition of the type.

## Provisional decisions

| ID | Current proposal | Reason and consequence |
| --- | --- | --- |
| D01 | A model denotes a rational polytope of distributions on a finite labeled joint frame | One object represents precise, imprecise, deterministic-support, and coupled information |
| D02 | The hull is real; finite descriptions and operation coefficients are rational | Exact algebra and finite canonical descriptions without interpreting probabilities as a fixed grid |
| D03 | Combination means all compatible couplings; same-scope combination is intersection | Repeating an assertion is idempotent; joining marginals makes no independence assertion |
| D04 | Pooling and specified-weight mixture are distinct choice operations | Unresolved possibility differs from known chance; the convex-semilattice theory owns their interaction |
| D05 | Full payoff lower previsions are the dual observations | Even all subset-event intervals can fail to identify the model |
| D06 | Scope binding uses law-derived carriers and distinct variable identities | Fits source typing; prevents same-roster axes and equal forecasts from silently becoming one variable |
| D07 | A stored assessment is nonempty; operations can return explicit conflict | Intersection is total mathematically; invalid, absent, inconsistent, impossible, and unfinished computations remain distinguishable |
| D08 | Conditioning uses regular extension when upper evidence mass is positive | Fixed-likelihood conditioning remains exactly a rational polytope; asserting certainty is a different operation |
| D09 | Boolean event logic is evaluated within a joint model | Repeated events keep their identity; no generic t-norm-based model `and` is needed |
| D10 | Extreme distributions determine a canonical reference normal form | Storage and intermediate computation can choose other exact coordinates without changing equality |
| D11 | No implicit process sampling, shared-parameter bind, or model-valued recursion | Those operations require richer denotations or extra fixed-point theory; they are not representation optimizations |
| D12 | No default conversion from source confidence to width, evidence, or weights | Calibration and statistical constructors retain their own stated assumptions and provenance |

“Provisional” means the proposal takes a definite position so it can be criticized. It does not mean the implementation may choose a different denotation under the same names.

## Alternatives judged by algebra, rather than storage cost

### A. Finite credal information — leading candidate

The important assets are the complete choice equations, unique finite basis, scoped information laws, ordinary-constraint embedding, and payoff separation. Finite linear observations and constraints compose exactly. The cost is a semantic boundary around some nonlinear/shared-parameter processes. It also supplies no established finite qualitative composition table.

**Reject this candidate if:** repeated sampling with retained unknown parameters is central to the intended primitive, or only a finite complete relation-mask algebra would satisfy the Allen analogy.

### B. Arbitrary compact convex information

This contains the convex hull of the repeated shared-bias family. The information and choice geometry remains natural and finite bases no longer characterize all values. There is a second boundary: conditioning a compact convex prior can yield a nonclosed set. Taking the closure would preserve infima/suprema for linear observations but add unattained posterior distributions and alter exact membership/witness meaning.

**Choose this if:** the broader denotational universe and its topology are essential. First specify which sets are values, whether conditioning takes closure, which bounds are attained, and what constitutes an exact finite description or result. A symbolic formula is a representation proposal, not by itself an equality theory.

### C. A process algebra with named dependencies

This tracks when choices are made, which unknown parameters are reused, and which draws are conditionally independent. It can distinguish things that the one-outcome credal hull intentionally identifies. The local 2026 Liell-Cock–Staton paper is directly relevant, subject to its name-use restrictions.

**Choose this if:** the object should describe experiments/programs, including repeated draws, rather than only information about one joint outcome. The proposal would need a process signature, dependency typing, laws for composition and copying, and a map from process denotations to queried credal information. It may be a complementary type instead of a replacement.

### D. Random sets and belief functions

Intersection convolution and the commonality/Möbius transform provide a particularly beautiful algebra. Independent evidence combination becomes pointwise multiplication in alternate coordinates. It describes a different notion of combining assertions: duplication as new independent evidence generally changes the answer. Arbitrary credal sets are not all belief-function cores, so it is also a semantic restriction rather than just a different encoding.

**Choose this if:** independent evidence sources and their fusion are the primitive notion. Specify source identity, conflict mass, and whether normalization is ever implicit. Compare the meaning of repeating one source against combining two distinct sources with identical values.

### E. Boxes, evidence monoids, semirings, and lineage

These remain legitimate specialized algebras. Boxes are exact for particular constraint families and useful as declared outer approximations. Evidence monoids encode specified sampling models. Semirings give laws for annotation evaluation. Lineage preserves event identities through query derivations. None alone supplies the current candidate's full joint-outcome information semantics. The earlier research remains useful when a workload asks for one of these specific structures.

## Review round 1 — Make the denotation and closure explicit

**Work performed:** revisited the algebra follow-up, retained primary theorem evidence, the original brief, and source-level law/face/query typing and Allen execution. Defined the finite rational carrier, scope alignment, constructors, normal form, choice operations, information operations, and observation functions. Wrote the standalone proposal and law registry.

**Finding:** “Full credal set” was too vague. A finite-polytope object closes under the proposed linear operations and fixed-likelihood conditioning, but not under every operation suggested by a probabilistic-program interpretation.

**Revision:** made finite rational joint-outcome information the candidate; moved strong product outside the normative core; added the shared-bias counterexample and a genuine alternative-universe decision. Distinguished carrier, axis, context, and value identity. Defined conflict and zero-scope behavior explicitly.

## Review round 2 — Try to break the attractive equations

**Work performed:** constructed twelve explicit failure cases and wrote the exact specification checker. The checker uses rational linear elimination, vertex enumeration of bounded constraint systems, and an extended marginal formulation for combination. The initial run passed twelve test groups, including 216 choice-law triples with conflict cases, a hand-derived Fréchet joint model, scoped elimination, likelihood reweighting, the six-vertex posterior, robust preference, and the numerical/recursion falsifiers.

**Findings and resulting revisions:**

- Arbitrary projection pushdown can turn conflict into a feasible model. The registry permits only the scope-correct elimination law.
- Conditioning differs from certainty refinement and updates mixture weights. Both operations now have separate meanings and error behavior.
- All event intervals can agree while a payoff separates two models. Full payoff duality, not an event-bound table, is now the observation contract.
- The five obvious geometric comparison atoms fail exact relational-composition closure. The proposal explicitly leaves the strict finite Allen analogy unproved rather than presenting that classification as a relation algebra.
- Strong-product convexification loses a factorization promise needed for later refinements. Its name and semantics cannot be substituted for general conjunction or a process-level independence assertion.
- Arbitrary compact convex models also have a conditioning-closure issue. Added the three-outcome compact prior whose posterior is `[2/3,1)` on a binary event frame, so that alternative has an explicit topology decision.

These examples are proof-oriented falsifiers. The finite checks validate the stated small models; the infinite-extreme and noncompactness claims rely on the analytic arguments in the registry.

## Review round 3 — Fit the proposal to the actual language

**Work performed:** compared the proposed type to the source's flat structural value vocabulary, law-inferred carrier classes, nonempty face projections, fresh query variables, high-level alternatives elaboration, and pure-data query/recursion boundaries. Added concrete relational and query pseudocode, admission outcomes, canonical equality requirements, set-semantic aggregation rules, and representation alternatives.

**Findings and resulting revisions:**

- `Credal<Frame>` cannot be treated as an existing recursive `ValueType`, and normalization is not already enforced by containment. The draft now names a new numerical model law and leaves ABI/exposure as an adoption gate.
- The original brief says a probability scalar needs its own `ValueType`. A relational model view alone would be a separate exposure decision, not compliance with that scalar requirement by another name. The alternatives remain explicit until that decision is made.
- Existing faces reject empty projections. The mathematical zero-scope unit needs an explicit IR representation or checked special case rather than calling today's `on(..., [])`.
- Relational descriptions can differ while their convex models are equal. Generator IDs and provenance must be separate from extensional model identity; generator-row inclusion cannot implement model refinement.
- Set deduplication can destroy a probability sum if distinct outcome or component identities are projected away too early. The draft specifies keyed weighted mixtures and coordinate-preserving marginalization.
- A source point forecast is not an assertion that the world has that exact probability. Pooling, mixing, or adopting multiple forecasts as simultaneous constraints remains an explicit application choice.

**Additional verification:** the final pass added direct half-space checks of the conditioned model's pairwise ratio constraints and a map that turns a genuine extreme input into a redundant interior output generator. See the checker result recorded in the README.

**Final proof correction:** an irrational singleton is not sufficient to demonstrate the lack of a lattice meet when conflict is included: its only rational-polytope subset is empty. The registry instead uses decreasing intervals with intersection `[0,α]` for irrational `α`, which have no greatest rational-polytope lower bound. The scope review also requires computed quantities to receive fresh variable identities, retaining an input axis under the same identity only when it is copied unchanged.

## Open decisions with concrete settlement criteria

| Gate | Question | What would settle it |
| --- | --- | --- |
| G01 — Meaning | Is the desired primitive joint-outcome information or a sampling process? | Work through copy, distinct marginal binding, independent precise draws, repeated unknown bias, and later evidence in the intended application. If shared-bias reuse must be exact in this type, reject the finite-polytope universe. |
| G02 — Allen criterion | Is a complete information/choice algebra sufficient, or must there be a finite exact relation-mask calculus? | Explicitly choose the criterion. For the latter, supply an exhaustive atom family and prove converse/composition closure on its stated universe; X08 refutes the current five atoms. |
| G03 — First-class surface | Is the value exposed through a relational model view, an owned structural scalar, or both? | Draft one end-to-end schema, ChangeSet, query IR, image layout, and log round-trip for each option. Demonstrate law-derived carrier alignment and extensional equality, including two unequal descriptions of the same model. |
| G04 — Exact numbers | What public and wire representation carries arbitrary finite rational coefficients? | Specify constructor parsing, canonical normalization, overflow/resource behavior, ordering, and rational result export. Verify regrouping invariance and X09's failure cannot enter exact semantics. |
| G05 — Coherence laws | How do model normalization, feasibility, and optional refinement appear in schema statements? | Write the materialized law descriptors, affected-final-state judge semantics, type/runtime parity cases, and diagnostic examples. Do not relabel polytope inclusion as ordinary row containment. |
| G06 — Observation surface | Are regular extension, conflict-as-result, and typed missing/empty-fold handling the desired interface? | Review zero-lower/positive-upper evidence, impossible evidence, inconsistent prior, empty groups, and unused zero-weight conflict branches against examples. Denotations are specified; public result syntax is not. |
| G07 — Canonical execution | Which exact coordinates own persistence and which own query kernels? | Compare vertex, half-space, and expression forms on the same exact model corpus. Fix equality/certificate checking and storage ownership before selecting numerical solvers. |

These are bounded design tasks, not requests for generic approval. No performance target chooses G01 or G02. Representation work cannot repair a denotation that has already lost dependency information.

## Validation status and limits

The current `spec-checks.py` passes all twelve reported groups using exact rational arithmetic. It does not import engine code, run the Rust/TypeScript test suites, benchmark solvers, or verify emitted assembly. Those would test implementation work that this proposal has deliberately not begun.

P02 gives a constructive rational specialization of the real-weight presentation theorem. P03 and P06 give direct proofs of scoped gluing and conditioning closure. The proofs are written for review, not formally verified. Arbitrary compact-set and process alternatives have no reference implementation here.

The proposal is ready for substantive mathematical and language review. The process boundary, strict Allen criterion, and first-class surface remain adoption decisions; they have not been disguised as routine implementation details.
