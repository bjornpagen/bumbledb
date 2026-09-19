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
renaming operation. Checked memory recipe transport and SDK construction/inspection
are described below; named compilation transport is also implemented. General strategy/program transport
and native query memory construction remain required consumer work.

## Limits and evidence

The reachable subset graph can be exponential. `BeliefLimits` bounds states,
transitions and cumulative admission/expansion/splitting steps. Allocation and
cancellation are fallible throughout. Kernel budgets remain in effect separately.
Failure publishes no partial graph. Retained inputs may keep canonical
intermediates; this is not the engine's unfinished aggregate retained-byte policy.
Empty evidence yields no memory states. Compiling an empty memory or an empty
action vocabulary refuses because an Event space must be inhabited.

Thirteen core tests cover 2,048 small-game/input cases against an independent dense
oracle, 256 compiled games and their goal regions, the signal-memory example,
zero-mass worlds, role/environment refusals, independently decoded owners,
budget/cancellation and shared parameter guards. One symbolic memory state
retains 2^20 hidden worlds. That is a semantic stress test, not timing evidence.
A native database fixture checks FDs/INDs, the code partition, reopen, both Free
Join paths and results retained after database release.

`BeliefMemory.lean` contains twenty-nine reference reports, including exact agreement
with all complete histories, enabledness, observation coverage/disjointness,
safe-successor equivalence and environment preservation. Native BFS/interner,
descriptor admission, code lowering and kernels remain tested correspondence
obligations, not Lean-verified Rust.

This is possibility memory. Equal supports can carry different posteriors.
Probability-sensitive policies must retain separately updated laws/receipts;
merging them by support would be wrong. No prior, stochastic coupling, evidence
weight or posterior revision is inferred by this construction. Full Coup,
TypeSafe integration and the other M0–M8 gates remain open.


## Checked recipe transport

`BeliefDescriptor` is plain untrusted data. Its full source marker, initial
`given`, ordered observation cells and ordered action descriptions contain
canonical BEVT values. Each action retains its original coordinate concatenation
and separate `reversed` flag through `FibreDescriptor`, plus its region. Import
admits both finite and supported shared-parameter sources, verifies full coverage,
reconstructs the products and closes the reachable graph anew. A memory with no
reachable states still checks every action. No graph or strategy certificate is
accepted from the transport.

```rust
let limits = BeliefDescriptorLimits::default();
let data = BeliefDescriptor::capture(&memory, limits.descriptors, &work)?;
let bytes = data.to_bytes(limits.descriptors, &work)?;
let restored = BeliefDescriptor::import(&bytes, limits, &mut arithmetic)?;
let compiled = restored.arena(names, &work)?;
```

Capture records the retained normalized memory, including all actions in the
first action's product presentation. It does not recover original syntax or
provider provenance. BFS order depends on the indexed recipe, so applications
must retain the corresponding public action/observation labels. Reordering them
is not a harmless source rename. The five compilation names remain explicit
application inputs. BEBM owns the memory recipe; BEBA below retains its named
compilation.

BEBM v1 uses `BEBM` followed by version byte 1, then source and given blobs,
an observation count and cell blobs, and an action count and action records.
Each blob is prefixed by a little-endian u64 length; counts are little-endian
u64. An action record is the BEDC fibre grammar (32-byte identity, one strict
0/1 reversed byte, two maps), followed by its region blob. Each map is source
blob, target blob, readout count and readout blobs. There is no nested BEDC
header. Trailing bytes, truncated data, unknown versions and invalid tags refuse.
Syntax parsing alone does not establish mathematical validity.

One descriptor byte/item allowance covers every nested map, readout, source and
roster occurrence, including duplicates. Encoding/decoding also bounds the full
envelope. `BeliefDescriptorLimits` supplies separate partition, reachable graph,
source solver and per-owner kernel bounds. All exact source admission and action
normalization uses the caller's single `ExactArithmetic` counter. Shared-counter
variants on face/fibre products, product pairing/reindexing and memory construction
prevent nested arithmetic allowances from silently restarting.

The SDK surface is intentionally a recipe over existing Event/fibre descriptions:

