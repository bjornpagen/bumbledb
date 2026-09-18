# The name is event

**Decision: `event` in schema fields, `Event` in Rust.** The user chose this name
after comparing the revised carrier with TypeSafe's Noul. It supersedes `noul`
as the public database type name. TypeSafe's product/API spelling remains Noul.

An event is the standard probability-theory term for a measurable set of
outcomes: the worlds in which a statement holds. Our finitely presented proposal
captures admissible worlds and optionally designates one normalized probability
law or a constrained family of such laws. The contract distinguishes admissibility
from positive mass. A relation is an Event on a product presentation with checked
faces, exposing composition, domain, modalities, residuals, and finite closure.
Boolean combinations operate on one-space predicates. [Source-law.md](source-law.md)
and [representation.md](representation.md) state the cross-scope qualifications.

## How close is it to TypeSafe's Noul?

TypeSafe's current [documentation](https://docs.typesafe.ai/primitives/noul) says:

> A Noul answer is a single number, `noul`, the probability that the answer is yes.

It also explicitly says that Noul does not return a separate confidence value.
Its request binds the number to a yes/no question, input state, and optional
true/false criteria. It is not a degree of skill or a three-valued logic;
0.5 gives the two answers equal probability.

| Aspect | TypeSafe Noul | bumbledb `event` proposal |
| --- | --- | --- |
| Central object | A model's numeric probability for yes to a supplied question | A region of admissible worlds, with optional captured measurement semantics |
| Typical result | `{"type":"noul","noul":0.63}` | A region representing the named yes outcome; probability can be queried |
| Equality | Equal scalar results can describe different questions | Region equality in an aligned world/source context |
| Related propositions | Separate scalar answers do not supply a joint distribution | Shared regions preserve overlap, complement, implications, and dependency |
| Negation | No has probability `1-p` for the same binary question | Relative event complement, followed by measurement if wanted |
| Compound queries | The documentation does not expose event-set algebra over returned numbers | Construct regions, compose world relations, derive possible/guaranteed outcomes, and measure them |
| Uncertain laws | The documented response is a point probability | An event can be measured across a constrained normalized law family |
| Provider dependency | A TypeSafe inference primitive | Can arise from a deal, shuffle, rule, query, Noul, Choice, or Score source |

For one isolated binary question, there is a straightforward bridge: an adapter
binds the request identity and the returned probability to a normalized binary
source, then exposes its yes event and complementary no event. This does not
recover how that proposition relates to other already-bound events. Nor does
allocating a new source imply independence.

For example, `P(Bob has Captain)=0.5` and `P(Cleo has Captain)=0.5` do not say
whether the two propositions are identical, independent, or mutually exclusive.
An event model of the legal cards can establish the last relationship when
only one live Captain remains. If a Noul estimate is meant to constrain that
existing card event, it must be reconciled with the same joint law rather than
minted as an unrelated binary outcome.

A Choice response supplies a whole categorical distribution. Its adapter can
construct a partition with one event per option. Score supplies a distribution
over described levels; its branches can be imported in the same way, with rank
or utility interpreted separately. This makes `event` broader than the Noul
primitive without implying that TypeSafe returns world structure or exact
calibration guarantees.

## Alternatives considered

| Name | Assessment |
| --- | --- |
| `event` / `Event` | Chosen: standard mathematical name for the denotation |
| `proposition` / `Proposition` | Reasonable logical reading, but may suggest a formula or unevaluated query rather than its extensional region |
| `region` / `Region` | Describes the set structure but is broader than the probability domain; potentially useful for a shared interval/event abstraction later |
| `noul` / `Noul` | Earlier product name; too easy to confuse with the provider's scalar response |
| `probability`, `chance` | Name a measure of the region rather than the region |
| `credal`, `belief` | Suggest a law family or an agent's assessment; those supply context for the event but are not its denotation |

The main naming collision is with events in a historical log. The Coup example
uses `Move`, `Proof`, `Loss`, and `Observation` for recorded occurrences, and
`when: event` for a mathematical region. "Bob proved Duke" is a recorded fact;
"Bob has a Duke after replacement" is an event whose probability can be 10%.

Kleene algebra with domain names an important algebraic lineage, not a better
field name. Its concrete relations explain the passage between actions and
predicates. `Event` still names the region that rows carry; `WorldRelation`
names a checked view of that same region. Neither name claims that an isolated
Noul response already contains hidden-world relationships.

## Evidence

The Noul page was fetched again from
`https://docs.typesafe.ai/primitives/noul.md` during this revision. Its SHA-256 is
`402a2f30ca71b53227058a5df3fb3e0a10063588a31b883f924d6660815c6916`, matching the
earlier retained snapshot. [Current retained copy](review-evidence/jev-noul-event-comparison.md).
The documented scalar response and absence of a separate confidence field are
directly observed; the event adapter and its algebra are our proposal.
