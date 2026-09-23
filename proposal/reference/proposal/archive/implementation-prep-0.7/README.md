# Event: an algebra of possibility, represented concretely

**Research closed at the user's request.** Start with the
[research handoff](RESEARCH-HANDOFF.md) for the design conclusions, verified
evidence and production work remaining. The optional participation-index draft
is shelved and unvalidated. This proposal does not add Event to the production engine.

**Revision 0.7: representation experiments; revision 0.5 algebra retained.**
Events and relations share a canonical region contract. The earlier BDD-only
implementation choice is reopened by the [Rust laboratory](experiments/event-repr-lab/README.md)
and its [measured findings](experiments/event-repr-lab/REPORT.md).
The algebra includes composition, converse, domain, may/must, residuals,
information abstraction, and finite reachability. Its closest program-algebra
lineage is Kleene algebra with domain. Boolean combinations are one fragment.
The resident key remains sixteen bytes; complement flips one bit.

Admissibility is explicit and distinct from probability. A measured context
designates a normalized joint law; an unmeasured structural context supports
logical reasoning without inventing a probability model. Model probabilities
of zero do not silently erase structurally possible worlds.

The existing FD/IND meanings do the important work: keys forbid overlap, mirrors
prove coverage, and a partition of full therefore totals probability one.
The decision order is native dependency-language synergy, algebraic elegance,
then performance. Timing chooses among implementations that preserve that fit.
Coup is the active application. Earlier competing type recommendations are
research history, not additional public APIs.

## Read in this order