```ts
const memory = yield* EventMemory.admit({
  source: hiddenStates,
  given: initialEvidence,
  observations: [blankScreen, sawLeft, sawRight, won, lost, emptyCell],
  actions: [
    { product: hiddenPairs, region: probe },
    { product: hiddenPairs, region: conceal },
    { product: hiddenPairs, region: chooseLeft },
    { product: hiddenPairs, region: chooseRight }
  ]
})
const saved = EventMemory.toBytes(memory)
const graph = yield* EventMemory.inspect(memory)
```

`describe` returns the retained recipe; `inspect` returns `initial` indices
(null for empty observations), and states with `possible`, `observation` and
ordered `transitions` (`action`, `observation`, `target`). These indices are local
to the memory. SDK operations reconstruct on the cancellable native worker.
Output has one cumulative item/byte bound, including every state and edge;
oversized inspection refuses rather than returning a truncated graph. Pure
`fromBytes` owns the bytes and recognizes the envelope without loading the addon.
All mathematical work remains in Rust. Detached Events can be stored and joined
through the ordinary Event field. Named compilation and strategy result consumers
are described below; general strategy/program transport remains open.


## Named compilation and strategy results

`BeliefArenaDescriptor` retains the memory recipe plus the five `BeliefSpaceIds`.
`capture`, `to_bytes`, `from_bytes`, `admit` and `import` parallel the memory recipe
API. Admission reconstructs memory and recompiles under those exact names. The
BFS code order still follows the authored action/observation rosters; changing
names changes the declared output contexts and does not change the hidden model.
No graph or policy assertion from the input can bypass reconstruction.

BEBA v1 is `BEBA`, version byte 1, then five 32-byte identities in state, action,
environment, state/action product and transition product order. The remaining
body is the BEBM v1 recipe grammar without its five-byte header. The outer root,
each identity and every nested recipe item share one cumulative byte/item policy;
the full encoded envelope has the same byte bound. Unknown versions, trailing
bytes and incomplete payloads refuse. Empty memory or an empty action vocabulary
cannot compile into inhabited Event spaces.

`BeliefMemory::arena_with_limits` bounds every new code/product owner explicitly;
`arena` retains the default convenience policy. `BeliefArena::identities` exposes
the five retained names, while `state_code` and `action_code` return one ordinary
Event per roster index and refuse unused indices. Code support excludes unused
binary codes. Existing memory owners retain the policy under which they were
admitted. This is not the unfinished aggregate retained-memory quota.

```ts
const controller = yield* EventMemory.compile(memory, {
  states: stateId,
  actions: actionId,
  environment: environmentId,
  stateActions: stateActionId,
  transitions: transitionId
})
const arena = yield* EventMemoryArena.inspect(controller)
const goal = yield* EventMemoryArena.known(controller, hiddenWin)
const result = yield* EventMemoryArena.reach(controller, goal)
// result.winning, result.ranks and the region in result.policy are Events.
// Store their code/label associations in ordinary application relations.
```

`EventMemoryArena.fromBytes` / `toBytes` own BEBA transport. `describe` returns
`memory` (an owned EventMemory) and `identities`; compiling that description
reproduces the same canonical bytes. `inspect` reconstructs and returns full
`states`, `choices`, `initial`, BEDC `actions` and `transition`, plus indexed
`stateCodes` and `actionCodes`. These positions correspond to the retained memory
and public action labels. Application schema columns continue to supply labels.

`known(controller, hidden)` and `possible(controller, hidden)` return Events on
the compiled memory-state space. `reach(controller, goal)` and
`safe(controller, invariant)` require objectives already on that space. Passing
a hidden-state Event directly refuses; explicitly choosing known/possible is a
substantive modeling decision. Reach exports `goal`, `winning`, a BEDC relation
`policy`, and ordered `ranks`. Safety exports `invariant`, `winning` and `policy`.
The existing Rust solvers establish progress or continued enabled behavior.
Inspection uses a cumulative output bound and refuses oversized code/rank rosters
rather than dropping entries. All operations use the cancellable native executor.

Reach can stop at a goal; safety needs an enabled continuation forever. In the
secret-probe example, replayed reach ranks prove three moves suffice, including
the remembered finishing choice after the display is blank again. Continuing
safety on that terminating game is empty. A self-looping memory instead admits
a continuing safety policy. Neither operation introduces a probability law.
These detached results do not claim general executable strategy transport;
import/restriction of arbitrary strategy certificates and native query memory
construction remain required implementation work.
