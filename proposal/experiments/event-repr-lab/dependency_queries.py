"""Independent finite derivation: dependency obstructions are queryable Events."""
from itertools import product
from pathlib import Path
import json
import hashlib

FULL = 15

def summary(events):
    covered, conflict = 0, 0
    for event in events:
        conflict |= covered & event
        covered |= event
    return covered, conflict

cases = gaps = 0
for events in product(range(16), repeat=3):
    # Each position identifies a distinct whole fact, even if its payload agrees.
    incidence = {(w, fact) for w in range(4) for fact, e in enumerate(events)
                 if e >> w & 1}
    covered = sum(1 << w for w in range(4) if any(x == w for x, _ in incidence))
    conflict = sum(1 << w for w in range(4)
                   if len({fact for x, fact in incidence if x == w}) >= 2)
    join_conflict = 0
    for a in range(3):
        for b in range(a + 1, 3):
            join_conflict |= events[a] & events[b]
    assert summary(events) == (covered, conflict)
    assert join_conflict == conflict
    # Converse(incidence);incidence is a relation on distinct fact identities.
    share_a_world = {(a, b) for w, a in incidence for x, b in incidence if w == x}
    assert (conflict == 0) == all(a == b for a, b in share_a_world)
    for source in range(16):
        gap = sum(1 << w for w in range(4)
                  if source >> w & 1 and not any(x == w for x, _ in incidence))
        assert gap == source & (FULL ^ covered)
        gaps += 1
    cases += 1

# A collision relation on fact IDs forgets WHERE those facts overlapped.
events = [3, 2]  # fact A at worlds 0,1; fact B only at world 1
incidence = {(w, f) for w in range(4) for f, e in enumerate(events) if e >> w & 1}
colliding = {(a, b) for w, a in incidence for x, b in incidence if w == x and a != b}
naive = sum(1 << w for w in range(4)
            if any(x == w and any(a == f for a, _ in colliding) for x, f in incidence))
assert summary(events)[1] == 2 and naive == 3

# Exact whole-fact normalization precedes contributor counting.
rows = [('same-fact', 3), ('same-fact', 3)]
assert summary([e for _, e in set(rows)])[1] == 0
assert summary([3, 3])[1] == 3  # distinct facts can have identical projected payloads

out = {
    'passed': True,
    'checker_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    'incidence_functionality_cases': cases,
    'containment_gap_cases': gaps,
    'lost_context_counterexample': {'true_conflict_worlds': [1], 'naive_worlds': [0, 1]},
    'scope': 'Independent finite query derivation, not a native query-planner implementation or benchmark',
}
path = Path(__file__).parent/'results/dependency-query-checks.json'
path.write_text(json.dumps(out, indent=2)+'\n')
print(json.dumps(out, indent=2))
