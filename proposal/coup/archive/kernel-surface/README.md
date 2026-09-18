**Historical proposal.** The current event-based rewrite is [here](../../README.md).

# Archived Coup kernel-surface example

**Probability surface revision:** [nouls as events](../../../event-surface.md) now
owns the proposed field denotation. The ordinary card/claim schemas and worked
game examples below remain useful; their probability sketches need reconciliation
with that event surface and are not a second preferred API.

**Coup is the lead application for the probability proposal.** This is a schema design and worked discussion, with no playable engine. The scope is base Coup with the ordinary 15-card setup, including the starting-coin exception for two players. Reformation and the alternative two-player draft setup are separate rulesets.

The central design choice is to represent **what exists, what was said, and what a player can infer** as different kinds of facts. All three meet at one public event, but they obey different laws.

| Schema | Meaning | Status |
| --- | --- | --- |
| [schema.ts](schema.ts), [vocabulary.ts](vocabulary.ts) | Referee card locations, seats, coins, temporary exchange cards | Concrete declarations using the current bumbledb API |
| [claims-schema.ts](claims-schema.ts) | Actions, targets, role claims, blocks, challenges | Concrete declarations using the current bumbledb API |
| Observation and belief relations below | Historical information boundaries and the joint stochastic model | Relational sketches; probability fields and laws are proposed |

The two concrete schemas share the `Game` and `Seat` declarations. They describe complementary parts of one game. Creating two databases would require an explicit synchronization contract; these files do not establish one. Nor does a schema name supply access control.

## 1. Fifteen cards, each in exactly one place

The referee's snapshot is:

```text
Card(id, role)                                      closed: 15 physical cards
Game(id, publicRevision)
Seat(game, seat, name, coins)

CardState(game, card, place)                        key(game, card)
  Court     → CourtCard(game, card)
  Influence → InfluenceCard(game, card, seat, slot, revealed)
  Exchange  → ExchangeCard(game, card, seat)

PendingExchange(game, seat)                         key(game)
```

`Card` contains three fixed copies of each of the five roles. A `CardState` must name one of those cards, its `(game,card)` key makes that identity unique, and capacity requires exactly 15 such facts per game. Together these laws account for the entire deck. No shuffle can manufacture a fourth Duke.

The existing high-level language expresses the location sum directly:

```ts
...alternatives(cardKey, "place", Place, {
  Court: key(CourtCard, ["game", "card"]),
  Influence: key(InfluenceCard, ["game", "card"]),
  Exchange: key(ExchangeCard, ["game", "card"])
})
```

The supplied keys must also be declared once, as the complete file does. The helper expands to ordinary containment and mirrors laws. Every parent has its selected payload; no card can simultaneously be in a hand and the court.

Each seat has exactly two influence slots. A lost card stays in its slot with `revealed=true`, including after elimination. During an exchange, the player still has those two slots, including any lost card, plus exactly two drawn cards in the exchange buffer. The private selection then replaces only live cards and returns exactly two cards to court. This temporary zone lets conservation hold during the choice rather than pretending the drawn cards disappeared.

The court is an unordered set of physical cards. The proposed draw kernel samples uniformly without replacement under the fair-shuffle assumption. Players never receive physical card IDs: their fixed role mapping would reveal hidden cards, even if the identifiers were renamed to opaque numbers. Public records expose a role when it is revealed; a player's private view exposes their own roles and private draws.

The schema admits dealt games with two through six seats. Eliminated seats remain. An explicit `clockwise` payload determines seat order; declaration order is not an implicit numerical rank.

## 2. A bluff can satisfy every schema law

Public declarations have this shape:

```text
Action(game, id, turn, actor, kind)
Target(game, action, seat)

Claim(game, id, speaker, role, origin)
  Action → ActionClaim(game, claim, action, actor, kind, role)
  Block  → BlockClaim(game, claim, action, speaker, kind, role)

Challenge(game, id, claim, challenger)
```

Closed payload rosters carry the action/role rules:

| Declaration | Claimed role | Target requirement |
| --- | --- | --- |
| Tax | Duke | No target |
| Assassinate | Assassin | One target |
| Steal | Captain | One target |
| Exchange | Ambassador | No target |
| Block Foreign Aid | Duke | Another player may block |
| Block Assassinate | Contessa | Only the target may block |
| Block Steal | Captain or Ambassador | Only the target may block |

