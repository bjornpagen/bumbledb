# Native Event programs and finite fixed points

`EventProgram` is an owned, inspectable function from one Event to another.
It captures constants, checked maps and checked world relations. Its executable
instructions are the same Boolean, image and modal operations exposed by the
Event algebra. `FixedPointProgram` additionally seals a monotone endoprogram over
one finite legal space. These are native host APIs on `codex/event-algebra`.
Query-head compilation, multi-binding programs and source integration remain
separate acceptance work; public v1.3.1 does not contain these APIs.

## Representation and admission

`EventProgramBuilder` assigns each authored node an index and a private builder
identity. Operands must belong to that builder, even when another builder has
the same input space and matching numeric indices. Each node retains its output
`Space`, variance and operation. Checked alignment validates Boolean operands;
map and modal instructions validate their input/output roles. Constants retain
their original context. An empty/full result cannot bypass these checks.

The instruction basis is:

| Instruction | Meaning |
| --- | --- |
| Input / Constant | Supplied Event / captured owned Event |
| Not / Binary / Ite | Complement, any of sixteen binary truth functions, conditional |
| Pullback / Image | Substitute a checked readout / existentially quantify its fibres |
| UniversalImage / NonvacuousImage | Every preimage / an inhabited fibre with every preimage |
| Possible / Guaranteed | Saturate a condition through a captured observation map |
| May / All / Must / Post | Modal operations of a captured, fixed relation |

`finish` walks dependencies backwards, removes unused authored nodes and rewrites
indices into a compact dependency order. Every child index points to an earlier
instruction. Evaluation walks that array once and retains owned intermediate
Events. Captured objects share their existing owners; there is no separate world
enumeration or alternate region encoding. Maps and relations use their existing
symbolic lift/conjoin/abstract kernels.

`ProgramOp` and `ProgramInstruction` expose read-only structure. Constructing an
inspection enum cannot admit an executable program. There are no arbitrary Rust
callbacks, provider calls or mutable constants inside the instruction set. A
program may change context through checked maps; iteration additionally requires
equal input/output contexts and aligns each answer into the carrier's owner.

Pruning removes only nodes outside the root's dependency graph. All participating
operands remain evaluated, including both operands of constant truth functions
and all three ITE inputs. Evaluation validates its supplied input even for a
constant program. This preserves participation; the complete query-level stable
fault-set contract is still M5 work. Native execution currently returns its first
operational error.

## Monotonicity is checked before iteration

Variance records which changes are possible when the input Event grows:
`Independent`, `Increasing`, `Decreasing`, or `Mixed`. Input is increasing;
captured constants are independent. The Boolean checker enumerates the permitted
before/after truth assignments. Complement reverses direction. Fixed map and
modal operations preserve inclusion, so they transport the operand's variance.

Only independent or increasing endoprograms enter `fixed_points`. This analysis
is conservative: correlated subexpressions can make a function monotone without
the local checker proving it. Refusal is `NonMonotoneProgram`; it is not a claim
that every refused function is mathematically nonmonotone. Double negation is
admitted; bare negation is refused. No user assertion can skip this gate.

## The finite bound and the operational budget are different

`FiniteCarrier` captures the existing native `Space` and counts its **original
legal support**, using the symbolic count kernel. A diagram node count, completed
code alias count or sampled world count cannot supply this bound. Native spaces
have at most 62 coordinates, hence between 1 and 2^62 legal worlds. Constructing
this certificate does not enumerate those worlds.

For N legal worlds, a strictly increasing inclusion chain can add a world at
most N times. Starting at empty therefore reaches the least fixed point by N
applications, and at most N+1 applications detect equality. Starting at full
gives the dual greatest fixed point. Returned `iterations()` includes the final
detection application. `program_steps()` counts retained instructions across all
applications. Each changing iterate also checks inclusion in the required
direction; an invariant failure refuses.

`least_with_layers` additionally retains the nonempty first-entry differences as
an indexed `EventPartition`, covering the result. Cell i entered at application
i+1. An immediately empty fixed point has no layers. The separate partition-cell
budget bounds retained ranks; rank exhaustion refuses the complete result.
The [action layer](event-actions.md) uses these layers to retain progress witnesses.

Operational limits may be smaller than the semantic bound:

| Limit | Default |
| --- | --- |
| Authored program nodes, including input | 100,000 |
| Fixed-point applications, including detection | 1,000,000 |
| Cumulative retained instruction executions | 20,000,000 |

