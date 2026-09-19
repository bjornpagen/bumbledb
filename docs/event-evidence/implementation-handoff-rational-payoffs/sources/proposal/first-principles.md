# From a Coup row to an event value

A normal row might say which seat belongs to a game. A functional dependency
(FD), written as a key, says the selected columns determine the entire row:

```rust
Seat(game, seat) -> Seat;
```

For one game and seat, there cannot be two distinct seat records. An inclusion
dependency (IND) requires every selected tuple on the left to occur on the right:

```rust
PositionSeat(game, seat) <= Seat(game, seat);
```

Every participant in a position must belong to that game. The combination matters:
matching the game and seat independently would not establish that they belong
together. A selected projection is also called a face. A mirror `==` means
inclusion in both directions.

Intervals extend this idea to points. A row can cover a stretch of time; a
pointwise key forbids two competing rows at the same instant, and containment
requires the source stretch to be covered by matching target stretches.

Event uses the same meanings over legal worlds. Imagine every possible Coup deal
and finite continuation consistent with the supplied sources. A row's `when`
selects the worlds where its card is in its stated location. A pointwise key
means one location per card per world. Coverage of `true` means every card always
has a location. Their combination conserves physical cards in every possibility.

The database need not store those worlds individually. An Event's resident value
is a sixteen-byte key. The initial backend shares symbolic decisions and small
truth tables over the coordinates a condition actually needs. A canonical
completion gives raw encodings their legal meaning. The key selects a region
and its polarity; toggling one bit selects its complement. The owned value
also retains its space so these references remain valid. Dense and packed
representations remain controls, rather than different public mathematical types.

That representation follows the algebra:

```rust
Event(!a)        // worlds where a is false
Event(a & b)     // worlds where both hold
Event(a | b)     // worlds where either holds
Event(a ^ b)     // worlds where exactly one holds
```

The result of each is another Event. Empty and full are real values. Joining two
rows first binds their event values; intersecting those values is explicit.
Repeating the same event through multiple join paths does not strengthen it.

Probability is a separate observation. The same event region can be measured
under each admitted setting of an unknown source parameter. Two events with the
same probability can describe completely different worlds. A displayed range
therefore cannot replace the event. Unknown parameters are not random until an
explicit prior constructor says so.

For Coup, a fair deal supplies one joint source. TypeSafe can supply a conditional
policy for what a player might declare given the information available to them.
Those inputs form a joint deal/decision space. Observing Tax selects part of it.
The database can then ask about any event on that same space, including another
player's hand. It does not need another model call for every consequence.

But Boolean combinations are only the first piece. A region can also contain
**pairs of positions**: every legal before/after pair for a move. It is still
an Event; a checked pair of faces tells the engine which coordinates mean
before and after. Composition connects the after side of one move to the
before side of the next, then hides the middle position. That is a join and
projection over possible states, represented without a row for every state.

Now start with a goal such as “Alice remains alive.” `May(move, goal)` constructs
the starting positions with at least one successful continuation.
`Must(move, goal)` constructs the positions where the move is available and
every allowed continuation meets the goal. These results are Events: they can
be joined, packed, complemented, stored, or measured like the originals.
A residual goes further: given an allowed overall behavior, it constructs the
largest set of continuations that can satisfy it. On a sealed finite state
space, fixed points compute reachability and invariant safety.

That is the deeper Allen analogy. The mathematical object gives us operations
that build other useful objects and make containments constructive. We are not
limiting Event to a list of canned probability questions.

For example, suppose Alice must Coup either Bob or Cleo. Both have one live
card; exactly one is the last live Assassin. Every hidden deal has a target
that removes the Assassin. But Alice cannot choose that target reliably without
knowing which deal she is in. The algebra can distinguish “some action works
in each world” from “one action works throughout this information case.”
Regardless of which target Jev favors, both choices spend seven coins and
remove one influence. Those derived facts are certain even while the target
remains uncertain.

Finally, possible does not mean probable. If Jev gives a legal choice zero
probability, it stays in the structural universe until a separate rule or
explicit restriction excludes it. A forecast helps measure choices; a guarantee
quantifies over the declared possibilities. The same representation supports
both questions without treating a model's confidence as a rule of the game.

Read [the proposal](proposal.md) for the representation, [event surface](event-surface.md)
for the schema, and [Coup queries](coup/query-walkthrough.md) for exact examples.
Everything here is proposed engine behavior, with reference checks and isolated
machine-code probes kept separately from native implementation.
