# Coup queries: construct the situation, then ask its probability

**These are proposed Rust macro extensions, not working engine syntax yet.**
The complete templates are in [queries.rs](queries.rs); the source mapping and
query contracts are in [query-algebra.md](../query-algebra.md).

The new operations are small:

```rust
Event(!a)             // complement
Event(a & b)          // both
Event(a | b)          // either, counting overlap once
Event(a ^ b)          // exactly one
Pack(a)               // union across matching rows
Probability(a, given) // conditional probability, preserving evidence mass
```

An event is a reusable predicate on the shared deal and history. It retains
which worlds it describes after a query. A displayed probability is one
observation of that value. These connections are classical dependence, with
no quantum behavior or assumed independence.

## 1. Get a role event without counting a two-Duke hand twice

The shared `held_roles` template starts from live `Influence` rows, joins
`Card` to find each role, and uses `Pack` to union their `when` regions. Its
output is an ordinary derived relation with four columns:

```text
held(position, seat, role, holds)
```

One row means: in this position, this player holds a live copy of this role
in the worlds described by `holds`. If two physical Dukes are in Bob's hand,
that world belongs to the union once.

The helper explicitly seeds every admitted position/seat/role group with an
empty event. An impossible role therefore still has a row. A missing position
stays missing. The full two-arm definition is in [queries.rs](queries.rs).

For the initial examples, Alice holds Duke and Assassin. Adopt a fair deal of
the remaining thirteen cards, with no behavioral observations yet. There are
4,290 equally likely assignments of two-card hands to Bob and Cleo. Then:

```text
P(Bob has Duke)                      = 23/78 ≈ 29.49%
Sum of Bob's individual Duke chances = 24/78 ≈ 30.77%  // double-counts one hand
```

## 2. Bob can defend his steal; Cleo cannot truthfully block it

Bob needs Captain to substantiate the steal. Cleo can truthfully block with
Captain or Ambassador. We want:

```rust
Event(bob_captain & !(cleo_captain | cleo_ambassador))
```

The complete staged query is:

```rust
let steal_window = bumbledb_query::query!(Coup {
    use held = &held_roles;

    interior window(position, when: Event(b & !(c | a)), given) |
        held(position, bob, captain, b),
        held(position, cleo, captain, c),
        held(position, cleo, ambassador, a),
        bob == ?bob_seat, cleo == ?cleo_seat,
        captain == ?captain_role, ambassador == ?ambassador_role,
        Position(id: position, perspective),
        Perspective(id: perspective, given),
        position == ?position_id;

    (position, when, chance: Probability(when, given)) |
        window(position, when, given);
});
```

Bind Bob's and Cleo's seats, the Captain and Ambassador IDs from the closed
role roster, and the desired position. No model, weight, or dependency argument
is added to these role lookups. Each event already retains its source context.

For Alice's initial information above:

| Question | Exact probability | Approximate |
| --- | --- | ---: |
| Bob has Captain | `11/26` | 42.31% |
| Cleo has neither blocking role | `7/26` | 26.92% |
| Both conditions in the same deal | **`189/1430`** | **13.22%** |
| Incorrect product of the two marginals | `77/676` | 11.39% |

The shared deck accounts for the difference. This is card support for an
action, not its eventual success probability: Cleo can still bluff a block,
and the players can choose whether to challenge. To ask about actual success,
combine the corresponding reaction and resolution events from a supplied game
transition and policy model. The card expression does not invent those choices.

The returned `when` remains an event. Another query can intersect it with
“Bob actually chooses Steal targeting Cleo,” or union it with another favorable
situation before measuring anything.

## 3. Bob speaks; Cleo's hidden hand becomes less likely to contain Duke

Use the same illustrative policy as the main Coup walkthrough:

```text
P(Bob chooses Tax | Bob has Duke)    = 4/5
P(Bob chooses Tax | Bob lacks Duke)  = 1/5
```

These numbers are assumptions for an inspectable example, not measured
TypeSafe output. A conditional policy constructor extends each deal with
the named action outcome. `move_kinds` packs `NextMove` events by action kind,
so its Tax result retains its relationship to Bob's hand and the shared deck.

Now evaluate:

```rust
Probability(!bob_duke, given & tax)  // Bob cannot substantiate this declaration
Probability(cleo_duke, given & tax)  // what did Bob's action imply about Cleo?
```

Inability to substantiate differs from the eventual challenge outcome; a holder
can still concede. The results are:

| Holding | Before Bob says Tax | After observing Tax |
| --- | ---: | ---: |
| Bob has Duke | `23/78` = 29.49% | `92/147` = 62.59% |
| Bob lacks Duke | `55/78` = 70.51% | **`55/147` = 37.41%** |
| Cleo has Duke | `23/78` = 29.49% | **`5/21` = 23.81%** |

Cleo has said nothing. Bob's declaration favors deals where he holds a Duke;
those deals leave fewer Dukes available to Cleo. The event algebra carries
that consequence across relations. There is no model call for the derived
question and no manual multiplication of disconnected confidence values.