Selected mirrors require a target exactly for Steal, Assassinate, and Coup. Another selected mirror requires a correctly attributed claim exactly for the four character actions. Claim alternatives link each claim to its action or block. Positional containment pins the repeated actor, action kind, and role columns to their authoritative rows.

For example, the target-only block rule is an ordinary containment:

```ts
contained(
  on(select(BlockClaim, { kind: ["Steal", "Assassinate"] }),
     ["game", "action", "speaker"]),
  on(Target, ["game", "action", "seat"])
)
```

And the allowed blocking role is another:

```text
BlockClaim(kind, role) ⊆ AllowedBlockRole(kind, role)
```

The current API addresses closed targets by their synthetic `id` only. The code therefore pins the closed role-rule facts into ordinary `AllowedActionRole` and `AllowedBlockRole` relations: every closed ground tuple must exist there, ordinary IDs must belong to the roster, and the ordinary ID key excludes any different payload for that ID. Their declared `(kind,role)` keys support the composite probes. These required catalog facts belong in the initial data alongside the game facts.

**There is deliberately no `Claim(speaker,role) ⊆ ActualHand(seat,role)` law.** If all three Dukes are face up, Bob may still declare Tax. His claim has valid structure and his chance of proving it is zero. Someone must challenge it for the bluff to fail.

Likewise, Income and Coup have no character claims to challenge. The role roster for blocks contains no Coup entry, so the declaration schema cannot attach a block to Coup.

This is the distinction worth preserving: the fact “Bob claimed Duke” is true even when “Bob holds Duke” is false. TypeSafe's opinion about the latter cannot change the former or make a legally shaped bluff inadmissible.

## 3. Proof, loss, and a new hand are different events

Historical observations need more structure than `Revealed(player,role)`. Proposed event relations include:

```text
HandVersion(game, player, epoch, publicRevision)
CurrentHand(game, player, epoch)
RoleProved(game, event, challenge, player, slot, role, fromEpoch, toEpoch)
InfluenceLost(game, event, player, slot, role, cause, fromEpoch, toEpoch)
ExchangeCompleted(game, event, player, fromEpoch, toEpoch)
PrivateObservation(game, event, recipient, kind, payload)
```

These are sketches, not additional implemented schema constructors. Event payloads would use closed alternatives, and keys/containments would bind each event to its game, participants, cause, and hand versions. A role proof must match the challenged claim; an influence loss may reveal any live role the losing player chooses.

A proof says a matching card was in the **old** hand. The player returns it to court, shuffles, and draws a replacement. A loss leaves the shown card on the table, unavailable for future draws. An exchange changes the live hand and gives its owner private observations. New epochs distinguish those states even if the replacement happens to have the same role.

Do not store the proof as a timeless `HasRole(Bob,Duke)` fact. Do not erase its evidence when the epoch changes either. Propagate that evidence through the transition into the new hand and the other hidden cards.

Here is a position with an exact, surprising answer:

1. There are three players, so nine cards are in court, and no exchange is pending.
2. You hold a Duke. Bob has a face-up lost Duke and one remaining live card.
3. Bob proves that live card is a Duke. All three Dukes are now accounted for outside court.
4. He returns the proven Duke to the nine-card court and draws a random replacement.

Immediately before replacement, Bob's live role is certainly Duke. Immediately after it, the court draw has one Duke among ten cards:

```text
P(Bob's new live card is Duke | this proof and position) = 1/10.
```

The challenger loses an influence, but that card stays on the table and cannot change this court count. We ask about the instant after replacement, before any subsequent claim supplies new behavioral evidence. No opponent-policy forecast is needed for this calculation.

This is a concrete job for the new algebra: condition on an observation, compose with a finite transition, then ask about a different outcome variable. Drawing Allen intervals around two percentages does not express that relationship.

## 4. Each observer needs one joint belief

The probability layer should bind a model to an information state:

```text
Perspective(game, observer, publicRevision, privateRevision, viewDigest)
Belief(id, perspective, modelVersion, process : proposed finite model)
PolicySource(id, game, subject, contextClass, version, law : proposed source spec)
UsesSource(belief, port, source)                     key(belief, port)
UsesObservation(belief, event)                      key(belief, event)
Assessment(id, belief, question, result)
```

Every reference needs the corresponding key and containment. `modelVersion` identifies the assumptions used for this belief; it is independent of the public and private information revisions. Multiple assumptions can be compared at the same perspective. The exact probability descriptor and source-capture format remain obligations of the main proposal.

