"""Exact finite specification oracle for Coup events; no bumbledb runtime."""

from collections import defaultdict
from fractions import Fraction as Q
from itertools import combinations, permutations
import json


ROLES = ("Duke", "Assassin", "Captain", "Ambassador", "Contessa")
CARDS = tuple(f"{role}{n}" for role in ROLES for n in (1, 2, 3))
ROLE = {card: role for role in ROLES for card in CARDS if card.startswith(role)}
COURT = ("court",)


def assignment(hands, revealed=frozenset()):
    result = {card: COURT for card in CARDS}
    dealt = set()
    for seat, hand in hands.items():
        for slot, card in enumerate(hand):
            assert card not in dealt
            dealt.add(card)
            result[card] = ("hand", seat, slot, card in revealed)
    return result


def locations(worlds):
    rows = defaultdict(set)
    for w, world in enumerate(worlds):
        assert set(world) == set(CARDS)
        for card, where in world.items():
            rows[card, where].add(w)
    return rows


def partition(events, universe):
    seen = set()
    for event in events:
        if seen & event:
            return False
        seen |= event
    return seen == universe


def check_locations(worlds):
    rows = locations(worlds)
    universe = set(range(len(worlds)))
    for card in CARDS:
        assert partition([e for (c, _), e in rows.items() if c == card], universe)
    for seat in ("A", "B", "C"):
        for slot in (0, 1):
            assert partition([
                e for (_, where), e in rows.items()
                if where[0] == "hand" and where[1:3] == (seat, slot)
            ], universe)
    return rows


def holds(worlds, seat, role):
    return {
        w for w, world in enumerate(worlds)
        if any(ROLE[card] == role and where[0] == "hand"
               and where[1] == seat and not where[3]
               for card, where in world.items())
    }


