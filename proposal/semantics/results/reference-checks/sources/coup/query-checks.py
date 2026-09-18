"""Finite reference semantics for the proposed staged Coup queries.

This executes ordinary Python row matching, region construction, and exact
measurement, then checks independent predicates on complete deals. It does not
parse Rust, execute Free Join, or validate the engine's admission/lowering.
"""

from dataclasses import dataclass
from fractions import Fraction as Q
from itertools import combinations
from pathlib import Path
import json
import runpy


BASE = runpy.run_path(str(Path(__file__).with_name("event-checks.py")))
ROLES, CARDS, ROLE, COURT = (BASE[k] for k in ("ROLES", "CARDS", "ROLE", "COURT"))
assignment, locations = (BASE[k] for k in ("assignment", "locations"))


@dataclass(frozen=True, eq=False)
class Scope:
    # Object identity stands for an already aligned named source descriptor.
    # This oracle deliberately has no implicit cross-scope alignment operation.
    weights: tuple

    def __post_init__(self):
        assert self.weights and all(w > 0 for w in self.weights)
        assert sum(self.weights, Q(0)) == 1

    @classmethod
    def uniform(cls, size):
        return cls((Q(1, size),) * size)

    @property
    def universe(self):
        return frozenset(range(len(self.weights)))


@dataclass(frozen=True)
class Event:
    scope: Scope
    worlds: frozenset

    def __post_init__(self):
        assert self.worlds <= self.scope.universe

    def check(self, other):
        if self.scope is not other.scope:
            raise ValueError("incompatible scopes")

    def __and__(self, other):
        self.check(other)
        return Event(self.scope, self.worlds & other.worlds)

    def __or__(self, other):
        self.check(other)
        return Event(self.scope, self.worlds | other.worlds)

    def __xor__(self, other):
        self.check(other)
        return Event(self.scope, self.worlds ^ other.worlds)

    def __invert__(self):
        return Event(self.scope, self.scope.universe - self.worlds)

    def mass(self):
        return sum((self.scope.weights[w] for w in self.worlds), Q(0))


def empty(scope):
    return Event(scope, frozenset())


def full(scope):
    return Event(scope, scope.universe)


def probability(event, given):
    event.check(given)
    evidence = given.mass()
    if not evidence:
        return {"status": "impossible_observation", "evidence": evidence}
    return {"status": "defined", "evidence": evidence,
            "probability": (event & given).mass() / evidence}


def join(left, right, on):
    """Ordinary matching, with no implicit event intersection."""
    return [a | b for a in left for b in right
            if all(a[k] == b[k] for k in on)]


def pack(rows, keys, field="region"):
    groups = {}
    for row in rows:
        key = tuple(row[k] for k in keys)
        event = row[field]
        groups[key] = groups.get(key, empty(event.scope)) | event
    return [dict(zip(keys, key)) | {field: event}
            for key, event in groups.items()]


def influence_rows(worlds, scope, position):
    return [dict(position=position, card=card, seat=place[1], slot=place[2],
                 revealed=place[3], region=Event(scope, frozenset(region)))
            for (card, place), region in locations(worlds).items()
            if place[0] == "hand"]


def fragments(influence):
    cards = [dict(card=card, role=ROLE[card]) for card in CARDS]
    return join([r for r in influence if not r["revealed"]], cards, ("card",))


def held_query(influence, positions, position_seats, perspectives):
    # Mirrors the two fragments arms and the following Pack. The catalog is
    # closed, and the other rosters are explicit relation inputs.
    rows = fragments(influence)
    parents = join(join(position_seats, positions, ("position",)),
                   perspectives, ("perspective",))
    seeds = [dict(position=p["position"], seat=p["seat"], role=role,
                  region=empty(p["given"].scope))
             for p in parents for role in ROLES]
    return pack(rows + seeds, ("position", "seat", "role"))


def fixture(worlds, scope, position=0, perspective=0, given=None):
    influence = influence_rows(worlds, scope, position)
    positions = [dict(position=position, perspective=perspective)]
    seats = [dict(position=position, seat=s) for s in ("A", "B", "C")]
    views = [dict(perspective=perspective,
                  given=full(scope) if given is None else given)]
    return influence, positions, seats, views


def role_operand(rows, seat, role, name, position=None):
    return [dict(position=r["position"]) | {name: r["region"]}
            for r in rows if r["seat"] == seat and r["role"] == role
            and (position is None or r["position"] == position)]


def region(rows, seat, role, position=0):
    matches = role_operand(rows, seat, role, "e", position)
    assert len(matches) == 1
    return matches[0]["e"]


def direct_has(world, seat, role):
    # Independent of the projected event rows and group/Boolean evaluator.
    return any(ROLE[card] == role and place[0] == "hand"
               and place[1] == seat and not place[3]
               for card, place in world.items())


