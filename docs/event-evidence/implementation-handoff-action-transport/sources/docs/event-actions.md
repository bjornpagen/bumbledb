# Native Event actions, permissions and strategy witnesses

`ActionArena` is a checked view of `Step(state, action, outcome)`. The transition
remains an ordinary Event relation; state and action vocabularies stay in their
declared faces. The API computes enabled choices, uniform one-step permissions
and finite fully observed reachability/safety strategies. It neither calls a
model nor supplies a probability law. These are host APIs on the implementation
branch. BEAC v1 now reconstructs general arenas and reach/safety strategies;
constructive query heads, general program transport and a general action SDK remain
open. Named belief-memory compilation has BEBA replay and SDK strategy result
consumers described in [belief memory](event-belief-memory.md).

## Roles, support and representation

An arena owns a full legal `FibreProduct` S×A and a `WorldRelation` from that
product to an outcome face T. Availability belongs inside the transition
region, not in a product that silently omits inconvenient actions. Admission
checks that the transition input is the named state/action product and that
its environment map equals the product's retained environment readout.
Equal coordinate counts or equal environment ranges cannot replace that check.

One-step reasoning permits different S and T contexts. Iteration additionally
requires the same state/outcome context and the same environment readout. All
checks apply before empty/full shortcuts. The finite environment is an explicit
modeling scope, not evidence that an actor observes its value. Source parameters,
actor visibility and information-memory adequacy retain their own contracts.

The native data consists of owned products, relation Events, result Events and
rank partitions. Existing symbolic maps, modalities and image kernels perform
the work. No runtime enumeration of worlds or separate game-graph encoding is
required. The current finite product coordinate limits still apply, including
the combined state/action/outcome workspace.

## Enabled choices and quantifier order

| API | Meaning |
| --- | --- |
| `enabled(control)` | State/action pairs with at least one legal outcome |
| `good(E, control)` | Enabled pairs whose every outcome lies in E |
| `controllable_predecessor(E, control)` | States with some action in Good(E) |
| `predecessor_program(control)` | Inspectable typed Event program implementing that predecessor |
| `reach_program(E, control)` | Sealed endoprogram X ↦ E ∪ CPre(X) |
| `safe_program(E, control)` | Sealed endoprogram X ↦ E ∩ CPre(X) |

The actual lowering is `Good(E) = Must(Step,E)` followed by image to the state
face. It preserves **exists action, forall legal outcome**. A pair with no
successor is unavailable, even though an unguarded universal condition would
hold there. The arena's step semantics must already describe what executing an
action permits; observing which action happened is not a causal transition model.

For hidden states, `RelationalProduct::uniform_permissions(I, Good, control)`
constructs an observation/action relation. I has role O→S; the prepared plan has
roles S→O, O→A and S→A. It computes:

```text
Permitted(o,a) = (exists s, I(o,s)) and
                (forall s, I(o,s) implies Good(s,a))
```

This is the greatest sound permission relation on inhabited cases. Its lowering
is `LeftResidual(Converse(I), Good)` restricted by `Domain(I)` on the observation
face. Use `arena.good(E)` when actual action enabledness must participate.
Empty information cases permit nothing. The two hidden states can each have a
good action while their shared observation case has none, if those actions differ.

## Reachability needs progress witnesses

`winning_reach(goal, fixed_limits, layer_limits, control)` computes the least
fixed point and returns a `ReachStrategy`. The winning region alone is not a
strategy: it can contain an action that stays inside the region forever.

`FixedPointProgram::least_with_layers` therefore retains each nonempty first-entry
difference as an `EventPartition`. Cell i first appears at application i+1.
Cells are disjoint and cover the result; an immediately empty result has no
layers. For controlled reachability, the first layer is exactly goal because
`CPre(Empty)` is empty. Goal states have rank zero and stop immediately.

For each later layer i, the policy retains every enabled action whose outcomes
all lie in the union of earlier layers. Each such choice strictly decreases rank.
Every admitted non-goal state has at least one such action. An application or model
can choose any remaining action without invalidating progress; every outcome
reaches goal within the starting rank's bound. No fairness assumption is used.

Consider `0:a0→0`, `0:a1→1`, with goal `{1}`. Both states are winning, but a0
at state 0 stalls forever. The returned policy permits a1 there and retains no
action at the completed goal. Rank regions are `{1}` at rank 0 and `{0}` at rank 1.
The compiled API example constructs exactly this case.

`ReachStrategy::with_policy` accepts a supplied subrelation only when it covers
every required non-goal state and contains only the retained rank-decreasing
choices. It can remain nondeterministic. This validates restriction of this
specific progress policy; it does not claim every alternative terminating policy
must decrease these particular first-entry ranks.

