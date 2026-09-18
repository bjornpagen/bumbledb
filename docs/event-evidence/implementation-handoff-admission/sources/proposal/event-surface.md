# Event through schemas and queries

The public type is `event` / `Event`. It denotes a region of explicitly admissible
worlds, optionally with a designated normalized measurement law. Its resident
identity is a scoped canonical region handle. Revision 0.9 starts with completed
symbolic functions and essential-coordinate tables; the laboratory preserves
the competing carriers. See [the proposal](proposal.md) and
[memory layout](representation.md). This page specifies the public meanings.
The `event` field, owned Rust values and ordinary binding/equality paths now
exist on the implementation branch, together with pointwise field dependencies.
Contextual `true`, source constructors and computed heads below remain proposed. See the
[native ledger](../docs/event-implementation.md) before using an example as code.

## 1. The interval analogy is pointwise

| Interval | Event |
| --- | --- |
| Region of temporal points | Region of legal worlds |
| A point is an instant | A point fixes source parameters and finite outcomes |
| Intersection is simultaneous occupancy | Intersection is simultaneous truth |
| Union is coverage | Union is alternative truth, including overlap once |
| Duration measures the region | Probability measures the region under its law |

A percentage does not identify an event. Equal probabilities can describe equal,
complementary, independent, or otherwise dependent events. The value retains
which worlds it refers to. Its source identity is captured, not a public outcome
type parameter. Finite outcome vocabularies remain ordinary closed relations.

## 2. Card conservation from FD/INDs

An excerpt from the [complete Coup schema](coup/schema.rs):

```rust
relation PositionCard {
    position: u64 as PositionId,
    card: u64 as CardId,
}
relation At {
    position: u64 as PositionId,
    card: u64 as CardId,
    place: u64 as PlaceId,
    when: event,
}

PositionCard(position, card) -> PositionCard;
At(position, card) <= PositionCard(position, card);
At(position, card, when) -> At;
PositionCard(position, card, true) -> PositionCard;
PositionCard(position, card, true) == At(position, card, when);
```

Fix a position and a physical card. The pointwise key forbids two distinct
stored facts from owning the same world. The mirror requires the locations to
cover the whole aligned space. Thus the card is somewhere exactly once in every
world. The full schema supplies the closed card/place rosters and exact keys.

Typed `true` in an event projection means the full aligned universe. Constant
faces are a real parser/IR extension. The explicit source key preserves the
engine's exact-target-key discipline rather than assuming an implied key.
An empty or incompatible source domain cannot prove coverage vacuously.

For location events `E1,...,En`:

```text
Ei & Ej = empty, for distinct contributing rows
E1 | ... | En = full
P(full) = 1
therefore sum_i P(Ei) = 1
```

There is no probability weight column or numeric normalization capacity. The
same proof holds at every admitted source-parameter assignment. Bounds on the
individual probabilities need not add to one separately.

## 3. Partial events preserve their mass

Future decisions need not be reached. The actual Coup schema therefore uses:

```rust
NextMove(decision, when) -> NextMove;
Decision(id, occurs) -> Decision;
Decision(id, occurs) == NextMove(decision, when);
NextMove(decision, option, when) <= Available(decision, option, when);
```

The alternatives partition `occurs`, so their raw total is `P(occurs)` and their
conditional total is one when the decision is reached. A partial event does not
become a normalized universe. Availability is a supplied region checked by
containment; the game protocol derives it. Claiming Duke need not imply holding
Duke, because bluffing is legal.

## 4. Import a source, then insert its event branches

An illustrative constructor for a present decision with two available options:

```rust
// Proposed authoring API. Exact decimals, one named categorical outcome.
let choice = Event::categorical(decision_scope, [
    (tax_option,    "0.8"),
    (income_option, "0.2"),
])?;

for (option, when) in choice.branches() {
    // Every declared option has an Event value, including empty where appropriate.
    tx.insert([&NextMove { decision, option, when }])?;
}
```

Insert the parent/roster and complete branch set in one transaction. Iteration
preserves the declared alternative roster; a legal option with probability zero
still has a nonempty branch. Explicitly filtering structurally empty branches
is an application choice that changes row existence. The constructor validates
a normalized law and binds the actual option identities;
it never rescales a malformed response. For a conditional/future decision,
construct the policy within its declared information cases and lift branches
into `occurs`. Do not apply the present-decision example as if every future turn
were certain.

TypeSafe Choice supplies an option distribution; Noul supplies the probability
of one binary question. A Noul adapter constructs the named event and its
complement. A source response does not determine the dependence among unrelated
preexisting events. Reconcile constraints with the existing law or retain a
separate scoped judgment. Noul has no separate confidence output.

[Source-law.md](source-law.md) defines named sources, finite guards, explicit
admissibility, optional exact law functions, and alignment. [Naming](naming.md) explains the
provider/type distinction. Source allocation, response provenance, and actual
question meaning are constructor responsibilities, not inferred by FD/INDs.

## 5. Query values form a total Boolean algebra

```rust
when: Event(a & !(b | c))
covered: Pack(when)
chance: Probability(when, given)
```

These are three head forms used in separate stages where their outputs depend
on each other. Event construction returns a value even when it is empty. Pack
unions matching regions; an absent group remains absent unless explicitly
seeded. Probability observes a fully bound event under explicit evidence.
[Query algebra](query-algebra.md) gives the grammar and staging rules.

The owned `Event` and stored event fields include full and empty. Structurally
impossible branches can be stored as empty values and later complemented.
A nonempty admissible branch with probability zero also remains representable.
Query emptiness, missing rows, impossible evidence, MissingLaw, and source failure
remain distinct outcomes. The [storage contract](event-storage.md) states the
corresponding pointwise-key and planner obligations; Allen's nonempty-key
distinctness shortcut needs a separate nonemptiness proof for Event.

The [larger relation algebra](world-relations.md) operates on owned typed faces
of the same Event representation. The [operator catalog](event-algebra.md)
specifies ITE, cardinality, structural tests, information abstraction and exact
expectation. [TypeSafe inference](typesafe-inference.md) defines imports that
preserve uncertainty and the existing proposition's meaning.

Repeated evidence is idempotent, `G & G = G`. Probability of `E | given` means
`P(E & given)/P(given)` on its positive domain, preserving the denominator.
Unknown parameters acquire no prior. Distinct source names acquire no independence.
The source construction and law measurement share exact coordinates with the
canonical event representation.