def run():
    # Condition on A's Duke+Assassin. Physical copy names are a symmetric
    # internal representative; a player receives roles, not these identifiers.
    alice = ("Duke1", "Assassin1")
    remaining = tuple(c for c in CARDS if c not in alice)
    deals = []
    for bob in combinations(remaining, 2):
        for cleo in combinations(tuple(c for c in remaining if c not in bob), 2):
            deals.append(assignment({"A": alice, "B": bob, "C": cleo}))
    assert len(deals) == 4290
    rows = check_locations(deals)
    universe = set(range(len(deals)))

    # A duplicated location in one world violates the key; a removed point
    # violates coverage. No amount of numeric renormalization repairs either.
    card_events = [e.copy() for (c, _), e in rows.items() if c == "Duke1"]
    assert not partition(card_events + [{0}], universe)
    card_events[0].remove(0)
    assert not partition(card_events, universe)

    duke = holds(deals, "B", "Duke")
    assert Q(len(duke), len(deals)) == Q(23, 78)
    individual = [e for (card, where), e in rows.items()
                  if ROLE[card] == "Duke" and where[0] == "hand"
                  and where[1] == "B" and not where[3]]
    assert set().union(*individual) == duke
    naive_card_total = sum((Q(len(e), len(deals)) for e in individual), Q(0))
    assert naive_card_total == Q(24, 78)
    assert duke | duke | duke == duke

    # Two legal allocations with just one live Captain between B and C.
    # A is eliminated; lost cards still occupy their slots.
    dead = frozenset(("Captain1", "Captain2", "Assassin1", "Ambassador1"))
    captain_worlds = [
        assignment({"A": ("Captain1", "Captain2"),
                    "B": ("Assassin1", b), "C": ("Ambassador1", c)}, dead)
        for b, c in [("Captain3", "Contessa1"), ("Contessa1", "Captain3")]
    ]
    check_locations(captain_worlds)
    bob_captain = holds(captain_worlds, "B", "Captain")
    cleo_captain = holds(captain_worlds, "C", "Captain")
    assert len(bob_captain) == len(cleo_captain) == 1
    assert not bob_captain & cleo_captain
    assert bob_captain | cleo_captain == {0, 1}

    # Extend the same deals with one five-way decision draw. The illustrative
    # policy says Tax with probability 4/5 when holding Duke, otherwise 1/5.
    choice_worlds = [(w, u) for w in range(len(deals)) for u in range(5)]
    h = {i for i, (w, _) in enumerate(choice_worlds) if w in duke}
    tax = {i for i, (w, u) in enumerate(choice_worlds)
           if u < (4 if w in duke else 1)}
    all_choices = set(range(len(choice_worlds)))
    other = all_choices - tax
    assert partition([tax, other], all_choices)
    prior_tax = Q(len(tax), len(choice_worlds))
    posterior = Q(len(h & tax), len(tax))
    assert prior_tax == Q(49, 130)
    assert posterior == Q(92, 147)
    assert tax & tax == tax
    assert Q(len(h & tax & tax), len(tax & tax)) == posterior
    # A later decision can exist only on an event. Its disjoint branches
    # cover that parent, retain its raw mass, and normalize conditionally.
    assert partition([tax & h, tax - h], tax)
    assert Q(len(tax & h) + len(tax - h), len(choice_worlds)) == prior_tax
    bounds = [Q(23)*a/(Q(23)*a+Q(55)*b)
              for a in (Q(7, 10), Q(9, 10))
              for b in (Q(1, 10), Q(3, 10))]
    assert min(bounds) == Q(161, 326) and max(bounds) == Q(207, 262)

    # Following a proved Duke: A has one Duke, B has a lost Duke plus the
    # proved Duke. Returning that card makes a ten-card court, one Duke.
    before = assignment({"A": ("Duke1", "Assassin1"),
                         "B": ("Duke2", "Duke3"),
                         "C": ("Captain1", "Ambassador1")}, frozenset(("Duke2",)))
    pool = [c for c, place in before.items() if place == COURT] + ["Duke3"]
    assert len(pool) == 10
    after_worlds = []
    for draw in pool:
        after = before.copy()
        after["Duke3"] = COURT
        after[draw] = ("hand", "B", 1, False)
        # C challenged and chooses Captain1 as the lost influence. It remains
        # in its slot and cannot change the court or the replacement odds.
        after["Captain1"] = ("hand", "C", 0, True)
        after_worlds.append(after)
    check_locations(after_worlds)
    old_duke = set(range(10))
    new_duke = holds(after_worlds, "B", "Duke")
    assert len(new_duke) == 1
    assert Q(len(old_duke - new_duke), 10) == Q(9, 10)

    # An optional exchange branch: half no exchange, half two ordered draws
    # without replacement. Both buffer slots cover precisely the pending event.
    base = assignment({"A": ("Duke1", "Assassin1"),
                       "B": ("Captain1", "Ambassador1"),
                       "C": ("Contessa1", "Duke2")})
    court = [c for c, place in base.items() if place == COURT]
    exchange_worlds = [base]
    for first, second in permutations(court, 2):
        world = base.copy()
        world[first] = ("draw", "B", 0)
        world[second] = ("draw", "B", 1)
        exchange_worlds.append(world)
    exchange_rows = check_locations(exchange_worlds)
    pending = set(range(1, 73))
    assert len(exchange_worlds) == 73
    for slot in (0, 1):
        assert partition([e for (_, where), e in exchange_rows.items()
                          if where == ("draw", "B", slot)], pending)
    assert Q(1, 2) + 72*Q(1, 144) == 1
    first_card = court[0]
    first_event = exchange_rows[first_card, ("draw", "B", 0)]
    second_event = exchange_rows[first_card, ("draw", "B", 1)]
    assert not first_event & second_event
    assert Q(len(first_event), 144) == Q(1, 18)
    assert Q(len(first_event), 144)/Q(1, 2) == Q(1, 9)
    assert not partition([pending - {1}], pending)
    assert not partition([pending | {0}], pending)

    # Three Dukes face up: a recorded Tax claim is still well formed.
    all_dukes_lost = assignment({"A": ("Duke1", "Assassin1"),
                                "B": ("Duke2", "Captain1"),
                                "C": ("Duke3", "Ambassador1")},
                               frozenset(("Duke1", "Duke2", "Duke3")))
    assert not holds([all_dukes_lost], "B", "Duke")
    allowed_claim = {"Tax": "Duke", "Assassinate": "Assassin",
                     "Steal": "Captain", "Exchange": "Ambassador"}
    assert allowed_claim["Tax"] == "Duke"

    return {
        "scope": "Finite specification examples, not macro compilation or native admission.",
        "groups_passed": [
            "card_and_hand_slot_partitions_over_4290_deals",
            "overlap_and_gap_refusals",
            "role_union_avoids_double_counting_two_dukes",
            "one_live_captain_implies_disjoint_opponents",
            "tax_event_conditioning_and_retained_evidence",
            "proof_changes_position_not_historical_truth",
            "exchange_buffers_partition_the_pending_event",
            "legal_claim_does_not_require_possession",
        ],
        "initial_deals": 4290,
        "decision_worlds": len(choice_worlds),
        "has_duke": "23/78",
        "incorrect_sum_of_individual_duke_events": str(naive_card_total),
        "tax_evidence": str(prior_tax),
        "duke_given_tax": str(posterior),
        "duke_given_tax_rate_rectangle": [str(min(bounds)), str(max(bounds))],
        "duke_after_proof_replacement": "1/10",
        "old_duke_and_no_new_duke": "9/10",
        "exchange_worlds": len(exchange_worlds),
        "exchange_evidence": "1/2",
        "specific_first_draw_given_exchange": "1/9",
    }


if __name__ == "__main__":
    print(json.dumps(run(), indent=2))
