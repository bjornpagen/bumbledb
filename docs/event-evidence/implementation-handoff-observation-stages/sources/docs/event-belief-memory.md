# Exact Event belief memory

`BeliefMemory` builds the hidden-state memory needed for repeated partially
observed play. Each memory state owns an ordinary Event: all current worlds
consistent with the actor's complete action/observation history. A world's
membership remains structural, including worlds with zero stochastic mass.

The inputs are a full indexed observation partition, initial evidence, and an
indexed roster of action relations. Labels remain ordinary application values.
Each action is a checked endorelation on the same hidden-state space and shared
environment. Choosing a label invokes that relation; all its outcomes are
possible. The modeled hidden state must contain everything affecting future
transitions. This constructor does not infer a transition model from observations.

## Updating memory

For a current belief B, action a and observed cell o:

```text
Enabled(B, a) = B is inhabited and every s in B has a successor under a
Update(B, a, o) = Post(a, B) ∩ ObservationCell(o)
```

Initial states are `given ∩ ObservationCell(o)`. After each uniformly enabled
action, the constructor computes all possible observation updates. Empty
updates have no successor. It repeats until every reachable Event is processed.
Equal belief Events share a node. Merely seeing the same screen does not merge
different memories. Action availability in just one possible hidden state is
insufficient: the actor must be able to execute the same action in every state.

`initial()` retains one position per observation cell, including `None` for
unreachable cells. `states()` exposes each possibility Event, last observation
index and transition roster. `update(state, action, observation)` uses only these
public labels and the previous memory. It returns `None` for an unavailable
action or impossible observation; `enabled` distinguishes these cases. Out-of-range
indices refuse even if the action was unavailable. Indices belong to this memory.

## From memory to a policy

```rust
let memory = BeliefMemory::new(
    &actions, &observations, &given, BeliefLimits::default(), &work,
)?;
let compiled = memory.arena(names, &work)?;
let goal = compiled.known(&hidden_goal, &work)?;
let strategy = compiled.arena().winning_reach(
    &goal, FixedPointLimits::default(), PartitionLimits::default(), &work,
)?;
```

`names` supplies five explicit `SpaceId`s for memory states, action labels, a
trivial environment and the two products. This lowers directly into the existing
`ActionArena`. Its ranks and policy are Events, with the existing progress and
continuing-safety contracts. No second game solver is introduced.

`known(G)` selects memory states whose every possible world satisfies G.
`possible(G)` selects states with at least one such world. Reaching `known(G)`
guarantees eventual knowledge of G under every retained policy choice and
outcome. This is stricter than merely visiting an unobservable hidden goal;
the API does not claim completeness for all partial-observation objectives.

A concrete test starts with an unknown secret. A probe reveals it; a later action
erases the signal from the screen. Both resulting worlds now show the same
observation, but their remembered Events are different. The compiled policy
chooses the correct finishing action in each case and proves a three-step bound
from the start. Replacing that memory with the screen alone loses the game.

## Representation, dependencies and ownership

The constructor explores reachable **Events**, without enumerating hidden worlds.
It retains an owned roster of sources, the initial evidence and observation
partition, canonical Event keys for interning, and edges indexed by action and
observation. Breadth-first insertion follows authored roster order; hash-table
iteration never determines codes. Transition rosters are ordered for binary lookup.

Compilation gives each memory node and action label an ordinary finite code,
restricting binary support to the exact roster. Unused codes are illegal.
It builds a full memory/action product and a transition Event using existing
maps and Boolean kernels. The public arena has a trivial environment: hidden
environment values do not become visible coordinates. Original possibility
Events retain their shared environments and parameter assignments through
the checked transition maps.

The database fixture uses ordinary rows:

```text
Memory(game, id, possible)
MemoryCode(game, id, when)
Next(game, from, action, observation, to)
Winning(game, when)
Policy(game, when)
```

`Memory(game,id)` is an ordinary key. Distinct beliefs may overlap in hidden
worlds, so `possible` is deliberately not a pointwise key. `MemoryCode(game,when)`
is a pointwise key: each code selects one memory node. Its regions partition a
declared memory domain. Ordinary inclusions check transition endpoints and
Event containment places winning states inside that domain. Free Join binds
these rows and compares Events through the existing owned registry.

All owners survive release of original inputs. BEVT stores region values, not
the memory-construction certificate. Reopening those values does not by itself
reconstruct a certified controller. Reusing a compiled `SpaceId` requires keeping
the same named roster/presentation; reordering application labels is not a source
renaming operation. Dedicated memory descriptor/SDK/query construction remains
required consumer work.

## Limits and evidence

The reachable subset graph can be exponential. `BeliefLimits` bounds states,
transitions and cumulative admission/expansion/splitting steps. Allocation and
cancellation are fallible throughout. Kernel budgets remain in effect separately.
Failure publishes no partial graph. Retained inputs may keep canonical
intermediates; this is not the engine's unfinished aggregate retained-byte policy.
Empty evidence yields no memory states. Compiling an empty memory or an empty
action vocabulary refuses because an Event space must be inhabited.

Seven core tests cover 2,048 small-game/input cases against an independent dense
oracle, 256 compiled games and their goal regions, the signal-memory example,
zero-mass worlds, role/environment refusals, independently decoded owners,
budget/cancellation and shared parameter guards. One symbolic memory state
retains 2^20 hidden worlds. That is a semantic stress test, not timing evidence.
A native database fixture checks FDs/INDs, the code partition, reopen, both Free
Join paths and results retained after database release.

`BeliefMemory.lean` adds eighteen reference reports, including exact agreement
with all complete histories, enabledness, observation coverage/disjointness,
safe-successor equivalence and environment preservation. Native BFS/interner,
descriptor admission, code lowering and kernels remain tested correspondence
obligations, not Lean-verified Rust.

This is possibility memory. Equal supports can carry different posteriors.
Probability-sensitive policies must retain separately updated laws/receipts;
merging them by support would be wrong. No prior, stochastic coupling, evidence
weight or posterior revision is inferred by this construction. Full Coup,
TypeSafe integration and the other M0–M8 gates remain open.