## Safety requires continued enabled behavior

`winning_safe(invariant, fixed_limits, control)` computes the greatest solution
of `X = invariant ∩ CPre(X)` and returns `SafetyStrategy`. Its policy contains
all enabled actions at winning states whose every outcome remains winning.
`with_policy` may remove choices only while retaining at least one at every
winning state. Every possible retained choice preserves the invariant.

A terminal state is excluded even if the invariant holds there. This differs
from `WorldRelation::safe_throughout`, whose All modality allows safe terminals.
If terminal success should permit indefinite continued behavior, model an explicit
absorbing success transition. Reachability instead permits stopping at the goal.

## Visibility, ownership and refusal boundaries

These strategy solvers are fully observed. Their state can be a supplied, adequate
information/memory state, but applying them directly to hidden worlds does not
solve a partially observed game. One-step uniform permissions alone do not build
the belief-state updates required for repeated Coup play.
[BeliefMemory](event-belief-memory.md) now supplies an exact reachable subset
construction from explicit action relations, observation cells and initial
evidence, then lowers it into this arena. Its history semantics preserve
possibility memory; posterior-sensitive policies need retained laws separately.

Strategies own the arena, goal/invariant, fixed-point result and policy. Reach
strategies additionally own the layer partition. All are inspectable, and their
Events can be stored and joined by the existing engine. BEVT persists region
values; it does not serialize an arena, role descriptor or strategy certificate.
Reopening those regions does not by itself re-admit a strategy. BEAC now retains
the complete arena, objective and selected-policy recipe for that re-admission.

Fixed-point iteration and cumulative instruction budgets remain unchanged.
`PartitionLimits::cells` additionally bounds retained layers. Witness construction
uses existing kernel budgets and cancellation, rather than counting its graph
operations as program instructions. Capacity/allocation/cancellation failure
publishes no partial strategy. Live arenas may retain canonical intermediates.
Unsafe and incomplete supplied policies have distinct errors.

## Proof and implementation evidence

[Actions.lean](../crates/bumbledb-event/semantics/Actions.lean) adds nineteen
reports for the controlled predecessor, enabledness, environment preservation,
inhabited greatest permissions, first-entry progress and policy restriction.
Its `TerminatesWithin` reference quantifies over every retained policy choice
and every outcome. Decreasing natural ranks prove a uniform finite bound.
Additional laws prove that every bounded winning policy lies inside each reach
prefixed point, and that safety policies remain enabled and preserve the
invariant on every finite path. Greatest safety contains every enabled invariant.

[FixedPoint.lean](../crates/bumbledb-event/semantics/FixedPoint.lean) adds three
reports for ordered iterates and first-entry layer uniqueness/coverage. These
connect to the existing finite fixed-point and typed-program reference laws.
Native role admission, graph kernels, iteration, arrays and resource handling
remain tested correspondence obligations; Lean does not verify the Rust engine.

Eight new core tests enumerate all 256 two-state/two-action transition relations
and four goals against all deterministic memoryless policies. Separate checks
cover all 256 information/good pairs and all 4,096 candidate permission relations,
stalling and incomplete choices, continuing safety, environment/role refusals,
context-changing one-step programs, budgets and cancellation. A symbolic 20-bit
rotation retains 20 rank layers over 2^20 states without world enumeration.
This is a semantic stress case, not a performance claim.

The database fixture stores winning/rank regions under native pointwise keys and
containment, persists policy regions, reopens, joins independently constructed
expected policy Events on resident/cursor paths and retains values after closing
the database. The [semantic record](event-evidence/native-action-semantics/check.json)
has 185 reports across eleven native modules. The
[qualification](event-evidence/native-action-qualification/check.json) and
[handoff](event-evidence/implementation-handoff-actions/check.json) retain exact
sources and checks. The later belief-memory slice has its own tests and reference
proofs. Full query/source/SDK integration and M8 performance qualification remain
open. No release.


## Checked arena and strategy transport: BEAC v1

`ActionDescriptor` is plain inspectable data with three cases:

- `Arena(ActionArenaDescriptor { actions, transition })` retains the complete
  state/action product and the transition's original product/region.
- `Reach { arena, goal, policy }` reconstructs the ranked reach solver.
- `Safe { arena, invariant, policy }` reconstructs the continuing-safety solver.

Objectives and policies are canonical BEVT blobs. A policy is a region on the
arena's state/action product: its roles are determined by that product.
`policy: None` requests all permitted solver choices. `Some(bytes)` requests a
restriction and must pass both inclusion and domain-coverage checks. Capture
always uses `Some`, preserving the selected policy even when it is a strict
restriction or empty. There is no imported winning-set, rank or iteration claim.

