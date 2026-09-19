# Coup through events

**The primitive is `event` / `Event`.** An event is a region of admissible
worlds, optionally measured under a captured law. In Coup, one world is a
complete compatible deal and finite history, including the random draws and
player decisions needed by the questions we ask. A row's `when` says which of
those worlds it describes.

This is a complete rewrite of the earlier Coup example around that denotation.
It is a schema proposal and worked discussion, not a playable game or a claim
that the entire example compiles on the implementation branch. Event fields,
field/full dependencies and Boolean/Event Pack heads now work there; this
example also uses source and query forms still awaiting integration.
Base Coup with two through six players
is the scope; the numerical examples use Alice, Bob, and Cleo.

| File | Purpose |
| --- | --- |
| [schema.rs](schema.rs) | One proposed Rust macro schema: card worlds, perspectives, declarations, observations, and action partitions |
| [queries.rs](queries.rs) | Proposed Rust templates for event construction, packing, complements, and conditional measurement |
| [query-walkthrough.md](query-walkthrough.md) | Steal support, bluff odds, coupled hidden hands, and events across a replacement |
| [algebra-applications.md](algebra-applications.md) | Construct safe actions, distinguish uniform decisions, derive certainty, and reason over legal sequences |
| [algebra-queries.rs](algebra-queries.rs) | Proposed Test, cardinality, information-case, expectation, and world-face query templates |
| [query-checks.py](query-checks.py) | Staged reference queries checked against direct predicates on complete worlds |
| [research/query-checks.json](research/query-checks.json) | Twelve passing groups, including empty-value and complement counterexamples |
| [event-checks.py](event-checks.py) | Exact finite deals and counterexamples supporting this walkthrough |
| [research/event-checks.json](research/event-checks.json) | Eight passing groups; 4,290 deals and 21,450 deal/decision worlds |
| [Archived example](archive/kernel-surface/README.md) | Previous walkthrough and TypeScript declarations; superseded as the main design |

The `event` field, typed `true` in projections, event-valued `Pack`, and explicit
Boolean Event heads are implemented extensions on `codex/event-algebra`, absent
from public v1.3.1. Full relation-query and measured-source forms remain proposed.
Existing relation declarations,
closed rosters, selected containments, pointwise key meanings, and unweighted
roster capacities supply the rest of the notation. See the
[type proposal](../event-surface.md) and [TypeSafe/name comparison](../naming.md).

## 1. A card's location is a set of worlds

Start with these relations from the full macro:

```rust
relation PositionCard { position: u64 as PositionId, card: u64 as CardId }
relation At {
    position: u64 as PositionId,
    card: u64 as CardId,
    place: u64 as PlaceId,
    when: event,
}

At(position, card, when) -> At;
PositionCard(position, card, true) -> PositionCard;
PositionCard(position, card, true) == At(position, card, when);
```

`PositionCard` contains all fifteen physical cards. The ordinary closed `Card`
roster has three copies of each role. Its scalar key, vocabulary containment,
and an unweighted count of fifteen guarantee that no card is omitted or invented.

The event laws then say something stronger:

- Fix a position, a card, and a world. The key permits at most one location.
- The mirror covers the entire world space, so that card always has a location.

Every world contains the same fifteen cards, each somewhere exactly once.
Card conservation holds in every admitted possibility, rather than merely on
average. The probabilities of one card's alternative locations sum to one
because those locations partition the normalized world space.

`true` is the full space in the aligned perspective. It does not allocate
another random draw. `when` is a symbolic region, not a percentage or a stored
list of world IDs; finite enumeration below is a reference explanation.

## 2. Hands and the court use the existing alternatives pattern

The location roster is `Court`, `Hand`, `DrawFirst`, and `DrawSecond`. Sidecars
carry each location's payload:

```rust
relation Influence {
    position: u64 as PositionId, card: u64 as CardId,
    seat: u64 as SeatNoId, slot: u64 as SlotId,
    revealed: bool, when: event,
}

At(position, card, when | place == Hand)
    == Influence(position, card, when);

Influence(position, seat, slot, when) -> Influence;
HandSlot(position, seat, slot, true) -> HandSlot;
HandSlot(position, seat, slot, true)
    == Influence(position, seat, slot, when);
```

The full file declares the exact target keys, other sidecars, and their
containments. The parent `At` key owns disjointness across all locations, just
as the cookbook's zone ledger owns disjointness across interval sidecars.
The extra influence key prevents two cards occupying one hand slot in a world.
Slot coverage requires a card in every slot in every world.