| Document | What it settles |
| --- | --- |
| [Research handoff](RESEARCH-HANDOFF.md) | Frozen conclusions, evidence boundaries, shelved work and the next implementation milestone |
| [Current design review](design-review.md) | The selected mathematical contract, concrete memory direction, measured alternatives and remaining production obligations |
| [Storage closure](event-storage.md) | Empty Events remain values through storage; precise pointwise-key and planner consequences |
| [proposal.md](proposal.md) | Semantic invariants and the earlier concrete layout candidate |
| [Rust experiments](experiments/event-repr-lab/REPORT.md) | Competing canonical carriers through the actual Free Join executor |
| [Factored Pack](experiments/event-repr-lab/FACTORIZED-PACK.md) | A join dependency licenses branch reduction; native measurements, group presence and participating validation |
| [Relational Pack](experiments/event-repr-lab/RELATIONAL-PACK.md) | Native composition and residual aggregation, a normalized-query separator check, and retained shared environments |
| [world-relations.md](world-relations.md) | The larger algebra: typed faces, domain, modalities, composition, residuals, and finite closure |
| [event-algebra.md](event-algebra.md) | Operator contracts, observable partitions, information abstraction, expectation, and explanations |
| [typesafe-inference.md](typesafe-inference.md) | Preserve judgments; distinguish conditional forecasts, posterior revisions, likelihoods, and constraints |
| [representation.md](representation.md) | Competing resident layouts, canonical equality, owners, and candidate wire format |
| [Packed representation](experiments/event-repr-lab/PACKED.md) | Ordered symbolic prefixes, truth-table terminals, and exact invariants |
| [Selector blocks](experiments/event-repr-lab/BLOCKS.md) | Word-sized selector partitions and coordinate groups that survive face permutations |
| [Essential-coordinate tables](experiments/event-repr-lab/ESSENTIAL.md) | A canonical cut derived from Boolean dependence; complete competing carriers and matched algorithm evidence |
| [Dependency-directed projection](experiments/event-repr-lab/PROJECTION-GATES.md) | Membership FDs and joint coverage remove support gates; measured 36-fold reduction in the difficult readout |
| [Local completion](experiments/event-repr-lab/LOCAL-COMPLETION.md) | Eight checked laws for preserving untouched faces; separate locality and traversal controls |
| [Direct completed projection](experiments/event-repr-lab/FUSED-PROJECTION.md) | Eight checked laws, 81 comparative processes and a repeated result favoring staged execution |
| [Completion kernels](experiments/event-repr-lab/TERNARY-COMPLETION.md) | Same Event normal form; distinguish direct ternary construction, live answer graphs and retained intermediates |
| [Relationship calculus](experiments/event-repr-lab/SIGNATURE-CALCULUS.md) | Fifteen exact signatures, a derived composition envelope, and the proved finite-scope boundary |
| [Word readouts](experiments/event-repr-lab/WORD-CLASSIFIER.md) | Borrowed/aligned local tables, matched native classification, and the limits of the measured gains |
| [Bounded scratch](experiments/event-repr-lab/SCRATCH.md) | Reusable local alignment buffers with unchanged resident identity and readout contract |
| [Borrowed products](experiments/event-repr-lab/VIEW-PRODUCT.md) | Measured mapped operands: preserve shared witnesses, compare local readout and elimination order |
| [Views and working order](experiments/event-repr-lab/VIEW-ORDER.md) | 292 passing native processes; stable identity permits different contraction schedules |
| [Residual normalization and reuse](experiments/event-repr-lab/REUSE.md) | Separate exact source cofactors from temporary aligned-plane caching in native Free Join |
| [Supported dependence](experiments/event-repr-lab/SUPPORTED-DEPENDENCE.md) | Logical dependencies need not have a least coordinate set; legal domains differ from their bit encodings |
| [Scoped contraction](experiments/event-repr-lab/SCOPED-PRODUCT.md) | Exact support gates extend mapped products to arbitrary legal worlds |
| [Legal relation roles](experiments/event-repr-lab/LEGAL-RELATIONS.md) | Checked support/roles, 61-coordinate symbolic tests, and 106 passing native processes |
| [Factorwise retraction candidate](experiments/event-repr-lab/FIBRE-RETRACTION.md) | Twenty-four decoder laws, 275 retained native processes across revisions, prefix FD reflection and measured layout-dependent gains |
| [Information algebra](experiments/event-repr-lab/INFORMATION-ALGEBRA.md) | Possible and Guaranteed as exact observable bounds; FD conditions for moving filters and ordering observations |
| [Information readout experiment](experiments/event-repr-lab/PREFIX-READOUTS.md) | Constructive readouts after actual Free Join; partial-projection gains and exposed fallback costs |
| [Counting through membership dependencies](experiments/event-repr-lab/FACTOR-COUNTS.md) | Proved legal-fibre elimination, identical resident Events, and a separate contract for source laws |
| [Lean proofs](experiments/event-repr-lab/LEAN.md) | Signature laws and limits, anchored identity, essential coordinates, map preservation, and the exact joint-image contract |
| [Layout experiments](experiments/event-repr-lab/LAYOUT-MEASUREMENTS.md) | Same queries under three coordinate orders, with fresh/warm/memory results |
| [Map kernel experiment](experiments/event-repr-lab/MAP-MEASUREMENTS.md) | Same-binary recursive rebuilding versus exact table-axis permutation |
| [Native dependency review](experiments/event-repr-lab/DEPENDENCIES.md) | Exact target keys, fact identity, coverage, conflicts and update summaries |
| [Exact observation experiment](experiments/event-repr-lab/OBSERVATION.md) | Shared parameters, retained coupling, coefficient folds and explicit zero evidence |
| [Transport and ownership](experiments/event-repr-lab/TRANSPORT.md) | Shared structural views, exact rebasing, retained owners and native query replay |
| [Owned query results](experiments/event-repr-lab/OWNED.md) | Actual-arena computation, novel publication and retained output lifetimes |
| [Symbolic identity](experiments/event-repr-lab/DIAGONAL.md) | Coordinate equalities, checked products and large symbolic relation programs |
| [Map proofs](research/space-maps.md) | Support preservation, quantifier base change and law preservation have distinct obligations |
| [Representation research](research/representation-search.md) | Five additional papers and their qualified implications |
| [Decomposition research](research/block-decomposition.md) | AOMDD canonicality, exact weighted normalization, and query-specific reachability |
| [source-law.md](source-law.md) | Named outcomes, parameter guards, legal support, joint laws, and exact contraction |
| [kernels.md](kernels.md) | Four-bit Boolean operations, Venn signatures, ARM64/NEON paths, and measured limits of the probes |
| [event-surface.md](event-surface.md) | Rust schemas, source construction, pointwise FD/INDs, and empty values |
| [query-algebra.md](query-algebra.md) | Explicit Event/Pack/Probability stages over Free Join |
| [coup/README.md](coup/README.md) | Complete game schema and its source/protocol contracts |
| [coup/query-walkthrough.md](coup/query-walkthrough.md) | Steals, bluffs, coupled hands, proofs, and future decisions |
| [coup/algebra-applications.md](coup/algebra-applications.md) | Construct safe actions, resolve information cases, and derive certainty from uncertain choices |
| [implementation-plan.md](implementation-plan.md) | Native dependency order, acceptance criteria, and workload comparisons |
| [laws.md](laws.md) / [decisions.md](decisions.md) | Derived invariants, retained source laws, and current decisions |