```rust
use bumbledb::event::{ActionDescriptor, ActionDescriptorLimits,
    AdmittedActionDescriptor, ArithmeticLimits, ExactArithmetic};

let limits = ActionDescriptorLimits::default();
let recipe = ActionDescriptor::capture(
    &AdmittedActionDescriptor::Reach(strategy), limits.descriptors, &(),
)?;
let bytes = recipe.to_bytes(limits.descriptors, &())?;
let AdmittedActionDescriptor::Reach(restored) = ActionDescriptor::import(
    &bytes, limits, &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
)? else { unreachable!() };
// Ordinary Event regions and role-checked relations remain available.
let winning = restored.winning();
let policy = restored.policy();
let ranks = restored.ranked().layers();
```

`strategy` is the checked native strategy described above. The executable
crate example runs this full roundtrip. `capture`, `to_bytes`, `from_bytes`,
`admit` and `import` have the same plain-data/checked-value distinction as BEDC.
`AdmittedActionDescriptor` owns the reconstructed arena or strategy. Reopening
is valid after all original source, relation and strategy owners are dropped.

Admission supports finite unmeasured sources, fixed-law sources and the supported
shared univariate parameter sources. No new law, prior or independence is inferred.
Possible zero-mass outcomes still participate in universal outcome checks.
All source replay, parameter map composition, product construction, policy
normalization and restriction share one caller-owned `ExactArithmetic` counter.
The corresponding explicit-work APIs are `CoordinateMap::then_with_parameters`,
`ActionArena::new_with_parameters`, `winning_reach_with_parameters`,
`winning_safe_with_parameters` and each strategy's `with_policy_and_parameters`.
Default host methods create a default counter and delegate. The BEAC import
path never restarts it. Parameter-changing maps remain unsupported.

`RelationDescriptor { product, region }` is shared plain structural data;
`BeliefActionDescriptor` remains a compatibility name. BEBM recipes reuse the
same finite/family admission path without changing their bytes. Reversal retains
original coordinate concatenation and exchanges only roles.

The wire grammar uses BEDC's `fibre` and length-prefixed `blob` definitions, with
no nested BEDC header. Header: ASCII `BEAC`, byte `1`, then kind byte:

| Kind | Body |
| --- | --- |
| 0 Arena | action fibre, transition fibre, transition region blob |
| 1 Reach | arena body, goal blob, optional-policy marker/payload |
| 2 Safe | arena body, invariant blob, optional-policy marker/payload |

A policy marker is byte `0` for absent, or byte `1` followed by a blob. Every
other marker, kind, orientation or version refuses, as do truncation and trailing
bytes. Parsing does not admit the enclosed sources or strategy. One cumulative
descriptor allowance covers the outer descriptor, arena, relation, all fibres,
maps and Event blobs. Names remain authored; wire bytes additionally bound the
whole envelope. Fixed-point iteration/instruction and partition bounds apply to
recomputed strategies; Event bounds apply to reconstructed owners. These limits
are not an aggregate retained-memory quota or a performance qualification.

The native tiny-game oracle now checks all 256 arenas after BEAC replay and all
2,048 goal/solver results after replay against independent enumerated policies.
Additional tests retain proper reach/safety restrictions, original converse
coordinates, law-bearing sources and zero-mass adversarial outcomes, parameter
identity and cumulative arithmetic. They refuse incomplete/stalling policies,
changed objectives, malformed dormant policies, wrong full markers/roles,
iteration/layer/kernel limits, every truncated prefix and cancellation. An
independently authored BEAC grammar checks all kinds and optional-policy cases.
The database fixture replays the strategy before persisting winning/rank/policy
Events and testing dependencies, reopen and both Free Join paths.

Five additional `Actions.lean` reports prove preservation of controlled
predecessors and both fixed-point iterate sequences under equivalent replayed
meanings, and the progress/continuation consequences of subset plus domain checks.
There are now 24 Actions reports, 789 across 44 native modules. These assume
extensional reconstruction and do not verify Rust, codecs, solver implementation,
allocator behavior or performance. Evidence is retained in
`event-evidence/action-transport-qualification`,
`event-evidence/action-transport-semantics`,
`event-evidence/implementation-handoff-action-transport` and
`event-evidence/action-transport-portable`.

This closes native arena/reach/safety transport. It does not attach hidden-world
visibility or BEBA provenance to a general strategy: replay BEBA separately when
that interpretation is required. General action/strategy SDK and query consumers,
general EventProgram transport, native memory construction in queries and all
remaining M0–M8 gates remain open. No release, tag or version bump.