Each participant has two slots, including after elimination. Losing influence
changes `revealed` to true; it does not remove the card from conservation.
Queries for usable influence select `revealed == false`.

The complete schema supports two through six participants. `GameSize` chooses
the corresponding ordinary roster count. A position's size is pinned to its
game, and its distinct participant rows must belong to the game's roster and
have the same count. Thus every position includes every participant. Two
distinct slot IDs from the two-slot roster similarly cover each participant's
slots. These are counts of physical roster members, not probability weights.

Physical card handles remain internal. A player receives roles they are
allowed to see, never handles they could join to the fixed role catalog.
The court is an unordered set; its random-draw law is an explicit fair-shuffle
assumption, not something inferred from its row order.

## 3. An exchange is a partial region with two occupied buffer slots

`PendingExchange(position, seat, when)` describes the worlds where that player
is choosing an exchange. Its pointwise key on `(position, when)` permits only
one exchange owner in a world. These two mirrors require both drawn cards:

```rust
PendingExchange(position, seat, when) == FirstDraw(position, seat, when);
PendingExchange(position, seat, when) == SecondDraw(position, seat, when);
```

Each draw relation has its own pointwise key and a matching location arm in
`At`. The buffers cover exactly the pending-exchange event. They cannot contain
the same physical card in the same world because that would violate `At`'s key.
Six hand cards plus two drawn cards leave seven in court in a three-player game.

This uses the distinction between a full space and a proper event. If exchange
occurs with probability one half, the first-draw alternatives have raw total
probability one half. Conditional on exchange, they total one. The mirror
preserves that evidence mass; it never silently normalizes a partial region.

First and second distinguish the two draw outcomes of a without-replacement
sampler. The later selection uses the available cards and returns two to court;
it need not treat draw order as strategically meaningful. Revealed losses stay
in their original hand slots. Completing the exchange is a transition contract.

## 4. A lie is still a valid fact

The schema includes ordinary `Move`, `Target`, `Claim`, `ActionClaim`,
`BlockClaim`, and `Challenge` relations. For example, the claim that Bob declared
Tax is a fact in the game log. Whether Bob holds a live Duke is an event over
the possible worlds.

| Declaration | Claimed role | Target rule |
| --- | --- | --- |
| Tax | Duke | No target |
| Assassinate | Assassin | One target |
| Steal | Captain | One target |
| Exchange | Ambassador | No target |
| Block Foreign Aid | Duke | Another player may block |
| Block Assassinate | Contessa | The target may block |
| Block Steal | Captain or Ambassador | The target may block |

Selected mirrors require targets exactly for Steal, Assassinate, and Coup, and
action claims exactly for the four character actions. The target-only block
law is the ordinary containment:

```rust
BlockClaim(game, action, speaker | kind == {Steal, Assassinate})
    <= Target(game, action, seat);
```

The full file pins closed role-rule tuples into keyed ordinary catalogs, since
today's closed targets support their ID projection rather than composite
`(kind, role)` probes. Seed those exact catalog rows at initialization.

There is no containment from a declaration into a player's hidden hand.
Even with all three Dukes face up, Bob can declare Tax. His Duke event is empty;
his claim remains structurally valid. Someone must challenge it for the bluff
to fail. Income and Coup have no character claim, and the block catalog has no
Coup entry. Actor eligibility and reaction timing remain protocol rules.

## 5. Query the event "Bob has a Duke"

Here is the proposed extension of today's query shape:

```rust
let held_roles = bumbledb_query::query!(Coup {
    interior fragments(position, seat, role, region: Event(e)) |
        Influence(position, card, seat, revealed == false, when: e),
        Card(id: card, role);
    interior fragments(position, seat, role, region: Event(Empty(given))) |
        PositionSeat(position, seat),
        Position(id: position, perspective),
        Perspective(id: perspective, given),
        Role(id: role);
    (position, seat, role, holds: Pack(region)) |
        fragments(position, seat, role, region);
});
```

`Pack` unions the regions in which some live card supplies that role. If Bob
has two Dukes in a world, that world contributes once. If a graph query reaches
the same evidence twice, its repeated region also contributes once.

An exact example shows why this matters. Alice initially holds Duke and
Assassin. Of the other thirteen cards, two are Dukes. Bob's two-card hand has
78 equally likely possibilities, and 23 contain at least one Duke:

```text
P(Bob has Duke) = 23/78 ≈ 29.49%.
```

Adding the probabilities of the individual Duke cards instead gives `24/78`.
The extra contribution is the hand containing both Dukes. Event union produces
the right result through its ordinary algebra.