The model's finite hidden state contains both:

- **Card allocation:** a whole legal assignment satisfying the same 15-card and location laws, scoped to that possible world.
- **Private information history:** the observations each player could remember, including cards seen and returned during exchanges.

Current allocation alone suffices for card counting, but not generally for predicting strategy. Two players with the same current hand can choose differently because they remember different cards in the court. A history-insensitive policy is a permitted explicit simplification. It must not be implied by calling a row `Hand`.

Conceptually, an inspectable expansion would prefix card-location facts with `(belief, world)` and apply conservation separately inside each world. The model carries probability jointly across worlds; individual location rows do not receive independent probabilities. Enumeration is an explanatory view, not a required storage format. For a finite observed history or bounded continuation, these histories and allocations form a finite carrier.

If only one Captain remains among hidden cards, Bob and Cleo cannot both have it. Two marginal answers of 50% each may be compatible, but their conjunction is exactly zero. The joint carrier retains that fact. Joining the same belief twice must refer to the same world; it must not allocate a new deal.

A source binding can also survive a hand change. Bob's behavior parameters and Bob's current hidden cards have different identities and lifetimes. A proof or exchange allocates a new hand outcome through a transition; it need not allocate a new opponent-policy source. Reusing a source requires an explicit stability/context assumption. Learning its parameters requires a declared prior or another justified update contract; an unspecified unknown rate is not automatically a Bayesian prior.

The reusable process retains unnormalized observation likelihoods. Posterior queries normalize on the positive-evidence domain, following [the core proposal](../../../proposal.md). Repeating the same public event through a join does not count it twice. Distinct declarations are distinct events, but multiplying their likelihoods still requires the declared conditional strategy process.

For one observed decision and its resolution, the composition has one form:

```text
w_next,θ(s′) = Σ_s w_now,θ(s) · π_θ(action | actor's information in s)
                             · K_θ(s′, observed resolution | s, action)
```

Here `s` is a complete hidden state, `π` is the declared decision policy, and `K` combines the rule transition with any modeled reactions and the observer's new evidence. The same source assignment `θ` is held fixed throughout. The schema constrains which states and transitions can receive weight; composition multiplies successive weights and sums alternative hidden paths. The total weight remains the evidence likelihood until a posterior is requested. A mere claim contributes a policy likelihood; a proof contributes a role observation and a replacement transition. These are applications of the same composition law.

Player privacy is an interface contract: send only that observer's authorized public history, private observations, and derived beliefs. A column named `observer` is not access control. Forecasting an opponent's behavior may use an **explicitly hypothetical opponent information state** from a possible world, never the referee's actual hidden answer.

## 5. TypeSafe models the player; the deck model counts cards

Suppose you initially hold Duke and Assassin and no cards have been publicly lost. Of the other 13 physical cards, two are Dukes. Bob has two cards. Under the ordinary uniform initial deal:

```text
P(Bob has at least one Duke) = 1 − C(11,2)/C(13,2) = 23/78 ≈ 29.49%.
```

Now he declares Tax. The declaration's evidential value depends on how he plays. For an illustrative two-class behavior model, suppose:

```text
P(Tax | has Duke, pre-action context)   = 4/5
P(Tax | lacks Duke, pre-action context) = 1/5
```

Conditioning the joint model on that declaration gives:

```text
P(has Duke | Tax) = (23 × 4)/(23 × 4 + 55 × 1)
                 = 92/147 ≈ 62.59%.
```

The rates are invented modeling assumptions for this example, not measured TypeSafe outputs. If externally justified constraints instead allow the truth rate in `[7/10,9/10]` and bluff rate in `[1/10,3/10]`, independently across this rectangular parameter domain, the exact posterior range is `[161/326,207/262]`, approximately **49.39%–79.01%**. That range follows from the declared model, not from a confidence-to-width conversion.

TypeSafe provides a plausible upstream source for behavior forecasts:

| Primitive | Concrete Coup question | Use of the answer |
| --- | --- | --- |
| Noul | Given this hypothetical private hand/history and the public position just before the action, how likely is Bob to declare Tax? | A proposed conditional behavior likelihood |
| Choice | Given this information state and this complete list of currently legal declarations, which will Bob choose next? | A proposed policy distribution, including bluffs and distinct targets |
| Score | Given this perspective, how threatening is each opponent on these explicitly defined ordered levels? | A stored ordinal assessment; numerical strategic utility requires a separate interpretation |

