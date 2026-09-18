"""Exact legal-hand fibres; a reference example, never a timing benchmark."""
from itertools import combinations
from pathlib import Path
import hashlib
import json

LAB = Path(__file__).resolve().parent
cards = [c for c in range(15) if c not in (0, 3)]
# Alice's fixed Duke and Assassin are excluded, as in the existing Coup fixture.
hands = list(combinations(cards, 2))
dukes = lambda hand: sum(c // 3 == 0 for c in hand)
fibres = {
    first_hand: [second_hand for second_hand in hands if set(first_hand).isdisjoint(second_hand)]
    for first_hand in hands
}
assert len(hands) == 78 and {len(bs) for bs in fibres.values()} == {55}
assert sum(map(len, fibres.values())) == 4290

# Restrict the ORIGINAL deal space by the event that Cleo holds a Duke.
# The cardinality of its Bob-hand fibres now varies, including an empty fibre.
evidence = {a: [b for b in bs if dukes(b)] for a, bs in fibres.items()}
classes = {}
for count in range(3):
    selected = [a for a in hands if dukes(a) == count]
    lengths = {len(evidence[a]) for a in selected}
    assert len(lengths) == 1
    classes[count] = dict(bob_hands=len(selected), legal_completions=55,
                          cleo_duke_completions=lengths.pop())
assert classes == {
    0: dict(bob_hands=55, legal_completions=55, cleo_duke_completions=19),
    1: dict(bob_hands=22, legal_completions=55, cleo_duke_completions=10),
    2: dict(bob_hands=1, legal_completions=55, cleo_duke_completions=0),
}

# The full deal count and the pushforward count must agree for every selected
# subset of these three membership classes, both before and after evidence.
checks = 0
for space in (fibres, evidence):
    for selected in range(8):
        predicate = lambda a: bool(selected >> dukes(a) & 1)
        direct = sum(predicate(a) for a, bs in space.items() for _ in bs)
        pushed = sum(len(bs) for a, bs in space.items() if predicate(a))
        assert direct == pushed
        checks += 1
assert sum(map(len, evidence.values())) == 1265
assert sum(len(bs) for a, bs in evidence.items() if dukes(a)) == 220

# An information readout preserves both sides of each nonempty legal fibre.
possible = {a for a, bs in fibres.items() if any(dukes(b) for b in bs)}
guaranteed = {a for a, bs in fibres.items() if all(dukes(b) for b in bs)}
ambiguous = possible - guaranteed
assert len(possible) == len(ambiguous) == 77 and not guaranteed
assert hands[-1] in possible  # both cards are non-Dukes
assert (1, 2) not in possible

# A player's public-to-them readout exposes roles, not physical copy handles.
# Merge physical-hand fibres before asking the same possibility question.
role_fibres = {}
for first_hand, compatible in fibres.items():
    roles = tuple(c // 3 for c in first_hand)
    role_fibres.setdefault(roles, []).extend((first_hand, b) for b in compatible)
role_possible = {r for r, ws in role_fibres.items() if any(dukes(b) for _, b in ws)}
role_guaranteed = {r for r, ws in role_fibres.items() if all(dukes(b) for _, b in ws)}
assert len(role_fibres) == 15 and sum(map(len, role_fibres.values())) == 4290
assert len(role_possible) == 14 and not role_guaranteed
assert (0, 0) not in role_possible
assert all((a in possible) == (tuple(c // 3 for c in a) in role_possible) for a in hands)
assert len({len(ws) for ws in role_fibres.values()}) > 1

record = dict(
    passed=True, purpose=__doc__,
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    available_cards=cards, hands=len(hands), deals=4290,
    known_alice_cards=[0, 3],
    fibre_classes=classes, pushforward_checks=checks,
    cleo_duke_deals=1265, both_players_duke_deals=220,
    bob_readouts=dict(possible_cleo_duke=len(possible),
                       guaranteed_cleo_duke=len(guaranteed),
                       ambiguous_cleo_duke=len(ambiguous)),
    bob_role_readouts=dict(readouts=len(role_fibres),
                          possible_cleo_duke=len(role_possible),
                          guaranteed_cleo_duke=len(role_guaranteed),
                          ambiguous_cleo_duke=len(role_possible-role_guaranteed),
                          fibre_cardinalities=sorted({len(ws) for ws in role_fibres.values()})),
    boundary='Exact finite counting and possibility; no independent-hand law is inferred. This is not a native readout workload.',
)
(LAB/'results/coup-fibre-reference.json').write_text(json.dumps(record, indent=2)+'\n')
print(json.dumps(record, indent=2))