Now suppose two Captains are revealed and the last live Captain is either
Bob's or Cleo's, with probability one half each. Let their holding events be
`B` and `C`. Card conservation gives:

```text
B ∩ C = empty       P(B ∩ C) = 0
B ∪ C = true        P(B ∪ C) = 1
```

Multiplying their marginal probabilities would incorrectly give one quarter.
No statistical independence follows from querying two people or two rows.

[queries.rs](queries.rs) retrieves both regions using ordinary joins, then
computes `Event(b & c)` or `Event(b ^ c)` in a query head. Reusing one
query variable for two `when` fields would ask for equal values; it must not
silently become an intersection join. Extending `Pack` does not change this.

An absent packed group stays absent in today's query semantics. The second
`fragments` arm explicitly seeds every admitted position/seat/role group with
an empty event. An impossible role therefore has a value to complement. A
missing position or perspective stays missing. The new query contract retains
scoped empty values through both queries and storage. A stored empty role answer
retains its row; a sparse relation still needs the explicit seeds above.
See [query algebra](../query-algebra.md) for this distinction and stage lowering.

The [query walkthrough](query-walkthrough.md) builds larger expressions. In the
initial deal, `BobCaptain & !(CleoCaptain | CleoAmbassador)` has probability
`189/1430`, about 13.22%. Under the illustrative policy below, observing Bob
declare Tax changes Cleo's Duke probability from `23/78` to `5/21`. Her odds
change through the shared deck even though she has said nothing.

## 6. A perspective retains its observations

`Perspective` identifies the game, public/private revisions, view kind, and one
accumulated observation event named `given`. Ordinary alternatives give it
either a `PlayerView` payload naming the observer, or a `RefereeView` payload.
`Position` identifies a point along
that perspective's joint trajectories. `Observation` records individual event
predicates bound to the view. The event values retain the adopted normalized
source laws and coordinate identities; there is no `Belief(process)` field.

A fully resolved referee snapshot fits the same card schema using a singleton
world context: its actual locations have event `true`, and absent locations
have no row. The location and slot laws become ordinary exact card assignment.
The protocol supplies the actual deal and resolved draws; labeling a view
`Referee` does not itself prove that its source is the actual game. Future
forecasts can extend that known state with fresh outcome coordinates. The
singleton construction concerns resolved state, not certainty about future play.

For holding event `H` and observation event `G`, the query is:

```text
P(H | G) = P(H ∩ G) / P(G), when P(G) > 0.
```

The result also retains `P(G)` relative to the adopted source context. Reaching
the same observation through another join gives `G ∩ G = G`, not a second
independent observation. If evidence is impossible, the result is an explicit
impossible-observation result, never certainty or a silently repaired law.
With an unknown law family, conditioning uses its positive-evidence domain.

Computing `given` as the intersection of the selected observations is a
maintained derivation. Foreign keys alone do not compute it or prove that a
raw public log item was correctly translated into an event. The constructor
must bind each observation to the actual question, time, and authorized data.
The initial source context may already assume a known starting hand; evidence
probabilities are relative to that stated context, not an invented absolute
likelihood for every earlier event in the game.

Distinct perspectives can use different evidence and source laws. Their values
cannot be combined merely because their numeric probabilities match. Within
one perspective, all positions use an explicitly aligned joint history; old and
new hand facts are predicates on the same trajectories. Every represented
trajectory supplies a state at every declared position. Branches that have
ended may carry an explicitly absorbing state, rather than gaps in coverage.

An observer column is not access control. TypeSafe and players receive only
the relevant authorized history, their own private observations, and explicitly
hypothetical information used for forecasts. The referee's actual hidden hand
must not enter an opponent forecast.

## 7. A proof establishes the old hand and changes the new hand

`Proof` is a recorded occurrence, with the claim, player, role, and before/after
revisions. Its ordinary containment pins the shown role to the challenged
claim. The protocol must check that the card was actually live in the old hand.
`Loss` is a different recorded occurrence: its card remains revealed on the table.

Consider this three-player position, with no exchange pending:

1. Alice holds a Duke.
2. Bob has a revealed lost Duke and one live card.
3. Bob proves that his live card is the third Duke.
4. He returns it to the nine-card court, shuffles, and draws a replacement.

The draw pool has ten cards, exactly one Duke. The challenger loses an
influence, which remains on the table and does not alter the court pool.

Let `D_before` and `D_after` be Bob's live-Duke events at the two positions,
and let `G` contain the stated observations. Then:

```text
P(D_before | G)                = 1
P(D_after | G)                 = 1/10
P(D_before ∩ ¬D_after | G)      = 9/10
```