For a likelihood, the input must stop **before** the action being predicted. Including “Bob just declared Tax” would leak the answer into the forecast. Forecasts conditional only on “has Duke” must explicitly average over, or deliberately abstract away, his other card and private history. Supplying separate Noul answers does not establish a coherent joint strategy table; a Choice over a complete action list supplies one normalized row, still subject to the modeling/calibration contract.

Store the exact prompt/input digest, information revision, hypothetical-state identity, returned probabilities, resolved model, and question version. Retain reported confidence as ordinary metadata. The [TypeSafe primitives documentation](https://docs.typesafe.ai/primitives) describes typed probabilities; it does not turn a forecast into proven game truth. Its independently evaluated questions do not imply independent cards or independent bluffs.

## 6. Queries that make the schemas worth having

| Query | What the answer needs |
| --- | --- |
| Can Bob legally declare Tax even though every Duke is accounted for? | Declaration rules and phase eligibility; possession is not required |
| Can Cleo block a steal aimed at Bob? | Target containment: no |
| Can both opponents have Captain? | Support in the joint card model, not a product of marginals |
| What is Bob's current Duke probability after that proof? | Observation followed by replacement: 10% in the worked position |
| How likely is my challenge to succeed? | Probability the claimant cannot prove, plus their policy for conceding despite being able to prove |
| Can challenging this assassination cost my last two influences? | The challenge-loss continuation followed by assassination; a single boolean success flag is insufficient |
| Which action is best across every admitted bluff model? | An explicit utility/horizon and a common joint continuation model for comparing actions |

The last query can return a robustly preferred move, an unresolved comparison, or a counterexample model explaining the disagreement. “Best move” is not supplied by a schema alone. A winning-probability query additionally needs future player policies and a stopping/horizon convention. We do not introduce an unbounded equilibrium solver through the probability type.

## 7. What remains a transition contract

The concrete declarations prove structural laws about admitted snapshots and declarations. They cannot, by themselves, prove that one snapshot was reached legally from another. In particular, the move protocol must enforce:

- Live participants, actor/target inequality, challenger/claimant inequality, turn order, and reaction-window eligibility. The declaration schema already enforces target-only blocks of steals and assassinations.
- Costs and balances, mandatory Coup at 10 or more coins, and returning eliminated players' coins to the treasury. These files store public balances without yet specifying a coin-supply model.
- Refund of an action's cost when its original role claim is successfully challenged; retaining the spent cost when the action is successfully blocked.
- Prove-or-concede choice, role-correct proof, replacement draw, chosen influence loss, and potentially two successive losses during an assassination. Losing a challenge is not proof that the player lacked the role: they may concede despite holding it.
- Private exchange draws and selection, preserving face-up lost cards, observation recipients, and hand-version changes.
- A deterministic application convention for competing reaction requests and the continuation after each challenge or block. Base tabletop rules do not specify a network arbitration protocol.

That suggests phase payload alternatives such as `ChooseAction`, `ChallengeAction`, `RespondToChallenge`, `OfferBlock`, `ChallengeBlock`, `ChooseInfluenceLoss`, and `ChooseExchange`, with an explicit continuation describing what remains to resolve. These are protocol design sketches, not additional guarantees claimed for the current files. Event append-only behavior, revision progression, and the consistency of history with current state also require that protocol.

## Sources and validation

Base rules were read from [UltraBoardGames' reproduction](https://www.ultraboardgames.com/coup/game-rules.php), retained as [HTML](../../research/source-1.html) and [extracted text](../../research/source-1.txt). The publisher page returned HTTP 406; this is a reproduced rule source, not a claim to have inspected an official publisher PDF. The proof replacement, optional concession, cost refund, target-only blocks, two-player starting coin, and double influence-loss rules are explicit in that reproduction.

The TypeSafe documentation snapshots are retained in [the proposal evidence directory](../../../review-evidence/use-cases). Current schema syntax was checked against `ts/src/alternatives.ts`, `face.ts`, `closed.ts`, `selection.ts`, `statements.ts`, `capacity.ts`, and `schema.ts`.

All three Coup TypeScript files passed syntax checks. Exact arithmetic confirms the worked examples; results are recorded in [checks.json](../../research/checks.json). Package dependencies are not installed here, so these declarations have not been package-typechecked or admitted by the native engine. No game implementation or new probability runtime is included.