Here is the actual proposed query for Cleo's update. It does not even need to
look up Bob's Duke event: the Tax event already preserves that dependence.

```rust
let another_players_hand_after_tax = bumbledb_query::query!(Coup {
    use held = &held_roles;
    use moves = &move_kinds;
    (decision, other, chance: Probability(duke, given & tax)) |
        Decision(id: decision, position, actor),
        Position(id: position, perspective),
        Perspective(id: perspective, given),
        held(position, other, role, duke), role == ?duke_role,
        other == ?other_seat, other != actor,
        moves(decision, kind, tax), kind == ?tax_kind,
        decision == ?decision_id;
});
```

Bind Bob's decision, Cleo's seat, and the Duke/Tax roster IDs. Use pre-action
evidence for `given`. The hypothetical Tax observation has probability `49/130`,
which the measurement result retains. If `given` already includes this same Tax
event, intersecting it again is harmless: `tax & tax = tax`.

TypeSafe's part is supplying an appropriate conditional policy from authorized
information. Noul supplies a probability for one binary question; Choice
supplies an option distribution. The adapter binds that response to its actual
decision and information case. A collection of marginal Noul answers cannot
by itself recover this joint dependence. These results follow from the adopted
policy and deal laws, not from knowing an opponent's true psychology.

## 4. Exactly one Captain claim is supportable

Consider a separate position: two Captains are revealed, and the remaining
live Captain is known to be either Bob's or Cleo's. Neither has another live
card. Let the two possible allocations each have probability one half.

```rust
(position, exactly_one: Event(b ^ c),
           both: Event(b & c), neither: Event(!(b | c))) |
    held(position, bob, captain, b),
    held(position, cleo, captain, c),
    bob == ?bob_seat, cleo == ?cleo_seat,
    captain == ?captain_role, position == ?position_id;
```

Here `exactly_one` is full, while `both` and `neither` are empty. Their
probabilities are 1, 0, and 0. The individual holding events each measure 1/2,
but are unequal and complementary. Two percentages would lose that distinction.

The 50/50 law is stipulated for this example. New behavioral evidence could
change the individual odds while preserving the structural fact that exactly
one of the two claims is supportable.

## 5. A truthful proof can make tomorrow's claim false

Use the proof/replacement position from the main walkthrough. Alice has a
Duke, Bob has a revealed Duke, and Bob proves his remaining live card is Duke.
He returns it to the nine-card court and draws from the resulting ten cards.
Only one of those ten is Duke. The challenger's lost influence stays on table.

```rust
Event(old_duke & !new_duke)
Probability(old_duke & !new_duke, given)
```

The exact result is **9/10**. Bob could prove Duke before the replacement and
cannot prove Duke immediately afterwards in 90% of these trajectories.
The historical proof remains true. `lost_the_proved_duke` in
[queries.rs](queries.rs) joins both positions within one perspective before
constructing this event. A transition constructor supplies their actual joint
history; joining arbitrary snapshots does not create it.

## 6. The complement includes worlds where a future turn never happens

Suppose Bob reaches a future decision in half the worlds. In half of those,
he chooses Tax. Then:

```rust
Event(!tax)           // no Tax: he chose otherwise OR never reached this turn
Event(occurs & !tax)  // he reached this turn and chose something else
```

The first has probability 3/4; the second 1/4. Conditional on reaching the
decision, not choosing Tax has probability 1/2:

```rust
Probability(!tax, given & occurs)
```

The next-move mirror covers `Decision.occurs`. It does not normalize that
partial region into a new universe. This lets future elimination and alternative
actions remain connected in the same history.

## 7. What Free Join and the math each do

For the steal query, Free Join matches three role rows and the perspective.
The computed event stage receives the bound regions and constructs:

```text
E = B ∩ complement(C ∪ A)
```

As a world predicate, this is multiplication of Boolean indicators:

```text
iE(w) = iB(w) × (1 - iC(w)) × (1 - iA(w))
```

Measurement sums that predicate under the shared law, restricted to `given`,
then divides by the retained evidence mass. Multiplication happens before
expectation. Multiplying three already-reported probabilities would lose the
shared-deck relationships.

The engine already has Free Join → computed outputs → grouped sinks and
nonrecursive derived stages. Revision 0.5 retains a
[canonical partition/BDD carrier](../representation.md) and its
[candidate ARM64 kernels](../kernels.md). The expression forms, packing, and
exact measurements still need native implementation. This architecture need
not store one database row per possible world.

[query-checks.py](query-checks.py) checks the proposed staged semantics against
direct predicates on all 4,290 initial deals and 21,450 deal/action worlds, plus
the smaller transition and complement examples. All 12 groups pass in the
[saved output](research/query-checks.json). This is reference semantic execution,
not a claim that the macro or native Free Join has run these queries.

[The larger applications](algebra-applications.md) go beyond compound predicates:
construct uniform safe actions, compare information cases, and derive definite
consequences from uncertain choices using the same Event representation.