The proof remains true about the old position. It gives no timeless
`HasDuke(Bob)` assertion. The replacement constructor extends the same
trajectories with a fresh random outcome; it does not copy the old event.
This calculation asks about the instant after replacement, before another
claim provides behavioral evidence.

## 8. TypeSafe supplies decisions; event rows preserve their meaning

TypeSafe's **Noul** returns the probability of yes to one question. **Choice**
returns a distribution over a supplied option roster. Neither response supplies
the full joint deal and history by itself. The new `event` type is the database's
algebra for relating those uncertain outcomes. Noul has no separate confidence
field; preserve confidence only where the provider actually returns it.

For a complete next-move forecast, Choice fits the structure:

```rust
relation NextMove {
    decision: u64 as DecisionId,
    option: u64 as OptionId,
    when: event,
}

NextMove(decision, when) -> NextMove;
Decision(id, occurs) -> Decision;
Decision(id, occurs) == NextMove(decision, when);
NextMove(decision, option, when) <= Available(decision, option, when);
```

The first three laws make the moves a complete disjoint partition of the
decision's `occurs` event. A present decision can have `occurs = true`; a future
decision may occur only in some worlds because a player can be eliminated
before reaching it. The alternatives have total raw probability `P(occurs)`
and conditional total one whenever that decision is reached. This uses the
same partial-coverage law as the exchange buffer. The last
requires each chosen move to lie inside the worlds where that declaration is
available. `Available` has the corresponding pointwise key in the full schema.
For example, a Coup option needs sufficient coins and a live target. Tax does
not require possession of Duke. Computing availability from the rule protocol
is a declared derivation; containment checks the supplied regions.

`MoveOption` names a distinct declaration, including its target where required.
`OptionTarget` binds that target to a participant in the decision's position.
The roster can include options available in different worlds; unavailable
options receive no event there. Options that are impossible throughout the
scope can have a stored empty `NextMove` branch. The pointwise partition also
allows a sparse relation omitting such branches; scalar roster coverage is a
separate contract when one row per declared option is required.
A structurally legal branch assigned probability zero remains a nonempty Event;
the model's zero is not an availability rule. Explicit support restriction is
a separate source view.

For forecasting Bob, `PolicyCase` partitions the worlds by the information Bob
could use: his private roles and remembered observations, plus public history.
Each case has one retained `Forecast` response. Hypothetical states that Bob
cannot distinguish must use the same conditional policy; a forecast may not
peek at Alice's actual hidden card. Ignoring private memory is an explicit
modeling simplification, not something implied by current hand equality.

The adapter uses each Choice response to construct an action outcome conditional
on its case, then lifts those outcomes into the perspective's shared world
scope. Normalization is intrinsic to the constructor. It validates the response,
source binding, and availability rather than silently rescaling a malformed
vector. The schema's partition proof does not by itself prove that the events'
probabilities match the raw response; that remains a source-construction contract.

For one binary forecast, Noul can instead provide `P(Tax | a specified
pre-action information case)`, yielding Tax and not-Tax events. Several
separately returned Noul probabilities do not automatically form a categorical
policy or determine dependence between the propositions.

Nor can a Noul answer to "Does Bob hold Duke?" simply overwrite the probability
of an existing card event. It must be reconciled as information about the same
joint law, or retained as a separately scoped model judgment. Giving it a fresh
independent event would change the question.

## 9. A claim updates an event through inference about behavior

Return to Alice's initial Duke-and-Assassin hand, where `P(H)=23/78` for Bob's
Duke event. To make the inference inspectable, assume these illustrative
behavior likelihoods, not measured TypeSafe output:

```text
P(Tax | H)  = 4/5
P(Tax | ¬H) = 1/5
```

The decision constructor extends the legal deals with a conditional action
outcome. Let `T` be its Tax event. Observing the recorded Tax declaration means
including `T` in `given`. Then:

```text
P(T)       = 49/130
P(H | T)   = 92/147 ≈ 62.59%
```

The event calculation retains the 49/130 evidence mass. Repeating `T` through
a query leaves the posterior unchanged. Conditional rates from a predictor
must be forecast from the context before the action, without leaking the answer
"Bob just declared Tax" into its input. A two-class model based only on Duke
possession deliberately abstracts or averages over his other card and memory.

If separately justified source constraints allow truth rates in `[0.7,0.9]`
and bluff rates in `[0.1,0.3]` over the full rectangular parameter domain, the
same events give a posterior range `[161/326,207/262]`, about 49.39%–79.01%.
Every underlying action law is still normalized. This interval comes from the
declared family, not from a Noul confidence field or arbitrary interval width.