[First principles](first-principles.md) explains the FD/IND terminology through
Coup. [Naming](naming.md) distinguishes Event from TypeSafe's numeric Noul result.
[Research index](research/README.md) contains the paper trail and historical designs.

## Concrete evidence

- [Native representation laboratory](experiments/event-repr-lab/REPORT.md): twenty
  carrier/kernel configurations, real Free Join, structured modal and coordinate-movement queries,
  residuals, finite closure, forty-coordinate symbolic tests and retained ARM64.
  The exact observation lane adds shared-parameter laws; the native dependency
  bridge matches 5,062 finite cases against the interval macro/validator/judge.
  Transport adds 1,024 finite transfers, 121 forty-coordinate symbolic transfers,
  and strict support-map/ownership checks. Native replay checks imported inputs
  against imported outputs using exact destination identities. The separate
  owned lane imports only inputs and publishes 80 computed results in each of
  82 verified cases across all fifteen carriers. The symbolic relation lane
  passes 27 cases across nine carriers, including full million-state counter
  closure in a sixty-coordinate presentation without world enumeration.
  The essential-table extension adds 163 matched native cases with scalar/word
  algorithms and exact shared-parameter observation; its first timing results
  favor the existing dense and fixed-tail controls.
  The scoped classifier extension adds 264 native query cases and focused
  repeats, plus direct-versus-constructive ARM64 evidence. [Lean proofs](experiments/event-repr-lab/LEAN.md)
  now check 195 central theorem reports, including signature laws,
  anchored representation, essential-coordinate minimality/projection, restriction,
  support-relative dependency limits, and the
  exact support-image condition for preserving possibility and pair readouts.
  The base-change proofs give the complete-fibre condition for existential,
  universal and nonvacuous rewrites without requiring unique lifts.
  Information-readout proofs characterize observable bounds, exact FD conditions
  for moving filters, and the FD order between observations.
  Borrowed-product proofs establish why the joint image of mapped operands,
  including the retained output context, is the exact contraction contract.
  Twelve additional reports, retained separately, prove Pack factorization,
  its exact scalar join-dependency, group existence, participating validation
  and typed relational analogues. The native Pack experiment passes 5,376
  differential executions and 144 main timing configurations; it wins every
  fresh comparison while exposing a small-fanout warm crossover.
  The relational extension adds six further axiom-free reports for scalar
  separators and residual aggregation, 11,904 native differential executions
  and 384 main timing configurations. Composition keeps its shared middle
  world; source environments remain shared as well.
  The [compact-storage comparison](experiments/event-repr-lab/SLABS.md) adds
  506 native cases with 146 matched layout pairs: lower bytes, mixed speed,
  and a repeated direct-classifier regression. The subsequent
  [word-classifier comparison](experiments/event-repr-lab/WORD-CLASSIFIER.md)
  adds 576 query cases and 144 matched scalar/word pairs. Independent repeats
  confirm large first-use gains under both stores, including ordered cuts where
  some combinations must be proved impossible. Existing controls still win
  several queries. Rust correctness retains its separate differential and
  native tests.
  The [bounded-scratch comparison](experiments/event-repr-lab/SCRATCH.md)
  adds 768 distinct cases and 192 matched kernel pairs across 130 passing
  processes. It confirms an alignment-heavy gain while rejecting a universal
  switch on Coup; fifteen extracted symbols retain the ARM64/stack evidence.
  The [residual-normalization comparison](experiments/event-repr-lab/REUSE.md)
  adds 445 passing native processes, 912 timed query configurations and 1,920
  matched comparisons. Independent repeats recover much of the deferred-view
  cost by canonicalizing source cofactors; cache reuse and output working order
  retain separate tradeoffs. Acceptance adds 192 owned cases and 48 symbolic
  programs. [Supported-dependence proofs](experiments/event-repr-lab/SUPPORTED-DEPENDENCE.md)
  distinguish raw least-coordinate masks from logical dependencies on coupled
  legal worlds.
  [Current lab audit](experiments/event-repr-lab/results/audit.json) checks source
  snapshots, production isolation, local links, results and new paper hashes.