def run():
    alice = ("Duke1", "Assassin1")
    rest = tuple(c for c in CARDS if c not in alice)
    deals = [assignment({"A": alice, "B": b, "C": c})
             for b in combinations(rest, 2)
             for c in combinations(tuple(x for x in rest if x not in b), 2)]
    assert len(deals) == 4290
    scope = Scope.uniform(len(deals))
    inputs = fixture(deals, scope)
    held = held_query(*inputs)
    assert len(held) == 15
    for seat in ("A", "B", "C"):
        for role in ROLES:
            assert region(held, seat, role).worlds == frozenset(
                i for i, w in enumerate(deals) if direct_has(w, seat, role))

    # Three ordinary held lookups, followed by one Boolean computed output.
    bs = role_operand(held, "B", "Captain", "b")
    cs = role_operand(held, "C", "Captain", "c")
    aa = role_operand(held, "C", "Ambassador", "a")
    bindings = join(join(bs, cs, ("position",)), aa, ("position",))
    assert len(bindings) == 1
    b, c, a = (bindings[0][k] for k in ("b", "c", "a"))
    window = b & ~(c | a)
    expected = frozenset(i for i, w in enumerate(deals)
                         if direct_has(w, "B", "Captain")
                         and not any(direct_has(w, "C", r)
                                     for r in ("Captain", "Ambassador")))
    assert window.worlds == expected
    assert window.mass() == Q(189, 1430)
    naive = b.mass() * (~(c | a)).mass()
    assert b.mass() == Q(11, 26) and (~(c | a)).mass() == Q(7, 26)
    assert naive == Q(77, 676) and naive != window.mass()
    for w in scope.universe:
        indicator = int(w in b.worlds) * (1-int(w in c.worlds)) * (1-int(w in a.worlds))
        assert indicator == int(w in expected)

    # Different graph paths are distinct bindings, yet their shared worlds
    # contribute only once. Even distinct physical Duke events overlap.
    duke = region(held, "B", "Duke")
    parts = [r for r in fragments(inputs[0])
             if r["seat"] == "B" and r["role"] == "Duke"]
    paths = [dict(card=card, path=path) for card in CARDS for path in (0, 1)]
    repeated = join(parts, paths, ("card",))
    assert len(repeated) == 2 * len(parts)
    assert pack(repeated, ("position", "seat", "role"))[0]["region"] == duke
    assert sum((r["region"].mass() for r in parts), Q(0)) == Q(24, 78)
    assert duke.mass() == Q(23, 78)

    # Anti-probing for a holding row is false, although its event complement
    # has substantial mass. Complement-before-Pack is also a different query.
    assert parts and (~duke).mass() == Q(55, 78)
    wrong = pack([r | {"region": ~r["region"]} for r in parts],
                 ("position", "seat", "role"))[0]["region"]
    assert wrong == full(scope) and wrong != ~duke
    assert (duke | (duke & b)) == duke
    assert (duke & (b | c)) == ((duke & b) | (duke & c))

    # A completed impossible role is a row with empty, not a missing group.
    lost = assignment({"A": ("Duke1", "Assassin1"),
                       "B": ("Duke2", "Captain1"),
                       "C": ("Duke3", "Ambassador1")},
                      frozenset(("Duke1", "Duke2", "Duke3")))
    lost_scope = Scope.uniform(1)
    lost_held = held_query(*fixture([lost], lost_scope))
    impossible_duke = region(lost_held, "B", "Duke")
    assert impossible_duke == empty(lost_scope)
    assert ~impossible_duke == full(lost_scope)
    assert held_query([], [], [], []) == []
    assert (duke & ~duke) == empty(scope)
    assert ~(duke & ~duke) == full(scope)

    # Two compatible deals, the last live Captain is in B's or C's last slot.
    dead = frozenset(("Captain1", "Captain2", "Assassin1", "Ambassador1"))
    two = [assignment({"A": ("Captain1", "Captain2"),
                       "B": ("Assassin1", b), "C": ("Ambassador1", c)}, dead)
           for b, c in (("Captain3", "Contessa1"), ("Contessa1", "Captain3"))]
    two_scope = Scope.uniform(2)
    two_held = held_query(*fixture(two, two_scope))
    bc, cc = (region(two_held, s, "Captain") for s in ("B", "C"))
    assert bc.mass() == cc.mass() == Q(1, 2) and bc != cc
    assert (bc ^ cc) == full(two_scope)
    assert (bc & cc) == ~(bc | cc) == empty(two_scope)
    assert join([dict(e=bc)], [dict(e=cc)], ("e",)) == []

    # Policy source: one named five-way draw, conditional on the actual hand.
    # Rates are illustrative (4/5 with Duke, 1/5 without), not model responses.
    trajectories = [(w, u) for w in deals for u in range(5)]
    decision_scope = Scope.uniform(len(trajectories))
    decision_deals = [w for w, _ in trajectories]
    tax = Event(decision_scope, frozenset(i for i, (w, u) in enumerate(trajectories)
                    if u < (4 if direct_has(w, "B", "Duke") else 1)))
    # An illustrative Tax/Income policy, joined to the ordinary option roster.
    next_moves = [dict(decision=1, option=10, region=tax),
                  dict(decision=1, option=11, region=~tax)]
    options = [dict(decision=1, option=10, kind="Tax"),
               dict(decision=1, option=11, kind="Income")]
    kinds = ("Income", "ForeignAid", "Tax", "Assassinate", "Steal", "Exchange", "CoupAction")
    seeds = [dict(decision=1, kind=k, region=empty(decision_scope)) for k in kinds]
    moves = pack(join(next_moves, options, ("decision", "option")) + seeds,
                 ("decision", "kind"))
    assert len(moves) == 7
    assert next(r["region"] for r in moves if r["kind"] == "Tax") == tax
    assert next(r["region"] for r in moves if r["kind"] == "Steal") == empty(decision_scope)
    decision_held = held_query(*fixture(decision_deals, decision_scope))
    bob_duke = region(decision_held, "B", "Duke")
    cleo_duke = region(decision_held, "C", "Duke")
    answer = probability(~bob_duke, full(decision_scope) & tax)
    direct_bluffs = sum(not direct_has(trajectories[i][0], "B", "Duke")
                        for i in tax.worlds)
    assert answer["probability"] == Q(direct_bluffs, len(tax.worlds)) == Q(55, 147)
    assert answer["evidence"] == Q(49, 130)
    assert probability(~bob_duke, tax & tax) == answer
    remote = probability(cleo_duke, tax)
    direct_remote = sum(direct_has(trajectories[i][0], "C", "Duke")
                        for i in tax.worlds)
    assert remote["probability"] == Q(direct_remote, len(tax.worlds)) == Q(5, 21)
    assert cleo_duke.mass() == Q(23, 78)

    # Before/after predicates refer to the same trajectory and fresh draw.
    before = assignment({"A": ("Duke1", "Assassin1"),
                         "B": ("Duke2", "Duke3"),
                         "C": ("Captain1", "Ambassador1")}, frozenset(("Duke2",)))
    pool = [card for card, place in before.items() if place == COURT] + ["Duke3"]
    after = []
    for draw in pool:
        w = before.copy()
        w["Duke3"] = COURT
        w[draw] = ("hand", "B", 1, False)
        w["Captain1"] = ("hand", "C", 0, True)
        after.append(w)
    proof_scope = Scope.uniform(len(pool))
    old_held = held_query(*fixture([before] * len(pool), proof_scope, position=0))
    new_held = held_query(*fixture(after, proof_scope, position=1))
    old, new = region(old_held, "B", "Duke", 0), region(new_held, "B", "Duke", 1)
    changed = old & ~new
    assert changed.worlds == frozenset(i for i, w in enumerate(after)
                  if direct_has(before, "B", "Duke") and not direct_has(w, "B", "Duke"))
    assert old.mass() == 1 and new.mass() == Q(1, 10) and changed.mass() == Q(9, 10)

    # Partial decision: complement is relative to full, never auto-normalized.
    partial_scope = Scope.uniform(4)
    occurs = Event(partial_scope, frozenset((0, 1)))
    partial_tax = Event(partial_scope, frozenset((0,)))
    assert (occurs & ~partial_tax).mass() == Q(1, 4)
    assert (~partial_tax).mass() == Q(3, 4)
    assert probability(~partial_tax, occurs)["probability"] == Q(1, 2)

    # Zero answer, impossible evidence, and scope error are distinct.
    assert probability(empty(scope), full(scope))["probability"] == 0
    assert probability(full(scope), empty(scope))["status"] == "impossible_observation"
    other_scope = Scope.uniform(len(deals))
    refused = 0
    for operation in (lambda: empty(scope) & full(other_scope),
                      lambda: probability(duke, full(other_scope))):
        try:
            operation()
        except ValueError as error:
            assert str(error) == "incompatible scopes"
            refused += 1
    assert refused == 2

    return {
        "scope": "Finite staged semantic evaluator; not query! or Free Join execution.",
        "groups_passed": [
            "held_roles_match_all_direct_deal_predicates",
            "three_way_steal_query_and_indicator_lowering",
            "overlap_and_duplicate_paths_pack_idempotently",
            "anti_join_is_not_event_complement",
            "complement_after_pack_and_boolean_rewrites",
            "explicit_empty_groups_and_missing_parent_distinction",
            "captain_xor_and_event_equality_not_equal_probability",
            "tax_conditioning_and_repeated_evidence",
            "bobs_declaration_updates_cleos_hand_probability",
            "cross_position_proof_and_replacement",
            "partial_decision_complement_retains_absence_worlds",
            "zero_answer_impossible_evidence_and_scope_errors",
        ],
        "initial_deals": len(deals),
        "decision_worlds": len(trajectories),
        "steal_support": str(window.mass()),
        "incorrect_independent_product": str(naive),
        "bob_lacks_duke_given_tax": str(answer["probability"]),
        "tax_evidence": str(answer["evidence"]),
        "cleo_duke_before_tax": str(cleo_duke.mass()),
        "cleo_duke_given_tax": str(remote["probability"]),
        "proved_duke_then_no_duke": str(changed.mass()),
    }


if __name__ == "__main__":
    print(json.dumps(run(), indent=2))
