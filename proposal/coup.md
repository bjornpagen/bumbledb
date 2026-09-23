# Coup as an ordinary BumbleDB schema

**Proposed Rust syntax, not code that runs on main.** This models the initial
hands for one deal, before drawing, exchange, or eliminating influence. The
caller owns game rules and constructs the possible deals. The database owns
row storage, dependencies and query execution.

Alice knows her two cards: Duke and Assassin. Fix their physical IDs. The
remaining thirteen cards give Bob an unordered pair and Cleo another unordered
pair: `C(13,2) × C(11,2) = 4,290` worlds. Undealt cards are the remainder;
deck order is absent. Sort each hand's physical card IDs into slots zero and
one so bookkeeping does not double the number of worlds.

## Schema and its actual guarantees

```rust
bumbledb::schema! {
    pub Coup;

    closed relation Role as RoleId = {
        Duke, Assassin, Captain, Ambassador, Contessa
    };

    relation Game { id: u64 as GameId, worlds: event }
    relation Player { game: u64 as GameId, seat: u64 as SeatId }
    relation Card {
        game: u64 as GameId,
        id: u64 as CardId,
        role: u64 as RoleId,
    }
    relation Slot {
        game: u64 as GameId,
        seat: u64 as SeatId,
        slot: u64,
        scope: event,
    }
    relation Holding {
        game: u64 as GameId,
        seat: u64 as SeatId,
        slot: u64,
        card: u64 as CardId,
        when: event,
    }

    Game(id) -> Game;
    Game(id, worlds) -> Game;
    Player(game, seat) -> Player;
    Player(game) <= Game(id);
    Card(game, id) -> Card;
    Card(game) <= Game(id);
    Card(role) <= Role(id);

    Slot(game, seat, slot) -> Slot;
    Slot(game, seat, slot, scope) -> Slot;
    Slot(game, seat) <= Player(game, seat);
    Slot(game, scope) <= Game(id, worlds);

    Holding(game, seat, slot) <= Slot(game, seat, slot);
    Holding(game, card) <= Card(game, id);
    Holding(game, seat, slot, when) -> Holding;
    Holding(game, card, when) -> Holding;
    Slot(game, seat, slot, scope) == Holding(game, seat, slot, when);
}
```

Insert a Game with full(U), three Players, fifteen Cards, and two Slot rows per
player with scope full(U). Supply a Holding region for each possible slot/card
assignment. Alice's two holdings are full; other holdings describe their
corresponding subsets of the 4,290 worlds.

The schema guarantees one card per declared slot in each point of its scope,
no physical card in two slots in the same world, and real card/slot references.
The Game containment anchors all Slot scopes to the same universe; the mirrors
then align their Holdings. Both scalar and Event keys on Game/Slot are needed
because target-key matching is exact. A scalar IND still checks an empty fact.

The fixture supplies three players, two slots, fifteen physical cards and full
parent scopes; these statements alone do not force those roster sizes or force
a root Event to be full. They validate the supplied partition. The caller is
also responsible for the meanings and completeness of the worlds themselves.
This is a database schema, not an encoding asking the engine to solve Coup.

## Ordinary grouped query: who can have a Duke?

```rust
let duke = query!(Coup {
    (seat, when: Pack(w)) |
        Holding(game == ?game, seat, card, when: w),
        Card(game == ?game, id: card, role == ?duke);
});
```

The card join is normal Free Join. Pack unions matching regions. A hand with
two physical Dukes contributes each world once to the result. The result is
an Event that can be saved, complemented, or used in a later query.

No matching row means no group for that seat. That is a query choice, not a
limitation of Event. The next example supplies explicit zero answers.

## A complete role roster and a useful compound answer

```rust
let steal_window = query!(Coup {
    interior claims(seat, role, region: Event(w)) |
        Holding(game == ?game, seat, card, when: w),
        Card(game == ?game, id: card, role);
    interior claims(seat, role, region: Event(Empty(worlds))) |
        Player(game == ?game, seat),
        Game(id == ?game, worlds),
        Role(id: role);

    interior held(seat, role, region: Pack(e)) | claims(seat, role, e);
    interior blockers(seat, region: Pack(e)) |
        held(seat, role, e), role in ?blocking_roles;

    (region: Event(b & !c)) |
        held(?bob, ?captain, b), blockers(?cleo, c);
});
```

Bind `blocking_roles` to Captain and Ambassador. The two adjacent `claims`
rules contribute ordinary rows to one stage. The seed rule supplies an empty
Event for every declared seat/role, including Alice's impossible Captain.
Pack merges the seed and the real claims. There is no implicit outer join or
new “all alternatives” subsystem.

The final Event is:

```text
Bob holds Captain ∩ complement(Cleo holds Captain ∪ Cleo holds Ambassador)
```

It describes a truthful steal for which Cleo lacks a truthful blocking role.
It predicts no bluff, challenge, action choice, or coin availability. Those are
caller decisions or additional ordinary game facts. Independent enumeration
finds 567 worlds in this Event. That is a set check, not a probability assignment
or a requirement to add world-counting queries.

The query returns one row even if its Event is empty, provided the supplied
players and blocker-role roster produce those groups. A nonexistent game/player
or empty blocker-role parameter set can instead give no body binding. Empty
values and missing roster data remain distinguishable.

## Evidence filters run during the join

```rust
let compatible_holdings = query!(Coup {
    (seat, slot, card) |
        Holding(game == ?game, seat, slot, card, when: w),
        EventRel(w, INTERSECTS, ?evidence);
});
```

Evidence is an Event supplied by the caller or a previous query. The filter
selects holdings compatible with it at the earliest available join node.
Different-universe evidence has no INTERSECTS matches. To diagnose that input,
use the `DIFFERENT_UNIVERSE` mask against the game's anchor or check descriptors
in the caller. Construction using foreign evidence would fail explicitly.

To return the surviving worlds too, change the head to include
`surviving: Event(w & ?evidence)`. The INTERSECTS filter establishes compatibility
before that constructor runs. To retain impossible holdings, use a
SAME_UNIVERSE filter instead and preserve the empty intersections.

For “is this holding certain under the evidence?”, ask `SUBSET(evidence, w)`.
An empty evidence Event satisfies subset vacuously, so add
`EventRel(?evidence, INTERSECTS, ?evidence)` when the question requires possible
evidence. No solver is involved in either query.

## A transaction that changes what is known

Learning a card does not require renumbering worlds. The caller can store a
narrower evidence Event and reuse the same Holding facts and universe. Queries
intersect with that evidence. If the caller instead replaces the universe,
construct the replacement values and update all affected anchors/partitions in
one transaction; final-state checking rejects an incomplete replacement.

Deleting one nonempty Holding without replacement exposes a coverage gap and
fails its Slot mirror. Replacing it with disjoint pieces covering the same region
can succeed in the same transaction, provided the card and slot keys still hold.
Saving an empty derived answer preserves its row. Re-inserting that identical
row remains idempotent.

## Where a model fits

Model judgments can be ordinary rows carrying source identity and whatever
result the application actually received. If a caller maps joint alternatives
or evidence to a region of these worlds, BumbleDB preserves and combines those
possibilities without collapsing them to one Boolean guess.

“70% Captain” alone does not identify the member deals or their dependence on
another judgment. Event does not invent the missing joint information. The model
or external solver supplies meaning; the database provides exact storage,
constraints and queries over the resulting values.