Repeated future decisions allocate new outcomes. Reusing the same player's
unknown behavior parameters is a separate, explicit stability assumption.
A hand replacement need not replace those parameters. A Bayesian prior over
them is another assumption; ordinary uncertain events do not manufacture one.

## 10. What the schema proves, and what the game protocol supplies

| Concern | What establishes it |
| --- | --- |
| All fifteen cards exist once per world | Closed physical roster, scalar roster completeness, `At` key and full-space mirror |
| A hand slot has exactly one card per world | Slot roster, influence pointwise key and mirror |
| Exchange has two distinct drawn cards in its pending worlds | Two buffer mirrors plus shared card-location disjointness |
| A declared role and target have the correct shape | Closed catalogs, selected mirrors, ordinary containment |
| Next-move events form a normalized choice when the decision occurs | Normalized source law, pointwise key, exact coverage of `Decision.occurs` |
| A selected move is within its supplied availability event | Event containment |
| A shuffle is uniform and without replacement | The declared draw constructor; card laws alone do not specify probabilities |
| The next position legally follows the old position | A rule transition constructor, preserving the pointwise state invariants |
| Observations and policy cases mean what their labels say | Correct log translation, information boundaries, and source bindings |

The move protocol still owns turn order, live participants, actor/target and
challenger/claimant inequality, reaction windows, mandatory Coup at ten coins,
and the order of challenge/block resolution. It owns coin costs, the refund
after a successful challenge of an action claim, the retained cost after a
successful block, and returning eliminated players' coins. `Balance` provides
one balance per participant per world; this example does not model the entire
treasury as a conserved coin ledger.

The protocol also owns prove-or-concede choice, fair replacement, chosen
influence losses, and the possibility of two successive losses during an
assassination. A player may concede despite holding the required role, so
"cannot prove" and "will lose the challenge" are different events. Exchange
selection preserves lost cards and returns two cards. Append-only history,
revision progression, and competing reaction requests require explicit rules.

Choosing a best move additionally requires an objective, future player policies,
and a horizon. The event type provides exact questions about an adopted finite
scenario; the schema does not supply a strategy or an unbounded equilibrium solver.

## Sources and validation

The base rules were read from [UltraBoardGames' reproduction](https://www.ultraboardgames.com/coup/game-rules.php),
retained as [rule source](https://www.ultraboardgames.com/coup/game-rules.php) and [rule page](https://www.ultraboardgames.com/coup/game-rules.php).
The earlier publisher fetch returned HTTP 406; this is a reproduced rule source,
not a claim to have inspected an official publisher PDF. The two-player starting
player receives one coin. Reformation and the alternate two-player setup are
outside this example.

TypeSafe's [Noul documentation](https://docs.typesafe.ai/primitives/noul) was
rechecked for this rewrite; the [retained snapshot](../review-evidence/jev-noul-event-comparison.md)
matches the earlier document. The macro shapes were checked against the native
cookbook and parsers, including exact projected target keys and the zone-ledger
sidecar pattern. New event syntax remains explicitly proposed.

Run `python3 proposal/coup/event-checks.py` from the repository. Eight groups
pass: full card/slot partitions, overlap/gap counterexamples, Duke union,
exclusive Captains, Tax conditioning and evidence, proof replacement, partial
exchange buffers, and a structurally legal bluff with no live Duke. The oracle
enumerates 4,290 three-player deals and 21,450 deal/decision worlds; its buffer
case includes a nonuniform law. These are finite semantic checks, not native
macro compilation, engine admission tests, or a proof of every game transition.

Run `python3 proposal/coup/query-checks.py` for the twelve additional groups
covering staged query semantics and the compound expressions in the
[query walkthrough](query-walkthrough.md). The [saved output](research/query-checks.json)
records exact fractions; this evaluator does not execute Free Join or compile
the proposed macro syntax.

Revision 0.9 retains the initial [resident representation](../representation.md)
and retains the [source descriptor](../source-law.md) and scope/equality boundary.
Canonical per-fact persistence is implemented for finite unmeasured Events in
[BEVT v1](../../docs/event-value-format.md); measured-source persistence remains open.
[Query algebra](../query-algebra.md) specifies the stage behavior.
The [implementation plan](../implementation-plan.md) and
[native ledger](../../docs/event-implementation.md) separate implemented schema
and query slices from the complete executable Coup acceptance gate. The
[larger algebra](../world-relations.md) adds relational composition, constructive
permissions, and finite fixed points over declared admissible states.