`EventProgramBuilder::with_limit` and `FixedPointLimits` set those policies.
Underlying space kernel limits and `Control` cancellation remain active.
Allocation, capacity or cancellation failures return no approximant disguised as
a fixed point. As with other kernels, failed operations can retain canonical
intermediates in the live arena. Successful results own their Events and remain
usable after dropping the program, relation, database or query snapshot.

This finite certificate covers current unmeasured native spaces. A continuous
source parameter or unbounded transcript needs the separate finite-presentation
contract in the proposal; the finite environment readout is not that certificate.

## Reachability, inevitability and safety

For an endorelation R and goal E, the native helpers construct these programs:

| API | Program and meaning |
| --- | --- |
| `R.can_reach(E, limits, control)` | Least X = E ∪ May(R,X): some finite path reaches E |
| `R.inevitably_reach(E, limits, control)` | Least X = E ∪ Must(R,X): every maximal path reaches E |
| `R.safe_throughout(E, limits, control)` | Greatest X = E ∩ All(R,X): E holds now and at every reachable state |

The empty path counts. `Must` requires a successor, while `All` includes dead
ends. A terminal state outside E fails inevitability. A terminal state inside E
passes safety. An available cycle that can avoid E forever fails inevitability,
even if an exit to E exists; no fairness assumption is implicit.

For the finite graph `0→1`, `1→1 or 2`, `2→3`, with state 3 terminal:

| Query | Result |
| --- | --- |
| Can reach `{3}` | `{0,1,2,3}` |
| Inevitably reach `{3}` | `{2,3}` |
| Remain throughout `{1,2,3}` | `{1,2,3}` |

Each answer remains an Event that can be stored, combined and joined. The native
database test stores these results, reopens the database, joins them against
independently constructed Events on resident and cursor execution paths, then
uses the returned Events after closing the database.

The helpers require matching endpoint spaces **and environment maps**. Sharing
the same coordinate count or environment range is insufficient. Both contexts
and roles are checked before empty/full shortcuts.

## Reflexive transitive closure

`RelationalProduct::star_program(R, control)` prepares **X ↦ Id ∪ R;X**.
`star(R, limits, control)` evaluates its least fixed point and returns a checked
`WorldRelation`. The program lives in the prepared S×U result space, reindexes
X into T×U, lifts R and X into one S×T×U workspace, intersects and images to S×U.
One shared middle witness and one environment survive the entire operation.

All three endpoint roles must form the same endorelation. The carrier counts
legal **pairs**, rather than only endpoint states. Existing product coordinate
limits still apply. Star includes identity even when R is empty and denotes all
finite paths; it does not silently imply a probability-one eventual outcome.

## Proof and implementation evidence

[FixedPoint](../crates/bumbledb-event/semantics/FixedPoint.lean) proves exhaustive
legal-roster counting, bounded stabilization, exact early detection and
least/greatest extremality. Its shared-environment theorem and counterexample
separate fibrewise iteration from environment mixing.
[Programs](../crates/bumbledb-event/semantics/Programs.lean) proves Boolean variance
soundness for typed expressions across contexts, monotone readouts and admission
of positive programs. [Closure](../crates/bumbledb-event/semantics/Closure.lean)
identifies least closure with finite paths, least reachability with reaching a
goal, greatest safety with invariance along all finite paths, and least Must
iteration with well-founded forcing. It also rules out a permanent avoiding cycle.

The [semantic record](event-evidence/native-fixed-point-semantics/check.json)
contains 150 native reports across nine files, alongside the unchanged 246-report
proposal suite. These prove reference denotations. Exact graph equality/counting,
Rust indices and pruning, kernel correspondence, ownership and resource handling
remain tested implementation obligations; no Rust extraction is claimed.

Eleven new core tests cover all sixteen truth functions, every two-state relation
on each nonempty support, 384 four-state closure samples against a Warshall
oracle, shared environments, role/context failures, all instruction families,
ownership, budgets and cancellation. Symbolic cases include a 62-coordinate
carrier converging after 63 applications and identity closure over 2^40 legal
pairs in a 60-coordinate workspace. Those are semantic stress tests, not speed
claims. A compiled doctest demonstrates the three-world rotation program.

The [native qualification](event-evidence/native-fixed-point-qualification/check.json)
and [proposal handoff](event-evidence/implementation-handoff-fixed-points/check.json)
pin sources, commands and proof/document checks. The [finite partition helper](event-partitions.md)
now supplies indexed observable rosters. The [action layer](event-actions.md)
now supplies enabled actions, uniform one-step permissions and fully observed
ranked/safe strategies. Measured expectation, belief-memory construction, query
compilation, descriptor transport and integrated performance remain open gates.