- [Algebra checks](research/event-algebra/checks.json): 21 groups; Boolean and relational
  operators, modalities, residuals, finite closure, admissibility versus zero
  mass, exact source updates, and decision examples. Includes 4,096 residual
  triples, 4,096 closure/goal cases, and 1,024 finite action-arena/goal cases.
- [New literature adjudication](research/algebra-resolution.md): primary-source
  reading scope, decisions, and limitations; five newly retained arXiv papers.
- [Earlier revision 0.5 document/source audit](research/event-algebra/document-audit.json): 39 active
  and supporting documents have no missing local links; all nine new source
  snapshots match their recorded hashes.
- [Verification record](research/event-algebra/verification.json): all eight
  proposal semantic scripts passed; earlier native instruction probes are retained
  separately and were not rerun for this documentation/algebra revision.
- [Representation checks](research/representation/checks.json): eight groups;
  all 15 small nonempty supports, 624 event pairs, and 9,984 Boolean operations;
  support-relative identity, complement, Venn symmetries, deterministic wire
  rebuilding, parameter guards, and the Coup compound query through the BDD carrier.
- [ARM64/layout probes](research/representation/probes.json): sixteen-byte key,
  sixteen-byte aligned BDD node, eight-byte pair; 32,000 NEON Boolean blocks,
  2,000 steal blocks; inspected complement `EOR`, steal `ORR/BIC`, predicate `TBL`.
- [Coup query checks](coup/research/query-checks.json): twelve groups over 4,290
  deals and 21,450 deal/decision worlds, plus transition/empty/complement cases.
- [Event laws](review-evidence/event-checks.json), [Coup state laws](coup/research/event-checks.json),
  [source processes](review-evidence/process-checks.json), and
  [polytope subtheory](review-evidence/spec-checks.json): retained exact semantic examples.

Run from the repository using installed tools:

```sh
python3 proposal/representation-checks.py
python3 proposal/algebra-checks.py
python3 proposal/kernels/run-probes.py
python3 proposal/coup/query-checks.py
python3 proposal/coup/event-checks.py
python3 proposal/event-checks.py
python3 proposal/process-checks.py
python3 proposal/spec-checks.py
```

The probe runner builds temporary standalone executables inside `proposal/`,
removes them after use, and retains assembly/results. It installs no toolchains.
On a non-ARM64 host it skips native NEON execution explicitly.

**No native engine feature has been implemented.** The macros are proposed;
the BDD checker is a reference carrier; the assembly belongs to isolated fixed
blocks. The temporary Rust laboratory now exercises actual native Free Join with candidate
Event sinks. Native Event schema/admission, source-solver integration, storage,
and complete query integration remain implementation work with concrete acceptance
criteria. The lab changes only a disposable copy of the engine.
